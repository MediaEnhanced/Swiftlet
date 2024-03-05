//Media Enhanced Swiftlet Rust Realtime Media Internet Communications
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

#![allow(unused_imports)]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::communication::{
    AudioOutputThreadChannels, AudioStateMessage, ConsoleAudioCommands, NetworkAudioPackets,
    Receiver, Sender, TryRecvError,
};

use swiftlet_audio::opus::Decoder;
pub(crate) use swiftlet_audio::opus::OpusData;
use swiftlet_audio::AudioIO;

struct OutputState {
    callback_count: u64,
    prev_callback_time: Instant,
    playbacks: Vec<OutputPlayback>,
    playback_cleanup: Vec<usize>,
    realtimes: Vec<OutputRealtime>,
}

struct OutputPlayback {
    id: u64,
    probable_index: usize,
    is_stereo: bool,
    decoder: Decoder,
    data: OutputData,
    opus_data_next_packet: usize,
    opus_data_next_data: usize, // opus
}

struct OutputRealtime {
    id: u64,
    is_stereo: bool,
    decoder: Decoder,
    data_queue: VecDeque<OutputData>,
}

// Better alignment in the future?
struct OutputData {
    data: [f32; 1920],
    data_len: usize,
    read_offset: usize,
}

pub(crate) fn audio_thread(output_channels: AudioOutputThreadChannels) {
    let mut audio_io = match swiftlet_audio::AudioIO::new() {
        Ok(a) => a,
        Err(_) => {
            let _ = output_channels
                .debug_send
                .send("Could not create Audio IO!");
            return;
        }
    };

    match audio_io.create_output(480) {
        Some(2) => {
            let mut output_state = OutputState {
                callback_count: 0,
                prev_callback_time: Instant::now(),
                playbacks: Vec::new(),
                playback_cleanup: Vec::new(),
                realtimes: Vec::new(),
            };
            let mut opus_list = Vec::new();
            let mut f = move |samples: &mut [f32]| {
                output_callback(samples, &mut output_state, &mut opus_list, &output_channels)
            };

            audio_io.run_output_event_loop(&mut f);
        }
        Some(_) => {
            let _ = output_channels
                .debug_send
                .send("Wrong Output Channel Count!");
        }
        None => {
            let _ = output_channels
                .debug_send
                .send("Could not create Audio IO Output!");
        }
    }
}

