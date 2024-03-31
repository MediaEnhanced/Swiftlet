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

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::communication::{
    AudioStateMessage, AudioThreadChannels, Consumer, NetworkAudioInPackets,
    NetworkAudioOutPackets, PopError, Producer, PushError, TerminalAudioInCommands,
    TerminalAudioOutCommands,
};

use swiftlet_audio::opus::{Decoder, Encoder, OpusData};

pub(crate) fn audio_thread(channels: AudioThreadChannels) {
    let output = Output {
        callback_count: 0,
        playbacks: Vec::new(),
        cleanup: Vec::new(),
        realtimes: Vec::new(),
        opus_list: Vec::new(),
        command_recv: channels.output_cmd_recv,
        packet_recv: channels.packet_recv,
        state_send: channels.state_send,
        debug_send: channels.output_debug_send,
    };

    let input = Input {
        callback_count: 0,
        last_instant: Instant::now(),
        avg_duration: Duration::from_millis(0),
        encoder: Encoder::new(false, true).unwrap(),
        data: [0; 512],
        data_len: 0,
        command_recv: channels.input_cmd_recv,
        packet_send: channels.packet_send,
        debug_send: channels.input_debug_send,
    };

    if swiftlet_audio::run_input_output(480, 2, 1, output, input).is_err() {
        panic!("Audio Error");
    }
}

struct Output {
    callback_count: u64,
    playbacks: Vec<OutputPlayback>,
    cleanup: Vec<usize>,
    realtimes: Vec<OutputRealtime>,
    opus_list: Vec<OpusData>,
    command_recv: Consumer<TerminalAudioOutCommands>,
    packet_recv: Consumer<NetworkAudioOutPackets>,
    state_send: Producer<AudioStateMessage>,
    debug_send: Producer<String>,
}

impl Output {
    #[inline]
    fn send_state(&mut self, state_msg: AudioStateMessage) -> bool {
        if self.state_send.is_abandoned() {
            true
        } else {
            match self.state_send.push(state_msg) {
                Ok(_) => false,
                Err(PushError::Full(_)) => panic!("Audio Output: State Send Full!"),
            }
        }
    }

    #[inline]
    fn send_debug(&mut self, s: String) -> bool {
        if self.debug_send.is_abandoned() {
            true
        } else {
            match self.debug_send.push(s) {
                Ok(_) => false,
                Err(PushError::Full(_)) => panic!("Audio Output: Debug Send Full!"),
            }
        }
    }

    #[inline]
    fn send_debug_str(&mut self, s: &str) -> bool {
        self.send_debug(s.to_string())
    }
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
    starve_counter: u64,
}

// Better alignment in the future?
struct OutputData {
    data: [f32; 1920],
    data_len: usize,
    read_offset: usize,
}

