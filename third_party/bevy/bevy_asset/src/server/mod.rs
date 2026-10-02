mod info;
mod loaders;

use crate::{
    io::{
        AssetReaderError, AssetSource, AssetSourceId, AssetSources, MissingAssetSourceError, Reader,
    },
    loader::{AssetLoader, ErasedAssetLoader, LoadContext, LoadedAsset},
    meta::{
        loader_settings_meta_transform, AssetActionMinimal, AssetMetaDyn, AssetMetaMinimal,
        MetaTransform, Settings,
    },
    path::AssetPath,
    Asset, AssetEvent, AssetHandleProvider, AssetIndex, AssetLoadFailedEvent, Assets,
    DeserializeMetaError, ErasedAssetIndex, ErasedLoadedAsset, Handle, UntypedAssetId,
    UntypedHandle,
};
use alloc::{borrow::ToOwned, boxed::Box, vec, vec::Vec};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use bevy_diagnostic::{DiagnosticPath, Diagnostics};
use bevy_ecs::prelude::*;
use bevy_platform::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use bevy_tasks::IoTaskPool;
use core::{any::TypeId, panic::AssertUnwindSafe};
use crossbeam_channel::{Receiver, Sender};
use either::Either;
use futures_lite::FutureExt;
use info::*;
use loaders::*;
use thiserror::Error;
use tracing::error;

/// Loads and tracks the state of [`Asset`] values from a configured [`AssetReader`](crate::io::AssetReader).
/// This can be used to kick off new asset loads and retrieve their current load states.
///
/// The general process to load an asset is:
/// 1. Initialize a new [`Asset`] type with the [`AssetServer`] via [`AssetApp::init_asset`], which
///    will internally call [`AssetServer::register_asset`] and set up related ECS [`Assets`]
///    storage and systems.
/// 2. Register one or more [`AssetLoader`]s for that asset with [`AssetApp::init_asset_loader`]
/// 3. Add the asset to your asset folder (defaults to `assets`).
/// 4. Call [`AssetServer::load`] with a path to your asset.
///
/// [`AssetServer`] can be cloned. It is backed by an [`Arc`] so clones will share state. Clones can be freely used in parallel.
///
/// [`AssetApp::init_asset`]: crate::AssetApp::init_asset
/// [`AssetApp::init_asset_loader`]: crate::AssetApp::init_asset_loader
#[derive(Resource, Clone)]
pub struct AssetServer {
    pub(crate) data: Arc<AssetServerData>,
}

/// Internal data used by [`AssetServer`]. This is intended to be used from within an [`Arc`].
pub(crate) struct AssetServerData {
    pub(crate) infos: RwLock<AssetInfos>,
    pub(crate) loaders: Arc<RwLock<AssetLoaders>>,
    asset_event_sender: Sender<InternalAssetEvent>,
    asset_event_receiver: Receiver<InternalAssetEvent>,
    sources: Arc<AssetSources>,
}

impl AssetServer {
    /// The number of loads that have been started by the server.
    pub const STARTED_LOAD_COUNT: DiagnosticPath = DiagnosticPath::const_new("started_load_count");

    /// Create a new instance of [`AssetServer`]. If `watching_for_changes` is true, the server runs as a watching
    /// server; no `AssetWatcher` exists in this build, so nothing is hot-reloaded.
    pub fn new_with_meta_check(sources: Arc<AssetSources>, watching_for_changes: bool) -> Self {
        Self::new_with_loaders(sources, Default::default(), watching_for_changes)
    }

    pub(crate) fn new_with_loaders(
        sources: Arc<AssetSources>,
        loaders: Arc<RwLock<AssetLoaders>>,
        watching_for_changes: bool,
    ) -> Self {
        let (asset_event_sender, asset_event_receiver) = crossbeam_channel::unbounded();
        let mut infos = AssetInfos::default();
        infos.watching_for_changes = watching_for_changes;
        Self {
            data: Arc::new(AssetServerData {
                sources,
                asset_event_sender,
                asset_event_receiver,
                loaders,
                infos: RwLock::new(infos),
            }),
        }
    }

