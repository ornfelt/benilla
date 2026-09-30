//! Implementations of the builder-pattern used for loading dependent assets via
//! [`LoadContext::loader`].

use crate::{
    Asset, AssetLoadError, AssetPath, ErasedAssetLoader, ErasedLoadedAsset, Handle, LoadContext,
    LoadDirectError, LoadedAsset,
};
use alloc::{borrow::ToOwned, sync::Arc};
use core::any::TypeId;

/// A builder for loading nested assets inside a [`LoadContext`].
///
/// # Loader state
///
/// The type parameters `T` and `M` determine how this will load assets:
/// - `T`: the typing of this loader. How do we know what type of asset to load?
///
///   See [`StaticTyped`], the only typing.
///
/// - `M`: the load mode. Do we want to load this asset right now (in which case
///   you will have to `await` the operation), or do we just want a [`Handle`],
///   and leave the actual asset loading to later?
///
///   See [`Deferred`] (the default) and [`Immediate`].
///
/// When configuring this builder, you can switch to [`Immediate`] via
/// [`immediate`].
///
/// ## Typing
///
/// To inform the loader of what type of asset to load:
/// - in [`StaticTyped`]: statically providing a type parameter `A: Asset` to
///   [`load`].
///
///   This is the simplest way to get a [`Handle<A>`] to the loaded asset, as
///   long as you know the type of `A` at compile time.
///
/// ## Load mode
///
/// To inform the loader how you want to load the asset:
/// - in [`Deferred`]: when you request to load the asset, you get a [`Handle`]
///   for it, but the actual loading won't be completed until later.
///
///   Use this if you only need a [`Handle`].
///
/// - in [`Immediate`]: the load request will load the asset right then and
///   there, waiting until the asset is fully loaded and giving you access to
///   it.
///
///   Note that this requires you to `await` a future, so you must be in an
///   async context to use direct loading. In an asset loader, you will be in
///   an async context.
///
///   Use this if you need the *value* of another asset in order to load the
///   current asset. For example, if you are deriving a new asset from the
///   referenced asset, or you are building a collection of assets. This will
///   add the path of the asset as a "load dependency".
///
/// # Load kickoff
///
/// If the current context is a normal [`AssetServer::load`], an actual asset
/// load will be kicked off immediately, which ensures the load happens as soon
/// as possible. "Normal loads" kicked from within a normal Bevy App will
/// generally configure the context to kick off loads immediately.
///
/// If the current context is configured to not load dependencies automatically,
/// a load will not be kicked off automatically. It is
/// then the calling context's responsibility to begin a load if necessary.
///
/// # Lifetimes
///
/// - `ctx`: the lifetime of the associated [`AssetServer`](crate::AssetServer) reference
/// - `builder`: the lifetime of the temporary builder structs
///
/// [`immediate`]: Self::immediate
/// [`load`]: Self::load
/// [`AssetServer::load`]: crate::AssetServer::load
pub struct NestedLoader<'ctx, 'builder, T, M> {
    load_context: &'builder mut LoadContext<'ctx>,
    typing: T,
    #[expect(dead_code, reason = "a marker: only its type is read")]
    mode: M,
}

mod sealed {
    pub trait Typing {}

    pub trait Mode {}
}

/// [`NestedLoader`] will be provided the type of asset as a type parameter on
/// [`load`].
///
/// [`load`]: NestedLoader::load
pub struct StaticTyped(());

impl sealed::Typing for StaticTyped {}

/// [`NestedLoader`] will create and return asset handles immediately, but only
/// actually load the asset later.
pub struct Deferred(());

impl sealed::Mode for Deferred {}

/// [`NestedLoader`] will immediately load an asset when requested.
pub struct Immediate(());

impl sealed::Mode for Immediate {}

// common to all states

impl<'ctx, 'builder> NestedLoader<'ctx, 'builder, StaticTyped, Deferred> {
    pub(crate) fn new(load_context: &'builder mut LoadContext<'ctx>) -> Self {
        NestedLoader {
            load_context,
            typing: StaticTyped(()),
            mode: Deferred(()),
        }
    }
}

impl<'ctx, 'builder, T: sealed::Typing, M: sealed::Mode> NestedLoader<'ctx, 'builder, T, M> {
    /// The [`load`] call itself will load an asset, rather than scheduling the
    /// loading to happen later.
    ///
    /// This gives you access to the loaded asset, but requires you to be in an
    /// async context, and be able to `await` the resulting future.
    ///
    /// [`load`]: Self::load
    #[must_use]
    pub fn immediate(self) -> NestedLoader<'ctx, 'builder, T, Immediate> {
        NestedLoader {
            load_context: self.load_context,
            typing: self.typing,
            mode: Immediate(()),
        }
    }
}

// deferred loading logic

impl NestedLoader<'_, '_, StaticTyped, Deferred> {
    /// Retrieves a handle for the asset at the given path and adds that path as
    /// a dependency of this asset.
    ///
    /// This requires you to know the type of asset statically.
    pub fn load<'c, A: Asset>(self, path: impl Into<AssetPath<'c>>) -> Handle<A> {
        let path = path.into().to_owned();
        let handle = if self.load_context.should_load_dependencies {
            self.load_context
                .asset_server
                .load_with_meta_transform(path, None, (), true)
        } else {
            self.load_context
                .asset_server
                .get_or_create_path_handle(path, None)
        };
        // `load_with_meta_transform` and `get_or_create_path_handle` always returns a Strong
        // variant, so we are safe to unwrap.
        let index = (&handle).try_into().unwrap();
        self.load_context.dependencies.insert(index);
        handle
    }
}

// immediate loading logic

impl<T> NestedLoader<'_, '_, T, Immediate> {
    async fn load_internal(
        self,
        path: &AssetPath<'static>,
        asset_type_id: Option<TypeId>,
    ) -> Result<(Arc<dyn ErasedAssetLoader>, ErasedLoadedAsset), LoadDirectError> {
        if path.label().is_some() {
            return Err(LoadDirectError::RequestedSubasset(path.clone()));
        }
        self.load_context
            .asset_server
            .write_infos()
            .stats
            .started_load_tasks += 1;
        let (meta, loader, mut reader) = self
            .load_context
            .asset_server
            .get_meta_loader_and_reader(path, asset_type_id)
            .await
            .map_err(|error| LoadDirectError::LoadError {
                dependency: path.clone(),
                error,
            })?;

        let asset = self
            .load_context
            .load_direct_internal(
                path.clone(),
                meta.loader_settings().expect("meta corresponds to a load"),
                &*loader,
                &mut *reader,
            )
            .await?;
        Ok((loader, asset))
    }
}

impl NestedLoader<'_, '_, StaticTyped, Immediate> {
    /// Attempts to load the asset at the given `path` immediately.
    ///
    /// This requires you to know the type of asset statically.
    pub async fn load<'p, A: Asset>(
        self,
        path: impl Into<AssetPath<'p>>,
    ) -> Result<LoadedAsset<A>, LoadDirectError> {
        let path = path.into().into_owned();
        self.load_internal(&path, Some(TypeId::of::<A>()))
            .await
            .and_then(move |(loader, untyped_asset)| {
                untyped_asset
                    .downcast::<A>()
                    .map_err(|_| LoadDirectError::LoadError {
                        dependency: path.clone(),
                        error: AssetLoadError::RequestedHandleTypeMismatch {
                            path,
                            requested: TypeId::of::<A>(),
                            actual_asset_name: loader.asset_type_name(),
                            loader_name: loader.type_path(),
                        },
                    })
            })
    }
}
