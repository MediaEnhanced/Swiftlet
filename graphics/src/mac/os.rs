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

use objc2::ffi::NSInteger;
use rustix::event::kqueue;
use rustix::{event::kqueue::kqueue, fd::AsRawFd};

#[derive(Debug)]
pub enum OsError {
    NotMainThread,
    EventSetup,
    EventCheck,
    EventSignal,
    MainThreadMarker,
}

pub(super) fn get_device_luid() -> Result<Option<[u32; 2]>, OsError> {
    Ok(None)
}

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSWindow, NSWindowDelegate,
    NSWindowStyleMask,
};
use objc2_foundation::{
    ns_string, CGFloat, MainThreadMarker, NSDefaultRunLoopMode, NSNotification, NSObject,
    NSObjectProtocol, NSPoint, NSRect, NSSize,
};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};

struct AppVars {
    width: CGFloat,
    height: CGFloat,
}

impl AppDelegate {
    fn new(mtm: MainThreadMarker, width: CGFloat, height: CGFloat) -> Id<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(AppVars { width, height });
        unsafe { msg_send_id![super(this), init] }
    }
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
        type Ivars = AppVars;
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            //println!("Finished Launching!");
        }
    }
);

struct WindowVars {
    app: Id<NSApplication>,
}

impl WindowDelegate {
    fn new(mtm: MainThreadMarker) -> Id<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(WindowVars {
            app: NSApplication::sharedApplication(mtm),
        });
        unsafe { msg_send_id![super(this), init] }
    }

    fn post_event(&self, data: NSInteger) {
        if let Some(post_event) = unsafe {
            NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined, NSPoint::ZERO, NSEventModifierFlags(0), 
            0.0, 0, None, 0, data, 0)
        } {
            self.ivars().app.postEvent_atStart(&post_event, false);
        }
    }
}

declare_class!(
    struct WindowDelegate;

    unsafe impl ClassType for WindowDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MyWindowDelegate";
    }

    impl DeclaredClass for WindowDelegate {
        type Ivars = WindowVars;
    }

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[method(windowWillClose:)]
        fn window_will_close(&self, _notification: &NSNotification) {
            println!("Window Closing!");
            self.post_event(0);
        }

        #[method(windowShouldClose:)]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            println!("Trying to close!");
            self.post_event(1);
            false
        }
    }
);

pub(super) struct OsWindow {
    mtm: MainThreadMarker,
    app: Id<NSApplication>,
    window: Id<NSWindow>,
}

pub(super) enum OsWindowState {
    Normal,
    CloseAttempt,
    Closing,
    ShouldDrop,
}

impl OsWindow {
    pub(super) fn new(width: u32, height: u32) -> Result<Self, OsError> {
        let mtm = match MainThreadMarker::new() {
            Some(m) => m,
            None => return Err(OsError::NotMainThread),
        };

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        let delegate = AppDelegate::new(mtm, width as f64, height as f64);
        let object = ProtocolObject::from_ref(&*delegate);
        app.setDelegate(Some(object));

        unsafe { app.finishLaunching() };

        let window = {
            let content_rect = NSRect::new(
                NSPoint::new(0., 0.),
                NSSize::new(width as f64, height as f64),
            );
            let style = NSWindowStyleMask(
                NSWindowStyleMask::Closable.0
                    | NSWindowStyleMask::Resizable.0
                    | NSWindowStyleMask::Titled.0,
            );
            unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    content_rect,
                    style,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                )
            }
        };

        let delegate = WindowDelegate::new(mtm);
        let object = ProtocolObject::from_ref(&*delegate);
        window.setDelegate(Some(object));

        window.center();
        window.setTitle(ns_string!("Window Title"));
        window.makeKeyAndOrderFront(None);

        // if let Some(cv) = window.contentView() {
        //     println!("Content View: {:?}", cv);
        //     cv.setWantsLayer(true);
        //     //let l = cv.make
        // }

        Ok(OsWindow { mtm, app, window })
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
                if unsafe { next_event.r#type() } != NSEventType::ApplicationDefined {
                    unsafe { self.app.sendEvent(&next_event) };
                    // unsafe { self.app.updateWindows() };
                } else {
                    match unsafe { next_event.data1() } {
                        1 => return Ok(OsWindowState::CloseAttempt),
                        _ => return Ok(OsWindowState::ShouldDrop),
                    }
                }
            } else {
                return Ok(OsWindowState::Normal);
            }
        }
    }

    pub(super) fn close_window(&mut self) -> Result<(), OsError> {
        //unsafe { self.app.terminate(None) };
        self.window.close();
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
                flags: kqueue::UserFlags::TRIGGER | kqueue::UserFlags::COPY,
                user_flags: kqueue::UserDefinedFlags::new(0),
            },
            kqueue::EventFlags::empty(),
            0,
        )];
        let mut eventlist = Vec::with_capacity(0);
        match unsafe {
            kqueue::kevent(
                self.kqueue_borrowed,
                &signaler_event,
                &mut eventlist,
                Some(std::time::Duration::from_millis(0)),
            )
        } {
            Ok(_n) => Ok(()),
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

        let add_event = [kqueue::Event::new(
            kqueue::EventFilter::User {
                ident: 42,
                flags: kqueue::UserFlags::COPY,
                user_flags: kqueue::UserDefinedFlags::new(0),
            },
            kqueue::EventFlags::ADD | kqueue::EventFlags::CLEAR,
            0,
        )];
        let mut eventlist = Vec::with_capacity(1);
        if let Err(_e) = unsafe {
            kqueue::kevent(
                &kqueue_fd,
                &add_event,
                &mut eventlist,
                Some(std::time::Duration::from_millis(0)),
            )
        } {
            return Err(OsError::EventSetup);
        }

        let check_event = kqueue::Event::new(
            kqueue::EventFilter::User {
                ident: 42,
                flags: kqueue::UserFlags::COPY,
                user_flags: kqueue::UserDefinedFlags::new(0),
            },
            kqueue::EventFlags::CLEAR,
            0,
        );
        Ok(OsEvent {
            kqueue_fd,
            event: [check_event],
            eventlist,
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
        //let start_instant = Instant::now();
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
