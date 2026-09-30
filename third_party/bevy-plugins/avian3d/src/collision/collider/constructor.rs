use core::iter::once;

use crate::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use itertools::Either;

/// A component that will automatically generate [`Collider`]s on its descendants at runtime.
/// The type of the generated collider can be specified using [`ColliderConstructor`].
/// This supports computing the shape dynamically from the mesh, in which case only the descendants
/// with a [`Mesh`] will have colliders generated.
///
/// In contrast to [`ColliderConstructor`], this component will *not* generate a collider on its own entity.
///
/// If this component is used on a scene, such as one spawned by a [`SceneRoot`], it will
/// wait until the scene is loaded before generating colliders. Note that this requires
/// the `bevy_scene` feature to be enabled.
///
/// The exact configuration for each descendant can be specified in the `config`.
///
/// This component will only override a pre-existing [`Collider`] component on a descendant entity
/// when it has been explicitly mentioned in the `config`.
///
/// # See Also
///
/// For inserting colliders on the same entity, use [`ColliderConstructor`].
///
/// # Caveats
///
/// When a component has multiple ancestors with [`ColliderConstructorHierarchy`], the insertion order is undefined.
///
/// # Example
///
/// Below are some examples of using [`ColliderConstructorHierarchy`] to generate colliders
/// for a glTF scene at runtime. Note that this requires the `bevy_scene` feature to be enabled.
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands, mut assets: ResMut<AssetServer>) {
///     let scene = assets.load("my_model.gltf#Scene0");
///
///     // Spawn the scene and automatically generate triangle mesh colliders
///     commands.spawn((
///         SceneRoot(scene.clone()),
///         ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
///     ));
///
///     // Specify configuration for specific meshes by name
///     commands.spawn((
///         SceneRoot(scene.clone()),
///         ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
///             .with_constructor_for_name("Tree", ColliderConstructor::ConvexHullFromMesh)
///             .with_layers_for_name("Tree", CollisionLayers::from_bits(0b0010, 0b1111))
///             .with_density_for_name("Tree", 2.5),
///     ));
///
///     // Only generate colliders for specific meshes by name
///     commands.spawn((
///         SceneRoot(scene.clone()),
///         ColliderConstructorHierarchy::new(None)
///             .with_constructor_for_name("Tree", ColliderConstructor::ConvexHullFromMesh),
///     ));
///
///     // Generate colliders for everything except specific meshes by name
///     commands.spawn((
///         SceneRoot(scene),
///         ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMeshWithConfig(
///              TrimeshFlags::MERGE_DUPLICATE_VERTICES
///         ))
///         .without_constructor_for_name("Tree"),
///     ));
/// }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Default)]
pub struct ColliderConstructorHierarchy {
    /// The default collider type used for each entity that isn't included in [`config`](Self::config).
    /// If `None`, all entities except the ones in [`config`](Self::config) will be skipped.
    pub default_constructor: Option<ColliderConstructor>,
    /// The default [`CollisionLayers`] used for colliders in the hierarchy.
    ///
    /// [`CollisionLayers::default()`] by default, with the first layer and all filters.
    pub default_layers: CollisionLayers,
    /// The default [`ColliderDensity`] used for colliders in the hierarchy.
    ///
    /// `1.0` by default.
    pub default_density: ColliderDensity,
    /// Specifies data like the [`ColliderConstructor`] and [`CollisionLayers`] for entities
    /// in the hierarchy by `Name`. Entries with a `None` value will be skipped.
    ///
    /// For the entities not found in this `HashMap`, [`default_constructor`](Self::default_constructor),
    /// [`default_layers`](Self::default_layers), and [`default_density`](Self::default_density) will be used instead.
    pub config: HashMap<String, Option<ColliderConstructorHierarchyConfig>>,
}

/// Triggered when a [`ColliderConstructor`] successfully inserted a [`Collider`].
///
/// The event is not triggered when the [`ColliderConstructor`] failed to construct the [`Collider`]
/// or when there was already a [`Collider`] on the entity.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq)]
pub struct ColliderConstructorReady {
    /// The entity that held the [`ColliderConstructor`].
    pub entity: Entity,
}

/// Triggered when a [`ColliderConstructorHierarchy`] finished inserting all its [`Collider`]s.
///
/// Note that the event will still be triggered when when the hierarchy had no colliders to insert
/// or failed to insert all of them, so this event is not a guarantee that there are actually
/// any colliders in the scene.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq)]
pub struct ColliderConstructorHierarchyReady {
    /// The entity that held the [`ColliderConstructorHierarchy`].
    pub entity: Entity,
}

/// Configuration for a specific collider generated from a scene using [`ColliderConstructorHierarchy`].
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Debug, Default, PartialEq)]
pub struct ColliderConstructorHierarchyConfig {
    /// The type of collider generated for the mesh.
    ///
    /// If `None`, [`ColliderConstructorHierarchy::default_constructor`] is used instead.
    pub constructor: Option<ColliderConstructor>,
    /// The [`CollisionLayers`] used for this collider.
    ///
    /// If `None`, [`ColliderConstructorHierarchy::default_layers`] is used instead.
    pub layers: Option<CollisionLayers>,
    /// The [`ColliderDensity`] used for this collider.
    ///
    /// If `None`, [`ColliderConstructorHierarchy::default_density`] is used instead.
    pub density: Option<ColliderDensity>,
}

