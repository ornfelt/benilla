//! A module for the [`GizmoConfig<T>`] [`Resource`].

use bevy_camera::visibility::RenderLayers;
pub use bevy_gizmos_macros::GizmoConfigGroup;

use bevy_ecs::resource::Resource;
use bevy_reflect::TypePath;
use bevy_utils::TypeIdMap;
use core::{
    any::{Any, TypeId},
    hash::Hash,
    ops::Deref,
    panic,
};

/// An enum configuring how line joints will be drawn.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum GizmoLineJoint {
    /// Does not draw any line joints.
    #[default]
    None,
    /// Extends both lines at the joining point until they meet in a sharp point.
    Miter,
    /// Draws a round corner with the specified resolution between the two lines.
    ///
    /// The resolution determines the amount of triangles drawn per joint,
    /// e.g. `GizmoLineJoint::Round(4)` will draw 4 triangles at each line joint.
    Round(u32),
    /// Draws a bevel, a straight line in this case, to connect the ends of both lines.
    Bevel,
}

/// An enum used to configure the style of gizmo lines, similar to CSS line-style
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum GizmoLineStyle {
    /// A solid line without any decorators
    #[default]
    Solid,
    /// A dotted line
    Dotted,
    /// A dashed line with configurable gap and line sizes
    Dashed {
        /// The length of the gap in `line_width`s
        gap_scale: f32,
        /// The length of the visible line in `line_width`s
        line_scale: f32,
    },
}

impl Eq for GizmoLineStyle {}

impl Hash for GizmoLineStyle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Solid => {
                0u64.hash(state);
            }
            Self::Dotted => 1u64.hash(state),
            Self::Dashed {
                gap_scale,
                line_scale,
            } => {
                2u64.hash(state);
                gap_scale.to_bits().hash(state);
                line_scale.to_bits().hash(state);
            }
        }
    }
}

/// A trait used to create gizmo configs groups.
///
/// Here you can store additional configuration for you gizmo group not covered by [`GizmoConfig`]
///
/// Make sure to derive [`Default`] + [`TypePath`] and register in the app using `app.init_gizmo_group::<T>()`
pub trait GizmoConfigGroup: TypePath + Default + Send + Sync {}

/// The default gizmo config group.
#[derive(Default, TypePath, GizmoConfigGroup)]
pub struct DefaultGizmoConfigGroup;

/// Used when the gizmo config group needs to be type-erased.
/// Also used for retained gizmos, which can't have a gizmo config group.
#[derive(Default, TypePath, GizmoConfigGroup, Debug, Clone)]
pub struct ErasedGizmoConfigGroup;

/// A [`Resource`] storing [`GizmoConfig`] and [`GizmoConfigGroup`] structs
///
/// Use `app.init_gizmo_group::<T>()` to register a custom config group.
#[derive(Resource, Default)]
pub struct GizmoConfigStore {
    // INVARIANT: must map TypeId::of::<T>() to correct type T
    store: TypeIdMap<(GizmoConfig, Box<dyn Any + Send + Sync>)>,
}

impl GizmoConfigStore {
    /// Returns [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`TypeId`] of a [`GizmoConfigGroup`]
    pub fn get_config_dyn(&self, config_type_id: &TypeId) -> Option<(&GizmoConfig, &dyn Any)> {
        let (config, ext) = self.store.get(config_type_id)?;
        Some((config, ext.deref()))
    }

    /// Returns [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`GizmoConfigGroup`] `T`
    pub fn config<T: GizmoConfigGroup>(&self) -> (&GizmoConfig, &T) {
        let Some((config, ext)) = self.get_config_dyn(&TypeId::of::<T>()) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_group<T>()`?", T::type_path());
        };
        // hash map invariant guarantees that &dyn Any is of correct type T
        let ext = ext.downcast_ref().unwrap();
        (config, ext)
    }

    /// Inserts [`GizmoConfig`] and [`GizmoConfigGroup`] replacing old values
    pub fn insert<T: GizmoConfigGroup>(&mut self, config: GizmoConfig, ext_config: T) {
        // INVARIANT: hash map must correctly map TypeId::of::<T>() to &dyn Any of type T
        self.store
            .insert(TypeId::of::<T>(), (config, Box::new(ext_config)));
    }

    pub(crate) fn register<T: GizmoConfigGroup>(&mut self) {
        self.insert(GizmoConfig::default(), T::default());
    }
}

/// A struct that stores configuration for gizmos.
#[derive(Clone, Debug)]
pub struct GizmoConfig {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Line settings.
    pub line: GizmoLineConfig,
    /// How closer to the camera than real geometry the gizmos should be.
    ///
    /// In 2D this setting has no effect and is effectively always -1.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.
    pub depth_bias: f32,
    /// Describes which rendering layers gizmos will be rendered to.
    ///
    /// Gizmos will only be rendered to cameras with intersecting layers.
    pub render_layers: RenderLayers,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            line: Default::default(),
            depth_bias: 0.,
            render_layers: Default::default(),
        }
    }
}

/// A struct that stores configuration for gizmos.
#[derive(Clone, Debug)]
pub struct GizmoLineConfig {
    /// Line width specified in pixels.
    ///
    /// If `perspective` is `true` then this is the size in pixels at the camera's near plane.
    ///
    /// Defaults to `2.0`.
    pub width: f32,
    /// Apply perspective to gizmo lines.
    ///
    /// This setting only affects 3D, non-orthographic cameras.
    ///
    /// Defaults to `false`.
    pub perspective: bool,
    /// Determine the style of gizmo lines.
    pub style: GizmoLineStyle,
    /// Describe how lines should join.
    pub joints: GizmoLineJoint,
}

impl Default for GizmoLineConfig {
    fn default() -> Self {
        Self {
            width: 2.,
            perspective: false,
            style: GizmoLineStyle::Solid,
            joints: GizmoLineJoint::None,
        }
    }
}
