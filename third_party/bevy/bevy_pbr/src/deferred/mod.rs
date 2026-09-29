use bevy_app::prelude::*;
use bevy_core_pipeline::prepass::DeferredPrepass;
use bevy_ecs::prelude::*;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};

pub struct DeferredPbrLightingPlugin;

pub const DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID: u8 = 1;

/// Component with a `depth_id` for specifying which corresponding materials should be rendered by this specific PBR deferred lighting pass.
///
/// Will be automatically added to entities with the [`DeferredPrepass`] component that don't already have a [`PbrDeferredLightingDepthId`].
#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct PbrDeferredLightingDepthId {
    depth_id: u32,
}

impl PbrDeferredLightingDepthId {
    pub fn new(value: u8) -> PbrDeferredLightingDepthId {
        PbrDeferredLightingDepthId {
            depth_id: value as u32,
        }
    }

    pub fn set(&mut self, value: u8) {
        self.depth_id = value as u32;
    }

    pub fn get(&self) -> u8 {
        self.depth_id as u8
    }
}

impl Default for PbrDeferredLightingDepthId {
    fn default() -> Self {
        PbrDeferredLightingDepthId {
            depth_id: DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID as u32,
        }
    }
}

impl Plugin for DeferredPbrLightingPlugin {
    fn build(&self, app: &mut App) {
        // The uniform plugin and the lighting pass only reached the RenderApp.
        app.add_plugins(ExtractComponentPlugin::<PbrDeferredLightingDepthId>::default())
            .add_systems(PostUpdate, insert_deferred_lighting_pass_id_component);
    }
}

pub fn insert_deferred_lighting_pass_id_component(
    mut commands: Commands,
    views: Query<Entity, (With<DeferredPrepass>, Without<PbrDeferredLightingDepthId>)>,
) {
    for entity in views.iter() {
        commands
            .entity(entity)
            .insert(PbrDeferredLightingDepthId::default());
    }
}