    pub(crate) fn read_infos(&self) -> RwLockReadGuard<'_, AssetInfos> {
        self.data
            .infos
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn write_infos(&self) -> RwLockWriteGuard<'_, AssetInfos> {
        self.data
            .infos
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn read_loaders(&self) -> RwLockReadGuard<'_, AssetLoaders> {
        self.data
            .loaders
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn write_loaders(&self) -> RwLockWriteGuard<'_, AssetLoaders> {
        self.data
            .loaders
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Retrieves the [`AssetSource`] for the given `source`.
    pub fn get_source<'a>(
        &self,
        source: impl Into<AssetSourceId<'a>>,
    ) -> Result<&AssetSource, MissingAssetSourceError> {
        self.data.sources.get(source.into())
    }

    /// Registers a new [`AssetLoader`]. [`AssetLoader`]s must be registered before they can be used.
    pub fn register_loader<L: AssetLoader>(&self, loader: L) {
        self.write_loaders().push(loader);
    }

    /// Registers a new [`Asset`] type. [`Asset`] types must be registered before assets of that type can be loaded.
    pub fn register_asset<A: Asset>(&self, assets: &Assets<A>) {
        self.register_handle_provider(assets.get_handle_provider());
        fn sender<A: Asset>(world: &mut World, index: AssetIndex) {
            world
                .resource_mut::<Messages<AssetEvent<A>>>()
                .write(AssetEvent::LoadedWithDependencies { id: index.into() });
        }
        fn failed_sender<A: Asset>(
            world: &mut World,
            index: AssetIndex,
            path: AssetPath<'static>,
            error: AssetLoadError,
        ) {
            world
                .resource_mut::<Messages<AssetLoadFailedEvent<A>>>()
                .write(AssetLoadFailedEvent {
                    id: index.into(),
                    path,
                    error,
                });
        }

        let mut infos = self.write_infos();

        infos
            .dependency_loaded_event_sender
            .insert(TypeId::of::<A>(), sender::<A>);

        infos
            .dependency_failed_event_sender
            .insert(TypeId::of::<A>(), failed_sender::<A>);
    }

    pub(crate) fn register_handle_provider(&self, handle_provider: AssetHandleProvider) {
        self.write_infos()
            .handle_providers
            .insert(handle_provider.type_id, handle_provider);
    }

    /// Returns the registered [`AssetLoader`] associated with the given type name, if it exists.
    pub async fn get_asset_loader_with_type_name(
        &self,
        type_name: &str,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForTypeNameError> {
        let error = || MissingAssetLoaderForTypeNameError {
            type_name: type_name.to_string(),
        };

        let loader = self
            .read_loaders()
            .get_by_name(type_name)
            .ok_or_else(error)?;
        loader.get().await.map_err(|_| error())
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path`. This will not block on the asset load. Instead,
    /// it returns a "strong" [`Handle`]. When the [`Asset`] is loaded (and enters [`LoadState::Loaded`]), it will be added to the
    /// associated [`Assets`] resource.
    ///
    /// Note that if the asset at this path is already loaded, this function will return the existing handle,
    /// and will not waste work spawning a new load task.
    ///
    /// In case the file path contains a hashtag (`#`), the `path` must be specified using [`Path`]
    /// or [`AssetPath`] because otherwise the hashtag would be interpreted as separator between
    /// the file path and the label. For example:
    ///
    /// ```no_run
    /// # use bevy_asset::{AssetServer, Handle, LoadedUntypedAsset};
    /// # use bevy_ecs::prelude::Res;
    /// # use std::path::Path;
    /// // `#path` is a label.
    /// # fn setup(asset_server: Res<AssetServer>) {
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load("some/file#path");
    ///
    /// // `#path` is part of the file name.
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load(Path::new("some/file#path"));
    /// # }
    /// ```
    ///
    /// Furthermore, if you need to load a file with a hashtag in its name _and_ a label, you can
    /// manually construct an [`AssetPath`].
    ///
    /// ```no_run
    /// # use bevy_asset::{AssetPath, AssetServer, Handle, LoadedUntypedAsset};
    /// # use bevy_ecs::prelude::Res;
    /// # use std::path::Path;
    /// # fn setup(asset_server: Res<AssetServer>) {
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load(AssetPath::from_path(Path::new("some/file#path")).with_label("subasset"));
    /// # }
    /// ```
    ///
    /// You can check the asset's load state by reading [`AssetEvent`] events, calling [`AssetServer::load_state`], or checking
    /// the [`Assets`] storage to see if the [`Asset`] exists yet.
    ///
    /// The asset load will fail and an error will be printed to the logs if the asset stored at `path` is not of type `A`.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.load_with_meta_transform(path, None, ())
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path`. The given `settings` function will override the asset's
    /// [`AssetLoader`] settings. The type `S` _must_ match the configured [`AssetLoader::Settings`] or `settings` changes
    /// will be ignored and an error will be printed to the log.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        self.load_with_meta_transform(path, Some(loader_settings_meta_transform(settings)), ())
    }

    pub(crate) fn load_with_meta_transform<'a, A: Asset, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
        guard: G,
    ) -> Handle<A> {
        let path = path.into().into_owned();

        if path.is_unapproved() {
            error!("Asset path {path} is unapproved. See UnapprovedPathMode for details.");
            return Handle::default();
        }

        let mut infos = self.write_infos();
        let (handle, should_load) = infos.get_or_create_path_handle::<A>(
            path.clone(),
            HandleLoadingMode::Request,
            meta_transform,
        );

        if should_load {
            self.spawn_load_task(handle.clone().untyped(), path, infos, guard);
        }

        handle
    }

    pub(crate) fn spawn_load_task<G: Send + Sync + 'static>(
        &self,
        handle: UntypedHandle,
        path: AssetPath<'static>,
        mut infos: RwLockWriteGuard<AssetInfos>,
        guard: G,
    ) {
        infos.stats.started_load_tasks += 1;

        let owned_handle = handle.clone();
        let server = self.clone();
        let task = IoTaskPool::get().spawn(async move {
            if let Err(err) = server
                .load_internal(Some(owned_handle), path, false, None)
                .await
            {
                error!("{}", err);
            }
            drop(guard);
        });

        infos
            .pending_tasks
            .insert((&handle).try_into().unwrap(), task);
    }

    /// Performs an async asset load.
    ///
    /// `input_handle` must only be [`Some`] if `should_load` was true when retrieving
    /// `input_handle`. This is an optimization to avoid looking up `should_load` twice, but it
    /// means you _must_ be sure a load is necessary when calling this function with [`Some`].
    ///
    /// Returns the handle of the asset if one was retrieved by this function. Otherwise, may return
    /// [`None`].
    async fn load_internal<'a>(
        &self,
        input_handle: Option<UntypedHandle>,
        path: AssetPath<'a>,
        force: bool,
        meta_transform: Option<MetaTransform>,
    ) -> Result<Option<UntypedHandle>, AssetLoadError> {
        let input_handle_type_id = input_handle.as_ref().map(UntypedHandle::type_id);

        let path = path.into_owned();
        let path_clone = path.clone();
        let (mut meta, loader, mut reader) = self
            .get_meta_loader_and_reader(&path_clone, input_handle_type_id)
            .await
            .inspect_err(|e| {
                // if there was an input handle, a "load" operation has already started, so we must produce a "failure" event, if
                // we cannot find the meta and loader
                if let Some(handle) = &input_handle {
                    self.send_asset_event(InternalAssetEvent::Failed {
                        index: handle.try_into().unwrap(),
                        path: path.clone_owned(),
                        error: e.clone(),
                    });
                }
            })?;

        if let Some(meta_transform) = input_handle.as_ref().and_then(|h| h.meta_transform()) {
            (*meta_transform)(&mut *meta);
        }

        let asset_id: Option<ErasedAssetIndex>; // The asset ID of the asset we are trying to load.
        let fetched_handle; // The handle if one was looked up/created.
        let should_load; // Whether we need to load the asset.
        if let Some(input_handle) = input_handle {
            // This must have been created with `get_or_create_path_handle_internal` at some point,
            // which only produces Strong variant handles, so this is safe.
            asset_id = Some((&input_handle).try_into().unwrap());
            // In this case, we intentionally drop the input handle so we can cancel loading the
            // asset if the handle gets dropped (externally) before it finishes loading.
            fetched_handle = None;
            // The handle was passed in, so the "should_load" check was already done.
            should_load = true;
        } else {
            // TODO: multiple asset loads for the same path can happen at the same time (rather than
            // "early out-ing" in the "normal" case). This would be resolved by a universal asset
            // id, as we would not need to resolve the asset type to generate the ID. See this
            // issue: https://github.com/bevyengine/bevy/issues/10549

            let mut infos = self.write_infos();
            let result = infos.get_or_create_path_handle_internal(
                path.clone(),
                path.label().is_none().then(|| loader.asset_type_id()),
                HandleLoadingMode::Request,
                meta_transform,
            );
            match unwrap_with_context(result, Either::Left(loader.asset_type_name())) {
                // We couldn't figure out the correct handle without its type ID (which can only
                // happen if we are loading a subasset).
                None => {
                    // We don't know the expected type since the subasset may have a different type
                    // than the "root" asset (which is the type the loader will load).
                    asset_id = None;
                    fetched_handle = None;
                    // If we couldn't find an appropriate handle, then the asset certainly needs to
                    // be loaded.
                    should_load = true;
                }
                Some((handle, result_should_load)) => {
                    // `get_or_create_path_handle_internal` always returns Strong variant, so this
                    // is safe.
                    asset_id = Some((&handle).try_into().unwrap());
                    fetched_handle = Some(handle);
                    should_load = result_should_load;
                }
            }
        }
        // Verify that the expected type matches the loader's type.
        if let Some(asset_type_id) = asset_id.map(|id| id.type_id) {
            // If we are loading a subasset, then the subasset's type almost certainly doesn't match
            // the loader's type - and that's ok.
            if path.label().is_none() && asset_type_id != loader.asset_type_id() {
                error!(
                    "Expected {:?}, got {:?}",
                    asset_type_id,
                    loader.asset_type_id()
                );
                return Err(AssetLoadError::RequestedHandleTypeMismatch {
                    path: path.into_owned(),
                    requested: asset_type_id,
                    actual_asset_name: loader.asset_type_name(),
                    loader_name: loader.type_path(),
                });
            }
        }
        // Bail out earlier if we don't need to load the asset.
        if !should_load && !force {
            return Ok(fetched_handle);
        }

        // We don't actually need to use _base_handle, but we do need to keep the handle alive.
        // Dropping it would cancel the load of the base asset, which would make the load of this
        // subasset never complete.
        let (base_asset_id, _base_handle, base_path) = if path.label().is_some() {
            let mut infos = self.write_infos();
            let base_path = path.without_label().into_owned();
            let base_handle = infos
                .get_or_create_path_handle_erased(
                    base_path.clone(),
                    loader.asset_type_id(),
                    Some(loader.asset_type_name()),
                    HandleLoadingMode::Force,
                    None,
                )
                .0;
            (
                // `get_or_create_path_handle_erased` always returns Strong variant, so this is
                // safe.
                (&base_handle).try_into().unwrap(),
                Some(base_handle),
                base_path,
            )
        } else {
            (asset_id.unwrap(), None, path.clone())
        };

        match self
            .load_with_settings_loader_and_reader(
                &base_path,
                meta.loader_settings().expect("meta is set to Load"),
                &*loader,
                &mut *reader,
                true,
            )
            .await
        {
            Ok(loaded_asset) => {
                let final_handle = if let Some(label) = path.label_cow() {
                    match loaded_asset.labeled_assets.get(&label) {
                        Some(labeled_asset) => Some(labeled_asset.handle.clone()),
                        None => {
                            let mut all_labels: Vec<String> = loaded_asset
                                .labeled_assets
                                .keys()
                                .map(|s| (**s).to_owned())
                                .collect();
                            all_labels.sort_unstable();
                            return Err(AssetLoadError::MissingLabel {
                                base_path,
                                label: label.to_string(),
                                all_labels,
                            });
                        }
                    }
                } else {
                    fetched_handle
                };

                self.send_asset_event(InternalAssetEvent::Loaded {
                    index: base_asset_id,
                    loaded_asset,
                });
                Ok(final_handle)
            }
            Err(err) => {
                self.send_asset_event(InternalAssetEvent::Failed {
                    index: base_asset_id,
                    error: err.clone(),
                    path: path.into_owned(),
                });
                Err(err)
            }
        }
    }

    /// Queues a new asset to be tracked by the [`AssetServer`] and returns a [`Handle`] to it. This can be used to track
    /// dependencies of assets created at runtime.
    ///
    /// After the asset has been fully loaded by the [`AssetServer`], it will show up in the relevant [`Assets`] storage.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn add<A: Asset>(&self, asset: A) -> Handle<A> {
        self.load_asset(LoadedAsset::new_with_dependencies(asset))
    }

    pub(crate) fn load_asset<A: Asset>(&self, asset: impl Into<LoadedAsset<A>>) -> Handle<A> {
        let loaded_asset: LoadedAsset<A> = asset.into();
        let erased_loaded_asset: ErasedLoadedAsset = loaded_asset.into();
        self.load_asset_untyped(erased_loaded_asset)
            .typed_debug_checked()
    }

    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub(crate) fn load_asset_untyped(&self, asset: impl Into<ErasedLoadedAsset>) -> UntypedHandle {
        let loaded_asset = asset.into();
        let handle = self.write_infos().create_loading_handle_untyped(
            loaded_asset.asset_type_id(),
            loaded_asset.asset_type_name(),
        );
        self.send_asset_event(InternalAssetEvent::Loaded {
            // `create_loading_handle_untyped` always returns Strong variant, so this is safe.
            index: (&handle).try_into().unwrap(),
            loaded_asset,
        });
        handle
    }

    fn send_asset_event(&self, event: InternalAssetEvent) {
        self.data.asset_event_sender.send(event).unwrap();
    }

    /// Retrieves all loads states for the given asset id.
    pub fn get_load_states(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<(LoadState, DependencyLoadState, RecursiveDependencyLoadState)> {
        let Ok(index) = id.into().try_into() else {
            // Always say we don't have Uuid assets.
            return None;
        };
        self.read_infos().get(index).map(|i| {
            (
                i.load_state.clone(),
                i.dep_load_state.clone(),
                i.rec_dep_load_state.clone(),
            )
        })
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    ///
    /// Note that this is "just" the root asset load state. To get the load state of
    /// its dependencies or recursive dependencies, see [`AssetServer::get_dependency_load_state`]
    /// and [`AssetServer::get_recursive_dependency_load_state`] respectively.
    pub fn get_load_state(&self, id: impl Into<UntypedAssetId>) -> Option<LoadState> {
        let Ok(index) = id.into().try_into() else {
            // Always say we don't have Uuid assets.
            return None;
        };
        self.read_infos().get(index).map(|i| i.load_state.clone())
    }

    /// Retrieves the [`DependencyLoadState`] of a given asset `id`'s dependencies.
    ///
    /// Note that this is only the load state of direct dependencies of the root asset. To get
    /// the load state of the root asset itself or its recursive dependencies, see
    /// [`AssetServer::get_load_state`] and [`AssetServer::get_recursive_dependency_load_state`] respectively.
    pub fn get_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<DependencyLoadState> {
        let Ok(index) = id.into().try_into() else {
            // Always say we don't have Uuid assets.
            return None;
        };
        self.read_infos()
            .get(index)
            .map(|i| i.dep_load_state.clone())
    }

    /// Retrieves the main [`RecursiveDependencyLoadState`] of a given asset `id`'s recursive dependencies.
    ///
    /// Note that this is only the load state of recursive dependencies of the root asset. To get
    /// the load state of the root asset itself or its direct dependencies only, see
    /// [`AssetServer::get_load_state`] and [`AssetServer::get_dependency_load_state`] respectively.
    pub fn get_recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<RecursiveDependencyLoadState> {
        let Ok(index) = id.into().try_into() else {
            // Always say we don't have Uuid assets.
            return None;
        };
        self.read_infos()
            .get(index)
            .map(|i| i.rec_dep_load_state.clone())
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_load_state`] except the result is unwrapped. If
    /// the result is None, [`LoadState::NotLoaded`] is returned.
    pub fn load_state(&self, id: impl Into<UntypedAssetId>) -> LoadState {
        self.get_load_state(id).unwrap_or(LoadState::NotLoaded)
    }

    /// Retrieves the [`DependencyLoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_dependency_load_state`] except the result is unwrapped. If
    /// the result is None, [`DependencyLoadState::NotLoaded`] is returned.
    pub fn dependency_load_state(&self, id: impl Into<UntypedAssetId>) -> DependencyLoadState {
        self.get_dependency_load_state(id)
            .unwrap_or(DependencyLoadState::NotLoaded)
    }

    /// Retrieves the  [`RecursiveDependencyLoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_recursive_dependency_load_state`] except the result is unwrapped. If
    /// the result is None, [`RecursiveDependencyLoadState::NotLoaded`] is returned.
    pub fn recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> RecursiveDependencyLoadState {
        self.get_recursive_dependency_load_state(id)
            .unwrap_or(RecursiveDependencyLoadState::NotLoaded)
    }

    /// Convenience method that returns true if the asset has been loaded.
    pub fn is_loaded(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(self.load_state(id), LoadState::Loaded)
    }

    /// Convenience method that returns true if the asset and all of its direct dependencies have been loaded.
    pub fn is_loaded_with_direct_dependencies(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(
            self.get_load_states(id),
            Some((LoadState::Loaded, DependencyLoadState::Loaded, _))
        )
    }

    /// Convenience method that returns true if the asset, all of its dependencies, and all of its recursive
    /// dependencies have been loaded.
    pub fn is_loaded_with_dependencies(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(
            self.get_load_states(id),
            Some((
                LoadState::Loaded,
                DependencyLoadState::Loaded,
                RecursiveDependencyLoadState::Loaded
            ))
        )
    }

    /// Returns the path for the given `id`, if it has one.
    pub fn get_path(&self, id: impl Into<UntypedAssetId>) -> Option<AssetPath<'_>> {
        let Ok(index) = id.into().try_into() else {
            // Always say we don't have Uuid assets.
            return None;
        };
        let infos = self.read_infos();
        let info = infos.get(index)?;
        Some(info.path.as_ref()?.clone())
    }

    /// Pre-register a loader that will later be added.
    ///
    /// Assets loaded with matching extensions will be blocked until the
    /// real loader is added.
    pub fn preregister_loader<L: AssetLoader>(&self, extensions: &[&str]) {
        self.write_loaders().reserve::<L>(extensions);
    }

    /// Retrieve a handle for the given path. This will create a handle (and [`AssetInfo`]) if it does not exist
    pub(crate) fn get_or_create_path_handle<'a, A: Asset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
    ) -> Handle<A> {
        self.write_infos()
            .get_or_create_path_handle::<A>(
                path.into().into_owned(),
                HandleLoadingMode::NotLoading,
                meta_transform,
            )
            .0
    }

    pub(crate) async fn get_meta_loader_and_reader<'a>(
        &'a self,
        asset_path: &'a AssetPath<'_>,
        asset_type_id: Option<TypeId>,
    ) -> Result<
        (
            Box<dyn AssetMetaDyn>,
            Arc<dyn ErasedAssetLoader>,
            Box<dyn Reader + 'a>,
        ),
        AssetLoadError,
    > {
        let source = self.get_source(asset_path.source())?;
        let asset_reader = source.reader();

        // Scope the meta reader up here. This allows the reader to be "transactional": for sources
        // that want to lock the asset before reading it (e.g., with a RwLock), this allows the meta
        // reader to take the RwLock, and since it overlaps with the asset reader, the asset reader
        // can "take over" the RwLock before the meta reader gets dropped.
        let mut meta_reader;

        let (meta, loader) = match asset_reader.read_meta(asset_path.path()).await {
            Ok(new_meta_reader) => {
                meta_reader = new_meta_reader;
                let mut meta_bytes = vec![];
                meta_reader
                    .read_to_end(&mut meta_bytes)
                    .await
                    .map_err(|err| AssetLoadError::AssetReaderError(err.into()))?;
                // TODO: this isn't fully minimal yet. we only need the loader
                let minimal: AssetMetaMinimal = ron::de::from_bytes(&meta_bytes).map_err(|e| {
                    AssetLoadError::DeserializeMeta {
                        path: asset_path.clone_owned(),
                        error: DeserializeMetaError::DeserializeMinimal(e).into(),
                    }
                })?;
                let loader_name = match minimal.asset {
                    AssetActionMinimal::Load { loader } => loader,
                    AssetActionMinimal::Process { .. } => {
                        return Err(AssetLoadError::CannotLoadProcessedAsset {
                            path: asset_path.clone_owned(),
                        })
                    }
                    AssetActionMinimal::Ignore => {
                        return Err(AssetLoadError::CannotLoadIgnoredAsset {
                            path: asset_path.clone_owned(),
                        })
                    }
                };
                let loader = self.get_asset_loader_with_type_name(&loader_name).await?;
                let meta = loader.deserialize_meta(&meta_bytes).map_err(|e| {
                    AssetLoadError::DeserializeMeta {
                        path: asset_path.clone_owned(),
                        error: e.into(),
                    }
                })?;

                (meta, loader)
            }
            Err(AssetReaderError::NotFound(_)) => {
                // TODO: Handle error transformation
                let loader = {
                    self.read_loaders()
                        .find(None, asset_type_id, None, Some(asset_path))
                };

                let error = || AssetLoadError::MissingAssetLoader {
                    loader_name: None,
                    asset_type_id,
                    extension: None,
                    asset_path: Some(asset_path.to_string()),
                };

                let loader = loader.ok_or_else(error)?.get().await.map_err(|_| error())?;

                let meta = loader.default_meta();
                (meta, loader)
            }
            Err(err) => return Err(err.into()),
        };
        let reader = asset_reader.read(asset_path.path()).await?;
        Ok((meta, loader, reader))
    }

    pub(crate) async fn load_with_settings_loader_and_reader(
        &self,
        asset_path: &AssetPath<'_>,
        settings: &dyn Settings,
        loader: &dyn ErasedAssetLoader,
        reader: &mut dyn Reader,
        load_dependencies: bool,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        // TODO: experiment with this
        let asset_path = asset_path.clone_owned();
        let load_context = LoadContext::new(self, asset_path.clone(), load_dependencies);
        AssertUnwindSafe(loader.load(reader, settings, load_context))
            .catch_unwind()
            .await
            .map_err(|_| AssetLoadError::AssetLoaderPanic {
                path: asset_path.clone_owned(),
                loader_name: loader.type_path(),
            })?
            .map_err(|e| {
                AssetLoadError::AssetLoaderError(AssetLoaderError {
                    path: asset_path.clone_owned(),
                    loader_name: loader.type_path(),
                    error: e.into(),
                })
            })
    }
}

