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

use rustix::event::kqueue;
use rustix::{event::kqueue::kqueue, fd::AsRawFd};

#[derive(Debug)]
pub enum OsError {
    // Dxgi(Error),
    // Window(Error),
    // WindowAlreadyDisplayed,
    // UnknownMsgHandle,
    // Event(Error),
    // UnexpectedEventCheckResult,
    EventSetup,
    EventCheck,
    EventSignal,
    MainThreadMarker,
}

pub(super) fn get_device_luid() -> Result<Option<[u32; 2]>, OsError> {
    Ok(None)
}

use icrate::AppKit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSEventMask,
};
use icrate::Foundation::{
    ns_string, MainThreadMarker, NSCopying, NSDefaultRunLoopMode, NSNotification, NSObject,
    NSObjectProtocol, NSString,
};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};

#[derive(Debug)]
#[allow(unused)]
struct Ivars {
    ivar: u8,
    another_ivar: bool,
    box_ivar: Box<i32>,
    maybe_box_ivar: Option<Box<i32>>,
    id_ivar: Id<NSString>,
    maybe_id_ivar: Option<Id<NSString>>,
}

declare_class!(
    struct AppDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is an application delegate.
    // - `AppDelegate` does not implement `Drop`.
    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MyAppDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = Ivars;
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, notification: &NSNotification) {
            println!("Did finish launching!");
            // Do something with the notification
            dbg!(notification);
        }

        #[method(applicationWillTerminate:)]
        fn will_terminate(&self, _notification: &NSNotification) {
            println!("Will terminate!");
        }
    }
);

impl AppDelegate {
    fn new(ivar: u8, another_ivar: bool, mtm: MainThreadMarker) -> Id<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(Ivars {
            ivar,
            another_ivar,
            box_ivar: Box::new(2),
            maybe_box_ivar: None,
            id_ivar: NSString::from_str("abc"),
            maybe_id_ivar: Some(ns_string!("def").copy()),
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

pub(super) struct OsWindow {
    app: Id<NSApplication>,
}

pub(super) enum OsWindowState {
    Normal,
    CloseAttempt,
    Closing,
    ShouldDrop,
}

impl OsWindow {
    pub(super) fn new(width: u32, height: u32) -> Result<Self, OsError> {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        // configure the application delegate
        let delegate = AppDelegate::new(42, true, mtm);
        let object = ProtocolObject::from_ref(&*delegate);
        app.setDelegate(Some(object));

        // run the app
        //unsafe { app.run() };

        Ok(OsWindow { app })
    }

    pub(super) fn get_surface_parameters(&self) { // -> (HINSTANCE, HWND) {
                                                  // (self.hinstance, self.handle)
    }

    pub(super) fn process_messages(&mut self) -> Result<OsWindowState, OsError> {
        loop {
            let mode = unsafe { NSDefaultRunLoopMode };
            if let Some(next_event) = unsafe {
                self.app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    None,
                    mode,
                    true,
                )
            } {
                println!("Application Event: {:?}", next_event);
                unsafe { self.app.sendEvent(&next_event) };
                // if self.msg.message != WindowsAndMessaging::WM_USER {
                //     if self.msg.message != WindowsAndMessaging::WM_QUIT {
                //         let _res = unsafe { WindowsAndMessaging::DispatchMessageW(&self.msg) };
                //     } else {
                //         println!("Msg Info: {:?}", self.msg);
                //         return Ok(OsWindowState::ShouldDrop);
                //     }
                // } else {
                //     return Ok(OsWindowState::CloseAttempt);
                // }
            } else {
                return Ok(OsWindowState::Normal);
            }
        }
    }

    pub(super) fn close_window(&mut self) -> Result<(), OsError> {
        // if let Err(e) = unsafe { WindowsAndMessaging::DestroyWindow(self.handle) } {
        //     Err(OsError::Window(e))
        // } else {
        //     Ok(())
        // }
        Ok(())
    }
}

pub struct OsEventSignaler {
    kqueue_borrowed: rustix::fd::BorrowedFd<'static>,
    event_id: isize,
}

impl OsEventSignaler {
    pub fn signal(&mut self) -> Result<(), OsError> {
        let signaler_event = [kqueue::Event::new(
            kqueue::EventFilter::User {
                ident: self.event_id,
                flags: kqueue::UserFlags::TRIGGER,
                user_flags: kqueue::UserDefinedFlags::new(0),
            },
            kqueue::EventFlags::empty(),
            0,
        )];
        let mut eventlist = Vec::with_capacity(1);
        match unsafe {
            kqueue::kevent(
                self.kqueue_borrowed,
                &signaler_event,
                &mut eventlist,
                Some(std::time::Duration::from_millis(0)),
            )
        } {
            Ok(_n) => Ok(println!("Signaled: {}", _n)),
            Err(_e) => Err(OsError::EventSignal),
        }
    }
}

pub(super) struct OsEvent {
    kqueue_fd: rustix::fd::OwnedFd,
    event: [kqueue::Event; 1],
    eventlist: Vec<kqueue::Event>,
}

impl OsEvent {
    pub(super) fn new() -> Result<Self, OsError> {
        let kqueue_fd = match kqueue() {
            Ok(q) => q,
            Err(_e) => return Err(OsError::EventSetup),
        };

        let check_event = kqueue::Event::new(
            kqueue::EventFilter::User {
                ident: 42,
                flags: kqueue::UserFlags::empty(),
                user_flags: kqueue::UserDefinedFlags::new(0),
            },
            kqueue::EventFlags::ADD | kqueue::EventFlags::CLEAR,
            0,
        );
        Ok(OsEvent {
            kqueue_fd,
            event: [check_event],
            eventlist: Vec::with_capacity(1),
        })
    }

    pub(super) fn create_signaler(&self) -> OsEventSignaler {
        OsEventSignaler {
            kqueue_borrowed: unsafe {
                rustix::fd::BorrowedFd::borrow_raw(self.kqueue_fd.as_raw_fd())
            },
            event_id: 42,
        }
    }

    pub(super) fn check(&mut self) -> Result<bool, OsError> {
        match unsafe {
            kqueue::kevent(
                &self.kqueue_fd,
                &self.event,
                &mut self.eventlist,
                Some(std::time::Duration::from_millis(10)),
            )
        } {
            Ok(0) => Ok(false),
            Ok(_n) => Ok(true),
            Err(_e) => Err(OsError::EventCheck),
        }
    }
}

// impl Drop for OsEvent {
//     fn drop(&mut self) {
//         unsafe {
//             let _ = CloseHandle(self.handle);
//         }
//     }
// }
