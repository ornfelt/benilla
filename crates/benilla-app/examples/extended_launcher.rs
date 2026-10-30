//! `extended_launcher`: the client with a plugin of its own on top, started through
//! [`benilla_app::run_with`] as a crate built on benilla starts it. A feature 1.12.1 does not have
//! lives in a crate like this one, never in benilla.
//!
//! ```text
//! cargo run -p benilla-app --example extended_launcher
//! ```
//!
//! The banner reads `benilla build … · extended`, and the plugin logs one line at `Startup`: the
//! `AddOns/` folder in [`benilla_app::config_dir`], where an addon the crate ships is written.
//!
//! A crate of its own copies the `benilla` launcher (`crates/benilla`): a build script that stamps
//! its commit, a `main` like this one that reads the stamp back as `crates/benilla/src/main.rs`
//! does, and a manifest that takes benilla as the workspace does:
//!
//! ```toml
//! [features]
//! default = ["dev"]
//! # The instruments; the player build is `--no-default-features`.
//! dev = ["benilla-app/dev"]
//!
//! [dependencies]
//! # `default-features = false`, or `--no-default-features` still builds benilla-app with `dev`.
//! benilla-app = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy", default-features = false }
//! # benilla's version, adding no features: bevy's defaults bring its audio stack back.
//! bevy = { version = "0.18.1", default-features = false }
//!
//! [build-dependencies]
//! benilla-buildstamp = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//!
//! # Cargo reads `[patch]` and `[profile]` from the root manifest only, so benilla's are repeated:
//! # without the patches the mixer is not benilla's, and Bevy and its plugins
//! # are crates.io's, not the trimmed copies under `third_party/` (`third_party/bevy/BENILLA.md`).
//! [patch.crates-io]
//! kira = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_animation = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_app = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_asset = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_asset_macros = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_camera = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_color = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_core_pipeline = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_derive = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_diagnostic = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_dylib = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_ecs = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_ecs_macros = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_encase_derive = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_gizmos = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_gizmos_macros = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_image = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_input = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_internal = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_light = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_log = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_macro_utils = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_math = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_mesh = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_pbr = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_platform = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_ptr = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_reflect = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_reflect_derive = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_render = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_render_macros = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_scene = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_shader = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_sprite = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_sprite_render = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_state = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_state_macros = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_tasks = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_text = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_time = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_transform = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_ui = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_ui_render = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_utils = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_window = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_winit = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! avian3d = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! avian_derive = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_egui = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_heavy = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//! bevy_transform_interpolation = { git = "https://github.com/ornfelt/benilla", branch = "trimmed-bevy" }
//!
//! [profile.dev]
//! opt-level = 1
//!
//! [profile.dev.package."*"]
//! opt-level = 3
//! ```
//!
//! ```text
//! // build.rs
//! fn main() {
//!     benilla_buildstamp::emit();
//! }
//! ```
//!
//! Its `Cargo.lock` starts as a copy of benilla's: cargo reads only the root's lockfile too, and
//! without it resolves every crate to its newest compatible version, not the one benilla is built
//! with. A dev build keeps its `WoW` link, `benilla-config/` and `.probe-identity` in its own
//! folder, beside that `Cargo.lock`, as benilla's does.

use benilla_app::{AppExit, BuildId};
use bevy::prelude::*;

/// The crate's own plugin: one line at `Startup`, to show it runs inside the client and where its
/// addons go.
struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, || {
            let addons = benilla_app::config_dir().map(|dir| dir.join("AddOns"));
            info!("a plugin on top of benilla is running; its addons go in {addons:?}");
        });
    }
}

fn main() -> AppExit {
    // A launcher of its own passes the stamp its build script made; an example has no build
    // script, so this names the version and profile alone.
    let build = BuildId {
        version: env!("CARGO_PKG_VERSION"),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        ..Default::default()
    };
    benilla_app::run_with(build, |app| {
        app.add_plugins(HelloPlugin);
    })
}