/// A system that manages internal [`AssetServer`] events, such as finalizing asset loads.
pub fn handle_internal_asset_events(world: &mut World) {
    world.resource_scope(|world, server: Mut<AssetServer>| {
        let mut infos = server.write_infos();
        for event in server.data.asset_event_receiver.try_iter() {
            match event {
                InternalAssetEvent::Loaded {
                    index,
                    loaded_asset,
                } => {
                    infos.process_asset_load(
                        index,
                        loaded_asset,
                        world,
                        &server.data.asset_event_sender,
                    );
                }
                InternalAssetEvent::LoadedWithDependencies { index } => {
                    let sender = infos
                        .dependency_loaded_event_sender
                        .get(&index.type_id)
                        .expect("Asset event sender should exist");
                    sender(world, index.index);
                }
                InternalAssetEvent::Failed { index, path, error } => {
                    infos.process_asset_fail(index, error.clone());

                    // Send typed failure event
                    let sender = infos
                        .dependency_failed_event_sender
                        .get(&index.type_id)
                        .expect("Asset failed event sender should exist");
                    sender(world, index.index, path, error);
                }
            }
        }

        // No `AssetWatcher` exists in this build, so a watching server has no source event to
        // reload; it only prunes its finished load tasks.
        if !infos.watching_for_changes {
            return;
        }

        infos
            .pending_tasks
            .retain(|_, load_task| !load_task.is_finished());
    });
}

