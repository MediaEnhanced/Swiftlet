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

#[macro_use]
pub mod api;
use std::mem;

use api::{
    c_void, ptr, ApplicationInfo, CString, Format, FormatFeatureFlagBit, FormatFeatureFlags,
    InstanceCreateInfo, OpaqueHandle, PhysicalDeviceIdProperties, PhysicalDeviceProperties2,
    StructureHeader, StructureType, BOOL_FALSE, BOOL_TRUE,
};

use self::api::vkUnmapMemory;

mod spirv;

// #[derive(Debug)]
// #[repr(i32)]
// pub enum VkResult {
//     Success = 0,
//     NotReady,
//     Timeout,
//     EventSet,
//     EventReset,
//     Incomplete,
//     Uncertain,
// }
// impl VkResult {
//     fn from_result(result: i32) -> Self {
//         match result {
//             x if x == Self::Ok as i32 => Error::Ok,
//             x if x == Error::FileNotFound as i32 => Error::FileNotFound,
//             x if x == Error::FilePermission as i32 => Error::FilePermission,
//             x if x == Error::TooManyFilesOpen as i32 => Error::TooManyFilesOpen,
//             x if x == Error::BadFilePath as i32 => Error::BadFilePath,
//             x if x == Error::Param as i32 => Error::Param,
//             x if x == Error::MemFull as i32 => Error::MemFull,
//             _ => Error::Unimplemented,
//         }
//     }
// }

#[derive(Debug)]
pub enum Error {
    StringConversion,
    VkResult(i32),
    //CannotFindPhysicalDevice,
    BadOptimalFeatures,
    Synchronization2NotSupported,
    BadQueueFamilyIndex,
    NoUseableFormat,
    NoPresentationMode,
    NoQueueFamily,
    SwapchainImageCount,
    BadTransform,
    SurfaceNoTransfer,
    BadSwapchainImageCount,
    InvalidMapPtr,
}

pub const LAYER_NAME_VALIDATION: &str = "VK_LAYER_KHRONOS_validation";
pub const LAYER_NAME_SYNC: &str = "VK_LAYER_KHRONOS_synchronization2";

pub const INSTANCE_EXTENSION_NAME_SURFACE: &str = "VK_KHR_surface";
#[cfg(target_os = "windows")]
pub const INSTANCE_EXTENSION_NAME_OS_SURFACE: &str = "VK_KHR_win32_surface";
#[cfg(target_os = "macos")]
pub const INSTANCE_EXTENSION_NAME_OS_SURFACE: &str = "VK_EXT_metal_surface";
pub const INSTANCE_EXTENSION_NAME_DEBUG: &str = "VK_EXT_debug_utils";

pub struct Instance {
    handle: OpaqueHandle,
}

