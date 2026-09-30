use alloc::vec::Vec;

use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

/// A "loaded folder" containing handles for all assets stored in a given [`AssetPath`].
///
/// `AssetPlugin` registers the asset type; nothing in this build loads one.
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    /// The handles of all assets stored in the folder.
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
