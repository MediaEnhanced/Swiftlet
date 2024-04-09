//Media Enhanced Swiftlet Graphics Rust Library using Vulkan
//MIT License
//Copyright (c) 2024 Jared Loewenthal
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

pub(super) use std::ffi::{c_char, c_void, CString};
pub(super) use std::ptr;

type NullTerminatedUTF8 = *const c_char;
type MutableU32Ptr = *const u32;
type Bool32 = u32;
type DeviceSize = u64;
type SampleCountFlags = u32;

pub(super) const BOOL_FALSE: u32 = 0;
pub(super) const BOOL_TRUE: u32 = 1;

#[macro_export]
macro_rules! make_api_version {
    ($variant:expr, $major:expr, $minor:expr, $patch:expr) => {
        (($variant as u32) << 29)
            | (($major as u32) << 22)
            | (($minor as u32) << 12)
            | ($patch as u32)
    };
}

#[macro_export]
macro_rules! api_version_1_3 {
    () => {
        make_api_version!(0, 1, 3, 0)
    };
}

#[repr(i32)]
pub(super) enum StructureType {
    ApplicationInfo = 0,
    InstanceCreateInfo,
    DeviceQueueCreateInfo,
    DeviceCreateInfo,
    SubmitInfo,
    MemoryAllocateInfo,
    MappedMemoryRange,
    BindSparseInfo,
    FenceCreateInfo,
    SemaphoreCreateInfo,
    EventCreateInfo,
    QueryPoolCreateInfo,
    BufferCreateInfo,
    BufferViewCreateInfo,
    ImageCreateInfo,
    ImageViewCreateInfo,
    ShaderModuleCreateInfo,
    GraphicsPipelineCreateInfo = 28,
    ComputePipelineCreateInfo,
    PipelineLayoutCreateInfo,
    SamplerCreateInfo,
    DescriptorSetLayoutCreateInfo,
    DescriptorPoolCreateInfo,
    DescriptorSetAllocateInfo,
    WriteDescriptorSet,
    CopyDescriptorSet,
    FramebufferCreateInfo,
    RenderPassCreateInfo,
    CommandPoolCreateInfo,
    CommandBufferAllocateInfo,
    CommandBufferInheritanceInfo,
    CommandBufferBeginInfo,
    RenderPassBeginInfo,
    BufferMemoryBarrier,
    ImageMemoryBarrier,
    MemoryBarrier,
    LoaderInstanceCreateInfo,
    LoaderDeviceCreateInfo,
    SwapchainCreateInfo = 1000001000,
    PresentInfo = 1000001001,
    SurfaceCreateInfoWin32 = 1000009000,
    QueueFamilyVideoProperties = 1000023012,
    PhysicalDeviceFeatures2 = 1000059000,
    PhysicalDeviceProperties2 = 1000059001,
    QueueFamilyProperties2 = 1000059005,
    PhysicalDeviceIdProperties = 1000071004,
    MemoryDedicatedRequirements = 1000127000,
    BufferMemoryRequirementsInfo2 = 1000146000,
    ImageMemoryRequirementsInfo2 = 1000146001,
    MemoryRequirements2 = 1000146003,
    BindBufferMemoryInfo = 1000157000,
    BindImageMemoryInfo = 1000157001,
    MemoryBarrier2 = 1000314000,
    BufferMemoryBarrier2 = 1000314001,
    ImageMemoryBarrier2 = 1000314002,
    DependencyInfo = 1000314003,
    SubmitInfo2 = 1000314004,
    SemaphoreSubmitInfo = 1000314005,
    CommandBufferSubmitInfo = 1000314006,
    PhysicalDeviceSynchronization2Features = 1000314007,
    CopyBufferInfo2 = 1000337000,
    CopyImageInfo2 = 1000337001,
    CopyBufferToImageInfo2 = 1000337002,
    CopyImageToBufferInfo2 = 1000337003,
    BlitImageInfo2 = 1000337004,
    ResolveImageInfo2 = 1000337005,
    BufferCopy2 = 1000337006,
    ImageCopy2 = 1000337007,
    ImageBlit2 = 1000337008,
    BufferImageCopy2 = 1000337009,
    ImageResolve2 = 1000337010,
}

#[repr(C)]
pub(super) struct StructureHeader {
    structure_type: i32,
    next_structure: *const c_void,
}

impl StructureHeader {
    pub(super) fn new(structure_type: StructureType) -> Self {
        StructureHeader {
            structure_type: structure_type as i32,
            next_structure: ptr::null(),
        }
    }

    pub(super) fn set_next_structure(&mut self, next_structure: *const c_void) {
        self.next_structure = next_structure;
    }
}

#[repr(C)]
pub(super) struct ApplicationInfo {
    pub(super) header: StructureHeader,
    pub(super) application_name: NullTerminatedUTF8,
    pub(super) application_version: u32,
    pub(super) engine_name: NullTerminatedUTF8,
    pub(super) engine_version: u32,
    pub(super) api_verison: u32,
}

#[repr(C)]
pub(super) struct InstanceCreateInfo<'a> {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) application_info: &'a ApplicationInfo,
    pub(super) enabled_layer_count: u32,
    pub(super) enabled_layer_names: *const NullTerminatedUTF8,
    pub(super) enabled_extension_count: u32,
    pub(super) enabled_extension_names: *const NullTerminatedUTF8,
}

#[repr(C)]
pub(super) struct AllocationCallbacks {
    // Opaque structure for now
    _unused: [u8; 0],
}

// A structure that is used as a Raw Pointer handle
#[repr(C)]
pub(super) struct OpaqueStructure {
    _unused: [u8; 0],
}
pub(super) type OpaqueHandle = *const OpaqueStructure;

#[repr(C)]
pub(super) struct PhysicalDeviceIdProperties {
    pub(super) header: StructureHeader,
    device_uuid: [u8; 16],
    driver_uuid: [u8; 16],
    pub(super) device_luid: [u8; 8],
    device_node_mask: u32,
    pub(super) device_luid_valid_bool: Bool32,
}

