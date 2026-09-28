//! bevy_ui on the gfx renderer: bevy_ui_render 0.18.1's extraction, queue and prepare (`lib.rs`,
//! `ui_texture_slice_pipeline.rs`, `ui_material_pipeline.rs`) over the main world. Each frame the
//! UI nodes on a UI lane camera ([`GfxUiLane`]) become the quads bevy builds: backgrounds,
//! images, borders, outlines, text and text shadows through the node program; sliced and tiled
//! images through the slice program; [`UiMaterial`] nodes through a description of their own
//! ([`GfxUiMaterialPlugin`]). They sort by stack index plus bevy's per-kind offset
//! (`stack_z_offsets`, stable over the extraction order), batch as bevy batches them (by image,
//! the default image joining any), go into two meshes rewritten in place, and draw as the lane's
//! late draws ([`DrawList::push_late`]) through bevy's UI view projection: the `UiPass` after the
//! 2D main pass.
//!
//! Not drawn: box shadows, gradients, viewport nodes, text backgrounds, underline and
//! strikethrough, the debug overlay (none in benilla's UI), and UI on a camera outside the lane.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;

use bevy::asset::RenderAssetUsages;
use bevy::image::{TextureAtlasLayout, TRANSPARENT_IMAGE_HANDLE};
use bevy::math::Affine2;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::text::{ComputedTextBlock, PositionedGlyph, TextLayoutInfo};
use bevy::ui::widget::{NodeImageMode, TextShadow};
use bevy::ui::{CalculatedClip, ComputedNode, ComputedUiTargetCamera, UiGlobalTransform};
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};
use bevy::ui_render::UiAntiAlias;

use crate::draw::{DrawList, GfxRenderer, LateDraw};
use crate::ffi::GfxFormat;
use crate::material::{GfxAlpha, GfxDrawState, GfxMaterialDesc, GfxProgram, GfxTextureSlot};
use crate::meshes::VertexInput;
use crate::pipelines::Blend;
use crate::render::{GfxRender, GfxRenderSystems};
use crate::ui::GfxUiLane;

/// The node vertex's `flags` (bevy's `shader_flags`).
pub const ATTRIBUTE_UI_FLAGS: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Flags", 988_201, VertexFormat::Uint32);
/// Corner radii: top left, top right, bottom right, bottom left.
pub const ATTRIBUTE_UI_RADIUS: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Radius", 988_202, VertexFormat::Float32x4);
/// Border widths: left, top, right, bottom.
pub const ATTRIBUTE_UI_BORDER: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Border", 988_203, VertexFormat::Float32x4);
/// The node's size in physical pixels.
pub const ATTRIBUTE_UI_SIZE: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Size", 988_204, VertexFormat::Float32x2);
/// The vertex relative to the node's centre.
pub const ATTRIBUTE_UI_POINT: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Point", 988_205, VertexFormat::Float32x2);
/// The slice vertex's normalized texture slice lines.
pub const ATTRIBUTE_UI_SLICES: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Slices", 988_211, VertexFormat::Float32x4);
/// The normalized target slice lines.
pub const ATTRIBUTE_UI_TARGET: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Target", 988_212, VertexFormat::Float32x4);
/// The side and centre repeat counts.
pub const ATTRIBUTE_UI_REPEAT: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Repeat", 988_213, VertexFormat::Float32x4);
/// The normalized atlas rect.
pub const ATTRIBUTE_UI_ATLAS: MeshVertexAttribute =
    MeshVertexAttribute::new("GfxUi_Atlas", 988_214, VertexFormat::Float32x4);

/// The node program's inputs, `ui.wgsl`'s vertex locations.
pub const UI_NODE_INPUTS: &[VertexInput] = &[
    VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
    VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
    VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_FLAGS, [0.0; 4]).read_as(GfxFormat::R32Sint),
    VertexInput::new(ATTRIBUTE_UI_RADIUS, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_BORDER, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_SIZE, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_POINT, [0.0; 4]),
];

/// The slice program's inputs, `ui_texture_slice.wgsl`'s vertex locations.
pub const UI_SLICE_INPUTS: &[VertexInput] = &[
    VertexInput::new(Mesh::ATTRIBUTE_POSITION, [0.0; 4]),
    VertexInput::new(Mesh::ATTRIBUTE_UV_0, [0.0; 4]),
    VertexInput::new(Mesh::ATTRIBUTE_COLOR, [1.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_SLICES, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_TARGET, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_REPEAT, [0.0; 4]),
    VertexInput::new(ATTRIBUTE_UI_ATLAS, [0.0; 4]),
];

/// bevy_ui_render's `stack_z_offsets`: a node's kinds of quad in draw order.
mod z {
    pub const BACKGROUND_COLOR: f32 = 0.0;
    pub const BORDER: f32 = 0.01;
    pub const IMAGE: f32 = 0.04;
    pub const MATERIAL: f32 = 0.05;
    pub const TEXT: f32 = 0.06;
}

/// bevy_ui_render's `shader_flags`, as the node program reads them.
mod flags {
    pub const TEXTURED: u32 = 1;
    /// Ordering: top left, top right, bottom right, bottom left.
    pub const CORNERS: [u32; 4] = [0, 2, 2 | 4, 4];
    pub const BORDER_LEFT: u32 = 256;
    pub const BORDER_TOP: u32 = 512;
    pub const BORDER_RIGHT: u32 = 1024;
    pub const BORDER_BOTTOM: u32 = 2048;
    pub const BORDER_ALL: u32 = BORDER_LEFT + BORDER_TOP + BORDER_RIGHT + BORDER_BOTTOM;
}

/// bevy_ui_render's UI view: moved back this far, and seeing this far.
const UI_CAMERA_FAR: f32 = 1000.0;
const UI_CAMERA_TRANSFORM_OFFSET: f32 = -0.1;

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];
const QUAD_INDICES: [u32; 6] = [0, 2, 3, 0, 1, 2];