/// A system publishing asset server statistics to [`bevy_diagnostic`].
pub fn publish_asset_server_diagnostics(
    asset_server: Res<AssetServer>,
    mut diagnostics: Diagnostics,
) {
    let infos = asset_server.read_infos();
    diagnostics.add_measurement(&AssetServer::STARTED_LOAD_COUNT, || {
        infos.stats.started_load_tasks as _
    });
}

/// Internal events for asset load results
pub(crate) enum InternalAssetEvent {
    Loaded {
        index: ErasedAssetIndex,
        loaded_asset: ErasedLoadedAsset,
    },
    LoadedWithDependencies {
        index: ErasedAssetIndex,
    },
    Failed {
        index: ErasedAssetIndex,
        path: AssetPath<'static>,
        error: AssetLoadError,
    },
}

/// The load state of an asset.
#[derive(Component, Clone, Debug)]
pub enum LoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// The asset is in the process of loading.
    Loading,

    /// The asset has been loaded and has been added to the [`World`]
    Loaded,

    /// The asset failed to load. The underlying [`AssetLoadError`] is
    /// referenced by [`Arc`] clones in all related [`DependencyLoadState`]s
    /// and [`RecursiveDependencyLoadState`]s in the asset's dependency tree.
    Failed(Arc<AssetLoadError>),
}

