#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

/// The basic components of the transform crate
pub mod components;

/// Transform related plugins
#[cfg(feature = "bevy-support")]
pub mod plugins;

/// Systems responsible for transform propagation
#[cfg(feature = "bevy-support")]
pub mod systems;

/// The transform prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::components::*;

    #[cfg(feature = "bevy-support")]
    #[doc(hidden)]
    pub use crate::{
        plugins::{TransformPlugin, TransformSystems},
        systems::StaticTransformOptimizations,
    };
}

#[cfg(feature = "bevy-support")]
pub use prelude::{StaticTransformOptimizations, TransformPlugin, TransformSystems};
