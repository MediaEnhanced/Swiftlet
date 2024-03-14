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

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::ptr;

/// Alsa Error
pub(super) enum Error {
    StringCreation(i32),
    Generic((i32, String)),
}

// A private structure that is only used as a Raw Pointer handle
// This handle "points" to the private structure
#[repr(C)]
struct OpaqueStructure {
    _unused: [u8; 0],
}

/// Alsa PCM Handle

pub(super) struct Pcm {
    handle: *mut OpaqueStructure,
    is_capture: bool,
}

/// Alsa PCM Stream Direction
#[repr(C)]
pub(super) enum PcmStream {
    Playback = 0,
    Capture = 1,
}

/// Alsa PCM State
#[derive(Debug)]
#[repr(C)]
pub(super) enum PcmState {
    Open = 0,
    Setup = 1,
    Prepared,
    Running,
    XRun,
    Draining,
    Paused,
    Suspended,
    Disconnected,
    Unknown,
}

impl PcmState {
    fn from_u32(v: u32) -> Self {
        match v {
            x if x == PcmState::Open as u32 => PcmState::Open,
            x if x == PcmState::Setup as u32 => PcmState::Setup,
            x if x == PcmState::Prepared as u32 => PcmState::Prepared,
            x if x == PcmState::Running as u32 => PcmState::Running,
            x if x == PcmState::XRun as u32 => PcmState::XRun,
            x if x == PcmState::Draining as u32 => PcmState::Draining,
            x if x == PcmState::Paused as u32 => PcmState::Paused,
            x if x == PcmState::Suspended as u32 => PcmState::Suspended,
            x if x == PcmState::Disconnected as u32 => PcmState::Disconnected,
            _ => PcmState::Unknown,
        }
    }
}

/// Alsa PCM Hardware Configuration Parameters
pub(super) struct PcmHwParams<'a> {
    handle: *mut OpaqueStructure,
    pcm_link: &'a Pcm,
}

pub(super) enum PcmHwParam {
    NearestRate(u32),
    FormatFloat,
    BufferInterleaved,
    Channels(u32),
    NearestPeriod(u64),
    NearestBufferSize(u64),
}

#[repr(C)]
enum PcmHwParamDirection {
    Less = -1,
    Nearest = 0,
    Greater = 1,
}

#[repr(C)]
enum PcmHwParamFormat {
    Unknown = -1,
    S8 = 0,
    U8 = 1,
    S16LE,
    S16BE,
    U16LE,
    U16BE,
    S24LE,
    S24BE,
    U24LE,
    U24BE,
    S32LE,
    S32BE,
    U32LE,
    U32BE,
    FloatLE,
    FloatBE,
    Float64LE,
    Float64BE,
}

#[repr(C)]
enum PcmHwParamAccess {
    MMapInterleaved = 0,
    MMapNonInterleaved = 1,
    MMapComplex,
    RWInterleaved,
    RWNonInterleaved,
}

/// Alsa PCM Software Configuration Parameters
pub(super) struct PcmSwParams<'a> {
    handle: *mut OpaqueStructure,
    pcm_link: &'a Pcm,
}

