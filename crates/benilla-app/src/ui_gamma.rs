//! The player-UI gamma lane's one decode. [`ui_quad.wgsl`](shaders/ui_quad.wgsl) composites the
//! UI in gamma bytes, as the reference's fixed-function device does into its 8-bit backbuffer:
//! blends are gamma arithmetic clamped at each write, and `alphaMode="ADD"` is `dst + texel·α`
//! (EGxBlend 3, `glBlendFunc(GL_SRC_ALPHA, GL_ONE)`; factor tables `0x85c1f8`/`0x85c224`). The
//! gfx UI pass (`benilla_gfx::ui`) decodes the finished image to linear once, whose sRGB write
//! re-encodes the client's byte. Without it the UI presents about 2.2 times too bright.
//! [`UiGammaLane`] marks the one camera it runs for.
//!
//! Bevy UI (the glue and loading screens) converts in its own gamma shaders and lands on this
//! camera, which [`crate::ui_pass`] makes `IsDefaultUiCamera`.

use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};

/// Marks the camera whose target holds the gamma-composited UI, with the [`DisplayGamma`]
/// exponent the decode applies.
#[derive(Component, Clone, Copy, ExtractComponent)]
pub(crate) struct UiGammaLane {
    /// The clamped `gamma` CVar; `1.0` is the identity ramp.
    pub(crate) gamma: f32,
}

impl Default for UiGammaLane {
    fn default() -> Self {
        Self {
            gamma: DEFAULT_GAMMA,
        }
    }
}

/// The `gamma` CVar (registered at `0x402d70`, name `0x82e924`, default `"1.0"` at `0x82e92c`,
/// flags 0). The reference's callback `0x4034d0` hands `SetDeviceGammaRamp` the ramp
/// `__ftol(pow(i / 255, gamma) · 65535)` (`0x591680`); there is no other whole-frame grade and
/// no `Brightness` or `Contrast` CVar.
///
/// Deviation: the reference skips that upload when windowed (`byte[dev+0x20b]` is `gxWindow`), and
/// every benilla mode is windowed, so the same curve applies in the compositor, where the slider
/// can move pixels: the decode raises the gamma byte the RAMDAC would have read to `gamma` first,
/// the continuous form of the 256-entry ramp.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct DisplayGamma(pub(crate) f32);

/// The registered `"1.0"`: the identity ramp, at which the pass changes no byte.
pub(crate) const DEFAULT_GAMMA: f32 = 1.0;

/// Deviation: the reference never clamps (`SetGamma` `0x4891f0` writes `-4.000000` for 5), but its
/// ramp is a fullscreen OS call and ours is the image, where an extreme exponent blacks out the
/// panel that would undo it. The range spans the stock slider's `[0.5, 1.5]` four times over; the
/// CVar keeps the value written and only its consumers clamp.
pub(crate) const GAMMA_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;

impl Default for DisplayGamma {
    fn default() -> Self {
        Self(DEFAULT_GAMMA)
    }
}

/// Copies the clamped gamma onto the lane camera when it changes.
fn stamp_lane_gamma(gamma: Res<DisplayGamma>, mut lanes: Query<&mut UiGammaLane>) {
    if !gamma.is_changed() {
        return;
    }
    for mut lane in &mut lanes {
        lane.gamma = gamma.0.clamp(*GAMMA_RANGE.start(), *GAMMA_RANGE.end());
    }
}

pub(crate) struct UiGammaPlugin;

/// The `gamma` CVar's change callback, clamped to [`GAMMA_RANGE`].
pub(crate) fn on_cvar(ev: On<crate::cvars::CvarChanged>, mut gamma: ResMut<DisplayGamma>) {
    if ev.is(benilla_ui::script::CVAR_GAMMA) {
        gamma.0 = ev.num().clamp(*GAMMA_RANGE.start(), *GAMMA_RANGE.end());
    }
}

impl Plugin for UiGammaPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_cvar);
        app.init_resource::<DisplayGamma>()
            .add_plugins(ExtractComponentPlugin::<UiGammaLane>::default())
            .add_systems(Update, stamp_lane_gamma);
    }
}
