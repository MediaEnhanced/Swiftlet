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

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Media::Audio::{
    IAudioCaptureClient, IAudioClient3, IAudioRenderClient, IMMDevice, ISimpleAudioVolume,
};
use windows::Win32::System::Com::CoTaskMemFree;

use std::ffi::c_void;
use std::fmt::Debug;
use std::mem::size_of;
use std::ptr;
use windows::Win32::Foundation;
//use windows::Win32::Foundation::BOOL;
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
//IUnknown
use windows::Win32::Media::{Audio, KernelStreaming::WAVE_FORMAT_EXTENSIBLE, Multimedia};
use windows::Win32::System::{Com, Threading};

pub(super) struct ComOwner;

impl ComOwner {
    pub(super) fn new() -> Option<Self> {
        unsafe {
            if Com::CoInitializeEx(
                Some(ptr::null()),
                Com::COINIT_MULTITHREADED | Com::COINIT_DISABLE_OLE1DDE,
            )
            .is_err()
            {
                return None;
            }

            Some(ComOwner {})
        }
    }
}

impl Drop for ComOwner {
    fn drop(&mut self) {
        unsafe {
            Com::CoUninitialize();
        }
    }
}

#[derive(Debug)]
enum FoundationError {
    Uncertain,
    WaitFailed,
    WaitAbandoned,
    GetBuffer,
}

struct Device {
    device: IMMDevice,
    manager: IAudioClient3,
    channels: u32,
    channel_mask: u32,
    event: HANDLE,
    buffer_size: u32,
    frame_period: u32,
}

impl Device {
    fn new(is_capture: bool, period: u32, desired_channels: u16) -> Option<Self> {
        let device_enum = match unsafe {
            Com::CoCreateInstance::<_, Audio::IMMDeviceEnumerator>(
                &Audio::MMDeviceEnumerator,
                None,
                Com::CLSCTX_ALL,
            )
        } {
            Ok(de) => de,
            Err(_) => return None,
        };

        let dataflow = match is_capture {
            false => Audio::eRender,
            true => Audio::eCapture,
        };

        let device = match unsafe { device_enum.GetDefaultAudioEndpoint(dataflow, Audio::eConsole) }
        {
            Ok(d) => d,
            Err(_) => return None,
        };

        // process loopback...?
        let manager =
            match unsafe { device.Activate::<Audio::IAudioClient3>(Com::CLSCTX_ALL, None) } {
                Ok(m) => m,
                Err(_) => return None,
            };

        let output_category = Audio::AudioCategory_Media;
        let properties = match unsafe { manager.IsOffloadCapable(output_category) } {
            Ok(b) => Audio::AudioClientProperties {
                cbSize: size_of::<Audio::AudioClientProperties>() as u32,
                bIsOffload: b,
                eCategory: output_category,
                Options: Audio::AUDCLNT_STREAMOPTIONS::default(),
            },
            Err(_) => return None,
        };

        if unsafe { manager.SetClientProperties(&properties) }.is_err() {
            return None;
        }

        let mix_format = match unsafe { manager.GetMixFormat() } {
            Ok(format) => {
                let format_tag = unsafe { (*format).wFormatTag };
                if format_tag as u32 != WAVE_FORMAT_EXTENSIBLE {
                    unsafe { CoTaskMemFree(Some(format as *const c_void)) };
                    return None;
                }

                let format_ext = format as *mut Audio::WAVEFORMATEXTENSIBLE;
                if let Some(format_safe) = unsafe { format_ext.as_mut() } {
                    format_safe
                } else {
                    unsafe { CoTaskMemFree(Some(format as *const c_void)) };
                    return None;
                }
            }
            Err(_) => return None,
        };

        let subformat_test = mix_format.SubFormat;
        if subformat_test != Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {
            mix_format.SubFormat = Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
            mix_format.Format.wBitsPerSample = 32;
            mix_format.Format.nBlockAlign = mix_format.Format.nChannels * 4;
        }

        if mix_format.Format.nSamplesPerSec != 48000 {
            mix_format.Format.nSamplesPerSec = 48000;
            mix_format.Format.nAvgBytesPerSec = 48000 * (mix_format.Format.nChannels as u32) * 4;
        }

        if desired_channels > 0 && mix_format.Format.nChannels != desired_channels {
            mix_format.Format.nChannels = desired_channels;
            mix_format.Format.nAvgBytesPerSec = 48000 * (desired_channels as u32) * 4;
            mix_format.Format.nBlockAlign = desired_channels * 4;
        }

        let mut closest_format = ptr::null_mut();
        let mix_format_ptr = mix_format as *const Audio::WAVEFORMATEXTENSIBLE;
        let final_format = match unsafe {
            manager.IsFormatSupported(
                Audio::AUDCLNT_SHAREMODE_SHARED,
                mix_format_ptr as *const Audio::WAVEFORMATEX,
                Some(&mut closest_format),
            )
        } {
            Foundation::S_OK => {
                //println!("Got Exact Format Match!");
                unsafe {
                    Com::CoTaskMemFree(Some(closest_format as *const c_void));
                }
                mix_format
            }
            Foundation::S_FALSE => {
                //println!("Got Closest Matching!");
                unsafe {
                    Com::CoTaskMemFree(Some(mix_format_ptr as *const c_void));
                }
                let format_tag = unsafe { (*closest_format).wFormatTag };
                if format_tag as u32 != WAVE_FORMAT_EXTENSIBLE {
                    unsafe { CoTaskMemFree(Some(closest_format as *const c_void)) };
                    return None;
                }

                let format_ext = closest_format as *mut Audio::WAVEFORMATEXTENSIBLE;
                if let Some(format_safe) = unsafe { format_ext.as_mut() } {
                    format_safe
                } else {
                    unsafe { CoTaskMemFree(Some(closest_format as *const c_void)) };
                    return None;
                }
            }
            Audio::AUDCLNT_E_UNSUPPORTED_FORMAT => return None,
            _ => {
                println!("Unsupported Format!");
                return None;
            }
        };

        // Check Period
        let final_format_ptr = final_format as *const Audio::WAVEFORMATEXTENSIBLE;
        let mut shared_format = ptr::null_mut();
        let mut current_period = 0;
        match unsafe {
            manager.GetCurrentSharedModeEnginePeriod(&mut shared_format, &mut current_period)
        } {
            Ok(_) => {
                unsafe { CoTaskMemFree(Some(shared_format as *const c_void)) };

                if current_period != period {
                    let mut default_period_in_frames = 0;
                    let mut fundamental_period_in_frames = 0;
                    let mut min_period_in_frames = 0;
                    let mut max_period_in_frames = 0;

                    match unsafe {
                        manager.GetSharedModeEnginePeriod(
                            final_format_ptr as *const Audio::WAVEFORMATEX,
                            &mut default_period_in_frames,
                            &mut fundamental_period_in_frames,
                            &mut min_period_in_frames,
                            &mut max_period_in_frames,
                        )
                    } {
                        Ok(_) => {
                            if (min_period_in_frames > period) || (max_period_in_frames < period) {
                                return None;
                            }
                        }
                        Err(_) => return None,
                    }
                }
            }
            Err(_) => return None,
        }

        if unsafe {
            manager.InitializeSharedAudioStream(
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                period,
                final_format_ptr as *const Audio::WAVEFORMATEX,
                None,
            )
        }
        .is_err()
        {
            return None;
        }

        // Get Channels and Channel Mask BEFORE deallocating the final_format
        let channels = final_format.Format.nChannels as u32;
        let channel_mask = final_format.dwChannelMask;
        unsafe { CoTaskMemFree(Some(final_format_ptr as *const c_void)) };

        let event = match unsafe {
            Threading::CreateEventW(
                None,
                Foundation::BOOL::from(false),
                Foundation::BOOL::from(false),
                PCWSTR(ptr::null()),
            )
        } {
            Ok(e) => e,
            Err(_) => return None,
        };

        if unsafe { manager.SetEventHandle(event) }.is_err() {
            return None;
        }

        let buffer_size = match unsafe { manager.GetBufferSize() } {
            Ok(bs) => {
                if bs < period {
                    return None;
                }
                bs
            }
            Err(_) => return None,
        };

        Some(Device {
            device,
            manager,
            channels,
            channel_mask,
            event,
            buffer_size,
            frame_period: period,
        })
    }
}

