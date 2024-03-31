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

use rtrb::RingBuffer;
pub(crate) use rtrb::{Consumer, PopError, Producer, PushError};
//RecvTimeoutError

pub(crate) struct NetworkTerminalThreadChannels {
    pub(crate) command_recv: Consumer<NetworkCommand>,
    pub(crate) state_send: Producer<NetworkStateMessage>,
    pub(crate) debug_send: Producer<String>, // String so that non-static debug messages can be made!
}

pub(crate) struct TerminalNetworkThreadChannels {
    pub(crate) command_send: Producer<NetworkCommand>,
    pub(crate) state_recv: Consumer<NetworkStateMessage>,
    pub(crate) debug_recv: Consumer<String>,
}

pub(crate) fn create_networking_channels(
) -> (NetworkTerminalThreadChannels, TerminalNetworkThreadChannels) {
    let (command_send, command_recv) = RingBuffer::new(64);
    let (state_send, state_recv) = RingBuffer::new(64);
    let (debug_send, debug_recv) = RingBuffer::new(256);

    let network_channels = NetworkTerminalThreadChannels {
        command_recv,
        state_send,
        debug_send,
    };
    let console_channels = TerminalNetworkThreadChannels {
        command_send,
        state_recv,
        debug_recv,
    };

    (network_channels, console_channels)
}

pub(crate) enum NetworkCommand {
    Stop(u64),
    Server(ServerCommand),
    #[cfg(feature = "client")]
    Client(ClientCommand),
}

pub(crate) enum ServerCommand {
    ConnectionClose(usize),
}

#[cfg(feature = "client")]
pub(crate) enum ClientCommand {
    StateChange(u8),
    ServerConnect(swiftlet_quic::endpoint::SocketAddr),
    MusicTransfer(swiftlet_audio::opus::OpusData),
    UploadTest(u8),
}

pub(crate) enum NetworkStateMessage {
    ServerNameChange(String),
    ConnectionsRefresh((Option<usize>, Vec<NetworkStateConnection>)),
    NewConnection((String, u8)),
    StateChange((usize, u8)),
}

pub(crate) struct NetworkStateConnection {
    pub(crate) name: String,
    pub(crate) state: u8,
}

#[cfg(feature = "client")]
pub(crate) struct AudioThreadChannels {
    // Audio Output Specific Channels
    pub(crate) output_cmd_recv: Consumer<TerminalAudioOutCommands>,
    pub(crate) packet_recv: Consumer<NetworkAudioOutPackets>,
    pub(crate) state_send: Producer<AudioStateMessage>,
    pub(crate) output_debug_send: Producer<String>,

    // Audio Input Specific Channels
    pub(crate) input_cmd_recv: Consumer<TerminalAudioInCommands>,
    pub(crate) packet_send: Producer<NetworkAudioInPackets>,
    pub(crate) input_debug_send: Producer<String>,
}

#[cfg(feature = "client")]
pub(crate) struct NetworkAudioThreadChannels {
    pub(crate) packet_send: Producer<NetworkAudioOutPackets>,
    pub(crate) packet_recv: Consumer<NetworkAudioInPackets>,
}

#[cfg(feature = "client")]
pub(crate) struct TerminalAudioThreadChannels {
    pub(crate) output_cmd_send: Producer<TerminalAudioOutCommands>,
    pub(crate) input_cmd_send: Producer<TerminalAudioInCommands>,
    pub(crate) state_recv: Consumer<AudioStateMessage>,
    pub(crate) output_debug_recv: Consumer<String>,
    pub(crate) input_debug_recv: Consumer<String>,
}

#[cfg(feature = "client")]
pub(crate) fn create_audio_channels() -> (
    AudioThreadChannels,
    NetworkAudioThreadChannels,
    TerminalAudioThreadChannels,
) {
    let (output_cmd_send, output_cmd_recv) = RingBuffer::new(64);
    let (input_cmd_send, input_cmd_recv) = RingBuffer::new(64);
    let (packet_send, audio_packet_recv) = RingBuffer::new(64);
    let (audio_packet_send, packet_recv) = RingBuffer::new(20); // 20 10ms Input Buffers
    let (state_send, state_recv) = RingBuffer::new(64);
    let (output_debug_send, output_debug_recv) = RingBuffer::new(256);
    let (input_debug_send, input_debug_recv) = RingBuffer::new(256);

    let audio_output_channels = AudioThreadChannels {
        output_cmd_recv,
        packet_recv: audio_packet_recv,
        state_send,
        output_debug_send,
        input_cmd_recv,
        packet_send: audio_packet_send,
        input_debug_send,
    };
    let network_audio_output_channels = NetworkAudioThreadChannels {
        packet_send,
        packet_recv,
    };
    let console_audio_output_channels = TerminalAudioThreadChannels {
        output_cmd_send,
        input_cmd_send,
        state_recv,
        output_debug_recv,
        input_debug_recv,
    };

    (
        audio_output_channels,
        network_audio_output_channels,
        console_audio_output_channels,
    )
}

// Quit happens as a result of the disconnect channel error
#[cfg(feature = "client")]
pub(crate) enum TerminalAudioOutCommands {
    LoadOpus(swiftlet_audio::opus::OpusData),
    PlayOpus(u64),
}

#[cfg(feature = "client")]
pub(crate) enum NetworkAudioOutPackets {
    MusicPacket((u8, Vec<u8>)),
    MusicStop(u8),
    VoiceData((u16, Vec<u8>)),
    VoiceStop(u16),
}

// Quit happens as a result of the disconnect channel error
#[cfg(feature = "client")]
pub(crate) enum TerminalAudioInCommands {
    Start,
    Stop,
    Quit,
}

#[cfg(feature = "client")]
pub(crate) struct NetworkAudioInPackets {
    pub(crate) data: [u8; 512],
    pub(crate) len: usize,
    pub(crate) instant: std::time::Instant,
}

#[cfg(feature = "client")]
pub(crate) enum AudioStateMessage {
    InputPaused,
}
