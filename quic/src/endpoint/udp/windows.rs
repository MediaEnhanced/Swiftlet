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

use crate::endpoint::SocketAddr;
use std::mem;
use std::ptr;
use windows::core::{PCWSTR, PSTR};
use windows::Win32::Foundation;
use windows::Win32::Foundation::{BOOL, HANDLE, WIN32_ERROR};
use windows::Win32::Networking::WinSock;
use windows::Win32::System::Threading;
use windows::Win32::System::IO::OVERLAPPED;

static WINSOCK_STARTUP: std::sync::Once = std::sync::Once::new();

fn winsock_startup() {
    let mut wsa_data = WinSock::WSADATA::default();
    let res = unsafe { WinSock::WSAStartup(0x0202, &mut wsa_data) };
    if res != 0 {
        panic!(
            "Windows Sockets (Version 2.2) could not be started!: {}",
            res
        );
    }
}

// fn winsock_cleanup() {
//     let _res = unsafe { WinSock::WSACleanup() };
// }

//pub(super) enum Error
#[derive(Debug)]
enum AsyncError {
    NotInitiated(WinSock::WSA_ERROR),
    NotEnoughRecvBuffers,
    WrongSendLength,
}

#[derive(Debug)]
enum WaitError {
    UnexpectedTimeout,
    Failure(WIN32_ERROR),
    AbandonedOverlappedHandle,
    AbandonedTimer,
    Unknown,
}

#[derive(Debug)]
enum GetDataError {
    WaitFailure(WIN32_ERROR),
    AbandonedOverlappedHandle,
    Unknown,
    OverlappedResult(WinSock::WSA_ERROR),
}

const RECV_MSGS: usize = 50; //50 Based on 250Mbps and 2ms processing time (not sure anymore)
const SEND_MSGS: usize = 20;

pub(super) struct UdpSocket {
    is_ipv6: bool,
    _is_server: bool,
    socket: WinSock::SOCKET,
    recv_msgs: Vec<AsyncMessage>,
    recv_current_msg: usize,
    send_msgs: Vec<AsyncMessage>,
    send_current_msg: usize,
    timer_handle: HANDLE,
}

impl UdpSocket {
    // if bind_port is 0 dictate that the socket should obtain a random port to bind to (useful for clients)
    pub(super) fn new(ipv6_mode: bool, bind_port: u16) -> Option<Self> {
        WINSOCK_STARTUP.call_once(winsock_startup);
        let is_ipv6 = ipv6_mode;
        let (address_family, address_length) = match is_ipv6 {
            true => (
                WinSock::AF_INET6,
                mem::size_of::<WinSock::SOCKADDR_IN6>() as i32,
            ),
            false => (
                WinSock::AF_INET,
                mem::size_of::<WinSock::SOCKADDR_IN>() as i32,
            ),
        };

        let socket = unsafe {
            WinSock::WSASocketW(
                address_family.0 as i32,
                WinSock::SOCK_DGRAM.0,
                WinSock::IPPROTO_UDP.0,
                None,
                0,
                WinSock::WSA_FLAG_OVERLAPPED,
            )
        };
        if socket == WinSock::INVALID_SOCKET {
            return None;
        }

        let mut recv_msgs = Vec::with_capacity(RECV_MSGS);
        for _ in 0..RECV_MSGS {
            let msg = AsyncMessage::new(is_ipv6)?;
            recv_msgs.push(msg);
        }

        let mut send_msgs = Vec::with_capacity(SEND_MSGS);
        for _ in 0..SEND_MSGS {
            let msg = AsyncMessage::new(is_ipv6)?;
            send_msgs.push(msg);
        }

        let mut bind_addr = WinSock::SOCKADDR_IN6 {
            sin6_family: address_family,
            ..Default::default()
        };
        let is_server = if bind_port == 0 {
            false
        } else {
            let port = u16::from_be_bytes(u16::to_ne_bytes(bind_port));
            if is_ipv6 {
                bind_addr.sin6_port = port;
            } else {
                let ipv4_bind_addr: &mut WinSock::SOCKADDR_IN =
                    unsafe { mem::transmute(&mut bind_addr) };
                ipv4_bind_addr.sin_port = port;
            }
            true
        };
        let wsa_error = unsafe {
            WinSock::bind(
                socket,
                ptr::addr_of!(bind_addr) as *const WinSock::SOCKADDR,
                address_length,
            )
        };
        if wsa_error == WinSock::SOCKET_ERROR {
            return None;
        }

        for msg in &mut recv_msgs {
            match msg.recv_queue(socket) {
                Ok(_) => {}
                Err(_) => {
                    unsafe {
                        WinSock::closesocket(socket);
                    }
                    return None;
                }
            }
        }

        let timer_name = PCWSTR(ptr::null());
        let timer_handle = match unsafe {
            Threading::CreateWaitableTimerExW(
                None,
                timer_name,
                Threading::CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                Threading::TIMER_MODIFY_STATE.0 | Threading::SYNCHRONIZATION_SYNCHRONIZE.0,
            )
        } {
            Ok(h) => h,
            Err(_e) => return None,
        };

        Some(UdpSocket {
            is_ipv6,
            _is_server: is_server,
            socket,
            recv_msgs,
            recv_current_msg: 0,
            send_msgs,
            send_current_msg: 0,
            timer_handle,
        })
    }

