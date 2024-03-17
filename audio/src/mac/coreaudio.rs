//Media Enhanced Swiftlet Audio Rust Library for Low Latency Audio OS I/O
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

use std::ffi::{c_char, CStr, CString};
use std::mem::size_of;
use std::os::raw::{c_int, c_uchar, c_uint, c_void};
use std::ptr;

enum PropertySelector {
    Devices,
    InputDefault,
    OutputDefault,
    NominalSampleRate,
    BufferFrameSize,
    BufferFrameSizeRange,
}

impl PropertySelector {
    fn get_u32(&self) -> u32 {
        let bytes = match self {
            Self::Devices => b"dev#",
            Self::InputDefault => b"dIn ",
            Self::OutputDefault => b"dOut",
            Self::NominalSampleRate => b"nsrt",
            Self::BufferFrameSize => b"fsiz",
            Self::BufferFrameSizeRange => b"fsz#",
        };
        u32::from_be_bytes(*bytes)
    }
}

#[repr(u32)]
enum PropertyScope {
    Global,
    Input,
    Output,
}

impl PropertyScope {
    fn get_u32(&self) -> u32 {
        let bytes = match self {
            Self::Global => b"glob",
            Self::Input => b"inpt",
            Self::Output => b"outp",
        };
        u32::from_be_bytes(*bytes)
    }
}

#[repr(C)]
struct PropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

impl PropertyAddress {
    fn new(selector: PropertySelector, scope: &PropertyScope) -> Self {
        PropertyAddress {
            selector: selector.get_u32(),
            scope: scope.get_u32(),
            element: 0,
        }
    }
}

// Reference
// type OSStatus = c_int;
// type

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object_id: c_uint,
        property_address: *const PropertyAddress,
        qualifier_data_size: c_uint,
        qualifier_data: *const c_uchar,
        out_data_size: *mut c_uint,
    ) -> c_int;

    fn AudioObjectGetPropertyData(
        object_id: c_uint,
        property_address: *const PropertyAddress,
        qualifier_data_size: c_uint,
        qualifier_data: *const c_uchar,
        io_data_size: *mut c_uint,
        out_data: *mut c_char,
    ) -> c_int;

    fn AudioObjectSetPropertyData(
        object_id: c_uint,
        property_address: *const PropertyAddress,
        qualifier_data_size: c_uint,
        qualifier_data: *const c_uchar,
        data_size: c_uint,
        data: *const c_char,
    ) -> c_int;

    // Can add functions to detect default device changing in future (and possibly nominal sample rate changing)
}

// A private structure that is only used as a Raw Pointer handle
// This handle "points" to the private structure
#[repr(C)]
struct OpaqueStructure {
    _unused: [u8; 0],
}

#[repr(C)]
struct AudioComponentDescription {
    component_type: u32,
    sub_type: u32,
    manufacturer: u32,
    flags: u32,
    flag_mask: u32,
}

#[repr(C)]
struct RenderCallbackData {
    closure_ptr: *mut c_void,
    is_capture: bool,
    sync_tx_ptr: *const c_void,
    audio_unit: *const OpaqueStructure,
}

#[repr(u32)]
enum AudioUnitActionFlags {
    PreRender = 0x4,
    PostRender = 0x8,
    OutputIsSilence = 0x10,
    Preflight = 0x20,
    Render = 0x40,
    Complete = 0x80,
    PostRenderError = 0x100,
    DoNotCheckRenderArgs = 0x200,
}

#[inline]
fn check_action_flag(action_flags: u32, flag: AudioUnitActionFlags) -> bool {
    ((flag as u32) & action_flags) > 0
}

// SMPTE Time
#[repr(C)]
struct SmpteTime {
    subframes: i16,
    subframe_divisor: i16,
    counter: u32,
    time_type: u32, // Future SmpteTimeType Enum
    flags: u32,     // Future SmpteTimeFlags Enum
    hours: i16,
    minutes: i16,
    seconds: i16,
    frames: i16,
}

#[repr(C)]
struct AudioTimeStamp {
    sample_time: f64,
    host_time: u64,
    rate_scalar: f64,
    word_clock_time: u64,
    smpte_time: SmpteTime,
    flags: u32,
    reserved: u32,
}

#[repr(C)]
struct AudioBuffer {
    num_channels: u32,
    data_byte_size: u32,
    data: *mut u8,
}

#[repr(C)]
struct AudioBufferList {
    num_buffers: u32,
    buffers: [AudioBuffer; 1], //[AudioBuffer]
}

