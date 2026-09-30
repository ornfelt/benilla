use crate::EguiManagedTextures;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_platform::collections::HashMap;
use bevy_render::extract_resource::ExtractResource;

/// The extracted version of [`EguiManagedTextures`].
#[derive(Debug, Resource)]
pub struct ExtractedEguiManagedTextures(pub HashMap<(Entity, u64), Handle<Image>>);
impl ExtractResource for ExtractedEguiManagedTextures {
    type Source = EguiManagedTextures;

    fn extract_resource(source: &Self::Source) -> Self {
        Self(source.iter().map(|(k, v)| (*k, v.handle.clone())).collect())
    }
}
