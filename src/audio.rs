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

use std::time::{Duration, Instant};

use crate::communication::{
    AudioOutputThreadChannels, AudioStateMessage, ConsoleAudioCommands, NetworkAudioPackets,
    Receiver, Sender, TryRecvError,
};
pub(crate) use cpal::Stream;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleFormat, StreamConfig,
};

enum OggPageHeaderAnalysisResult {
    InvalidPage,
    IdentificationHeader([u8; 4]),
    CommentHeader,
    AudioDataPage(u32),
    AudioDataPageContinuation(u32),
    AudioDataPageFinal(u32),
    AudioDataPageContinuationFinal(u32),
}

use OggPageHeaderAnalysisResult::*;
fn analyze_ogg_page_header(
    data: &[u8; 26],
    serial_num: Option<&[u8; 4]>,
) -> OggPageHeaderAnalysisResult {
    let capture_pattern = [b'O', b'g', b'g', b'S']; // Magic Number 0x5367674F
    if data[0..4] != capture_pattern {
        return InvalidPage;
    }
    if data[4] != 0 {
        return InvalidPage;
    }

    match data[5] {
        0 => {
            if let Some(comp) = serial_num {
                if &data[14..18] != comp {
                    // Bitstream Serial Number
                    return InvalidPage;
                }
                if data[6..14] == [0; 8] {
                    // Granule Position
                    if data[18..22] != [1, 0, 0, 0] {
                        // Page Sequence Number
                        return InvalidPage;
                    }
                    CommentHeader
                } else {
                    let mut page_sequence_num = data[18] as u32;
                    page_sequence_num |= (data[19] as u32) << 8;
                    page_sequence_num |= (data[20] as u32) << 16;
                    page_sequence_num |= (data[21] as u32) << 24;
                    AudioDataPage(page_sequence_num)
                }
            } else {
                InvalidPage
            }
        }
        1 => {
            if let Some(comp) = serial_num {
                if &data[14..18] != comp {
                    // Bitstream Serial Number
                    return InvalidPage;
                }
                if data[6..14] == [0; 8] {
                    // Granule Position
                    return InvalidPage;
                }
                let mut page_sequence_num = data[18] as u32;
                page_sequence_num |= (data[19] as u32) << 8;
                page_sequence_num |= (data[20] as u32) << 16;
                page_sequence_num |= (data[21] as u32) << 24;
                AudioDataPageContinuation(page_sequence_num)
            } else {
                InvalidPage
            }
        }
        2 => {
            if data[6..14] != [0; 8] {
                // Granule Position
                return InvalidPage;
            }
            if data[18..22] != [0; 4] {
                // Page Sequence Number
                return InvalidPage;
            }

            IdentificationHeader(data[14..18].try_into().unwrap()) // Bitstream Serial Number
        }
        4 => {
            if let Some(comp) = serial_num {
                if &data[14..18] != comp {
                    // Bitstream Serial Number
                    return InvalidPage;
                }
                if data[6..14] == [0; 8] {
                    // Granule Position
                    return InvalidPage;
                }
                let mut page_sequence_num = data[18] as u32;
                page_sequence_num |= (data[19] as u32) << 8;
                page_sequence_num |= (data[20] as u32) << 16;
                page_sequence_num |= (data[21] as u32) << 24;
                AudioDataPageFinal(page_sequence_num)
            } else {
                InvalidPage
            }
        }
        5 => {
            if let Some(comp) = serial_num {
                if &data[14..18] != comp {
                    // Bitstream Serial Number
                    return InvalidPage;
                }
                if data[6..14] == [0; 8] {
                    // Granule Position
                    return InvalidPage;
                }
                let mut page_sequence_num = data[18] as u32;
                page_sequence_num |= (data[19] as u32) << 8;
                page_sequence_num |= (data[20] as u32) << 16;
                page_sequence_num |= (data[21] as u32) << 24;
                AudioDataPageContinuationFinal(page_sequence_num)
            } else {
                InvalidPage
            }
        }
        _ => InvalidPage,
    }
}

pub(crate) struct OpusData {
    id: u64,
    stereo: bool, // 1 or 2 channels
    pre_skip: u16,
    output_gain: f32,
    packet_len: Vec<u16>,
    packet_data: Vec<u8>,
}