#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    fn AudioComponentFindNext(
        search_after: *const OpaqueStructure,
        description: *const AudioComponentDescription,
    ) -> *const OpaqueStructure;

    fn AudioComponentInstanceNew(
        component: *const OpaqueStructure,
        instance: *mut *const OpaqueStructure,
    ) -> c_int;

    fn AudioComponentInstanceDispose(instance: *const OpaqueStructure) -> c_int;

    fn AudioUnitSetProperty(
        audio_unit: *const OpaqueStructure,
        property_id: c_uint,
        scope: c_uint,
        element: c_uint,
        data: *const c_char,
        data_size: c_uint,
    ) -> c_int;

    fn AudioUnitGetProperty(
        audio_unit: *const OpaqueStructure,
        property_id: c_uint,
        scope: c_uint,
        element: c_uint,
        data: *mut c_char,
        data_size: *mut c_uint,
    ) -> c_int;

    fn AudioUnitAddRenderNotify(
        audio_unit: *const OpaqueStructure,
        callback_function: extern "C" fn(
            *mut RenderCallbackData,
            *mut u32,
            *const AudioTimeStamp,
            u32,
            u32,
            *mut AudioBufferList,
        ) -> c_int,
        callback_data: *mut RenderCallbackData,
    ) -> c_int;

    fn AudioUnitInitialize(audio_unit: *const OpaqueStructure) -> c_int;

    fn AudioOutputUnitStart(audio_unit: *const OpaqueStructure) -> c_int;
    fn AudioOutputUnitStop(audio_unit: *const OpaqueStructure) -> c_int;

    fn AudioUnitRender(
        audio_unit: *const OpaqueStructure,
        io_action_flags: *mut u32,
        in_time_stamp: *const AudioTimeStamp,
        in_output_bus_number: u32,
        in_number_frames: u32,
        io_data: *mut AudioBufferList,
    ) -> c_int;

    // fn AudioUnitGetPropertyInfo() -> c_int;
    //
}

/// CoreAudio Error
#[derive(Debug)]
pub enum Error {
    Test = 2,
    SliceTooLong = 1,
    Ok = 0, // No Error
    Unimplemented = -4,
    FileNotFound = -43,
    FilePermission = -54,
    TooManyFilesOpen = -42,
    BadFilePath = 0x21707468, // '!pth', 561017960
    Param = -50,
    MemFull = -108,
}

impl Error {
    fn from_i32(v: i32) -> Self {
        match v {
            x if x == Error::Ok as i32 => Error::Ok,
            x if x == Error::FileNotFound as i32 => Error::FileNotFound,
            x if x == Error::FilePermission as i32 => Error::FilePermission,
            x if x == Error::TooManyFilesOpen as i32 => Error::TooManyFilesOpen,
            x if x == Error::BadFilePath as i32 => Error::BadFilePath,
            x if x == Error::Param as i32 => Error::Param,
            x if x == Error::MemFull as i32 => Error::MemFull,
            _ => Error::Unimplemented,
        }
    }
}

const SYSTEM_OBJECT_ID: c_uint = 1;

#[repr(C)]
struct ValueRange {
    minimum: f64,
    maximum: f64,
}

fn list_audio_objects() -> Result<(), Error> {
    let property_address = PropertyAddress::new(PropertySelector::Devices, &PropertyScope::Global);

    let null_ptr = ptr::null();
    let mut object_id_bytes = 0;

    let errnum = unsafe {
        AudioObjectGetPropertyDataSize(
            SYSTEM_OBJECT_ID,
            &property_address,
            0,
            null_ptr,
            &mut object_id_bytes,
        )
    };
    if errnum != 0 {
        return Err(Error::from_i32(errnum));
    }

    //println!("Device Size: {}", device_size);
    let mut object_ids: Vec<i8> = vec![0; object_id_bytes as usize];

    let errnum = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM_OBJECT_ID,
            &property_address,
            0,
            null_ptr,
            &mut object_id_bytes,
            object_ids.as_mut_ptr(),
        )
    };
    if errnum != 0 {
        return Err(Error::from_i32(errnum));
    }

    for ind in 0..(object_id_bytes >> 2) as usize {
        let id = u32::from_ne_bytes([
            object_ids[ind * 4] as u8,
            object_ids[ind * 4 + 1] as u8,
            object_ids[ind * 4 + 2] as u8,
            object_ids[ind * 4 + 3] as u8,
        ]);
        //println!("Object ID: {}", id);
    }

    Ok(())
}

