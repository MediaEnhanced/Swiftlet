//Media Enhanced Swiftlet Graphics Rust Library using Vulkan
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

#[derive(Debug)]
pub enum OsError {
    // Dxgi(Error),
    // Window(Error),
    // WindowAlreadyDisplayed,
    // UnknownMsgHandle,
    // Event(Error),
    // UnexpectedEventCheckResult,
}

pub(super) fn get_device_luid() -> Result<[u32; 2], OsError> {
    // let interface = match dxgi::Interface::new(false) {
    //     Ok(i) => i,
    //     Err(e) => return Err(OsError::Dxgi(e)),
    // };
    // Ok(interface.get_luid())
}

// unsafe extern "system" fn os_window_callback(
//     hwnd: HWND,
//     msg: u32,
//     wparam: WPARAM,
//     lparam: LPARAM,
// ) -> LRESULT {
//     //println!("Msg #: {}, {}, {}", msg, wparam.0, lparam.0);
//     match msg {
//         WindowsAndMessaging::WM_CLOSE => {
//             match unsafe {
//                 WindowsAndMessaging::PostMessageW(
//                     hwnd,
//                     WindowsAndMessaging::WM_USER,
//                     WPARAM(0),
//                     LPARAM(0),
//                 )
//             } {
//                 Ok(_) => LRESULT(0),
//                 Err(_e) => LRESULT(1),
//             }
//         }
//         WindowsAndMessaging::WM_DESTROY => {
//             unsafe { WindowsAndMessaging::PostQuitMessage(0) };
//             LRESULT(0)
//         }
//         _ => unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam) },
//     }
// }

pub(super) struct OsWindow {}

pub(super) enum OsWindowState {
    Normal,
    CloseAttempt,
    Closing,
    ShouldDrop,
}

impl OsWindow {
    pub(super) fn new(width: u32, height: u32) -> Result<Self, OsError> {
        Ok(OsWindow {})
    }

    pub(super) fn get_surface_parameters(&self) { // -> (HINSTANCE, HWND) {
                                                  // (self.hinstance, self.handle)
    }

    pub(super) fn process_messages(&mut self) -> Result<OsWindowState, OsError> {
        // loop {
        //     let bool_res = unsafe {
        //         WindowsAndMessaging::PeekMessageW(
        //             &mut self.msg,
        //             HWND::default(),
        //             0,
        //             0,
        //             WindowsAndMessaging::PM_REMOVE,
        //         )
        //     };
        //     if bool_res.0 == 0 {
        //         return Ok(OsWindowState::Normal);
        //     } else if self.msg.message != WindowsAndMessaging::WM_USER {
        //         if self.msg.message != WindowsAndMessaging::WM_QUIT {
        //             let _res = unsafe { WindowsAndMessaging::DispatchMessageW(&self.msg) };
        //         } else {
        //             println!("Msg Info: {:?}", self.msg);
        //             return Ok(OsWindowState::ShouldDrop);
        //         }
        //     } else {
        //         return Ok(OsWindowState::CloseAttempt);
        //     }
        // }
    }

    pub(super) fn close_window(&mut self) -> Result<(), OsError> {
        // if let Err(e) = unsafe { WindowsAndMessaging::DestroyWindow(self.handle) } {
        //     Err(OsError::Window(e))
        // } else {
        //     Ok(())
        // }
    }
}

pub struct OsEventSignaler {
    //handle: HANDLE,
}

impl OsEventSignaler {
    pub fn signal(&self) -> Result<(), OsError> {
        // if let Err(e) = unsafe { Threading::SetEvent(self.handle) } {
        //     Err(OsError::Event(e))
        // } else {
        //     Ok(())
        // }
    }
}

pub(super) struct OsEvent {
    //handle: HANDLE,
}

impl OsEvent {
    pub(super) fn new() -> Result<Self, OsError> {
        // match unsafe {
        //     Threading::CreateEventW(None, BOOL::from(false), BOOL::from(false), PCWSTR::null())
        // } {
        //     Ok(handle) => Ok(OsEvent { handle }),
        //     Err(e) => Err(OsError::Event(e)),
        // }
    }

    pub(super) fn create_signaler(&self) -> OsEventSignaler {
        OsEventSignaler {
            //handle: self.handle,
        }
    }

    pub(super) fn check(&self) -> Result<bool, OsError> {
        // let wait_event = unsafe { Threading::WaitForSingleObject(self.handle, 10) };
        // if wait_event == WAIT_TIMEOUT {
        //     Ok(false)
        // } else if wait_event == WAIT_OBJECT_0 {
        //     Ok(true)
        // } else if wait_event == WAIT_FAILED {
        //     Err(OsError::Event(unsafe { GetLastError().into() }))
        // } else {
        //     Err(OsError::UnexpectedEventCheckResult)
        // }
    }
}

impl Drop for OsEvent {
    fn drop(&mut self) {
        unsafe {
            // let _ = CloseHandle(self.handle);
        }
    }
}