fn output_callback(
    samples: &mut [f32],
    state: &mut OutputState,
    opus_list: &mut Vec<OpusData>,
    channels: &AudioOutputThreadChannels,
) -> bool {
    state.callback_count += 1;

    let samples_len = samples.len();
    if samples_len != 960 {
        let _ = channels
            .debug_send
            .send("Not the expected amount of samples!\n");
        if samples_len == 0 {
            // Quit Early
            return true;
        }
    }

    loop {
        match channels.command_recv.try_recv() {
            Err(TryRecvError::Empty) => break,
            Ok(command_recv) => match command_recv {
                ConsoleAudioCommands::LoadOpus(opus) => {
                    opus_list.push(opus);
                }
                ConsoleAudioCommands::PlayOpus(id) => {
                    for (index, opus_data) in opus_list.iter().enumerate() {
                        if opus_data.matches_id(id) {
                            let decoder = match Decoder::new(opus_data.is_stereo()) {
                                Ok(decoder) => decoder,
                                Err(err) => {
                                    let _ =
                                        channels.debug_send.send("Cannot Create Opus Decoder\n");
                                    return true;
                                }
                            };
                            let out_playback = OutputPlayback {
                                id,
                                probable_index: index,
                                is_stereo: opus_data.is_stereo(),
                                decoder,
                                data: OutputData {
                                    data: [0.0; 1920],
                                    data_len: 0,
                                    read_offset: 0,
                                },
                                opus_data_next_packet: 0,
                                opus_data_next_data: 0,
                            };
                            state.playbacks.push(out_playback);
                            break;
                        }
                    }
                }
            },
            Err(TryRecvError::Disconnected) => {
                let _ = channels
                    .debug_send
                    .send("Audio Command Recv Disconnected!!!\n");
                return true;
            }
        }
    }

    loop {
        match channels.packet_recv.try_recv() {
            Err(TryRecvError::Empty) => break,
            Ok(NetworkAudioPackets::MusicPacket((music_id, music_data))) => {
                if let Some(realtime_ind) =
                    state.realtimes.iter().position(|p| p.id == music_id as u64)
                {
                    let realtime = &mut state.realtimes[realtime_ind];

                    let mut output_data = OutputData {
                        data: [0.0; 1920],
                        data_len: 0,
                        read_offset: 0,
                    };

                    match realtime.decoder.decode_float(
                        &music_data[1..],
                        &mut output_data.data,
                        false,
                    ) {
                        Ok(decode_len) => {
                            output_data.data_len = decode_len;
                            if realtime.is_stereo {
                                output_data.data_len *= 2;
                            }
                        }
                        Err(_) => {
                            let _ = channels.debug_send.send("Opus Decode Issue\n");
                            return true;
                        }
                    }
                    realtime.data_queue.push_back(output_data);
                } else {
                    let is_stereo = music_data[0] > 0;
                    let mut decoder = match Decoder::new(is_stereo) {
                        Ok(decoder) => decoder,
                        Err(err) => {
                            let _ = channels.debug_send.send("Cannot Create Opus Decoder\n");
                            return true;
                        }
                    };
                    let mut output_data = OutputData {
                        data: [0.0; 1920],
                        data_len: 0,
                        read_offset: 0,
                    };
                    match decoder.decode_float(&music_data[1..], &mut output_data.data, false) {
                        Ok(decode_len) => {
                            output_data.data_len = decode_len;
                            if is_stereo {
                                output_data.data_len *= 2;
                            }
                        }
                        Err(_) => {
                            let _ = channels.debug_send.send("Opus Decode Issue\n");
                            return true;
                        }
                    }

                    let mut output_realtime = OutputRealtime {
                        id: music_id as u64,
                        is_stereo,
                        decoder,
                        data_queue: VecDeque::with_capacity(4),
                    };
                    output_realtime.data_queue.push_back(output_data);
                    state.realtimes.push(output_realtime);
                }
            }
            Ok(NetworkAudioPackets::MusicStop(music_id)) => {
                if let Some(realtime_ind) =
                    state.realtimes.iter().position(|p| p.id == music_id as u64)
                {
                    state.realtimes.remove(realtime_ind);
                }
            }
            Ok(_) => {
                //Nothing yet
            }
            Err(TryRecvError::Disconnected) => {
                let _ = channels
                    .debug_send
                    .send("Audio Packet Recv Disconnected!!!\n");
                return true;
            }
        }
    }

    // Samples are Left Right Interleaved for normal stereo stuff
    // Does NOT currently assume that the samples are zero to begin with
    samples.fill(0.0);

    for (playback_ind, playback) in state.playbacks.iter_mut().enumerate() {
        let opus_data = match opus_list[playback.probable_index].matches_id(playback.id) {
            true => &opus_list[playback.probable_index],
            false => {
                if let Some(new_index) = opus_list.iter().position(|od| od.matches_id(playback.id))
                {
                    playback.probable_index = new_index;
                    &opus_list[playback.probable_index]
                } else {
                    let _ = channels
                        .debug_send
                        .send("Could not find playback Opus Data!\n");
                    return true;
                }
            }
        };

        if playback.is_stereo {
            let mut samples_count = 0;
            loop {
                let mut readable_samples = playback.data.data_len - playback.data.read_offset;
                if readable_samples == 0 {
                    if let Some(input_data) = opus_data.get_input_slice(
                        playback.opus_data_next_packet,
                        playback.opus_data_next_data,
                    ) {
                        playback.opus_data_next_packet += 1;
                        playback.opus_data_next_data += input_data.len();
                        match playback.decoder.decode_float(
                            input_data,
                            &mut playback.data.data,
                            false,
                        ) {
                            Ok(decode_len) => {
                                playback.data.data_len = decode_len * 2;
                                playback.data.read_offset = 0;
                                readable_samples = decode_len * 2;
                            }
                            Err(err) => {
                                let _ = channels.debug_send.send("Opus Decode Issue\n");
                                return true;
                            }
                        }
                    } else {
                        state.playback_cleanup.push(playback_ind);
                        break;
                    }
                }

                let writeable_samples = samples_len - samples_count;
                if readable_samples >= writeable_samples {
                    //let next_read_offset = playback.data.read_offset + writeable_samples;
                    //samples[samples_count..].copy_from_slice(&playback.data.data[playback.data.read_offset..next_read_offset]);
                    //playback.data.read_offset = next_read_offset;
                    for (s_ind, s) in samples[samples_count..].iter_mut().enumerate() {
                        *s += playback.data.data[playback.data.read_offset + s_ind]
                    }
                    playback.data.read_offset += writeable_samples;
                    break;
                }

                // Else condition
                let next_samples_count = samples_count + readable_samples;
                //samples[samples_count..next_samples_count].copy_from_slice(&playback.data.data[playback.data.read_offset..]);
                for (s_ind, s) in samples[samples_count..next_samples_count]
                    .iter_mut()
                    .enumerate()
                {
                    *s += playback.data.data[playback.data.read_offset + s_ind]
                }
                playback.data.read_offset += readable_samples;
                samples_count = next_samples_count;
            }
        } else {
            // Not handled yet
        }
    }

    while let Some(ind) = state.playback_cleanup.pop() {
        //let _ = channels.debug_send.send("Playback Finished!\n");
        state.playbacks.remove(ind);
    }

    for realtime in state.realtimes.iter_mut() {
        if realtime.is_stereo {
            let mut samples_count = 0;
            loop {
                if let Some(output_data) = realtime.data_queue.front_mut() {
                    let readable_samples = output_data.data_len - output_data.read_offset;
                    let writeable_samples = samples_len - samples_count;
                    if readable_samples >= writeable_samples {
                        let next_read_offset = output_data.read_offset + writeable_samples;
                        //samples[samples_count..].copy_from_slice(&output_data.data[output_data.read_offset..next_read_offset]);
                        for (s_ind, s) in samples[samples_count..].iter_mut().enumerate() {
                            *s += output_data.data[output_data.read_offset + s_ind]
                        }
                        // Handle > case with error in future...?
                        if next_read_offset >= output_data.data_len {
                            realtime.data_queue.pop_front();
                        } else {
                            output_data.read_offset = next_read_offset;
                        }

                        break;
                    }
                    // Else condition
                    let next_samples_count = samples_count + readable_samples;
                    //samples[samples_count..next_samples_count].copy_from_slice(&output_data.data[output_data.read_offset..]);
                    for (s_ind, s) in samples[samples_count..next_samples_count]
                        .iter_mut()
                        .enumerate()
                    {
                        *s += output_data.data[output_data.read_offset + s_ind]
                    }
                    samples_count = next_samples_count;
                } else {
                    let _ = channels.debug_send.send("Realtime playback starved!\n");
                    break;
                }
            }
        } else {
            // Not handled yet
        }
    }

    false
}
