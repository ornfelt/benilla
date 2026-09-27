//! The UI lane: a `Camera2d` on the window that composites in gamma bytes, as the reference's
//! fixed-function UI does into its 8-bit backbuffer (benilla-app's `ui_pass.rs`, `ui_gamma.rs`).
//! Its draws land in the lane's byte target ([`crate::target::UiTarget`]), grounded by the world
//! view it claims, whose FFXGlow combine writes premultiplied gamma there instead of into the
//! scene target; the lane's decode then writes the scene target, which the present encodes.

use bevy::prelude::*;

/// A `Camera2d` drawing the UI lane: `gamma` is the display-gamma ramp its decode applies, and
/// `backdrop` the world camera whose combine is its first draw (`FfxBackdrop::source`), drawn
/// through gfx wherever that camera targets.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GfxUiLane {
    pub gamma: f32,
    pub backdrop: Option<Entity>,
}
