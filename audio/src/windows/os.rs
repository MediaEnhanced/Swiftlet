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

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows::Win32::Foundation;
//use windows::Win32::Foundation::BOOL;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Media::Audio::AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
//IUnknown
use windows::Win32::Media::{Audio, KernelStreaming::WAVE_FORMAT_EXTENSIBLE, Multimedia};
use windows::Win32::System::{Com, Threading};

fn cmp_guid(a: &GUID, b: &GUID) -> bool {
    //println!("GUID A: {:#x}, {:#x}, {:#x}", a.data1, a.data2, a.data3);
    //println!("GUID B: {:#x}, {:#x}, {:#x}", b.data1, b.data2, b.data3);
    (a.data1, a.data2, a.data3, a.data4) == (b.data1, b.data2, b.data3, b.data4)
}

#[derive(Debug)]
pub(super) enum Error {
    Uncertain,
    WaitFailed,
    WaitAbandoned,
    GetBuffer,
}

pub(super) struct AudioDevice {
    enumerator: Audio::IMMDeviceEnumerator,
}

impl AudioDevice {
    pub(super) fn new() -> Option<Self> {
        unsafe {
            if Com::CoInitializeEx(
                Some(null()),
                Com::COINIT_MULTITHREADED | Com::COINIT_DISABLE_OLE1DDE,
            )
            .is_err()
            {
                return None;
            }
            match Com::CoCreateInstance::<_, Audio::IMMDeviceEnumerator>(
                &Audio::MMDeviceEnumerator,
                None,
                Com::CLSCTX_ALL,
            ) {
                Ok(enumerator) => Some(AudioDevice { enumerator }),
                Err(_) => None,
            }
        }
    }
}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        // drop(self.input);
        // drop(self.output);
        // drop(self.enumerator);
        unsafe {
            Com::CoUninitialize();
        }
    }
}

pub(super) struct AudioOutput {
    device: Audio::IMMDevice,
    manager: Audio::IAudioClient3,
    channels: u32,
    channel_mask: u32,
    writer: Audio::IAudioRenderClient,
    event: Foundation::HANDLE,
    buffer_size: u32,
    frame_period: u32,
    volume_control: Audio::ISimpleAudioVolume,
}

impl AudioOutput {
    pub(super) fn new(audio_device: &AudioDevice, desired_period: u32) -> Option<Self> {
        unsafe {
            let device = match audio_device
                .enumerator
                .GetDefaultAudioEndpoint(Audio::eRender, Audio::eConsole)
            {
                Ok(d) => d,
                Err(_) => return None,
            };

            // process loopback...?
            let manager = match device.Activate::<Audio::IAudioClient3>(Com::CLSCTX_ALL, None) {
                Ok(m) => m,
                Err(_) => return None,
            };

            let output_category = Audio::AudioCategory_Media;
            let properties = match manager.IsOffloadCapable(output_category) {
                Ok(b) => Audio::AudioClientProperties {
                    cbSize: size_of::<Audio::AudioClientProperties>() as u32,
                    bIsOffload: b,
                    eCategory: output_category,
                    Options: Audio::AUDCLNT_STREAMOPTIONS::default(),
                },
                Err(_) => return None,
            };

            if manager.SetClientProperties(&properties).is_err() {
                return None;
            }

            let (channels, channel_mask) = match manager.GetMixFormat() {
                Ok(format) => {
                    if ((*format).wFormatTag as u32) != WAVE_FORMAT_EXTENSIBLE {
                        return None;
                    }

                    // Convert pointer types and try stuff
                    let format_ext = format as *mut Audio::WAVEFORMATEXTENSIBLE;
                    let format_guid = (*format_ext).SubFormat;
                    if !cmp_guid(&format_guid, &Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) {
                        println!("Trying different Audio Output Format!");
                        (*format_ext).SubFormat.data1 =
                            Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.data1;
                        (*format_ext).SubFormat.data2 =
                            Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.data2;
                        (*format_ext).SubFormat.data3 =
                            Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.data3;
                        (*format_ext).SubFormat.data4 =
                            Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.data4;
                        (*format_ext).Format.wBitsPerSample = 32;
                        (*format_ext).Format.nBlockAlign = ((*format_ext).Format.nChannels) * 4;
                    }

                    if (*format_ext).Format.nSamplesPerSec != 48000 {
                        println!("Trying different Sample Rate!");
                        (*format_ext).Format.nSamplesPerSec = 48000;
                        (*format_ext).Format.nAvgBytesPerSec =
                            48000 * ((*format_ext).Format.nChannels as u32) * 4;
                    }

                    let format_test = format_ext as *const Audio::WAVEFORMATEX;
                    let mut closest_match_p = null_mut();
                    let closest_match_p_convert =
                        &mut closest_match_p as *mut *mut Audio::WAVEFORMATEX;
                    match manager.IsFormatSupported(
                        Audio::AUDCLNT_SHAREMODE_SHARED,
                        format_test,
                        Some(closest_match_p_convert),
                    ) {
                        Foundation::S_OK => {
                            //println!("Format Found!");
                        }
                        Foundation::S_FALSE => {
                            println!("Got Closest Matching!");
                            let free_ptr = closest_match_p as *const c_void;
                            Com::CoTaskMemFree(Some(free_ptr));
                            return None;
                        }
                        Audio::AUDCLNT_E_UNSUPPORTED_FORMAT => return None,
                        _ => {
                            println!("Unsupported Format!");
                            return None;
                        }
                    }

                    let mut format_final = format_test as *mut Audio::WAVEFORMATEX;
                    let mut current_period: u32 = 0;
                    match manager.GetCurrentSharedModeEnginePeriod(
                        &mut format_final as *mut *mut Audio::WAVEFORMATEX,
                        &mut current_period as *mut u32,
                    ) {
                        Ok(_) => {
                            if current_period != desired_period {
                                let mut default_period_in_frames: u32 = 0;
                                let mut fundamental_period_in_frames: u32 = 0;
                                let mut min_period_in_frames: u32 = 0;
                                let mut max_period_in_frames: u32 = 0;

                                match manager.GetSharedModeEnginePeriod(
                                    format_test,
                                    &mut default_period_in_frames as *mut u32,
                                    &mut fundamental_period_in_frames as *mut u32,
                                    &mut min_period_in_frames as *mut u32,
                                    &mut max_period_in_frames as *mut u32,
                                ) {
                                    Ok(_) => {
                                        if (min_period_in_frames > desired_period)
                                            || (max_period_in_frames < desired_period)
                                        {
                                            return None;
                                        }
                                    }
                                    Err(_) => return None,
                                }
                            }
                        }
                        Err(_) => return None,
                    }

                    if manager
                        .InitializeSharedAudioStream(
                            AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                            desired_period,
                            format_test,
                            None,
                        )
                        .is_err()
                    {
                        return None;
                    }

                    let p_format_info = format_final as *const Audio::WAVEFORMATEXTENSIBLE;
                    let c = (*p_format_info).Format.nChannels as u32;
                    let c_mask = (*p_format_info).dwChannelMask;

                    let free_ptr = format as *const c_void;
                    Com::CoTaskMemFree(Some(free_ptr));
                    (c, c_mask)
                }
                Err(_) => return None,
            };

            let writer = match manager.GetService() {
                Ok(w) => w,
                Err(_) => return None,
            };

            let event = match Threading::CreateEventW(
                None,
                Foundation::BOOL::from(false),
                Foundation::BOOL::from(false),
                PCWSTR(null()),
            ) {
                Ok(e) => e,
                Err(_) => return None,
            };

            if manager.SetEventHandle(event).is_err() {
                return None;
            }

            let buffer_size = match manager.GetBufferSize() {
                Ok(bs) => {
                    if bs < desired_period {
                        return None;
                    }
                    bs
                }
                Err(_) => return None,
            };

            let volume_control = match manager.GetService() {
                Ok(vc) => vc,
                Err(_) => return None,
            };

            let audio_output = AudioOutput {
                device,
                manager,
                channels,
                channel_mask,
                writer,
                event,
                buffer_size,
                frame_period: desired_period,
                volume_control,
            };

            Some(audio_output)
        }
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    pub(super) fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        unsafe {
            let num_frames = match self.manager.GetCurrentPadding() {
                Ok(f) => f,
                Err(_) => return false,
            };

            //println!("Initial frames: {}", num_frames);

            match self.writer.GetBuffer(num_frames) {
                Ok(_) => {
                    if self
                        .writer
                        .ReleaseBuffer(num_frames, Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)
                        .is_err()
                    {
                        return false;
                    }
                }
                Err(_) => return false,
            }

            self.manager.Start().is_ok()
        }
    }

