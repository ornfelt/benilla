//! The reference's full-screen glow (`FFXGlow.bls`): the scene is box-downsampled to ¼ in one
//! pass (dims floored at 8, `0x6cdb40`), blurred by Gauss4 H then V (weights ⅛ ⅜ ⅜ ⅛), and
//! combined as `lerp(screen, blur, z) + w·blur²` in gamma bytes (`ffx_combine.fs.gfxs`), `w` the
//! zone glow and `z` the haze. The combine also owns the frame's one gamma decode. A world view
//! the player-UI camera claims ([`FfxBackdrop`]) runs only the filter passes, and the UI camera
//! draws its combine.

use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};

use crate::view::WorldCamera;

/// A camera running the FFXGlow pass: the frame's one gamma→linear decode, plus the glow add.
#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct FfxGlow {
    /// The per-view multiplier on the zone glow weight; `0.0` keeps the decode, drops the glow.
    pub(crate) gain_scale: f32,
    /// Which FFX pass pair this view runs, and so what drives the combine's death gate `y` and
    /// haze `z`. The zone glow is scene data, outside it, so a bake keeps it.
    pub(crate) state: FfxState,
}

/// Which of the reference's two FFX pass pairs a view runs. The reference keeps the painting
/// screen's pair in one active-pass slot `[0xce8bb4]`; our views coexist, so it is per view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FfxState {
    /// The WorldFrame pair (`0x6cc130` CFFXGlow → `[0xb4b350]`, `0x6cc690` CFFXDeath →
    /// `[0xb4b39c]`, built at `0x481c46`): the ghost death combine ([`FfxDeathFade`]) and the
    /// drunk/underwater haze ([`FfxHazeMix`]) of the live player. The world view only.
    Player,
    /// The glue pair (`[0xb414c4]` glow, `[0xb41468]` death, built at `0x46a723`/`0x46a752`): the
    /// death combine when the selected roster row is a ghost ([`GlueFfx::death`]), and no haze.
    Glue,
    /// Neither: a bake standing in for a UI model widget. The reference's pass runs inside the
    /// WorldFrame paint (`0x6cd890`/`0x6cda70`, at `0x48350e`/`0x48379d` in `0x483460`) on its own
    /// targets (`0xce8b5c`, `0xce8ae8`, `0xce8b98`), and its portrait bake `0x524f60` copies its
    /// pixels out of the framebuffer (`0x58acd0`/`0x449bf0`), so a bake never carries the pass.
    None,
}

impl FfxGlow {
    /// The world camera: the zone glow and the live player's pass pair.
    pub const WORLD: Self = Self {
        gain_scale: 1.0,
        state: FfxState::Player,
    };
    /// The glue screens' scene: the zone glow and the glue pair, around the glue paint (`0x46fad3`
    /// calls `0x6cd890`, `0x46fae0` jumps to `0x6cda70`); GlueXML composites after, untinted.
    pub const GLUE_SCENE: Self = Self {
        gain_scale: 1.0,
        state: FfxState::Glue,
    };
    /// A portrait bake: the zone glow at world parity and no player-state pass, so a ghost's
    /// portrait keeps its living face and a drunk player's unit frame stays sharp.
    pub const BOOTH: Self = Self {
        gain_scale: 1.0,
        state: FfxState::None,
    };
    /// Decode only, no glow: a bake standing in for a UI model widget (a `<PlayerModel>` pane),
    /// which the reference paints after the WorldFrame's FFX pass, so it never glows.
    pub const UI_PANE: Self = Self {
        gain_scale: 0.0,
        state: FfxState::None,
    };
}

impl Default for FfxGlow {
    fn default() -> Self {
        Self::WORLD
    }
}

/// The player-UI camera's ground: the combine of the world view `source` names, drawn first in
/// its UI pass as the gamma byte. That world view runs no combine of
/// its own, and runs `CameraOutputMode::Skip`, since its main texture is read unflipped.
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
pub struct FfxBackdrop {
    /// The world camera drawing this frame (main-world entity). While `None` the camera's own
    /// clear is the ground; the component stays so its pipelines compile before any world exists.
    pub source: Option<Entity>,
}

