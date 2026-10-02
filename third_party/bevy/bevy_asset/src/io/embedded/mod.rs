use crate::io::{
    memory::{Dir, MemoryAssetReader},
    AssetSourceBuilder, AssetSourceBuilders,
};
use alloc::boxed::Box;
use bevy_ecs::resource::Resource;
use std::path::{Path, PathBuf};

/// The name of the `embedded` [`AssetSource`](crate::io::AssetSource),
/// as stored in the [`AssetSourceBuilders`] resource.
pub const EMBEDDED: &str = "embedded";

/// A [`Resource`] that manages "rust source files" in a virtual in memory [`Dir`], which is intended
/// to be shared with a [`MemoryAssetReader`].
/// Nothing in this build inserts into it: the `embedded` source it backs is empty.
#[derive(Resource, Default)]
pub struct EmbeddedAssetRegistry {
    dir: Dir,
}

impl EmbeddedAssetRegistry {
    /// Registers the [`EMBEDDED`] [`AssetSource`](crate::io::AssetSource) with the given [`AssetSourceBuilders`].
    pub fn register_source(&self, sources: &mut AssetSourceBuilders) {
        let dir = self.dir.clone();

        let source =
            AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: dir.clone() }));

        sources.insert(EMBEDDED, source);
    }
}

/// Returns the [`Path`] for a given `embedded` asset: the [`AssetPath`](crate::AssetPath)
/// the `embedded` source would serve it at.
#[macro_export]
macro_rules! embedded_path {
    ($path_str: expr) => {{
        $crate::embedded_path!("src", $path_str)
    }};

    ($source_path: expr, $path_str: expr) => {{
        let crate_name = module_path!().split(':').next().unwrap();
        $crate::io::embedded::_embedded_asset_path(
            crate_name,
            $source_path.as_ref(),
            file!().as_ref(),
            $path_str.as_ref(),
        )
    }};
}

/// Implementation detail of `embedded_path`, do not use this!
///
/// Returns an embedded asset path, given:
///   - `crate_name`: name of the crate where the asset is embedded
///   - `src_prefix`: path prefix of the crate's source directory, relative to the workspace root
///   - `file_path`: `std::file!()` path of the source file where `embedded_path!` is called
///   - `asset_path`: path of the embedded asset relative to `file_path`
#[doc(hidden)]
pub fn _embedded_asset_path(
    crate_name: &str,
    src_prefix: &Path,
    file_path: &Path,
    asset_path: &Path,
) -> PathBuf {
    let file_path = if cfg!(not(target_family = "windows")) {
        // Work around bug: https://github.com/bevyengine/bevy/issues/14246
        // Note, this will break any paths on Linux/Mac containing "\"
        PathBuf::from(file_path.to_str().unwrap().replace("\\", "/"))
    } else {
        PathBuf::from(file_path)
    };
    let mut maybe_parent = file_path.parent();
    let after_src = loop {
        let Some(parent) = maybe_parent else {
            panic!("Failed to find src_prefix {src_prefix:?} in {file_path:?}")
        };
        if parent.ends_with(src_prefix) {
            break file_path.strip_prefix(parent).unwrap();
        }
        maybe_parent = parent.parent();
    };
    let asset_path = after_src.parent().unwrap().join(asset_path);
    Path::new(crate_name).join(asset_path)
}

/// Loads an "internal" binary asset by embedding the bytes stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world_mut().resource_mut::<$crate::Assets<_>>();
        assets
            .insert(
                $handle.id(),
                ($loader)(
                    include_bytes!($path_str).as_ref(),
                    std::path::Path::new(file!())
                        .parent()
                        .unwrap()
                        .join($path_str)
                        .to_string_lossy()
                        .into(),
                ),
            )
            .unwrap();
    }};
}

#[cfg(test)]
mod tests {
    use super::_embedded_asset_path;
    use std::path::Path;

    // Relative paths show up if this macro is being invoked by a local crate.
    // In this case we know the relative path is a sub- path of the workspace
    // root.

    #[test]
    fn embedded_asset_path_from_local_crate() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // A blank src_path removes the embedded's file path altogether only the
    // asset path remains.
    #[test]
    fn embedded_asset_path_from_local_crate_blank_src_path_questionable() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "".as_ref(),
            "src/foo/some/deep/path/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/the/asset.png"));
    }

    #[test]
    #[should_panic(expected = "Failed to find src_prefix \"NOT-THERE\" in \"src")]
    fn embedded_asset_path_from_local_crate_bad_src() {
        let _asset_path = _embedded_asset_path(
            "my_crate",
            "NOT-THERE".as_ref(),
            "src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
    }

    #[test]
    fn embedded_asset_path_from_local_example_crate() {
        let asset_path = _embedded_asset_path(
            "example_name",
            "examples/foo".as_ref(),
            "examples/foo/example.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("example_name/the/asset.png"));
    }

    // Absolute paths show up if this macro is being invoked by an external
    // dependency, e.g. one that's being checked out from a crates repo or git.
    #[test]
    fn embedded_asset_path_from_external_crate() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    #[test]
    fn embedded_asset_path_from_external_crate_root_src_path() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "/path/to/crate/src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // Although extraneous slashes are permitted at the end, e.g., "src////",
    // one or more slashes at the beginning are not.
    #[test]
    #[should_panic(expected = "Failed to find src_prefix \"////src\" in")]
    fn embedded_asset_path_from_external_crate_extraneous_beginning_slashes() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "////src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // We don't handle this edge case because it is ambiguous with the
    // information currently available to the embedded_path macro.
    #[test]
    fn embedded_asset_path_from_external_crate_is_ambiguous() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "/path/to/.cargo/registry/src/crate/src/src/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        // Really, should be "my_crate/src/the/asset.png"
        assert_eq!(asset_path, Path::new("my_crate/the/asset.png"));
    }
}
