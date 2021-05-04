use std::convert::TryFrom;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use libc::{c_char, c_int, c_long, c_short, c_uint, c_ulong, c_ushort};
use num_enum::{IntoPrimitive, TryFromPrimitive};

type XID = usize;
type Mask = usize;
type Atom = usize;
type VisualID = usize;
type Time = usize;
type Bool = c_int;
type Status = c_int;

type Colormap = XID;
type Cursor = XID;
type Drawable = XID;
type Font = XID;
type Pixmap = XID;
pub type Window = XID;
type XserverRegion = XID;

pub struct Display(pub *mut ffi::Display);
impl std::ops::Deref for Display {
    type Target = *mut ffi::Display;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Display {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        // TODO
        //
        // The problem here is basically that Xlib is not Sync (unless XInitThreads was called),
        // so we must avoid calling XCloseDisplay if another thread might be calling Xlib
        // functions.
        //
        // The obvious easy workaroud is to call XInitThreads. (though when do we do that?)
        //
        // But it would be nice to e.g. support using Xlib through a singleton handle (which
        // would be Send but not Sync), in which case we could be sure
    }
}
// Safety: TODO
unsafe impl Send for Display {}

#[repr(C)]
pub struct XSetWindowAttributes {
    pub background_pixmap: Pixmap,
    pub background_pixel: c_ulong,
    pub border_pixmap: Pixmap,
    pub border_pixel: c_ulong,
    pub bit_gravity: c_int,
    pub win_gravity: c_int,
    pub backing_store: c_int,
    pub backing_places: c_ulong,
    pub backing_pixel: c_ulong,
    pub save_under: Bool,
    pub event_mask: c_long,
    pub do_not_propagate_mask: c_long,
    pub override_redirect: Bool,
    pub colormap: Colormap,
    pub cursor: Cursor,
}

#[repr(C)]
#[derive(Debug)]
pub struct XColor {
    pub pixel: c_ulong,
    red: c_ushort,
    green: c_ushort,
    blue: c_ushort,
    flags: c_char,
    pad: c_char,
}

impl XColor {
    #[allow(non_upper_case_globals)]
    const DoRed: c_char = 1 << 0;
    #[allow(non_upper_case_globals)]
    const DoGreen: c_char = 1 << 1;
    #[allow(non_upper_case_globals)]
    const DoBlue: c_char = 1 << 2;
    pub unsafe fn from_rgba(
        display: *mut ffi::Display,
        screen_number: c_int,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) -> XColor {
        let mut color = XColor {
            red: red as c_ushort * (0xffff / 0xff),
            green: green as c_ushort * (0xffff / 0xff),
            blue: blue as c_ushort * (0xffff / 0xff),
            flags: XColor::DoRed | XColor::DoGreen | XColor::DoBlue,
            pixel: 0,
            pad: 0,
        };
        if ffi::XAllocColor(
            display,
            ffi::XDefaultColormap(display, screen_number),
            &mut color,
        ) == 0
        {
            panic!("Cannot create colour");
        }
        // TODO: why do this? why here and not earlier?
        color.pixel = (color.pixel & 0x00_ff_ff_ff) | ((alpha as c_ulong) << 24);
        color
    }
}

#[repr(C)]
pub struct Visual {
    _private: [u8; 0],
}

#[repr(C)]
pub struct XVisualInfo {
    pub visual: *mut Visual,
    visualid: VisualID,
    screen: c_int,
    pub depth: c_int,
    class: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    colormap_size: c_int,
    bits_per_rgb: c_int,
}

#[repr(C)]
pub struct XRectangle {
    x: c_short,
    y: c_short,
    width: c_ushort,
    height: c_ushort,
}

#[repr(C)]
pub struct _XGC {
    _private: [u8; 0],
}
type GC = *mut _XGC; // not always strictly true, but close enough

#[repr(C)]
pub struct XGCValues {
    function: c_int,
    plane_mask: c_ulong,
    foreground: c_ulong,
    background: c_ulong,
    line_width: c_int,
    line_style: c_int,
    cap_style: c_int,
    join_style: c_int,
    fill_style: c_int,
    fill_rule: c_int,
    arc_mode: c_int,
    tile: Pixmap,
    stipple: Pixmap,
    ts_x_origin: c_int,
    ts_y_origin: c_int,
    font: Font,
    subwindow_mode: c_int,
    graphics_exposures: Bool,
    clip_x_origin: c_int,
    clip_y_origin: c_int,
    clip_mask: Pixmap,
    dash_offset: c_int,
    dashes: c_char,
}

