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

// This file could be used as a template for using quic with a different aplication protocol negotiated with ALPN
// The protocol used in this program is called "swiftlet"

pub(super) const ALPN_NAME: &[u8] = b"swiftlet"; // Application-Layer Protocol Negotiation Name used to define the Quic-Application Protocol used in this program

pub(super) const MESSAGE_HEADER_SIZE: usize = 3;
pub(super) const MAX_MESSAGE_SIZE: usize = 65535;

// All stream message data (application protocol information) is always in little endian form
#[repr(u8)]
pub(super) enum StreamMsgType {
    InvalidType = 0, // Enforce that it is zero

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    RemoveClient,       // ClientIndex,
    ClientNewState,     // ClientIndex, ClientState
    MusicIdReady,       // MusicID (1 byte)
    NextMusicPacket,    // Stereo, Music Packet

    // General Messages:
    TransferRequest,  // Data_Len_Size (3), TransferIntention (1)
    TransferResponse, // Transfer ID (2)
    TransferData,     // Header (Includes Transfer ID instead of Size) TransferData
    VoiceData,

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    MusicRequest,      // MusicID (1 byte)
}

impl StreamMsgType {
    #[inline] // Verbose but compiles down to minimal instructions
    pub(super) fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == Self::ServerStateRefresh as u8 => Self::ServerStateRefresh,
            x if x == Self::NewClient as u8 => Self::NewClient,
            x if x == Self::RemoveClient as u8 => Self::RemoveClient,
            x if x == Self::ClientNewState as u8 => Self::ClientNewState,
            x if x == Self::MusicIdReady as u8 => Self::MusicIdReady,
            x if x == Self::NextMusicPacket as u8 => Self::NextMusicPacket,

            x if x == Self::TransferRequest as u8 => Self::TransferRequest,
            x if x == Self::TransferResponse as u8 => Self::TransferResponse,
            x if x == Self::TransferData as u8 => Self::TransferData,
            x if x == Self::VoiceData as u8 => Self::VoiceData,

            x if x == Self::NewClientAnnounce as u8 => Self::NewClientAnnounce,
            x if x == Self::NewStateRequest as u8 => Self::NewStateRequest,
            x if x == Self::MusicRequest as u8 => Self::MusicRequest,

            _ => Self::InvalidType,
        }
    }

    #[inline]
    pub(super) fn from_header(header: &[u8]) -> Option<(Self, u16)> {
        if header.len() == MESSAGE_HEADER_SIZE {
            Some((
                Self::from_u8(header[0]),
                u16::from_le_bytes([header[1], header[2]]),
            ))
        } else {
            None
        }
    }

    #[inline]
    pub(super) fn to_u8(&self) -> u8 {
        match self {
            Self::ServerStateRefresh => Self::ServerStateRefresh as u8,
            Self::NewClient => Self::NewClient as u8,
            Self::RemoveClient => Self::RemoveClient as u8,
            Self::ClientNewState => Self::ClientNewState as u8,
            Self::MusicIdReady => Self::MusicIdReady as u8,
            Self::NextMusicPacket => Self::NextMusicPacket as u8,

            Self::TransferRequest => Self::TransferRequest as u8,
            Self::TransferResponse => Self::TransferResponse as u8,
            Self::TransferData => Self::TransferData as u8,
            Self::VoiceData => Self::VoiceData as u8,

            Self::NewClientAnnounce => Self::NewClientAnnounce as u8,
            Self::NewStateRequest => Self::NewStateRequest as u8,
            Self::MusicRequest => Self::MusicRequest as u8,

            _ => Self::InvalidType as u8,
        }
    }

    #[inline]
    pub(super) fn intended_for_client(&self) -> bool {
        matches!(
            self,
            Self::ServerStateRefresh
                | Self::NewClient
                | Self::RemoveClient
                | Self::ClientNewState
                | Self::MusicIdReady
                | Self::NextMusicPacket
                | Self::TransferRequest
                | Self::TransferResponse
                | Self::TransferData
                | Self::VoiceData
        )
    }

    #[inline]
    pub(super) fn intended_for_server(&self) -> bool {
        matches!(
            self,
            Self::TransferRequest
                | Self::TransferResponse
                | Self::TransferData
                | Self::VoiceData
                | Self::NewClientAnnounce
                | Self::NewStateRequest
                | Self::MusicRequest
        )
    }

    // Optimized enough for compiler...?
    #[inline]
    pub(super) fn get_send_data_vec(&self, body_capacity: Option<usize>) -> Vec<u8> {
        if let Some(body_size) = body_capacity {
            let mut send_data = Vec::with_capacity(body_size + MESSAGE_HEADER_SIZE);
            send_data.push(self.to_u8());
            send_data.push(0);
            send_data.push(0);
            send_data
        } else {
            Vec::from([self.to_u8(), 0, 0])
        }
    }
}

#[repr(u8)]
pub(super) enum TransferIntention {
    Deletion = 0,
    Music,
}

impl TransferIntention {
    #[inline]
    pub(super) fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == Self::Music as u8 => Self::Music,
            _ => Self::Deletion,
        }
    }

    #[inline]
    pub(super) fn to_u8(&self) -> u8 {
        match self {
            Self::Music => Self::Music as u8,
            _ => Self::Deletion as u8,
        }
    }
}

#[inline]
pub(super) fn set_stream_msg_size(vec_data: &mut [u8]) {
    let num_bytes = usize::to_le_bytes(vec_data.len() - MESSAGE_HEADER_SIZE);
    vec_data[1] = num_bytes[0];
    vec_data[2] = num_bytes[1];
}
