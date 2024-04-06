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

use std::ffi::{c_int, c_uchar};

#[derive(Clone, Copy)]
#[repr(C)]
enum Channels {
    Mono = 1,
    Stereo = 2,
}

#[repr(C)]
enum EncodingApplication {
    Voip = 2048,
    Audio = 2049,
    LowDelay = 2051,
}

#[link(name = "opus", kind = "static")]
extern "C" {
    fn opus_decoder_get_size(channels: Channels) -> c_int;

    fn opus_decoder_init(decoder: *mut u8, sample_rate: c_int, channels: Channels) -> c_int;

    fn opus_decode_float(
        decoder: *mut u8,
        data: *const c_uchar,
        data_len: c_int,
        samples: *mut f32,
        samples_len: c_int,
        decode_fec: c_int,
    ) -> c_int;

    fn opus_encoder_get_size(channels: Channels) -> c_int;

    fn opus_encoder_init(
        encoder: *mut u8,
        sample_rate: c_int,
        channels: Channels,
        mode: EncodingApplication,
    ) -> c_int;

    fn opus_encode_float(
        encoder: *mut u8,
        samples: *const f32,
        samples_len: c_int,
        data: *mut c_uchar,
        data_len: c_int,
    ) -> c_int;
}

#[derive(Debug)]
pub enum Error {
    InputSize = 2,
    SliceTooLong = 1,
    Ok = 0,
    BadArg = -1,
    BufferTooSmall = -2,
    InternalError = -3,
    InvalidPacket = -4,
    Unimplemented = -5,
    InvalidState = -6,
    AllocFail = -7,
    Unknown = -8,
}

impl Error {
    fn from_i32(v: i32) -> Self {
        match v {
            x if x == Error::Ok as i32 => Error::Ok,
            x if x == Error::BadArg as i32 => Error::BadArg,
            x if x == Error::BufferTooSmall as i32 => Error::BufferTooSmall,
            x if x == Error::InternalError as i32 => Error::InternalError,
            x if x == Error::InvalidPacket as i32 => Error::InvalidPacket,
            x if x == Error::Unimplemented as i32 => Error::Unimplemented,
            x if x == Error::InvalidState as i32 => Error::InvalidState,
            x if x == Error::AllocFail as i32 => Error::AllocFail,
            _ => Error::Unknown,
        }
    }
}

pub struct Decoder {
    decoder: Vec<u8>,
    is_stereo: bool,
}

impl Decoder {
    pub fn new(is_stereo: bool) -> Result<Self, Error> {
        let channels = match is_stereo {
            true => Channels::Stereo,
            false => Channels::Mono,
        };
        let decoder_size = unsafe { opus_decoder_get_size(channels) };
        let mut decoder = vec![0; decoder_size as usize];

        let status = unsafe { opus_decoder_init(decoder.as_mut_ptr(), 48000, channels) };
        if status != Error::Ok as i32 {
            return Err(Error::from_i32(status));
        }
        Ok(Decoder { decoder, is_stereo })
    }

    pub fn decode_float(&mut self, input: &[u8], output: &mut [f32]) -> Result<usize, Error> {
        // Packet loss when input.len() is zero
        let ptr = match input.len() {
            0 => std::ptr::null(),
            _ => input.as_ptr(),
        };
        let data_len = match c_int::try_from(input.len()) {
            Ok(v) => v,
            Err(_) => return Err(Error::SliceTooLong),
        };

        let samples_len = match c_int::try_from(output.len()) {
            Ok(v) => {
                if self.is_stereo {
                    v >> 1
                } else {
                    v
                }
            }
            Err(_) => return Err(Error::SliceTooLong),
        };

        let status = unsafe {
            opus_decode_float(
                self.decoder.as_mut_ptr(),
                ptr,
                data_len,
                output.as_mut_ptr(),
                samples_len,
                0,
            )
        };

        if status < 0 {
            return Err(Error::from_i32(status));
        }
        Ok(status as usize)
    }
}

