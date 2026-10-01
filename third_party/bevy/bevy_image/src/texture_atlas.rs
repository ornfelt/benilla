use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp as _, Assets, Handle};
use bevy_math::{URect, UVec2};
use bevy_reflect::TypePath;

/// Adds support for texture atlases.
pub struct TextureAtlasPlugin;

impl Plugin for TextureAtlasPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<TextureAtlasLayout>();
    }
}

/// Stores a map used to lookup the position of a texture in a [`TextureAtlas`].
/// This can be used to either use and look up a specific section of a texture, or animate frame-by-frame as a sprite sheet.
///
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage animating sprite in response to an event.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_animation.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Asset, TypePath, PartialEq, Eq, Debug, Clone)]
pub struct TextureAtlasLayout {
    /// Total size of texture atlas.
    pub size: UVec2,
    /// The specific areas of the atlas where each texture can be found
    pub textures: Vec<URect>,
}

impl TextureAtlasLayout {
    /// Create a new empty layout with custom `dimensions`
    pub fn new_empty(dimensions: UVec2) -> Self {
        Self {
            size: dimensions,
            textures: Vec::new(),
        }
    }

    /// Add a *section* to the list in the layout and returns its index
    /// which can be used with [`TextureAtlas`]
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the texture to be added
    ///
    /// [`TextureAtlas`]: crate::TextureAtlas
    pub fn add_texture(&mut self, rect: URect) -> usize {
        self.textures.push(rect);
        self.textures.len() - 1
    }

    /// The number of textures in the [`TextureAtlasLayout`]
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

/// An index into a [`TextureAtlasLayout`], which corresponds to a specific section of a texture.
///
/// It stores a handle to [`TextureAtlasLayout`] and the index of the current section of the atlas.
/// The texture atlas contains various *sections* of a given texture, allowing users to have a single
/// image file for either sprite animation or global mapping.
/// You can change the texture [`index`](Self::index) of the atlas to animate the sprite or display only a *section* of the texture
/// for efficient rendering of related game objects.
///
/// Check the following examples for usage:
/// - [`animated sprite sheet example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// - [`sprite animation event example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_animation.rs)
/// - [`texture atlas example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAtlas {
    /// Texture atlas layout handle
    pub layout: Handle<TextureAtlasLayout>,
    /// Texture atlas section index
    pub index: usize,
}

impl TextureAtlas {
    /// Retrieves the current texture [`URect`] of the sprite sheet according to the section `index`
    pub fn texture_rect(&self, texture_atlases: &Assets<TextureAtlasLayout>) -> Option<URect> {
        let atlas = texture_atlases.get(&self.layout)?;
        atlas.textures.get(self.index).copied()
    }
}

impl From<Handle<TextureAtlasLayout>> for TextureAtlas {
    fn from(texture_atlas: Handle<TextureAtlasLayout>) -> Self {
        Self {
            layout: texture_atlas,
            index: 0,
        }
    }
}