impl Default for PhysicalDeviceIdProperties {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::PhysicalDeviceIdProperties),
            device_uuid: Default::default(),
            driver_uuid: Default::default(),
            device_luid: Default::default(),
            device_node_mask: Default::default(),
            device_luid_valid_bool: Default::default(),
        }
    }
}

#[repr(C)]
pub(super) enum PhysicalDeviceType {
    Other = 0,
    IntegratedGpu = 1,
    DiscreteGpu = 2,
    VirtualGpu = 3,
    Cpu = 4,
}

impl PhysicalDeviceType {
    pub(super) fn from_u32(v: u32) -> Self {
        match v {
            x if x == Self::IntegratedGpu as u32 => Self::IntegratedGpu,
            x if x == Self::DiscreteGpu as u32 => Self::DiscreteGpu,
            x if x == Self::VirtualGpu as u32 => Self::VirtualGpu,
            x if x == Self::Cpu as u32 => Self::Cpu,
            _ => Self::Other,
        }
    }
}

#[derive(Default)]
#[repr(C)]
struct PhysicalDeviceLimits {
    max_image_dimension1_d: u32,
    max_image_dimension2_d: u32,
    max_image_dimension3_d: u32,
    max_image_dimension_cube: u32,
    max_image_array_layers: u32,
    max_texel_buffer_elements: u32,
    max_uniform_buffer_range: u32,
    max_storage_buffer_range: u32,
    max_push_constants_size: u32,
    max_memory_allocation_count: u32,
    max_sampler_allocation_count: u32,
    buffer_image_granularity: DeviceSize,
    sparse_address_space_size: DeviceSize,
    max_bound_descriptor_sets: u32,
    max_per_stage_descriptor_samplers: u32,
    max_per_stage_descriptor_uniform_buffers: u32,
    max_per_stage_descriptor_storage_buffers: u32,
    max_per_stage_descriptor_sampled_images: u32,
    max_per_stage_descriptor_storage_images: u32,
    max_per_stage_descriptor_input_attachments: u32,
    max_per_stage_resources: u32,
    max_descriptor_set_samplers: u32,
    max_descriptor_set_uniform_buffers: u32,
    max_descriptor_set_uniform_buffers_dynamic: u32,
    max_descriptor_set_storage_buffers: u32,
    max_descriptor_set_storage_buffers_dynamic: u32,
    max_descriptor_set_sampled_images: u32,
    max_descriptor_set_storage_images: u32,
    max_descriptor_set_input_attachments: u32,
    max_vertex_input_attributes: u32,
    max_vertex_input_bindings: u32,
    max_vertex_input_attribute_offset: u32,
    max_vertex_input_binding_stride: u32,
    max_vertex_output_components: u32,
    max_tessellation_generation_level: u32,
    max_tessellation_patch_size: u32,
    max_tessellation_control_per_vertex_input_components: u32,
    max_tessellation_control_per_vertex_output_components: u32,
    max_tessellation_control_per_patch_output_components: u32,
    max_tessellation_control_total_output_components: u32,
    max_tessellation_evaluation_input_components: u32,
    max_tessellation_evaluation_output_components: u32,
    max_geometry_shader_invocations: u32,
    max_geometry_input_components: u32,
    max_geometry_output_components: u32,
    max_geometry_output_vertices: u32,
    max_geometry_total_output_components: u32,
    max_fragment_input_components: u32,
    max_fragment_output_attachments: u32,
    max_fragment_dual_src_attachments: u32,
    max_fragment_combined_output_resources: u32,
    max_compute_shared_memory_size: u32,
    max_compute_work_group_count: [u32; 3],
    max_compute_work_group_invocations: u32,
    max_compute_work_group_size: [u32; 3],
    sub_pixel_precision_bits: u32,
    sub_texel_precision_bits: u32,
    mipmap_precision_bits: u32,
    max_draw_indexed_index_value: u32,
    max_draw_indirect_count: u32,
    max_sampler_lod_bias: f32,
    max_sampler_anisotropy: f32,
    max_viewports: u32,
    max_viewport_dimensions: [u32; 2],
    viewport_bounds_range: [f32; 2],
    viewport_sub_pixel_bits: u32,
    min_memory_map_alignment: usize,
    min_texel_buffer_offset_alignment: DeviceSize,
    min_uniform_buffer_offset_alignment: DeviceSize,
    min_storage_buffer_offset_alignment: DeviceSize,
    min_texel_offset: i32,
    max_texel_offset: u32,
    min_texel_gather_offset: i32,
    max_texel_gather_offset: u32,
    min_interpolation_offset: f32,
    max_interpolation_offset: f32,
    sub_pixel_interpolation_offset_bits: u32,
    max_framebuffer_width: u32,
    max_framebuffer_height: u32,
    max_framebuffer_layers: u32,
    framebuffer_color_sample_counts: SampleCountFlags,
    framebuffer_depth_sample_counts: SampleCountFlags,
    framebuffer_stencil_sample_counts: SampleCountFlags,
    framebuffer_no_attachments_sample_counts: SampleCountFlags,
    max_color_attachments: u32,
    sampled_image_color_sample_counts: SampleCountFlags,
    sampled_image_integer_sample_counts: SampleCountFlags,
    sampled_image_depth_sample_counts: SampleCountFlags,
    sampled_image_stencil_sample_counts: SampleCountFlags,
    storage_image_sample_counts: SampleCountFlags,
    max_sample_mask_words: u32,
    timestamp_compute_and_graphics: Bool32,
    timestamp_period: f32,
    max_clip_distances: u32,
    max_cull_distances: u32,
    max_combined_clip_and_cull_distances: u32,
    discrete_queue_priorities: u32,
    point_size_range: [f32; 2],
    line_width_range: [f32; 2],
    point_size_granularity: f32,
    line_width_granularity: f32,
    strict_lines: Bool32,
    standard_sample_locations: Bool32,
    optimal_buffer_copy_offset_alignment: DeviceSize,
    optimal_buffer_copy_row_pitch_alignment: DeviceSize,
    non_coherent_atom_size: DeviceSize,
}