#[link(name = "asound")]
extern "C" {
    /// Convert a typically returned errorcode into a debug string
    fn snd_strerror(errnum: c_int) -> *const c_char;

    /// Open an Alsa PCM device for the given stream direction.
    ///
    /// Using "default" for the name field yields the default device.
    ///
    /// Returns 0 on success otherwise a negative error code
    fn snd_pcm_open(
        handle_ptr: *mut *mut OpaqueStructure,
        name: *const c_char,
        stream: PcmStream,
        mode: c_int,
    ) -> c_int;
    fn snd_pcm_close(pcm_handle: *mut OpaqueStructure) -> c_int;

    /// Get Alsa PCM state
    fn snd_pcm_state(pcm_handle: *mut OpaqueStructure) -> u32;

    /// Set Alsa PCM Hardware Parameters
    fn snd_pcm_hw_params(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
    ) -> c_int;

    // Start and stop the PCM device
    fn snd_pcm_start(pcm_handle: *mut OpaqueStructure) -> c_int;
    fn snd_pcm_drop(pcm_handle: *mut OpaqueStructure) -> c_int;

    /// Alsa PCM wait til ready
    fn snd_pcm_wait(pcm_handle: *mut OpaqueStructure, timeout: c_int) -> c_int;

    /// Alsa PCM get available frames
    fn snd_pcm_avail(pcm_handle: *mut OpaqueStructure) -> c_long;

    /// Alsa PCM write interleaved data
    fn snd_pcm_writei(
        pcm_handle: *mut OpaqueStructure,
        frame_buffer: *const c_void,
        num_frames: c_ulong,
    ) -> c_long;

    // Hw Parameter Functions
    fn snd_pcm_hw_params_malloc(handle_ptr: *mut *mut OpaqueStructure) -> c_int;
    fn snd_pcm_hw_params_free(pcm_handle: *mut OpaqueStructure) -> c_int;

    fn snd_pcm_hw_params_any(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
    ) -> c_int;

    fn snd_pcm_hw_params_current(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
    ) -> c_int;

    fn snd_pcm_hw_params_set_rate(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        rate: c_uint,
        direction: PcmHwParamDirection,
    ) -> c_int;

    fn snd_pcm_hw_params_set_format(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        format: PcmHwParamFormat,
    ) -> c_int;

    fn snd_pcm_hw_params_set_access(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        access: PcmHwParamAccess,
    ) -> c_int;

    fn snd_pcm_hw_params_set_channels(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        channel_count: c_uint,
    ) -> c_int;

    fn snd_pcm_hw_params_set_period_size_near(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        period: *mut c_ulong,
        direction: *mut PcmHwParamDirection,
    ) -> c_int;

    fn snd_pcm_hw_params_set_buffer_size_near(
        pcm_handle: *mut OpaqueStructure,
        hw_params_handle: *mut OpaqueStructure,
        buffer: *mut c_ulong,
    ) -> c_int;

    // Sw Parameter Functions
    fn snd_pcm_sw_params_malloc(handle_ptr: *mut *mut OpaqueStructure) -> c_int;
    fn snd_pcm_sw_params_free(pcm_handle: *mut OpaqueStructure) -> c_int;

    fn snd_pcm_sw_params_current(
        pcm_handle: *mut OpaqueStructure,
        sw_params_handle: *mut OpaqueStructure,
    ) -> c_int;

}

impl Error {
    pub(super) fn from_errnum(errnum: i32) -> Self {
        unsafe {
            let error_str = CStr::from_ptr(snd_strerror(errnum));
            match error_str.to_str() {
                Ok(s) => Error::Generic((errnum, s.to_string())),
                Err(_) => Error::StringCreation(errnum),
            }
        }
    }
}

impl Pcm {
    pub(super) fn new_from_default_playback() -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let cname = CString::new("default").unwrap();
        let errnum = unsafe { snd_pcm_open(&mut handle, cname.as_ptr(), PcmStream::Playback, 0) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        //println!("Hw Parameters Pointer: {:?}", handle);
        Ok(Pcm {
            handle,
            is_capture: false,
        })
    }

    pub(super) fn new_from_default_capture() -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let cname = CString::new("default").unwrap();
        let errnum = unsafe { snd_pcm_open(&mut handle, cname.as_ptr(), PcmStream::Playback, 0) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(Pcm {
            handle,
            is_capture: true,
        })
    }

    pub(super) fn new_from_default(stream: PcmStream) -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let cname = CString::new("default").unwrap();
        let errnum = unsafe { snd_pcm_open(&mut handle, cname.as_ptr(), PcmStream::Playback, 0) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        let is_capture = match stream {
            PcmStream::Playback => false,
            PcmStream::Capture => true,
        };
        Ok(Pcm { handle, is_capture })
    }

    pub(super) fn get_state(&self) -> PcmState {
        let state_value = unsafe { snd_pcm_state(self.handle) };
        PcmState::from_u32(state_value)
    }