    pub(super) fn get_local_address(&self) -> Option<SocketAddr> {
        let mut address = WinSock::SOCKADDR_IN6::default();
        let mut address_len = mem::size_of::<WinSock::SOCKADDR_IN6>() as i32;
        let wsa_error = unsafe {
            WinSock::getsockname(
                self.socket,
                ptr::addr_of_mut!(address) as *mut WinSock::SOCKADDR,
                &mut address_len,
            )
        };
        if wsa_error != 0 {
            return None;
        }

        if address.sin6_family == WinSock::AF_INET6 {
            let ip = unsafe { std::net::Ipv6Addr::from(address.sin6_addr.u.Byte) };
            Some(SocketAddr::V6(std::net::SocketAddrV6::new(
                ip,
                u16::from_ne_bytes(u16::to_be_bytes(address.sin6_port)),
                0,
                0,
            )))
        } else if address.sin6_family == WinSock::AF_INET {
            let ipv4: &WinSock::SOCKADDR_IN = unsafe { mem::transmute(&address) };
            let ip = unsafe { std::net::Ipv4Addr::from(ipv4.sin_addr.S_un.S_addr) };
            Some(SocketAddr::V4(std::net::SocketAddrV4::new(
                ip,
                u16::from_ne_bytes(u16::to_be_bytes(ipv4.sin_port)),
            )))
        } else {
            None
        }
    }

    pub(super) fn sleep_till_next_recv(&mut self, timeout_duration: std::time::Duration) -> bool {
        let time_convert = (timeout_duration.as_secs() * 10_000_000)
            + (timeout_duration.subsec_nanos() as u64 / 100);
        let relative_time = -(time_convert as i64);
        match unsafe {
            Threading::SetWaitableTimer(self.timer_handle, &relative_time, 0, None, None, BOOL(0))
        } {
            Ok(_) => match self.recv_msgs[self.recv_current_msg].wait_for_msg(
                self.timer_handle,
                (timeout_duration.as_millis() as u32) + 10,
            ) {
                Ok(b) => b,
                Err(e) => panic!("UDP Socket Windows Wait Error: {:?}", e),
            },
            Err(e) => panic!("UDP Socket Windows Set Waitable Timer Error: {:?}", e),
        }
    }

    pub(super) fn get_next_recv(&mut self) -> Option<(&mut [u8], SocketAddr)> {
        match self.recv_msgs[self.recv_current_msg].get_recv_data(self.socket, self.is_ipv6) {
            Ok(opt) => opt,
            Err(e) => panic!("UDP Socket Windows Get Recv Error: {:?}", e),
        }
    }

    pub(super) fn done_with_recv(&mut self) {
        match self.recv_msgs[self.recv_current_msg].recv_queue(self.socket) {
            Ok(true) => {
                self.recv_current_msg += 1;
                if self.recv_current_msg >= self.recv_msgs.len() {
                    self.recv_current_msg = 0;
                }
            }
            Ok(false) => {
                // Do nothing
            }
            Err(e) => panic!("UDP Socket Windows Recv Done Error: {:?}", e),
        }
    }

    pub(super) fn get_next_send(&mut self) -> &mut [u8] {
        match self.send_msgs[self.send_current_msg].get_send_data() {
            Ok(Some(data)) => data,
            Ok(None) => panic!("UDP Socket Windows Not Enough Send Buffers!"),
            Err(e) => panic!("UDP Socket Windows Get Send Error: {:?}", e),
        }
    }

    pub(super) fn done_with_send(&mut self, address: SocketAddr, data_len: usize) {
        match self.send_msgs[self.send_current_msg].send_queue(
            self.socket,
            address,
            data_len as u32,
        ) {
            Ok(true) => {
                self.send_current_msg += 1;
                if self.send_current_msg >= self.send_msgs.len() {
                    self.send_current_msg = 0;
                }
            }
            Ok(false) => {
                // Do nothing
            }
            Err(e) => panic!("UDP Socket Windows Send Done Error: {:?}", e),
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.timer_handle);
            WinSock::closesocket(self.socket);
        }
    }
}

const MSG_DATA_SIZE: usize = 2048;