/// The node and slice programs, by name: the gamma-emitting copies of bevy's UI shaders an app
/// ports (benilla's `ui_node_gamma`, `ui_slice_gamma`).
#[derive(Resource, Clone, Copy)]
struct UiPrograms {
    node: GfxProgram,
    slice: GfxProgram,
}

/// Draws bevy_ui through the named node and slice programs, which read [`UI_NODE_INPUTS`] and
/// [`UI_SLICE_INPUTS`].
pub struct GfxBevyUiPlugin {
    pub node: &'static str,
    pub slice: &'static str,
}

impl Plugin for GfxBevyUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiPrograms {
            node: GfxProgram {
                name: self.node,
                inputs: UI_NODE_INPUTS,
                params: 1,
                samplers: 1,
            },
            slice: GfxProgram {
                name: self.slice,
                inputs: UI_SLICE_INPUTS,
                params: 0,
                samplers: 1,
            },
        })
        .init_resource::<UiItems>()
        .add_systems(Startup, init_streams)
        .configure_sets(
            GfxRender,
            (
                GfxUiSystems::Extract,
                GfxUiSystems::ExtractSlices,
                GfxUiSystems::ExtractMaterials,
                GfxUiSystems::Queue,
            )
                .chain()
                .in_set(GfxRenderSystems::Collect),
        )
        .add_systems(
            GfxRender,
            (
                extract_nodes.in_set(GfxUiSystems::Extract),
                extract_slices.in_set(GfxUiSystems::ExtractSlices),
                queue.in_set(GfxUiSystems::Queue),
            ),
        );
    }
}

/// The stages of the bevy_ui pass inside [`GfxRenderSystems::Collect`]: node quads, then slices,
/// then materials, the order bevy's queue systems add them in; then the sort, batch and write.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GfxUiSystems {
    Extract,
    ExtractSlices,
    ExtractMaterials,
    Queue,
}

/// Draws every `MaterialNode<M>` with the description `describe` gives its asset, its program
/// reading the material vertex (position, UV, [`ATTRIBUTE_UI_SIZE`], [`ATTRIBUTE_UI_BORDER`],
/// [`ATTRIBUTE_UI_RADIUS`]) as `ui_material.wgsl`'s vertex stage does.
pub struct GfxUiMaterialPlugin<M: UiMaterial> {
    describe: fn(&M) -> GfxMaterialDesc,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> GfxUiMaterialPlugin<M> {
    pub fn new(describe: fn(&M) -> GfxMaterialDesc) -> Self {
        Self {
            describe,
            marker: PhantomData,
        }
    }
}

#[derive(Resource)]
struct DescribeUi<M: UiMaterial>(fn(&M) -> GfxMaterialDesc);

impl<M: UiMaterial> Plugin for GfxUiMaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<DescribeUi<M>>() {
            return;
        }
        app.insert_resource(DescribeUi::<M>(self.describe))
            .add_systems(
                GfxRender,
                extract_materials::<M>.in_set(GfxUiSystems::ExtractMaterials),
            );
    }

    fn is_unique(&self) -> bool {
        false
    }
}

/// bevy's `NodeType`: a filled rect, or a border with its side flags.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NodeType {
    Rect,
    Border(u32),
}

/// One extracted quad or run of quads (bevy's `ExtractedUiNode`, `ExtractedUiTextureSlice`,
/// `ExtractedUiMaterialNode`).
struct UiItem {
    camera: Entity,
    z: f32,
    image: AssetId<Image>,
    clip: Option<Rect>,
    transform: Affine2,
    kind: UiKind,
}

enum UiKind {
    Node {
        color: LinearRgba,
        rect: Rect,
        atlas_scaling: Option<Vec2>,
        flip_x: bool,
        flip_y: bool,
        radius: [f32; 4],
        border: BorderRect,
        node_type: NodeType,
    },
    /// Indices into [`UiItems::glyphs`].
    Glyphs(Range<usize>),
    Slice {
        color: LinearRgba,
        rect: Rect,
        atlas_rect: Option<Rect>,
        flip_x: bool,
        flip_y: bool,
        mode: NodeImageMode,
        inverse_scale_factor: f32,
    },
    Material {
        desc: u32,
        rect: Rect,
        radius: [f32; 4],
        border: BorderRect,
    },
}

struct Glyph {
    color: LinearRgba,
    translation: Vec2,
    rect: Rect,
}

/// This frame's extracted UI, cleared by the queue.
#[derive(Resource, Default)]
struct UiItems {
    items: Vec<UiItem>,
    glyphs: Vec<Glyph>,
}

/// A mesh the frame's quads are written into, and the capacities its streams are padded to, so
/// its device buffers keep their size and are rewritten in place.
struct Stream {
    mesh: Handle<Mesh>,
    vertices: usize,
    indices: usize,
}

#[derive(Resource)]
struct UiStreams {
    node: Stream,
    slice: Stream,
}

fn init_streams(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut stream = |vertices: &NodeVertices| {
        let mut s = Stream {
            mesh: Handle::default(),
            vertices: 256,
            indices: 384,
        };
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        );
        vertices.fill(&mut mesh, &mut s, &[]);
        s.mesh = meshes.add(mesh);
        s
    };
    let node = stream(&NodeVertices::Node(Vec::new()));
    let slice = stream(&NodeVertices::Slice(Vec::new()));
    commands.insert_resource(UiStreams { node, slice });
}