/// The glow weight `w`: the live `LightParams.glow`, synced every frame by [`sync_gain`].
#[derive(Resource, Clone)]
pub struct FfxGlowGain(pub f32);

/// The FFXDeath gate, the combine's `y`: `1.0` while the player is a released ghost (`0x5de9c0`,
/// off `PLAYER_FLAGS_GHOST`), else `0.0`, with no ramp on either edge.
#[derive(Resource, Clone, Default)]
pub struct FfxDeathFade(pub f32);

/// The glue screens' FFX state. The select build's tail (`0x472fba`–`0x473007`) forks on the
/// selected record's `CHARSELECT+0xfc & 0x2000` (`0x472fd9`), installs (`0x6cde60`) the glue
/// death or glow pass and pins `LightParams.glow` (`[0x6d48b0()+0x110]`) to a constant: both
/// combines' blur² weight, which the death combine packs as its alpha byte (`0x6cb930`).
#[derive(Resource, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum GlueFfx {
    /// A glue model widget is shown and nothing has re-pinned it: login, create, and a character
    /// select with no characters. The widget's OnShow (`0x46fa60`) installs the glue glow pass and
    /// pins `*0x8380b4` = 0.30, once on show, so the select build's pins hold.
    #[default]
    Shown,
    /// Character select, a living selection: the glue glow pass, pinned at `*0x838574` = 0.40
    /// (`0x472ff7`/`0x473002`).
    SelectLiving,
    /// Character select, a ghost selection: the glue death pass, pinned at `*0x838570` = 0.15
    /// (`0x472fde`/`0x472fe9`).
    SelectGhost,
}

impl GlueFfx {
    /// The glue pair's death gate, with no ramp. The swap re-runs on every selection, an
    /// already-built record included (`0x472a6d`), so the scene changes on the click's frame.
    fn death(self) -> f32 {
        match self {
            Self::SelectGhost => 1.0,
            Self::Shown | Self::SelectLiving => 0.0,
        }
    }

    /// `LightParams.glow` (`0xce9c70`) on a glue screen; the death combine packs it into its alpha
    /// byte (`0x6cb9ec`: 0.15 → 38, 0.40 → 102). Deviation: kept a float where every reference
    /// pack (`0x6cb0e2`/`0x6cb557`/`0x6cb9e1`) quantizes it to a byte, because the two differ by
    /// under a thousandth (38/255 against 0.15).
    fn glow(self) -> f32 {
        match self {
            // `*0x8380b4`, the widget's OnShow pin.
            Self::Shown => 0.30,
            // `*0x838574`, the select build's living arm.
            Self::SelectLiving => 0.40,
            // `*0x838570`, the select build's ghost arm.
            Self::SelectGhost => 0.15,
        }
    }
}

/// The haze mix `z`, the combine's cross-fade toward the blur, packed per frame from the player
/// (`0x6cb134`/`0x6cb599`): `max(min(drunkByte, 100)/100, submerged ? 84/255 : 0)`, submerged
/// being the camera eye in any liquid (`0x672470` not `0xf`).
#[derive(Resource, Clone, Default)]
pub struct FfxHazeMix(pub f32);

/// The reference's byte 84 in the colour `z` lane while the eye is submerged, drunk or sober.
const HAZE_SUBMERGED_FLOOR: f32 = 84.0 / 255.0;

/// [`FfxHazeMix`] from the drunk byte (`PLAYER_BYTES_3` byte 1) and the eye's submersion; both
/// are empty off-world, so a glue screen never hazes.
fn sync_haze(
    viewer: Res<crate::view::Viewer>,
    underwater: Option<Res<crate::liquid::Underwater>>,
    mut haze: ResMut<FfxHazeMix>,
) {
    let drunk = viewer.drunk;
    let submerged = if underwater.is_some_and(|u| u.0 != benilla_formats::Submersion::Dry) {
        HAZE_SUBMERGED_FLOOR
    } else {
        0.0
    };
    let target = drunk.max(submerged);
    if haze.0 != target {
        haze.0 = target;
    }
}