/// A component that will automatically generate a [`Collider`] at runtime using [`Collider::try_from_constructor`].
/// Enabling the `collider-from-mesh` feature activates support for computing the shape dynamically from the mesh attached to the same entity.
///
/// Since [`Collider`] is not [`Reflect`], you can use this type to statically specify a collider's shape instead.
///
/// This component will never override a pre-existing [`Collider`] component on the same entity.
///
/// # See Also
///
/// For inserting colliders on an entity's descendants, use [`ColliderConstructorHierarchy`].
///
/// # Panics
///
/// The system handling the generation of colliders will panic if the specified [`ColliderConstructor`]
/// requires a mesh, but the entity does not have a `Handle<Mesh>` component.
///
/// # Example
///
/// ```
/// use avian3d::prelude::*;
/// use bevy::prelude::*;
///
/// fn setup(mut commands: Commands, mut assets: ResMut<AssetServer>, mut meshes: Assets<Mesh>) {
///     // Spawn a cube with a convex hull collider generated from the mesh
///     commands.spawn((
///         ColliderConstructor::ConvexHullFromMesh,
///         Mesh3d(meshes.add(Cuboid::default())),
///     ));
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Reflect, Component, Default)]
#[reflect(Default)]
#[reflect(Debug, Component, PartialEq)]
#[reflect(no_field_bounds)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum ColliderConstructor {
    /// Constructs a collider with [`Collider::sphere`].
    Sphere { radius: Scalar },
    /// Constructs a collider with [`Collider::cuboid`].
    Cuboid {
        x_length: Scalar,
        y_length: Scalar,
        z_length: Scalar,
    },
    /// Constructs a collider with [`Collider::round_cuboid`].
    RoundCuboid {
        x_length: Scalar,
        y_length: Scalar,
        z_length: Scalar,
        border_radius: Scalar,
    },
    /// Constructs a collider with [`Collider::cylinder`].
    Cylinder { radius: Scalar, height: Scalar },
    /// Constructs a collider with [`Collider::cone`].
    Cone { radius: Scalar, height: Scalar },
    /// Constructs a collider with [`Collider::capsule`].
    Capsule { radius: Scalar, height: Scalar },
    /// Constructs a collider with [`Collider::capsule_endpoints`].
    CapsuleEndpoints {
        radius: Scalar,
        a: Vector,
        b: Vector,
    },
    /// Constructs a collider with [`Collider::half_space`].
    HalfSpace { outward_normal: Vector },
    /// Constructs a collider with [`Collider::segment`].
    Segment { a: Vector, b: Vector },
    /// Constructs a collider with [`Collider::triangle`].
    Triangle { a: Vector, b: Vector, c: Vector },
    /// Constructs a collider with [`Collider::polyline`].
    Polyline {
        vertices: Vec<Vector>,
        indices: Option<Vec<[u32; 2]>>,
    },
    /// Constructs a collider with [`Collider::trimesh`].
    Trimesh {
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
    },
    /// Constructs a collider with [`Collider::trimesh_with_config`].
    TrimeshWithConfig {
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
        flags: TrimeshFlags,
    },
    /// Constructs a collider with [`Collider::convex_decomposition`].
    ConvexDecomposition {
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
    },
    /// Constructs a collider with [`Collider::convex_decomposition_with_config`].
    ConvexDecompositionWithConfig {
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
        params: VhacdParameters,
    },
    /// Constructs a collider with [`Collider::convex_hull`].
    ConvexHull { points: Vec<Vector> },
    /// Constructs a collider with [`Collider::voxels`].
    Voxels {
        voxel_size: Vector,
        grid_coordinates: Vec<IVector>,
    },
    /// Constructs a collider with [`Collider::voxelized_trimesh`].
    VoxelizedTrimesh {
        vertices: Vec<Vector>,
        indices: Vec<[u32; 3]>,
        voxel_size: Scalar,
        fill_mode: FillMode,
    },
    /// Constructs a collider with [`Collider::heightfield`].
    Heightfield {
        heights: Vec<Vec<Scalar>>,
        scale: Vector,
    },
    /// Constructs a collider with [`Collider::trimesh_from_mesh`].
    #[default]
    TrimeshFromMesh,
    /// Constructs a collider with [`Collider::trimesh_from_mesh_with_config`].
    TrimeshFromMeshWithConfig(TrimeshFlags),
    /// Constructs a collider with [`Collider::convex_decomposition_from_mesh`].
    ConvexDecompositionFromMesh,
    /// Constructs a collider with [`Collider::convex_decomposition_from_mesh_with_config`].
    ConvexDecompositionFromMeshWithConfig(VhacdParameters),
    /// Constructs a collider with [`Collider::convex_hull_from_mesh`].
    ConvexHullFromMesh,
    /// Constructs a collider with [`Collider::voxelized_trimesh_from_mesh`].
    VoxelizedTrimeshFromMesh {
        voxel_size: Scalar,
        fill_mode: FillMode,
    },
    /// Constructs a collider with [`Collider::compound`].
    Compound(Vec<(Position, Rotation, ColliderConstructor)>),
}

impl ColliderConstructor {
    /// Returns `true` if the collider type requires a mesh to be generated.
    pub fn requires_mesh(&self) -> bool {
        matches!(
            self,
            Self::TrimeshFromMesh
                | Self::TrimeshFromMeshWithConfig(_)
                | Self::ConvexDecompositionFromMesh
                | Self::ConvexDecompositionFromMeshWithConfig(_)
                | Self::ConvexHullFromMesh
                | Self::VoxelizedTrimeshFromMesh { .. }
        )
    }

    pub(crate) fn flatten_compound_constructors(
        constructors: Vec<(Position, Rotation, ColliderConstructor)>,
    ) -> Vec<(Position, Rotation, ColliderConstructor)> {
        constructors
            .into_iter()
            .flat_map(|(pos, rot, constructor)| match constructor {
                ColliderConstructor::Compound(nested) => {
                    Either::Left(Self::flatten_compound_constructors(nested).into_iter().map(
                        move |(nested_pos, nested_rot, nested_constructor)| {
                            (
                                Position(pos.0 + rot * nested_pos.0),
                                rot * nested_rot,
                                nested_constructor,
                            )
                        },
                    ))
                }
                other => Either::Right(once((pos, rot, other))),
            })
            .collect()
    }
}
