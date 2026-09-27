//! How a material draws through gfx. A material type registers with [`GfxMaterialPlugin`] and a
//! function describing one of its assets as a [`GfxMaterialDesc`]; each frame the visible
//! entities carrying it become draw items ([`crate::draw::DrawList`]). A type that is not
//! registered is not drawn.
//!
//! A description names its program, the textures bound to the program's sampler slots, the
//! program's parameter rows (its draw block after the world matrix and the tag) and the fixed
//! states. [`standard`] describes a `StandardMaterial` as its unlit form: the base colour times
//! the base colour texture times the vertex colour, the alpha mode applied as bevy_pbr's
//! `alpha_discard` and `premultiply_alpha` do. Lit shading, emissive and normal maps are not drawn
//! yet; an `ExtendedMaterial` without a program of its own draws through its base
//! ([`extended_base`]).

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::asset::AssetId;
use bevy::camera::primitives::Aabb;
use bevy::image::Image;
use bevy::math::Affine2;
use bevy::mesh::MeshTag;
use bevy::pbr::{ExtendedMaterial, Material, MaterialExtension, MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy::render::render_resource::{BufferId, Face};

use crate::draw::{DrawItem, DrawList};
use crate::images::GfxSampler;
use crate::meshes::VertexInput;
use crate::pipelines::Blend;
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

    /// The shader's mode number (`standard.fs.gfxs`, `wow_model.fs.gfxs`).
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

    /// Drawn in the sorted transparent phase, without depth writes by default.
    pub fn is_transparent(self) -> bool {
        !matches!(self, Self::Opaque | Self::Mask(_))
    }
}

/// One gfx program: its compiled base name, the vertex attributes it reads in input order, how
/// many parameter rows its draw block holds and how many sampler slots it binds.
#[derive(Debug, Clone, Copy)]
pub struct GfxProgram {
    pub name: &'static str,
    pub inputs: &'static [VertexInput],
    pub params: usize,
    pub samplers: usize,
}

impl PartialEq for GfxProgram {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Parameter rows a description carries at most.
pub const MAX_PARAMS: usize = 12;
/// Sampler slots a description binds at most.
pub const MAX_TEXTURES: usize = 4;

/// What one of a program's sampler slots samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GfxTextureSlot {
    /// Opaque white, 1x1.
    White,
    /// An image asset; a draw whose image is not on the device yet is skipped, as Bevy skips a
    /// material whose texture is not loaded.
    Image(AssetId<Image>),
    /// An image asset through a sampler of the draw's own ([`crate::images::GfxSampler`]).
    ImageSampled(AssetId<Image>, GfxSampler),
    /// An image asset through the second image's sampler, as a Bevy `#[sampler]` binding
    /// samples its group's textures.
    ImageSampledLike(AssetId<Image>, AssetId<Image>),
    /// The data texture standing in for a storage buffer ([`crate::data`]).
    Data(BufferId),
}

/// Fixed states a material keys beyond its alpha mode, as a `specialize` sets them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfxDrawState {
    /// The blend state in place of the alpha mode's.
    pub blend: Option<Blend>,
    /// Depth writes in place of the phase's (opaque and mask write, transparent does not).
    pub depth_write: Option<bool>,
    /// `CompareFunction::Always` in place of reverse-Z `GreaterEqual`.
    pub depth_always: bool,
    /// Colour writes on; off for a depth-only draw.
    pub color_write: bool,
    /// Added to the transparent phase's sort distance, as `StandardMaterial::depth_bias` is.
    pub sort_bias: f32,
}

impl Default for GfxDrawState {
    fn default() -> Self {
        Self {
            blend: None,
            depth_write: None,
            depth_always: false,
            color_write: true,
            sort_bias: 0.0,
        }
    }
}

/// What one material asset draws with.
#[derive(Debug, Clone, PartialEq)]
pub struct GfxMaterialDesc {
    pub program: GfxProgram,
    pub textures: [GfxTextureSlot; MAX_TEXTURES],
    /// The program's parameter rows; the first `program.params` are uploaded.
    pub params: [[f32; 4]; MAX_PARAMS],
    pub alpha: GfxAlpha,
    pub cull: Option<Face>,
    pub state: GfxDrawState,
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
    params: 4,
    samplers: 1,
};

/// A `StandardMaterial`'s base colour, texture, UV transform and alpha as parameter rows 0-2 and
/// texture slot 0, the layout `standard` and the world programs share: row 0 the linear base
/// colour, row 1 the UV matrix, row 2 the UV offset, the mask cutoff and the alpha mode number.
pub fn standard_rows(
    m: &StandardMaterial,
) -> ([[f32; 4]; 3], GfxTextureSlot, GfxAlpha, GfxDrawState) {
    let alpha = GfxAlpha::from_bevy(m.alpha_mode);
    (
        [
            LinearRgba::from(m.base_color).to_f32_array(),
            uv_matrix(m.uv_transform),
            uv_offset(m.uv_transform, alpha),
        ],
        m.base_color_texture
            .as_ref()
            .map_or(GfxTextureSlot::White, |h| GfxTextureSlot::Image(h.id())),
        alpha,
        GfxDrawState {
            sort_bias: m.depth_bias,
            ..default()
        },
    )
}

fn uv_matrix(t: Affine2) -> [f32; 4] {
    let m = t.matrix2;
    [m.x_axis.x, m.x_axis.y, m.y_axis.x, m.y_axis.y]
}

fn uv_offset(t: Affine2, alpha: GfxAlpha) -> [f32; 4] {
    [
        t.translation.x,
        t.translation.y,
        alpha.cutoff(),
        alpha.shader_mode(),
    ]
}

/// A `StandardMaterial` as the standard program draws it.
pub fn standard(m: &StandardMaterial) -> GfxMaterialDesc {
    let (rows, texture, alpha, state) = standard_rows(m);
    let mut params = [[0.0; 4]; MAX_PARAMS];
    params[..3].copy_from_slice(&rows);
    GfxMaterialDesc {
        program: STANDARD,
        textures: [
            texture,
            GfxTextureSlot::White,
            GfxTextureSlot::White,
            GfxTextureSlot::White,
        ],
        params,
        alpha,
        cull: m.cull_mode,
        state,
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
#[allow(clippy::type_complexity)]
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
        Option<&MeshTag>,
        Option<&Aabb>,
    )>,
) {
    descs.clear();
    let Some(materials) = materials else {
        return;
    };
    for (entity, mesh, material, transform, visibility, tag, aabb) in &items {
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
        let world_from_local = transform.to_matrix();
        // bevy_pbr sorts a transparent mesh by its world AABB centre (`RenderMeshInstance::center`).
        let center = aabb.map_or(transform.translation(), |a| {
            world_from_local.transform_point3(a.center.into())
        });
        list.push(
            entity,
            DrawItem {
                mesh: mesh.0.id(),
                world_from_local,
                center,
                tag: tag.map_or(0, |t| t.0),
                desc,
            },
        );
    }
}
