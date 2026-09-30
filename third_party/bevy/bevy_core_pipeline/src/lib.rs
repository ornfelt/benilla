#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

pub mod core_2d;
pub mod core_3d;
pub mod oit;
pub mod prepass;
pub mod tonemapping;

pub use skybox::Skybox;

mod skybox;

use crate::{core_2d::Core2dPlugin, core_3d::Core3dPlugin, tonemapping::TonemappingPlugin};
use bevy_app::{App, Plugin};
use oit::OrderIndependentTransparencyPlugin;

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((Core2dPlugin, Core3dPlugin))
            .add_plugins((TonemappingPlugin, OrderIndependentTransparencyPlugin));
    }
}
