//! How a material draws through gfx. A material type registers with [`GfxMaterialPlugin`] and a
//! function describing one of its assets as a [`GfxMaterialDesc`]; each frame the visible
//! entities carrying it become draw items ([`crate::draw::DrawList`]). A type that is not
//! registered is not drawn.
//!
//! [`standard`] describes a `StandardMaterial` as its unlit form: the base colour times the base
//! colour texture times the vertex colour, the alpha mode applied as bevy_pbr's
//! `alpha_discard` and `premultiply_alpha` do. Lit shading, fog, emissive, normal maps and depth
//! bias are not drawn yet; the world's `ExtendedMaterial`s draw through their base until their own
//! programs land (milestone 4).

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::asset::AssetId;
use bevy::image::Image;
use bevy::math::Affine2;
use bevy::pbr::{ExtendedMaterial, Material, MaterialExtension, MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy::render::render_resource::Face;

use crate::draw::{DrawItem, DrawList};
use crate::meshes::VertexInput;
use crate::render::GfxRenderSystems;

/// How the fragment's alpha is used, and the blend state that goes with it (bevy_pbr's
/// `MeshPipelineKey` blend bits).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GfxAlpha {
    Opaque,
    Mask(f32),
    Blend,
    Premultiplied,
    Add,
    Multiply,
}

impl GfxAlpha {
    /// Bevy's own mapping; `AlphaToCoverage` falls back to `Mask(0.5)` as it does without MSAA.
    pub fn from_bevy(mode: AlphaMode) -> Self {
        match mode {
            AlphaMode::Opaque => Self::Opaque,
            AlphaMode::Mask(c) => Self::Mask(c),
            AlphaMode::AlphaToCoverage => Self::Mask(0.5),
            AlphaMode::Blend => Self::Blend,
            AlphaMode::Premultiplied => Self::Premultiplied,
            AlphaMode::Add => Self::Add,
            AlphaMode::Multiply => Self::Multiply,
        }
    }

    /// The shader's mode number (`standard.fs.gfxs`).
    pub fn shader_mode(self) -> f32 {
        match self {
            Self::Opaque => 0.0,
            Self::Mask(_) => 1.0,
            Self::Blend => 2.0,
            Self::Premultiplied => 3.0,
            Self::Add => 4.0,
            Self::Multiply => 5.0,
        }
    }

    pub fn cutoff(self) -> f32 {
        match self {
            Self::Mask(c) => c,
            _ => 0.0,
        }
    }

    /// Drawn in the sorted transparent phase, without depth writes.
    pub fn is_transparent(self) -> bool {
        !matches!(self, Self::Opaque | Self::Mask(_))
    }
}

/// One gfx program: its compiled base name and the vertex attributes it reads, in input order.
#[derive(Debug, Clone, Copy)]
pub struct GfxProgram {
    pub name: &'static str,
    pub inputs: &'static [VertexInput],
}

/// `standard.{vs,fs}.gfxs`: position, normal, UV 0 and colour, as bevy_pbr's forward mesh reads
/// them; a mesh without colour draws white, as without `VERTEX_COLORS`.
pub const STANDARD: GfxProgram = GfxProgram {
    name: "standard",
    inputs: &[
        VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_NORMAL, [0.0, 0.0, 1.0, 0.0]),
        VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
        VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    ],
};

/// What one material asset draws with.
#[derive(Debug, Clone, PartialEq)]
pub struct GfxMaterialDesc {
    pub program: GfxProgram,
    /// Linear.
    pub base_color: LinearRgba,
    pub texture: Option<AssetId<Image>>,
    pub uv_transform: Affine2,
    pub alpha: GfxAlpha,
    pub cull: Option<Face>,
}

impl PartialEq for GfxProgram {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// A `StandardMaterial` as the standard program draws it.
pub fn standard(m: &StandardMaterial) -> GfxMaterialDesc {
    GfxMaterialDesc {
        program: STANDARD,
        base_color: m.base_color.into(),
        texture: m.base_color_texture.as_ref().map(Handle::id),
        uv_transform: m.uv_transform,
        alpha: GfxAlpha::from_bevy(m.alpha_mode),
        cull: m.cull_mode,
    }
}

/// An `ExtendedMaterial` over `StandardMaterial` as its base draws, until the extension has a
/// program of its own.
pub fn extended_base<E: MaterialExtension>(
    m: &ExtendedMaterial<StandardMaterial, E>,
) -> GfxMaterialDesc {
    standard(&m.base)
}

/// Draws every visible `MeshMaterial3d<M>` with the description `describe` gives its asset.
pub struct GfxMaterialPlugin<M: Material> {
    describe: fn(&M) -> GfxMaterialDesc,
    marker: PhantomData<M>,
}

impl<M: Material> GfxMaterialPlugin<M> {
    pub fn new(describe: fn(&M) -> GfxMaterialDesc) -> Self {
        Self {
            describe,
            marker: PhantomData,
        }
    }
}

#[derive(Resource)]
struct Describe<M: Material>(fn(&M) -> GfxMaterialDesc);

impl<M: Material> Plugin for GfxMaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<Describe<M>>() {
            return;
        }
        app.insert_resource(Describe::<M>(self.describe))
            .add_systems(
                crate::render::GfxRender,
                collect::<M>.in_set(GfxRenderSystems::Collect),
            );
    }

    fn is_unique(&self) -> bool {
        false
    }
}

/// The visible entities carrying `M` become draw items, one description per material asset.
fn collect<M: Material>(
    describe: Res<Describe<M>>,
    // Absent when the type's `MaterialPlugin` is not in this app.
    materials: Option<Res<Assets<M>>>,
    mut list: ResMut<DrawList>,
    mut descs: Local<HashMap<AssetId<M>, u32>>,
    items: Query<(
        Entity,
        &Mesh3d,
        &MeshMaterial3d<M>,
        &GlobalTransform,
        &ViewVisibility,
    )>,
) {
    descs.clear();
    let Some(materials) = materials else {
        return;
    };
    for (entity, mesh, material, transform, visibility) in &items {
        if !visibility.get() {
            continue;
        }
        let id = material.0.id();
        let desc = match descs.get(&id) {
            Some(d) => *d,
            None => {
                let Some(m) = materials.get(id) else {
                    continue;
                };
                let d = list.push_desc((describe.0)(m));
                descs.insert(id, d);
                d
            }
        };
        list.push(
            entity,
            DrawItem {
                mesh: mesh.0.id(),
                world_from_local: transform.to_matrix(),
                desc,
            },
        );
    }
}