#[derive(Default)]
#[repr(C)]
struct PhysicalDeviceSparseProperties {
    residency_standard2_d_block_shape: Bool32,
    residency_standard2_d_multisample_block_shape: Bool32,
    residency_standard3_d_block_shape: Bool32,
    residency_aligned_mip_size: Bool32,
    residency_non_resident_strict: Bool32,
}

#[repr(C)]
pub(super) struct PhysicalDeviceProperties2 {
    pub(super) header: StructureHeader,
    api_version: u32,
    driver_version: u32,
    vendor_id: u32,
    device_id: u32,
    device_type: PhysicalDeviceType,
    device_name: [u8; 256],
    pipeline_cache_uuid: [u8; 16],
    limits: PhysicalDeviceLimits,
    sparse_properties: PhysicalDeviceSparseProperties,
}

impl Default for PhysicalDeviceProperties2 {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::PhysicalDeviceProperties2),
            api_version: Default::default(),
            driver_version: Default::default(),
            vendor_id: Default::default(),
            device_id: Default::default(),
            device_type: PhysicalDeviceType::Other,
            device_name: [0; 256],
            pipeline_cache_uuid: Default::default(),
            limits: Default::default(),
            sparse_properties: Default::default(),
        }
    }
}

#[repr(C)]
pub(super) enum Format {
    Undefined = 0,
    B8G8R8A8unorm = 44,
}

impl Format {
    fn from_u32(v: u32) -> Self {
        match v {
            x if x == Self::B8G8R8A8unorm as u32 => Self::B8G8R8A8unorm,
            _ => Self::Undefined,
        }
    }
}

#[repr(u32)]
pub(super) enum FormatFeatureFlagBit {
    SampledImage = 0x1,
    BlitSrc = 0x400,
    BlitDst = 0x800,
}
pub(super) type FormatFeatureFlags = u32;

#[derive(Default)]
#[repr(C)]
pub(super) struct FormatProperties {
    linear_tiling_features: FormatFeatureFlags,
    pub(super) optimal_tiling_features: FormatFeatureFlags,
    buffer_features: FormatFeatureFlags,
}

#[repr(u32)]
pub(super) enum MemoryPropertyFlagBit {
    DeviceLocal = 0x01,
    HostVisible = 0x02,
    HostCoherent = 0x04,
    HostCached = 0x08,
    LazilyAllocated = 0x10,
    PropertyProtected = 0x20,
    DeviceCoherentAmd = 0x40,
    DeviceUncachedAmd = 0x80,
    RdmaCapableNv = 0x100,
}
pub(super) type MemoryPropertyFlags = u32;

#[derive(Default)]
#[repr(C)]
pub(super) struct MemoryType {
    pub(super) property_flags: MemoryPropertyFlags,
    pub(super) heap_index: u32,
}

#[repr(u32)]
pub(super) enum MemoryHeapFlagBit {
    DeviceLocal = 0x01,
    MultiInstance = 0x02,
}
pub(super) type MemoryHeapFlags = u32;

#[derive(Default)]
#[repr(C)]
pub(super) struct MemoryHeap {
    pub(super) size: DeviceSize,
    pub(super) flags: MemoryHeapFlags,
}

#[derive(Default)]
#[repr(C)]
pub(super) struct PhysicalDeviceMemoryProperties {
    pub(super) memory_type_count: u32,
    pub(super) memory_types: [MemoryType; 32],
    pub(super) memory_heap_count: u32,
    pub(super) memory_heaps: [MemoryHeap; 16],
}

#[cfg(windows)]
#[repr(C)]
pub(super) struct SurfaceCreateInfoWin32 {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) hinstance: windows::Win32::Foundation::HINSTANCE,
    pub(super) hwnd: windows::Win32::Foundation::HWND,
}

#[repr(C)]
pub(super) enum ColorSpace {
    SrgbNonlinear = 0,
    DisplayP3NonlinearExt = 1000104001,
}

#[repr(C)]
pub(super) struct SurfaceFormat {
    pub(super) format: Format,
    pub(super) color_space: ColorSpace,
}

impl Default for SurfaceFormat {
    fn default() -> Self {
        Self {
            format: Format::Undefined,
            color_space: ColorSpace::SrgbNonlinear,
        }
    }
}

#[repr(C)]
pub(super) enum PresentMode {
    Immediate = 0,
    Mailbox = 1,
    Fifo = 2,
    FifoRelaxed = 3,
    SharedDemandRefresh = 1000111000,
    SharedContinuousRefresh = 1000111001,
}

#[repr(u32)]
pub(super) enum QueueFlagBit {
    Graphics = 0x01,
    Compute = 0x02,
    Transfer = 0x04,
    SparseBinding = 0x08,
    Protected = 0x10,
    VideoDecode = 0x20,
    VideoEncode = 0x40,
    OpticalFlow = 0x80,
}
pub(super) type QueueFlags = u32;

#[derive(Default)]
#[repr(C)]
pub(super) struct Extent3d {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) depth: u32,
}

#[repr(C)]
pub(super) struct QueueFamilyProperties2 {
    pub(super) header: StructureHeader,
    pub(super) queue_flags: QueueFlags,
    queue_count: u32,
    timestamp_valid_bits: u32,
    min_image_transfer_granularity: Extent3d,
}

impl Default for QueueFamilyProperties2 {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::QueueFamilyProperties2),
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: Extent3d::default(),
        }
    }
}

#[repr(u32)]
pub(super) enum VideoCodecOperationFlagBit {
    None = 0,
    DecodeH264 = 0x00000001,
    DecodeH265 = 0x00000002,
    DecodeAv1 = 0x00000004,
    EncodeH264 = 0x00010000,
    EncodeH265 = 0x00020000,
}
pub(super) type VideoCodecOperationFlags = u32;

#[repr(C)]
pub(super) struct QueueFamilyVideoProperties {
    pub(super) header: StructureHeader,
    pub(super) video_codec_operations: VideoCodecOperationFlags,
}

