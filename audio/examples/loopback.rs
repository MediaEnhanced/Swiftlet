//Media Enhanced Swiftlet Cross-Compile Friendly Audio Loopback Example
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

use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

const CALLBACK_MAX_COUNT: u64 = 500; // 500 10ms sections (480 frames @ 48kHz) = 5 seconds of callback runtime

fn main() -> std::io::Result<()> {
    println!("Audio Loopback Starting!");
    let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel(4);

    let output = Output::new(sync_rx);
    let input = Input::new(sync_tx);

    match swiftlet_audio::run_input_output(480, 2, 1, output, input) {
        Ok(_) => println!("Finished Loopback!"),
        Err(e) => println!("Audio Error: {:?}", e),
    }

    Ok(())
}

struct Output {
    callback_count: u64,
    sync_rx: Receiver<Vec<f32>>,
}

impl Output {
    fn new(sync_rx: Receiver<Vec<f32>>) -> Self {
        Output {
            callback_count: 0,
            sync_rx,
        }
    }
}

impl swiftlet_audio::OutputCallback for Output {
    fn output_callback(&mut self, samples: &mut [f32]) -> bool {
        self.callback_count += 1;

        let samples_len = samples.len();
        if samples_len != 960 {
            println!("{}, Output Samples: {}", self.callback_count, samples_len);
            if samples_len == 0 {
                return true;
            }
            samples.fill(0.0);
        } else {
            match self.sync_rx.try_recv() {
                Ok(data) => samples.copy_from_slice(&data),
                Err(TryRecvError::Empty) => {
                    samples.fill(0.0);
                }
                Err(_e) => {
                    //return true;
                }
            }
        }

        self.callback_count >= CALLBACK_MAX_COUNT
    }
}

struct Input {
    callback_count: u64,
    sync_tx: SyncSender<Vec<f32>>,
}

impl Input {
    fn new(sync_tx: SyncSender<Vec<f32>>) -> Self {
        Input {
            callback_count: 0,
            sync_tx,
        }
    }
}

impl swiftlet_audio::InputCallback for Input {
    fn input_callback(&mut self, samples: &[f32]) -> bool {
        self.callback_count += 1;

        let samples_len = samples.len();
        if samples_len != 480 {
            println!("{}, Input Samples: {}", self.callback_count, samples_len);
            if samples_len == 0 {
                return true;
            }
        } else {
            let mut data = Vec::with_capacity(960);
            // Inefficient but simple:
            for s in samples {
                data.push(*s);
                data.push(*s);
            }
            if self.sync_tx.send(data).is_err() {
                return true;
            }
        }

        self.callback_count >= CALLBACK_MAX_COUNT
    }
}