/// The lane camera a node draws on, if it is one.
fn lane_camera(
    target: &ComputedUiTargetCamera,
    lanes: &Query<(), With<GfxUiLane>>,
) -> Option<Entity> {
    target.get().filter(|c| lanes.contains(*c))
}

type NodeItem<'a, T> = (
    &'a ComputedNode,
    &'a UiGlobalTransform,
    &'a InheritedVisibility,
    Option<&'a CalculatedClip>,
    &'a ComputedUiTargetCamera,
    T,
);

/// `extract_uinode_background_colors`, `extract_uinode_images`, `extract_uinode_borders`,
/// `extract_text_shadows` and `extract_text_sections`, in bevy's `RenderUiSystems` order.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn extract_nodes(
    mut out: ResMut<UiItems>,
    lanes: Query<(), With<GfxUiLane>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    backgrounds: Query<NodeItem<&BackgroundColor>>,
    image_nodes: Query<NodeItem<&ImageNode>>,
    borders: Query<(&Node, NodeItem<AnyOf<(&BorderColor, &Outline)>>)>,
    shadows: Query<NodeItem<(&TextLayoutInfo, &TextShadow)>>,
    texts: Query<NodeItem<(&ComputedTextBlock, &TextColor, &TextLayoutInfo)>>,
    text_styles: Query<&TextColor>,
) {
    let out = &mut *out;
    for (node, transform, visibility, clip, target, background) in &backgrounds {
        if !visibility.get() || background.0.is_fully_transparent() || node.is_empty() {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        out.items.push(UiItem {
            camera,
            z: node.stack_index as f32 + z::BACKGROUND_COLOR,
            image: AssetId::default(),
            clip: clip.map(|c| c.clip),
            transform: transform.into(),
            kind: UiKind::Node {
                color: background.0.into(),
                rect: Rect::from_corners(Vec2::ZERO, node.size),
                atlas_scaling: None,
                flip_x: false,
                flip_y: false,
                radius: node.border_radius().into(),
                border: node.border(),
                node_type: NodeType::Rect,
            },
        });
    }

    for (node, transform, visibility, clip, target, image) in &image_nodes {
        if !visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
            || image.image_mode.uses_slices()
            || node.is_empty()
        {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&atlases))
            .map(|r| r.as_rect());
        let mut rect = match (atlas_rect, image.rect) {
            (None, None) => Rect::from_corners(Vec2::ZERO, node.size),
            (None, Some(image_rect)) => image_rect,
            (Some(atlas_rect), None) => atlas_rect,
            (Some(atlas_rect), Some(mut image_rect)) => {
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                image_rect
            }
        };
        let atlas_scaling = if atlas_rect.is_some() || image.rect.is_some() {
            let scaling = node.size() / rect.size();
            rect.min *= scaling;
            rect.max *= scaling;
            Some(scaling)
        } else {
            None
        };
        out.items.push(UiItem {
            camera,
            z: node.stack_index as f32 + z::IMAGE,
            image: image.image.id(),
            clip: clip.map(|c| c.clip),
            transform: transform.into(),
            kind: UiKind::Node {
                color: image.color.into(),
                rect,
                atlas_scaling,
                flip_x: image.flip_x,
                flip_y: image.flip_y,
                radius: node.border_radius.into(),
                border: node.border,
                node_type: NodeType::Rect,
            },
        });
    }

    const BORDER_FLAGS: [u32; 4] = [
        flags::BORDER_LEFT,
        flags::BORDER_TOP,
        flags::BORDER_RIGHT,
        flags::BORDER_BOTTOM,
    ];
    for (style, (node, transform, visibility, clip, target, (border_color, outline))) in &borders {
        if !visibility.get() || style.display == Display::None {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        let clip = clip.map(|c| c.clip);
        let transform: Affine2 = transform.into();
        if let Some(border_color) = border_color.filter(|_| node.border() != BorderRect::ZERO) {
            let colors = [
                border_color.left.to_linear(),
                border_color.top.to_linear(),
                border_color.right.to_linear(),
                border_color.bottom.to_linear(),
            ];
            let mut completed = 0;
            for (i, &color) in colors.iter().enumerate() {
                if color.is_fully_transparent() {
                    continue;
                }
                let mut side_flags = BORDER_FLAGS[i];
                if completed & side_flags != 0 {
                    continue;
                }
                for j in i + 1..4 {
                    if color == colors[j] {
                        side_flags |= BORDER_FLAGS[j];
                    }
                }
                completed |= side_flags;
                out.items.push(UiItem {
                    camera,
                    z: node.stack_index as f32 + z::BORDER,
                    image: AssetId::default(),
                    clip,
                    transform,
                    kind: UiKind::Node {
                        color,
                        rect: Rect::from_corners(Vec2::ZERO, node.size()),
                        atlas_scaling: None,
                        flip_x: false,
                        flip_y: false,
                        radius: node.border_radius().into(),
                        border: node.border(),
                        node_type: NodeType::Border(side_flags),
                    },
                });
            }
        }
        if node.outline_width() <= 0.0 {
            continue;
        }
        if let Some(outline) = outline.filter(|o| !o.color.is_fully_transparent()) {
            out.items.push(UiItem {
                camera,
                z: node.stack_index as f32 + z::BORDER,
                image: AssetId::default(),
                clip,
                transform,
                kind: UiKind::Node {
                    color: outline.color.into(),
                    rect: Rect::from_corners(Vec2::ZERO, node.outlined_node_size()),
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    radius: node.outline_radius().into(),
                    border: BorderRect::all(node.outline_width()),
                    node_type: NodeType::Border(flags::BORDER_ALL),
                },
            });
        }
    }

    // Text shadows: a run breaks on a new span or atlas texture.
    for (node, transform, visibility, clip, target, (layout, shadow)) in &shadows {
        if !visibility.get() || node.is_empty() {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        let transform = Affine2::from(transform)
            * Affine2::from_translation(
                -0.5 * node.size() + shadow.offset / node.inverse_scale_factor,
            );
        let color: LinearRgba = shadow.color.into();
        push_glyph_runs(
            out,
            &atlases,
            &layout.glyphs,
            |_| color,
            |a, b| a.span_index != b.span_index || a.atlas_info.texture != b.atlas_info.texture,
            (camera, node.stack_index, clip.map(|c| c.clip), transform),
        );
    }

    // Text: a run breaks on a new atlas texture; each span's own colour.
    for (node, transform, visibility, clip, target, (block, text_color, layout)) in &texts {
        if !visibility.get() || node.is_empty() {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        let transform = Affine2::from(transform) * Affine2::from_translation(-0.5 * node.size());
        let mut color = text_color.0.to_linear();
        let mut span = 0;
        push_glyph_runs(
            out,
            &atlases,
            &layout.glyphs,
            |glyph| {
                if span != glyph.span_index {
                    if let Some(entity) = block.entities().get(glyph.span_index).map(|t| t.entity) {
                        color = text_styles
                            .get(entity)
                            .map(|c| LinearRgba::from(c.0))
                            .unwrap_or_default();
                        span = glyph.span_index;
                    }
                }
                color
            },
            |a, b| a.atlas_info.texture != b.atlas_info.texture,
            (camera, node.stack_index, clip.map(|c| c.clip), transform),
        );
    }
}

/// Pushes `glyphs` as runs of one atlas texture each, broken where `breaks(glyph, next)`.
fn push_glyph_runs(
    out: &mut UiItems,
    atlases: &Assets<TextureAtlasLayout>,
    glyphs: &[PositionedGlyph],
    mut color: impl FnMut(&PositionedGlyph) -> LinearRgba,
    breaks: impl Fn(&PositionedGlyph, &PositionedGlyph) -> bool,
    (camera, stack_index, clip, transform): (Entity, u32, Option<Rect>, Affine2),
) {
    let mut start = out.glyphs.len();
    for (i, glyph) in glyphs.iter().enumerate() {
        let Some(layout) = atlases.get(glyph.atlas_info.texture_atlas) else {
            continue;
        };
        let Some(rect) = layout.textures.get(glyph.atlas_info.location.glyph_index) else {
            continue;
        };
        out.glyphs.push(Glyph {
            color: color(glyph),
            translation: glyph.position,
            rect: rect.as_rect(),
        });
        if glyphs.get(i + 1).is_none_or(|next| breaks(glyph, next)) {
            let end = out.glyphs.len();
            out.items.push(UiItem {
                camera,
                z: stack_index as f32 + z::TEXT,
                image: glyph.atlas_info.texture,
                clip,
                transform,
                kind: UiKind::Glyphs(start..end),
            });
            start = end;
        }
    }
}

/// `extract_ui_texture_slices`: sliced and tiled images.
fn extract_slices(
    mut out: ResMut<UiItems>,
    lanes: Query<(), With<GfxUiLane>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    image_nodes: Query<NodeItem<&ImageNode>>,
) {
    for (node, transform, visibility, clip, target, image) in &image_nodes {
        if !visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
            || !image.image_mode.uses_slices()
        {
            continue;
        }
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&atlases))
            .map(|r| r.as_rect());
        let atlas_rect = match (atlas_rect, image.rect) {
            (None, None) => None,
            (None, Some(image_rect)) => Some(image_rect),
            (Some(atlas_rect), None) => Some(atlas_rect),
            (Some(atlas_rect), Some(mut image_rect)) => {
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                Some(image_rect)
            }
        };
        out.items.push(UiItem {
            camera,
            z: node.stack_index as f32 + z::IMAGE,
            image: image.image.id(),
            clip: clip.map(|c| c.clip),
            transform: transform.into(),
            kind: UiKind::Slice {
                color: image.color.into(),
                rect: Rect::from_corners(Vec2::ZERO, node.size),
                atlas_rect,
                flip_x: image.flip_x,
                flip_y: image.flip_y,
                mode: image.image_mode.clone(),
                inverse_scale_factor: node.inverse_scale_factor,
            },
        });
    }
}

/// `extract_ui_material_nodes::<M>`; one description per material asset per frame.
#[allow(clippy::type_complexity)]
fn extract_materials<M: UiMaterial>(
    mut out: ResMut<UiItems>,
    mut list: ResMut<DrawList>,
    describe: Res<DescribeUi<M>>,
    materials: Option<Res<Assets<M>>>,
    lanes: Query<(), With<GfxUiLane>>,
    nodes: Query<NodeItem<&MaterialNode<M>>>,
    mut descs: Local<HashMap<AssetId<M>, u32>>,
) {
    descs.clear();
    let Some(materials) = materials else {
        return;
    };
    for (node, transform, visibility, clip, target, handle) in &nodes {
        if !visibility.get() || node.is_empty() {
            continue;
        }
        let id = handle.0.id();
        let desc = match descs.get(&id) {
            Some(d) => *d,
            None => {
                // Bevy skips a material still loading.
                let Some(m) = materials.get(id) else {
                    continue;
                };
                let d = list.push_desc((describe.0)(m));
                descs.insert(id, d);
                d
            }
        };
        let Some(camera) = lane_camera(target, &lanes) else {
            continue;
        };
        out.items.push(UiItem {
            camera,
            z: node.stack_index as f32 + z::MATERIAL,
            image: AssetId::default(),
            clip: clip.map(|c| c.clip),
            transform: transform.into(),
            kind: UiKind::Material {
                desc,
                rect: Rect::from_corners(Vec2::ZERO, node.size()),
                radius: node.border_radius().into(),
                border: node.border(),
            },
        });
    }
}

#[derive(Clone, Copy)]
struct NodeVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    flags: u32,
    radius: [f32; 4],
    border: [f32; 4],
    size: [f32; 2],
    point: [f32; 2],
}