#[repr(C)]
pub struct XFontStruct {
    ext_data: *mut XExtData,
    pub fid: Font,
    direction: c_uint,
    min_char_or_byte2: c_uint,
    max_char_or_byte2: c_uint,
    min_byte1: c_uint,
    max_byte1: c_uint,
    all_chars_exist: Bool,
    default_char: c_uint,
    n_properties: c_int,
    properties: *mut XFontProp,
    min_bounds: XCharStruct,
    max_bounds: XCharStruct,
    per_char: *mut XCharStruct,
    ascent: c_int,
    descent: c_int,
}

#[repr(C)]
pub struct XFontProp {
    name: Atom,
    card32: c_ulong,
}

#[repr(C)]
pub struct XCharStruct {
    pub lbearing: c_short,
    pub rbearing: c_short,
    pub width: c_short,
    pub ascent: c_short,
    pub descent: c_short,
    attributes: c_ushort,
}

#[repr(C)]
pub struct XExtData {
    _private: [u8; 0],
}

#[repr(C)]
pub struct XPoint {
    pub x: c_short,
    pub y: c_short,
}

#[allow(non_upper_case_globals)]
pub mod coord_mode {
    use libc::c_int;
    pub const CoordModeOrigin: c_int = 0;
    pub const CoordModePrevious: c_int = 1;
}

#[allow(non_upper_case_globals)]
pub mod shape_notify {
    use libc::c_ulong;
    pub const ShapeNotifyMask: c_ulong = 1;
    pub const ShapeNotify: c_ulong = 0;
}

#[allow(non_upper_case_globals)]
pub mod shape_op {
    use libc::c_int;
    pub const ShapeSet: c_int = 0;
    pub const ShapeUnion: c_int = 1;
    pub const ShapeIntersect: c_int = 2;
    pub const ShapeSubtract: c_int = 3;
    pub const ShapeInvert: c_int = 4;
}

#[allow(non_upper_case_globals)]
pub mod shape_dest_kind {
    use libc::c_int;
    pub const ShapeBounding: c_int = 0;
    pub const ShapeClip: c_int = 1;
    pub const ShapeInput: c_int = 2;
}

#[allow(non_upper_case_globals)]
pub mod window_attributes {
    use libc::c_ulong;
    pub const CWBackPixmap: c_ulong = 1 << 0;
    pub const CWBackPixel: c_ulong = 1 << 1;
    pub const CWBorderPixmap: c_ulong = 1 << 2;
    pub const CWBorderPixel: c_ulong = 1 << 3;
    pub const CWBitGravity: c_ulong = 1 << 4;
    pub const CWWinGravity: c_ulong = 1 << 5;
    pub const CWBackingStore: c_ulong = 1 << 6;
    pub const CWBackingPlanes: c_ulong = 1 << 7;
    pub const CWBackingPixel: c_ulong = 1 << 8;
    pub const CWOverrideRedirect: c_ulong = 1 << 9;
    pub const CWSaveUnder: c_ulong = 1 << 10;
    pub const CWEventMask: c_ulong = 1 << 11;
    pub const CWDontPropagate: c_ulong = 1 << 12;
    pub const CWColormap: c_ulong = 1 << 13;
    pub const CWCursor: c_ulong = 1 << 14;
}

#[allow(non_upper_case_globals)]
pub mod create_colormap_alloc {
    use libc::c_int;
    pub const AllocNone: c_int = 0;
    pub const AllocAll: c_int = 1;
}

#[allow(non_upper_case_globals)]
pub mod create_window_class {
    use libc::c_uint;
    pub const InputOutput: c_uint = 1;
    pub const InputOnly: c_uint = 2;
}

#[allow(non_upper_case_globals)]
pub mod display_class {
    use libc::c_int;
    pub const StaticGray: c_int = 0;
    pub const GrayScale: c_int = 1;
    pub const StaticColor: c_int = 2;
    pub const PseudoColor: c_int = 3;
    pub const TrueColor: c_int = 4;
    pub const DirectColor: c_int = 5;
}

#[allow(non_upper_case_globals)]
mod visual_info_mask {
    use libc::c_ulong;
    pub const VisualNoMask: c_ulong = 0x0;
    pub const VisualIDMask: c_ulong = 0x1;
    pub const VisualScreenMask: c_ulong = 0x2;
    pub const VisualDepthMask: c_ulong = 0x4;
    pub const VisualClassMask: c_ulong = 0x8;
    pub const VisualRedMaskMask: c_ulong = 0x10;
    pub const VisualGreenMaskMask: c_ulong = 0x20;
    pub const VisualBlueMaskMask: c_ulong = 0x40;
    pub const VisualColormapSizeMask: c_ulong = 0x80;
    pub const VisualBitsPerRGBMask: c_ulong = 0x100;
    pub const VisualAllMask: c_ulong = 0x1ff;
}