    pub(super) fn stop(&self) -> bool {
        unsafe { self.manager.Stop().is_ok() }
    }

    pub(super) fn wait_for_next_output(
        &mut self,
        millisecond_timeout: u32,
    ) -> Result<Option<&mut [f32]>, Error> {
        unsafe {
            match Threading::WaitForSingleObject(self.event, millisecond_timeout) {
                Foundation::WAIT_OBJECT_0 => {
                    //println!("Wait Finished!");
                }
                Foundation::WAIT_TIMEOUT => {
                    return Ok(None);
                }
                Foundation::WAIT_FAILED => {
                    // Additional info with GetLastError
                    return Err(Error::WaitFailed);
                }
                Foundation::WAIT_ABANDONED => {
                    return Err(Error::WaitAbandoned);
                }
                _ => return Err(Error::Uncertain),
            }

            match self.writer.GetBuffer(self.frame_period) {
                Ok(b) => {
                    let num_floats = self.frame_period * self.channels;
                    let buffer = std::slice::from_raw_parts_mut(b as *mut f32, num_floats as usize);
                    Ok(Some(buffer))
                }
                Err(_) => Err(Error::GetBuffer),
            }
        }
    }

    pub(super) fn release_output(&self) -> bool {
        // Handle different flags in future
        unsafe { self.writer.ReleaseBuffer(self.frame_period, 0).is_ok() }
    }

    fn event_loop(&self, callback: &mut dyn FnMut(&mut [f32]) -> bool) -> bool {
        unsafe {
            if !self.start() {
                return false;
            }

            loop {
                match Threading::WaitForSingleObject(self.event, 15) {
                    Foundation::WAIT_OBJECT_0 => {
                        //println!("Event Triggered!");
                    }
                    Foundation::WAIT_TIMEOUT => {
                        println!("Wait Timeout!");
                        //return false;
                    }
                    Foundation::WAIT_FAILED => {
                        // Additional info with GetLastError
                        return false;
                    }
                    _ => return false, // Includes WAIT_ABANDONED
                }

                // Can use get padding to determine if frame period will fit into it in the future
                match self.writer.GetBuffer(self.frame_period) {
                    Ok(b) => {
                        let num_floats = self.frame_period * self.channels;
                        let float_p = b as *mut f32;
                        let buffer = std::slice::from_raw_parts_mut(float_p, num_floats as usize);

                        let callback_quit = callback(buffer);
                        if self.writer.ReleaseBuffer(self.frame_period, 0).is_err() {
                            return false;
                        }
                        if callback_quit {
                            break;
                        }
                    }
                    Err(_) => return false,
                }
            }
            self.stop();
            true
        }
    }
}

pub(super) struct AudioInput {
    device: Audio::IMMDevice,
}

impl AudioInput {
    //pub(super) fn new() -> Self {}
}
