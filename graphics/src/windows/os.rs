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
use windows::Win32::UI::{HiDpi, Input::KeyboardAndMouse, WindowsAndMessaging};

mod dxgi;
mod manifest;

#[derive(Debug)]
pub enum OsError {
    Dxgi(Error),
    Window(Error),
    WindowAlreadyDisplayed,
    UnknownMsgHandle,
    Event(Error),
    UnexpectedEventCheckResult,
    TimerSet,
}

pub(super) fn get_device_luid() -> Result<Option<[u32; 2]>, OsError> {
    let interface = match dxgi::Interface::new(false) {
        Ok(i) => i,
        Err(e) => return Err(OsError::Dxgi(e)),
    };
    Ok(Some(interface.get_luid()))
}

#[derive(Debug)]
pub enum KeyCode {
    Unknown,
    LeftMouse, // Forgot if it is considered primary for switch buttons
    RightMouse,
    MiddleMouse,
    X1Mouse,
    X2Mouse,
    Backspace,
    Tab,
    Enter,
    Escape,
    Space,
    LeftArrow,
    UpArrow,
    RightArrow,
    DownArrow,
    Char(char),
    Chars(([char; 7], usize)),
}

impl KeyCode {
    fn get_from_virtual_code(virtual_key_code: u32, scan_code: u32) -> Self {
        // println!(
        //     "Virtual Key Code | Scan Code: {} | {}",
        //     virtual_key_code, scan_code
        // );
        match KeyboardAndMouse::VIRTUAL_KEY(virtual_key_code as u16) {
            KeyboardAndMouse::VK_LBUTTON => Self::LeftMouse,
            KeyboardAndMouse::VK_RBUTTON => Self::RightMouse,
            KeyboardAndMouse::VK_MBUTTON => Self::MiddleMouse,
            KeyboardAndMouse::VK_XBUTTON1 => Self::X1Mouse,
            KeyboardAndMouse::VK_XBUTTON2 => Self::X2Mouse,
            KeyboardAndMouse::VK_BACK => Self::Backspace,
            KeyboardAndMouse::VK_TAB => Self::Tab,
            KeyboardAndMouse::VK_RETURN => Self::Enter,
            KeyboardAndMouse::VK_ESCAPE => Self::Escape,
            KeyboardAndMouse::VK_SPACE => Self::Space,
            KeyboardAndMouse::VK_LEFT => Self::LeftArrow,
            KeyboardAndMouse::VK_UP => Self::UpArrow,
            KeyboardAndMouse::VK_RIGHT => Self::RightArrow,
            KeyboardAndMouse::VK_DOWN => Self::DownArrow,
            _ => {
                let mut keyboard_state = [0; 256];
                unsafe { KeyboardAndMouse::GetKeyboardState(&mut keyboard_state) }.unwrap();
                let mut u16_buff = [0; 8];
                let code_units = unsafe {
                    KeyboardAndMouse::ToUnicode(
                        virtual_key_code,
                        scan_code,
                        Some(&keyboard_state),
                        &mut u16_buff[..7],
                        0x4,
                    )
                };
                if code_units > 0 {
                    let buff_iter = u16_buff.into_iter();
                    let decode_iter = char::decode_utf16(buff_iter);
                    let mut char_buff = ['\0'; 7];
                    let mut char_buff_len = 0;
                    for char_res in decode_iter {
                        match char_res {
                            Ok(c) => {
                                if c == '\0' {
                                    break;
                                }
                                char_buff[char_buff_len] = c;
                                char_buff_len += 1;
                            }
                            Err(_e) => {
                                char_buff[char_buff_len] = char::REPLACEMENT_CHARACTER;
                                char_buff_len += 1;
                            }
                        }
                    }
                    #[allow(clippy::comparison_chain)]
                    if char_buff_len == 1 {
                        Self::Char(char_buff[0])
                    } else if char_buff_len > 1 {
                        Self::Chars((char_buff, char_buff_len))
                    } else {
                        Self::Unknown
                    }
                } else {
                    Self::Unknown
                }
            }
        }
    }
}

#[repr(isize)]
enum CallbackResult {
    Destroy = 0,
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
            LRESULT(CallbackResult::Destroy as isize)
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
    KeyPressed(KeyCode),
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
                if self.msg.message == WindowsAndMessaging::WM_KEYDOWN {
                    if (self.msg.lParam.0 & 0x40000000) == 0 {
                        // Key was up previously
                        let virtual_key_code = self.msg.wParam.0 as u32;
                        let scan_code = (self.msg.lParam.0 >> 16) & 0xFF;
                        return Ok(OsWindowState::KeyPressed(KeyCode::get_from_virtual_code(
                            virtual_key_code,
                            scan_code as u32,
                        )));
                    }
                } else if self.msg.message != WindowsAndMessaging::WM_QUIT {
                    let _res = unsafe { WindowsAndMessaging::DispatchMessageW(&self.msg) };
                } else {
                    //println!("Msg Info: {:?}", self.msg);
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

    pub(super) fn get_dpi(&self) -> u32 {
        unsafe { HiDpi::GetDpiForWindow(self.handle) }
    }
}

pub(super) struct OsWait {
    handle: HANDLE,
}

impl OsWait {
    pub(super) fn new() -> Result<Self, OsError> {
        let timer_name = PCWSTR(std::ptr::null());
        match unsafe {
            Threading::CreateWaitableTimerExW(
                None,
                timer_name,
                Threading::CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                Threading::TIMER_MODIFY_STATE.0 | Threading::SYNCHRONIZATION_SYNCHRONIZE.0,
            )
        } {
            Ok(handle) => Ok(OsWait { handle }),
            Err(e) => Err(OsError::Event(e)),
        }
    }

    pub(super) fn wait(&self, timeout_duration: std::time::Duration) -> Result<bool, OsError> {
        let time_convert = (timeout_duration.as_secs() * 10_000_000)
            + (timeout_duration.subsec_nanos() as u64 / 100);
        let relative_time = -(time_convert as i64);
        match unsafe {
            Threading::SetWaitableTimer(self.handle, &relative_time, 0, None, None, BOOL(0))
        } {
            Ok(_) => {
                let millisecond_timeout = (timeout_duration.as_millis() as u32) + 100;
                let wait_event =
                    unsafe { Threading::WaitForSingleObject(self.handle, millisecond_timeout) };
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
            Err(_e) => Err(OsError::TimerSet),
        }
    }
}

impl Drop for OsWait {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
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