#[allow(non_upper_case_globals)]
pub mod event_masks {
    use libc::c_long;
    pub const NoEventMask: c_long = 0;
    pub const KeyPressMask: c_long = 1 << 0;
    pub const KeyReleaseMask: c_long = 1 << 1;
    pub const ButtonPressMask: c_long = 1 << 2;
    pub const ButtonReleaseMask: c_long = 1 << 3;
    pub const EnterWindowMask: c_long = 1 << 4;
    pub const LeaveWindowMask: c_long = 1 << 5;
    pub const PointerMotionMask: c_long = 1 << 6;
    pub const PointerMotionHintMask: c_long = 1 << 7;
    pub const Button1MotionMask: c_long = 1 << 8;
    pub const Button2MotionMask: c_long = 1 << 9;
    pub const Button3MotionMask: c_long = 1 << 10;
    pub const Button4MotionMask: c_long = 1 << 11;
    pub const Button5MotionMask: c_long = 1 << 12;
    pub const ButtonMotionMask: c_long = 1 << 13;
    pub const KeymapStateMask: c_long = 1 << 14;
    pub const ExposureMask: c_long = 1 << 15;
    pub const VisibilityChangeMask: c_long = 1 << 16;
    pub const StructureNotifyMask: c_long = 1 << 17;
    pub const ResizeRedirectMask: c_long = 1 << 18;
    pub const SubstructureNotifyMask: c_long = 1 << 19;
    pub const SubstructureRedirectMask: c_long = 1 << 20;
    pub const FocusChangeMask: c_long = 1 << 21;
    pub const PropertyChangeMask: c_long = 1 << 22;
    pub const ColormapChangeMask: c_long = 1 << 23;
    pub const OwnerGrabButtonMask: c_long = 1 << 24;
}

#[allow(non_upper_case_globals)]
pub mod gravity {
    use libc::c_int;
    pub const ForgetGravity: c_int = 0;
    pub const NorthWestGravity: c_int = 1;
    pub const NorthGravity: c_int = 2;
    pub const NorthEastGravity: c_int = 3;
    pub const WestGravity: c_int = 4;
    pub const CenterGravity: c_int = 5;
    pub const EastGravity: c_int = 6;
    pub const SouthWestGravity: c_int = 7;
    pub const SouthGravity: c_int = 8;
    pub const SouthEastGravity: c_int = 9;
    pub const StaticGravity: c_int = 10;
}

// TODO: proper error types
#[allow(non_snake_case)]
pub unsafe fn XOpenDisplay(display_name: Option<&str>) -> eyre::Result<Display> {
    let display_name = display_name.map_or_else(
        || Ok(std::ptr::null()),
        |s| CString::new(s).map(|c| c.as_ptr()),
    )?;
    let display = ffi::XOpenDisplay(display_name);
    if display.is_null() {
        Err(eyre::eyre!("XOpenDisplay failed to open display"))
    } else {
        Ok(Display(display))
    }
}

/// Whether any Xlib handle has ever existed.
static XLIB_USED: AtomicBool = AtomicBool::new(false);
/// Whether Xlib's threading support has been enabled.
///
/// While this is false, only a single Xlib handle may exist at a time. Once it has been set to
/// true, it will never be reset.
static XLIB_THREADED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum XlibHandleState {
    Unchosen,
    SingleThreaded,
    ExSingleThreaded,
    MultiThreadedPending,
    MultiThreaded,
}

static XLIB_THREAD_STATE: AtomicU8 = AtomicU8::new(XlibHandleState::Unchosen as _);

