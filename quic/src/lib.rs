//Media Enhanced Swiftlet Quic Rust Library for Real-time Internet Communications
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

#![deny(missing_docs)]

//! Providing real-time internet communications using the QUIC protocol!
//!
//! Using this QUIC swiftlet sub-library makes it easy to create a single-threaded QUIC endpoint
//! (server or client) and run an application protocol in response to various events. Both reliable
//! (time-insensitive) and unreliable (real-time communication) messages are possible to be sent and
//! received using this library.

/// QUIC Endpoint Module
pub mod endpoint;
use endpoint::{
    ConnectionEndReason, ConnectionId, Endpoint, Error, NextEvent, ReadInfo, RecvEvent,
};

use std::time::{Duration, Instant};

/// Required QUIC Endpoint Handler Event Callback Functions
///
/// These functions will get called for their respective events.
/// These callbacks are expected to return within a couple milliseconds AT THE MOST
/// for all processing cases.
pub trait EndpointEventCallbacks {
    /// Called when a new connection is started and is application ready.
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId);

    /// Called when a connection has ended and should be cleaned up.
    ///
    /// Return true if you want the Endpoint Handler event loop to exit.
    /// The event loop will return an Ok(true) indicating that the connection_ended callback function caused the exit.
    /// This is intended to be useful in case the application wants to start up the Endpoint Handler event loop again.
    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
        remaining_connections: usize,
    ) -> bool;

    /// Called when a connection is in the process of ending and allows an application to clean up
    /// relevant states earlier before the connection fully ends.
    ///
    /// By default, this function does nothing when called.
    fn connection_ending_warning(
        &mut self,
        _endpoint: &mut Endpoint,
        _cid: &ConnectionId,
        _reason: ConnectionEndReason,
    ) {
        // Do nothing by default
    }

    /// Called when the next tick occurrs based on the tick duration given to the run_event_loop call.
    ///
    /// Return true if you want the Endpoint Handler event loop to exit.
    /// The event loop will return an Ok(false) indicating that the tick callback function caused the exit.
    fn tick(&mut self, endpoint: &mut Endpoint) -> bool;

    /// Called when there is something to read on the main stream.
    ///
    /// The main stream is a reliable (ordered) stream that focuses on communicating
    /// high-priority, small(ish) messages between the server and client.
    ///
    /// The read_data length will be the number of bytes asked for on the previous call.
    /// The first time it is called the length will be the number of bytes set by the Endpoint Config
    /// (initial_main_recv_size). This data can be processed based on the application protocol.
    ///
    /// Return the number of bytes you want to read the next time this callback is called.
    /// If the optional usize value is set to zero (0) it will be interpreted as the Endpoint Config
    /// initial_main_recv_size value.
    /// Returning a None will close the main stream but since the main stream is required,
    /// the connection will start the close process.
    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize>; // Just a usize in future where 0 represents close the stream...?

    /// Called when there is something to read on the real-time stream.
    ///
    /// The real-time "stream" is different than the main stream because it uses multiple
    /// incremental QUIC unidirectional streams in the backend where each stream id represents
    /// a single time segment that has unreliability the moment when the next single time segment
    /// arrives before the previous stream had finished.
    ///
    /// The read_data length will be the number of bytes asked for on the previous call.
    /// The first time it is called the length will be the number of bytes set by the Endpoint Config
    /// (initial_rt_recv_size). This data can be processed based on the application protocol.
    ///
    /// Return the number of bytes you want to read the next time this callback is called.
    /// If the usize value is set to zero (0) it will be interpreted as waiting for the next finished
    /// real-time stream.
    ///
    /// By default, this function will return 0, which translates to waiting for the next finished
    /// real-time stream as indicated above. This function should be overwritten in order to handle
    /// processing real-time stream data.
    fn rt_stream_recv(
        &mut self,
        _endpoint: &mut Endpoint,
        _cid: &ConnectionId,
        _read_data: &[u8],
        _rt_id: u64,
    ) -> usize {
        0
    }

    /// Called when there is something to read on the background stream.
    ///
    /// The background stream is a reliable (ordered) stream that focuses on communicating
    /// large(ish) messages between the server and client such as a file transfer.
    ///
    /// The read_data length will be the number of bytes asked for on the previous call.
    /// The first time it is called the length will be the number of bytes set by the Endpoint Config
    /// (initial_background_recv_size). This data can be processed based on the application protocol.
    ///
    /// Return the number of bytes you want to read the next time this callback is called.
    /// If the optional usize value is set to zero (0) it will be interpreted as the as the Endpoint Config
    /// initial_background_recv_size value.
    /// Returning a None will close the background stream but since the background stream is required,
    /// the connection will start the close process.
    ///
    /// By default, this function will return None, which translates to a connection closure as indicated above.
    /// This function should be overwritten to handle cases where a connection might send
    /// information over the background stream to prevent accidental connection closures.
    fn background_stream_recv(
        &mut self,
        _endpoint: &mut Endpoint,
        _cid: &ConnectionId,
        _read_data: &[u8],
    ) -> Option<usize> {
        // Return None by default since the background stream is not managed
        None
    }
}

/// Main library structure that handles the QUIC Endpoint
pub struct EndpointHandler<'a> {
    current_tick: u64,
    endpoint: &'a mut Endpoint,
    events: &'a mut dyn EndpointEventCallbacks,
}