impl OpusData {
    pub(crate) fn convert_ogg_opus_file(data: &[u8], id: u64) -> Option<Self> {
        let mut index = 0;

        let first_page_result = match &data[index..index + 26].try_into() {
            Ok(ogg_page_header) => analyze_ogg_page_header(ogg_page_header, None),
            Err(err) => {
                return None;
            }
        };
        let serial_num = match first_page_result {
            IdentificationHeader(arr) => arr,
            _ => {
                //println!("First Header Error!");
                return None;
            }
        };

        index += 26;
        let mut remaining_bytes = data.len() - 26;
        if remaining_bytes < 2 {
            return None;
        }
        let page_segments = data[index];
        if page_segments != 1 {
            return None;
        }
        let segment_len = data[index + 1] as usize;

        index += 2;
        remaining_bytes -= 2;
        if remaining_bytes < segment_len {
            return None;
        }
        if segment_len < 19 {
            return None;
        }

        let opus_head_pattern = [b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd']; // Opus Head Magic Signature
        if data[index..index + 8] != opus_head_pattern {
            return None;
        }
        if data[index + 8] != 1 {
            // Opus Version
            return None;
        }
        let stereo = match data[index + 9] {
            1 => false,
            2 => true,
            _ => return None,
        };
        let mut pre_skip = data[index + 10] as u16;
        pre_skip |= (data[index + 11] as u16) << 8;
        let mut output_gain = data[index + 12] as i16;
        output_gain |= (data[index + 13] as i16) << 8;
        let output_gain = f32::powf(10.0, (output_gain as f32) / (5120.0));

        let mut opus_data = OpusData {
            id,
            stereo,
            pre_skip,
            output_gain,
            packet_len: Vec::new(),
            packet_data: Vec::new(),
        };

        index += segment_len;
        remaining_bytes -= segment_len;
        let second_page_result = match &data[index..index + 26].try_into() {
            Ok(ogg_page_header) => analyze_ogg_page_header(ogg_page_header, Some(&serial_num)),
            Err(err) => {
                return None;
            }
        };
        match second_page_result {
            CommentHeader => {}
            _ => {
                //println!("Second Header Error!");
                return None;
            }
        }

        index += 26;
        remaining_bytes -= 26;
        if remaining_bytes < 1 {
            return None;
        }
        let page_segments = data[index] as usize;

        index += 1;
        remaining_bytes -= 1;
        if remaining_bytes < page_segments {
            return None;
        }
        let mut comment_size = 0;
        for d in &data[index..index + page_segments] {
            comment_size += *d as usize;
        }

        index += page_segments;
        remaining_bytes -= page_segments;
        if remaining_bytes < comment_size {
            return None;
        }

        index += comment_size;
        remaining_bytes -= comment_size;
        let mut page_sequence_count = 2;
        let mut packet_length = 0;
        loop {
            let page_result = match &data[index..index + 26].try_into() {
                Ok(ogg_page_header) => analyze_ogg_page_header(ogg_page_header, Some(&serial_num)),
                Err(err) => {
                    return Some(opus_data);
                }
            };
            let (page_sequence_num, is_continuation, is_final) = match page_result {
                AudioDataPage(psn) => (psn, false, false),
                AudioDataPageContinuation(psn) => (psn, true, false),
                AudioDataPageFinal(psn) => (psn, false, true),
                AudioDataPageContinuationFinal(psn) => (psn, true, true),
                _ => return Some(opus_data),
            };
            if page_sequence_num != page_sequence_count {
                return Some(opus_data);
            }
            page_sequence_count += 1;
            if is_continuation != (packet_length > 0) {
                return Some(opus_data);
            }

            index += 26;
            remaining_bytes -= 26;
            if remaining_bytes < 1 {
                return Some(opus_data);
            }
            let page_segments = data[index] as usize;

            index += 1;
            remaining_bytes -= 1;
            if remaining_bytes < page_segments {
                return Some(opus_data);
            }

            let mut data_length = 0;
            for d in &data[index..index + page_segments] {
                data_length += *d as usize;

                packet_length += *d as u16;
                if *d != 255 {
                    opus_data.packet_len.push(packet_length);
                    packet_length = 0;
                }
            }

            index += page_segments;
            remaining_bytes -= page_segments;
            if remaining_bytes < data_length {
                return None; // Since I haven't added the data and there will be a mismatch
            }
            opus_data
                .packet_data
                .extend_from_slice(&data[index..index + data_length]);

            index += data_length;
            remaining_bytes -= data_length;
            if is_final {
                break;
            }
        }

        //println!("Remaining Bytes: {}", remaining_bytes);

        Some(opus_data)
    }

    pub(crate) fn to_data(&self) -> (u8, usize, usize, Vec<u8>) {
        let mut data = Vec::new();

        let stereo_byte = if self.stereo { 1 } else { 0 };

        //data.extend_from_slice(&self.packet_len.len().to_ne_bytes());
        for d in &self.packet_len {
            data.extend_from_slice(&u16::to_ne_bytes(*d));
        }
        //data.extend_from_slice(&self.packet_data.len().to_ne_bytes());
        data.extend_from_slice(&self.packet_data);

        (
            stereo_byte,
            self.packet_len.len(),
            self.packet_data.len(),
            data,
        )
    }

    pub(crate) fn add_to_vec(&self, data: &mut Vec<u8>) {
        if self.stereo {
            data.push(1);
        } else {
            data.push(0);
        }

        data.extend_from_slice(&self.packet_len.len().to_ne_bytes());
        for d in &self.packet_len {
            data.extend_from_slice(&u16::to_ne_bytes(*d));
        }
        data.extend_from_slice(&self.packet_data.len().to_ne_bytes());
        data.extend_from_slice(&self.packet_data);
    }
}

struct AudioOutputState {
    prev_callback_time: Instant,
    playbacks: Vec<AudioOutputPlayback>,
    cleanup: Vec<usize>,
}

struct AudioOutputPlayback {
    is_stereo: bool,
    decoder: opus::Decoder,
    decoded_data: [f32; 15360],
    decoded_data_count: usize,
    decoded_data_offset: usize,
    realtime_id: u8, // 0 is Non-realtime
    opus_data_index: usize,
    opus_data_next_packet: usize,
    opus_data_next_data: usize, // opus
}

pub(crate) fn start_audio_output(channels: AudioOutputThreadChannels) -> Option<Stream> {
    let state_send = channels.state_send.clone();
    let debug_send = channels.debug_send.clone();

    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        _ => {
            return None;
        }
    };