impl swiftlet_audio::OutputCallback for Output {
    fn output_callback(&mut self, samples: &mut [f32]) -> bool {
        self.callback_count += 1;

        let samples_len = samples.len();
        if samples_len != 960 {
            if self.send_debug_str("Not the expected amount of samples!\n") {
                return true;
            }
            if samples_len == 0 {
                // Quit Early
                return true;
            }
        }

        if self.command_recv.is_abandoned() {
            return true;
        }
        loop {
            match self.command_recv.pop() {
                Err(PopError::Empty) => break,
                Ok(command_recv) => match command_recv {
                    TerminalAudioOutCommands::LoadOpus(opus) => {
                        self.opus_list.push(opus);
                    }
                    TerminalAudioOutCommands::PlayOpus(id) => {
                        for (index, opus_data) in self.opus_list.iter().enumerate() {
                            if opus_data.matches_id(id) {
                                let decoder = match Decoder::new(opus_data.is_stereo()) {
                                    Ok(decoder) => decoder,
                                    Err(err) => {
                                        self.send_debug_str("Cannot Create Opus Decoder\n");
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
                                self.playbacks.push(out_playback);
                                break;
                            }
                        }
                    }
                },
            }
        }

        if self.packet_recv.is_abandoned() {
            return true;
        }
        loop {
            match self.packet_recv.pop() {
                Err(PopError::Empty) => break,
                Ok(NetworkAudioOutPackets::MusicPacket((music_id, music_data))) => {
                    if let Some(realtime_ind) =
                        self.realtimes.iter().position(|p| p.id == music_id as u64)
                    {
                        let realtime = &mut self.realtimes[realtime_ind];

                        let mut output_data = OutputData {
                            data: [0.0; 1920],
                            data_len: 0,
                            read_offset: 0,
                        };

                        match realtime
                            .decoder
                            .decode_float(&music_data[1..], &mut output_data.data)
                        {
                            Ok(decode_len) => {
                                output_data.data_len = decode_len;
                                if realtime.is_stereo {
                                    output_data.data_len *= 2;
                                }
                            }
                            Err(_) => {
                                self.send_debug_str("Opus Decode Issue\n");
                                return true;
                            }
                        }
                        realtime.data_queue.push_back(output_data);
                    } else {
                        let is_stereo = music_data[0] > 0;
                        let mut decoder = match Decoder::new(is_stereo) {
                            Ok(decoder) => decoder,
                            Err(err) => {
                                self.send_debug_str("Cannot Create Opus Decoder\n");
                                return true;
                            }
                        };
                        let mut output_data = OutputData {
                            data: [0.0; 1920],
                            data_len: 0,
                            read_offset: 0,
                        };
                        match decoder.decode_float(&music_data[1..], &mut output_data.data) {
                            Ok(decode_len) => {
                                output_data.data_len = decode_len;
                                if is_stereo {
                                    output_data.data_len *= 2;
                                }
                            }
                            Err(_) => {
                                self.send_debug_str("Opus Decode Issue\n");
                                return true;
                            }
                        }

                        let mut output_realtime = OutputRealtime {
                            id: music_id as u64,
                            is_stereo,
                            decoder,
                            data_queue: VecDeque::with_capacity(4),
                            starve_counter: 0,
                        };
                        output_realtime.data_queue.push_back(output_data);
                        self.realtimes.push(output_realtime);
                    }
                }
                Ok(NetworkAudioOutPackets::MusicStop(music_id)) => {
                    if let Some(realtime_ind) =
                        self.realtimes.iter().position(|p| p.id == music_id as u64)
                    {
                        self.realtimes.remove(realtime_ind);
                    }
                }
                Ok(NetworkAudioOutPackets::VoiceData((voice_id, voice_data))) => {
                    if let Some(realtime_ind) =
                        self.realtimes.iter().position(|p| p.id == voice_id as u64)
                    {
                        let realtime = &mut self.realtimes[realtime_ind];

                        let mut output_data = OutputData {
                            data: [0.0; 1920],
                            data_len: 0,
                            read_offset: 0,
                        };

                        match realtime
                            .decoder
                            .decode_float(&voice_data, &mut output_data.data)
                        {
                            Ok(decode_len) => {
                                output_data.data_len = decode_len;
                            }
                            Err(_) => {
                                self.send_debug_str("Opus Decode Issue\n");
                                return true;
                            }
                        }
                        realtime.data_queue.push_back(output_data);
                    } else {
                        let decoder = match Decoder::new(false) {
                            Ok(decoder) => decoder,
                            Err(err) => {
                                self.send_debug_str("Cannot Create Opus Decoder\n");
                                return true;
                            }
                        };

                        let mut output_realtime = OutputRealtime {
                            id: voice_id as u64,
                            is_stereo: false,
                            decoder,
                            data_queue: VecDeque::with_capacity(4),
                            starve_counter: 0,
                        };

                        let output_data = OutputData {
                            data: [0.0; 1920],
                            data_len: 960,
                            read_offset: 0,
                        };
                        output_realtime.data_queue.push_back(output_data);

                        let mut output_data = OutputData {
                            data: [0.0; 1920],
                            data_len: 0,
                            read_offset: 0,
                        };
                        match output_realtime
                            .decoder
                            .decode_float(&voice_data, &mut output_data.data)
                        {
                            Ok(decode_len) => {
                                output_data.data_len = decode_len;
                            }
                            Err(_) => {
                                self.send_debug_str("Opus Decode Issue\n");
                                return true;
                            }
                        }

                        output_realtime.data_queue.push_back(output_data);
                        self.realtimes.push(output_realtime);
                    }
                }
                Ok(NetworkAudioOutPackets::VoiceStop(voice_id)) => {
                    if let Some(realtime_ind) =
                        self.realtimes.iter().position(|p| p.id == voice_id as u64)
                    {
                        self.realtimes.remove(realtime_ind);
                    }
                } // Ok(_) => {
                  //     //Nothing yet
                  // }
            }
        }

        // Samples are Left Right Interleaved for normal stereo stuff
        // Does NOT currently assume that the samples are zero to begin with
        samples.fill(0.0);

        for (playback_ind, playback) in self.playbacks.iter_mut().enumerate() {
            let opus_data = match self.opus_list[playback.probable_index].matches_id(playback.id) {
                true => &self.opus_list[playback.probable_index],
                false => {
                    if let Some(new_index) = self
                        .opus_list
                        .iter()
                        .position(|od| od.matches_id(playback.id))
                    {
                        playback.probable_index = new_index;
                        &self.opus_list[playback.probable_index]
                    } else {
                        self.send_debug_str("Could not find playback Opus Data!\n");
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
                            match playback
                                .decoder
                                .decode_float(input_data, &mut playback.data.data)
                            {
                                Ok(decode_len) => {
                                    playback.data.data_len = decode_len * 2;
                                    playback.data.read_offset = 0;
                                    readable_samples = decode_len * 2;
                                }
                                Err(err) => {
                                    self.send_debug_str("Opus Decode Issue\n");
                                    return true;
                                }
                            }
                        } else {
                            self.cleanup.push(playback_ind);
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

        while let Some(ind) = self.cleanup.pop() {
            //let _ = channels.debug_send.send("Playback Finished!\n");
            self.playbacks.remove(ind);
        }

        for (realtime_ind, realtime) in self.realtimes.iter_mut().enumerate() {
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
                        match self
                            .debug_send
                            .push("Realtime playback starved!\n".to_string())
                        {
                            Ok(_) => {}
                            Err(PushError::Full(_)) => return true,
                        }
                        realtime.starve_counter += 1;
                        if realtime.starve_counter >= 200 {
                            self.cleanup.push(realtime_ind);
                        }
                        break;
                    }
                }
            } else {
                let mut samples_count = 0;
                loop {
                    if let Some(output_data) = realtime.data_queue.front_mut() {
                        let readable_samples = (output_data.data_len - output_data.read_offset) * 2;
                        let writeable_samples = samples_len - samples_count;
                        if readable_samples >= writeable_samples {
                            realtime.starve_counter = 0;
                            let next_read_offset = output_data.read_offset + writeable_samples;
                            //samples[samples_count..].copy_from_slice(&output_data.data[output_data.read_offset..next_read_offset]);
                            for (s_ind, s) in samples[samples_count..].iter_mut().enumerate() {
                                *s += output_data.data[output_data.read_offset + (s_ind >> 1)]
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
                            *s += output_data.data[output_data.read_offset + (s_ind >> 1)]
                        }
                        samples_count = next_samples_count;
                    } else {
                        //let _ = self.debug_send.send("Voice Realtime Starved!\n");
                        realtime.starve_counter += 1;
                        if realtime.starve_counter == 1 {
                            match self
                                .debug_send
                                .push("Realtime voice starved!\n".to_string())
                            {
                                Ok(_) => {}
                                Err(PushError::Full(_)) => return true,
                            }
                        } else if realtime.starve_counter >= 200 {
                            self.cleanup.push(realtime_ind);
                            match self
                                .debug_send
                                .push("Realtime voice starved out!\n".to_string())
                            {
                                Ok(_) => {}
                                Err(PushError::Full(_)) => return true,
                            }
                        }
                        break;
                    }
                }
            }
        }

        while let Some(ind) = self.cleanup.pop() {
            self.realtimes.remove(ind);
        }

        false
    }
}

struct Input {
    callback_count: u64,
    last_instant: Instant,
    avg_duration: Duration,
    encoder: Encoder,
    data: [u8; 512],
    data_len: usize,
    command_recv: Consumer<TerminalAudioInCommands>,
    packet_send: Producer<NetworkAudioInPackets>,
    // state_send: Producer<AudioStateMessage>,
    debug_send: Producer<String>,
}

impl Input {
    #[inline]
    fn send_state(&self, state_msg: AudioStateMessage) -> bool {
        // match self
        //     .state_send
        //     .send_timeout(state_msg, Duration::from_millis(1))
        // {
        //     Ok(_) => false,
        //     Err(SendTimeoutError::Disconnected(_)) => true,
        //     Err(SendTimeoutError::Timeout(_)) => panic!("Audio Input: State Send Timeout!"),
        // }
        false
    }

    #[inline]
    fn send_debug(&mut self, s: String) -> bool {
        if self.debug_send.is_abandoned() {
            true
        } else {
            match self.debug_send.push(s) {
                Ok(_) => false,
                Err(PushError::Full(_)) => panic!("Audio Output: Debug Send Full!"),
            }
        }
    }

    #[inline]
    fn send_debug_str(&mut self, s: &str) -> bool {
        self.send_debug(s.to_string())
    }
}

impl swiftlet_audio::InputTrait for Input {
    fn wait_to_start(&mut self) -> bool {
        loop {
            loop {
                match self.command_recv.pop() {
                    Err(PopError::Empty) => break,
                    Ok(TerminalAudioInCommands::Start) => {
                        match Encoder::new(false, true) {
                            Ok(enc) => {
                                self.encoder = enc;
                                return true;
                            }
                            Err(e) => {
                                self.send_debug_str("Encoder could not be created!!!\n");
                                return false;
                            }
                        };
                    }
                    Ok(TerminalAudioInCommands::Quit) => return false,
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn callback(&mut self, samples: &[f32]) -> bool {
        self.callback_count += 1;

        let current_instant = Instant::now();
        self.avg_duration += current_instant - self.last_instant;
        self.last_instant = current_instant;
        if (self.callback_count % 100) == 0 {
            // let s = format!("Avg Input Callback Timing: {:?}\n", self.avg_duration / 100);
            // if self.send_debug(s) {
            //     return true;
            // }
            self.avg_duration = Duration::from_millis(0);
        }

        if self.command_recv.is_abandoned() {
            return true;
        }
        loop {
            match self.command_recv.pop() {
                Err(PopError::Empty) => break,
                Ok(TerminalAudioInCommands::Stop) => return true,
                Ok(TerminalAudioInCommands::Quit) => return true,
                _ => {}
            }
        }

        let samples_len = samples.len();
        if samples_len != 480 {
            if self.send_state(AudioStateMessage::InputPaused) {
                return true;
            }
            if self.send_debug_str("Audio Input: Did not get the expected amount of samples!") {
                return true;
            }
        }

        match self.encoder.encode_float(samples, &mut self.data) {
            Ok(len) => {
                let in_packet = NetworkAudioInPackets {
                    data: self.data,
                    len,
                    instant: Instant::now(),
                };
                match self.packet_send.push(in_packet) {
                    Ok(_) => {}
                    Err(PushError::Full(_)) => {
                        self.send_debug_str("Audio Input: Send Packet Full!");
                        return true;
                    }
                }
            }
            Err(e) => {
                if self.send_state(AudioStateMessage::InputPaused) {
                    return true;
                }
                if self.send_debug_str("Audio Input: Did not get the expected amount of samples!") {
                    return true;
                }
            }
        }

        false
    }

    fn error(&mut self, e: swiftlet_audio::Error, _recoverable: bool) {
        let s = format!("Audio Input Error: {:?}", e);
        self.send_debug(s);
    }
}