impl<'a> EndpointHandler<'a> {
    /// Create a QUIC Endpoint Handler by giving it an already created Endpoint
    /// and a mutable reference of a structure that implements the Endpoint Event Callbacks trait.
    pub fn new(endpoint: &'a mut Endpoint, events: &'a mut dyn EndpointEventCallbacks) -> Self {
        EndpointHandler {
            current_tick: 0,
            endpoint,
            events,
        }
    }

    /// QUIC Endpoint Handler Event Loop
    ///
    /// Allows the endpoint handler to take control of the thread!
    ///
    /// Communicates with the application code with the previously passed event callbacks
    ///
    /// Tick "0" callback will happen immediately
    ///
    /// Returns true if this event loop function should be maybe called again
    ///  (ie. run a client endpoint in "low power" mode when it has no connections)
    pub fn run_event_loop(&mut self, tick_duration: Duration) -> Result<bool, Error> {
        let start_instant = Instant::now();
        let mut next_tick_instant = start_instant;

        loop {
            // This function will sleep the thread while waiting for the next instant or recv udp data
            match self.endpoint.get_next_event(next_tick_instant)? {
                NextEvent::ReceivedData => {
                    if self.run_recv_loop()? {
                        return Ok(true);
                    }
                }
                NextEvent::Tick => {
                    next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                    self.current_tick += 1;

                    if self.events.tick(self.endpoint) {
                        return Ok(false);
                    }
                }
                NextEvent::ConnectionEnded((cid, reason)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self.events.connection_ended(
                        self.endpoint,
                        &cid,
                        reason,
                        remaining_connections,
                    ) {
                        return Ok(true);
                    }
                }
                NextEvent::ConnectionEnding((cid, reason)) => {
                    self.events
                        .connection_ending_warning(self.endpoint, &cid, reason);
                }
                NextEvent::AlreadyHandled => {
                    // Do Nothing and try to call get_next_event ASAP
                }
            }
        }
    }

    fn run_recv_loop(&mut self) -> Result<bool, Error> {
        loop {
            match self.endpoint.recv()? {
                RecvEvent::DoneReceiving => {
                    return Ok(false);
                }
                RecvEvent::MainStreamReceived((cid, verified_index, mut data_vec, mut len)) => {
                    loop {
                        let target_len_opt =
                            self.events
                                .main_stream_recv(self.endpoint, &cid, &data_vec[..len]);
                        match self.endpoint.main_stream_read(
                            verified_index,
                            data_vec,
                            target_len_opt,
                        )? {
                            ReadInfo::ReadData((new_data_vec, new_len)) => {
                                data_vec = new_data_vec;
                                len = new_len;
                            }
                            ReadInfo::DoneReceiving => {
                                break;
                            }
                            ReadInfo::ConnectionEnded(reason) => {
                                let remaining_connections = self.endpoint.get_num_connections();
                                if self.events.connection_ended(
                                    self.endpoint,
                                    &cid,
                                    reason,
                                    remaining_connections,
                                ) {
                                    return Ok(true);
                                }
                                break;
                            }
                            ReadInfo::ConnectionEnding(reason) => {
                                self.events
                                    .connection_ending_warning(self.endpoint, &cid, reason);
                                break;
                            }
                        }
                    }
                    // self.endpoint.connection_send(verified_index)?;
                }
                RecvEvent::RealtimeReceived(cid, verified_index, mut data_vec, mut len, rt_id) => {
                    loop {
                        let target_len = self.events.rt_stream_recv(
                            self.endpoint,
                            &cid,
                            &data_vec[..len],
                            rt_id,
                        );
                        match self
                            .endpoint
                            .rt_stream_read(verified_index, data_vec, target_len)?
                        {
                            Some((new_data_vec, new_len)) => {
                                data_vec = new_data_vec;
                                len = new_len;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    // self.endpoint.connection_send(verified_index)?;
                }
                RecvEvent::BackgroundStreamReceived((
                    cid,
                    verified_index,
                    mut data_vec,
                    mut len,
                )) => {
                    loop {
                        let target_len_opt = self.events.background_stream_recv(
                            self.endpoint,
                            &cid,
                            &data_vec[..len],
                        );
                        match self.endpoint.background_stream_read(
                            verified_index,
                            data_vec,
                            target_len_opt,
                        )? {
                            ReadInfo::ReadData((new_data_vec, new_len)) => {
                                data_vec = new_data_vec;
                                len = new_len;
                            }
                            ReadInfo::DoneReceiving => {
                                break;
                            }
                            ReadInfo::ConnectionEnded(reason) => {
                                let remaining_connections = self.endpoint.get_num_connections();
                                if self.events.connection_ended(
                                    self.endpoint,
                                    &cid,
                                    reason,
                                    remaining_connections,
                                ) {
                                    return Ok(true);
                                }
                                break;
                            }
                            ReadInfo::ConnectionEnding(reason) => {
                                self.events
                                    .connection_ending_warning(self.endpoint, &cid, reason);
                                break;
                            }
                        }
                    }
                    // self.endpoint.connection_send(verified_index)?;
                }
                RecvEvent::ConnectionEnded((cid, reason)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self.events.connection_ended(
                        self.endpoint,
                        &cid,
                        reason,
                        remaining_connections,
                    ) {
                        return Ok(true);
                    }
                }
                RecvEvent::ConnectionEnding((cid, reason)) => {
                    self.events
                        .connection_ending_warning(self.endpoint, &cid, reason);
                }
                RecvEvent::EstablishedOnce(cid) => {
                    self.events.connection_started(self.endpoint, &cid);
                }
                RecvEvent::NoUpdate => {
                    // Do nothing and call recv again
                }
            }
        }
    }
}
