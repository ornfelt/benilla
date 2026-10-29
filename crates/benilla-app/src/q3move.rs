//! Quake 3 movement (the `q3_movement` feature, off by default): a client-side hack, not the
//! reference's, for a local vmangos with its movement anticheat off (`Anticheat.Enable = 0`),
//! which accepts the faster positions as reported.
//!
//! The mover's horizontal velocity runs through ioq3's VQ3 pmove (`code/game/bg_pmove.c`) instead
//! of WoW's straight key-to-velocity: ground friction and acceleration (`PM_Friction`,
//! `PM_Accelerate` at `pm_accelerate`), air acceleration at `pm_airaccelerate` with no friction,
//! and the air slide clipping the velocity (`PM_SlideMove`), so forward + strafe with a mouse turn
//! builds speed in the air as in Quake 3. The speeds stay WoW's: the wish speed is the live run,
//! walk, backward or mounted speed, and gravity and the jump are WoW's. Holding jump hops again on
//! the landing frame, with no friction frame between (the auto-hop is in the controller).
//! Swimming, the settle hold, a root and a knockback's launch frame keep WoW's movement.
//!
//! On at start; the dev chord + `Q` (Ctrl+Shift+Q) or the `q3move [on|off]` console command
//! switches between it and WoW's movement. [`Q3MovePlugin`] also draws the speedometer: the
//! horizontal speed in Q3 units per second ([`UNITS_PER_YARD`]) and in yards per second.

use std::sync::atomic::{AtomicBool, Ordering};

use benilla_ui::script::{JustifyH, JustifyV, Outline};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use crate::player::Player;
use crate::ui_pass::{UiQuad, UiQuads};
use crate::ui_text::{layout_text_quads, FontSpec, Justify, UiFontAtlas};

/// Q3 units per yard: one unit is an inch, so WoW's 7 yd/s run reads 252 ups (Q3's `g_speed` is
/// 320).
pub(crate) const UNITS_PER_YARD: f32 = 36.0;
/// `pm_accelerate`, `pm_airaccelerate` and `pm_friction` (bg_pmove.c): rates per second, so the
/// same in any unit.
const ACCELERATE: f32 = 10.0;
const AIR_ACCELERATE: f32 = 1.0;
const FRICTION: f32 = 6.0;
/// `pm_stopspeed` (100 ups) in yd/s: under it, friction brakes as if at this speed.
const STOP_SPEED: f32 = 100.0 / UNITS_PER_YARD;
/// `PM_Friction`'s `speed < 1` (ups) rest cutoff, in yd/s.
const REST_SPEED: f32 = 1.0 / UNITS_PER_YARD;
/// `pmove_msec` 8 (125 Hz): the velocity integrates in steps no longer than this, so the strafe
/// gain does not depend on the frame rate.
const STEP_SECS: f32 = 0.008;

static ON: AtomicBool = AtomicBool::new(true);