pub(super) struct OutputDevice {
    device: Device,
    writer: IAudioRenderClient,
    //reader: IAudioCaptureClient,
    volume_control: ISimpleAudioVolume,
}

impl OutputDevice {
    pub(super) fn new(period: u32) -> Option<Self> {
        let device = Device::new(false, period, 0)?;

        let writer = match unsafe { device.manager.GetService() } {
            Ok(w) => w,
            Err(_) => return None,
        };

        let volume_control = match unsafe { device.manager.GetService() } {
            Ok(vc) => vc,
            Err(_) => return None,
        };

        Some(OutputDevice {
            device,
            writer,
            volume_control,
        })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.device.channels
    }

    fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation

        let num_frames = match unsafe { self.device.manager.GetCurrentPadding() } {
            Ok(f) => f,
            Err(_) => return false,
        };

        //println!("Initial frames: {}", num_frames);

        match unsafe { self.writer.GetBuffer(num_frames) } {
            Ok(_) => {
                if unsafe {
                    self.writer
                        .ReleaseBuffer(num_frames, Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)
                }
                .is_err()
                {
                    return false;
                }
            }
            Err(_) => return false,
        }

        unsafe { self.device.manager.Start() }.is_ok()
    }

    fn stop(&self) -> bool {
        unsafe { self.device.manager.Stop() }.is_ok()
    }

    fn wait_for_next_output(
        &self,
        millisecond_timeout: u32,
    ) -> Result<Option<&mut [f32]>, FoundationError> {
        match unsafe { Threading::WaitForSingleObject(self.device.event, millisecond_timeout) } {
            Foundation::WAIT_OBJECT_0 => {
                //println!("Wait Finished!");
            }
            Foundation::WAIT_TIMEOUT => {
                return Ok(None);
            }
            Foundation::WAIT_FAILED => {
                // Additional info with GetLastError
                return Err(FoundationError::WaitFailed);
            }
            Foundation::WAIT_ABANDONED => {
                return Err(FoundationError::WaitAbandoned);
            }
            _ => return Err(FoundationError::Uncertain),
        }

        match unsafe { self.writer.GetBuffer(self.device.frame_period) } {
            Ok(b) => {
                let num_floats = self.device.frame_period * self.device.channels;
                let buffer =
                    unsafe { std::slice::from_raw_parts_mut(b as *mut f32, num_floats as usize) };
                Ok(Some(buffer))
            }
            Err(_e) => Err(FoundationError::GetBuffer),
        }
    }

    fn release_output(&self) -> bool {
        // Handle different flags in future
        unsafe { self.writer.ReleaseBuffer(self.device.frame_period, 0) }.is_ok()
    }

    pub(super) fn run_output_event_loop(&self, mut callback: impl crate::OutputCallback) -> bool {
        if !self.start() {
            return false;
        }
        loop {
            match self.wait_for_next_output(15) {
                Ok(Some(buffer)) => {
                    let callback_quit = callback.output_callback(buffer);
                    if !self.release_output() {
                        return false;
                    }
                    if callback_quit {
                        break;
                    }
                }
                Ok(None) => {
                    // Timeout here
                }
                Err(e) => {
                    println!("Output Wait Error: {:?}", e);
                }
            }
        }

        self.stop()
    }
}