struct AsyncMessage {
    data: [u8; MSG_DATA_SIZE],
    buffers: [WinSock::WSABUF; 1],
    flags: u32,
    address: WinSock::SOCKADDR_IN6,
    address_length: i32,
    overlapped: OVERLAPPED,
    already_waited: bool,
}

impl AsyncMessage {
    fn new(is_ipv6: bool) -> Option<Self> {
        let handle = match unsafe { WinSock::WSACreateEvent() } {
            Ok(h) => h,
            Err(_) => return None,
        };
        let overlapped = OVERLAPPED {
            hEvent: handle,
            ..Default::default()
        };

        let mut address = WinSock::SOCKADDR_IN6::default();

        let address_length = if is_ipv6 {
            address.sin6_family = WinSock::AF_INET6;
            mem::size_of::<WinSock::SOCKADDR_IN6>() as i32
        } else {
            address.sin6_family = WinSock::AF_INET;
            mem::size_of::<WinSock::SOCKADDR_IN>() as i32
        };

        let mut msg = AsyncMessage {
            data: [0; MSG_DATA_SIZE],
            buffers: [WinSock::WSABUF::default(); 1],
            flags: 0,
            address,
            address_length,
            overlapped,
            already_waited: true,
        };

        msg.buffers[0].len = MSG_DATA_SIZE as u32;
        msg.buffers[0].buf = PSTR::from_raw(ptr::addr_of_mut!(msg.data) as *mut u8);

        Some(msg)
    }

    // Returns true if queue was successful
    fn recv_queue(&mut self, s: WinSock::SOCKET) -> Result<bool, AsyncError> {
        if self.already_waited {
            let buffers = [WinSock::WSABUF {
                len: MSG_DATA_SIZE as u32,
                buf: PSTR::from_raw(ptr::addr_of_mut!(self.data) as *mut u8),
            }];

            //let mut bytes_recv =
            if unsafe {
                WinSock::WSARecvFrom(
                    s,
                    &buffers,
                    None,
                    ptr::addr_of_mut!(self.flags),
                    Some(ptr::addr_of_mut!(self.address) as *mut WinSock::SOCKADDR),
                    Some(ptr::addr_of_mut!(self.address_length)),
                    Some(ptr::addr_of_mut!(self.overlapped)),
                    None,
                )
            } != 0
            {
                let wsa_error = unsafe { WinSock::WSAGetLastError() };
                if wsa_error == WinSock::WSA_IO_PENDING {
                    self.already_waited = false;
                    Ok(true)
                } else {
                    Err(AsyncError::NotInitiated(wsa_error))
                }
            } else {
                Err(AsyncError::NotEnoughRecvBuffers)
            }
        } else {
            Ok(false)
        }
    }

    // Returns true when recv event triggered, false on timeout
    fn wait_for_msg(
        &mut self,
        timer_handle: HANDLE,
        backup_timeout_in_ms: u32,
    ) -> Result<bool, WaitError> {
        if self.already_waited {
            Ok(true)
        } else {
            let handles = [self.overlapped.hEvent, timer_handle];
            let wait_event = unsafe {
                Threading::WaitForMultipleObjects(&handles, BOOL(0), backup_timeout_in_ms)
            };
            if wait_event == Foundation::WAIT_EVENT(1) {
                //Timeout
                Ok(false)
            } else if wait_event == Foundation::WAIT_OBJECT_0 {
                self.already_waited = true;
                Ok(true)
            } else if wait_event == Foundation::WAIT_TIMEOUT {
                Err(WaitError::UnexpectedTimeout)
            } else if wait_event == Foundation::WAIT_FAILED {
                let win_error = unsafe { Foundation::GetLastError() };
                Err(WaitError::Failure(win_error))
            } else if wait_event == Foundation::WAIT_ABANDONED_0 {
                Err(WaitError::AbandonedOverlappedHandle)
            } else if wait_event == Foundation::WAIT_EVENT(129) {
                Err(WaitError::AbandonedTimer)
            } else {
                Err(WaitError::Unknown)
            }
        }
    }

