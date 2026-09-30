use crate::wgpu::{BindGroupLayoutEntry, BindingType, ShaderStages};
use core::num::NonZero;

/// Helper for constructing bind group layouts.
#[derive(Clone, Copy)]
pub struct BindGroupLayoutEntryBuilder {
    ty: BindingType,
    visibility: Option<ShaderStages>,
    count: Option<NonZero<u32>>,
}

impl BindGroupLayoutEntryBuilder {
    pub fn visibility(mut self, visibility: ShaderStages) -> Self {
        self.visibility = Some(visibility);
        self
    }

    pub fn count(mut self, count: NonZero<u32>) -> Self {
        self.count = Some(count);
        self
    }

    pub fn build(&self, binding: u32, default_visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            ty: self.ty,
            visibility: self.visibility.unwrap_or(default_visibility),
            count: self.count,
        }
    }
}

pub trait IntoBindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder;
}

impl IntoBindGroupLayoutEntryBuilder for BindingType {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindGroupLayoutEntryBuilder {
            ty: self,
            visibility: None,
            count: None,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for BindGroupLayoutEntry {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        if self.binding != u32::MAX {
            tracing::warn!("The BindGroupLayoutEntries api ignores the binding index when converting a raw wgpu::BindGroupLayoutEntry. You can ignore this warning by setting it to u32::MAX.");
        }
        BindGroupLayoutEntryBuilder {
            ty: self.ty,
            visibility: Some(self.visibility),
            count: self.count,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for BindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        self
    }
}

pub mod binding_types {
    use crate::render_resource::{
        BufferBindingType, SamplerBindingType, TextureSampleType, TextureViewDimension,
    };
    use crate::wgpu::{StorageTextureAccess, TextureFormat};
    use core::num::NonZero;
    use encase::ShaderType;

    use super::*;

    pub fn storage_buffer<T: ShaderType>(has_dynamic_offset: bool) -> BindGroupLayoutEntryBuilder {
        storage_buffer_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn storage_buffer_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZero<u64>>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn storage_buffer_read_only<T: ShaderType>(
        has_dynamic_offset: bool,
    ) -> BindGroupLayoutEntryBuilder {
        storage_buffer_read_only_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn storage_buffer_read_only_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZero<u64>>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn uniform_buffer<T: ShaderType>(has_dynamic_offset: bool) -> BindGroupLayoutEntryBuilder {
        uniform_buffer_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn uniform_buffer_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZero<u64>>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_1d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D1,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_multisampled(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_array(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2Array,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_array_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2Array,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_depth_2d() -> BindGroupLayoutEntryBuilder {
        texture_2d(TextureSampleType::Depth).into_bind_group_layout_entry_builder()
    }

    pub fn texture_depth_2d_multisampled() -> BindGroupLayoutEntryBuilder {
        texture_2d_multisampled(TextureSampleType::Depth).into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::Cube,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::Cube,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_array(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::CubeArray,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_array_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::CubeArray,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_3d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D3,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_3d_multisampled(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D3,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn sampler(sampler_binding_type: SamplerBindingType) -> BindGroupLayoutEntryBuilder {
        BindingType::Sampler(sampler_binding_type).into_bind_group_layout_entry_builder()
    }

    pub fn texture_storage_2d(
        format: TextureFormat,
        access: StorageTextureAccess,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access,
            format,
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_storage_2d_array(
        format: TextureFormat,
        access: StorageTextureAccess,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access,
            format,
            view_dimension: TextureViewDimension::D2Array,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_storage_3d(
        format: TextureFormat,
        access: StorageTextureAccess,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access,
            format,
            view_dimension: TextureViewDimension::D3,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn acceleration_structure() -> BindGroupLayoutEntryBuilder {
        BindingType::AccelerationStructure {
            vertex_return: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn acceleration_structure_vertex_return() -> BindGroupLayoutEntryBuilder {
        BindingType::AccelerationStructure {
            vertex_return: true,
        }
        .into_bind_group_layout_entry_builder()
    }
}
