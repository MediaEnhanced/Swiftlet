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
pub const MAX_MESSAGE_SIZE: usize = 65536;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum MessageType {
    InvalidType = 0,

    // Server Messages: (Seperate/nested enum in the future...? Would work with try_from?)
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    ClientNewState,     // ClientIndex, ClientState
    FileTransferResponse,

    // Client Messages: (Seperate/nested enum in the future...? Would work with try_from?)
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    KeepConnectionAlive,
    FileTransferRequest,

    // Data Messages: (Seperate/nested enum in the future...? Would work with try_from?)
    FileTransferData,
}

impl MessageType {
    fn get_value(&self) -> u8 {
        // Requires Copy and Clone derived...?
        *self as u8
    }

    fn get_max_size(&self) -> usize {
        match self {
            MessageType::ServerStateRefresh => MAX_MESSAGE_SIZE,
            MessageType::NewClient => MESSAGE_HEADER_SIZE + 256,
            MessageType::ClientNewState => MESSAGE_HEADER_SIZE + 2,
            MessageType::FileTransferResponse => MAX_MESSAGE_SIZE,

            MessageType::NewClientAnnounce => MESSAGE_HEADER_SIZE + 256,
            MessageType::NewStateRequest => MESSAGE_HEADER_SIZE + 1,
            MessageType::KeepConnectionAlive => MESSAGE_HEADER_SIZE,
            MessageType::FileTransferRequest => MAX_MESSAGE_SIZE,

            MessageType::FileTransferData => MAX_MESSAGE_SIZE,

            _ => 3,
        }
    }
}

// This method is supported by: https://stackoverflow.com/questions/71167454/rust-implement-try-from-for-u8-enum
// It gets properly compiled down to a few assembly instructions
impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == MessageType::ServerStateRefresh as u8 => Ok(MessageType::ServerStateRefresh),
            x if x == MessageType::NewClient as u8 => Ok(MessageType::NewClient),
            x if x == MessageType::ClientNewState as u8 => Ok(MessageType::ClientNewState),
            x if x == MessageType::FileTransferResponse as u8 => {
                Ok(MessageType::FileTransferResponse)
            }
            x if x == MessageType::NewClientAnnounce as u8 => Ok(MessageType::NewClientAnnounce),
            x if x == MessageType::NewStateRequest as u8 => Ok(MessageType::NewStateRequest),
            x if x == MessageType::KeepConnectionAlive as u8 => {
                Ok(MessageType::KeepConnectionAlive)
            }
            x if x == MessageType::FileTransferRequest as u8 => {
                Ok(MessageType::FileTransferRequest)
            }
            x if x == MessageType::FileTransferData as u8 => Ok(MessageType::FileTransferData),
            _ => Ok(MessageType::InvalidType),
        }
    }
}

pub struct StreamMessage {
    msg_type: MessageType,
    size: usize, // Includes Message Type and Size
    data: [u8; MAX_MESSAGE_SIZE],
    for_recv: bool,
    valid_data: usize,
}

impl StreamMessage {
    pub fn new_recv(header_data: [u8; MESSAGE_HEADER_SIZE]) -> Self {
        let msg_type = match MessageType::try_from(header_data[0]) {
            Ok(mt) => mt,
            Err(_) => MessageType::InvalidType,
        };

        let size = usize::from_ne_bytes([header_data[2], header_data[1], 0, 0, 0, 0, 0, 0]);

        let mut stream_msg = StreamMessage {
            msg_type,
            size,
            data: [0; MAX_MESSAGE_SIZE],
            for_recv: true,
            valid_data: 3,
        };

        stream_msg.data[0] = header_data[0];
        stream_msg.data[1] = header_data[1];
        stream_msg.data[2] = header_data[2];

        stream_msg
    }

