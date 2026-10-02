use crate::sync_component::SyncComponentPlugin;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    bundle::NoBundleEffect,
    component::Component,
    prelude::*,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
};
use core::marker::PhantomData;

pub use bevy_render_macros::ExtractComponent;

/// Describes how a component gets extracted for rendering.
///
/// Therefore the component is transferred from the "app world" into the "render world"
/// in the [`ExtractSchedule`] step.
pub trait ExtractComponent: Component {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;

    /// The output from extraction.
    ///
    /// Returning `None` based on the queried item will remove the component from the entity in
    /// the render world. This can be used, for example, to conditionally extract camera settings
    /// in order to disable a rendering feature on the basis of those settings, without removing
    /// the component from the entity in the main world.
    ///
    /// The output may be different from the queried component.
    /// This can be useful for example if only a subset of the fields are useful
    /// in the render world.
    ///
    /// `Out` has a [`Bundle`] trait bound instead of a [`Component`] trait bound in order to allow use cases
    /// such as tuples of components as output.
    type Out: Bundle<Effect: NoBundleEffect>;

    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Out: Component = Self;

    /// Defines how the component is transferred into the "render world".
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;
}

/// This plugin extracts the components into the render world for synced entities.
///
/// To do so, it sets up the [`ExtractSchedule`] step for the specified [`ExtractComponent`].
pub struct ExtractComponentPlugin<C, F = ()> {
    #[expect(
        dead_code,
        reason = "read only when adding the render world's extraction, which is cut"
    )]
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (C, F)>,
}

impl<C, F> Default for ExtractComponentPlugin<C, F> {
    fn default() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }
}

impl<C: ExtractComponent> Plugin for ExtractComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        // The extraction systems only reached the RenderApp.
        app.add_plugins(SyncComponentPlugin::<C>::default());
    }
}