impl Default for QueueFamilyVideoProperties {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::QueueFamilyVideoProperties),
            video_codec_operations: VideoCodecOperationFlagBit::None as VideoCodecOperationFlags,
        }
    }
}

#[derive(Default)]
#[repr(C)]
pub(super) struct Extent2d {
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Default)]
#[repr(C)]
pub(super) struct SurfaceCapabilities {
    min_image_count: u32,
    pub(super) max_image_count: u32,
    pub(super) current_extent: Extent2d,
    min_image_extent: Extent2d,
    max_image_extent: Extent2d,
    max_image_array_layers: u32,
    pub(super) supported_transforms: u32,
    pub(super) current_transform: u32,
    supported_composite_alpha: u32,
    pub(super) supported_usage_flags: u32,
}

#[derive(Default)]
#[repr(C)]
pub(super) struct PhysicalDeviceFeatures {
    robust_buffer_access: Bool32,
    full_draw_index_uint32: Bool32,
    image_cube_array: Bool32,
    independent_blend: Bool32,
    geometry_shader: Bool32,
    tessellation_shader: Bool32,
    sample_rate_shading: Bool32,
    dual_src_blend: Bool32,
    logic_op: Bool32,
    multi_draw_indirect: Bool32,
    draw_indirect_first_instance: Bool32,
    depth_clamp: Bool32,
    depth_bias_clamp: Bool32,
    fill_mode_non_solid: Bool32,
    depth_bounds: Bool32,
    wide_lines: Bool32,
    large_points: Bool32,
    alpha_to_one: Bool32,
    multi_viewport: Bool32,
    sampler_anisotropy: Bool32,
    texture_compression_etc2: Bool32,
    texture_compression_astc_ldr: Bool32,
    texture_compression_bc: Bool32,
    occlusion_query_precise: Bool32,
    pipeline_statistics_query: Bool32,
    vertex_pipeline_stores_and_atomics: Bool32,
    fragment_stores_and_atomics: Bool32,
    shader_tessellation_and_geometry_point_size: Bool32,
    shader_image_gather_extended: Bool32,
    shader_storage_image_extended_formats: Bool32,
    shader_storage_image_multisample: Bool32,
    shader_storage_image_read_without_format: Bool32,
    shader_storage_image_write_without_format: Bool32,
    shader_uniform_buffer_array_dynamic_indexing: Bool32,
    shader_sampled_image_array_dynamic_indexing: Bool32,
    shader_storage_buffer_array_dynamic_indexing: Bool32,
    shader_storage_image_array_dynamic_indexing: Bool32,
    shader_clip_distance: Bool32,
    shader_cull_distance: Bool32,
    shader_float64: Bool32,
    shader_int64: Bool32,
    shader_int16: Bool32,
    shader_resource_residency: Bool32,
    shader_resource_min_lod: Bool32,
    sparse_binding: Bool32,
    sparse_residency_buffer: Bool32,
    sparse_residency_image2_d: Bool32,
    sparse_residency_image3_d: Bool32,
    sparse_residency2_samples: Bool32,
    sparse_residency4_samples: Bool32,
    sparse_residency8_samples: Bool32,
    sparse_residency16_samples: Bool32,
    sparse_residency_aliased: Bool32,
    variable_multisample_rate: Bool32,
    inherited_queries: Bool32,
}

#[repr(C)]
pub(super) struct PhysicalDeviceFeatures2 {
    pub(super) header: StructureHeader,
    pub(super) features: PhysicalDeviceFeatures,
}

impl Default for PhysicalDeviceFeatures2 {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::PhysicalDeviceFeatures2),
            features: PhysicalDeviceFeatures::default(),
        }
    }
}

#[repr(C)]
pub(super) struct PhysicalDeviceSynchronization2Features {
    pub(super) header: StructureHeader,
    pub(super) synchronization2: Bool32,
}

impl Default for PhysicalDeviceSynchronization2Features {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::PhysicalDeviceSynchronization2Features),
            synchronization2: BOOL_FALSE,
        }
    }
}

#[repr(C)]
pub(super) struct DeviceCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) queue_create_info_count: u32,
    pub(super) queue_create_infos: *const DeviceQueueCreateInfo,
    pub(super) enabled_layer_count: u32,
    pub(super) enabled_layer_names: *const NullTerminatedUTF8,
    pub(super) enabled_extension_count: u32,
    pub(super) enabled_extension_names: *const NullTerminatedUTF8,
    pub(super) enabled_features: *const PhysicalDeviceFeatures,
}

#[repr(C)]
pub(super) struct DeviceQueueCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) queue_family_index: u32,
    pub(super) queue_count: u32,
    pub(super) queue_priorities: *const f32,
}

#[repr(u32)]
pub(super) enum ImageUsageFlagBit {
    TransferSrc = 0x00000001,
    TransferDst = 0x00000002,
    Sampled = 0x00000004,
    Storage = 0x00000008,
    ColorAttachment = 0x00000010,
    DepthStencilAttachment = 0x00000020,
    TransientAttachment = 0x00000040,
    InputAttachment = 0x00000080,
    VideoDecodeDst = 0x00000400,
    VideoDecodeSrc = 0x00000800,
    VideoDecodeDpb = 0x00001000,
    VideoEncodeDst = 0x00002000,
    VideoEncodeSrc = 0x00004000,
    VideoEncodeDpb = 0x00008000,
}
pub(super) type ImageUsageFlags = u32;

#[repr(u32)]
pub(super) enum SharingMode {
    Exclusive = 0,
    Concurrent = 1,
}

#[repr(u32)]
pub(super) enum SurfaceTransformFlagBit {
    Identity = 0x00000001,
    Rotate90 = 0x00000002,
    Rotate180 = 0x00000004,
    Rotate270 = 0x00000008,
    HorizontalMirror = 0x00000010,
    HorizontalMirrorRotate90 = 0x00000020,
    HorizontalMirrorRotate180 = 0x00000040,
    HorizontalMirrorRotate270 = 0x00000080,
    Inherit = 0x00000100,
}
pub(super) type SurfaceTransformFlags = u32;