    pub fn refresh_recv(&mut self, header_data: [u8; MESSAGE_HEADER_SIZE]) -> bool {
        if self.for_recv {
            self.msg_type = match MessageType::try_from(header_data[0]) {
                Ok(mt) => mt,
                Err(_) => MessageType::InvalidType,
            };

            self.size = ((header_data[1] as usize) << 8) | (header_data[2] as usize);

            self.data[0] = header_data[0];
            self.data[1] = header_data[1];
            self.data[2] = header_data[2];

            self.valid_data = 3;

            true
        } else {
            false
        }
    }

    #[inline]
    pub fn get_message_type(&self) -> &MessageType {
        &self.msg_type
    }

    pub fn get_data_to_recv(&mut self) -> Option<&mut [u8]> {
        if self.for_recv {
            Some(&mut self.data[self.valid_data..self.size])
        } else {
            None
        }
    }

    // Returns true if full
    pub fn update_data_recv(&mut self, recv_bytes: usize) -> Result<bool, ()> {
        if self.for_recv {
            let total_data_collected = self.valid_data + recv_bytes;
            match total_data_collected.cmp(&self.size) {
                std::cmp::Ordering::Equal => {
                    self.valid_data = total_data_collected;
                    Ok(true)
                }
                std::cmp::Ordering::Less => {
                    self.valid_data = total_data_collected;
                    Ok(false)
                }
                std::cmp::Ordering::Greater => Err(()),
            }
        } else {
            Err(())
        }
    }

    pub fn get_data_to_read(&self) -> Option<&[u8]> {
        if self.for_recv && self.valid_data == self.size {
            Some(&self.data[MESSAGE_HEADER_SIZE..self.valid_data])
        } else {
            None
        }
    }

    #[inline]
    pub fn is_done_recving(&self) -> bool {
        self.valid_data == self.size
    }

    pub fn new_send(msg_type: MessageType) -> Self {
        let size = msg_type.get_max_size();
        let data0 = msg_type.get_value();

        let mut stream_msg = StreamMessage {
            msg_type,
            size,
            data: [0; MAX_MESSAGE_SIZE],
            for_recv: false,
            valid_data: 3,
        };

        stream_msg.data[0] = data0;
        stream_msg
    }

    pub fn refresh_send(&mut self, msg_type: MessageType) -> bool {
        if !self.for_recv {
            self.data[0] = msg_type.get_value();
            self.msg_type = msg_type;

            self.size = self.msg_type.get_max_size();
            self.valid_data = 3;

            true
        } else {
            false
        }
    }

    pub fn get_data_to_write(&mut self) -> Option<&mut [u8]> {
        if !self.for_recv {
            Some(&mut self.data[self.valid_data..self.size])
        } else {
            None
        }
    }

    // Returns true if full
    pub fn update_data_write(&mut self, write_bytes: usize) -> Result<bool, ()> {
        if !self.for_recv {
            let total_data_written = self.valid_data + write_bytes;
            match total_data_written.cmp(&self.size) {
                std::cmp::Ordering::Less => {
                    self.valid_data = total_data_written;
                    Ok(false)
                }
                std::cmp::Ordering::Equal => {
                    self.valid_data = total_data_written;
                    Ok(true)
                }
                std::cmp::Ordering::Greater => Err(()),
            }
        } else {
            Err(())
        }
    }

    pub fn get_data_to_send(&mut self) -> Option<&[u8]> {
        if !self.for_recv {
            let num_bytes = usize::to_ne_bytes(self.valid_data);
            self.data[1] = num_bytes[1];
            self.data[2] = num_bytes[0];
            Some(&self.data[..self.valid_data])
        } else {
            None
        }
    }

    pub fn get_mut_data_to_send(&mut self) -> Option<&mut [u8]> {
        if !self.for_recv {
            let num_bytes = usize::to_ne_bytes(self.valid_data);
            self.data[1] = num_bytes[1];
            self.data[2] = num_bytes[0];
            Some(&mut self.data[..self.valid_data])
        } else {
            None
        }
    }
}
