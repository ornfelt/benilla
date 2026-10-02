use bevy_app::prelude::*;
use bevy_core_pipeline::prepass::DeferredPrepass;
use bevy_ecs::prelude::*;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};

pub struct DeferredPbrLightingPlugin;

pub const DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID: u8 = 1;

/// Component marking a view for the PBR deferred lighting pass.
///
/// Will be automatically added to entities with the [`DeferredPrepass`] component that don't already have a [`PbrDeferredLightingDepthId`].
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
pub struct PbrDeferredLightingDepthId;

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
