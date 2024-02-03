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

pub const MESSAGE_HEADER_SIZE: usize = 3;
pub const MAX_MESSAGE_SIZE: usize = 65535;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum StreamMsgType {
    InvalidType = 0,

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    RemoveClient,       // ClientIndex,
    ClientNewState,     // ClientIndex, ClientState
    TransferResponse,   // Transfer ID (1)

    // Not fully working yet:
    MusicTransferResponse, // MusicTransferID (2 bytes)
    MusicIdReady,          // MusicID (2 bytes)

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    KeepConnectionAlive,
    TransferRequest, // Data_Len_Size (3), TransferIntention (1)
    TransferData,    // Transfer ID (1), TransferData

    // Not fully working yet:
    MusicTransferRequest, // Packet_Len_Size (8), Data_Len_Size(8), Stereo

    MusicTransferData, // MusicTransferID (2 bytes), MusicTransferData
}

pub enum StreamMsgIntended {
    Nobody,
    Client,
    Server,
    Anyone, // Might get rid of this type in future
}

impl StreamMsgType {
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == StreamMsgType::ServerStateRefresh as u8 => StreamMsgType::ServerStateRefresh,
            x if x == StreamMsgType::NewClient as u8 => StreamMsgType::NewClient,
            x if x == StreamMsgType::RemoveClient as u8 => StreamMsgType::RemoveClient,
            x if x == StreamMsgType::ClientNewState as u8 => StreamMsgType::ClientNewState,
            x if x == StreamMsgType::TransferResponse as u8 => StreamMsgType::TransferResponse,

            x if x == StreamMsgType::MusicTransferResponse as u8 => {
                StreamMsgType::MusicTransferResponse
            }
            x if x == StreamMsgType::MusicIdReady as u8 => StreamMsgType::MusicIdReady,

            x if x == StreamMsgType::NewClientAnnounce as u8 => StreamMsgType::NewClientAnnounce,
            x if x == StreamMsgType::NewStateRequest as u8 => StreamMsgType::NewStateRequest,
            x if x == StreamMsgType::KeepConnectionAlive as u8 => {
                StreamMsgType::KeepConnectionAlive
            }
            x if x == StreamMsgType::TransferRequest as u8 => StreamMsgType::TransferRequest,
            x if x == StreamMsgType::TransferData as u8 => StreamMsgType::TransferData,

            x if x == StreamMsgType::MusicTransferRequest as u8 => {
                StreamMsgType::MusicTransferRequest
            }
            x if x == StreamMsgType::MusicTransferData as u8 => StreamMsgType::MusicTransferData,
            _ => StreamMsgType::InvalidType,
        }
    }

    fn get_value(&self) -> u8 {
        // Requires Copy and Clone derived...?
        *self as u8
    }

    fn intended_for(&self) -> StreamMsgIntended {
        match self {
            StreamMsgType::ServerStateRefresh => StreamMsgIntended::Client,
            StreamMsgType::NewClient => StreamMsgIntended::Client,
            StreamMsgType::RemoveClient => StreamMsgIntended::Client,
            StreamMsgType::ClientNewState => StreamMsgIntended::Client,
            StreamMsgType::TransferResponse => StreamMsgIntended::Client,

            StreamMsgType::MusicTransferResponse => StreamMsgIntended::Client,
            StreamMsgType::MusicIdReady => StreamMsgIntended::Client,

            StreamMsgType::NewClientAnnounce => StreamMsgIntended::Server,
            StreamMsgType::NewStateRequest => StreamMsgIntended::Server,
            StreamMsgType::KeepConnectionAlive => StreamMsgIntended::Server,
            StreamMsgType::TransferRequest => StreamMsgIntended::Server,
            StreamMsgType::TransferData => StreamMsgIntended::Server,

            StreamMsgType::MusicTransferRequest => StreamMsgIntended::Server,

            StreamMsgType::MusicTransferData => StreamMsgIntended::Anyone,

            _ => StreamMsgIntended::Nobody,
        }
    }

    fn get_max_size(&self) -> usize {
        match self {
            StreamMsgType::ServerStateRefresh => MAX_MESSAGE_SIZE,
            StreamMsgType::NewClient => MESSAGE_HEADER_SIZE + 256,
            StreamMsgType::RemoveClient => MESSAGE_HEADER_SIZE + 1,
            StreamMsgType::ClientNewState => MESSAGE_HEADER_SIZE + 2,
            StreamMsgType::TransferResponse => MESSAGE_HEADER_SIZE + 1,

            StreamMsgType::MusicTransferResponse => MESSAGE_HEADER_SIZE + 3,
            StreamMsgType::MusicIdReady => MESSAGE_HEADER_SIZE + 3,

            StreamMsgType::NewClientAnnounce => MESSAGE_HEADER_SIZE + 256,
            StreamMsgType::NewStateRequest => MESSAGE_HEADER_SIZE + 1,
            StreamMsgType::KeepConnectionAlive => MESSAGE_HEADER_SIZE,
            StreamMsgType::TransferRequest => MESSAGE_HEADER_SIZE + 4,
            StreamMsgType::TransferData => MAX_MESSAGE_SIZE,

            StreamMsgType::MusicTransferRequest => MESSAGE_HEADER_SIZE + 3,

            StreamMsgType::MusicTransferData => MAX_MESSAGE_SIZE,

            _ => 3,
        }
    }
}