#[derive(Clone, Copy)]
struct SliceVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    slices: [f32; 4],
    border: [f32; 4],
    repeat: [f32; 4],
    atlas: [f32; 4],
}

/// One mesh's frame of vertices.
enum NodeVertices {
    Node(Vec<NodeVertex>),
    Slice(Vec<SliceVertex>),
}

impl NodeVertices {
    fn len(&self) -> usize {
        match self {
            Self::Node(v) => v.len(),
            Self::Slice(v) => v.len(),
        }
    }

    /// Writes the padded streams into `mesh`, growing the capacities to fit first.
    fn fill(&self, mesh: &mut Mesh, stream: &mut Stream, indices: &[u32]) {
        stream.vertices = stream.vertices.max(self.len().next_power_of_two());
        stream.indices = stream.indices.max(indices.len().next_power_of_two());
        let n = stream.vertices;
        fn column<V: Copy, T: Copy + Default>(v: &[V], n: usize, f: impl Fn(&V) -> T) -> Vec<T> {
            let mut c: Vec<T> = v.iter().map(f).collect();
            c.resize(n, T::default());
            c
        }
        match self {
            Self::Node(v) => {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, column(v, n, |v| v.position));
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, column(v, n, |v| v.uv));
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, column(v, n, |v| v.color));
                mesh.insert_attribute(ATTRIBUTE_UI_FLAGS, column(v, n, |v| v.flags));
                mesh.insert_attribute(ATTRIBUTE_UI_RADIUS, column(v, n, |v| v.radius));
                mesh.insert_attribute(ATTRIBUTE_UI_BORDER, column(v, n, |v| v.border));
                mesh.insert_attribute(ATTRIBUTE_UI_SIZE, column(v, n, |v| v.size));
                mesh.insert_attribute(ATTRIBUTE_UI_POINT, column(v, n, |v| v.point));
            }
            Self::Slice(v) => {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, column(v, n, |v| v.position));
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, column(v, n, |v| v.uv));
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, column(v, n, |v| v.color));
                mesh.insert_attribute(ATTRIBUTE_UI_SLICES, column(v, n, |v| v.slices));
                mesh.insert_attribute(ATTRIBUTE_UI_TARGET, column(v, n, |v| v.border));
                mesh.insert_attribute(ATTRIBUTE_UI_REPEAT, column(v, n, |v| v.repeat));
                mesh.insert_attribute(ATTRIBUTE_UI_ATLAS, column(v, n, |v| v.atlas));
            }
        }
        let mut i = indices.to_vec();
        i.resize(stream.indices, 0);
        mesh.insert_indices(Indices::U32(i));
    }
}