impl LoadState {
    /// Returns `true` if this instance is [`LoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`LoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`LoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// The load state of an asset's dependencies.
#[derive(Component, Clone, Debug)]
pub enum DependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// Dependencies are still loading
    Loading,

    /// Dependencies have all loaded
    Loaded,

    /// One or more dependencies have failed to load.
    Failed,
}

impl DependencyLoadState {
    /// Returns `true` if this instance is [`DependencyLoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`DependencyLoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`DependencyLoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// The recursive load state of an asset's dependencies.
#[derive(Component, Clone, Debug)]
pub enum RecursiveDependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// Dependencies in this asset's dependency tree are still loading
    Loading,

    /// Dependencies in this asset's dependency tree have all loaded
    Loaded,

    /// One or more dependencies have failed to load in this asset's dependency
    /// tree. The underlying [`AssetLoadError`] is referenced by [`Arc`] clones
    /// in all related [`LoadState`]s in the asset's dependency tree.
    Failed(Arc<AssetLoadError>),
}

impl RecursiveDependencyLoadState {
    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// An error that occurs during an [`Asset`] load.
#[derive(Error, Debug, Clone)]
#[expect(
    missing_docs,
    reason = "Adding docs to the variants would not add information beyond the error message and the names"
)]
pub enum AssetLoadError {
    #[error("Requested handle of type {requested:?} for asset '{path}' does not match actual asset type '{actual_asset_name}', which used loader '{loader_name}'")]
    RequestedHandleTypeMismatch {
        path: AssetPath<'static>,
        requested: TypeId,
        actual_asset_name: &'static str,
        loader_name: &'static str,
    },
    #[error("Could not find an asset loader matching: Loader Name: {loader_name:?}; Asset Type: {asset_type_id:?}; Extension: {extension:?}; Path: {asset_path:?};")]
    MissingAssetLoader {
        loader_name: Option<String>,
        asset_type_id: Option<TypeId>,
        extension: Option<String>,
        asset_path: Option<String>,
    },
    #[error(transparent)]
    MissingAssetLoaderForTypeName(#[from] MissingAssetLoaderForTypeNameError),
    #[error(transparent)]
    AssetReaderError(#[from] AssetReaderError),
    #[error(transparent)]
    MissingAssetSourceError(#[from] MissingAssetSourceError),
    #[error("Encountered an error while reading asset metadata bytes")]
    AssetMetaReadError,
    #[error("Failed to deserialize meta for asset {path}: {error}")]
    DeserializeMeta {
        path: AssetPath<'static>,
        error: Box<DeserializeMetaError>,
    },
    #[error("Asset '{path}' is configured to be processed. It cannot be loaded directly.")]
    #[from(ignore)]
    CannotLoadProcessedAsset { path: AssetPath<'static> },
    #[error("Asset '{path}' is configured to be ignored. It cannot be loaded.")]
    #[from(ignore)]
    CannotLoadIgnoredAsset { path: AssetPath<'static> },
    #[error("Failed to load asset '{path}', asset loader '{loader_name}' panicked")]
    AssetLoaderPanic {
        path: AssetPath<'static>,
        loader_name: &'static str,
    },
    #[error(transparent)]
    AssetLoaderError(#[from] AssetLoaderError),
    #[error("The file at '{}' does not contain the labeled asset '{}'; it contains the following {} assets: {}",
            base_path,
            label,
            all_labels.len(),
            all_labels.iter().map(|l| format!("'{l}'")).collect::<Vec<_>>().join(", "))]
    MissingLabel {
        base_path: AssetPath<'static>,
        label: String,
        all_labels: Vec<String>,
    },
}

/// An error that can occur during asset loading.
#[derive(Error, Debug, Clone)]
#[error("Failed to load asset '{path}' with asset loader '{loader_name}': {error}")]
pub struct AssetLoaderError {
    path: AssetPath<'static>,
    loader_name: &'static str,
    error: Arc<BevyError>,
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given [`core::any::type_name`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("no `AssetLoader` found with the name '{type_name}'")]
pub struct MissingAssetLoaderForTypeNameError {
    /// The type name that was not found.
    pub type_name: String,
}

impl core::fmt::Debug for AssetServer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AssetServer")
            .field("info", &self.data.infos.read())
            .finish()
    }
}
