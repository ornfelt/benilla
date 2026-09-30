use bevy_asset::{Asset, AssetId};
use bevy_ecs::prelude::Resource;
use bevy_platform::collections::HashMap;

/// The GPU representation of a main-world asset.
///
/// Bevy extracted the [`RenderAsset::SourceAsset`] into the render world and prepared it there;
/// no render world exists, so only the pairing of the two types is left, which
/// [`RenderAssets`] is keyed by.
pub trait RenderAsset: Send + Sync + 'static + Sized {
    /// The representation of the asset in the "main world".
    type SourceAsset: Asset + Clone;
}

/// Stores all GPU representations ([`RenderAsset`])
/// of [`RenderAsset::SourceAsset`] as long as they exist.
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A::SourceAsset>, A>);

impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<A: RenderAsset> RenderAssets<A> {
    pub fn get(&self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&mut A> {
        self.0.get_mut(&id.into())
    }

    pub fn insert(&mut self, id: impl Into<AssetId<A::SourceAsset>>, value: A) -> Option<A> {
        self.0.insert(id.into(), value)
    }

    pub fn remove(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<A> {
        self.0.remove(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &A)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &mut A)> {
        self.0.iter_mut().map(|(k, v)| (*k, v))
    }
}

/// A resource that defines the amount of data allowed to be transferred from CPU to GPU
/// each frame, preventing choppy frames at the cost of waiting longer for GPU assets
/// to become available.
#[derive(Resource, Default)]
pub struct RenderAssetBytesPerFrame {
    pub max_bytes: Option<usize>,
}

impl RenderAssetBytesPerFrame {
    /// `max_bytes`: the number of bytes to write per frame.
    ///
    /// This is a soft limit: only full assets are written currently, uploading stops
    /// after the first asset that exceeds the limit.
    ///
    /// To participate, assets should implement `RenderAsset::byte_len`. If the default
    /// is not overridden, the assets are assumed to be small enough to upload without restriction.
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: Some(max_bytes),
        }
    }
}