#[repr(u32)]
pub(super) enum CompositeAlphaFlagBit {
    Opaque = 0x00000001,
    PreMultiplied = 0x00000002,
    PostMultiplied = 0x00000004,
    Inherit = 0x00000008,
}
pub(super) type CompositeAlphaFlags = u32;

#[repr(C)]
pub(super) struct SwapchainCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) surface: OpaqueHandle,
    pub(super) min_image_count: u32,
    pub(super) image_format: Format,
    pub(super) image_color_space: ColorSpace,
    pub(super) image_extent: Extent2d,
    pub(super) image_array_layers: u32,
    pub(super) image_usage: ImageUsageFlags,
    pub(super) image_sharing_mode: SharingMode,
    pub(super) queue_family_index_count: u32,
    pub(super) p_queue_family_indices: *const u32,
    pub(super) pre_transform: SurfaceTransformFlags,
    pub(super) composite_alpha: CompositeAlphaFlags,
    pub(super) present_mode: PresentMode,
    pub(super) clipped: Bool32,
    pub(super) old_swapchain: OpaqueHandle,
}

#[repr(C)]
pub(super) struct CommandPoolCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
    pub(super) queue_family_index: u32,
}

#[repr(u32)]
pub(super) enum CommandBufferLevel {
    Primary = 0,
    Secondary = 1,
}

#[repr(C)]
pub(super) struct CommandBufferAllocateInfo {
    pub(super) header: StructureHeader,
    pub(super) command_pool: OpaqueHandle,
    pub(super) command_buffer_level: CommandBufferLevel,
    pub(super) command_buffer_count: u32,
}

#[repr(u32)]
pub(super) enum ImageCreateFlagBit {
    None = 0,
    SparseBinding = 0x00000001,
    SparseResidency = 0x00000002,
    SparseAliased = 0x00000004,
    MutableFormat = 0x00000008,
    CubeCompatible = 0x00000010,
}
pub(super) type ImageCreateFlags = u32;

#[repr(u32)]
pub(super) enum ImageTypeDimensions {
    One = 0,
    Two = 1,
    Three = 2,
}

#[repr(u32)]
pub(super) enum ImageTiling {
    Optimal = 0,
    Linear = 1,
}

#[repr(u32)]
pub(super) enum ImageLayout {
    Undefined = 0,
    General = 1,
    ColorAttachmentOptimal = 2,
    DepthStencilAttachmentOptimal = 3,
    DepthStencilReadOnlyOptimal = 4,
    ShaderReadOnlyOptimal = 5,
    TransferSrcOptimal = 6,
    TransferDstOptimal = 7,
    Preinitialized = 8,
    PresentSrc = 1000001002,
}

#[repr(C)]
pub(super) struct ImageCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: ImageCreateFlags,
    pub(super) image_type: ImageTypeDimensions,
    pub(super) format: Format,
    pub(super) extent: Extent3d,
    pub(super) mip_levels: u32,
    pub(super) array_layers: u32,
    pub(super) samples: SampleCountFlags,
    pub(super) tiling: ImageTiling,
    pub(super) usage: ImageUsageFlags,
    pub(super) sharing_mode: SharingMode,
    pub(super) queue_family_index_count: u32,
    pub(super) p_queue_family_indices: *const u32,
    pub(super) initial_layout: ImageLayout,
}

#[repr(C)]
pub(super) struct ImageMemoryRequirementsInfo2 {
    pub(super) header: StructureHeader,
    pub(super) image: OpaqueHandle,
}

#[repr(u32)]
pub(super) enum BufferCreateFlagBit {
    None = 0,
    SparseBinding = 0x00000001,
    SparseResidency = 0x00000002,
    SparseAliased = 0x00000004,
}
pub(super) type BufferCreateFlags = u32;

#[repr(u32)]
pub(super) enum BufferUsageFlagBit {
    TransferSrc = 0x00000001,
    TransferDst = 0x00000002,
    UniformTexelBuffer = 0x00000004,
    StorageTexelBuffer = 0x00000008,
    UniformBuffer = 0x00000010,
    StorageBuffer = 0x00000020,
    IndexBuffer = 0x00000040,
    VertexBuffer = 0x00000080,
    IndirectBuffer = 0x00000100,
    VideoDecodeSrc = 0x00002000,
    VideoDecodeDst = 0x00004000,
    VideoEncodeDst = 0x00008000,
    VideoEncodeSrc = 0x00010000,
}
pub(super) type BufferUsageFlags = u32;

#[repr(C)]
pub(super) struct BufferCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: BufferCreateFlags,
    pub(super) size: DeviceSize,
    pub(super) usage: BufferUsageFlags,
    pub(super) sharing_mode: SharingMode,
    pub(super) queue_family_index_count: u32,
    pub(super) p_queue_family_indices: *const u32,
}

#[repr(C)]
pub(super) struct BufferMemoryRequirementsInfo2 {
    pub(super) header: StructureHeader,
    pub(super) buffer: OpaqueHandle,
}

#[repr(C)]
pub(super) struct MemoryRequirements2 {
    pub(super) header: StructureHeader,
    pub(super) size: DeviceSize,
    pub(super) alignment: DeviceSize,
    pub(super) memory_type_bits: u32,
}

impl Default for MemoryRequirements2 {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::MemoryRequirements2),
            size: 0,
            alignment: 0,
            memory_type_bits: 0,
        }
    }
}

#[repr(u64)]
pub(super) enum PipelineStageFlag2Bit {
    None = 0x00,
    TopOfPipe = 0x01,
    DrawIndirect = 0x02,
    VertexInput = 0x04,
    VertexShader = 0x08,
    TessellationControlShader = 0x10,
    TessellationEvaluationShader = 0x20,
    GeometryShader = 0x40,
    FragmentShader = 0x80,
    AllCommands = 0x00010000,
}
pub(super) type PipelineStageFlags2 = u64;

