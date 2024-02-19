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
use endpoint::{ConnectionId, Endpoint, EndpointEvent, Error};

use std::time::{Duration, Instant};

/// Required QUIC Endpoint Handler Event Callback Functions
///
/// These functions will get called for their respective events!
pub trait EndpointEventCallbacks {
    /// Called when a new connection is started and is application ready.
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId);

    /// Called when a connection has ended and should be cleaned up.
    ///
    /// Return true if you want the Handler event loop to exit.
    /// It will return the reference to the endpoint in that case.
    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        remaining_connections: usize,
    ) -> bool;

    /// Called when a connection is in the process of ending and allows an application to clean up relevant states earlier before it fully closes.
    fn connection_ending_warning(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId);

    /// Called when the next tick occurrs based on the tick duration given to the run_event_loop call.
    fn tick(&mut self, endpoint: &mut Endpoint) -> bool;

    /// Called when there is something to read on the main stream.
    ///
    /// The read_data length will be the number of bytes asked for on the previous call.
    /// The first time it is called the length will be the number of bytes set by the Endpoint Config.
    /// Return the number of bytes you want to read the next time this callback is called.
    /// Returning a None will close the connection.
    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize>;

    /// Called when there is something to read on the background stream.
    ///
    /// The read_data length will be the number of bytes asked for on the previous call.
    /// The first time it is called the length will be the number of bytes set by the Endpoint Config.
    /// Return the number of bytes you want to read the next time this callback is called.
    /// Returning a None will close the connection.
    fn background_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize>;
}

/// Main library structure that handles the QUIC Endpoint
pub struct EndpointHandler<'a> {
    current_tick: u64,
    endpoint: Endpoint,
    events: &'a mut dyn EndpointEventCallbacks,
}

impl<'a> EndpointHandler<'a> {
    /// Create a QUIC Endpoint Handler by giving it an already created Endpoint
    /// and a mutable reference of a structure that implements the Endpoint Event Callbacks trait.
    pub fn new(endpoint: Endpoint, events: &'a mut dyn EndpointEventCallbacks) -> Self {
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
    /// Returns the endpoint reference if this event loop function should be maybe called again
    ///  (ie. run a client endpoint in "low power" mode when it has no connections)
    pub fn run_event_loop(
        &mut self,
        tick_duration: Duration,
    ) -> Result<Option<&mut Endpoint>, Error> {
        let start_instant = Instant::now();
        let mut next_tick_instant = start_instant;

        loop {
            // This update sleeps when waiting for the next instant or recv udp data and the duration is >= 1ms
            match self.endpoint.get_next_event(next_tick_instant) {
                Ok(EndpointEvent::ReceivedData) => {
                    if self.run_recv_loop()? {
                        return Ok(Some(&mut self.endpoint));
                    }
                }
                Ok(EndpointEvent::NextTick) => {
                    next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                    self.current_tick += 1;

                    if self.events.tick(&mut self.endpoint) {
                        return Ok(None);
                    }
                }
                Ok(EndpointEvent::ConnectionClosed(cid)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self
                        .events
                        .connection_ended(&mut self.endpoint, &cid, remaining_connections)
                    {
                        return Ok(Some(&mut self.endpoint));
                    }
                }
                Ok(EndpointEvent::ConnectionClosing(cid)) => {
                    self.events
                        .connection_ending_warning(&mut self.endpoint, &cid);
                }
                Ok(EndpointEvent::AlreadyHandled) => {
                    // Do Nothing and try to call get_next_event ASAP
                }
                Err(e) => {
                    return Err(e);
                }
                _ => {
                    // Unexpected case where a panic will happen for now
                    panic!("Unexpected Next Event!");
                    // In the future have endpoint isolate the enums instead of one master EndpointEvent
                }
            }
        }
    }

    fn run_recv_loop(&mut self) -> Result<bool, Error> {
        loop {
            match self.endpoint.recv() {
                Ok(EndpointEvent::DoneReceiving) => {
                    return Ok(false);
                }
                Ok(EndpointEvent::MainStreamReceived(cid)) => loop {
                    let (read_bytes_option, vec_option) = self.endpoint.main_stream_recv(&cid)?;
                    if let Some(read_bytes) = read_bytes_option {
                        let vec_data = match vec_option {
                            Some(v) => v,
                            None => {
                                vec![0; 4096] // Backup and shouldn't ever be called
                                              // Maybe a panic or code redesign so this is not even an issue???
                            }
                        };
                        if let Some(next_recv_target) = self.events.main_stream_recv(
                            &mut self.endpoint,
                            &cid,
                            &vec_data[..read_bytes],
                        ) {
                            let _ = self.endpoint.main_stream_set_target(
                                &cid,
                                next_recv_target,
                                vec_data,
                            );
                        } else {
                            let _ = self.endpoint.close_connection(&cid, 16);
                            break;
                        }
                    } else {
                        break;
                    }
                },
                Ok(EndpointEvent::BackgroundStreamReceived(cid)) => loop {
                    let (read_bytes_option, vec_option) =
                        self.endpoint.background_stream_recv(&cid)?;
                    if let Some(read_bytes) = read_bytes_option {
                        let vec_data = match vec_option {
                            Some(v) => v,
                            None => {
                                vec![0; 4096] // Backup and shouldn't ever be called
                                              // Maybe a panic or code redesign so this is not even an issue???
                            }
                        };
                        if let Some(next_recv_target) = self.events.background_stream_recv(
                            &mut self.endpoint,
                            &cid,
                            &vec_data[..read_bytes],
                        ) {
                            let _ = self.endpoint.background_stream_set_target(
                                &cid,
                                next_recv_target,
                                vec_data,
                            );
                        } else {
                            let _ = self.endpoint.close_connection(&cid, 16);
                            break;
                        }
                    } else {
                        break;
                    }
                },
                Ok(EndpointEvent::ConnectionClosed(cid)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self
                        .events
                        .connection_ended(&mut self.endpoint, &cid, remaining_connections)
                    {
                        return Ok(true);
                    }
                }
                Ok(EndpointEvent::ConnectionClosing(cid)) => {
                    self.events
                        .connection_ending_warning(&mut self.endpoint, &cid);
                }
                Ok(EndpointEvent::EstablishedOnce(cid)) => {
                    self.events.connection_started(&mut self.endpoint, &cid);
                }
                Ok(EndpointEvent::NoUpdate) => {
                    // Do nothing and call recv again
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(_) => {
                    // Unexpected case where a panic will happen for now
                    panic!("Unexpected Receive Event!");
                    // In the future have endpoint isolate the enums instead of one master EndpointEvent
                }
            }
        }
    }
}