// send, but not sync
#[derive(Debug)]
pub struct XlibHandle(std::marker::PhantomData<Display>);
impl XlibHandle {
    pub fn new() -> Option<Self> {
        loop {
            return match XLIB_THREAD_STATE
                .compare_exchange(
                    XlibHandleState::Unchosen.into(),
                    XlibHandleState::SingleThreaded.into(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .map_or_else(
                    |e| Err(XlibHandleState::try_from(e).unwrap()),
                    |t| Ok(XlibHandleState::try_from(t).unwrap()),
                ) {
                Ok(XlibHandleState::Unchosen) => {
                    // We are entering single-threaded mode.
                    Some(XlibHandle(std::marker::PhantomData))
                }
                Ok(_) => unreachable!(),
                Err(XlibHandleState::Unchosen) => unreachable!(),
                Err(XlibHandleState::SingleThreaded) => {
                    // We are already in single-threaded mode, we cannot hand out another handle.
                    None
                }
                Err(XlibHandleState::ExSingleThreaded) => {
                    // We were previously in single-threaded mode but the handle was dropped, so
                    // it may be safe to hand out a fresh handle.
                    match XLIB_THREAD_STATE
                        .compare_exchange(
                            XlibHandleState::ExSingleThreaded.into(),
                            XlibHandleState::SingleThreaded.into(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        )
                        .map_or_else(
                            |e| Err(XlibHandleState::try_from(e).unwrap()),
                            |t| Ok(XlibHandleState::try_from(t).unwrap()),
                        ) {
                        Ok(XlibHandleState::ExSingleThreaded) => {
                            // We are re-entering single-threaded mode.
                            Some(XlibHandle(std::marker::PhantomData))
                        }
                        Ok(_) => unreachable!(),
                        Err(XlibHandleState::ExSingleThreaded) => unreachable!(),
                        Err(XlibHandleState::MultiThreadedPending)
                        | Err(XlibHandleState::MultiThreaded) => panic!(
                            "illegal transition from single-threaded mode to multi-threaded mode"
                        ),
                        Err(XlibHandleState::Unchosen) => {
                            panic!("illegal transition from single-threaded mode to uninitialised")
                        }
                        Err(XlibHandleState::SingleThreaded) => {
                            // We were preempted.
                            None
                        }
                    }
                }
                Err(XlibHandleState::MultiThreadedPending) => {
                    // We are entering multi-threaded mode, it will be safe to hand out a handle
                    // once we finish calling XInitThreads.
                    continue;
                }
                Err(XlibHandleState::MultiThreaded) => {
                    // We are in multi-threaded mode, but it's safe to continue to hand out
                    // unsync handles.
                    Some(XlibHandle(std::marker::PhantomData))
                }
            };
        }
    }
}

// both send and sync
#[derive(Debug)]
pub struct XlibThreadedHandle(());
// no Drop impl required
impl XlibThreadedHandle {
    pub fn new() -> Option<Self> {
        loop {
            return match XLIB_THREAD_STATE
                .compare_exchange(
                    XlibHandleState::Unchosen.into(),
                    XlibHandleState::MultiThreadedPending.into(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .map_or_else(
                    |e| Err(XlibHandleState::try_from(e).unwrap()),
                    |t| Ok(XlibHandleState::try_from(t).unwrap()),
                ) {
                Ok(XlibHandleState::Unchosen) => {
                    // We are entering multi-threaded mode.
                    unsafe {
                        ffi::XInitThreads();
                    }
                    XLIB_THREAD_STATE
                        .store(XlibHandleState::MultiThreaded.into(), Ordering::SeqCst);
                    Some(XlibThreadedHandle(()))
                }
                Ok(_) => unreachable!(),
                Err(XlibHandleState::Unchosen) => unreachable!(),
                Err(XlibHandleState::SingleThreaded) | Err(XlibHandleState::ExSingleThreaded) => {
                    // It's not possible to initialise multi-threaded mode once Xlib has been
                    // used.
                    None
                }
                Err(XlibHandleState::MultiThreadedPending) => {
                    // Another thread is initialising multi-threaded mode, it will be safe to
                    // hand out a handle soon.
                    continue;
                }
                Err(XlibHandleState::MultiThreaded) => {
                    // We are already in multi-threaded mode.
                    Some(XlibThreadedHandle(()))
                }
            };
        }
    }
}

pub mod ffi {
    use super::*;
    use libc::{c_char, c_int, c_uint, c_ulong};

    // TODO: make private once wrapper interface is done
    #[repr(C)]
    pub struct Display {
        _private: [u8; 0],
    }

    #[link(name = "X11")]
    extern "C" {
        pub(super) fn XInitThreads() -> c_int;
        pub(super) fn XOpenDisplay(display_name: *const c_char) -> *mut ffi::Display;
        pub(super) fn XCloseDisplay(display: *mut ffi::Display) -> c_int;
        pub fn XDefaultScreen(display: *mut ffi::Display) -> c_int;
        pub fn XDefaultColormap(display: *mut ffi::Display, screen_number: c_int) -> Colormap;
        pub fn XDefaultRootWindow(display: *mut ffi::Display) -> Window;
        pub fn XDefaultVisual(display: *mut ffi::Display, screen_number: c_int) -> Visual;
        pub fn XShapeQueryExtension(
            dpy: *mut ffi::Display,
            event_basep: *mut c_int,
            error_basep: *mut c_int,
        ) -> Bool;
        pub fn XAllocColor(
            display: *mut ffi::Display,
            colormap: Colormap,
            screen_in_out: *mut XColor,
        ) -> Status;
        pub fn XMatchVisualInfo(
            display: *mut ffi::Display,
            screen: c_int,
            depth: c_int,
            class: c_int,
            vinfo_return: *mut XVisualInfo,
        ) -> Status;
        pub fn XCreateColormap(
            display: *mut ffi::Display,
            w: Window,
            visual: *mut Visual,
            alloc: c_int,
        ) -> Colormap;
        pub fn XCreateWindow(
            display: *mut ffi::Display,
            parent: Window,
            x: c_int,
            y: c_int,
            width: c_uint,
            height: c_uint,
            border_width: c_uint,
            depth: c_int,
            class: c_uint,
            visual: *mut Visual,
            valuemask: c_ulong,
            attributes: *mut XSetWindowAttributes,
        ) -> Window;
        pub fn XShapeCombineMask(
            dpy: *mut ffi::Display,
            dest: XID,
            destKind: c_int,
            xOff: c_int,
            yOff: c_int,
            src: Pixmap,
            op: c_int,
        );
        pub fn XShapeSelectInput(dpy: *mut ffi::Display, window: Window, mask: c_ulong);
        pub fn XMapWindow(display: *mut ffi::Display, window: Window) -> c_int;
        pub fn XCreateGC(
            display: *mut ffi::Display,
            d: Drawable,
            valuemask: c_ulong,
            values: *mut XGCValues,
        ) -> GC;
        pub fn XFreeGC(display: *mut ffi::Display, gc: GC) -> c_int;
        pub fn XSetBackground(display: *mut ffi::Display, gc: GC, background: c_ulong) -> c_int;
        pub fn XSetForeground(display: *mut ffi::Display, gc: GC, foreground: c_ulong) -> c_int;
        pub fn XFillRectangle(
            display: *mut ffi::Display,
            d: Drawable,
            gc: GC,
            x: c_int,
            y: c_int,
            width: c_uint,
            height: c_uint,
        ) -> c_int;
        pub fn XFlush(display: *mut ffi::Display) -> c_int;
        pub fn XLoadQueryFont(display: *mut ffi::Display, name: *const c_char) -> *mut XFontStruct;
        pub fn XFreeFont(display: *mut ffi::Display, font_struct: *mut XFontStruct) -> c_int;
        pub fn XSetFont(display: *mut ffi::Display, gc: GC, font: Font) -> c_int;
        pub fn XDrawString(
            display: *mut ffi::Display,
            drawable: Drawable,
            gc: GC,
            x: c_int,
            y: c_int,
            string: *const c_char,
            length: c_int,
        ) -> c_int;
        pub fn XDrawRectangle(
            display: *mut ffi::Display,
            drawable: Drawable,
            gc: GC,
            x: c_int,
            y: c_int,
            width: c_uint,
            height: c_uint,
        ) -> c_int;
        pub fn XDrawLines(
            display: *mut ffi::Display,
            drawable: Drawable,
            gc: GC,
            points: *const XPoint,
            npoints: c_int,
            mode: c_int,
        ) -> c_int;
        pub fn XTextExtents(
            font_struct: *const XFontStruct,
            string: *const c_char,
            nchars: c_int,
            direction_return: *mut c_int,
            font_ascent_return: *mut c_int,
            font_descent_return: *mut c_int,
            overall_return: *mut XCharStruct,
        ) -> c_int;
    }
}

#[link(name = "Xfixes")]
extern "C" {
    pub fn XFixesCreateRegion(
        dpy: *mut ffi::Display,
        rectangles: *mut XRectangle,
        nrectangle: c_int,
    ) -> XserverRegion;
    pub fn XFixesSetWindowShapeRegion(
        dpy: *mut ffi::Display,
        window: Window,
        shape_kind: c_int,
        x_off: c_int,
        y_off: c_int,
        region: XserverRegion,
    );
    pub fn XFixesDestroyRegion(dpy: *mut ffi::Display, region: XserverRegion);
}

#[link(name = "Xext")]
extern "C" {}
