use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use atomicow::CowArc;
use bevy_ecs::resource::Resource;
use bevy_platform::collections::HashMap;
use core::{fmt::Display, hash::Hash};
use thiserror::Error;
use tracing::warn;

use super::ErasedAssetReader;

/// A reference to an "asset source", which maps to an [`AssetReader`](crate::io::AssetReader).
///
/// * [`AssetSourceId::Default`] corresponds to "default asset paths" that don't specify a source: `/path/to/asset.png`
/// * [`AssetSourceId::Name`] corresponds to asset paths that _do_ specify a source: `remote://path/to/asset.png`, where `remote` is the name.
#[derive(Default, Clone, Debug, Eq)]
pub enum AssetSourceId<'a> {
    /// The default asset source.
    #[default]
    Default,
    /// A non-default named asset source.
    Name(CowArc<'a, str>),
}

impl<'a> Display for AssetSourceId<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.as_str() {
            None => write!(f, "AssetSourceId::Default"),
            Some(v) => write!(f, "AssetSourceId::Name({v})"),
        }
    }
}

impl<'a> AssetSourceId<'a> {
    /// Returns [`None`] if this is [`AssetSourceId::Default`] and [`Some`] containing the
    /// name if this is [`AssetSourceId::Name`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AssetSourceId::Default => None,
            AssetSourceId::Name(v) => Some(v),
        }
    }

    /// If this is not already an owned / static id, create one. Otherwise, it will return itself (with a static lifetime).
    pub fn into_owned(self) -> AssetSourceId<'static> {
        match self {
            AssetSourceId::Default => AssetSourceId::Default,
            AssetSourceId::Name(v) => AssetSourceId::Name(v.into_owned()),
        }
    }

    /// Clones into an owned [`AssetSourceId<'static>`].
    /// This is equivalent to `.clone().into_owned()`.
    #[inline]
    pub fn clone_owned(&self) -> AssetSourceId<'static> {
        self.clone().into_owned()
    }
}

// This is only implemented for static lifetimes to ensure `Path::clone` does not allocate
// by ensuring that this is stored as a `CowArc::Static`.
// Please read https://github.com/bevyengine/bevy/issues/19844 before changing this!
impl From<&'static str> for AssetSourceId<'static> {
    fn from(value: &'static str) -> Self {
        AssetSourceId::Name(value.into())
    }
}

impl<'a, 'b> From<&'a AssetSourceId<'b>> for AssetSourceId<'b> {
    fn from(value: &'a AssetSourceId<'b>) -> Self {
        value.clone()
    }
}

impl From<Option<&'static str>> for AssetSourceId<'static> {
    fn from(value: Option<&'static str>) -> Self {
        match value {
            Some(value) => AssetSourceId::Name(value.into()),
            None => AssetSourceId::Default,
        }
    }
}

impl From<String> for AssetSourceId<'static> {
    fn from(value: String) -> Self {
        AssetSourceId::Name(value.into())
    }
}

impl<'a> Hash for AssetSourceId<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<'a> PartialEq for AssetSourceId<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(&other.as_str())
    }
}

/// Metadata about an "asset source", such as how to construct the [`AssetReader`](crate::io::AssetReader) for the source,
/// and whether or not the source is processed.
pub struct AssetSourceBuilder {
    /// The [`ErasedAssetReader`] to use on the unprocessed asset.
    pub reader: Box<dyn FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync>,
    /// The [`ErasedAssetReader`] to use for processed assets.
    pub processed_reader: Option<Box<dyn FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync>>,
    /// The warning message to display when watching an unprocessed asset fails.
    pub watch_warning: Option<&'static str>,
    /// The warning message to display when watching a processed asset fails.
    pub processed_watch_warning: Option<&'static str>,
}