    fn get_recv_data(
        &mut self,
        s: WinSock::SOCKET,
        is_ipv6: bool,
    ) -> Result<Option<(&mut [u8], SocketAddr)>, GetDataError> {
        if !self.already_waited {
            let wait_event = unsafe { Threading::WaitForSingleObject(self.overlapped.hEvent, 0) };
            if wait_event == Foundation::WAIT_TIMEOUT {
                return Ok(None);
            } else if wait_event == Foundation::WAIT_OBJECT_0 {
                self.already_waited = true;
            } else if wait_event == Foundation::WAIT_FAILED {
                let win_error = unsafe { Foundation::GetLastError() };
                return Err(GetDataError::WaitFailure(win_error));
            } else if wait_event == Foundation::WAIT_ABANDONED {
                return Err(GetDataError::AbandonedOverlappedHandle);
            } else {
                return Err(GetDataError::Unknown);
            }
        }

        let mut transfered_bytes = 0;
        let mut flags_result = 0;
        match unsafe {
            WinSock::WSAGetOverlappedResult(
                s,
                &self.overlapped,
                &mut transfered_bytes,
                BOOL(1),
                &mut flags_result,
            )
        } {
            Ok(_) => {
                // Check if data_len matches transferd bytes due to design
                //let data_len = self.buffers[0].len as usize;
                let data_len = transfered_bytes as usize;
                let socket_addr = if is_ipv6 {
                    // Sanity Check of address length?
                    //Sanity address family check?
                    let ip = unsafe { std::net::Ipv6Addr::from(self.address.sin6_addr.u.Byte) };

                    SocketAddr::V6(std::net::SocketAddrV6::new(
                        ip,
                        u16::from_ne_bytes(u16::to_be_bytes(self.address.sin6_port)),
                        0,
                        0,
                    ))
                } else {
                    // Sanity Check of address length?
                    //Sanity address family check?
                    let ipv4: &WinSock::SOCKADDR_IN = unsafe { mem::transmute(&self.address) };
                    let ip = unsafe { std::net::Ipv4Addr::from(ipv4.sin_addr.S_un.S_addr) };
                    SocketAddr::V4(std::net::SocketAddrV4::new(
                        ip,
                        u16::from_ne_bytes(u16::to_be_bytes(ipv4.sin_port)),
                    ))
                };
                Ok(Some((&mut self.data[..data_len], socket_addr)))
            }
            Err(_e) => {
                let wsa_error = unsafe { WinSock::WSAGetLastError() };
                Err(GetDataError::OverlappedResult(wsa_error))
            }
        }
    }

    fn get_send_data(&mut self) -> Result<Option<&mut [u8]>, GetDataError> {
        if !self.already_waited {
            let wait_event = unsafe { Threading::WaitForSingleObject(self.overlapped.hEvent, 0) };
            if wait_event == Foundation::WAIT_TIMEOUT {
                return Ok(None);
            } else if wait_event == Foundation::WAIT_OBJECT_0 {
                self.already_waited = true;
            } else if wait_event == Foundation::WAIT_FAILED {
                let win_error = unsafe { Foundation::GetLastError() };
                return Err(GetDataError::WaitFailure(win_error));
            } else if wait_event == Foundation::WAIT_ABANDONED {
                return Err(GetDataError::AbandonedOverlappedHandle);
            } else {
                return Err(GetDataError::Unknown);
            }
        }

        Ok(Some(&mut self.data))
    }

    // Returns true if send was queued up
    fn send_queue(
        &mut self,
        s: WinSock::SOCKET,
        address: SocketAddr,
        length: u32,
    ) -> Result<bool, AsyncError> {
        if self.already_waited {
            match address {
                SocketAddr::V6(addr) => {
                    self.address.sin6_port = u16::from_be_bytes(u16::to_ne_bytes(addr.port()));
                    self.address.sin6_addr = WinSock::IN6_ADDR {
                        u: WinSock::IN6_ADDR_0 {
                            Byte: addr.ip().octets(),
                        },
                    };
                }
                SocketAddr::V4(addr) => {
                    let ipv4: &mut WinSock::SOCKADDR_IN =
                        unsafe { mem::transmute(&mut self.address) };
                    ipv4.sin_port = u16::from_be_bytes(u16::to_ne_bytes(addr.port()));
                    ipv4.sin_addr = WinSock::IN_ADDR {
                        S_un: WinSock::IN_ADDR_0 {
                            S_addr: u32::from_ne_bytes(addr.ip().octets()),
                        },
                    };
                }
            }

            let buffers = [WinSock::WSABUF {
                len: length,
                buf: PSTR::from_raw(ptr::addr_of_mut!(self.data) as *mut u8),
            }];
            let mut bytes_sent = 0;
            if unsafe {
                WinSock::WSASendTo(
                    s,
                    &buffers,
                    Some(&mut bytes_sent),
                    0,
                    Some(ptr::addr_of!(self.address) as *const WinSock::SOCKADDR),
                    self.address_length,
                    Some(ptr::addr_of_mut!(self.overlapped)),
                    None,
                )
            } != 0
            {
                let wsa_error = unsafe { WinSock::WSAGetLastError() };
                if wsa_error == WinSock::WSA_IO_PENDING {
                    self.already_waited = false;
                    Ok(true)
                } else {
                    Err(AsyncError::NotInitiated(wsa_error))
                }
            } else if bytes_sent == length {
                Ok(false)
            } else {
                Err(AsyncError::WrongSendLength)
            }
        } else {
            Ok(false)
        }
    }
}

impl Drop for AsyncMessage {
    fn drop(&mut self) {
        let _ = unsafe { WinSock::WSACloseEvent(self.overlapped.hEvent) };
    }
}