#[repr(u64)]
pub(super) enum AccessFlag2Bit {
    None = 0x00,
    IndirectCommandRead = 0x01,
    IndexRead = 0x02,
    VertexAttributeRead = 0x04,
    UniformRead = 0x08,
    TransferRead = 0x800,
    TransferWrite = 0x1000,
}
pub(super) type AccessFlags2 = u64;

#[repr(u32)]
pub(super) enum ImageAspectFlagBit {
    None = 0x00,
    Color = 0x01,
    Depth = 0x02,
    Stencil = 0x04,
    Metadata = 0x08,
    Plane0 = 0x10,
    Plane1 = 0x20,
    Plane2 = 0x40,
}
pub(super) type ImageAspectFlags = u32;

#[repr(C)]
pub(super) struct ImageSubresourceRange {
    pub(super) aspect_mask: ImageAspectFlags,
    pub(super) base_mip_level: u32,
    pub(super) level_count: u32,
    pub(super) base_array_layer: u32,
    pub(super) layer_count: u32,
}

#[repr(C)]
pub(super) struct ImageMemoryBarrier2 {
    pub(super) header: StructureHeader,
    pub(super) src_stage_mask: PipelineStageFlags2,
    pub(super) src_access_mask: AccessFlags2,
    pub(super) dst_stage_mask: PipelineStageFlags2,
    pub(super) dst_access_mask: AccessFlags2,
    pub(super) old_layout: ImageLayout,
    pub(super) new_layout: ImageLayout,
    pub(super) src_queue_family_index: u32,
    pub(super) dst_queue_family_index: u32,
    pub(super) image: OpaqueHandle,
    pub(super) subresource_range: ImageSubresourceRange,
}

#[repr(C)]
pub(super) struct MemoryDedicatedRequirements {
    pub(super) header: StructureHeader,
    pub(super) prefers_dedicated_allocation: Bool32,
    pub(super) requires_dedicated_allocation: Bool32,
}

impl Default for MemoryDedicatedRequirements {
    fn default() -> Self {
        Self {
            header: StructureHeader::new(StructureType::MemoryDedicatedRequirements),
            prefers_dedicated_allocation: BOOL_FALSE,
            requires_dedicated_allocation: BOOL_FALSE,
        }
    }
}

#[repr(C)]
pub(super) struct MemoryAllocateInfo {
    pub(super) header: StructureHeader,
    pub(super) allocation_size: DeviceSize,
    pub(super) memory_type_index: u32,
}

#[repr(C)]
pub(super) struct BindImageMemoryInfo {
    pub(super) header: StructureHeader,
    pub(super) image: OpaqueHandle,
    pub(super) memory: OpaqueHandle,
    pub(super) memory_offset: DeviceSize,
}

#[repr(C)]
pub(super) struct BindBufferMemoryInfo {
    pub(super) header: StructureHeader,
    pub(super) buffer: OpaqueHandle,
    pub(super) memory: OpaqueHandle,
    pub(super) memory_offset: DeviceSize,
}

#[repr(C)]
pub(super) struct CommandBufferSubmitInfo {
    pub(super) header: StructureHeader,
    pub(super) command_buffer: OpaqueHandle,
    pub(super) device_mask: u32,
}

#[repr(C)]
pub(super) struct SemaphoreCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: u32,
}

#[repr(u32)]
pub(super) enum FenceCreateFlagBit {
    None = 0x0,
    Signaled = 0x1,
}
pub(super) type FenceCreateFlags = u32;

#[repr(C)]
pub(super) struct FenceCreateInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: FenceCreateFlags,
}

#[repr(C)]
pub(super) struct SemaphoreSubmitInfo {
    pub(super) header: StructureHeader,
    pub(super) semaphore: OpaqueHandle,
    pub(super) value: u64,
    pub(super) stage_mask: PipelineStageFlags2,
    pub(super) device_index: u32,
}

#[repr(u32)]
pub(super) enum SubmitFlagBit {
    None = 0x0,
    Protected = 0x1,
}
pub(super) type SubmitFlags = u32;

#[repr(C)]
pub(super) struct SubmitInfo2 {
    pub(super) header: StructureHeader,
    pub(super) flags: SubmitFlags,
    pub(super) wait_semaphore_info_count: u32,
    pub(super) wait_semaphore_infos: *const SemaphoreSubmitInfo,
    pub(super) command_buffer_info_count: u32,
    pub(super) command_buffer_infos: *const CommandBufferSubmitInfo,
    pub(super) signal_semaphore_info_count: u32,
    pub(super) signal_semaphore_infos: *const SemaphoreSubmitInfo,
}

#[repr(C)]
pub(super) struct PresentInfo {
    pub(super) header: StructureHeader,
    pub(super) wait_semaphore_count: u32,
    pub(super) wait_semaphores: *const OpaqueHandle,
    pub(super) swapchain_count: u32,
    pub(super) swapchains: *const OpaqueHandle,
    pub(super) image_indicies: *const u32,
    pub(super) results: *const i32,
}

#[repr(u32)]
pub(super) enum CommandBufferUsageFlagBit {
    None = 0x0,
    OneTimeSubmit = 0x1,
    RenderPassContinue = 0x2,
    SimultaneousUse = 0x4,
}
pub(super) type CommandBufferUsageFlags = u32;

#[repr(u32)]
pub(super) enum QueryControlFlagBit {
    None = 0x0,
    Precise = 0x1,
}
pub(super) type QueryControlFlags = u32;

#[repr(u32)]
pub(super) enum QueryPipelineStatisticFlagBit {
    InputAssemblyVertices = 0x00000001,
    InputAssemblyPrimitives = 0x00000002,
    VertexShaderInvocations = 0x00000004,
    GeometryShaderInvocations = 0x00000008,
    GeometryShaderPrimitives = 0x00000010,
    ClippingInvocations = 0x00000020,
    ClippingPrimitives = 0x00000040,
    FragmentShaderInvocations = 0x00000080,
}
pub(super) type QueryPipelineStatisticFlags = u32;