pub struct Encoder {
    encoder: Vec<u8>,
    is_stereo: bool,
}

impl Encoder {
    pub fn new(is_stereo: bool, is_voip: bool) -> Result<Self, Error> {
        let channels = match is_stereo {
            true => Channels::Stereo,
            false => Channels::Mono,
        };
        let encoder_size = unsafe { opus_encoder_get_size(channels) };
        let mut encoder = vec![0; encoder_size as usize];

        let mode = match is_voip {
            true => EncodingApplication::Voip,
            false => EncodingApplication::Audio,
        };

        let status = unsafe { opus_encoder_init(encoder.as_mut_ptr(), 48000, channels, mode) };
        if status != Error::Ok as i32 {
            return Err(Error::from_i32(status));
        }
        Ok(Encoder { encoder, is_stereo })
    }

    pub fn encode_float(&mut self, input: &[f32], output: &mut [u8]) -> Result<usize, Error> {
        if (input.len() != 480) && (input.len() != 960) {
            return Err(Error::InputSize);
        }
        let samples = input.as_ptr();

        let status = unsafe {
            opus_encode_float(
                self.encoder.as_mut_ptr(),
                samples,
                input.len() as c_int,
                output.as_mut_ptr(),
                output.len() as c_int,
            )
        };

        if status < 0 {
            return Err(Error::from_i32(status));
        }
        Ok(status as usize)
    }
}

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

pub struct OpusData {
    id: u64,
    is_stereo: bool, // 1 or 2 channels
    pre_skip: u16,
    output_gain: f32,
    packet_len: Vec<u16>,
    packet_data: Vec<u8>,
}

impl OpusData {
    pub fn create_from_ogg_file(data: &[u8], id: u64) -> Option<Self> {
        let mut index = 0;

        let first_page_result = match &data[index..index + 26].try_into() {
            Ok(ogg_page_header) => analyze_ogg_page_header(ogg_page_header, None),
            Err(_err) => {
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
            is_stereo: stereo,
            pre_skip,
            output_gain,
            packet_len: Vec::new(),
            packet_data: Vec::new(),
        };

        index += segment_len;
        remaining_bytes -= segment_len;
        let second_page_result = match &data[index..index + 26].try_into() {
            Ok(ogg_page_header) => analyze_ogg_page_header(ogg_page_header, Some(&serial_num)),
            Err(_err) => {
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
                Err(_err) => {
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

    pub fn matches_id(&self, id: u64) -> bool {
        self.id == id
    }

    pub fn is_stereo(&self) -> bool {
        self.is_stereo
    }

    pub fn get_input_slice(&self, packet: usize, data_offset: usize) -> Option<&[u8]> {
        if packet >= self.packet_len.len() {
            None
        } else {
            Some(&self.packet_data[data_offset..(data_offset + (self.packet_len[packet] as usize))])
        }
    }

    pub fn get_stereo(&self) -> Option<Vec<f32>> {
        if self.is_stereo {
            let mut decoder = match Decoder::new(true) {
                Ok(decoder) => decoder,
                Err(_) => {
                    return None;
                }
            };

            let mut stereo = Vec::new();
            let mut stereo_data = vec![0.0; 1920];
            let mut packet_data_postion = 0;
            for l in &self.packet_len {
                let next_packet_data_position = packet_data_postion + (*l as usize);
                match decoder.decode_float(
                    &self.packet_data[packet_data_postion..next_packet_data_position],
                    &mut stereo_data,
                ) {
                    Ok(frames_decoded) => {
                        stereo.extend_from_slice(&stereo_data[..frames_decoded * 2]);
                        packet_data_postion = next_packet_data_position;
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }

            Some(stereo)
        } else {
            None
        }
    }

    pub fn to_data(&self) -> (u8, usize, usize, Vec<u8>) {
        let mut data = Vec::new();

        let stereo_byte = if self.is_stereo { 1 } else { 0 };

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

    pub fn add_to_vec(&self, data: &mut Vec<u8>) {
        if self.is_stereo {
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