pub struct StreamMsgRecv {
    msg_type: StreamMsgType,
    size: usize, // Includes Message Type and Size
    data: [u8; MAX_MESSAGE_SIZE],
    valid_data: usize,
}

impl StreamMsgRecv {
    pub fn new(header_data: [u8; MESSAGE_HEADER_SIZE]) -> Self {
        let msg_type = StreamMsgType::from_u8(header_data[0]);

        let size = usize::from_ne_bytes([header_data[2], header_data[1], 0, 0, 0, 0, 0, 0]);

        let mut stream_msg = StreamMsgRecv {
            msg_type,
            size,
            data: [0; MAX_MESSAGE_SIZE],
            valid_data: 3,
        };

        stream_msg.data[0] = header_data[0];
        stream_msg.data[1] = header_data[1];
        stream_msg.data[2] = header_data[2];

        stream_msg
    }

    pub fn refresh_recv(&mut self, header_data: [u8; MESSAGE_HEADER_SIZE]) {
        self.msg_type = StreamMsgType::from_u8(header_data[0]);

        self.size = ((header_data[1] as usize) << 8) | (header_data[2] as usize);

        self.data[0] = header_data[0];
        self.data[1] = header_data[1];
        self.data[2] = header_data[2];

        self.valid_data = 3;
    }

    #[inline]
    pub fn get_message_type(&self) -> StreamMsgType {
        self.msg_type
    }

    #[inline]
    pub fn get_data_to_recv(&mut self) -> &mut [u8] {
        &mut self.data[self.valid_data..self.size]
    }

    // Returns true if full
    pub fn update_data_recv(&mut self, recv_bytes: usize) -> bool {
        let total_data_collected = self.valid_data + recv_bytes;
        match total_data_collected.cmp(&self.size) {
            std::cmp::Ordering::Equal => {
                self.valid_data = total_data_collected;
                true
            }
            std::cmp::Ordering::Less => {
                self.valid_data = total_data_collected;
                false
            }
            std::cmp::Ordering::Greater => false, // better way to handle this in future..?
        }
    }

    #[inline]
    pub fn is_done_recving(&self) -> bool {
        self.valid_data == self.size
    }

    #[inline]
    pub fn get_done_intention(&self) -> Option<StreamMsgIntended> {
        if self.valid_data == self.size {
            Some(self.msg_type.intended_for())
        } else {
            None
        }
    }

    #[inline]
    pub fn get_data_to_read(&self) -> &[u8] {
        &self.data[MESSAGE_HEADER_SIZE..self.valid_data]
    }
}

pub struct StreamMsgSend {
    msg_type: StreamMsgType,
    size: usize, // Includes Message Type and Size
    data: [u8; MAX_MESSAGE_SIZE],
    valid_data: usize,
}

impl StreamMsgSend {
    pub fn new(msg_type: StreamMsgType) -> Self {
        let size = msg_type.get_max_size();
        let data0 = msg_type.get_value();

        let mut stream_msg = StreamMsgSend {
            msg_type,
            size,
            data: [0; MAX_MESSAGE_SIZE],
            valid_data: 3,
        };

        stream_msg.data[0] = data0;
        stream_msg
    }

    pub fn refresh_send(&mut self, msg_type: StreamMsgType) {
        self.data[0] = msg_type.get_value();
        self.msg_type = msg_type;

        self.size = self.msg_type.get_max_size();
        self.valid_data = 3;
    }

    pub fn get_data_to_write(&mut self) -> &mut [u8] {
        &mut self.data[self.valid_data..self.size]
    }

    // Returns true if full
    pub fn update_data_write(&mut self, write_bytes: usize) -> bool {
        let total_data_written = self.valid_data + write_bytes;
        match total_data_written.cmp(&self.size) {
            std::cmp::Ordering::Less => {
                self.valid_data = total_data_written;
                false
            }
            std::cmp::Ordering::Equal => {
                self.valid_data = total_data_written;
                true
            }
            std::cmp::Ordering::Greater => false, // better way to handle this in future..?
        }
    }

    pub fn get_data_to_send(&mut self) -> &[u8] {
        let num_bytes = usize::to_ne_bytes(self.valid_data);
        self.data[1] = num_bytes[1];
        self.data[2] = num_bytes[0];
        &self.data[..self.valid_data]
    }

    pub fn get_mut_data_to_send(&mut self) -> &mut [u8] {
        let num_bytes = usize::to_ne_bytes(self.valid_data);
        self.data[1] = num_bytes[1];
        self.data[2] = num_bytes[0];
        &mut self.data[..self.valid_data]
    }
}
