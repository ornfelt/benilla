//! The plugins of the crates cut from benilla's Bevy (`bevy_anti_alias`, `bevy_gilrs`,
//! `bevy_gltf`). benilla names each only to disable it in [`DefaultPlugins`](crate::DefaultPlugins)
//! (`PluginGroupBuilder::disable` panics on a plugin the group lacks), so each keeps its path and
//! its slot in the group and builds nothing.

/// `bevy_anti_alias`'s plugin, cut: no FXAA, SMAA, TAA or CAS.
#[cfg(feature = "bevy_anti_alias")]
pub mod anti_alias {
    use bevy_app::{App, Plugin};

    /// Adds nothing; benilla disables it.
    #[derive(Default)]
    pub struct AntiAliasPlugin;

    impl Plugin for AntiAliasPlugin {
        fn build(&self, _app: &mut App) {}
    }
}

/// `bevy_gilrs`'s plugin, cut: no gamepad backend.
#[cfg(feature = "bevy_gilrs")]
pub mod gilrs {
    use bevy_app::{App, Plugin};

    /// Adds nothing; benilla disables it.
    #[derive(Default)]
    pub struct GilrsPlugin;

    impl Plugin for GilrsPlugin {
        fn build(&self, _app: &mut App) {}
    }
}

/// `bevy_gltf`'s plugin, cut: no glTF loader.
#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    use bevy_app::{App, Plugin};

    /// Adds nothing; benilla disables it.
    #[derive(Default)]
    pub struct GltfPlugin;

    impl Plugin for GltfPlugin {
        fn build(&self, _app: &mut App) {}
    }
}