#[cfg(test)]
thread_local! {
    /// A test's own switch-off, so a test pinning WoW's mover cannot race the others' threads.
    static TEST_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether Quake 3 movement is on now (it starts on).
pub(crate) fn enabled() -> bool {
    #[cfg(test)]
    if TEST_OFF.with(|c| c.get()) {
        return false;
    }
    ON.load(Ordering::Relaxed)
}

pub(crate) fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Runs `f` with Quake 3 movement off on this thread: the tests that pin WoW's mover, which
/// switched off must hand it back unchanged.
#[cfg(test)]
pub(crate) fn with_wow_mover<R>(f: impl FnOnce() -> R) -> R {
    let was = TEST_OFF.with(|c| c.replace(true));
    let out = f();
    TEST_OFF.with(|c| c.set(was));
    out
}

/// `PM_Friction` for a walking player: the ground's drop, at least `pm_stopspeed`'s.
fn friction(vel: Vec3, dt: f32) -> Vec3 {
    let speed = vel.xz().length();
    if speed < REST_SPEED {
        return Vec3::ZERO;
    }
    let control = speed.max(STOP_SPEED);
    let new_speed = (speed - control * FRICTION * dt).max(0.0);
    vel * (new_speed / speed)
}

/// `PM_Accelerate`: speed is added only along `wish_dir`, up to `wish_speed` measured along it,
/// which is why turning into the strafe lets the total speed grow past it.
fn accelerate(vel: Vec3, wish_dir: Vec3, wish_speed: f32, accel: f32, dt: f32) -> Vec3 {
    let add = wish_speed - vel.dot(wish_dir);
    if add <= 0.0 {
        return vel;
    }
    vel + wish_dir * (accel * dt * wish_speed).min(add)
}

/// One frame of the horizontal velocity (Bevy XZ, yd/s), in [`STEP_SECS`] steps: `PM_Friction`
/// then `PM_Accelerate` at `pm_accelerate` when `on_ground`, else `pm_airaccelerate` alone, which
/// the jump frame takes too (`PM_CheckJump` hands it to `PM_AirMove`). `wish_dir` is a horizontal
/// unit vector, or zero with no keys; `wish_speed` is the WoW speed.
pub(crate) fn velocity(
    mut vel: Vec3,
    on_ground: bool,
    wish_dir: Vec3,
    wish_speed: f32,
    dt: f32,
) -> Vec3 {
    vel.y = 0.0;
    if dt <= 0.0 || !dt.is_finite() {
        return vel;
    }
    let steps = (dt / STEP_SECS).ceil() as u32;
    let step_dt = dt / steps as f32;
    let wish = wish_speed > 0.0 && wish_dir != Vec3::ZERO;
    let accel = if on_ground {
        ACCELERATE
    } else {
        AIR_ACCELERATE
    };
    for _ in 0..steps {
        if on_ground {
            vel = friction(vel, step_dt);
        }
        if wish {
            vel = accelerate(vel, wish_dir, wish_speed, accel, step_dt);
        }
    }
    vel
}

/// The air slide's clip (`PM_SlideMove`): the horizontal part of the slide's projected velocity
/// when the slide took speed off `vel`, a wall or a ledge's lip; else `vel`, so a fall onto a
/// slope never turns into horizontal speed.
pub(crate) fn clip(vel: Vec3, projected: Vec3) -> Vec3 {
    let clipped = projected.with_y(0.0);
    if clipped.xz().length() < vel.xz().length() {
        clipped
    } else {
        vel
    }
}

// ---- The toggle ------------------------------------------------------------------------------

fn toggle(keys: Res<ButtonInput<KeyCode>>) {
    // The dev chord, not a bare `Q`, which strafes left; a chord cannot be typed text.
    if benilla_world::modkeys::dev_chord(&keys, KeyCode::KeyQ) {
        set_enabled(!enabled());
    }
}

/// `q3move [on|off]`: sets the mode, or toggles it with no argument, and prints it.
fn q3move_command(_world: &mut World, args: &str) -> Vec<String> {
    let mut line = String::new();
    match args {
        "" => set_enabled(!enabled()),
        a if a.eq_ignore_ascii_case("on") || a == "1" => set_enabled(true),
        a if a.eq_ignore_ascii_case("off") || a == "0" => set_enabled(false),
        a => line.push_str(&format!("q3move: '{a}' is not on or off; ")),
    }
    line.push_str(&format!(
        "Quake 3 movement is {}",
        if enabled() { "on" } else { "off" }
    ));
    vec![line]
}

// ---- The speedometer -------------------------------------------------------------------------
// Client-space sRGB for [`UiQuad::color`]: the perf pill's black alpha 224 fill and gray 235 text.
const S_FILL: [f32; 4] = [0.0, 0.0, 0.0, 224.0 / 255.0];
const S_TEXT: [f32; 4] = [0.92, 0.92, 0.92, 1.0];
/// `|cAARRGGBB` markup for the dim yd/s and mode run (gray 180).
const S_DIM_MARKUP: &str = "|cffb4b4b4";
/// Above every packed WoW z_key, as the perf pill's.
const Z_SPEEDO_BG: u128 = u128::MAX - 1;
const Z_SPEEDO: u128 = u128::MAX;
/// The font height (logical px) and box padding.
const SPEEDO_PX: f32 = 16.0;
const SPEEDO_PAD: Vec2 = Vec2::new(10.0, 4.0);
/// The pill's centre, as a fraction of the window height: under the character, above the bars.
const SPEEDO_Y_FRAC: f32 = 0.62;
/// A frame faster than this (yd/s) is a teleport or a deck change, not movement: it is skipped.
const SPEEDO_JUMP_SPEED: f32 = 200.0;

/// [`speedometer`]'s state: the last position measured from, and the laid-out pill, valid for
/// one reading, mode and window size.
#[derive(Default)]
struct Speedo {
    last: Option<Vec3>,
    speed: f32,
    /// `(ups, tenths of yd/s, mode, window size)` the quads were laid out for.
    key: Option<(i32, i32, bool, Vec2)>,
    quads: Vec<UiQuad>,
}

/// The pill's quads for `text`, its strip centred on `center`.
fn pill(atlas: &UiFontAtlas, text: &str, center: Vec2) -> Vec<UiQuad> {
    let mut e = atlas.lock();
    let glyphs = layout_text_quads(
        &mut e,
        text,
        Rect::from_center_size(center, Vec2::ZERO),
        S_TEXT,
        Justify {
            h: JustifyH::Center,
            v: JustifyV::Middle,
        },
        Z_SPEEDO,
        FontSpec {
            path: None,
            height: Some(SPEEDO_PX),
            outline: Outline::None,
            monochrome: false,
            spacing: 0.0,
            shadow_x: 0.0,
            alpha_gradient: None,
            non_space_wrap: false,
            max_lines: 0,
        },
        crate::ui_text::TextSeat::Exact,
    );
    drop(e);
    let bounds = glyphs
        .iter()
        .map(|q| q.rect)
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::from_center_size(center, Vec2::ZERO));
    let mut out = vec![UiQuad {
        rect: Rect::from_corners(bounds.min - SPEEDO_PAD, bounds.max + SPEEDO_PAD),
        z_key: Z_SPEEDO_BG,
        color: S_FILL,
        ..Default::default()
    }];
    out.extend(glyphs);
    out
}