    let mut supported_configs_range = match device.supported_output_configs() {
        Ok(scr) => scr,
        Err(e) => {
            return None;
        }
    };

    let config = supported_configs_range
        .find(|c| c.max_sample_rate().0 == 48000 && c.min_sample_rate().0 == 48000);

    config.as_ref()?;

    let config = config.unwrap();

    if config.sample_format() != SampleFormat::F32 {
        let _ = debug_send.send("Supported Config Format is Not F32\n");
        return None;
    }

    if config.channels() != 2 {
        let _ = debug_send.send("Supported Config is Not Stereo\n");
        return None;
    }

    match config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            if 480 >= *min && 480 <= *max {
                let _ = debug_send.send("480 Buffer supported!\n");
            } else {
                let _ = debug_send.send("Supported Config is Buffer Unknown\n");
                return None;
            }
        }
        cpal::SupportedBufferSize::Unknown => {
            let _ = debug_send.send("Supported Config is Buffer Unknown\n");
            return None;
        }
    }

    let config = StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(480),
    };

    let mut output_state = AudioOutputState {
        prev_callback_time: Instant::now(),
        playbacks: Vec::new(),
        cleanup: Vec::new(),
    };
    let mut opus_list = Vec::new();

    let error_state_send = state_send.clone();
    let error_debug_send = debug_send.clone();

    let stream_result = device.build_output_stream(
        &config,
        move |samples, info| {
            audio_output_callback(samples, info, &mut output_state, &mut opus_list, &channels)
        },
        move |err| audio_output_error(err, &error_state_send, &error_debug_send),
        None,
    );

    match stream_result {
        Ok(stream) => match stream.play() {
            Ok(_) => {
                let _ = debug_send.send("Audio output callback function will be called!\n");
                Some(stream)
            }
            Err(err) => None,
        },
        Err(err) => {
            match err {
                cpal::BuildStreamError::DeviceNotAvailable => {
                    let _ = debug_send.send("Audio Output Device Not Available!\n");
                }
                cpal::BuildStreamError::StreamConfigNotSupported => {
                    let _ = debug_send.send("Audio Output Stream Config Not Supported!\n");
                }
                cpal::BuildStreamError::InvalidArgument => {
                    let _ = debug_send.send("Audio Output Device Creation Invalid Argument!\n");
                }
                cpal::BuildStreamError::StreamIdOverflow => {
                    let _ = debug_send.send("Audio Output Stream Config Not Supported!\n");
                }
                cpal::BuildStreamError::BackendSpecific { err } => {
                    let _ = debug_send.send("Audio Output Backend Specific!\n");
                }
            }
            None
        }
    }
}