/// How far a clip rect pulls each corner in (bevy's `positions_diff`); zero without a clip.
fn clip_diff(positions: &[Vec2; 4], clip: Option<Rect>) -> [Vec2; 4] {
    let Some(clip) = clip else {
        return [Vec2::ZERO; 4];
    };
    [
        Vec2::new(
            f32::max(clip.min.x - positions[0].x, 0.),
            f32::max(clip.min.y - positions[0].y, 0.),
        ),
        Vec2::new(
            f32::min(clip.max.x - positions[1].x, 0.),
            f32::max(clip.min.y - positions[1].y, 0.),
        ),
        Vec2::new(
            f32::min(clip.max.x - positions[2].x, 0.),
            f32::min(clip.max.y - positions[2].y, 0.),
        ),
        Vec2::new(
            f32::max(clip.min.x - positions[3].x, 0.),
            f32::min(clip.max.y - positions[3].y, 0.),
        ),
    ]
}

/// Whether the clip leaves nothing of the quad (never for a rotated one, as bevy skips the test).
fn clipped_away(transform: &Affine2, diff: &[Vec2; 4], rect_size: Vec2, test: bool) -> bool {
    let size = transform.transform_vector2(rect_size).abs();
    test && (diff[0].x - diff[1].x >= size.x || diff[1].y - diff[2].y >= size.y)
}

/// The UV rect's four corners, pulled in by the clip.
fn clipped_uvs(rect: Rect, diff: &[Vec2; 4]) -> [Vec2; 4] {
    [
        Vec2::new(rect.min.x + diff[0].x, rect.min.y + diff[0].y),
        Vec2::new(rect.max.x + diff[1].x, rect.min.y + diff[1].y),
        Vec2::new(rect.max.x + diff[2].x, rect.max.y + diff[2].y),
        Vec2::new(rect.min.x + diff[3].x, rect.max.y + diff[3].y),
    ]
}

fn border_row(b: BorderRect) -> [f32; 4] {
    [b.min_inset.x, b.min_inset.y, b.max_inset.x, b.max_inset.y]
}