#[repr(C)]
pub(super) struct CommandBufferInheritanceInfo {
    pub(super) header: StructureHeader,
    pub(super) render_pass: OpaqueHandle,
    pub(super) subpass: u32,
    pub(super) framebuffer: OpaqueHandle,
    pub(super) occlusion_query_enable: Bool32,
    pub(super) query_flags: QueryControlFlags,
    pub(super) pipeline_statistics: QueryPipelineStatisticFlags,
}

#[repr(C)]
pub(super) struct CommandBufferBeginInfo {
    pub(super) header: StructureHeader,
    pub(super) flags: CommandBufferUsageFlags,
    pub(super) inheritance_info: *const CommandBufferInheritanceInfo,
}

#[repr(C)]
pub(super) struct ImageSubresourceLayers {
    pub(super) aspect_mask: ImageAspectFlags,
    pub(super) mip_level: u32,
    pub(super) base_array_layer: u32,
    pub(super) layer_count: u32,
}

#[repr(u32)]
pub(super) enum DependencyFlagBit {
    None = 0x0,
    ByRegion = 0x1,
    ViewLocal = 0x2,
    DeviceGroup = 0x4,
    FeedbackLoop = 0x8,
}
pub(super) type DependencyFlags = u32;

#[repr(C)]
pub(super) struct MemoryBarrier2 {
    pub(super) header: StructureHeader,
    pub(super) src_stage_mask: PipelineStageFlags2,
    pub(super) src_access_mask: AccessFlags2,
    pub(super) dst_stage_mask: PipelineStageFlags2,
    pub(super) dst_access_mask: AccessFlags2,
}

#[repr(C)]
pub(super) struct BufferMemoryBarrier2 {
    pub(super) header: StructureHeader,
    pub(super) src_stage_mask: PipelineStageFlags2,
    pub(super) src_access_mask: AccessFlags2,
    pub(super) dst_stage_mask: PipelineStageFlags2,
    pub(super) dst_access_mask: AccessFlags2,
    pub(super) src_queue_family_index: u32,
    pub(super) dst_queue_family_index: u32,
    pub(super) buffer: OpaqueHandle,
    pub(super) offset: DeviceSize,
    pub(super) size: DeviceSize,
}

#[repr(C)]
pub(super) struct DependencyInfo {
    pub(super) header: StructureHeader,
    pub(super) dependency_flags: DependencyFlags,
    pub(super) memory_barrier_count: u32,
    pub(super) memory_barriers: *const MemoryBarrier2,
    pub(super) buffer_memory_barrier_count: u32,
    pub(super) buffer_memory_barriers: *const BufferMemoryBarrier2,
    pub(super) image_memory_barrier_count: u32,
    pub(super) image_memory_barriers: *const ImageMemoryBarrier2,
}

#[derive(Default)]
#[repr(C)]
pub(super) struct Offset3d {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) z: i32,
}

#[repr(C)]
pub(super) struct BufferImageCopy {
    pub(super) buffer_offset: DeviceSize,
    pub(super) buffer_row_length: u32,
    pub(super) buffer_image_height: u32,
    pub(super) image_subresource: ImageSubresourceLayers,
    pub(super) image_offset: Offset3d,
    pub(super) image_extent: Extent3d,
}

#[repr(C)]
pub(super) struct ImageBlit2 {
    pub(super) header: StructureHeader,
    pub(super) src_subresource: ImageSubresourceLayers,
    pub(super) src_offsets: [Offset3d; 2],
    pub(super) dst_subresource: ImageSubresourceLayers,
    pub(super) dst_offsets: [Offset3d; 2],
}

#[repr(u32)]
pub(super) enum Filter {
    Nearest = 0,
    Linear = 1,
    Cubic = 1000015000,
}

#[repr(C)]
pub(super) struct BlitImageInfo2 {
    pub(super) header: StructureHeader,
    pub(super) src_image: OpaqueHandle,
    pub(super) src_image_layout: ImageLayout,
    pub(super) dst_image: OpaqueHandle,
    pub(super) dst_image_layout: ImageLayout,
    pub(super) region_count: u32,
    pub(super) regions: *const ImageBlit2,
    pub(super) filter: Filter,
}

#[repr(u32)]
pub(super) enum MemoryMapFlagBit {
    None = 0x0,
    Placed = 0x1,
}
pub(super) type MemoryMapFlags = u32;

