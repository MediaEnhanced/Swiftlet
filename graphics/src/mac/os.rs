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
    NoContentView,
    EventSetup,
    EventCheck,
    EventSignal,
}

pub(super) fn get_device_luid() -> Result<Option<[u32; 2]>, OsError> {
    Ok(None)
}

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSWindow, NSWindowDelegate, NSWindowStyleMask, CAMetalLayer
};
use objc2_foundation::{
    ns_string, CGFloat, MainThreadMarker, NSDefaultRunLoopMode, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect,  NSSize};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class,   msg_send_id, mutability, ClassType, DeclaredClass};

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
            //println!("Window Closing!");
            self.post_event(0);
        }

        #[method(windowShouldClose:)]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            //println!("Trying to close!");
            self.post_event(1);
            false
        }
    }
);

pub(super) struct OsWindow {
    mtm: MainThreadMarker,
    app: Id<NSApplication>,
    window: Id<NSWindow>,
    layer: Id<CAMetalLayer>,
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

        let layer = match window.contentView() {
            Some(cv) => {
                let l = unsafe {CAMetalLayer::new()} ;
                //println!("Content View: {:?}", cv);
                cv.setWantsLayer(true);
                cv.setLayer(Some(l.as_super()));
                l
            } None =>{
                return Err(OsError::NoContentView);
            }
        };