/// `compute_texture_slices`: the texture slices, target slices and repeats of a sliced or tiled
/// image of `image_size` drawn at `target_size`.
fn compute_texture_slices(
    image_size: Vec2,
    target_size: Vec2,
    mode: &NodeImageMode,
) -> [[f32; 4]; 3] {
    match mode {
        NodeImageMode::Sliced(TextureSlicer {
            border,
            center_scale_mode,
            sides_scale_mode,
            max_corner_scale,
        }) => {
            let min_coeff = (target_size / image_size)
                .min_element()
                .min(*max_corner_scale);
            let slices = [
                border.min_inset.x / image_size.x,
                border.min_inset.y / image_size.y,
                1. - border.max_inset.x / image_size.x,
                1. - border.max_inset.y / image_size.y,
            ];
            let target = [
                (border.min_inset.x / target_size.x) * min_coeff,
                (border.min_inset.y / target_size.y) * min_coeff,
                1. - (border.max_inset.x / target_size.x) * min_coeff,
                1. - (border.max_inset.y / target_size.y) * min_coeff,
            ];
            let image_side_width = image_size.x * (slices[2] - slices[0]);
            let image_side_height = image_size.y * (slices[3] - slices[1]);
            let target_side_width = target_size.x * (target[2] - target[0]);
            let target_side_height = target_size.y * (target[3] - target[1]);
            [
                slices,
                target,
                [
                    tiled_subaxis(image_side_width, target_side_width, sides_scale_mode),
                    tiled_subaxis(image_side_height, target_side_height, sides_scale_mode),
                    tiled_subaxis(image_side_width, target_side_width, center_scale_mode),
                    tiled_subaxis(image_side_height, target_side_height, center_scale_mode),
                ],
            ]
        }
        NodeImageMode::Tiled {
            tile_x,
            tile_y,
            stretch_value,
        } => {
            let rx = tiled_axis(*tile_x, image_size.x, target_size.x, *stretch_value);
            let ry = tiled_axis(*tile_y, image_size.y, target_size.y, *stretch_value);
            [[0., 0., 1., 1.], [0., 0., 1., 1.], [1., 1., rx, ry]]
        }
        NodeImageMode::Auto | NodeImageMode::Stretch => {
            unreachable!("only sliced and tiled images are extracted as slices")
        }
    }
}

fn tiled_axis(tile: bool, image_extent: f32, target_extent: f32, stretch: f32) -> f32 {
    if tile {
        target_extent / (image_extent * stretch)
    } else {
        1.
    }
}

fn tiled_subaxis(image_extent: f32, target_extent: f32, mode: &SliceScaleMode) -> f32 {
    match mode {
        SliceScaleMode::Stretch => 1.,
        SliceScaleMode::Tile { stretch_value } => target_extent / (image_extent * *stretch_value),
    }
}

/// Which program a batch draws through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Node,
    Slice,
    Material(u32),
}

/// A run of consecutive items drawn as one: its program, image and index range.
struct Batch {
    camera: Entity,
    lane: Lane,
    image: AssetId<Image>,
    indices: Range<u32>,
}

