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

use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, BOOL, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WAIT_FAILED,
    WAIT_OBJECT_0, WAIT_TIMEOUT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{COLOR_BACKGROUND, HBRUSH};
use windows::Win32::System::{LibraryLoader, Threading};
use windows::Win32::UI::WindowsAndMessaging;

mod dxgi;

#[derive(Debug)]
pub enum OsError {
    Dxgi(Error),
    Window(Error),
    WindowAlreadyDisplayed,
    UnknownMsgHandle,
    Event(Error),
    UnexpectedEventCheckResult,
}

pub(super) fn get_device_luid() -> Result<Option<[u32; 2]>, OsError> {
    let interface = match dxgi::Interface::new(false) {
        Ok(i) => i,
        Err(e) => return Err(OsError::Dxgi(e)),
    };
    Ok(Some(interface.get_luid()))
}

unsafe extern "system" fn os_window_callback(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    //println!("Msg #: {}, {}, {}", msg, wparam.0, lparam.0);
    match msg {
        WindowsAndMessaging::WM_CLOSE => {
            match unsafe {
                WindowsAndMessaging::PostMessageW(
                    hwnd,
                    WindowsAndMessaging::WM_USER,
                    WPARAM(0),
                    LPARAM(0),
                )
            } {
                Ok(_) => LRESULT(0),
                Err(_e) => LRESULT(1),
            }
        }
        WindowsAndMessaging::WM_DESTROY => {
            unsafe { WindowsAndMessaging::PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub(super) struct OsWindow {
    hinstance: HINSTANCE,
    handle: HWND,
    resize_disabled: bool,
    placement: WindowsAndMessaging::WINDOWPLACEMENT,
    msg: WindowsAndMessaging::MSG,
}

pub(super) enum OsWindowState {
    Normal,
    CloseAttempt,
    Closing,
    ShouldDrop,
}

impl OsWindow {
    pub(super) fn new(width: u32, height: u32) -> Result<Self, OsError> {
        //windows::Win32::
        let hinstance = match unsafe { LibraryLoader::GetModuleHandleW(None) } {
            Ok(i) => i.into(),
            Err(e) => return Err(OsError::Window(e)),
        };

        let mut class_name: Vec<u16> = "Vulkan Window Class".encode_utf16().collect();
        class_name.push(0);
        let window_class = WindowsAndMessaging::WNDCLASSEXW {
            cbSize: std::mem::size_of::<WindowsAndMessaging::WNDCLASSEXW>() as u32,
            style: WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(os_window_callback),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: match unsafe {
                WindowsAndMessaging::LoadIconW(None, WindowsAndMessaging::IDI_WINLOGO)
            } {
                Ok(i) => i,
                Err(e) => return Err(OsError::Window(e)),
            },
            hCursor: match unsafe {
                WindowsAndMessaging::LoadCursorW(None, WindowsAndMessaging::IDC_HAND)
            } {
                Ok(i) => i,
                Err(e) => return Err(OsError::Window(e)),
            },
            hbrBackground: HBRUSH(COLOR_BACKGROUND.0 as isize),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            hIconSm: match unsafe {
                WindowsAndMessaging::LoadIconW(None, WindowsAndMessaging::IDI_INFORMATION)
            } {
                Ok(i) => i,
                Err(e) => return Err(OsError::Window(e)),
            },
        };
        let res = unsafe { WindowsAndMessaging::RegisterClassExW(&window_class) };
        if res == 0 {
            return Err(OsError::Window(unsafe { GetLastError().into() }));
        }

        let mut r = RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };

        let style = WindowsAndMessaging::WS_OVERLAPPEDWINDOW;
        let ex_style = WindowsAndMessaging::WINDOW_EX_STYLE(0);
        if let Err(e) = unsafe {
            WindowsAndMessaging::AdjustWindowRectEx(&mut r, style, BOOL::from(false), ex_style)
        } {
            return Err(OsError::Window(e));
        }

        let corrected_width = r.right - r.left;
        let corrected_height = r.bottom - r.top;

        let mut window_name: Vec<u16> = "Window Title".encode_utf16().collect();
        window_name.push(0);
        let handle = unsafe {
            WindowsAndMessaging::CreateWindowExW(
                ex_style,
                PCWSTR::from_raw(class_name.as_ptr()),
                PCWSTR::from_raw(window_name.as_ptr()),
                style,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                corrected_width,
                corrected_height,
                HWND::default(),
                WindowsAndMessaging::HMENU::default(),
                hinstance,
                None,
            )
        };

        let bool_res =
            unsafe { WindowsAndMessaging::ShowWindow(handle, WindowsAndMessaging::SW_NORMAL) };
        if bool_res.as_bool() {
            // Not REALLY and error
            return Err(OsError::WindowAlreadyDisplayed);
        }

        let mut placement = WindowsAndMessaging::WINDOWPLACEMENT {
            length: std::mem::size_of::<WindowsAndMessaging::WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };

        if let Err(e) = unsafe { WindowsAndMessaging::GetWindowPlacement(handle, &mut placement) } {
            return Err(OsError::Window(e));
        }

        println!("Window Handle: {:?}", handle);

        Ok(OsWindow {
            hinstance,
            handle,
            resize_disabled: false,
            placement,
            msg: WindowsAndMessaging::MSG::default(),
        })
    }

    pub(super) fn get_surface_parameters(&self) -> (HINSTANCE, HWND) {
        (self.hinstance, self.handle)
    }

    pub(super) fn process_messages(&mut self) -> Result<OsWindowState, OsError> {
        loop {
            let bool_res = unsafe {
                WindowsAndMessaging::PeekMessageW(
                    &mut self.msg,
                    HWND::default(),
                    0,
                    0,
                    WindowsAndMessaging::PM_REMOVE,
                )
            };
            if bool_res.0 == 0 {
                return Ok(OsWindowState::Normal);
            } else if self.msg.message != WindowsAndMessaging::WM_USER {
                if self.msg.message != WindowsAndMessaging::WM_QUIT {
                    let _res = unsafe { WindowsAndMessaging::DispatchMessageW(&self.msg) };
                } else {
                    println!("Msg Info: {:?}", self.msg);
                    return Ok(OsWindowState::ShouldDrop);
                }
            } else {
                return Ok(OsWindowState::CloseAttempt);
            }
        }
    }

    pub(super) fn close_window(&mut self) -> Result<(), OsError> {
        if let Err(e) = unsafe { WindowsAndMessaging::DestroyWindow(self.handle) } {
            Err(OsError::Window(e))
        } else {
            Ok(())
        }
    }
}

pub struct OsEventSignaler {
    handle: HANDLE,
}

impl OsEventSignaler {
    pub fn signal(&mut self) -> Result<(), OsError> {
        if let Err(e) = unsafe { Threading::SetEvent(self.handle) } {
            Err(OsError::Event(e))
        } else {
            Ok(())
        }
    }
}

pub(super) struct OsEvent {
    handle: HANDLE,
}

impl OsEvent {
    pub(super) fn new() -> Result<Self, OsError> {
        match unsafe {
            Threading::CreateEventW(None, BOOL::from(false), BOOL::from(false), PCWSTR::null())
        } {
            Ok(handle) => Ok(OsEvent { handle }),
            Err(e) => Err(OsError::Event(e)),
        }
    }

    pub(super) fn create_signaler(&self) -> OsEventSignaler {
        OsEventSignaler {
            handle: self.handle,
        }
    }

    pub(super) fn check(&self) -> Result<bool, OsError> {
        let wait_event = unsafe { Threading::WaitForSingleObject(self.handle, 10) };
        if wait_event == WAIT_TIMEOUT {
            Ok(false)
        } else if wait_event == WAIT_OBJECT_0 {
            Ok(true)
        } else if wait_event == WAIT_FAILED {
            Err(OsError::Event(unsafe { GetLastError().into() }))
        } else {
            Err(OsError::UnexpectedEventCheckResult)
        }
    }
}

impl Drop for OsEvent {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