    pub(super) fn set_hw_params(&self, hw_params: &PcmHwParams) -> Result<(), Error> {
        // In Future check if this is even allowed based on the pcm state
        let errnum = unsafe { snd_pcm_hw_params(self.handle, hw_params.handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(())
    }

    pub(super) fn start(&self) -> Result<(), Error> {
        // In Future check if this is even allowed based on the pcm state
        let errnum = unsafe { snd_pcm_start(self.handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(())
    }

    pub(super) fn stop(&self) -> Result<(), Error> {
        // In Future check if this is even allowed based on the pcm state
        let errnum = unsafe { snd_pcm_drop(self.handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(())
    }

    pub(super) fn wait_until_ready(&self, timeout: i32) -> Result<bool, Error> {
        // In Future check if this is even allowed based on the pcm state
        let status = unsafe { snd_pcm_wait(self.handle, timeout as c_int) };
        if status < 0 {
            Err(Error::from_errnum(status))
        } else if status == 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub(super) fn get_available_frames(&self) -> i64 {
        unsafe { snd_pcm_avail(self.handle) }
    }

    pub(super) fn write_interleaved_float_frames(
        &self,
        data: &[f32],
        num_frames: u64,
    ) -> Result<u64, Error> {
        // In Future check if this is even allowed based on the pcm config
        let res =
            unsafe { snd_pcm_writei(self.handle, data.as_ptr() as *const c_void, num_frames) };
        if res < 0 {
            return Err(Error::from_errnum(res as i32));
        }
        Ok(res as u64)
    }
}

impl Drop for Pcm {
    fn drop(&mut self) {
        unsafe {
            // Error Not Currently Handled:
            snd_pcm_close(self.handle);
        }
    }
}

impl<'a> PcmHwParams<'a> {
    pub(super) fn any_from_pcm(pcm: &'a Pcm) -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let errnum = unsafe { snd_pcm_hw_params_malloc(&mut handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        //println!("Hw Parameters Pointer: {:?}", handle);
        let errnum = unsafe { snd_pcm_hw_params_any(pcm.handle, handle) };
        // Can safely...? give an errnum/status of 1
        if (errnum != 0) && (errnum != 1) {
            return Err(Error::from_errnum(errnum));
        }
        Ok(PcmHwParams {
            handle,
            pcm_link: pcm,
        })
    }

    pub(super) fn current_from_pcm(pcm: &'a Pcm) -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let errnum = unsafe { snd_pcm_hw_params_malloc(&mut handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }

        let errnum = unsafe { snd_pcm_hw_params_current(pcm.handle, handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(PcmHwParams {
            handle,
            pcm_link: pcm,
        })
    }

    pub(super) fn set_param(&self, param: PcmHwParam) -> Result<(), Error> {
        //println!("Hw Parameters Pointer: {:?}", self.handle);
        match param {
            PcmHwParam::NearestRate(rate) => {
                let errnum = unsafe {
                    snd_pcm_hw_params_set_rate(
                        self.pcm_link.handle,
                        self.handle,
                        rate,
                        PcmHwParamDirection::Nearest,
                    )
                };
                //println!("errnum: {}", errnum);
                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
            PcmHwParam::FormatFloat => {
                #[cfg(target_endian = "little")]
                let format = PcmHwParamFormat::FloatLE;
                #[cfg(target_endian = "big")]
                let format = PcmHwParamFormat::FloatBE;

                let errnum = unsafe {
                    snd_pcm_hw_params_set_format(self.pcm_link.handle, self.handle, format)
                };
                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
            PcmHwParam::BufferInterleaved => {
                let errnum = unsafe {
                    snd_pcm_hw_params_set_access(
                        self.pcm_link.handle,
                        self.handle,
                        PcmHwParamAccess::RWInterleaved,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
            PcmHwParam::Channels(channel_count) => {
                let errnum = unsafe {
                    snd_pcm_hw_params_set_channels(self.pcm_link.handle, self.handle, channel_count)
                };
                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
            PcmHwParam::NearestPeriod(mut period) => {
                let errnum = unsafe {
                    snd_pcm_hw_params_set_period_size_near(
                        self.pcm_link.handle,
                        self.handle,
                        &mut period,
                        &mut PcmHwParamDirection::Nearest,
                    )
                };
                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
            PcmHwParam::NearestBufferSize(mut buffer_size) => {
                let errnum = unsafe {
                    snd_pcm_hw_params_set_buffer_size_near(
                        self.pcm_link.handle,
                        self.handle,
                        &mut buffer_size,
                    )
                };

                if errnum != 0 {
                    return Err(Error::from_errnum(errnum));
                }
            }
        }
        Ok(())
    }
}

impl<'a> Drop for PcmHwParams<'a> {
    fn drop(&mut self) {
        unsafe {
            // Error Not Currently Handled:
            snd_pcm_hw_params_free(self.handle);
        }
    }
}

impl<'a> PcmSwParams<'a> {
    pub(super) fn current_from_pcm(pcm: &'a Pcm) -> Result<Self, Error> {
        let mut handle = ptr::null_mut();
        let errnum = unsafe { snd_pcm_sw_params_malloc(&mut handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }

        let errnum = unsafe { snd_pcm_sw_params_current(pcm.handle, handle) };
        if errnum != 0 {
            return Err(Error::from_errnum(errnum));
        }
        Ok(PcmSwParams {
            handle,
            pcm_link: pcm,
        })
    }
}

impl<'a> Drop for PcmSwParams<'a> {
    fn drop(&mut self) {
        unsafe {
            // Error Not Currently Handled:
            snd_pcm_sw_params_free(self.handle);
        }
    }
}