#[repr(u32)]
enum AudioUnitPropertyId {
    SampleRate = 2,
    StreamFormat = 8,
    MaximumFramesPerSlice = 14,
    LastRenderError = 22,
    SetRenderCallback = 23,
    CurrentDevice = 2000,
    EnableIo = 2003,
    SetInputCallback = 2005,
}

#[repr(u32)]
enum AudioUnitScope {
    Global = 0,
    Input = 1,
    Output = 2,
}

#[repr(u32)]
enum AudioFormatId {
    LinearPcm, // Should account for fixed-point representation in future
    Opus,
}

impl AudioFormatId {
    fn get_u32(&self) -> u32 {
        let bytes = match self {
            Self::LinearPcm => b"lpcm",
            Self::Opus => b"opus",
        };
        u32::from_be_bytes(*bytes)
    }
}

#[repr(u32)]
enum AudioFormatFlags {
    IsFloat = 0x1,
    IsBigEndian = 0x2,
    IsSignedInteger = 0x4,
    IsPacked = 0x8,
    IsAlignedHigh = 0x10,
    IsNonInterleaved = 0x20,
    IsNonMixable = 0x40,
    AreAllClear = 0x80,
}

#[inline]
fn check_format_flag(format_flags: u32, flag: AudioFormatFlags) -> bool {
    ((flag as u32) & format_flags) > 0
}

#[repr(C)]
struct AudioStreamBasicDescription {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
    reserved: u32,
}

impl AudioStreamBasicDescription {
    fn new_blank() -> Self {
        AudioStreamBasicDescription {
            sample_rate: 0.0,
            format_id: 0,
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: 0,
            bytes_per_frame: 0,
            channels_per_frame: 0,
            bits_per_channel: 0,
            reserved: 0,
        }
    }

    fn print(&self) {
        println!("Sample Rate: {}", self.sample_rate);
        println!("Channels Per Frame: {}", self.channels_per_frame);
        println!("Bits Per Channel: {}", self.bits_per_channel);
        println!("Bytes Per Frame: {}", self.bytes_per_frame);
        println!("Frames Per Packet: {}", self.frames_per_packet);
        println!("Bytes Per Packet: {}", self.bytes_per_packet);
        println!("Format Flags: {}", self.format_flags);
    }
}

#[repr(C)]
struct AudioUnitRenderCallback {
    function: extern "C" fn(
        *mut RenderCallbackData,
        *mut u32,
        *const AudioTimeStamp,
        u32,
        u32,
        *mut AudioBufferList,
    ) -> c_int,
    data: *mut RenderCallbackData,
}

/// CoreAudio Device Manager
pub(super) struct Device {
    //handle: OpaqueStructure,
    id: u32,
    is_capture: bool,
    audio_unit: *const OpaqueStructure,
}