/// The underwater screen warp's inputs. With the camera eye in liquid, `CFFXGlow::Render`
/// (`0x6cc630`) walks a second pass list ending in FFXGlowWave (`0x6cb1f0`, render `0x6cb310`),
/// which displaces the combine's two samples through a sine bump map. The guard compares the
/// eye-liquid byte `[0xc7f288]` with `0xf` (dry) alone (`0x6cc644`): lava and slime warp too.
#[derive(Resource, Clone, Copy, Default)]
pub struct FfxWave {
    /// `(t mod 3174)/3174`, the u-axis phase.
    phase1: f32,
    /// `(t mod 2805)/2805`, the v-axis phase; the two rejoin only every ~49 minutes.
    phase2: f32,
    /// The swap: in-world (`0x467d00`) and the eye in liquid (`0x672470` not `0xf`), with no ramp.
    active: bool,
}

/// The phase periods (`[0xce89c4]`, `[0xce89c8]`), moduli of an integer millisecond clock.
const WAVE_PERIOD_MS: [u64; 2] = [3174, 2805];

/// The wave LUT is 128×128 texels (`[0xce89a0]`'s descriptor).
pub(crate) const WAVE_LUT_EDGE: u32 = 128;

/// The reference's glow-wave LUT (`0x6cbea0`), a displacement map: `du = sin(2πx/128)`,
/// `dv = sin(2πy/128)`, stored as the unsigned pack `(s·0.5 + 0.5)·255` that the default Direct3D
/// path's `ps_2_0` permutation biases back; `as u8` truncates toward zero, as `__ftol` does.
pub(crate) fn wave_lut_texels() -> Vec<u8> {
    let pack = |s: f32| ((s * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
    let sine = |i: u32| (std::f32::consts::TAU * i as f32 / WAVE_LUT_EDGE as f32).sin();
    let mut texels = Vec::with_capacity((WAVE_LUT_EDGE * WAVE_LUT_EDGE * 2) as usize);
    for y in 0..WAVE_LUT_EDGE {
        let dv = pack(sine(y));
        for x in 0..WAVE_LUT_EDGE {
            texels.push(pack(sine(x)));
            texels.push(dv);
        }
    }
    texels
}

/// [`FfxWave`] from the swap guard's inputs and the render's clock (`OsGetAsyncTimeMs`,
/// `0x6cb43a`); the phases run free, armed or not, as the reference's do, so a dive never restarts
/// them.
fn sync_wave(
    time: Res<Time>,
    live: Res<crate::schedule::WorldLive>,
    underwater: Option<Res<crate::liquid::Underwater>>,
    mut wave: ResMut<FfxWave>,
    mut last_dump: Local<Option<u32>>,
) {
    let ms = (time.elapsed_secs_f64() * 1000.0) as u64;
    wave.phase1 = (ms % WAVE_PERIOD_MS[0]) as f32 / WAVE_PERIOD_MS[0] as f32;
    wave.phase2 = (ms % WAVE_PERIOD_MS[1]) as f32 / WAVE_PERIOD_MS[1] as f32;
    let verdict = underwater.map(|u| u.0);
    wave.active = live.0 && verdict.is_some_and(|v| v.any());

    // `WOW_WAVE_DUMP`: a 1 Hz line, warping or not, naming why not.
    if std::env::var_os("WOW_WAVE_DUMP").is_some() {
        let sec = time.elapsed_secs() as u32;
        if last_dump.replace(sec) != Some(sec) {
            let why = match (live.0, verdict) {
                (false, _) => "off — not in world (the reference's 0x467d00 half of the guard)",
                (_, None) => "off — no submersion verdict resolved yet",
                (_, Some(benilla_formats::Submersion::Dry)) => "off — the eye is dry",
                (_, Some(_)) => "WARPING",
            };
            info!(
                "glow-wave: {why} — verdict {verdict:?} phase {:.3}/{:.3} (periods {} / {} ms)",
                wave.phase1, wave.phase2, WAVE_PERIOD_MS[0], WAVE_PERIOD_MS[1]
            );
        }
    }
}

/// Whether a view draws the warped combine: the WorldFrame pair (the glue pair reaches the same
/// render, but the guard's in-world half `0x467d00` never trips there), [`FfxWave::active`], and
/// no ghost, whose `CFFXDeath::Render` (`0x6cdf20`) has one list and no guard.
fn wave_armed(state: FfxState, wave: FfxWave, death: f32) -> bool {
    matches!(state, FfxState::Player) && wave.active && death == 0.0
}

/// [`FfxGlowGain`]: the zone's `LightParams.glow` in world (its default 0.5 while no lighting
/// exists), and the glue screen's pin off world ([`GlueFfx::glow`]).
fn sync_gain(
    lighting: Option<Res<crate::lighting::WowLighting>>,
    live: Res<crate::schedule::WorldLive>,
    glue: Res<GlueFfx>,
    mut gain: ResMut<FfxGlowGain>,
) {
    let target = if live.0 {
        lighting.map_or(0.5, |l| l.glow)
    } else {
        // A glue screen always pins the weight; it never falls back to the 0.5 default.
        glue.glow()
    };
    if gain.0 != target {
        gain.0 = target;
    }
}

/// Adds [`FfxGlow`] to any world camera without it: its combine is the frame's one gamma decode,
/// without which the frame presents over-bright.
fn ensure_ffx_glow(
    mut commands: Commands,
    cam: Query<Entity, (With<WorldCamera>, Without<FfxGlow>)>,
    mut with: Query<&mut FfxGlow>,
) {
    // `WOW_NO_FFX`, a perf lever: every camera drops to `UI_PANE`, no blur and no glow add, but
    // keeps the combine, which is the frame's decode and the UI camera's ground.
    static NO_FFX: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *NO_FFX.get_or_init(|| std::env::var_os("WOW_NO_FFX").is_some()) {
        for mut glow in &mut with {
            if glow.gain_scale != 0.0 || glow.state != FfxState::None {
                *glow = FfxGlow::UI_PANE;
            }
        }
        for e in &cam {
            commands.entity(e).insert(FfxGlow::UI_PANE);
        }
        return;
    }
    for e in &cam {
        commands.entity(e).insert(FfxGlow::WORLD);
    }
}

/// `WOW_DITHER=1` arms the combine's deband dither (`ffx.lane.w`). Deviation, opt-in, off by
/// default: the reference's 8-bit framebuffer is undithered, but a smooth gradient under slow
/// motion steps visibly at 1/255. Bevy's own dither never runs under `Tonemapping::None`.
fn dither_armed() -> f32 {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    match *ON.get_or_init(|| std::env::var_os("WOW_DITHER").is_some()) {
        true => 1.0,
        false => 0.0,
    }
}

/// One view's combine uniform: the zone glow × [`FfxGlow::gain_scale`], the death gate and haze of
/// its pass pair, the dither arm, then the wave phases, written every frame so the clock runs free.
fn combine_uniform(
    zone_gain: f32,
    world: FfxPassState,
    glue: FfxPassState,
    glow: &FfxGlow,
    wave: FfxWave,
) -> [f32; 8] {
    let feed = match glow.state {
        FfxState::Player => world,
        FfxState::Glue => glue,
        FfxState::None => FfxPassState::INERT,
    };
    [
        zone_gain * glow.gain_scale,
        feed.death,
        feed.haze,
        dither_armed(),
        wave.phase1,
        wave.phase2,
        0.0,
        0.0,
    ]
}

/// One pass pair's live death gate and haze mix.
#[derive(Clone, Copy)]
struct FfxPassState {
    death: f32,
    haze: f32,
}

impl FfxPassState {
    fn world(death: f32, haze: f32) -> Self {
        Self { death, haze }
    }

    /// Death only: the reference packs the haze into `primary.z` of the glow combine (`0x6cb020`)
    /// from the in-world player's state, which a glue screen does not have.
    fn glue(death: f32) -> Self {
        Self { death, haze: 0.0 }
    }

    /// What a view running neither pair reads: a bake.
    const INERT: Self = Self {
        death: 0.0,
        haze: 0.0,
    };
}

/// For gfx: one view's combine uniform and whether its underwater warp is armed.
pub(crate) fn gfx_live_combine(world: &World, glow: &FfxGlow) -> ([f32; 8], bool) {
    let death = world.resource::<FfxDeathFade>().0;
    let wave = *world.resource::<FfxWave>();
    let uniform = combine_uniform(
        world.resource::<FfxGlowGain>().0,
        FfxPassState::world(death, world.resource::<FfxHazeMix>().0),
        FfxPassState::glue(world.resource::<GlueFfx>().death()),
        glow,
        wave,
    );
    (uniform, wave_armed(glow.state, wave, death))
}

pub struct FfxGlowPlugin;

impl Plugin for FfxGlowPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FfxGlowGain(0.647)) // overwritten by sync_gain from zone data
            .init_resource::<FfxDeathFade>()
            .init_resource::<GlueFfx>()
            .init_resource::<FfxHazeMix>()
            .init_resource::<FfxWave>()
            .add_plugins((
                ExtractComponentPlugin::<FfxGlow>::default(),
                ExtractComponentPlugin::<FfxBackdrop>::default(),
            ))
            .add_systems(
                Update,
                (
                    // The gain reads the resolved lighting; the haze and the wave read the
                    // submersion verdict, and would flip a frame late unordered.
                    sync_gain.in_set(crate::lighting::LightingConsumeSet),
                    (sync_haze, sync_wave).after(crate::liquid::SubmersionVerdict),
                    ensure_ffx_glow,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The world pair, live: a released ghost, sober.
    fn ghost_world() -> FfxPassState {
        FfxPassState::world(1.0, 0.0)
    }
    fn living_glue() -> FfxPassState {
        FfxPassState::glue(0.0)
    }

    /// Checked as one table, so a new preset cannot quietly join the wrong side.
    #[test]
    fn only_the_two_screen_views_run_a_pass_pair() {
        for (name, view) in [("BOOTH", FfxGlow::BOOTH), ("UI_PANE", FfxGlow::UI_PANE)] {
            assert_eq!(
                view.state,
                FfxState::None,
                "{name} is a bake standing in for a UI widget — it runs no pass pair"
            );
        }
        assert_eq!(FfxGlow::WORLD.state, FfxState::Player);
        assert_eq!(FfxGlow::GLUE_SCENE.state, FfxState::Glue);
        // A bake still glows like the world.
        assert_eq!(FfxGlow::BOOTH.gain_scale, 1.0);
        assert_eq!(FfxGlow::GLUE_SCENE.gain_scale, 1.0);
        assert_eq!(FfxGlow::UI_PANE.gain_scale, 0.0);
    }

    /// Death-combined, a ghost's portrait would bake steel-blue; the zone glow still reaches it.
    #[test]
    fn a_ghosts_bake_is_not_death_combined() {
        let zone = 0.5;
        assert_eq!(
            combine_uniform(
                zone,
                ghost_world(),
                living_glue(),
                &FfxGlow::WORLD,
                FfxWave::default()
            ),
            [0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            combine_uniform(
                zone,
                ghost_world(),
                living_glue(),
                &FfxGlow::BOOTH,
                FfxWave::default()
            ),
            [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "the booth keeps the zone glow and drops the death gate"
        );
        assert_eq!(
            combine_uniform(
                zone,
                ghost_world(),
                living_glue(),
                &FfxGlow::UI_PANE,
                FfxWave::default()
            ),
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_drunk_players_bake_stays_sharp() {
        let (zone, drunk) = (0.5, FfxPassState::world(0.0, 1.0));
        assert_eq!(
            combine_uniform(
                zone,
                drunk,
                living_glue(),
                &FfxGlow::WORLD,
                FfxWave::default()
            ),
            [0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            combine_uniform(
                zone,
                drunk,
                living_glue(),
                &FfxGlow::BOOTH,
                FfxWave::default()
            ),
            [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn each_pass_pair_reaches_only_its_own_view() {
        let zone = 0.5;
        let ghost_glue = FfxPassState::glue(1.0);
        let alive_world = FfxPassState::world(0.0, 0.0);
        assert_eq!(
            combine_uniform(
                zone,
                alive_world,
                ghost_glue,
                &FfxGlow::GLUE_SCENE,
                FfxWave::default()
            ),
            [0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "a ghost roster row death-combines the glue scene"
        );
        assert_eq!(
            combine_uniform(
                zone,
                alive_world,
                ghost_glue,
                &FfxGlow::WORLD,
                FfxWave::default()
            ),
            [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "…and never the world view, whose own player is alive"
        );
        assert_eq!(
            combine_uniform(
                zone,
                ghost_world(),
                living_glue(),
                &FfxGlow::GLUE_SCENE,
                FfxWave::default()
            ),
            [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "and a world ghost never reaches the glue view"
        );
    }

    /// Against a fully hazed world, the state that would leak if the two pairs were ever merged.
    #[test]
    fn the_glue_pair_never_hazes() {
        let drunk_world = FfxPassState::world(0.0, 1.0);
        assert_eq!(
            combine_uniform(
                0.5,
                drunk_world,
                FfxPassState::glue(1.0),
                &FfxGlow::GLUE_SCENE,
                FfxWave::default()
            ),
            [0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "the glue view death-combines on its own gate and never hazes"
        );
    }

    /// The swap's guard is `0x467d00() != 0 && 0x672470() != 0xf` (`0x6cc630`), and a ghost runs
    /// the death pass in place of the glow pass.
    #[test]
    fn the_warp_arms_only_for_a_living_submerged_world_view() {
        let wet = FfxWave {
            phase1: 0.0,
            phase2: 0.0,
            active: true,
        };
        let dry = FfxWave::default();
        assert!(
            wave_armed(FfxState::Player, wet, 0.0),
            "in-world, submerged, alive — the reference walks its second list"
        );
        assert!(
            !wave_armed(FfxState::Player, dry, 0.0),
            "a dry camera keeps the first list"
        );
        assert!(
            !wave_armed(FfxState::Player, wet, 1.0),
            "a ghost runs CFFXDeath, which owns one list and no guard"
        );
        assert!(
            !wave_armed(FfxState::Glue, wet, 0.0),
            "the glue pair reaches the same Render, but 0x467d00 gates it out of the swap"
        );
        assert!(
            !wave_armed(FfxState::None, wet, 0.0),
            "a bake runs neither pair"
        );
    }

    /// Dry, the uniform's first row is the plain combine's and the node binds the dry pipeline.
    #[test]
    fn a_dry_frame_pays_nothing_for_the_warp() {
        let live = FfxWave {
            phase1: 0.4,
            phase2: 0.7,
            active: false,
        };
        let u = combine_uniform(
            0.5,
            FfxPassState::world(0.0, 0.0),
            living_glue(),
            &FfxGlow::WORLD,
            live,
        );
        assert_eq!(
            u[..4],
            [0.5, 0.0, 0.0, 0.0],
            "the combine lane is exactly what it was before the wave lane existed"
        );
        assert!(
            !wave_armed(FfxState::Player, live, 0.0),
            "and the node binds the unwarped pipeline"
        );
    }

    #[test]
    fn the_two_phases_run_independently_off_one_clock() {
        let phase = |ms: u64, i: usize| (ms % WAVE_PERIOD_MS[i]) as f32 / WAVE_PERIOD_MS[i] as f32;
        assert_eq!(WAVE_PERIOD_MS, [3174, 2805]);
        // Each wraps on its own period and nowhere else.
        assert!(phase(3173, 0) > 0.999 && phase(3174, 0) == 0.0);
        assert!(phase(2804, 1) > 0.999 && phase(2805, 1) == 0.0);
        // At one phase's wrap the other is mid-stride.
        assert!(phase(3174, 1) > 0.13 && phase(3174, 1) < 0.14);
        // The joint period: lcm(3174, 2805) ms ≈ 49 min 28 s.
        let gcd = |mut a: u64, mut b: u64| {
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a
        };
        let joint =
            WAVE_PERIOD_MS[0] / gcd(WAVE_PERIOD_MS[0], WAVE_PERIOD_MS[1]) * WAVE_PERIOD_MS[1];
        assert_eq!(joint, 2_967_690);
    }

    #[test]
    fn the_wave_lut_is_one_sine_per_axis() {
        let lut = wave_lut_texels();
        let edge = WAVE_LUT_EDGE as usize;
        assert_eq!(lut.len(), edge * edge * 2);
        let at = |x: usize, y: usize| {
            let i = (y * edge + x) * 2;
            (lut[i], lut[i + 1])
        };
        for x in 0..edge {
            for y in [0usize, 37, 91, 127] {
                assert_eq!(at(x, y).0, at(x, 0).0, "du must not vary down the texture");
                assert_eq!(at(x, y).1, at(0, y).1, "dv must not vary across it");
            }
        }
        // Biased back as the shipped `ps_2_0` permutation does, each texel is its sine within one
        // 8-bit step, the reference's own quantization.
        for i in 0..edge {
            let want = (std::f32::consts::TAU * i as f32 / edge as f32).sin();
            let got = (at(i, i).0 as f32 / 255.0 - 0.5) * 2.0;
            assert!(
                (got - want).abs() <= 2.0 / 255.0,
                "texel {i}: {got} vs sin {want}"
            );
        }
    }

    /// The reference packs `(s·0.5 + 0.5)·255`, clamps, and truncates by `__ftol`: `sin = 0` packs
    /// to 127, not 128, a half-step bias the reference ships too.
    #[test]
    fn the_wave_pack_truncates_like_ftol() {
        let lut = wave_lut_texels();
        // x = 0 and x = 64 are the sine's two zeros; both truncate down.
        assert_eq!(lut[0], 127, "sin(0) = 0 → trunc(127.5) = 127");
        assert_eq!(lut[64 * 2], 127, "sin(π) ≈ 0 → 127");
        // The quarter points saturate the ends of the range.
        assert_eq!(lut[32 * 2], 255, "sin(π/2) = 1 → 255");
        assert_eq!(lut[96 * 2], 0, "sin(3π/2) = −1 → 0");
    }

    /// The default is the shown, unselected glue screen (login, create, an empty account).
    #[test]
    fn the_glue_screens_pin_three_verified_glow_weights() {
        assert_eq!(GlueFfx::default(), GlueFfx::Shown);
        assert_eq!(
            GlueFfx::Shown.glow(),
            0.30,
            "*0x8380b4, the widget's OnShow"
        );
        assert_eq!(GlueFfx::SelectLiving.glow(), 0.40, "*0x838574");
        assert_eq!(GlueFfx::SelectGhost.glow(), 0.15, "*0x838570");
        // Only the ghost arm installs the death pass.
        assert_eq!(GlueFfx::SelectGhost.death(), 1.0);
        assert_eq!(GlueFfx::Shown.death(), 0.0);
        assert_eq!(GlueFfx::SelectLiving.death(), 0.0);
        // The packed byte (`0x6cb9ec`: ×255, +512, `fstp`, `shr 14`): 38, the ghost's death
        // combine alpha, and 102, the living glow pass's `w`.
        let alpha_byte = |glow: f32| ((glow * 255.0 + 512.0).to_bits() >> 14) & 0xff;
        assert_eq!(alpha_byte(GlueFfx::SelectGhost.glow()), 38);
        assert_eq!(alpha_byte(GlueFfx::SelectLiving.glow()), 102);
    }
}