/// bevy's `queue_uinodes`, `queue_ui_slices`, `queue_ui_material_nodes`, the `TransparentUi`
/// sort and the three `prepare_*`: sorts each lane camera's items, batches them, writes the
/// quads into the streams and pushes one late draw per batch.
#[allow(clippy::too_many_arguments)]
fn queue(
    mut items: ResMut<UiItems>,
    programs: Res<UiPrograms>,
    mut streams: Option<ResMut<UiStreams>>,
    mut list: ResMut<DrawList>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut renderer: NonSendMut<GfxRenderer>,
    images: Res<Assets<Image>>,
    lanes: Query<(Entity, &Camera, Option<&UiAntiAlias>), With<GfxUiLane>>,
) {
    let UiItems { items, glyphs } = &mut *items;
    let Some(streams) = streams.as_deref_mut() else {
        items.clear();
        glyphs.clear();
        return;
    };
    // By camera, then z; stable, so ties keep the extraction order.
    items.sort_by(|a, b| a.camera.cmp(&b.camera).then(a.z.total_cmp(&b.z)));

    let mut node_vertices = Vec::new();
    let mut node_indices: Vec<u32> = Vec::new();
    let mut slice_vertices = Vec::new();
    let mut slice_indices: Vec<u32> = Vec::new();
    let mut batches: Vec<Batch> = Vec::new();
    let size_of = |id: AssetId<Image>| images.get(id).map(|i| i.size().as_vec2());
    let mut batch_image = AssetId::<Image>::invalid();

    for item in items.iter() {
        let lane = match item.kind {
            UiKind::Node { .. } | UiKind::Glyphs(_) => Lane::Node,
            UiKind::Slice { .. } => Lane::Slice,
            UiKind::Material { desc, .. } => Lane::Material(desc),
        };
        let image = item.image;
        let textured = image != AssetId::default();
        // An image still loading drops its item, as bevy's missing `GpuImage` does.
        let image_size = if textured {
            match size_of(image) {
                Some(s) => Some(s),
                None => continue,
            }
        } else {
            None
        };
        let new_batch = match batches.last() {
            None => true,
            Some(b) => {
                b.camera != item.camera
                    || b.lane != lane
                    || matches!(lane, Lane::Material(_))
                    || batch_image == AssetId::invalid()
                    || (batch_image != AssetId::default() && textured && batch_image != image)
            }
        };
        let (vertex_start, index_start) = match lane {
            Lane::Slice => (slice_vertices.len() as u32, slice_indices.len() as u32),
            _ => (node_vertices.len() as u32, node_indices.len() as u32),
        };
        if new_batch {
            batch_image = image;
            batches.push(Batch {
                camera: item.camera,
                lane,
                image,
                indices: index_start..index_start,
            });
        } else if batch_image == AssetId::default() && textured {
            batch_image = image;
            if let Some(b) = batches.last_mut() {
                b.image = image;
            }
        }

        let transform = item.transform;
        match &item.kind {
            UiKind::Node {
                color,
                rect,
                atlas_scaling,
                flip_x,
                flip_y,
                radius,
                border,
                node_type,
            } => {
                let mut uinode_rect = *rect;
                let rect_size = uinode_rect.size();
                let positions =
                    QUAD_VERTEX_POSITIONS.map(|p| transform.transform_point2(p * rect_size));
                let points = QUAD_VERTEX_POSITIONS.map(|p| p * rect_size);
                let mut diff = clip_diff(&positions, item.clip);
                let clipped = [0, 1, 2, 3].map(|i| (positions[i] + diff[i]).extend(0.0));
                let points = [0, 1, 2, 3].map(|i| points[i] + diff[i]);
                if clipped_away(&transform, &diff, rect_size, transform.x_axis[1] == 0.0) {
                    continue;
                }
                let uvs = match image_size {
                    None => [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                    Some(size) => {
                        let extent = atlas_scaling.map_or(uinode_rect.max, |s| size * s);
                        if *flip_x {
                            std::mem::swap(&mut uinode_rect.max.x, &mut uinode_rect.min.x);
                            diff.iter_mut().for_each(|d| d.x *= -1.0);
                        }
                        if *flip_y {
                            std::mem::swap(&mut uinode_rect.max.y, &mut uinode_rect.min.y);
                            diff.iter_mut().for_each(|d| d.y *= -1.0);
                        }
                        clipped_uvs(uinode_rect, &diff).map(|p| p / extent)
                    }
                };
                let mut f = if textured { flags::TEXTURED } else { 0 };
                if let NodeType::Border(b) = *node_type {
                    f |= b;
                }
                for i in 0..4 {
                    node_vertices.push(NodeVertex {
                        position: clipped[i].into(),
                        uv: uvs[i].into(),
                        color: color.to_f32_array(),
                        flags: f | flags::CORNERS[i],
                        radius: *radius,
                        border: border_row(*border),
                        size: rect_size.into(),
                        point: points[i].into(),
                    });
                }
                node_indices.extend(QUAD_INDICES.map(|i| vertex_start + i));
            }
            UiKind::Glyphs(range) => {
                let extent = image_size.unwrap_or(Vec2::ONE);
                for glyph in &glyphs[range.clone()] {
                    let rect_size = glyph.rect.size();
                    let positions = QUAD_VERTEX_POSITIONS
                        .map(|p| transform.transform_point2(glyph.translation + p * rect_size));
                    let diff = clip_diff(&positions, item.clip);
                    let clipped = [0, 1, 2, 3].map(|i| (positions[i] + diff[i]).extend(0.0));
                    if clipped_away(&transform, &diff, rect_size, true) {
                        continue;
                    }
                    let uvs = clipped_uvs(glyph.rect, &diff).map(|p| p / extent);
                    let base = node_vertices.len() as u32;
                    for i in 0..4 {
                        node_vertices.push(NodeVertex {
                            position: clipped[i].into(),
                            uv: uvs[i].into(),
                            color: glyph.color.to_f32_array(),
                            flags: flags::TEXTURED | flags::CORNERS[i],
                            radius: [0.0; 4],
                            border: [0.0; 4],
                            size: rect_size.into(),
                            point: [0.0; 2],
                        });
                    }
                    node_indices.extend(QUAD_INDICES.map(|i| base + i));
                }
            }
            UiKind::Slice {
                color,
                rect,
                atlas_rect,
                flip_x,
                flip_y,
                mode,
                inverse_scale_factor,
            } => {
                let rect_size = rect.size();
                let positions =
                    QUAD_VERTEX_POSITIONS.map(|p| transform.transform_point2(p * rect_size));
                let diff = clip_diff(&positions, item.clip);
                let clipped = [0, 1, 2, 3].map(|i| (positions[i] + diff[i]).extend(0.0));
                if clipped_away(&transform, &diff, rect_size, transform.x_axis[1] == 0.0) {
                    continue;
                }
                let uvs = if textured {
                    clipped_uvs(*rect, &diff).map(|p| p / rect.max)
                } else {
                    [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
                };
                // The batch's image size, as bevy reads `batch_image_size`.
                let batch_size = size_of(batch_image).unwrap_or(Vec2::ONE);
                let (slice_image, mut atlas) = match atlas_rect {
                    Some(a) => (
                        a.size(),
                        [
                            a.min.x / batch_size.x,
                            a.min.y / batch_size.y,
                            a.max.x / batch_size.x,
                            a.max.y / batch_size.y,
                        ],
                    ),
                    None => (batch_size, [0., 0., 1., 1.]),
                };
                if *flip_x {
                    atlas.swap(0, 2);
                }
                if *flip_y {
                    atlas.swap(1, 3);
                }
                let [slices, target, repeat] =
                    compute_texture_slices(slice_image, rect_size * *inverse_scale_factor, mode);
                for i in 0..4 {
                    slice_vertices.push(SliceVertex {
                        position: clipped[i].into(),
                        uv: uvs[i].into(),
                        color: color.to_f32_array(),
                        slices,
                        border: target,
                        repeat,
                        atlas,
                    });
                }
                slice_indices.extend(QUAD_INDICES.map(|i| vertex_start + i));
            }
            UiKind::Material {
                rect,
                radius,
                border,
                ..
            } => {
                let rect_size = rect.size();
                let positions =
                    QUAD_VERTEX_POSITIONS.map(|p| transform.transform_point2(p * rect_size));
                let diff = clip_diff(&positions, item.clip);
                // The material vertex sits at z 1, as `prepare_uimaterial_nodes` extends it.
                let clipped = [0, 1, 2, 3].map(|i| (positions[i] + diff[i]).extend(1.0));
                if clipped_away(&transform, &diff, rect_size, transform.x_axis[1] == 0.0) {
                    continue;
                }
                let uvs = clipped_uvs(*rect, &diff).map(|p| p / rect.max);
                for i in 0..4 {
                    node_vertices.push(NodeVertex {
                        position: clipped[i].into(),
                        uv: uvs[i].into(),
                        color: [0.0; 4],
                        flags: 0,
                        radius: *radius,
                        border: border_row(*border),
                        size: rect_size.into(),
                        point: [0.0; 2],
                    });
                }
                node_indices.extend(QUAD_INDICES.map(|i| vertex_start + i));
            }
        }
        if let Some(b) = batches.last_mut() {
            b.indices.end = match lane {
                Lane::Slice => slice_indices.len() as u32,
                _ => node_indices.len() as u32,
            };
        }
    }
    items.clear();
    glyphs.clear();

    let mut write = |stream: &mut Stream, vertices: NodeVertices, indices: &[u32]| {
        let id = stream.mesh.id();
        if indices.is_empty() {
            return;
        }
        let Some(mesh) = meshes.get_mut_untracked(id) else {
            return;
        };
        vertices.fill(mesh, stream, indices);
        // Untracked, so the device copy is marked here, before this frame's draw rewrites it.
        renderer.meshes.modified(id);
    };
    write(
        &mut streams.node,
        NodeVertices::Node(node_vertices),
        &node_indices,
    );
    write(
        &mut streams.slice,
        NodeVertices::Slice(slice_vertices),
        &slice_indices,
    );

    // One description per (lane, image, anti-alias) this frame.
    let mut descs: HashMap<(u8, AssetId<Image>, bool), u32> = HashMap::new();
    for batch in batches {
        if batch.indices.is_empty() {
            continue;
        }
        let Ok((_, camera, aa)) = lanes.get(batch.camera) else {
            continue;
        };
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };
        let clip_from_world = ui_clip_from_world(viewport.size().as_vec2());
        let anti_alias = !matches!(aa, Some(UiAntiAlias::Off));
        let (mesh, desc) = match batch.lane {
            Lane::Material(desc) => (streams.node.mesh.id(), desc),
            lane => {
                let (program, rank, mesh) = match lane {
                    Lane::Slice => (programs.slice, 1u8, streams.slice.mesh.id()),
                    _ => (programs.node, 0, streams.node.mesh.id()),
                };
                let desc = *descs
                    .entry((rank, batch.image, anti_alias))
                    .or_insert_with(|| list.push_desc(node_desc(program, batch.image, anti_alias)));
                (mesh, desc)
            }
        };
        list.push_late(
            batch.camera,
            LateDraw {
                mesh,
                indices: batch.indices,
                clip_from_world,
                desc,
            },
        );
    }
}

/// bevy's UI view over a viewport of `size` physical pixels: origin top left, y down, the camera
/// `UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET` back.
fn ui_clip_from_world(size: Vec2) -> Mat4 {
    let clip_from_view = Mat4::orthographic_rh(0.0, size.x, size.y, 0.0, 0.0, UI_CAMERA_FAR);
    let world_from_view = Mat4::from_translation(Vec3::new(
        0.0,
        0.0,
        UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
    ));
    clip_from_view * world_from_view.inverse()
}

/// A node or slice batch: its image (the default image is untextured, white), `ALPHA_BLENDING`,
/// no depth; `params[0].x` is `ANTI_ALIAS`.
fn node_desc(program: GfxProgram, image: AssetId<Image>, anti_alias: bool) -> GfxMaterialDesc {
    let mut params = [[0.0; 4]; crate::material::MAX_PARAMS];
    params[0][0] = if anti_alias { 1.0 } else { 0.0 };
    let texture = if image == AssetId::default() {
        GfxTextureSlot::White
    } else {
        GfxTextureSlot::Image(image)
    };
    GfxMaterialDesc {
        program,
        textures: [
            texture,
            GfxTextureSlot::White,
            GfxTextureSlot::White,
            GfxTextureSlot::White,
        ],
        params,
        alpha: GfxAlpha::Blend,
        cull: None,
        state: GfxDrawState {
            blend: Some(Blend::Alpha),
            depth_test: false,
            depth_write: Some(false),
            ..default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ui_view_maps_the_viewport_top_left_down() {
        let m = ui_clip_from_world(Vec2::new(800.0, 600.0));
        let tl = m * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let br = m * Vec4::new(800.0, 600.0, 0.0, 1.0);
        assert!((tl.x + 1.0).abs() < 1e-6 && (tl.y - 1.0).abs() < 1e-6);
        assert!((br.x - 1.0).abs() < 1e-6 && (br.y + 1.0).abs() < 1e-6);
        // Inside [0, 1], the clip depth range on every device.
        assert!(tl.z > 0.0 && tl.z < 1.0);
    }

    #[test]
    fn a_clip_pulls_the_corners_in() {
        let positions = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let clip = Rect::new(2.0, 3.0, 8.0, 10.0);
        let d = clip_diff(&positions, Some(clip));
        assert_eq!(d[0], Vec2::new(2.0, 3.0));
        assert_eq!(d[1], Vec2::new(-2.0, 3.0));
        assert_eq!(d[2], Vec2::new(-2.0, 0.0));
        assert!(!clipped_away(
            &Affine2::IDENTITY,
            &d,
            Vec2::splat(10.0),
            true
        ));
        let gone = clip_diff(&positions, Some(Rect::new(20.0, 20.0, 30.0, 30.0)));
        assert!(clipped_away(
            &Affine2::IDENTITY,
            &gone,
            Vec2::splat(10.0),
            true
        ));
    }

    #[test]
    fn tiling_repeats_by_the_target_over_the_stretched_image() {
        let mode = NodeImageMode::Tiled {
            tile_x: true,
            tile_y: false,
            stretch_value: 1.0,
        };
        let [slices, target, repeat] =
            compute_texture_slices(Vec2::new(16.0, 16.0), Vec2::new(64.0, 32.0), &mode);
        assert_eq!(slices, [0., 0., 1., 1.]);
        assert_eq!(target, [0., 0., 1., 1.]);
        assert_eq!(repeat, [1., 1., 4., 1.]);
    }
}