pub(super) struct InputDevice {
    device: Device,
    reader: IAudioCaptureClient,
}

impl InputDevice {
    pub(super) fn new(period: u32, channels: u32) -> Option<Self> {
        let device = Device::new(true, period, channels as u16)?;

        let reader = match unsafe { device.manager.GetService() } {
            Ok(w) => w,
            Err(_) => return None,
        };

        Some(InputDevice { device, reader })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.device.channels
    }

    fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        unsafe { self.device.manager.Start() }.is_ok()
    }

    fn stop(&self) -> bool {
        unsafe { self.device.manager.Stop() }.is_ok()
    }

    fn wait_for_next_input(
        &self,
        millisecond_timeout: u32,
    ) -> Result<Option<&[f32]>, FoundationError> {
        match unsafe { Threading::WaitForSingleObject(self.device.event, millisecond_timeout) } {
            Foundation::WAIT_OBJECT_0 => {
                //println!("Wait Finished!");
            }
            Foundation::WAIT_TIMEOUT => {
                return Ok(None);
            }
            Foundation::WAIT_FAILED => {
                // Additional info with GetLastError
                return Err(FoundationError::WaitFailed);
            }
            Foundation::WAIT_ABANDONED => {
                return Err(FoundationError::WaitAbandoned);
            }
            _ => return Err(FoundationError::Uncertain),
        }

        let mut buffer_ptr = ptr::null_mut();
        let mut num_frames = self.device.frame_period;
        let mut flags = 0;
        match unsafe {
            self.reader
                .GetBuffer(&mut buffer_ptr, &mut num_frames, &mut flags, None, None)
        } {
            Ok(_) => {
                let num_floats = num_frames * self.device.channels;
                let buffer = unsafe {
                    std::slice::from_raw_parts(buffer_ptr as *mut f32, num_floats as usize)
                };
                Ok(Some(buffer))
            }
            Err(_) => Err(FoundationError::GetBuffer),
        }
    }

    fn release_input(&self) -> bool {
        // Handle different flags in future
        unsafe { self.reader.ReleaseBuffer(self.device.frame_period) }.is_ok()
    }

    pub(super) fn run_input_event_loop(&self, input_trait: &mut impl crate::InputTrait) -> bool {
        if !self.start() {
            return false;
        }
        loop {
            match self.wait_for_next_input(15) {
                Ok(Some(buffer)) => {
                    let callback_quit = input_trait.callback(buffer);
                    if !self.release_input() {
                        return false;
                    }
                    if callback_quit {
                        break;
                    }
                }
                Ok(None) => {
                    // Timeout here
                }
                Err(e) => {
                    println!("Input Wait Error: {:?}", e);
                }
            }
        }

        self.stop()
    }

    pub(super) fn run_input_event_loop2(&self, input_trait: &mut impl crate::InputTrait) -> bool {
        if !self.start() {
            return false;
        }
        let mut buffer_convert = vec![0.0; 480];
        loop {
            match self.wait_for_next_input(15) {
                Ok(Some(buffer)) => {
                    for ind in 0..480 {
                        buffer_convert[ind] = buffer[ind << 1];
                    }
                    let callback_quit = input_trait.callback(&buffer_convert);
                    if !self.release_input() {
                        return false;
                    }
                    if callback_quit {
                        break;
                    }
                }
                Ok(None) => {
                    // Timeout here
                }
                Err(e) => {
                    println!("Input Wait Error: {:?}", e);
                }
            }
        }

        self.stop()
    }
}
