#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! This module is separated into its own crate to enable simple dynamic linking for Bevy, and should not be used directly

/// `use bevy::prelude::*;` to import common components, bundles, and plugins.
pub mod prelude;

mod cut;
mod default_plugins;
pub use default_plugins::*;

pub use bevy_animation as animation;
pub use bevy_app as app;
pub use bevy_asset as asset;
pub use bevy_camera as camera;
pub use bevy_color as color;
pub use bevy_core_pipeline as core_pipeline;
pub use bevy_diagnostic as diagnostic;
pub use bevy_ecs as ecs;
pub use bevy_gizmos as gizmos;
pub use bevy_image as image;
pub use bevy_input as input;
pub use bevy_light as light;
pub use bevy_log as log;
pub use bevy_math as math;
pub use bevy_mesh as mesh;
pub use bevy_pbr as pbr;
pub use bevy_platform as platform;
pub use bevy_ptr as ptr;
pub use bevy_reflect as reflect;
pub use bevy_render as render;
pub use bevy_scene as scene;
pub use bevy_shader as shader;
pub use bevy_sprite as sprite;
pub use bevy_sprite_render as sprite_render;
pub use bevy_state as state;
pub use bevy_tasks as tasks;
pub use bevy_text as text;
pub use bevy_time as time;
pub use bevy_transform as transform;
pub use bevy_ui as ui;
pub use bevy_ui_render as ui_render;
pub use bevy_utils as utils;
pub use bevy_window as window;
pub use cut::anti_alias;
pub use cut::gilrs;
pub use cut::gltf;