        Ok(OsWindow { mtm, app, window, layer })
    }

    pub(super) fn get_surface_parameters(&self) -> super::vulkan::api::CAMetalLayerPtr {
        let layer_ptr = self.layer.as_ref() as *const CAMetalLayer;
        layer_ptr as super::vulkan::api::CAMetalLayerPtr
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

// // Append to icrate/framework-crates/objc2-app-kit/src/generated/mod.rs
// #[cfg(feature = "AppKit_NSView")]
// pub use self::__NSView::CALayer;
// #[cfg(feature = "AppKit_NSView")]
// pub use self::__NSView::CAMetalLayer;

// // Append to icrate/framework-crates/objc2-app-kit/src/generated/NSView.rs

// pub type CFTimeInterval = std::os::raw::c_double;
// pub type CAMediaTimingFillMode = NSString;

// extern_protocol!(
//     pub unsafe trait CAMediaTiming {
//         #[method(beginTime)]
//         unsafe fn beginTime(&self) -> CFTimeInterval;

//         #[method(setBeginTime:)]
//         unsafe fn setBeginTime(&self, begin_time: CFTimeInterval);

//         #[method(duration)]
//         unsafe fn duration(&self) -> CFTimeInterval;

//         #[method(setDuration:)]
//         unsafe fn setDuration(&self, duration: CFTimeInterval);

//         #[method(speed)]
//         unsafe fn speed(&self) -> c_float;

//         #[method(setSpeed:)]
//         unsafe fn setSpeed(&self, speed: c_float);

//         #[method(timeOffset)]
//         unsafe fn timeOffset(&self) -> CFTimeInterval;

//         #[method(setTimeOffset:)]
//         unsafe fn setTimeOffset(&self, time_offset: CFTimeInterval);

//         #[method(repeatCount)]
//         unsafe fn repeatCount(&self) -> c_float;

//         #[method(setRepeatCount:)]
//         unsafe fn setRepeatCount(&self, repeat_count: c_float);

//         #[method(repeatDuration)]
//         unsafe fn repeatDuration(&self) -> CFTimeInterval;

//         #[method(setRepeatDuration:)]
//         unsafe fn setRepeatDuration(&self, repeat_duration: CFTimeInterval);

//         #[method(autoreverses)]
//         unsafe fn autoreverses(&self) -> bool;

//         #[method(setAutoreverses:)]
//         unsafe fn setAutoreverses(&self, autoreverses: bool);

//         #[method_id(@__retain_semantics Other fillMode)]
//         unsafe fn fillMode(&self) -> Id<CAMediaTimingFillMode>;

//         #[method(setFillMode:)]
//         unsafe fn setFillMode(&self, fill_mode: &CAMediaTimingFillMode);
//     }

//     unsafe impl ProtocolType for dyn CAMediaTiming {}
// );

// extern "C" {
//     pub static kCAFillModeForwards: &'static CAMediaTimingFillMode;
// }

// extern "C" {
//     pub static kCAFillModeBackwards: &'static CAMediaTimingFillMode;
// }

// extern "C" {
//     pub static kCAFillModeBoth: &'static CAMediaTimingFillMode;
// }

// extern "C" {
//     pub static kCAFillModeRemoved: &'static CAMediaTimingFillMode;
// }

// extern_class!(
//     #[derive(Debug, PartialEq, Eq, Hash)]
//     pub struct CALayer;

//     unsafe impl ClassType for CALayer {
//         type Super = NSObject;
//         type Mutability = InteriorMutable;
//     }
// );

// unsafe impl CAMediaTiming for CALayer {}

// unsafe impl NSCoding for CALayer {}

// unsafe impl NSObjectProtocol for CALayer {}

// unsafe impl NSSecureCoding for CALayer {}

// extern_methods!(
//     unsafe impl CALayer {
//         #[method_id(@__retain_semantics Other layer)]
//         pub fn layer() -> Id<Self>;

//         #[method_id(@__retain_semantics Init init)]
//         pub fn init(this: Allocated<Self>) -> Id<Self>;

//         #[method_id(@__retain_semantics Init initWithLayer:)]
//         pub unsafe fn initWithLayer(this: Allocated<Self>, layer: &AnyObject) -> Id<Self>;

//         #[method_id(@__retain_semantics Other presentationLayer)]
//         pub unsafe fn presentationLayer(&self) -> Option<Id<Self>>;

//         #[method_id(@__retain_semantics Other modelLayer)]
//         pub unsafe fn modelLayer(&self) -> Id<Self>;

//         // #[method_id(@__retain_semantics Other defaultValueForKey:)]
//         // pub unsafe fn defaultValueForKey(key: &NSString) -> Option<Id<AnyObject>>;

//         // #[method(needsDisplayForKey:)]
//         // pub unsafe fn needsDisplayForKey(key: &NSString) -> bool;

//         // #[method(shouldArchiveValueForKey:)]
//         // pub unsafe fn shouldArchiveValueForKey(&self, key: &NSString) -> bool;

//         // #[method(bounds)]
//         // pub fn bounds(&self) -> CGRect;

//         // #[method(setBounds:)]
//         // pub fn setBounds(&self, bounds: CGRect);

//         // #[method(position)]
//         // pub fn position(&self) -> CGPoint;

//         // #[method(setPosition:)]
//         // pub fn setPosition(&self, position: CGPoint);

//         // #[method(zPosition)]
//         // pub fn zPosition(&self) -> CGFloat;

//         // #[method(setZPosition:)]
//         // pub fn setZPosition(&self, z_position: CGFloat);

//         // #[method(anchorPoint)]
//         // pub fn anchorPoint(&self) -> CGPoint;

//         // #[method(setAnchorPoint:)]
//         // pub fn setAnchorPoint(&self, anchor_point: CGPoint);

//         // #[method(anchorPointZ)]
//         // pub fn anchorPointZ(&self) -> CGFloat;

//         // #[method(setAnchorPointZ:)]
//         // pub fn setAnchorPointZ(&self, anchor_point_z: CGFloat);

//         // #[cfg(feature = "QuartzCore_CATransform3D")]
//         // #[method(transform)]
//         // pub fn transform(&self) -> CATransform3D;

//         // #[cfg(feature = "QuartzCore_CATransform3D")]
//         // #[method(setTransform:)]
//         // pub fn setTransform(&self, transform: CATransform3D);

//         // #[method(frame)]
//         // pub fn frame(&self) -> CGRect;

//         // #[method(setFrame:)]
//         // pub fn setFrame(&self, frame: CGRect);

//         // #[method(isHidden)]
//         // pub fn isHidden(&self) -> bool;

//         // #[method(setHidden:)]
//         // pub fn setHidden(&self, hidden: bool);

//         // #[method(isDoubleSided)]
//         // pub fn isDoubleSided(&self) -> bool;

//         // #[method(setDoubleSided:)]
//         // pub fn setDoubleSided(&self, double_sided: bool);

//         // #[method(isGeometryFlipped)]
//         // pub fn isGeometryFlipped(&self) -> bool;

//         // #[method(setGeometryFlipped:)]
//         // pub fn setGeometryFlipped(&self, geometry_flipped: bool);

//         // #[method(contentsAreFlipped)]
//         // pub fn contentsAreFlipped(&self) -> bool;

//         // #[method_id(@__retain_semantics Other superlayer)]
//         // pub fn superlayer(&self) -> Option<Id<CALayer>>;

//         // #[method(removeFromSuperlayer)]
//         // pub fn removeFromSuperlayer(&self);

//         // #[method_id(@__retain_semantics Other sublayers)]
//         // pub unsafe fn sublayers(&self) -> Option<Id<NSArray<CALayer>>>;

//         // #[method(setSublayers:)]
//         // pub unsafe fn setSublayers(&self, sublayers: Option<&NSArray<CALayer>>);

//         // #[method(addSublayer:)]
//         // pub fn addSublayer(&self, layer: &CALayer);

//         // #[method(insertSublayer:atIndex:)]
//         // pub fn insertSublayer_atIndex(&self, layer: &CALayer, idx: c_uint);

//         // #[method(insertSublayer:below:)]
//         // pub fn insertSublayer_below(&self, layer: &CALayer, sibling: Option<&CALayer>);

//         // #[method(insertSublayer:above:)]
//         // pub fn insertSublayer_above(&self, layer: &CALayer, sibling: Option<&CALayer>);

//         // #[method(replaceSublayer:with:)]
//         // pub unsafe fn replaceSublayer_with(&self, old_layer: &CALayer, new_layer: &CALayer);

//         // #[cfg(feature = "QuartzCore_CATransform3D")]
//         // #[method(sublayerTransform)]
//         // pub fn sublayerTransform(&self) -> CATransform3D;

//         // #[cfg(feature = "QuartzCore_CATransform3D")]
//         // #[method(setSublayerTransform:)]
//         // pub fn setSublayerTransform(&self, sublayer_transform: CATransform3D);

//         // #[method_id(@__retain_semantics Other mask)]
//         // pub fn mask(&self) -> Option<Id<CALayer>>;

//         // #[method(setMask:)]
//         // pub unsafe fn setMask(&self, mask: Option<&CALayer>);

//         // #[method(masksToBounds)]
//         // pub fn masksToBounds(&self) -> bool;

//         // #[method(setMasksToBounds:)]
//         // pub fn setMasksToBounds(&self, masks_to_bounds: bool);

//         // #[method(convertPoint:fromLayer:)]
//         // pub fn convertPoint_fromLayer(&self, p: CGPoint, l: Option<&CALayer>) -> CGPoint;

//         // #[method(convertPoint:toLayer:)]
//         // pub fn convertPoint_toLayer(&self, p: CGPoint, l: Option<&CALayer>) -> CGPoint;

//         // #[method(convertRect:fromLayer:)]
//         // pub fn convertRect_fromLayer(&self, r: CGRect, l: Option<&CALayer>) -> CGRect;

//         // #[method(convertRect:toLayer:)]
//         // pub fn convertRect_toLayer(&self, r: CGRect, l: Option<&CALayer>) -> CGRect;

//         // #[method(convertTime:fromLayer:)]
//         // pub fn convertTime_fromLayer(
//         //     &self,
//         //     t: CFTimeInterval,
//         //     l: Option<&CALayer>,
//         // ) -> CFTimeInterval;

//         // #[method(convertTime:toLayer:)]
//         // pub fn convertTime_toLayer(&self, t: CFTimeInterval, l: Option<&CALayer>)
//         //     -> CFTimeInterval;

//         // #[method_id(@__retain_semantics Other hitTest:)]
//         // pub fn hitTest(&self, p: CGPoint) -> Option<Id<CALayer>>;

//         // #[method(containsPoint:)]
//         // pub fn containsPoint(&self, p: CGPoint) -> bool;

//         // #[method_id(@__retain_semantics Other contents)]
//         // pub unsafe fn contents(&self) -> Option<Id<AnyObject>>;

//         // #[method(setContents:)]
//         // pub unsafe fn setContents(&self, contents: Option<&AnyObject>);

//         // #[method(contentsRect)]
//         // pub fn contentsRect(&self) -> CGRect;

//         // #[method(setContentsRect:)]
//         // pub fn setContentsRect(&self, contents_rect: CGRect);

//         // #[method_id(@__retain_semantics Other contentsGravity)]
//         // pub fn contentsGravity(&self) -> Id<CALayerContentsGravity>;

//         // #[method(setContentsGravity:)]
//         // pub fn setContentsGravity(&self, contents_gravity: &CALayerContentsGravity);

//         // #[method(contentsScale)]
//         // pub fn contentsScale(&self) -> CGFloat;

//         // #[method(setContentsScale:)]
//         // pub fn setContentsScale(&self, contents_scale: CGFloat);

//         // #[method(contentsCenter)]
//         // pub fn contentsCenter(&self) -> CGRect;

//         // #[method(setContentsCenter:)]
//         // pub fn setContentsCenter(&self, contents_center: CGRect);

//         // #[method_id(@__retain_semantics Other contentsFormat)]
//         // pub fn contentsFormat(&self) -> Id<CALayerContentsFormat>;

//         // #[method(setContentsFormat:)]
//         // pub fn setContentsFormat(&self, contents_format: &CALayerContentsFormat);

//         // #[method(wantsExtendedDynamicRangeContent)]
//         // pub unsafe fn wantsExtendedDynamicRangeContent(&self) -> bool;

//         // #[method(setWantsExtendedDynamicRangeContent:)]
//         // pub unsafe fn setWantsExtendedDynamicRangeContent(
//         //     &self,
//         //     wants_extended_dynamic_range_content: bool,
//         // );

//         // #[method_id(@__retain_semantics Other minificationFilter)]
//         // pub fn minificationFilter(&self) -> Id<CALayerContentsFilter>;

//         // #[method(setMinificationFilter:)]
//         // pub fn setMinificationFilter(&self, minification_filter: &CALayerContentsFilter);

//         // #[method_id(@__retain_semantics Other magnificationFilter)]
//         // pub fn magnificationFilter(&self) -> Id<CALayerContentsFilter>;

//         // #[method(setMagnificationFilter:)]
//         // pub fn setMagnificationFilter(&self, magnification_filter: &CALayerContentsFilter);

//         // #[method(minificationFilterBias)]
//         // pub fn minificationFilterBias(&self) -> c_float;

//         // #[method(setMinificationFilterBias:)]
//         // pub fn setMinificationFilterBias(&self, minification_filter_bias: c_float);

//         // #[method(isOpaque)]
//         // pub fn isOpaque(&self) -> bool;

//         // #[method(setOpaque:)]
//         // pub fn setOpaque(&self, opaque: bool);

//         // #[method(display)]
//         // pub fn display(&self);

//         // #[method(setNeedsDisplay)]
//         // pub fn setNeedsDisplay(&self);

//         // #[method(setNeedsDisplayInRect:)]
//         // pub fn setNeedsDisplayInRect(&self, r: CGRect);

//         // #[method(needsDisplay)]
//         // pub fn needsDisplay(&self) -> bool;

//         // #[method(displayIfNeeded)]
//         // pub fn displayIfNeeded(&self);

//         // #[method(needsDisplayOnBoundsChange)]
//         // pub fn needsDisplayOnBoundsChange(&self) -> bool;

//         // #[method(setNeedsDisplayOnBoundsChange:)]
//         // pub fn setNeedsDisplayOnBoundsChange(&self, needs_display_on_bounds_change: bool);

//         // #[method(drawsAsynchronously)]
//         // pub fn drawsAsynchronously(&self) -> bool;

//         // #[method(setDrawsAsynchronously:)]
//         // pub fn setDrawsAsynchronously(&self, draws_asynchronously: bool);

//         // #[method(edgeAntialiasingMask)]
//         // pub fn edgeAntialiasingMask(&self) -> CAEdgeAntialiasingMask;

//         // #[method(setEdgeAntialiasingMask:)]
//         // pub fn setEdgeAntialiasingMask(&self, edge_antialiasing_mask: CAEdgeAntialiasingMask);

//         // #[method(allowsEdgeAntialiasing)]
//         // pub fn allowsEdgeAntialiasing(&self) -> bool;

//         // #[method(setAllowsEdgeAntialiasing:)]
//         // pub fn setAllowsEdgeAntialiasing(&self, allows_edge_antialiasing: bool);

//         // #[method(cornerRadius)]
//         // pub fn cornerRadius(&self) -> CGFloat;

//         // #[method(setCornerRadius:)]
//         // pub fn setCornerRadius(&self, corner_radius: CGFloat);

//         // #[method(maskedCorners)]
//         // pub fn maskedCorners(&self) -> CACornerMask;

//         // #[method(setMaskedCorners:)]
//         // pub fn setMaskedCorners(&self, masked_corners: CACornerMask);

//         // #[method_id(@__retain_semantics Other cornerCurve)]
//         // pub fn cornerCurve(&self) -> Id<CALayerCornerCurve>;

//         // #[method(setCornerCurve:)]
//         // pub fn setCornerCurve(&self, corner_curve: &CALayerCornerCurve);

//         // #[method(cornerCurveExpansionFactor:)]
//         // pub fn cornerCurveExpansionFactor(curve: &CALayerCornerCurve) -> CGFloat;

//         // #[method(borderWidth)]
//         // pub fn borderWidth(&self) -> CGFloat;

//         // #[method(setBorderWidth:)]
//         // pub fn setBorderWidth(&self, border_width: CGFloat);

//         // #[method(opacity)]
//         // pub fn opacity(&self) -> c_float;

//         // #[method(setOpacity:)]
//         // pub fn setOpacity(&self, opacity: c_float);

//         // #[method(allowsGroupOpacity)]
//         // pub fn allowsGroupOpacity(&self) -> bool;

//         // #[method(setAllowsGroupOpacity:)]
//         // pub fn setAllowsGroupOpacity(&self, allows_group_opacity: bool);

//         // #[method_id(@__retain_semantics Other compositingFilter)]
//         // pub unsafe fn compositingFilter(&self) -> Option<Id<AnyObject>>;

//         // #[method(setCompositingFilter:)]
//         // pub unsafe fn setCompositingFilter(&self, compositing_filter: Option<&AnyObject>);

//         // #[method_id(@__retain_semantics Other filters)]
//         // pub unsafe fn filters(&self) -> Option<Id<NSArray>>;

//         // #[method(setFilters:)]
//         // pub unsafe fn setFilters(&self, filters: Option<&NSArray>);

//         // #[method_id(@__retain_semantics Other backgroundFilters)]
//         // pub unsafe fn backgroundFilters(&self) -> Option<Id<NSArray>>;

//         // #[method(setBackgroundFilters:)]
//         // pub unsafe fn setBackgroundFilters(&self, background_filters: Option<&NSArray>);

//         // #[method(shouldRasterize)]
//         // pub fn shouldRasterize(&self) -> bool;

//         // #[method(setShouldRasterize:)]
//         // pub fn setShouldRasterize(&self, should_rasterize: bool);

//         // #[method(rasterizationScale)]
//         // pub fn rasterizationScale(&self) -> CGFloat;

//         // #[method(setRasterizationScale:)]
//         // pub fn setRasterizationScale(&self, rasterization_scale: CGFloat);

//         // #[method(shadowOpacity)]
//         // pub fn shadowOpacity(&self) -> c_float;

//         // #[method(setShadowOpacity:)]
//         // pub fn setShadowOpacity(&self, shadow_opacity: c_float);

//         // #[method(shadowOffset)]
//         // pub fn shadowOffset(&self) -> CGSize;

//         // #[method(setShadowOffset:)]
//         // pub fn setShadowOffset(&self, shadow_offset: CGSize);

//         // #[method(shadowRadius)]
//         // pub fn shadowRadius(&self) -> CGFloat;

//         // #[method(setShadowRadius:)]
//         // pub fn setShadowRadius(&self, shadow_radius: CGFloat);

//         // #[method(autoresizingMask)]
//         // pub fn autoresizingMask(&self) -> CAAutoresizingMask;

//         // #[method(setAutoresizingMask:)]
//         // pub fn setAutoresizingMask(&self, autoresizing_mask: CAAutoresizingMask);

//         // #[method_id(@__retain_semantics Other layoutManager)]
//         // pub fn layoutManager(&self) -> Option<Id<ProtocolObject<dyn CALayoutManager>>>;

//         // #[method(setLayoutManager:)]
//         // pub fn setLayoutManager(
//         //     &self,
//         //     layout_manager: Option<&ProtocolObject<dyn CALayoutManager>>,
//         // );

//         // #[method(preferredFrameSize)]
//         // pub fn preferredFrameSize(&self) -> CGSize;

//         // #[method(setNeedsLayout)]
//         // pub fn setNeedsLayout(&self);

//         // #[method(needsLayout)]
//         // pub fn needsLayout(&self) -> bool;

//         // #[method(layoutIfNeeded)]
//         // pub fn layoutIfNeeded(&self);

//         // #[method(layoutSublayers)]
//         // pub fn layoutSublayers(&self);

//         // #[method(resizeSublayersWithOldSize:)]
//         // pub fn resizeSublayersWithOldSize(&self, size: CGSize);

//         // #[method(resizeWithOldSuperlayerSize:)]
//         // pub fn resizeWithOldSuperlayerSize(&self, size: CGSize);

//         // #[method_id(@__retain_semantics Other defaultActionForKey:)]
//         // pub fn defaultActionForKey(event: &NSString) -> Option<Id<ProtocolObject<dyn CAAction>>>;

//         // #[method_id(@__retain_semantics Other actionForKey:)]
//         // pub fn actionForKey(&self, event: &NSString) -> Option<Id<ProtocolObject<dyn CAAction>>>;

//         // #[method_id(@__retain_semantics Other actions)]
//         // pub fn actions(&self) -> Option<Id<NSDictionary<NSString, ProtocolObject<dyn CAAction>>>>;

//         // #[method(setActions:)]
//         // pub fn setActions(
//         //     &self,
//         //     actions: Option<&NSDictionary<NSString, ProtocolObject<dyn CAAction>>>,
//         // );

//         // #[cfg(feature = "QuartzCore_CAAnimation")]
//         // #[method(addAnimation:forKey:)]
//         // pub fn addAnimation_forKey(&self, anim: &CAAnimation, key: Option<&NSString>);

//         // #[method(removeAllAnimations)]
//         // pub fn removeAllAnimations(&self);

//         // #[method(removeAnimationForKey:)]
//         // pub fn removeAnimationForKey(&self, key: &NSString);

//         // #[method_id(@__retain_semantics Other animationKeys)]
//         // pub fn animationKeys(&self) -> Option<Id<NSArray<NSString>>>;

//         // #[cfg(feature = "QuartzCore_CAAnimation")]
//         // #[method_id(@__retain_semantics Other animationForKey:)]
//         // pub unsafe fn animationForKey(&self, key: &NSString) -> Option<Id<CAAnimation>>;

//         // #[method_id(@__retain_semantics Other name)]
//         // pub fn name(&self) -> Option<Id<NSString>>;

//         // #[method(setName:)]
//         // pub fn setName(&self, name: Option<&NSString>);

//         // #[method_id(@__retain_semantics Other delegate)]
//         // pub fn delegate(&self) -> Option<Id<ProtocolObject<dyn CALayerDelegate>>>;

//         // #[method(setDelegate:)]
//         // pub fn setDelegate(&self, delegate: Option<&ProtocolObject<dyn CALayerDelegate>>);

//         // #[method_id(@__retain_semantics Other style)]
//         // pub unsafe fn style(&self) -> Option<Id<NSDictionary>>;

//         // #[method(setStyle:)]
//         // pub unsafe fn setStyle(&self, style: Option<&NSDictionary>);
//     }
// );

// extern_methods!(
//     /// Methods declared on superclass `NSObject`
//     unsafe impl CALayer {
//         #[method_id(@__retain_semantics New new)]
//         pub fn new() -> Id<Self>;
//     }
// );

// extern_class!(
//     #[derive(Debug, PartialEq, Eq, Hash)]
//     pub struct CAMetalLayer;

//     unsafe impl ClassType for CAMetalLayer {
//         #[inherits(NSObject)]
//         type Super = CALayer;
//         type Mutability = InteriorMutable;
//     }
// );

// unsafe impl CAMediaTiming for CAMetalLayer {}
// unsafe impl NSCoding for CAMetalLayer {}
// unsafe impl NSSecureCoding for CAMetalLayer {}

// //unsafe impl NSObjectProtocol for CAPropertyAnimation {}

// extern_methods!(
//     unsafe impl CAMetalLayer {
//         // #[method_id(@__retain_semantics Other animationWithKeyPath:)]
//         // pub unsafe fn animationWithKeyPath(path: Option<&NSString>) -> Id<Self>;

//         // #[method_id(@__retain_semantics Other keyPath)]
//         // pub unsafe fn keyPath(&self) -> Option<Id<NSString>>;

//         // #[method(setKeyPath:)]
//         // pub unsafe fn setKeyPath(&self, key_path: Option<&NSString>);

//         // #[method(isAdditive)]
//         // pub unsafe fn isAdditive(&self) -> bool;

//         // #[method(setAdditive:)]
//         // pub unsafe fn setAdditive(&self, additive: bool);

//         // #[method(isCumulative)]
//         // pub unsafe fn isCumulative(&self) -> bool;

//         // #[method(setCumulative:)]
//         // pub unsafe fn setCumulative(&self, cumulative: bool);

//         // #[cfg(feature = "QuartzCore_CAValueFunction")]
//         // #[method_id(@__retain_semantics Other valueFunction)]
//         // pub unsafe fn valueFunction(&self) -> Option<Id<CAValueFunction>>;

//         // #[cfg(feature = "QuartzCore_CAValueFunction")]
//         // #[method(setValueFunction:)]
//         // pub unsafe fn setValueFunction(&self, value_function: Option<&CAValueFunction>);
//     }
// );

// extern_methods!(
//     /// Methods declared on superclass `CALayer`
//     unsafe impl CAMetalLayer {
//         #[method_id(@__retain_semantics Other layer)]
//         pub fn layer() -> Id<Self>;

//         #[method_id(@__retain_semantics Init initWithLayer:)]
//         pub unsafe fn initWithLayer(this: Allocated<Self>, layer: &AnyObject) -> Id<Self>;

//         #[method_id(@__retain_semantics Other presentationLayer)]
//         pub unsafe fn presentationLayer(&self) -> Option<Id<Self>>;

//         #[method_id(@__retain_semantics Other modelLayer)]
//         pub unsafe fn modelLayer(&self) -> Id<Self>;
//     }
// );

// extern_methods!(
//     /// Methods declared on superclass `NSObject`
//     unsafe impl CAMetalLayer {
//         #[method_id(@__retain_semantics Init init)]
//         pub unsafe fn init(this: Allocated<Self>) -> Id<Self>;

//         #[method_id(@__retain_semantics New new)]
//         pub unsafe fn new() -> Id<Self>;
//     }
// );

// extern_methods!(
//     unsafe impl NSView {
//         #[method_id(@__retain_semantics Other layer)]
//         pub unsafe fn layer(&self) -> Option<Id<CALayer>>;

//         #[method(setLayer:)]
//         pub fn setLayer(&self, layer: Option<&CALayer>);
//     }
// );

