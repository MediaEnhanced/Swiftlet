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

#![allow(dead_code)] // Temporary

pub trait OutputCallback {
    fn output_callback(&mut self, samples: &mut [f32]) -> bool;
}

pub trait InputTrait {
    /// return true to start callback loop
    ///
    /// return false to quit input audio thread
    fn wait_to_start(&mut self) -> bool;

    /// will get called everytime there is new input audio samples
    ///
    /// return true to stop the callback loop at which point the
    /// wait_to_start function will be called
    fn callback(&mut self, samples: &[f32]) -> bool;

    /// provides a way for the application to receive input audio errors
    /// in order to handle them
    fn error(&mut self, _e: Error, _recoverable: bool) {}
}

#[cfg_attr(target_os = "windows", path = "windows/os.rs")]
#[cfg_attr(target_os = "linux", path = "linux/os.rs")]
#[cfg_attr(target_os = "macos", path = "mac/os.rs")]
mod os;
use os::{AudioInput, AudioOutput, AudioOwner};

pub mod raw;

#[cfg(feature = "opus")]
pub mod opus;

#[derive(Debug)]
pub enum Error {
    OwnerCreation,
    OutputCreation,
    InputCreation,
    OutputPlayback,
    InputCapture,
    ChannelMismatch,
    InputCallback,
}

/// Takes control of thread and calls the callback function with a fillable sample buffer every
/// desired_period with the expected_channel count
///
/// This function returns false if exited prematurely, otherwise true indicating that
/// the output was safely stopped after the callback returned true.
pub fn run_output(
    desired_period: u32,
    expected_channels: u32,
    callback: impl OutputCallback + 'static,
) -> Result<bool, Error> {
    let owner = match AudioOwner::new() {
        Some(d) => d,
        None => return Err(Error::OwnerCreation),
    };

    output_thread(&owner, desired_period, expected_channels, callback)
}

pub fn run_input_output(
    desired_period: u32,
    output_expected_channels: u32,
    input_expected_channels: u32,
    output_callback: impl OutputCallback + Send + 'static,
    input_trait: impl InputTrait + Send,
) -> Result<(), Error> {
    let owner = match AudioOwner::new() {
        Some(d) => d,
        None => return Err(Error::OwnerCreation),
    };

    std::thread::scope(|scope| {
        scope.spawn(|| {
            output_thread(
                &owner,
                desired_period,
                output_expected_channels,
                output_callback,
            )
        });
        scope.spawn(|| input_thread(&owner, desired_period, input_expected_channels, input_trait));
    });

    Ok(())
}

fn output_thread(
    owner: &AudioOwner,
    desired_period: u32,
    expected_channels: u32,
    callback: impl OutputCallback + 'static,
) -> Result<bool, Error> {
    let output = match AudioOutput::new(owner, desired_period) {
        Some(d) => d,
        None => return Err(Error::OutputCreation),
    };

    if output.get_channels() != expected_channels {
        return Err(Error::ChannelMismatch);
    }

    Ok(output.run_callback_loop(callback))
}

fn input_thread(
    owner: &AudioOwner,
    desired_period: u32,
    expected_channels: u32,
    mut input_trait: impl InputTrait,
) {
    loop {
        if input_trait.wait_to_start() {
            match AudioInput::new(owner, desired_period, expected_channels) {
                Some(input) => {
                    if input.get_channels() == expected_channels {
                        match input.run_callback_loop(&mut input_trait) {
                            true => {}
                            false => {
                                input_trait.error(Error::InputCallback, true);
                            }
                        }
                    } else if (input.get_channels() == 2) && (expected_channels == 1) {
                        match input.run_callback_loop2(&mut input_trait) {
                            true => {}
                            false => {
                                input_trait.error(Error::InputCallback, true);
                            }
                        }
                    } else {
                        input_trait.error(Error::ChannelMismatch, true);
                    }
                }
                None => {
                    input_trait.error(Error::InputCreation, true);
                }
            }
        } else {
            break;
        }
    }
}