fn speedometer(
    time: Res<Time>,
    player: Option<Res<Player>>,
    atlas: Option<Res<UiFontAtlas>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut quads: ResMut<UiQuads>,
    mut s: Local<Speedo>,
) {
    let Some(player) = player.filter(|p| p.active) else {
        s.last = None;
        return;
    };
    // The achieved horizontal speed this frame, from the body's own motion: the controller has
    // moved it already (`UiQuadAppend` runs after `WorldStage::Input`), so this frame's delta goes
    // with this frame's step.
    let dt = time.delta_secs();
    if let Some(last) = s.last {
        if dt > 0.0 {
            let speed = (player.pos - last).xz().length() / dt;
            if speed <= SPEEDO_JUMP_SPEED {
                s.speed = speed;
            }
        }
    }
    s.last = Some(player.pos);

    let (Some(atlas), Ok(win)) = (atlas, windows.single()) else {
        return; // headless
    };
    let size = Vec2::new(win.width(), win.height());
    let ups = (s.speed * UNITS_PER_YARD).round() as i32;
    let tenths = (s.speed * 10.0).round() as i32;
    let on = enabled();
    let key = (ups, tenths, on, size);
    if s.key != Some(key) {
        // "412 ups  11.4 yd/s q3": the Q3 reading in full, the WoW one and the mode dim.
        let mode = if on { "q3" } else { "wow" };
        let text = format!(
            "{ups} ups  {S_DIM_MARKUP}{}.{} yd/s {mode}|r",
            tenths / 10,
            tenths % 10
        );
        s.quads = pill(
            &atlas,
            &text,
            Vec2::new(size.x * 0.5, size.y * SPEEDO_Y_FRAC),
        );
        s.key = Some(key);
    }
    quads.overlays.extend(s.quads.iter().cloned());
}

/// The toggle (the chord and the console command) and the speedometer. After `ConsolePlugin`.
pub(crate) struct Q3MovePlugin;

impl Plugin for Q3MovePlugin {
    fn build(&self, app: &mut App) {
        use crate::console::ConsoleCommandApp;
        app.console_command(
            "q3move",
            crate::console::GAME,
            Some("Quake 3 movement: q3move toggles it, q3move on|off sets it."),
            q3move_command,
        );
        app.add_systems(Update, toggle)
            .add_systems(Update, speedometer.in_set(crate::ui_pass::UiQuadAppend));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: f32 = 7.0;

    #[test]
    fn the_ground_tops_out_at_the_wow_speed_and_brakes_to_rest() {
        let dt = 1.0 / 144.0;
        let mut v = Vec3::ZERO;
        for _ in 0..144 {
            v = velocity(v, true, Vec3::NEG_Z, RUN, dt);
        }
        assert!((v.length() - RUN).abs() < 1e-4, "{v}");
        for _ in 0..144 {
            v = velocity(v, true, Vec3::ZERO, RUN, dt);
        }
        assert_eq!(v, Vec3::ZERO);
    }

    #[test]
    fn a_strafe_in_the_air_builds_speed_past_the_wish_speed() {
        let dt = 1.0 / 144.0;
        let mut v = Vec3::NEG_Z * RUN;
        for _ in 0..120 {
            // The wish direction held just past a right angle off the velocity, the strafe.
            let a = v.z.atan2(v.x) + 1.5;
            v = velocity(v, false, Vec3::new(a.cos(), 0.0, a.sin()), RUN, dt);
        }
        assert!(v.length() > RUN * 1.3, "{v}");
    }

    #[test]
    fn the_clip_takes_speed_off_and_never_adds_it() {
        let v = Vec3::new(5.0, 0.0, 0.0);
        assert_eq!(clip(v, Vec3::new(0.0, -3.0, 0.0)), Vec3::ZERO);
        assert_eq!(clip(v, Vec3::new(9.0, -3.0, 4.0)), v);
    }
}