impl Instance {
    pub fn new(
        app_name: &str,
        engine_name: &str,
        layer_names: &[&str],
        extension_names: &[&str],
    ) -> Result<Self, Error> {
        let app_name_c = match CString::new(app_name) {
            Ok(s) => s,
            Err(_e) => return Err(Error::StringConversion),
        };
        let engine_name_c = match CString::new(engine_name) {
            Ok(s) => s,
            Err(_e) => return Err(Error::StringConversion),
        };
        let app_info = ApplicationInfo {
            header: StructureHeader::new(StructureType::ApplicationInfo),
            application_name: app_name_c.as_ptr(),
            application_version: 1,
            engine_name: engine_name_c.as_ptr(),
            engine_version: make_api_version!(0, 1, 0, 0),
            api_verison: api_version_1_3!(),
        };

        let mut layer_names_ptr = Vec::with_capacity(layer_names.len());
        let mut layer_names_c = Vec::with_capacity(layer_names.len());
        for n in layer_names {
            let cstr = match CString::new(*n) {
                Ok(s) => s,
                Err(_e) => return Err(Error::StringConversion),
            };
            layer_names_ptr.push(cstr.as_ptr());
            layer_names_c.push(cstr);
        }

        let mut ext_names_ptr = Vec::with_capacity(extension_names.len());
        let mut ext_names_c = Vec::with_capacity(extension_names.len());
        for n in extension_names {
            let cstr = match CString::new(*n) {
                Ok(s) => s,
                Err(_e) => return Err(Error::StringConversion),
            };
            ext_names_ptr.push(cstr.as_ptr());
            ext_names_c.push(cstr);
        }

        let create_info = InstanceCreateInfo {
            header: StructureHeader::new(StructureType::InstanceCreateInfo),
            flags: 0,
            application_info: &app_info,
            enabled_layer_count: layer_names_ptr.len() as u32,
            enabled_layer_names: layer_names_ptr.as_ptr(),
            enabled_extension_count: ext_names_ptr.len() as u32,
            enabled_extension_names: ext_names_ptr.as_ptr(),
        };

        let handle = ptr::null();
        let result = unsafe { api::vkCreateInstance(&create_info, ptr::null(), &handle) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        Ok(Instance { handle })
    }
}

pub struct PhysicalDevice {
    handle: OpaqueHandle,
    instance: Instance,
    local_only_memory_type_index: u32,
    basic_cpu_access_memory_type_index: u32,
}

impl PhysicalDevice {
    pub fn new(instance: Instance) -> Result<Option<Self>, Error> {
        let devices = [ptr::null(); 32];
        let device_count = devices.len() as u32;
        let result = unsafe {
            api::vkEnumeratePhysicalDevices(instance.handle, &device_count, devices.as_ptr())
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        let handle = devices[0];

        let format_properties = api::FormatProperties::default();
        unsafe {
            api::vkGetPhysicalDeviceFormatProperties(
                handle,
                Format::B8G8R8A8unorm,
                &format_properties,
            )
        }
        let optimal_features = format_properties.optimal_tiling_features;
        if !((optimal_features & FormatFeatureFlagBit::BlitSrc as FormatFeatureFlags > 0)
            && (optimal_features & FormatFeatureFlagBit::BlitDst as FormatFeatureFlags > 0)
            && (optimal_features & FormatFeatureFlagBit::SampledImage as FormatFeatureFlags > 0))
        {
            return Err(Error::BadOptimalFeatures);
        }

        let memory_properties = api::PhysicalDeviceMemoryProperties::default();
        unsafe { api::vkGetPhysicalDeviceMemoryProperties(handle, &memory_properties) };

        let mut local_only_memory_type_index = 0;
        for i in 0..memory_properties.memory_type_count {
            if (memory_properties.memory_types[i as usize].property_flags
                & api::MemoryPropertyFlagBit::DeviceLocal as api::MemoryPropertyFlags)
                > 0
            {
                local_only_memory_type_index = i;
                break;
            }
        }

        let mut basic_cpu_access_memory_type_index = 0;
        for i in 0..memory_properties.memory_type_count {
            if ((memory_properties.memory_types[i as usize].property_flags
                & api::MemoryPropertyFlagBit::HostVisible as api::MemoryPropertyFlags)
                > 0)
                && ((memory_properties.memory_types[i as usize].property_flags
                    & api::MemoryPropertyFlagBit::HostCoherent as api::MemoryPropertyFlags)
                    > 0)
            {
                basic_cpu_access_memory_type_index = i;
                break;
            }
        }

        Ok(Some(PhysicalDevice {
            handle,
            instance,
            local_only_memory_type_index,
            basic_cpu_access_memory_type_index,
        }))
    }

    pub fn new_from_luid(instance: Instance, luid: [u32; 2]) -> Result<Option<Self>, Error> {
        let devices = [ptr::null(); 32];
        let device_count = devices.len() as u32;
        let result = unsafe {
            api::vkEnumeratePhysicalDevices(instance.handle, &device_count, devices.as_ptr())
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut device_properties = PhysicalDeviceProperties2::default();
        let device_id_properties = PhysicalDeviceIdProperties::default();
        device_properties
            .header
            .set_next_structure(ptr::addr_of!(device_id_properties) as *const c_void);

        let devices_len = devices.len();
        let mut device_index = devices_len;
        #[allow(clippy::needless_range_loop)]
        for i in 0..devices_len {
            unsafe { api::vkGetPhysicalDeviceProperties2(devices[i], &device_properties) }

            if device_id_properties.device_luid_valid_bool == BOOL_TRUE {
                let device_luid_0 = u32::from_ne_bytes([
                    device_id_properties.device_luid[0],
                    device_id_properties.device_luid[1],
                    device_id_properties.device_luid[2],
                    device_id_properties.device_luid[3],
                ]);
                let device_luid_1 = u32::from_ne_bytes([
                    device_id_properties.device_luid[4],
                    device_id_properties.device_luid[5],
                    device_id_properties.device_luid[6],
                    device_id_properties.device_luid[7],
                ]);

                if (luid[0] == device_luid_0) && (luid[1] == device_luid_1) {
                    device_index = i;
                    break;
                }
            }
        }
        if device_index == devices_len {
            return Ok(None);
        }
        let handle = devices[device_index];

        let format_properties = api::FormatProperties::default();
        unsafe {
            api::vkGetPhysicalDeviceFormatProperties(
                handle,
                Format::B8G8R8A8unorm,
                &format_properties,
            )
        }
        let optimal_features = format_properties.optimal_tiling_features;
        if !((optimal_features & FormatFeatureFlagBit::BlitSrc as FormatFeatureFlags > 0)
            && (optimal_features & FormatFeatureFlagBit::BlitDst as FormatFeatureFlags > 0)
            && (optimal_features & FormatFeatureFlagBit::SampledImage as FormatFeatureFlags > 0))
        {
            return Err(Error::BadOptimalFeatures);
        }

        let memory_properties = api::PhysicalDeviceMemoryProperties::default();
        unsafe { api::vkGetPhysicalDeviceMemoryProperties(handle, &memory_properties) };

        let mut local_only_memory_type_index = 0;
        for i in 0..memory_properties.memory_type_count {
            if (memory_properties.memory_types[i as usize].property_flags
                & api::MemoryPropertyFlagBit::DeviceLocal as api::MemoryPropertyFlags)
                > 0
            {
                local_only_memory_type_index = i;
                break;
            }
        }

        let mut basic_cpu_access_memory_type_index = 0;
        for i in 0..memory_properties.memory_type_count {
            if ((memory_properties.memory_types[i as usize].property_flags
                & api::MemoryPropertyFlagBit::HostVisible as api::MemoryPropertyFlags)
                > 0)
                && ((memory_properties.memory_types[i as usize].property_flags
                    & api::MemoryPropertyFlagBit::HostCoherent as api::MemoryPropertyFlags)
                    > 0)
            {
                basic_cpu_access_memory_type_index = i;
                break;
            }
        }

        Ok(Some(PhysicalDevice {
            handle,
            instance,
            local_only_memory_type_index,
            basic_cpu_access_memory_type_index,
        }))
    }
}

const DEVICE_EXTENSION_NAME_SYNCHRONIZATION: &str = "VK_KHR_synchronization2"; // Shouldn't need this function
const DEVICE_EXTENSION_NAME_SWAPCHAIN: &str = "VK_KHR_swapchain";
const DEVICE_EXTENSION_NAME_EXTERNAL_MEMORY_WIN32: &str = "VK_KHR_external_memory_win32";
const DEVICE_EXTENSION_NAME_WIN32_KEYED_MUTEX: &str = "VK_KHR_win32_keyed_mutex";

const DEVICE_EXTENSION_NAME_VIDEO_QUEUE: &str = "VK_KHR_video_queue";
const DEVICE_EXTENSION_NAME_VIDEO_DECODE_QUEUE: &str = "VK_KHR_video_decode_queue";
const DEVICE_EXTENSION_NAME_VIDEO_DECODE_H265: &str = "VK_KHR_video_decode_h265";

pub struct Device {
    handle: OpaqueHandle,
    physical_device: PhysicalDevice,
}

impl Device {
    fn new(
        physical_device: PhysicalDevice,
        queue_create_infos: &[api::DeviceQueueCreateInfo],
        extension_names: &[&str],
    ) -> Result<Self, Error> {
        let mut ext_names_ptr = Vec::with_capacity(extension_names.len());
        let mut ext_names_c = Vec::with_capacity(extension_names.len());
        for n in extension_names {
            let cstr = match CString::new(*n) {
                Ok(s) => s,
                Err(_e) => return Err(Error::StringConversion),
            };
            ext_names_ptr.push(cstr.as_ptr());
            ext_names_c.push(cstr);
        }

        let mut physical_device_features = api::PhysicalDeviceFeatures2::default();
        let sync_fetures = api::PhysicalDeviceSynchronization2Features::default();
        physical_device_features
            .header
            .set_next_structure(ptr::addr_of!(sync_fetures) as *const c_void);

        unsafe {
            api::vkGetPhysicalDeviceFeatures2(physical_device.handle, &physical_device_features)
        };
        if sync_fetures.synchronization2 != BOOL_TRUE {
            return Err(Error::Synchronization2NotSupported);
        }

        let mut device_create_info = api::DeviceCreateInfo {
            header: StructureHeader::new(StructureType::DeviceCreateInfo),
            flags: 0,
            queue_create_info_count: queue_create_infos.len() as u32,
            queue_create_infos: queue_create_infos.as_ptr(),
            enabled_layer_count: 0,           // Depricated and Ignored
            enabled_layer_names: ptr::null(), // Depricated and Ignored
            enabled_extension_count: ext_names_ptr.len() as u32,
            enabled_extension_names: ext_names_ptr.as_ptr(),
            enabled_features: &physical_device_features.features,
        };

        device_create_info
            .header
            .set_next_structure(ptr::addr_of!(sync_fetures) as *const c_void);

        let handle = ptr::null();
        let result = unsafe {
            api::vkCreateDevice(
                physical_device.handle,
                &device_create_info,
                ptr::null(),
                &handle,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        Ok(Device {
            handle,
            physical_device,
        })
    }

    fn get_queue_handle(&self, queue_family_index: u32) -> Result<OpaqueHandle, Error> {
        let queue_handle = ptr::null();
        unsafe { api::vkGetDeviceQueue(self.handle, queue_family_index, 0, &queue_handle) }
        if !queue_handle.is_null() {
            Ok(queue_handle)
        } else {
            Err(Error::BadQueueFamilyIndex)
        }
    }
}

const SWAPCHAIN_PRESENT_MODE: api::PresentMode = api::PresentMode::Immediate;
const SWAPCHAIN_IMAGE_COUNT: u32 = 3;

pub struct Swapchain {
    present_info: api::PresentInfo,
    submit_info: api::SubmitInfo2,
    signal_semaphore_submit_info: api::SemaphoreSubmitInfo,
    wait_semaphore_submit_info: api::SemaphoreSubmitInfo,
    cmd_buffer_submit_infos: [api::CommandBufferSubmitInfo; SWAPCHAIN_IMAGE_COUNT as usize],
    command_pool: OpaqueHandle,
    //image_memory_barrier: api::ImageMemoryBarrier2,
    image_handles: [OpaqueHandle; SWAPCHAIN_IMAGE_COUNT as usize],
    swapchain_create_info: api::SwapchainCreateInfo,
    handle: OpaqueHandle,
    queue: OpaqueHandle,
    device: Device,
}

impl Swapchain {
    fn create(
        physical_device: PhysicalDevice,
        surface_handle: OpaqueHandle,
    ) -> Result<Self, Error> {
        let surface_format_count = 0;
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfaceFormatsKHR(
                physical_device.handle,
                surface_handle,
                &surface_format_count,
                ptr::null(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        //println!("Format Count: {}", surface_format_count);

        let mut surface_formats = Vec::with_capacity(surface_format_count as usize);
        for _i in 0..surface_format_count {
            surface_formats.push(api::SurfaceFormat::default());
        }
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfaceFormatsKHR(
                physical_device.handle,
                surface_handle,
                &surface_format_count,
                surface_formats.as_ptr(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut found_format = false;
        for sf in surface_formats {
            if let api::Format::B8G8R8A8unorm = sf.format {
                if let api::ColorSpace::SrgbNonlinear = sf.color_space {
                    found_format = true;
                    break;
                }
            }
        }
        if !found_format {
            return Err(Error::NoUseableFormat);
        }
        //println!("Found Format!");

        let present_mode_count = 0;
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfacePresentModesKHR(
                physical_device.handle,
                surface_handle,
                &present_mode_count,
                ptr::null(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut present_modes = Vec::with_capacity(present_mode_count as usize);
        for _i in 0..present_mode_count {
            present_modes.push(SWAPCHAIN_PRESENT_MODE);
        }
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfacePresentModesKHR(
                physical_device.handle,
                surface_handle,
                &present_mode_count,
                present_modes.as_ptr(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut found_presentation = false;
        for pm in present_modes {
            if (SWAPCHAIN_PRESENT_MODE as u32) == (pm as u32) {
                found_presentation = true;
                break;
            }
        }
        if !found_presentation {
            return Err(Error::NoPresentationMode);
        }
        //println!("Found Presentation!");

        let queue_family_property_count = 0;
        unsafe {
            api::vkGetPhysicalDeviceQueueFamilyProperties2(
                physical_device.handle,
                &queue_family_property_count,
                ptr::null(),
            )
        };
        //println!("Queue Family Count: {}", queue_family_property_count);

        let count = queue_family_property_count as usize;
        let mut queue_family_properties = Vec::with_capacity(count);
        let mut queue_family_video_properties =
            Vec::with_capacity(queue_family_property_count as usize);
        for i in 0..count {
            queue_family_properties.push(api::QueueFamilyProperties2::default());
            queue_family_video_properties.push(api::QueueFamilyVideoProperties::default());
            queue_family_properties[i]
                .header
                .set_next_structure(
                    ptr::addr_of!(queue_family_video_properties[i]) as *const c_void
                );
        }
        unsafe {
            api::vkGetPhysicalDeviceQueueFamilyProperties2(
                physical_device.handle,
                &queue_family_property_count,
                queue_family_properties.as_ptr(),
            )
        };

        let mut queue_family_index = None;
        for (ind, qf) in queue_family_properties.iter().enumerate() {
            let presentation_support = BOOL_FALSE;
            let result = unsafe {
                api::vkGetPhysicalDeviceSurfaceSupportKHR(
                    physical_device.handle,
                    ind as u32,
                    surface_handle,
                    &presentation_support,
                )
            };
            if result != 0 {
                return Err(Error::VkResult(result));
            }

            let flags = qf.queue_flags;
            if presentation_support == BOOL_TRUE
                && ((flags & api::QueueFlagBit::Graphics as api::QueueFlags) > 0)
                && ((flags & api::QueueFlagBit::Transfer as api::QueueFlags) > 0)
            {
                queue_family_index = Some(ind as u32);
                break;
            }
        }
        let queue_family_index = match queue_family_index {
            Some(i) => i,
            None => return Err(Error::NoQueueFamily),
        };
        //println!("Queue Family Index: {}", queue_family_index);

        let queue_priority_data = [1.0]; // Account for safety
        let queue_create_info = api::DeviceQueueCreateInfo {
            header: StructureHeader::new(StructureType::DeviceQueueCreateInfo),
            flags: 0,
            queue_family_index,
            queue_count: 1,
            queue_priorities: queue_priority_data.as_ptr(),
        };

        let queue_create_infos = [queue_create_info];
        let extension_names = [
            DEVICE_EXTENSION_NAME_SYNCHRONIZATION, //Necessary?
            DEVICE_EXTENSION_NAME_SWAPCHAIN,
        ];
        let device = Device::new(physical_device, &queue_create_infos, &extension_names)?;

        let queue = device.get_queue_handle(queue_family_index)?;

        let surface_capabilities = api::SurfaceCapabilities::default();
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
                device.physical_device.handle,
                surface_handle,
                &surface_capabilities,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        if surface_capabilities.max_image_count < SWAPCHAIN_IMAGE_COUNT {
            return Err(Error::SwapchainImageCount);
        }
        if (surface_capabilities.supported_transforms
            & api::SurfaceTransformFlagBit::Identity as api::SurfaceTransformFlags)
            == 0
        {
            return Err(Error::BadTransform);
        }
        if (surface_capabilities.supported_usage_flags
            & api::ImageUsageFlagBit::TransferDst as api::ImageUsageFlags)
            == 0
        {
            return Err(Error::SurfaceNoTransfer);
        }
        println!(
            "Surface Width | Height: {} | {}",
            surface_capabilities.current_extent.width, surface_capabilities.current_extent.height
        );

        let swapchain_create_info = api::SwapchainCreateInfo {
            header: StructureHeader::new(StructureType::SwapchainCreateInfo),
            flags: 0,
            surface: surface_handle,
            min_image_count: SWAPCHAIN_IMAGE_COUNT,
            image_format: api::Format::B8G8R8A8unorm,
            image_color_space: api::ColorSpace::SrgbNonlinear,
            image_extent: api::Extent2d {
                width: surface_capabilities.current_extent.width,
                height: surface_capabilities.current_extent.height,
            },
            image_array_layers: 1,
            image_usage: (api::ImageUsageFlagBit::TransferDst as api::ImageUsageFlags)
                | (api::ImageUsageFlagBit::ColorAttachment as api::ImageUsageFlags),
            image_sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            pre_transform: surface_capabilities.current_transform,
            composite_alpha: api::CompositeAlphaFlagBit::Opaque as api::CompositeAlphaFlags,
            present_mode: SWAPCHAIN_PRESENT_MODE,
            clipped: BOOL_TRUE,
            old_swapchain: ptr::null(),
        };

        let handle = ptr::null();
        let result = unsafe {
            api::vkCreateSwapchainKHR(device.handle, &swapchain_create_info, ptr::null(), &handle)
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let swapchain_image_count = SWAPCHAIN_IMAGE_COUNT;
        let image_handles = [ptr::null(); SWAPCHAIN_IMAGE_COUNT as usize];
        let result = unsafe {
            api::vkGetSwapchainImagesKHR(
                device.handle,
                handle,
                &swapchain_image_count,
                image_handles.as_ptr(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        if swapchain_image_count != SWAPCHAIN_IMAGE_COUNT {
            return Err(Error::BadSwapchainImageCount);
        }

        // let image_memory_barrier = api::ImageMemoryBarrier2 {
        //     header: StructureHeader::new(StructureType::ImageMemoryBarrier2),
        //     src_access_mask: api::PipelineStageFlag2Bit::None as api::PipelineStageFlags2,
        //     src_stage_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
        //     dst_access_mask: api::PipelineStageFlag2Bit::None as api::PipelineStageFlags2,
        //     dst_stage_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
        //     old_layout: api::ImageLayout::Undefined,
        //     new_layout: api::ImageLayout::PresentSrc,
        //     src_queue_family_index: queue_family_index,
        //     dst_queue_family_index: queue_family_index,
        //     image: ptr::null(),
        //     subresource_range: api::ImageSubresourceRange {
        //         aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
        //         base_mip_level: 0,
        //         level_count: 1,
        //         base_array_layer: 0,
        //         layer_count: 1,
        //     },
        // };

        let command_pool_create_info = api::CommandPoolCreateInfo {
            header: StructureHeader::new(StructureType::CommandPoolCreateInfo),
            flags: api::CommandPoolCreateFlagBit::ResetCommandBufferBit
                as api::CommandPoolCreateFlags,
            queue_family_index,
        };
        let command_pool = ptr::null();
        let result = unsafe {
            api::vkCreateCommandPool(
                device.handle,
                &command_pool_create_info,
                ptr::null(),
                &command_pool,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let cmd_buffer_allocate_info = api::CommandBufferAllocateInfo {
            header: StructureHeader::new(StructureType::CommandBufferAllocateInfo),
            command_pool,
            command_buffer_level: api::CommandBufferLevel::Primary,
            command_buffer_count: SWAPCHAIN_IMAGE_COUNT,
        };

        let cmd_buffer_handles = [ptr::null(); SWAPCHAIN_IMAGE_COUNT as usize];
        let result = unsafe {
            api::vkAllocateCommandBuffers(
                device.handle,
                &cmd_buffer_allocate_info,
                cmd_buffer_handles.as_ptr(),
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let cmd_buffer_submit_infos = [
            api::CommandBufferSubmitInfo {
                header: StructureHeader::new(StructureType::CommandBufferSubmitInfo),
                command_buffer: cmd_buffer_handles[0],
                device_mask: 0,
            },
            api::CommandBufferSubmitInfo {
                header: StructureHeader::new(StructureType::CommandBufferSubmitInfo),
                command_buffer: cmd_buffer_handles[1],
                device_mask: 0,
            },
            api::CommandBufferSubmitInfo {
                header: StructureHeader::new(StructureType::CommandBufferSubmitInfo),
                command_buffer: cmd_buffer_handles[2],
                device_mask: 0,
            },
        ];

        let semaphore_create_info = api::SemaphoreCreateInfo {
            header: StructureHeader::new(StructureType::SemaphoreCreateInfo),
            flags: 0,
        };
        let wait_semaphore = ptr::null();
        let result = unsafe {
            api::vkCreateSemaphore(
                device.handle,
                &semaphore_create_info,
                ptr::null(),
                &wait_semaphore,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        let signal_semaphore = ptr::null();
        let result = unsafe {
            api::vkCreateSemaphore(
                device.handle,
                &semaphore_create_info,
                ptr::null(),
                &signal_semaphore,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        println!(
            "Wait | Signal Sempahores: {:?} | {:?}",
            wait_semaphore, signal_semaphore
        );

        let wait_semaphore_submit_info = api::SemaphoreSubmitInfo {
            header: StructureHeader::new(StructureType::SemaphoreSubmitInfo),
            semaphore: wait_semaphore,
            value: 0,
            stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            device_index: 0,
        };

        let signal_semaphore_submit_info = api::SemaphoreSubmitInfo {
            header: StructureHeader::new(StructureType::SemaphoreSubmitInfo),
            semaphore: signal_semaphore,
            value: 0,
            stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            device_index: 0,
        };

        let submit_info = api::SubmitInfo2 {
            header: StructureHeader::new(StructureType::SubmitInfo2),
            flags: api::SubmitFlagBit::None as api::SubmitFlags,
            wait_semaphore_info_count: 1,
            wait_semaphore_infos: ptr::null(),
            command_buffer_info_count: 1,
            command_buffer_infos: ptr::null(),
            signal_semaphore_info_count: 1,
            signal_semaphore_infos: ptr::null(),
        };

        let present_info = api::PresentInfo {
            header: StructureHeader::new(StructureType::PresentInfo),
            wait_semaphore_count: 1,
            wait_semaphores: ptr::null(),
            swapchain_count: 1,
            swapchains: ptr::null(),
            image_indicies: ptr::null(),
            results: ptr::null(),
        };

        Ok(Swapchain {
            present_info,
            submit_info,
            signal_semaphore_submit_info,
            wait_semaphore_submit_info,
            cmd_buffer_submit_infos,
            command_pool,
            //image_memory_barrier,
            image_handles,
            swapchain_create_info,
            handle,
            queue,
            device,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn new(
        physical_device: PhysicalDevice,
        surface_parameters: (
            windows::Win32::Foundation::HINSTANCE,
            windows::Win32::Foundation::HWND,
        ),
    ) -> Result<Self, Error> {
        let surface_create_info = api::SurfaceCreateInfoWin32 {
            header: StructureHeader::new(StructureType::SurfaceCreateInfoWin32),
            flags: 0,
            hinstance: surface_parameters.0,
            hwnd: surface_parameters.1,
        };

        let surface_handle = ptr::null();
        let result = unsafe {
            api::vkCreateWin32SurfaceKHR(
                physical_device.instance.handle,
                &surface_create_info,
                ptr::null(),
                &surface_handle,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        println!("Got Surface!");

        Swapchain::create(physical_device, surface_handle)
    }

    #[cfg(target_os = "macos")]
    pub fn new(
        physical_device: PhysicalDevice,
        surface_parameters: api::CAMetalLayerPtr,
    ) -> Result<Self, Error> {
        let surface_create_info = api::SurfaceCreateInfoMetal {
            header: StructureHeader::new(StructureType::SurfaceCreateInfoMetal),
            flags: 0,
            layer: surface_parameters,
        };

        let surface_handle = ptr::null();
        let result = unsafe {
            api::vkCreateMetalSurfaceEXT(
                physical_device.instance.handle,
                &surface_create_info,
                ptr::null(),
                &surface_handle,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        println!("Got Surface!");

        Swapchain::create(physical_device, surface_handle)
    }

    fn get_current_size(&self) -> Result<(u32, u32), Error> {
        let surface_capabilities = api::SurfaceCapabilities::default();
        let result = unsafe {
            api::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
                self.device.physical_device.handle,
                self.swapchain_create_info.surface,
                &surface_capabilities,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        Ok((
            surface_capabilities.current_extent.width,
            surface_capabilities.current_extent.height,
        ))
    }

    fn render_next_image(&mut self, fence: OpaqueHandle) -> Result<(), Error> {
        let timeout = 1000000000;
        let next_image_index = 0;

        let result = unsafe {
            api::vkAcquireNextImageKHR(
                self.device.handle,
                self.handle,
                timeout,
                self.wait_semaphore_submit_info.semaphore,
                ptr::null(),
                &next_image_index,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        //println!("Next Image Index: {}", next_image_index);

        self.submit_info.wait_semaphore_infos = &self.wait_semaphore_submit_info;
        self.submit_info.command_buffer_infos =
            &(self.cmd_buffer_submit_infos[next_image_index as usize]);
        self.submit_info.signal_semaphore_infos = &self.signal_semaphore_submit_info;
        let result = unsafe { api::vkQueueSubmit2(self.queue, 1, &self.submit_info, fence) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // let result = unsafe { api::vkQueueWaitIdle(self.queue) };
        // if result != 0 {
        //     return Err(Error::VkResult(result));
        // }
        // self.present_info.wait_semaphore_count = 0;

        self.present_info.wait_semaphores = &self.signal_semaphore_submit_info.semaphore;
        self.present_info.swapchains = &self.handle;
        self.present_info.image_indicies = &next_image_index;
        let result = unsafe { api::vkQueuePresentKHR(self.queue, &self.present_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        Ok(())
    }

    fn get_next_image_index(&self) -> Result<u32, Error> {
        let timeout = 1000000000;
        let next_image_index = 0;

        let result = unsafe {
            api::vkAcquireNextImageKHR(
                self.device.handle,
                self.handle,
                timeout,
                self.wait_semaphore_submit_info.semaphore,
                ptr::null(),
                &next_image_index,
            )
        };
        if result == 0 {
            Ok(next_image_index)
        } else {
            Err(Error::VkResult(result))
        }
    }

    fn submit_queue_and_present(
        &mut self,
        next_image_index: u32,
        fence: OpaqueHandle,
    ) -> Result<(), Error> {
        self.submit_info.wait_semaphore_infos = &self.wait_semaphore_submit_info;
        self.submit_info.command_buffer_infos =
            &(self.cmd_buffer_submit_infos[next_image_index as usize]);
        self.submit_info.signal_semaphore_infos = &self.signal_semaphore_submit_info;
        let result = unsafe { api::vkQueueSubmit2(self.queue, 1, &self.submit_info, fence) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        self.present_info.wait_semaphores = &self.signal_semaphore_submit_info.semaphore;
        self.present_info.swapchains = &self.handle;
        self.present_info.image_indicies = &next_image_index;
        let result = unsafe { api::vkQueuePresentKHR(self.queue, &self.present_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        Ok(())
    }
}

pub struct SwapchainCpuRender {
    img_buffer_mem: OpaqueHandle,
    img_buffer_check: OpaqueHandle,
    fence: OpaqueHandle,
    cpu_buffer_size: u64,
    cpu_buffer_img_mem: OpaqueHandle,
    cpu_buffer_image: OpaqueHandle,
    gpu_img_mem: OpaqueHandle,
    gpu_image: OpaqueHandle,
    swapchain: Swapchain,
}

impl SwapchainCpuRender {
    fn write_command_buffers(&mut self, width: u32, height: u32) -> Result<(), Error> {
        let cmd_buffer_begin_info = api::CommandBufferBeginInfo {
            header: StructureHeader::new(StructureType::CommandBufferBeginInfo),
            flags: api::CommandBufferUsageFlagBit::None as api::CommandBufferUsageFlags,
            inheritance_info: ptr::null(),
        };

        let buffer_image_copy = api::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: api::ImageSubresourceLayers {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: api::Offset3d::default(),
            image_extent: api::Extent3d {
                width,
                height,
                depth: 1,
            },
        };

        let mut dependency_info = api::DependencyInfo {
            header: StructureHeader::new(StructureType::DependencyInfo),
            dependency_flags: api::DependencyFlagBit::None as api::DependencyFlags,
            memory_barrier_count: 0,
            memory_barriers: ptr::null(),
            buffer_memory_barrier_count: 0,
            buffer_memory_barriers: ptr::null(),
            image_memory_barrier_count: 0,
            image_memory_barriers: ptr::null(),
        };

        let image_memory_barriers_1 = api::ImageMemoryBarrier2 {
            header: StructureHeader::new(StructureType::ImageMemoryBarrier2),
            src_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            src_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
            dst_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            dst_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
            old_layout: api::ImageLayout::Undefined,
            new_layout: api::ImageLayout::TransferDstOptimal,
            src_queue_family_index: self
                .swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
            dst_queue_family_index: self
                .swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
            image: self.gpu_image,
            subresource_range: api::ImageSubresourceRange {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        let mut image_memory_barriers_2 = [
            api::ImageMemoryBarrier2 {
                header: StructureHeader::new(StructureType::ImageMemoryBarrier2),
                src_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
                src_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
                dst_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
                dst_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
                old_layout: api::ImageLayout::TransferDstOptimal,
                new_layout: api::ImageLayout::TransferSrcOptimal,
                src_queue_family_index: self
                    .swapchain
                    .device
                    .physical_device
                    .local_only_memory_type_index,
                dst_queue_family_index: self
                    .swapchain
                    .device
                    .physical_device
                    .local_only_memory_type_index,
                image: self.gpu_image,
                subresource_range: api::ImageSubresourceRange {
                    aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            },
            api::ImageMemoryBarrier2 {
                header: StructureHeader::new(StructureType::ImageMemoryBarrier2),
                src_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
                src_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
                dst_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
                dst_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
                old_layout: api::ImageLayout::Undefined,
                new_layout: api::ImageLayout::TransferDstOptimal,
                src_queue_family_index: self
                    .swapchain
                    .device
                    .physical_device
                    .local_only_memory_type_index,
                dst_queue_family_index: self
                    .swapchain
                    .device
                    .physical_device
                    .local_only_memory_type_index,
                image: ptr::null(),
                subresource_range: api::ImageSubresourceRange {
                    aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            },
        ];

        let img_blit = api::ImageBlit2 {
            header: StructureHeader::new(StructureType::ImageBlit2),
            src_subresource: api::ImageSubresourceLayers {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offsets: [
                api::Offset3d::default(),
                api::Offset3d {
                    x: width as i32,
                    y: height as i32,
                    z: 1,
                },
            ],
            dst_subresource: api::ImageSubresourceLayers {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_offsets: [
                api::Offset3d::default(),
                api::Offset3d {
                    x: width as i32,
                    y: height as i32,
                    z: 1,
                },
            ],
        };
        let mut blit_info = api::BlitImageInfo2 {
            header: StructureHeader::new(StructureType::BlitImageInfo2),
            src_image: self.gpu_image,
            src_image_layout: api::ImageLayout::TransferSrcOptimal,
            dst_image: ptr::null(),
            dst_image_layout: api::ImageLayout::TransferDstOptimal,
            region_count: 1,
            regions: &img_blit,
            filter: api::Filter::Nearest,
        };

        let mut image_memory_barriers_3 = api::ImageMemoryBarrier2 {
            header: StructureHeader::new(StructureType::ImageMemoryBarrier2),
            src_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            src_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
            dst_stage_mask: api::PipelineStageFlag2Bit::AllCommands as api::PipelineStageFlags2,
            dst_access_mask: api::AccessFlag2Bit::None as api::AccessFlags2,
            old_layout: api::ImageLayout::TransferDstOptimal,
            new_layout: api::ImageLayout::PresentSrc,
            src_queue_family_index: self
                .swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
            dst_queue_family_index: self
                .swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
            image: ptr::null(),
            subresource_range: api::ImageSubresourceRange {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        for (ind, cmd_buffer) in self.swapchain.cmd_buffer_submit_infos.iter().enumerate() {
            let result = unsafe {
                api::vkBeginCommandBuffer(cmd_buffer.command_buffer, &cmd_buffer_begin_info)
            };
            if result != 0 {
                return Err(Error::VkResult(result));
            }

            dependency_info.image_memory_barrier_count = 1;
            dependency_info.image_memory_barriers = &image_memory_barriers_1;
            unsafe { api::vkCmdPipelineBarrier2(cmd_buffer.command_buffer, &dependency_info) };

            unsafe {
                api::vkCmdCopyBufferToImage(
                    cmd_buffer.command_buffer,
                    self.cpu_buffer_image,
                    self.gpu_image,
                    api::ImageLayout::TransferDstOptimal,
                    1,
                    &buffer_image_copy,
                )
            };

            dependency_info.image_memory_barrier_count = 2;
            dependency_info.image_memory_barriers = image_memory_barriers_2.as_ptr();
            image_memory_barriers_2[1].image = self.swapchain.image_handles[ind];
            unsafe { api::vkCmdPipelineBarrier2(cmd_buffer.command_buffer, &dependency_info) };

            blit_info.dst_image = self.swapchain.image_handles[ind];
            unsafe { api::vkCmdBlitImage2(cmd_buffer.command_buffer, &blit_info) };

            dependency_info.image_memory_barrier_count = 1;
            dependency_info.image_memory_barriers = &image_memory_barriers_3;
            image_memory_barriers_3.image = self.swapchain.image_handles[ind];
            unsafe { api::vkCmdPipelineBarrier2(cmd_buffer.command_buffer, &dependency_info) };

            unsafe {
                api::vkCmdCopyImageToBuffer(
                    cmd_buffer.command_buffer,
                    self.gpu_image,
                    api::ImageLayout::TransferSrcOptimal,
                    self.img_buffer_check,
                    1,
                    &buffer_image_copy,
                )
            };

            let result = unsafe { api::vkEndCommandBuffer(cmd_buffer.command_buffer) };
            if result != 0 {
                return Err(Error::VkResult(result));
            }
        }

        Ok(())
    }

    pub fn new(swapchain: Swapchain, width: u32, height: u32) -> Result<Self, Error> {
        let gpu_image_create_info = api::ImageCreateInfo {
            header: StructureHeader::new(StructureType::ImageCreateInfo),
            flags: api::ImageCreateFlagBit::None as api::ImageCreateFlags,
            image_type: api::ImageTypeDimensions::Two,
            format: api::Format::B8G8R8A8unorm,
            extent: api::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            tiling: api::ImageTiling::Optimal,
            usage: (api::ImageUsageFlagBit::TransferSrc as api::ImageUsageFlags)
                | (api::ImageUsageFlagBit::TransferDst as api::ImageUsageFlags),
            sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0, // Exclusive to zero here
            p_queue_family_indices: ptr::null(),
            initial_layout: api::ImageLayout::Undefined,
        };

        let gpu_image = ptr::null();
        let result = unsafe {
            api::vkCreateImage(
                swapchain.device.handle,
                &gpu_image_create_info,
                ptr::null(),
                &gpu_image,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let img_mem_reqs_info = api::ImageMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::ImageMemoryRequirementsInfo2),
            image: gpu_image,
        };
        let mut mem_reqs = api::MemoryRequirements2::default();
        let mem_dedicated_reqs = api::MemoryDedicatedRequirements::default();
        mem_reqs
            .header
            .set_next_structure(ptr::addr_of!(mem_dedicated_reqs) as *const c_void);
        unsafe {
            api::vkGetImageMemoryRequirements2(
                swapchain.device.handle,
                &img_mem_reqs_info,
                &mem_reqs,
            )
        };

        let mem_alloc_info = api::MemoryAllocateInfo {
            header: StructureHeader::new(StructureType::MemoryAllocateInfo),
            allocation_size: mem_reqs.size,
            memory_type_index: swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
        };
        let gpu_img_mem = ptr::null();
        let result = unsafe {
            api::vkAllocateMemory(
                swapchain.device.handle,
                &mem_alloc_info,
                ptr::null(),
                &gpu_img_mem,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let bind_img_mem_info = api::BindImageMemoryInfo {
            header: StructureHeader::new(StructureType::BindImageMemoryInfo),
            image: gpu_image,
            memory: gpu_img_mem,
            memory_offset: 0,
        };
        let result =
            unsafe { api::vkBindImageMemory2(swapchain.device.handle, 1, &bind_img_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Buffer Create
        let cpu_buffer_size = (width * height * 4) as u64;
        let buffer_create_info = api::BufferCreateInfo {
            header: StructureHeader::new(StructureType::BufferCreateInfo),
            flags: api::BufferCreateFlagBit::None as api::BufferCreateFlags,
            size: cpu_buffer_size,
            usage: api::BufferUsageFlagBit::TransferSrc as api::BufferUsageFlags,
            sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0, // Exclusive to zero here
            p_queue_family_indices: ptr::null(),
        };

        let cpu_buffer_image = ptr::null();
        let result = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &buffer_create_info,
                ptr::null(),
                &cpu_buffer_image,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: cpu_buffer_image,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };

        let mem_alloc_info = api::MemoryAllocateInfo {
            header: StructureHeader::new(StructureType::MemoryAllocateInfo),
            allocation_size: mem_reqs.size,
            memory_type_index: swapchain
                .device
                .physical_device
                .basic_cpu_access_memory_type_index,
        };
        let cpu_buffer_img_mem = ptr::null();
        let result = unsafe {
            api::vkAllocateMemory(
                swapchain.device.handle,
                &mem_alloc_info,
                ptr::null(),
                &cpu_buffer_img_mem,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let bind_buf_mem_info = api::BindBufferMemoryInfo {
            header: StructureHeader::new(StructureType::BindBufferMemoryInfo),
            buffer: cpu_buffer_image,
            memory: cpu_buffer_img_mem,
            memory_offset: 0,
        };
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Fence Create
        let fence_create_info = api::FenceCreateInfo {
            header: StructureHeader::new(StructureType::FenceCreateInfo),
            flags: 0,
        };
        let fence = ptr::null();
        let result = unsafe {
            api::vkCreateFence(
                swapchain.device.handle,
                &fence_create_info,
                ptr::null(),
                &fence,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Buffer Verification Create
        let buffer_create_info = api::BufferCreateInfo {
            header: StructureHeader::new(StructureType::BufferCreateInfo),
            flags: api::BufferCreateFlagBit::None as api::BufferCreateFlags,
            size: cpu_buffer_size,
            usage: api::BufferUsageFlagBit::TransferDst as api::BufferUsageFlags,
            sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0, // Exclusive to zero here
            p_queue_family_indices: ptr::null(),
        };

        let img_buffer_check = ptr::null();
        let result = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &buffer_create_info,
                ptr::null(),
                &img_buffer_check,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: img_buffer_check,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };

        let mem_alloc_info = api::MemoryAllocateInfo {
            header: StructureHeader::new(StructureType::MemoryAllocateInfo),
            allocation_size: mem_reqs.size,
            memory_type_index: swapchain
                .device
                .physical_device
                .basic_cpu_access_memory_type_index,
        };
        let img_buffer_mem = ptr::null();
        let result = unsafe {
            api::vkAllocateMemory(
                swapchain.device.handle,
                &mem_alloc_info,
                ptr::null(),
                &img_buffer_mem,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let bind_buf_mem_info = api::BindBufferMemoryInfo {
            header: StructureHeader::new(StructureType::BindBufferMemoryInfo),
            buffer: img_buffer_check,
            memory: img_buffer_mem,
            memory_offset: 0,
        };
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut scr = SwapchainCpuRender {
            img_buffer_mem,
            img_buffer_check,
            fence,
            cpu_buffer_size,
            cpu_buffer_img_mem,
            cpu_buffer_image,
            gpu_img_mem,
            gpu_image,
            swapchain,
        };
        scr.write_command_buffers(width, height)?;
        scr.swapchain.render_next_image(scr.fence)?; // Render once for a test and to allow the fence to be signalled

        Ok(scr)
    }

    pub fn get_buffer(&mut self) -> Result<&mut [u32], Error> {
        let result = unsafe {
            api::vkWaitForFences(
                self.swapchain.device.handle,
                1,
                &self.fence,
                BOOL_FALSE,
                100000000, // 100 ms in nanoseconds
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        let result = unsafe { api::vkResetFences(self.swapchain.device.handle, 1, &self.fence) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let data_ptr = ptr::null_mut();
        let result = unsafe {
            api::vkMapMemory(
                self.swapchain.device.handle,
                self.cpu_buffer_img_mem,
                0,
                self.cpu_buffer_size,
                api::MemoryMapFlagBit::None as api::MemoryMapFlags,
                &data_ptr,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        if !data_ptr.is_null() {
            Ok(unsafe {
                std::slice::from_raw_parts_mut(
                    data_ptr as *mut u32,
                    (self.cpu_buffer_size >> 2) as usize,
                )
            })
        } else {
            Err(Error::InvalidMapPtr)
        }
    }

    pub fn render(&mut self) -> Result<(), Error> {
        unsafe { vkUnmapMemory(self.swapchain.device.handle, self.cpu_buffer_img_mem) };
        //std::thread::sleep(std::time::Duration::from_millis(100));

        self.swapchain.render_next_image(self.fence)?;
        Ok(())
    }

    pub fn buffer_check(&mut self, width: u32, height: u32) -> Result<(), Error> {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let data_ptr = ptr::null_mut();
        let result = unsafe {
            api::vkMapMemory(
                self.swapchain.device.handle,
                self.img_buffer_mem,
                0,
                self.cpu_buffer_size,
                api::MemoryMapFlagBit::None as api::MemoryMapFlags,
                &data_ptr,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        if data_ptr.is_null() {
            return Err(Error::InvalidMapPtr);
        }
        let data_check = unsafe {
            std::slice::from_raw_parts(data_ptr as *const u32, (self.cpu_buffer_size >> 2) as usize)
        };
        for h in 0..height as usize {
            let d = data_check[h * width as usize];
            println!("{} @: {}", d, h);
        }

        unsafe { vkUnmapMemory(self.swapchain.device.handle, self.img_buffer_mem) };
        Ok(())
    }
}

const MAIN_DATA: [i8; 5] = [109, 97, 105, 110, 0];

fn create_shader_stage_from_bytes(
    device: OpaqueHandle,
    data: &[u8],
    stage_bit: api::ShaderStageFlagBit,
) -> Result<api::PipelineShaderStageCreateInfo, Error> {
    //let code_size = data.len() & !0x3;
    let num_words = data.len() >> 2;
    let code_size = num_words * 4;
    let mut code_data = Vec::with_capacity(num_words);
    for ind in (0..code_size).step_by(4) {
        code_data.push(u32::from_ne_bytes([
            data[ind],
            data[ind + 1],
            data[ind + 2],
            data[ind + 3],
        ]))
    }

    let shader_module_create_info = api::ShaderModuleCreateInfo {
        header: StructureHeader::new(StructureType::ShaderModuleCreateInfo),
        flags: 0,
        code_size,
        code: code_data.as_ptr(),
    };

    let shader_module = ptr::null();
    let result = unsafe {
        api::vkCreateShaderModule(
            device,
            &shader_module_create_info,
            ptr::null(),
            &shader_module,
        )
    };
    if result != 0 {
        return Err(Error::VkResult(result));
    }

    Ok(api::PipelineShaderStageCreateInfo {
        header: StructureHeader::new(StructureType::PipelineShaderStageCreateInfo),
        flags: 0,
        stage: stage_bit as api::ShaderStageFlags,
        module: shader_module,
        name: MAIN_DATA.as_ptr(),
        specialization_info: ptr::null(),
    })
}

const BASIC_VERTEX_SHADER_DATA: [u8; 824] = [
    0x03, 0x02, 0x23, 0x07, 0x00, 0x06, 0x01, 0x00, 0x0b, 0x00, 0x08, 0x00, 0x1b, 0x00, 0x00,
    0x00, // ..#.............
    0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x06,
    0x00, // ................
    0x01, 0x00, 0x00, 0x00, 0x47, 0x4c, 0x53, 0x4c, 0x2e, 0x73, 0x74, 0x64, 0x2e, 0x34, 0x35,
    0x30, // ....GLSL.std.450
    0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // ................
    0x0f, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x6d, 0x61, 0x69,
    0x6e, // ............main
    0x00, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03,
    0x00, // ................
    0x02, 0x00, 0x00, 0x00, 0xc2, 0x01, 0x00, 0x00, 0x05, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00,
    0x00, // ................
    0x6d, 0x61, 0x69, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x06, 0x00, 0x0b, 0x00, 0x00,
    0x00, // main............
    0x67, 0x6c, 0x5f, 0x50, 0x65, 0x72, 0x56, 0x65, 0x72, 0x74, 0x65, 0x78, 0x00, 0x00, 0x00,
    0x00, // gl_PerVertex....
    0x06, 0x00, 0x06, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x67, 0x6c, 0x5f,
    0x50, // ............gl_P
    0x6f, 0x73, 0x69, 0x74, 0x69, 0x6f, 0x6e, 0x00, 0x06, 0x00, 0x07, 0x00, 0x0b, 0x00, 0x00,
    0x00, // osition.........
    0x01, 0x00, 0x00, 0x00, 0x67, 0x6c, 0x5f, 0x50, 0x6f, 0x69, 0x6e, 0x74, 0x53, 0x69, 0x7a,
    0x65, // ....gl_PointSize
    0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x07, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    0x00, // ................
    0x67, 0x6c, 0x5f, 0x43, 0x6c, 0x69, 0x70, 0x44, 0x69, 0x73, 0x74, 0x61, 0x6e, 0x63, 0x65,
    0x00, // gl_ClipDistance.
    0x06, 0x00, 0x07, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x67, 0x6c, 0x5f,
    0x43, // ............gl_C
    0x75, 0x6c, 0x6c, 0x44, 0x69, 0x73, 0x74, 0x61, 0x6e, 0x63, 0x65, 0x00, 0x05, 0x00, 0x03,
    0x00, // ullDistance.....
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x05, 0x00, 0x12, 0x00, 0x00,
    0x00, // ................
    0x69, 0x6e, 0x50, 0x6f, 0x73, 0x69, 0x74, 0x69, 0x6f, 0x6e, 0x00, 0x00, 0x48, 0x00, 0x05,
    0x00, // inPosition..H...
    0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // ................
    0x48, 0x00, 0x05, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00,
    0x00, // H...............
    0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x05, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    0x00, // ....H...........
    0x0b, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x48, 0x00, 0x05, 0x00, 0x0b, 0x00, 0x00,
    0x00, // ........H.......
    0x03, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x47, 0x00, 0x03,
    0x00, // ............G...
    0x0b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x47, 0x00, 0x04, 0x00, 0x12, 0x00, 0x00,
    0x00, // ........G.......
    0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00,
    0x00, // ................
    0x21, 0x00, 0x03, 0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x16, 0x00, 0x03,
    0x00, // !...............
    0x06, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x17, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00,
    0x00, // .... ...........
    0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x15, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00,
    0x00, // ................
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2b, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00,
    0x00, //  .......+.......
    0x09, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x04, 0x00, 0x0a, 0x00, 0x00,
    0x00, // ................
    0x06, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x06, 0x00, 0x0b, 0x00, 0x00,
    0x00, // ................
    0x07, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00,
    0x00, // ................
    0x20, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00,
    0x00, //  ...............
    0x3b, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
    0x00, // ;...............
    0x15, 0x00, 0x04, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // ........ .......
    0x2b, 0x00, 0x04, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // +...............
    0x17, 0x00, 0x04, 0x00, 0x10, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    0x00, // ................
    0x20, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00,
    0x00, //  ...............
    0x3b, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // ;...............
    0x2b, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // +...............
    0x2b, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    0x3f, // +..............?
    0x20, 0x00, 0x04, 0x00, 0x19, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00,
    0x00, //  ...............
    0x36, 0x00, 0x05, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 6...............
    0x03, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x02, 0x00, 0x05, 0x00, 0x00, 0x00, 0x3d, 0x00, 0x04,
    0x00, // ............=...
    0x10, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x51, 0x00, 0x05,
    0x00, // ............Q...
    0x06, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // ................
    0x51, 0x00, 0x05, 0x00, 0x06, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00,
    0x00, // Q...............
    0x01, 0x00, 0x00, 0x00, 0x50, 0x00, 0x07, 0x00, 0x07, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00,
    0x00, // ....P...........
    0x16, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x15, 0x00, 0x00,
    0x00, // ................
    0x41, 0x00, 0x05, 0x00, 0x19, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00,
    0x00, // A...............
    0x0f, 0x00, 0x00, 0x00, 0x3e, 0x00, 0x03, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00,
    0x00, // ....>...........
    0xfd, 0x00, 0x01, 0x00, 0x38, 0x00, 0x01, 0x00, // ....8...
];

const BASIC_FRAGMENT_SHADER_DATA: [u8; 336] = [
    0x03, 0x02, 0x23, 0x07, 0x00, 0x06, 0x01, 0x00, 0x0b, 0x00, 0x08, 0x00, 0x0c, 0x00, 0x00,
    0x00, // ..#.............
    0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x06,
    0x00, // ................
    0x01, 0x00, 0x00, 0x00, 0x47, 0x4c, 0x53, 0x4c, 0x2e, 0x73, 0x74, 0x64, 0x2e, 0x34, 0x35,
    0x30, // ....GLSL.std.450
    0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // ................
    0x0f, 0x00, 0x06, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x6d, 0x61, 0x69,
    0x6e, // ............main
    0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x10, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00,
    0x00, // ................
    0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x00, 0x02, 0x00, 0x00, 0x00, 0xc2, 0x01, 0x00,
    0x00, // ................
    0x05, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00, 0x00, 0x6d, 0x61, 0x69, 0x6e, 0x00, 0x00, 0x00,
    0x00, // ........main....
    0x05, 0x00, 0x05, 0x00, 0x09, 0x00, 0x00, 0x00, 0x6f, 0x75, 0x74, 0x43, 0x6f, 0x6c, 0x6f,
    0x72, // ........outColor
    0x00, 0x00, 0x00, 0x00, 0x47, 0x00, 0x04, 0x00, 0x09, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x00,
    0x00, // ....G...........
    0x00, 0x00, 0x00, 0x00, 0x13, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x21, 0x00, 0x03,
    0x00, // ............!...
    0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x16, 0x00, 0x03, 0x00, 0x06, 0x00, 0x00,
    0x00, // ................
    0x20, 0x00, 0x00, 0x00, 0x17, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00,
    0x00, //  ...............
    0x04, 0x00, 0x00, 0x00, 0x20, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
    0x00, // .... ...........
    0x07, 0x00, 0x00, 0x00, 0x3b, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00,
    0x00, // ....;...........
    0x03, 0x00, 0x00, 0x00, 0x2b, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00,
    0x00, // ....+...........
    0x00, 0x00, 0x80, 0x3f, 0x2c, 0x00, 0x07, 0x00, 0x07, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00,
    0x00, // ...?,...........
    0x0a, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00,
    0x00, // ................
    0x36, 0x00, 0x05, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 6...............
    0x03, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x02, 0x00, 0x05, 0x00, 0x00, 0x00, 0x3e, 0x00, 0x03,
    0x00, // ............>...
    0x09, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0xfd, 0x00, 0x01, 0x00, 0x38, 0x00, 0x01,
    0x00, // ............8...
];

#[repr(C)]
pub struct TriangleVertex {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct TriangleIndicies {
    //Maybe add color in future
    pub p0: u16,
    pub p1: u16,
    pub p2: u16,
}

pub struct SwapchainTriangleRender {
    fence: OpaqueHandle,
    graphics_pipeline: OpaqueHandle,
    pipeline_layout: OpaqueHandle,
    descriptor_set_layout: OpaqueHandle,
    shader_stages: [api::PipelineShaderStageCreateInfo; 2],
    gpu_triangle_buffer_mem: OpaqueHandle,
    gpu_index_buffer: OpaqueHandle,
    gpu_vertex_buffer: OpaqueHandle,
    cpu_triangle_buffer_mem: OpaqueHandle,
    cpu_triangle_buffer_mem_size: u64,
    cpu_index_buffer: OpaqueHandle,
    cpu_vertex_buffer: OpaqueHandle,
    max_triangles: u32,
    framebuffers: [OpaqueHandle; SWAPCHAIN_IMAGE_COUNT as usize],
    swapchain_image_views: [OpaqueHandle; SWAPCHAIN_IMAGE_COUNT as usize],
    render_pass: OpaqueHandle,
    swapchain: Swapchain,
}

impl SwapchainTriangleRender {
    fn write_initial_command_buffers(&mut self, width: u32, height: u32) -> Result<(), Error> {
        let cmd_buffer_begin_info = api::CommandBufferBeginInfo {
            header: StructureHeader::new(StructureType::CommandBufferBeginInfo),
            flags: api::CommandBufferUsageFlagBit::None as api::CommandBufferUsageFlags,
            inheritance_info: ptr::null(),
        };

        let clear_value = api::ClearValue {
            color: api::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };

        let mut render_pass_begin_info = api::RenderPassBeginInfo {
            header: StructureHeader::new(StructureType::RenderPassBeginInfo),
            render_pass: self.render_pass,
            framebuffer: ptr::null(),
            render_area: api::Rect2D {
                offset: api::Offset2d::default(),
                extent: api::Extent2d { width, height },
            },
            clear_value_count: 1,
            clear_values: &clear_value,
        };

        let vertex_buffers = [self.gpu_vertex_buffer];
        let vertex_offsets = [0];

        for (ind, cmd_buffer) in self.swapchain.cmd_buffer_submit_infos.iter().enumerate() {
            let result = unsafe {
                api::vkBeginCommandBuffer(cmd_buffer.command_buffer, &cmd_buffer_begin_info)
            };
            if result != 0 {
                return Err(Error::VkResult(result));
            }

            render_pass_begin_info.framebuffer = self.framebuffers[ind];
            unsafe {
                api::vkCmdBeginRenderPass(
                    cmd_buffer.command_buffer,
                    &render_pass_begin_info,
                    api::SubpassContents::Inline,
                )
            };

            unsafe {
                api::vkCmdBindPipeline(
                    cmd_buffer.command_buffer,
                    api::PipelineBindPoint::Graphics,
                    self.graphics_pipeline,
                )
            };
            unsafe {
                api::vkCmdBindVertexBuffers(
                    cmd_buffer.command_buffer,
                    0,
                    1,
                    vertex_buffers.as_ptr(),
                    vertex_offsets.as_ptr(),
                )
            };
            unsafe {
                api::vkCmdBindIndexBuffer(
                    cmd_buffer.command_buffer,
                    self.gpu_index_buffer,
                    0,
                    api::IndexType::Uint16,
                )
            };
            unsafe { api::vkCmdDrawIndexed(cmd_buffer.command_buffer, 0, 0, 0, 0, 0) };

            unsafe { api::vkCmdEndRenderPass(cmd_buffer.command_buffer) };

            let result = unsafe { api::vkEndCommandBuffer(cmd_buffer.command_buffer) };
            if result != 0 {
                return Err(Error::VkResult(result));
            }
        }

        Ok(())
    }

    pub fn new(swapchain: Swapchain, max_triangles: u32) -> Result<Self, Error> {
        let (swapchain_width, swapchain_height) = swapchain.get_current_size()?;
        let swapchain_format = swapchain.swapchain_create_info.image_format;

        // Create Renderpass:
        let color_attachment_description = api::AttachmentDescription2 {
            header: StructureHeader::new(StructureType::AttachmentDescription2),
            flags: 0,
            format: swapchain_format,
            samples: 1,
            load_op: api::AttachmentLoadOp::Clear,
            store_op: api::AttachmentStoreOp::Store,
            stencil_load_op: api::AttachmentLoadOp::DontCare,
            stencil_store_op: api::AttachmentStoreOp::DontCare,
            initial_layout: api::ImageLayout::Undefined,
            final_layout: api::ImageLayout::PresentSrc,
        };

        let color_attachment_reference = api::AttachmentReference2 {
            header: StructureHeader::new(StructureType::AttachmentReference2),
            attachment: 0,
            layout: api::ImageLayout::ColorAttachmentOptimal,
            aspect_mask: 0,
        };

        let subpass = api::SubpassDescription2 {
            header: StructureHeader::new(StructureType::SubpassDescription2),
            flags: 0,
            pipeline_bind_point: api::PipelineBindPoint::Graphics,
            view_mask: 0,
            input_attachment_count: 0,
            input_attachments: ptr::null(),
            color_attachment_count: 1,
            color_attachments: &color_attachment_reference,
            resolve_attachments: ptr::null(),
            depth_stencil_attachment: ptr::null(),
            preserve_attachment_count: 0,
            preserve_attachments: ptr::null(),
        };

        let dependency = api::SubpassDependency2 {
            header: StructureHeader::new(StructureType::SubpassDependency2),
            src_subpass: api::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: api::PipelineStageFlagBit::ColorAttachmentOutput
                as api::PipelineStageFlags, // | api::PipelineStageFlagBit::EarlyFragmentTests,
            dst_stage_mask: api::PipelineStageFlagBit::ColorAttachmentOutput
                as api::PipelineStageFlags, // | api::PipelineStageFlagBit::EarlyFragmentTests,
            src_access_mask: 0,
            dst_access_mask: api::AccessFlagBit::ColorAttachmentWrite as api::AccessFlags,
            dependency_flags: 0, // Not sure
            view_offset: 0,      // Not sure
        };

        let render_pass_create_info = api::RenderPassCreateInfo2 {
            header: StructureHeader::new(StructureType::RenderPassCreateInfo2),
            flags: 0,
            attachment_count: 1,
            attachments: &color_attachment_description,
            subpass_count: 1,
            subpasses: &subpass,
            dependency_count: 1,
            dependencies: &dependency,
            correlated_view_mask_count: 0,
            correlated_view_masks: ptr::null(),
        };

        let render_pass = ptr::null();
        let result = unsafe {
            api::vkCreateRenderPass2(
                swapchain.device.handle,
                &render_pass_create_info,
                ptr::null(),
                &render_pass,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Create Swapchain ImageViews and Corresponding Framebuffers
        let swapchain_image_views = [ptr::null(); SWAPCHAIN_IMAGE_COUNT as usize];
        let mut image_view_create_info = api::ImageViewCreateInfo {
            header: StructureHeader::new(StructureType::ImageViewCreateInfo),
            flags: 0,
            image: ptr::null(),
            view_type: api::ImageViewType::TwoDimensions,
            format: swapchain_format,
            components: api::ComponentMapping {
                r: api::ComponentSwizzle::Identity,
                g: api::ComponentSwizzle::Identity,
                b: api::ComponentSwizzle::Identity,
                a: api::ComponentSwizzle::Identity,
            },
            subresource_range: api::ImageSubresourceRange {
                aspect_mask: api::ImageAspectFlagBit::Color as api::ImageAspectFlags,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        for (ind, iv) in swapchain_image_views.iter().enumerate() {
            image_view_create_info.image = swapchain.image_handles[ind];
            let result = unsafe {
                api::vkCreateImageView(
                    swapchain.device.handle,
                    &image_view_create_info,
                    ptr::null(),
                    iv,
                )
            };
            if result != 0 {
                return Err(Error::VkResult(result));
            }
        }

        let framebuffers: [*const _; 3] = [ptr::null(); SWAPCHAIN_IMAGE_COUNT as usize];
        let mut framebuffer_create_info = api::FramebufferCreateInfo {
            header: StructureHeader::new(StructureType::FramebufferCreateInfo),
            flags: 0,
            render_pass,
            attachment_count: 1,
            attachments: ptr::null(),
            width: swapchain_width,
            height: swapchain_height,
            layers: 1,
        };
        for (ind, fb) in framebuffers.iter().enumerate() {
            framebuffer_create_info.attachments = &swapchain_image_views[ind];
            let result = unsafe {
                api::vkCreateFramebuffer(
                    swapchain.device.handle,
                    &framebuffer_create_info,
                    ptr::null(),
                    fb,
                )
            };
            if result != 0 {
                return Err(Error::VkResult(result));
            }
        }

        // Vertex Buffer and Index Buffer Create
        let vertex_buffer_size = (mem::size_of::<TriangleVertex>() * (1 << 16)) as u64;
        let mut vertex_buffer_create_info = api::BufferCreateInfo {
            header: StructureHeader::new(StructureType::BufferCreateInfo),
            flags: api::BufferCreateFlagBit::None as api::BufferCreateFlags,
            size: vertex_buffer_size,
            usage: api::BufferUsageFlagBit::TransferSrc as api::BufferUsageFlags,
            sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0, // Exclusive to zero here
            p_queue_family_indices: ptr::null(),
        };

        let cpu_vertex_buffer = ptr::null();
        let result: i32 = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &vertex_buffer_create_info,
                ptr::null(),
                &cpu_vertex_buffer,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let index_buffer_size = (mem::size_of::<TriangleIndicies>() as u64) * max_triangles as u64;
        let mut index_buffer_create_info = api::BufferCreateInfo {
            header: StructureHeader::new(StructureType::BufferCreateInfo),
            flags: api::BufferCreateFlagBit::None as api::BufferCreateFlags,
            size: index_buffer_size,
            usage: api::BufferUsageFlagBit::TransferSrc as api::BufferUsageFlags,
            sharing_mode: api::SharingMode::Exclusive,
            queue_family_index_count: 0, // Exclusive to zero here
            p_queue_family_indices: ptr::null(),
        };

        let cpu_index_buffer = ptr::null();
        let result: i32 = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &index_buffer_create_info,
                ptr::null(),
                &cpu_index_buffer,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mem_reqs = api::MemoryRequirements2::default();
        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: cpu_vertex_buffer,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };
        let vertex_buffer_mem_reqs_size = mem_reqs.size;

        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: cpu_index_buffer,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };
        let index_buffer_mem_reqs_size = mem_reqs.size;

        let cpu_triangle_buffer_mem_size = vertex_buffer_mem_reqs_size + index_buffer_mem_reqs_size;
        let mem_alloc_info = api::MemoryAllocateInfo {
            header: StructureHeader::new(StructureType::MemoryAllocateInfo),
            allocation_size: cpu_triangle_buffer_mem_size,
            memory_type_index: swapchain
                .device
                .physical_device
                .basic_cpu_access_memory_type_index,
        };
        let cpu_triangle_buffer_mem = ptr::null();
        let result = unsafe {
            api::vkAllocateMemory(
                swapchain.device.handle,
                &mem_alloc_info,
                ptr::null(),
                &cpu_triangle_buffer_mem,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut bind_buf_mem_info = api::BindBufferMemoryInfo {
            header: StructureHeader::new(StructureType::BindBufferMemoryInfo),
            buffer: cpu_vertex_buffer,
            memory: cpu_triangle_buffer_mem,
            memory_offset: 0,
        };
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        bind_buf_mem_info.buffer = cpu_index_buffer;
        bind_buf_mem_info.memory_offset = vertex_buffer_size;
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        vertex_buffer_create_info.usage = (api::BufferUsageFlagBit::TransferDst
            as api::BufferUsageFlags)
            | (api::BufferUsageFlagBit::VertexBuffer as api::BufferUsageFlags);
        let gpu_vertex_buffer = ptr::null();
        let result: i32 = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &vertex_buffer_create_info,
                ptr::null(),
                &gpu_vertex_buffer,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        index_buffer_create_info.usage = (api::BufferUsageFlagBit::TransferDst
            as api::BufferUsageFlags)
            | (api::BufferUsageFlagBit::IndexBuffer as api::BufferUsageFlags);
        let gpu_index_buffer = ptr::null();
        let result: i32 = unsafe {
            api::vkCreateBuffer(
                swapchain.device.handle,
                &index_buffer_create_info,
                ptr::null(),
                &gpu_index_buffer,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: gpu_vertex_buffer,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };
        let vertex_buffer_mem_reqs_size = mem_reqs.size;

        let buf_mem_reqs_info = api::BufferMemoryRequirementsInfo2 {
            header: StructureHeader::new(StructureType::BufferMemoryRequirementsInfo2),
            buffer: gpu_index_buffer,
        };
        unsafe {
            api::vkGetBufferMemoryRequirements2(
                swapchain.device.handle,
                &buf_mem_reqs_info,
                &mem_reqs,
            )
        };
        let index_buffer_mem_reqs_size = mem_reqs.size;

        let mem_alloc_info = api::MemoryAllocateInfo {
            header: StructureHeader::new(StructureType::MemoryAllocateInfo),
            allocation_size: vertex_buffer_mem_reqs_size + index_buffer_mem_reqs_size,
            memory_type_index: swapchain
                .device
                .physical_device
                .local_only_memory_type_index,
        };
        let gpu_triangle_buffer_mem = ptr::null();
        let result = unsafe {
            api::vkAllocateMemory(
                swapchain.device.handle,
                &mem_alloc_info,
                ptr::null(),
                &gpu_triangle_buffer_mem,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut bind_buf_mem_info = api::BindBufferMemoryInfo {
            header: StructureHeader::new(StructureType::BindBufferMemoryInfo),
            buffer: gpu_vertex_buffer,
            memory: gpu_triangle_buffer_mem,
            memory_offset: 0,
        };
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        bind_buf_mem_info.buffer = gpu_index_buffer;
        bind_buf_mem_info.memory_offset = vertex_buffer_size;
        let result =
            unsafe { api::vkBindBufferMemory2(swapchain.device.handle, 1, &bind_buf_mem_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Shader Stage Create
        let vertex_shader_stage = create_shader_stage_from_bytes(
            swapchain.device.handle,
            &BASIC_VERTEX_SHADER_DATA,
            api::ShaderStageFlagBit::Vertex,
        )?;
        let fragment_shader_stage = create_shader_stage_from_bytes(
            swapchain.device.handle,
            &BASIC_FRAGMENT_SHADER_DATA,
            api::ShaderStageFlagBit::Fragment,
        )?;
        let shader_stages = [vertex_shader_stage, fragment_shader_stage];

        // Create Pipeline Layout
        let descriptor_set_layout_binding = api::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: api::DescriptorType::UniformBuffer,
            descriptor_count: 1,
            stage_flags: api::ShaderStageFlagBit::Vertex as api::ShaderStageFlags,
            immutable_samplers: ptr::null(),
        };

        let descriptor_set_layout_create_info = api::DescriptorSetLayoutCreateInfo {
            header: StructureHeader::new(StructureType::DescriptorSetLayoutCreateInfo),
            flags: 0,
            binding_count: 1,
            bindings: &descriptor_set_layout_binding,
        };

        let descriptor_set_layout = ptr::null();
        let result = unsafe {
            api::vkCreateDescriptorSetLayout(
                swapchain.device.handle,
                &descriptor_set_layout_create_info,
                ptr::null(),
                &descriptor_set_layout,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let pipeline_layout_create_info = api::PipelineLayoutCreateInfo {
            header: StructureHeader::new(StructureType::PipelineLayoutCreateInfo),
            flags: 0,
            set_layout_count: 1,
            set_layouts: &descriptor_set_layout,
            push_constant_range_count: 0,
            push_constant_ranges: ptr::null(),
        };

        let pipeline_layout = ptr::null();
        let result = unsafe {
            api::vkCreatePipelineLayout(
                swapchain.device.handle,
                &pipeline_layout_create_info,
                ptr::null(),
                &pipeline_layout,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Create Graphics Pipeline
        let vertex_input_binding_description = api::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<TriangleVertex>() as u32,
            input_rate: api::VertexInputRate::Vertex,
        };
        let vertex_input_attribute_description = api::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: api::Format::R32G32sfloat,
            offset: 0,
        };
        let vertex_input_create_info = api::PipelineVertexInputStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineVertexInputStateCreateInfo),
            flags: 0,
            vertex_binding_description_count: 1,
            vertex_binding_descriptions: &vertex_input_binding_description,
            vertex_attribute_description_count: 1,
            vertex_attribute_descriptions: &vertex_input_attribute_description,
        };

        let input_assembly_create_info = api::PipelineInputAssemblyStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineInputAssemblyStateCreateInfo),
            flags: 0,
            topology: api::PrimitiveTopology::TriangleList,
            primitive_restart_enable: BOOL_FALSE,
        };

        let tessilation_create_info = api::PipelineTessellationStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineTessellationStateCreateInfo),
            flags: 0,
            patch_control_points: 0, // Not sure but probably valid
        };

        let viewport = api::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain_width as f32,
            height: swapchain_height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = api::Rect2D {
            offset: api::Offset2d::default(),
            extent: api::Extent2d {
                width: swapchain_width,
                height: swapchain_height,
            },
        };
        let viewport_create_info = api::PipelineViewportStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineViewportStateCreateInfo),
            flags: 0,
            viewport_count: 1,
            viewports: &viewport,
            scissor_count: 1,
            scissors: &scissor,
        };

        let rasterization_create_info = api::PipelineRasterizationStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineRasterizationStateCreateInfo),
            flags: 0,
            depth_clamp_enable: BOOL_FALSE,
            rasterizer_discard_enable: BOOL_FALSE,
            polygon_mode: api::PolygonMode::Fill,
            cull_mode: api::CullModeFlagBit::None as api::CullModeFlags,
            front_face: api::FrontFace::Clockwise,
            depth_bias_enable: BOOL_FALSE,
            depth_bias_constant_factor: 0.0, // Not used when depth_bias_enable is false...?
            depth_bias_clamp: 0.0,           // Not used when depth_bias_enable is false...?
            depth_bias_slope_factor: 0.0,    // Not used when depth_bias_enable is false...?
            line_width: 1.0,
        };

        let multisampling_create_info = api::PipelineMultisampleStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineMultisampleStateCreateInfo),
            flags: 0,
            rasterization_samples: 1,
            sample_shading_enable: BOOL_FALSE,
            min_sample_shading: 1.0,
            sample_mask: ptr::null(),
            alpha_to_coverage_enable: BOOL_FALSE,
            alpha_to_one_enable: BOOL_FALSE,
        };

        let depth_stencil_create_info = api::PipelineDepthStencilStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineDepthStencilStateCreateInfo),
            flags: 0,
            depth_test_enable: BOOL_FALSE,
            depth_write_enable: BOOL_FALSE,
            depth_compare_op: api::CompareOp::Less,
            depth_bounds_test_enable: BOOL_FALSE,
            stencil_test_enable: BOOL_FALSE,
            front: api::StencilOpState::default(),
            back: api::StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        };

        let color_blend_attachment = api::PipelineColorBlendAttachmentState {
            blend_enable: BOOL_FALSE,
            src_color_blend_factor: api::BlendFactor::One,
            dst_color_blend_factor: api::BlendFactor::Zero,
            color_blend_op: api::BlendOp::Add,
            src_alpha_blend_factor: api::BlendFactor::One,
            dst_alpha_blend_factor: api::BlendFactor::Zero,
            alpha_blend_op: api::BlendOp::Add,
            color_write_mask: api::ColorComponentFlagBit::All as api::ColorComponentFlags,
        };
        let color_blend_create_info = api::PipelineColorBlendStateCreateInfo {
            header: StructureHeader::new(StructureType::PipelineColorBlendStateCreateInfo),
            flags: 0,
            logic_op_enable: BOOL_FALSE,
            logic_op: api::LogicOp::Copy,
            attachment_count: 1,
            attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        };

        let graphics_pipeline_create_info = api::GraphicsPipelineCreateInfo {
            header: StructureHeader::new(StructureType::GraphicsPipelineCreateInfo),
            flags: 0,
            stage_count: 2,
            stages: shader_stages.as_ptr(),
            vertex_input_state: &vertex_input_create_info,
            input_assembly_state: &input_assembly_create_info,
            tessellation_state: &tessilation_create_info,
            viewport_state: &viewport_create_info,
            rasterization_state: &rasterization_create_info,
            multisample_state: &multisampling_create_info,
            depth_stencil_state: &depth_stencil_create_info,
            color_blend_state: &color_blend_create_info,
            dynamic_state: ptr::null(),
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: ptr::null(),
            base_pipeline_index: -1,
        };
        let graphics_pipeline = ptr::null();
        let result = unsafe {
            api::vkCreateGraphicsPipelines(
                swapchain.device.handle,
                ptr::null(),
                1,
                &graphics_pipeline_create_info,
                ptr::null(),
                &graphics_pipeline,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        // Fence Create
        let fence_create_info = api::FenceCreateInfo {
            header: StructureHeader::new(StructureType::FenceCreateInfo),
            flags: 0,
        };
        let fence = ptr::null();
        let result = unsafe {
            api::vkCreateFence(
                swapchain.device.handle,
                &fence_create_info,
                ptr::null(),
                &fence,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let mut scr = SwapchainTriangleRender {
            fence,
            graphics_pipeline,
            pipeline_layout,
            descriptor_set_layout,
            shader_stages,
            gpu_triangle_buffer_mem,
            gpu_index_buffer,
            gpu_vertex_buffer,
            cpu_triangle_buffer_mem_size,
            cpu_triangle_buffer_mem,
            cpu_index_buffer,
            cpu_vertex_buffer,
            max_triangles,
            framebuffers,
            swapchain_image_views,
            render_pass,
            swapchain,
        };
        scr.write_initial_command_buffers(swapchain_width, swapchain_height)?;
        scr.swapchain.render_next_image(scr.fence)?; // Render once for a test and to allow the fence to be signalled

        Ok(scr)
    }

    pub fn get_verticies_and_indicies(
        &mut self,
    ) -> Result<(&mut [TriangleVertex], &mut [TriangleIndicies]), Error> {
        let result = unsafe {
            api::vkWaitForFences(
                self.swapchain.device.handle,
                1,
                &self.fence,
                BOOL_FALSE,
                100000000, // 100 ms in nanoseconds
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        let result = unsafe { api::vkResetFences(self.swapchain.device.handle, 1, &self.fence) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        let data_ptr = ptr::null_mut();
        let result = unsafe {
            api::vkMapMemory(
                self.swapchain.device.handle,
                self.cpu_triangle_buffer_mem,
                0,
                self.cpu_triangle_buffer_mem_size,
                api::MemoryMapFlagBit::None as api::MemoryMapFlags,
                &data_ptr,
            )
        };
        if result != 0 {
            return Err(Error::VkResult(result));
        }
        if !data_ptr.is_null() {
            let index_offset =
                unsafe { data_ptr.offset((mem::size_of::<TriangleVertex>() as isize) * (1 << 16)) };
            Ok((
                unsafe { std::slice::from_raw_parts_mut(data_ptr as *mut TriangleVertex, 1 << 16) },
                unsafe {
                    std::slice::from_raw_parts_mut(
                        index_offset as *mut TriangleIndicies,
                        self.max_triangles as usize,
                    )
                },
            ))
        } else {
            Err(Error::InvalidMapPtr)
        }
    }

    pub fn render(
        &mut self,
        num_verticies: u32,
        num_triangles: u32,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        unsafe { vkUnmapMemory(self.swapchain.device.handle, self.cpu_triangle_buffer_mem) };

        // Resize if necessarry here in future based on next image index results
        let next_image_index = self.swapchain.get_next_image_index()? as usize;

        let vertex_buffer_copy_region = api::BufferCopy2 {
            header: StructureHeader::new(StructureType::BufferCopy2),
            src_offset: 0,
            dst_offset: 0,
            size: (num_verticies as u64) * (mem::size_of::<TriangleVertex>() as u64),
        };
        let vertex_buffer_copy_info = api::CopyBufferInfo2 {
            header: StructureHeader::new(StructureType::CopyBufferInfo2),
            src_buffer: self.cpu_vertex_buffer,
            dst_buffer: self.gpu_vertex_buffer,
            region_count: 1,
            regions: &vertex_buffer_copy_region,
        };

        let index_buffer_copy_region = api::BufferCopy2 {
            header: StructureHeader::new(StructureType::BufferCopy2),
            src_offset: 0,
            dst_offset: 0,
            size: (num_triangles as u64) * (mem::size_of::<TriangleIndicies>() as u64),
        };
        let index_buffer_copy_info = api::CopyBufferInfo2 {
            header: StructureHeader::new(StructureType::CopyBufferInfo2),
            src_buffer: self.cpu_index_buffer,
            dst_buffer: self.gpu_index_buffer,
            region_count: 1,
            regions: &index_buffer_copy_region,
        };

        let cmd_buffer_begin_info = api::CommandBufferBeginInfo {
            header: StructureHeader::new(StructureType::CommandBufferBeginInfo),
            flags: api::CommandBufferUsageFlagBit::None as api::CommandBufferUsageFlags,
            inheritance_info: ptr::null(),
        };

        let clear_value = api::ClearValue {
            color: api::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };

        let render_pass_begin_info = api::RenderPassBeginInfo {
            header: StructureHeader::new(StructureType::RenderPassBeginInfo),
            render_pass: self.render_pass,
            framebuffer: self.framebuffers[next_image_index],
            render_area: api::Rect2D {
                offset: api::Offset2d::default(),
                extent: api::Extent2d { width, height },
            },
            clear_value_count: 1,
            clear_values: &clear_value,
        };

        let vertex_buffers = [self.gpu_vertex_buffer];
        let vertex_offsets = [0];

        let cmd_buffer = self.swapchain.cmd_buffer_submit_infos[next_image_index].command_buffer;

        let result = unsafe { api::vkBeginCommandBuffer(cmd_buffer, &cmd_buffer_begin_info) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        unsafe { api::vkCmdCopyBuffer2(cmd_buffer, &vertex_buffer_copy_info) };

        unsafe { api::vkCmdCopyBuffer2(cmd_buffer, &index_buffer_copy_info) };

        unsafe {
            api::vkCmdBeginRenderPass(
                cmd_buffer,
                &render_pass_begin_info,
                api::SubpassContents::Inline,
            )
        };

        unsafe {
            api::vkCmdBindPipeline(
                cmd_buffer,
                api::PipelineBindPoint::Graphics,
                self.graphics_pipeline,
            )
        };
        unsafe {
            api::vkCmdBindVertexBuffers(
                cmd_buffer,
                0,
                1,
                vertex_buffers.as_ptr(),
                vertex_offsets.as_ptr(),
            )
        };
        unsafe {
            api::vkCmdBindIndexBuffer(cmd_buffer, self.gpu_index_buffer, 0, api::IndexType::Uint16)
        };
        unsafe { api::vkCmdDrawIndexed(cmd_buffer, num_verticies, num_triangles, 0, 0, 0) };

        unsafe { api::vkCmdEndRenderPass(cmd_buffer) };

        let result = unsafe { api::vkEndCommandBuffer(cmd_buffer) };
        if result != 0 {
            return Err(Error::VkResult(result));
        }

        self.swapchain
            .submit_queue_and_present(next_image_index as u32, self.fence)?;
        Ok(())
    }
}