impl AssetSourceBuilder {
    /// Creates a new builder, starting with the provided reader.
    pub fn new(
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> AssetSourceBuilder {
        Self {
            reader: Box::new(reader),
            processed_reader: None,
            watch_warning: None,
            processed_watch_warning: None,
        }
    }

    /// Builds a new [`AssetSource`] with the given `id`. If `watch` is true, the unprocessed source will watch for changes.
    /// If `watch_processed` is true, the processed source will watch for changes.
    pub fn build(
        &mut self,
        id: AssetSourceId<'static>,
        watch: bool,
        watch_processed: bool,
    ) -> AssetSource {
        let reader = self.reader.as_mut()();
        let source = AssetSource {
            id: id.clone(),
            reader,
            processed_reader: self
                .processed_reader
                .as_mut()
                .map(|r| r())
                .map(Into::<Arc<_>>::into),
        };

        // No `AssetWatcher` exists in this build: a source asked to watch only logs its warning.
        if watch && let Some(warning) = self.watch_warning {
            warn!("{id} does not have an AssetWatcher configured. {warning}");
        }

        if watch_processed && let Some(warning) = self.processed_watch_warning {
            warn!("{id} does not have a processed AssetWatcher configured. {warning}");
        }
        source
    }

    /// Will use the given `reader` function to construct processed [`AssetReader`](crate::io::AssetReader) instances.
    pub fn with_processed_reader(
        mut self,
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.processed_reader = Some(Box::new(reader));
        self
    }

    /// Enables a warning for the unprocessed source watcher, which will print when watching is enabled and the unprocessed source doesn't have a watcher.
    pub fn with_watch_warning(mut self, warning: &'static str) -> Self {
        self.watch_warning = Some(warning);
        self
    }

    /// Enables a warning for the processed source watcher, which will print when watching is enabled and the processed source doesn't have a watcher.
    pub fn with_processed_watch_warning(mut self, warning: &'static str) -> Self {
        self.processed_watch_warning = Some(warning);
        self
    }

    /// Returns a builder containing the "platform default source" for the given `path` and `processed_path`.
    /// This uses [`FileAssetReader`](crate::io::file::FileAssetReader).
    pub fn platform_default(path: &str, processed_path: Option<&str>) -> Self {
        let default = Self::new(AssetSource::get_default_reader(path.to_string()))
            .with_watch_warning(AssetSource::get_default_watch_warning());
        if let Some(processed_path) = processed_path {
            default
                .with_processed_reader(AssetSource::get_default_reader(processed_path.to_string()))
                .with_processed_watch_warning(AssetSource::get_default_watch_warning())
        } else {
            default
        }
    }
}

/// A [`Resource`] that hold (repeatable) functions capable of producing new [`AssetReader`](crate::io::AssetReader) instances
/// for a given asset source.
#[derive(Resource, Default)]
pub struct AssetSourceBuilders {
    sources: HashMap<CowArc<'static, str>, AssetSourceBuilder>,
    default: Option<AssetSourceBuilder>,
}

impl AssetSourceBuilders {
    /// Inserts a new builder with the given `id`
    pub fn insert(&mut self, id: impl Into<AssetSourceId<'static>>, source: AssetSourceBuilder) {
        match id.into() {
            AssetSourceId::Default => {
                self.default = Some(source);
            }
            AssetSourceId::Name(name) => {
                self.sources.insert(name, source);
            }
        }
    }

    /// Builds a new [`AssetSources`] collection. If `watch` is true, the unprocessed sources will watch for changes.
    /// If `watch_processed` is true, the processed sources will watch for changes.
    pub fn build_sources(&mut self, watch: bool, watch_processed: bool) -> AssetSources {
        let mut sources = <HashMap<_, _>>::default();
        for (id, source) in &mut self.sources {
            let source = source.build(
                AssetSourceId::Name(id.clone_owned()),
                watch,
                watch_processed,
            );
            sources.insert(id.clone_owned(), source);
        }

        AssetSources {
            sources,
            default: self
                .default
                .as_mut()
                .map(|p| p.build(AssetSourceId::Default, watch, watch_processed))
                .expect(MISSING_DEFAULT_SOURCE),
        }
    }

    /// Initializes the default [`AssetSourceBuilder`] if it has not already been set.
    pub fn init_default_source(&mut self, path: &str, processed_path: Option<&str>) {
        self.default
            .get_or_insert_with(|| AssetSourceBuilder::platform_default(path, processed_path));
    }
}

/// A collection of unprocessed and processed [`AssetReader`](crate::io::AssetReader) instances
/// for a specific asset source, identified by an [`AssetSourceId`].
pub struct AssetSource {
    id: AssetSourceId<'static>,
    reader: Box<dyn ErasedAssetReader>,
    processed_reader: Option<Arc<dyn ErasedAssetReader>>,
}

impl AssetSource {
    /// Return's this source's unprocessed [`AssetReader`](crate::io::AssetReader).
    #[inline]
    pub fn reader(&self) -> &dyn ErasedAssetReader {
        &*self.reader
    }

    /// Return's this source's processed [`AssetReader`](crate::io::AssetReader), if it exists.
    #[inline]
    pub fn processed_reader(
        &self,
    ) -> Result<&dyn ErasedAssetReader, MissingProcessedAssetReaderError> {
        self.processed_reader
            .as_deref()
            .ok_or_else(|| MissingProcessedAssetReaderError(self.id.clone_owned()))
    }

    /// Returns a builder function for this platform's default [`AssetReader`](crate::io::AssetReader). `path` is the relative path to
    /// the asset root.
    pub fn get_default_reader(
        path: String,
    ) -> impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync {
        move || Box::new(super::file::FileAssetReader::new(&path))
    }

    /// Returns the default non-existent `AssetWatcher` warning for the current platform.
    pub fn get_default_watch_warning() -> &'static str {
        "Consider enabling the `file_watcher` feature."
    }
}

/// A collection of [`AssetSource`]s.
pub struct AssetSources {
    sources: HashMap<CowArc<'static, str>, AssetSource>,
    default: AssetSource,
}

impl AssetSources {
    /// Gets the [`AssetSource`] with the given `id`, if it exists.
    pub fn get<'a, 'b>(
        &'a self,
        id: impl Into<AssetSourceId<'b>>,
    ) -> Result<&'a AssetSource, MissingAssetSourceError> {
        match id.into().into_owned() {
            AssetSourceId::Default => Ok(&self.default),
            AssetSourceId::Name(name) => self
                .sources
                .get(&name)
                .ok_or(MissingAssetSourceError(AssetSourceId::Name(name))),
        }
    }
}

/// An error returned when an [`AssetSource`] does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not exist")]
pub struct MissingAssetSourceError(AssetSourceId<'static>);

/// An error returned when a processed [`AssetReader`](crate::io::AssetReader) does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not have a processed AssetReader.")]
pub struct MissingProcessedAssetReaderError(AssetSourceId<'static>);

const MISSING_DEFAULT_SOURCE: &str =
    "A default AssetSource is required. Add one to `AssetSourceBuilders`";