fn audio_output_callback(
    samples: &mut [f32],
    info: &cpal::OutputCallbackInfo,
    state: &mut AudioOutputState,
    opus_list: &mut Vec<OpusData>,
    channels: &AudioOutputThreadChannels,
) {
    // Need to use info.timestamp().playback and samples.len to "fix" unsual audio callback timing situations in the future:

    // let curr_callback_time = Instant::now();
    // if curr_callback_time.duration_since(state.prev_callback_time) > Duration::from_millis(13) {
    // 	channels.debug_send.send("Audio output callback happened late!\n");
    // 	if samples.len() == 1920 {
    // 		channels.debug_send.send("20 ms buffer\n");
    // 	}
    // }
    // state.prev_callback_time = curr_callback_time;

    loop {
        match channels.command_recv.try_recv() {
            Err(try_recv_error) => match try_recv_error {
                TryRecvError::Empty => {
                    break;
                }
                TryRecvError::Disconnected => {
                    let _ = channels
                        .debug_send
                        .send("Audio Command Recv Disconnected!!!\n");
                    break;
                }
            },
            Ok(command_recv) => match command_recv {
                ConsoleAudioCommands::LoadOpus(opus) => {
                    opus_list.push(opus);
                }
                ConsoleAudioCommands::PlayOpus(id) => {
                    for (index, opus) in opus_list.iter().enumerate() {
                        if opus.id == id {
                            let opus_channels = match opus.stereo {
                                true => opus::Channels::Stereo,
                                false => opus::Channels::Mono,
                            };
                            let decoder = match opus::Decoder::new(48000, opus_channels) {
                                Ok(decoder) => decoder,
                                Err(err) => {
                                    let _ =
                                        channels.debug_send.send("Cannot Create Opus Decoder\n");
                                    break;
                                }
                            };
                            let audio_out_playback = AudioOutputPlayback {
                                is_stereo: opus.stereo,
                                decoder,
                                decoded_data: [0.0; 15360],
                                decoded_data_count: 0,
                                decoded_data_offset: 0,
                                realtime_id: 0,
                                opus_data_index: index,
                                opus_data_next_packet: 0,
                                opus_data_next_data: 0,
                            };
                            state.playbacks.push(audio_out_playback);
                            break;
                        }
                    }
                }
            },
        }
    }

    loop {
        match channels.packet_recv.try_recv() {
            Err(TryRecvError::Empty) => break,
            Ok(NetworkAudioPackets::MusicPacket((music_id, music_data))) => {
                if let Some(playback_ind) = state
                    .playbacks
                    .iter()
                    .position(|p| p.realtime_id == music_id)
                {
                    let playback = &mut state.playbacks[playback_ind];

                    let mut start_ind = playback.decoded_data_offset;
                    let mut end_ind = start_ind + 1920;
                    if start_ind > 0 {
                        if playback.decoded_data_count < 15360 {
                            start_ind = playback.decoded_data_count;
                            end_ind = start_ind + 1920;
                        } else {
                            let _ = channels.debug_send.send("Unfortunate\n");
                            playback.decoded_data_offset = 0;
                            playback.decoded_data_count = 0;
                            start_ind = 0;
                            end_ind = 1920;
                        }
                    }

                    match playback.decoder.decode_float(
                        &music_data[1..],
                        &mut playback.decoded_data[start_ind..end_ind],
                        false,
                    ) {
                        Ok(decode_len) => {
                            state.playbacks[playback_ind].decoded_data_count += decode_len * 2;
                            //let _ = channels.debug_send.send("Music Decode!\n");
                        }
                        Err(_) => {
                            let _ = channels.debug_send.send("Opus Decode Issue\n");
                            continue;
                        }
                    }
                } else {
                    let is_stereo = music_data[0] > 0;
                    let opus_channels = match is_stereo {
                        true => opus::Channels::Stereo,
                        false => opus::Channels::Mono,
                    };
                    let mut decoder = match opus::Decoder::new(48000, opus_channels) {
                        Ok(decoder) => decoder,
                        Err(err) => {
                            let _ = channels.debug_send.send("Cannot Create Opus Decoder\n");
                            break;
                        }
                    };
                    let mut decoded_data = [0.0; 15360];
                    let mut decoded_data_count = 0;
                    match decoder.decode_float(&music_data[1..], &mut decoded_data, false) {
                        Ok(decode_len) => {
                            decoded_data_count += decode_len * 2;
                        }
                        Err(_) => {
                            let _ = channels.debug_send.send("Opus Decode Issue\n");
                            continue;
                        }
                    }

                    let audio_out_playback = AudioOutputPlayback {
                        is_stereo,
                        decoder,
                        decoded_data,
                        decoded_data_count,
                        decoded_data_offset: 0,
                        realtime_id: music_id,
                        opus_data_index: 0,
                        opus_data_next_packet: 0,
                        opus_data_next_data: 0,
                    };
                    state.playbacks.push(audio_out_playback);
                }
            }
            Ok(NetworkAudioPackets::MusicStop(music_id)) => {
                if let Some(playback_ind) = state
                    .playbacks
                    .iter()
                    .position(|p| p.realtime_id == music_id)
                {
                    state.playbacks.remove(playback_ind);
                }
            }
            Ok(_) => {
                //Nothing yet
            }
            Err(TryRecvError::Disconnected) => {
                let _ = channels
                    .debug_send
                    .send("Audio Packet Recv Disconnected!!!\n");
                break;
            }
        }
    }

    // Samples are Left Right Interleaved

    for sample in &mut *samples {
        *sample = 0.0;
    }

    if samples.len() != 960 {
        let _ = channels
            .debug_send
            .send("Not the right amount of samples!\n");
        return;
    }

    for (s_ind, s) in state.playbacks.iter_mut().enumerate() {
        if s.realtime_id > 0 {
            if s.is_stereo {
                for (index, sample) in samples.iter_mut().enumerate() {
                    *sample += s.decoded_data[index + s.decoded_data_offset];
                }
                s.decoded_data_offset += 960;
                if s.decoded_data_offset >= s.decoded_data_count {
                    s.decoded_data_offset = 0;
                    s.decoded_data_count = 0;
                }
            }
        } else {
            // Non-realtime
            if s.is_stereo {
                let opus_data = &opus_list[s.opus_data_index];
                while s.decoded_data_count < 960
                    && s.opus_data_next_packet < opus_data.packet_len.len()
                {
                    let next_packet_bytes = opus_data.packet_len[s.opus_data_next_packet] as usize;
                    match s.decoder.decode_float(
                        &opus_data.packet_data
                            [s.opus_data_next_data..s.opus_data_next_data + next_packet_bytes],
                        &mut s.decoded_data[s.decoded_data_count..],
                        false,
                    ) {
                        Ok(decode_len) => {
                            s.opus_data_next_packet += 1;
                            s.opus_data_next_data += next_packet_bytes;
                            s.decoded_data_count += decode_len * 2;
                        }
                        Err(err) => {
                            let _ = channels.debug_send.send("Opus Decode Issue\n");
                            break;
                        }
                    }
                }
                if s.decoded_data_count >= 960 {
                    #[allow(clippy::needless_range_loop)]
                    for index in 0..960 {
                        samples[index] += s.decoded_data[index + s.decoded_data_offset];
                    }
                    s.decoded_data_count -= 960;
                    if s.decoded_data_count > 0 {
                        s.decoded_data_offset += 960;
                    } else {
                        s.decoded_data_offset = 0;
                    }
                } else if s.decoded_data_count > 0 {
                    let _ = channels.debug_send.send("Decode Case Unhandled\n");
                } else {
                    state.cleanup.push(s_ind);
                }
            }
            // else {

            // }
        }
    }

    while let Some(ind) = state.cleanup.pop() {
        state.playbacks.remove(ind);
    }
}

fn audio_output_error(
    err: cpal::StreamError,
    state_send: &Sender<AudioStateMessage>,
    debug_send: &Sender<&'static str>,
) {
    match err {
        cpal::StreamError::DeviceNotAvailable => {
            let _ = debug_send.send("Audio Output Device Not Available!\n");
        }
        cpal::StreamError::BackendSpecific { err } => {
            let _ = debug_send.send("Audio Output Backend Specific!\n");
        }
    }
}