#[cfg_attr(windows, link(name = "vulkan-1", kind = "raw-dylib"))]
extern "C" {
    pub(super) fn vkCreateInstance(
        create_info: *const InstanceCreateInfo,
        allocator: *const AllocationCallbacks,
        instance_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkEnumeratePhysicalDevices(
        instance: OpaqueHandle,
        physical_device_count: MutableU32Ptr,
        physical_devices: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetPhysicalDeviceProperties2(
        physical_device: OpaqueHandle,
        physical_device_count: *const PhysicalDeviceProperties2,
    );

    pub(super) fn vkGetPhysicalDeviceFormatProperties(
        physical_device: OpaqueHandle,
        format: Format,
        format_properties: *const FormatProperties,
    );

    pub(super) fn vkGetPhysicalDeviceMemoryProperties(
        physical_device: OpaqueHandle,
        memory_properties: *const PhysicalDeviceMemoryProperties,
    );

    #[cfg(windows)]
    pub(super) fn vkCreateWin32SurfaceKHR(
        instance: OpaqueHandle,
        create_info: *const SurfaceCreateInfoWin32,
        allocator: *const AllocationCallbacks,
        surface_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetPhysicalDeviceSurfaceFormatsKHR(
        physical_device: OpaqueHandle,
        surface: OpaqueHandle,
        surface_format_count: MutableU32Ptr,
        surface_formats: *const SurfaceFormat,
    ) -> i32;

    pub(super) fn vkGetPhysicalDeviceSurfacePresentModesKHR(
        physical_device: OpaqueHandle,
        surface: OpaqueHandle,
        present_mode_count: MutableU32Ptr,
        present_modes: *const PresentMode,
    ) -> i32;

    pub(super) fn vkGetPhysicalDeviceQueueFamilyProperties2(
        physical_device: OpaqueHandle,
        queue_family_property_count: MutableU32Ptr,
        queue_family_properties: *const QueueFamilyProperties2,
    );

    pub(super) fn vkGetPhysicalDeviceSurfaceSupportKHR(
        physical_device: OpaqueHandle,
        queue_family_index: u32,
        surface: OpaqueHandle,
        is_supported: MutableU32Ptr,
    ) -> i32;

    // Should convert to v2 in future
    pub(super) fn vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
        physical_device: OpaqueHandle,
        surface: OpaqueHandle,
        surface_capabilities: *const SurfaceCapabilities,
    ) -> i32;

    pub(super) fn vkGetPhysicalDeviceFeatures2(
        physical_device: OpaqueHandle,
        features: *const PhysicalDeviceFeatures2,
    );

    pub(super) fn vkCreateDevice(
        physical_device: OpaqueHandle,
        create_info: *const DeviceCreateInfo,
        allocator: *const AllocationCallbacks,
        device_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetDeviceQueue(
        device: OpaqueHandle,
        queue_family_index: u32,
        queue_index: u32,
        queue_ptr: *const OpaqueHandle,
    );

    pub(super) fn vkCreateSwapchainKHR(
        device: OpaqueHandle,
        create_info: *const SwapchainCreateInfo,
        allocator: *const AllocationCallbacks,
        swapchain_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetSwapchainImagesKHR(
        device: OpaqueHandle,
        swapchain: OpaqueHandle,
        swapchain_image_count: MutableU32Ptr,
        swapchain_images: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkCreateCommandPool(
        device: OpaqueHandle,
        create_info: *const CommandPoolCreateInfo,
        allocator: *const AllocationCallbacks,
        command_pool_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkAllocateCommandBuffers(
        device: OpaqueHandle,
        allocate_info: *const CommandBufferAllocateInfo,
        command_buffers: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkCreateImage(
        device: OpaqueHandle,
        create_info: *const ImageCreateInfo,
        allocator: *const AllocationCallbacks,
        image_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetImageMemoryRequirements2(
        device: OpaqueHandle,
        info: *const ImageMemoryRequirementsInfo2,
        memory_requirements: *const MemoryRequirements2,
    );

    pub(super) fn vkCreateBuffer(
        device: OpaqueHandle,
        create_info: *const BufferCreateInfo,
        allocator: *const AllocationCallbacks,
        buffer_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkGetBufferMemoryRequirements2(
        device: OpaqueHandle,
        info: *const BufferMemoryRequirementsInfo2,
        memory_requirements: *const MemoryRequirements2,
    );

    pub(super) fn vkAllocateMemory(
        device: OpaqueHandle,
        allocate_info: *const MemoryAllocateInfo,
        allocator: *const AllocationCallbacks,
        device_memory_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkBindImageMemory2(
        device: OpaqueHandle,
        bind_info_count: u32,
        bind_infos: *const BindImageMemoryInfo,
    ) -> i32;

    pub(super) fn vkBindBufferMemory2(
        device: OpaqueHandle,
        bind_info_count: u32,
        bind_infos: *const BindBufferMemoryInfo,
    ) -> i32;

    pub(super) fn vkCreateSemaphore(
        device: OpaqueHandle,
        create_info: *const SemaphoreCreateInfo,
        allocator: *const AllocationCallbacks,
        semaphore_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkCreateFence(
        device: OpaqueHandle,
        create_info: *const FenceCreateInfo,
        allocator: *const AllocationCallbacks,
        fence_ptr: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkAcquireNextImageKHR(
        device: OpaqueHandle,
        swapchain: OpaqueHandle,
        timeout: u64,
        semaphore: OpaqueHandle,
        fence: OpaqueHandle,
        image_index: MutableU32Ptr,
    ) -> i32;

    pub(super) fn vkQueueSubmit2(
        queue: OpaqueHandle,
        submit_info_count: u32,
        submit_infos: *const SubmitInfo2,
        fence: OpaqueHandle,
    ) -> i32;

    pub(super) fn vkQueueWaitIdle(queue: OpaqueHandle) -> i32;

    pub(super) fn vkQueuePresentKHR(queue: OpaqueHandle, present_info: *const PresentInfo) -> i32;

    pub(super) fn vkBeginCommandBuffer(
        cmd_buffer: OpaqueHandle,
        begin_info: *const CommandBufferBeginInfo,
    ) -> i32;

    pub(super) fn vkEndCommandBuffer(cmd_buffer: OpaqueHandle) -> i32;

    pub(super) fn vkCmdPipelineBarrier2(
        cmd_buffer: OpaqueHandle,
        dependency_info: *const DependencyInfo,
    );

    pub(super) fn vkCmdCopyBufferToImage(
        cmd_buffer: OpaqueHandle,
        buffer: OpaqueHandle,
        image: OpaqueHandle,
        img_layout: ImageLayout,
        region_count: u32,
        regions: *const BufferImageCopy,
    );

    pub(super) fn vkCmdCopyImageToBuffer(
        cmd_buffer: OpaqueHandle,
        image: OpaqueHandle,
        img_layout: ImageLayout,
        buffer: OpaqueHandle,
        region_count: u32,
        regions: *const BufferImageCopy,
    );

    pub(super) fn vkCmdBlitImage2(cmd_buffer: OpaqueHandle, blit_image_info: *const BlitImageInfo2);

    pub(super) fn vkWaitForFences(
        device: OpaqueHandle,
        fence_count: u32,
        fences: *const OpaqueHandle,
        wait_all: Bool32,
        timeout: u64,
    ) -> i32;

    pub(super) fn vkResetFences(
        device: OpaqueHandle,
        fence_count: u32,
        fences: *const OpaqueHandle,
    ) -> i32;

    pub(super) fn vkMapMemory(
        device: OpaqueHandle,
        memory: OpaqueHandle,
        offset: DeviceSize,
        size: DeviceSize,
        flags: MemoryMapFlags,
        data: *const *mut c_void,
    ) -> i32;

    pub(super) fn vkUnmapMemory(device: OpaqueHandle, memory: OpaqueHandle);
}