impl Device {
    fn get_audio_unit(id: u32, is_capture: bool) -> Result<*const OpaqueStructure, Error> {
        let description = AudioComponentDescription {
            component_type: u32::from_be_bytes(*b"auou"),
            sub_type: u32::from_be_bytes(*b"ahal"),
            manufacturer: u32::from_be_bytes(*b"appl"),
            flags: 0,
            flag_mask: 0,
        };

        if let Some(component) =
            unsafe { AudioComponentFindNext(ptr::null(), &description).as_ref() }
        {
            let mut audio_unit = ptr::null();
            let errnum = unsafe { AudioComponentInstanceNew(component, &mut audio_unit) };
            if errnum != 0 {
                return Err(Error::from_i32(errnum));
            }

            let element_bus_output = 0;
            let element_bus_input = 1;
            let data_disable = 0;
            let data_enable = 1;

            if !is_capture {
                let errnum = unsafe {
                    AudioUnitSetProperty(
                        audio_unit,
                        AudioUnitPropertyId::EnableIo as u32,
                        AudioUnitScope::Input as u32,
                        element_bus_input,
                        ptr::addr_of!(data_disable) as *const i8,
                        size_of::<u32>() as u32,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }
                let errnum = unsafe {
                    AudioUnitSetProperty(
                        audio_unit,
                        AudioUnitPropertyId::EnableIo as u32,
                        AudioUnitScope::Output as u32,
                        element_bus_output,
                        ptr::addr_of!(data_enable) as *const i8,
                        size_of::<u32>() as u32,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }
            } else {
                let errnum = unsafe {
                    AudioUnitSetProperty(
                        audio_unit,
                        AudioUnitPropertyId::EnableIo as u32,
                        AudioUnitScope::Output as u32,
                        element_bus_output,
                        ptr::addr_of!(data_disable) as *const i8,
                        size_of::<u32>() as u32,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }
                let errnum = unsafe {
                    AudioUnitSetProperty(
                        audio_unit,
                        AudioUnitPropertyId::EnableIo as u32,
                        AudioUnitScope::Input as u32,
                        element_bus_input,
                        ptr::addr_of!(data_enable) as *const i8,
                        size_of::<u32>() as u32,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }
            }

            let errnum = unsafe {
                AudioUnitSetProperty(
                    audio_unit,
                    AudioUnitPropertyId::CurrentDevice as u32,
                    AudioUnitScope::Global as u32,
                    0,
                    ptr::addr_of!(id) as *const i8,
                    size_of::<u32>() as u32,
                )
            };
            if errnum != 0 {
                return Err(Error::from_i32(errnum));
            }

            Ok(audio_unit)
        } else {
            Err(Error::Test)
        }
    }

    pub(super) fn new_from_default_playback(sample_rate: u32, period: u32) -> Result<Self, Error> {
        //list_audio_objects()?;

        let property_address =
            PropertyAddress::new(PropertySelector::OutputDefault, &PropertyScope::Global);

        let null_ptr = ptr::null();
        let mut object_id_bytes = 4;
        let mut object_id_data: [i8; 4] = [0; 4];

        let errnum = unsafe {
            AudioObjectGetPropertyData(
                SYSTEM_OBJECT_ID,
                &property_address,
                0,
                null_ptr,
                &mut object_id_bytes,
                object_id_data.as_mut_ptr(),
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let id = u32::from_ne_bytes([
            object_id_data[0] as u8,
            object_id_data[1] as u8,
            object_id_data[2] as u8,
            object_id_data[3] as u8,
        ]);
        //println!("Object ID: {}", id);

        let is_capture = false;
        let audio_unit = Device::get_audio_unit(id, is_capture)?;

        let device = Device {
            id,
            is_capture,
            audio_unit,
        };

        device.set_sample_rate(sample_rate)?;
        device.set_period(period)?;

        Ok(device)
    }

    pub(super) fn new_from_default_capture(sample_rate: u32, period: u32) -> Result<Self, Error> {
        //list_audio_objects()?;

        let property_address =
            PropertyAddress::new(PropertySelector::InputDefault, &PropertyScope::Global);

        let null_ptr = ptr::null();
        let mut object_id_bytes = 4;
        let mut object_id_data: [i8; 4] = [0; 4];

        let errnum = unsafe {
            AudioObjectGetPropertyData(
                SYSTEM_OBJECT_ID,
                &property_address,
                0,
                null_ptr,
                &mut object_id_bytes,
                object_id_data.as_mut_ptr(),
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let id = u32::from_ne_bytes([
            object_id_data[0] as u8,
            object_id_data[1] as u8,
            object_id_data[2] as u8,
            object_id_data[3] as u8,
        ]);
        //println!("Object ID: {}", id);

        let is_capture = true;
        let audio_unit = Device::get_audio_unit(id, is_capture)?;

        let device = Device {
            id,
            is_capture,
            audio_unit,
        };

        device.set_sample_rate(sample_rate)?;
        device.set_period(period)?;

        Ok(device)
    }

    fn get_stream_description(&self) -> Result<AudioStreamBasicDescription, Error> {
        let (scope, element) = match self.is_capture {
            false => (AudioUnitScope::Output as u32, 0),
            true => (AudioUnitScope::Input as u32, 1),
        };

        let mut data_size = size_of::<AudioStreamBasicDescription>() as u32;
        let data = AudioStreamBasicDescription::new_blank();

        let errnum = unsafe {
            AudioUnitGetProperty(
                self.audio_unit,
                AudioUnitPropertyId::StreamFormat as u32,
                scope,
                element,
                ptr::addr_of!(data) as *mut i8,
                &mut data_size,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        Ok(data)
    }

    fn set_sample_rate(&self, sample_rate: u32) -> Result<(), Error> {
        if self.get_stream_description()?.sample_rate != sample_rate as f64 {
            let scope = match self.is_capture {
                false => PropertyScope::Output,
                true => PropertyScope::Input,
            };

            let property_address =
                PropertyAddress::new(PropertySelector::NominalSampleRate, &scope);

            let data_size = size_of::<ValueRange>();
            let data = ValueRange {
                minimum: sample_rate as f64,
                maximum: sample_rate as f64,
            };

            let errnum = unsafe {
                AudioObjectSetPropertyData(
                    self.id,
                    &property_address,
                    0,
                    ptr::null(),
                    data_size as u32,
                    ptr::addr_of!(data) as *const i8,
                )
            };
            if errnum != 0 {
                return Err(Error::from_i32(errnum));
            }

            std::thread::sleep(std::time::Duration::from_secs(4));
        }

        Ok(())
    }

    fn set_period(&self, period: u32) -> Result<(), Error> {
        let adjusted_period = period; // *self.get_num_channels()?;

        let scope = match self.is_capture {
            false => PropertyScope::Output,
            true => PropertyScope::Input,
        };

        let property_address = PropertyAddress::new(PropertySelector::BufferFrameSizeRange, &scope);

        let mut data_size = size_of::<ValueRange>() as u32;
        let data = ValueRange {
            minimum: adjusted_period as f64,
            maximum: adjusted_period as f64,
        };

        let errnum = unsafe {
            AudioObjectGetPropertyData(
                self.id,
                &property_address,
                0,
                ptr::null(),
                &mut data_size,
                ptr::addr_of!(data) as *mut i8,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let period_check = adjusted_period as f64;
        if (period_check < data.minimum) || (period_check > data.maximum) {
            return Err(Error::Test);
        }

        let property_address = PropertyAddress::new(PropertySelector::BufferFrameSize, &scope);
        let errnum = unsafe {
            AudioObjectSetPropertyData(
                self.id,
                &property_address,
                0,
                ptr::null(),
                size_of::<u32>() as u32,
                ptr::addr_of!(adjusted_period) as *const i8,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        // Should possibly double check that it was actually set here

        let adjusted_period = adjusted_period * 2;
        let errnum = unsafe {
            AudioUnitSetProperty(
                self.audio_unit,
                AudioUnitPropertyId::MaximumFramesPerSlice as u32,
                AudioUnitScope::Global as u32,
                0,
                ptr::addr_of!(adjusted_period) as *const i8,
                size_of::<u32>() as u32,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        Ok(())
    }

    pub(super) fn get_num_channels(&self) -> Result<u32, Error> {
        let stream_data = self.get_stream_description()?;
        Ok(stream_data.channels_per_frame)
    }

    pub(super) fn print_stream_format(&self) -> Result<(), Error> {
        let stream_data = self.get_stream_description()?;
        stream_data.print();
        Ok(())
    }

    pub(super) fn print_device_period(&self) -> Result<(), Error> {
        let scope = match self.is_capture {
            false => PropertyScope::Output,
            true => PropertyScope::Input,
        };

        let property_address = PropertyAddress::new(PropertySelector::BufferFrameSize, &scope);

        let mut data_size = size_of::<u32>() as u32;
        let data = 0;

        let errnum = unsafe {
            AudioObjectGetPropertyData(
                self.id,
                &property_address,
                0,
                ptr::null(),
                &mut data_size,
                ptr::addr_of!(data) as *mut i8,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        println!("Device Period: {}", data);

        Ok(())
    }

    pub(super) fn run_output_callback_loop(
        &self,
        mut closure: &mut OutputClosure,
    ) -> Result<(), Error> {
        // Double check the canonical default
        let stream_data = self.get_stream_description()?;
        if stream_data.format_id != AudioFormatId::LinearPcm.get_u32() {
            return Err(Error::Test);
        }
        if !check_format_flag(stream_data.format_flags, AudioFormatFlags::IsFloat) {
            return Err(Error::Test);
        }
        if !check_format_flag(stream_data.format_flags, AudioFormatFlags::IsPacked) {
            return Err(Error::Test);
        }
        if check_format_flag(stream_data.format_flags, AudioFormatFlags::IsNonInterleaved) {
            return Err(Error::Test);
        }

        let errnum = unsafe {
            AudioUnitSetProperty(
                self.audio_unit,
                AudioUnitPropertyId::StreamFormat as u32,
                AudioUnitScope::Input as u32,
                0,
                ptr::addr_of!(stream_data) as *const i8,
                size_of::<AudioStreamBasicDescription>() as u32,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<CallbackStop>(1);

        //println!("Got Here!");
        //let mut closure = move |samples: &mut [f32]| callback.output_callback(samples);
        //let mut cp = (&mut closure) as &mut OutputClosure;
        //let s = ptr::addr_of_mut!(&mut dyn callback);

        let mut callback_data = RenderCallbackData {
            closure_ptr: ptr::addr_of_mut!(closure) as *mut c_void,
            is_capture: self.is_capture,
            sync_tx_ptr: ptr::addr_of!(sync_tx) as *const c_void,
            audio_unit: ptr::null(),
        };

        // let errnum = unsafe {
        //     AudioUnitAddRenderNotify(self.audio_unit, render_callback, &mut callback_data)
        // };
        // if errnum != 0 {
        //     return Err(Error::from_i32(errnum));
        // }

        let callback_info = AudioUnitRenderCallback {
            function: render_callback,
            data: &mut callback_data,
        };

        let errnum = unsafe {
            AudioUnitSetProperty(
                self.audio_unit,
                AudioUnitPropertyId::SetRenderCallback as u32,
                AudioUnitScope::Global as u32,
                0,
                ptr::addr_of!(callback_info) as *const i8,
                size_of::<AudioUnitRenderCallback>() as u32,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let errnum = unsafe { AudioUnitInitialize(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let errnum = unsafe { AudioOutputUnitStart(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let callback_stop = match sync_rx.recv() {
            Ok(cs) => cs,
            Err(_) => return Err(Error::Test),
        };

        let errnum = unsafe { AudioOutputUnitStop(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        match callback_stop {
            CallbackStop::Normal => Ok(()),
            CallbackStop::UnexpectedBufferListNum(_num) => Err(Error::Test),
            CallbackStop::LastRenderError => {
                let mut data_size = size_of::<i32>() as u32;
                let data = Error::Ok as i32;
                let errnum = unsafe {
                    AudioUnitGetProperty(
                        self.audio_unit,
                        AudioUnitPropertyId::LastRenderError as u32,
                        AudioUnitScope::Global as u32,
                        0,
                        ptr::addr_of!(data) as *mut i8,
                        &mut data_size,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }

                println!("Last Render Error: {}", data);
                Err(Error::Test)
            }
            CallbackStop::Errnum(num) => {
                println!("Errornum: {}", num);
                Err(Error::Test)
            }
        }
    }

    pub(super) fn run_input_callback_loop(
        &self,
        channels: u32,
        mut closure: &mut InputClosure,
    ) -> Result<(), Error> {
        // let mut data_size = size_of::<AudioStreamBasicDescription>() as u32;
        // let stream_data_result = AudioStreamBasicDescription::new_blank();

        // let errnum = unsafe {
        //     AudioUnitGetProperty(
        //         self.audio_unit,
        //         AudioUnitPropertyId::StreamFormat as u32,
        //         AudioUnitScope::Output as u32,
        //         1,
        //         ptr::addr_of!(stream_data_result) as *mut i8,
        //         &mut data_size,
        //     )
        // };
        // if errnum != 0 {
        //     return Err(Error::from_i32(errnum));
        // }

        // println!(
        //     "Set Channel Count: {}",
        //     stream_data_result.channels_per_frame
        // );

        let mut stream_data = self.get_stream_description()?;
        if stream_data.format_id != AudioFormatId::LinearPcm.get_u32() {
            return Err(Error::Test);
        }
        if !check_format_flag(stream_data.format_flags, AudioFormatFlags::IsFloat) {
            return Err(Error::Test);
        }
        if !check_format_flag(stream_data.format_flags, AudioFormatFlags::IsPacked) {
            return Err(Error::Test);
        }
        if check_format_flag(stream_data.format_flags, AudioFormatFlags::IsNonInterleaved) {
            return Err(Error::Test);
        }

        if stream_data.channels_per_frame != channels {
            println!("Original Channel Count: {}", stream_data.channels_per_frame);
            stream_data.channels_per_frame = channels;
            stream_data.bytes_per_frame = channels * 4; //stream_data.bits_per_channel * 8;
            stream_data.bytes_per_packet =
                stream_data.frames_per_packet * stream_data.bytes_per_frame;
        }

        let errnum = unsafe {
            AudioUnitSetProperty(
                self.audio_unit,
                AudioUnitPropertyId::StreamFormat as u32,
                AudioUnitScope::Output as u32,
                1,
                ptr::addr_of!(stream_data) as *const i8,
                size_of::<AudioStreamBasicDescription>() as u32,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let mut data_size = size_of::<AudioStreamBasicDescription>() as u32;
        let stream_data_result = AudioStreamBasicDescription::new_blank();

        let errnum = unsafe {
            AudioUnitGetProperty(
                self.audio_unit,
                AudioUnitPropertyId::StreamFormat as u32,
                AudioUnitScope::Output as u32,
                1,
                ptr::addr_of!(stream_data_result) as *mut i8,
                &mut data_size,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        println!(
            "Set Channel Count: {}",
            stream_data_result.channels_per_frame
        );

        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<CallbackStop>(1);

        let mut callback_data = RenderCallbackData {
            closure_ptr: ptr::addr_of_mut!(closure) as *mut c_void,
            is_capture: self.is_capture,
            sync_tx_ptr: ptr::addr_of!(sync_tx) as *const c_void,
            audio_unit: self.audio_unit,
        };

        let callback_info = AudioUnitRenderCallback {
            function: capture_callback,
            data: &mut callback_data,
        };

        let errnum = unsafe {
            AudioUnitSetProperty(
                self.audio_unit,
                AudioUnitPropertyId::SetInputCallback as u32,
                AudioUnitScope::Global as u32,
                0,
                ptr::addr_of!(callback_info) as *const i8,
                size_of::<AudioUnitRenderCallback>() as u32,
            )
        };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let errnum = unsafe { AudioUnitInitialize(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let errnum = unsafe { AudioOutputUnitStart(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        let callback_stop = match sync_rx.recv() {
            Ok(cs) => cs,
            Err(_) => return Err(Error::Test),
        };

        let errnum = unsafe { AudioOutputUnitStop(self.audio_unit) };
        if errnum != 0 {
            return Err(Error::from_i32(errnum));
        }

        match callback_stop {
            CallbackStop::Normal => Ok(()),
            CallbackStop::UnexpectedBufferListNum(num) => {
                println!("Unexpected Buffer: {}", num);
                Err(Error::Test)
            }
            CallbackStop::LastRenderError => {
                let mut data_size = size_of::<i32>() as u32;
                let data = Error::Ok as i32;
                let errnum = unsafe {
                    AudioUnitGetProperty(
                        self.audio_unit,
                        AudioUnitPropertyId::LastRenderError as u32,
                        AudioUnitScope::Global as u32,
                        0,
                        ptr::addr_of!(data) as *mut i8,
                        &mut data_size,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_i32(errnum));
                }

                println!("Last Render Error: {}", data);
                Err(Error::Test)
            }
            CallbackStop::Errnum(num) => {
                println!("Errornum: {}", num);
                Err(Error::Test)
            }
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            // Error Not Currently Handled:
            AudioComponentInstanceDispose(self.audio_unit);
        }
    }
}

enum CallbackStop {
    Normal,
    UnexpectedBufferListNum(u32),
    LastRenderError,
    Errnum(i32),
}

fn sync_send(sync_tx_ptr: *const c_void, callback_stop: CallbackStop) {
    if let Some(sync) = unsafe { sync_tx_ptr.as_ref() } {
        let sync_tx: &std::sync::mpsc::SyncSender<CallbackStop> =
            unsafe { std::mem::transmute(sync) };
        if sync_tx.send(callback_stop).is_err() {
            panic!("Sender Error!");
        }
    }
}

type OutputClosure = dyn FnMut(&mut [f32]) -> bool;
type InputClosure = dyn FnMut(&[f32]) -> bool;

extern "C" fn render_callback(
    callback_data: *mut RenderCallbackData,
    action_flags_ptr: *mut u32,
    time_stamp: *const AudioTimeStamp,
    bus_number: u32,
    frame_count: u32,
    buffer_list: *mut AudioBufferList,
) -> c_int {
    //println!("Render Callback Frames: {}", frame_count);

    if let Some(cb_data) = unsafe { callback_data.as_mut() } {
        // if let Some(action_flags) = unsafe { action_flags_ptr.as_mut() } {
        //     //println!("Action Flags: {}", action_flags);
        //     if check_action_flag(*action_flags, AudioUnitActionFlags::OutputIsSilence) {
        //         return Error::Ok as c_int;
        //         //*action_flags = 0;
        //     } else if check_action_flag(*action_flags, AudioUnitActionFlags::PostRenderError) {
        //         sync_send(cb_data.sync_tx_ptr, CallbackStop::LastRenderError);
        //         return Error::Ok as c_int;
        //     }
        // } //else {
        //   //  panic!("Callback not working as expected")
        //   //}

        if !cb_data.is_capture {
            if let Some(cb_fn) = unsafe { cb_data.closure_ptr.as_mut() } {
                let closure: &mut &mut OutputClosure = unsafe { std::mem::transmute(cb_fn) };

                if let Some(buffer_list) = unsafe { buffer_list.as_mut() } {
                    // println!("Buffers Address: {:?}", buffer_list.buffers);
                    // let buffers = unsafe {
                    //     std::slice::from_raw_parts_mut(
                    //         buffer_list.buffers,
                    //         buffer_list.num_buffers as usize,
                    //     )
                    // };

                    if buffer_list.num_buffers == 1 {
                        //println!("Buffer Channels: {}", buffer_list.buffers[0].num_channels);
                        //println!("Buffer Bytes: {}", buffer_list.buffers[0].data_byte_size);

                        let float_data = unsafe {
                            std::slice::from_raw_parts_mut(
                                buffer_list.buffers[0].data as *mut f32,
                                // (buffer_list.buffers[0].data_byte_size
                                //     / (4 * buffer_list.buffers[0].num_channels))
                                (buffer_list.buffers[0].data_byte_size >> 2) as usize,
                            )
                        };

                        if closure(float_data) {
                            sync_send(cb_data.sync_tx_ptr, CallbackStop::Normal);
                        }
                    } else {
                        sync_send(
                            cb_data.sync_tx_ptr,
                            CallbackStop::UnexpectedBufferListNum(buffer_list.num_buffers),
                        );
                    }
                }
            }
        } else {
            //println!("Got to Else!");
        }

        Error::Ok as c_int
    } else {
        panic!("Callback data was not set properly!");
    }
}

extern "C" fn capture_callback(
    callback_data: *mut RenderCallbackData,
    action_flags_ptr: *mut u32,
    time_stamp: *const AudioTimeStamp,
    bus_number: u32,
    frame_count: u32,
    buffer_list: *mut AudioBufferList,
) -> c_int {
    //println!("Render Callback Frames: {}", frame_count);

    if let Some(cb_data) = unsafe { callback_data.as_mut() } {
        // if let Some(action_flags) = unsafe { action_flags_ptr.as_mut() } {
        //     println!("Action Flags: {}", action_flags);
        //     if check_action_flag(*action_flags, AudioUnitActionFlags::OutputIsSilence) {
        //         return Error::Ok as c_int;
        //         //*action_flags = 0;
        //     } else if check_action_flag(*action_flags, AudioUnitActionFlags::PostRenderError) {
        //         sync_send(cb_data.sync_tx_ptr, CallbackStop::LastRenderError);
        //         return Error::Ok as c_int;
        //     }
        // } else {
        //     panic!("Callback not working as expected")
        // }

        if cb_data.is_capture {
            if let Some(cb_fn) = unsafe { cb_data.closure_ptr.as_mut() } {
                let closure: &mut &mut InputClosure = unsafe { std::mem::transmute(cb_fn) };
                //println!("Action Flags: {}, Frame_count: {}", 0, frame_count);

                let mut buffer_vec = vec![0; 480 * 4];
                let buffer_vec_ptr = buffer_vec.as_mut_ptr();
                //Recreate Audio Buffer List each time for now:
                let mut local_buffer_list = AudioBufferList {
                    num_buffers: 1,
                    buffers: [AudioBuffer {
                        num_channels: 1,
                        data_byte_size: 480 * 4,
                        data: buffer_vec_ptr,
                    }],
                };

                let errnum = unsafe {
                    AudioUnitRender(
                        cb_data.audio_unit,
                        action_flags_ptr,
                        time_stamp,
                        bus_number,
                        frame_count,
                        &mut local_buffer_list,
                    )
                };

                if errnum == 0 {
                    // println!(
                    //     "Channels: {}, Byte Size: {}",
                    //     local_buffer_list.buffers[0].num_channels,
                    //     local_buffer_list.buffers[0].data_byte_size
                    // );
                    //println!("Buffers: {}", local_buffer_list.num_buffers);
                    let local_buffer_list_data_ptr = local_buffer_list.buffers[0].data;
                    if buffer_vec_ptr != local_buffer_list_data_ptr {
                        println!("Don't need a vector!");
                    }

                    let float_data = unsafe {
                        std::slice::from_raw_parts(local_buffer_list_data_ptr as *const f32, 480)
                    };

                    if closure(float_data) {
                        sync_send(cb_data.sync_tx_ptr, CallbackStop::Normal);
                    }
                } else {
                    sync_send(cb_data.sync_tx_ptr, CallbackStop::Errnum(errnum));
                }
            }
        } else {
            //println!("Got to Else!");
        }

        Error::Ok as c_int
    } else {
        panic!("Callback data was not set properly!");
    }
}
