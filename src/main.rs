#![allow(dead_code)]

mod graphics_data;
mod x11;

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ffi::CString;
use std::mem::MaybeUninit;

use eyre::{bail, eyre, WrapErr};
use lazy_static::lazy_static;
use regex::Regex;
use structopt::StructOpt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, event, info, info_span, instrument, warn, Level};
use tracing_error::ErrorLayer;
use tracing_futures::Instrument;
use tracing_subscriber::prelude::*;

use graphics_data::{Drawable, Graphic, Size};

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "edmcoverlay")]
struct Opt {
    /// X position of overlay
    #[structopt(name = "X")]
    x_position: i32,
    /// Y position of overlay
    #[structopt(name = "Y")]
    y_position: i32,
    /// Width of overlay
    #[structopt(name = "WIDTH")]
    width: u32,
    /// Height of overlay
    #[structopt(name = "HEIGHT")]
    height: u32,
}

#[derive(Debug)]
struct Command {
    client_id: usize,
    graphic: Graphic,
}

#[derive(Debug)]
struct Config {
    x_position: i32,
    y_position: i32,
    width: u32,
    height: u32,
    title_font: Option<*mut x11::XFontStruct>,
    body_font: Option<*mut x11::XFontStruct>,
}
// TODO: safety
unsafe impl Send for Config {}

const FPS: u32 = 1;

fn scale_w(x: usize, width: u32) -> usize {
    x * width as usize / 1280
}
fn scale_h(y: usize, height: u32) -> usize {
    y * height as usize / 1024
}
fn scale_x(x: usize, width: u32) -> usize {
    scale_w(x, width) + 20
}
fn scale_y(y: usize, height: u32) -> usize {
    scale_h(y, height) + 40
}

// TODO: enable once https://github.com/tokio-rs/tracing/issues/1318 is fixed
#[instrument(skip(display, window))]
fn do_redraw(
    config: &Config,
    graphics: &HashMap<(usize, String), Option<Graphic>>,
    expired: &[Graphic],
    display: &x11::Display,
    window: x11::Window,
) -> eyre::Result<()> {
    event!(
        Level::TRACE,
        ?graphics,
        "redrawing {} graphics",
        graphics.len()
    );
    let gc = unsafe { x11::ffi::XCreateGC(**display, window, 0, std::ptr::null_mut()) };
    lazy_static! {
        static ref HEX_REGEX: Regex =
            Regex::new(r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})$").unwrap();
    }
    unsafe {
        x11::ffi::XSetForeground(
            **display,
            gc,
            x11::XColor::from_rgba(**display, x11::ffi::XDefaultScreen(**display), 0, 0, 0, 0)
                .pixel,
        );
    }
    for graphic in expired.iter() {
        unsafe {
            match &graphic.drawable.as_ref().unwrap() {
                Drawable::Rectangle { x, y, w, h, .. } => {
                    x11::ffi::XFillRectangle(
                        **display,
                        window,
                        gc,
                        scale_x(*x, config.width) as i32,
                        scale_y(*y, config.height) as i32,
                        scale_w(*w, config.width) as u32,
                        scale_h(*h, config.height) as u32,
                    );
                }
                Drawable::Vector { vector, .. } => {
                    let (xmin, xmax, ymin, ymax) = vector.iter().fold((0, 0, 0, 0), |acc, val| {
                        (
                            val.x.min(acc.0),
                            val.x.max(acc.1),
                            val.y.min(acc.2),
                            val.y.max(acc.3),
                        )
                    });
                    x11::ffi::XFillRectangle(
                        **display,
                        window,
                        gc,
                        scale_x(xmin, config.width) as i32,
                        scale_y(ymin, config.height) as i32,
                        scale_w(xmax - xmin, config.width) as u32,
                        scale_h(ymax - ymin, config.height) as u32,
                    );
                }
                Drawable::Text {
                    text, size, x, y, ..
                } => {
                    let font = match size {
                        Size::Normal => config.body_font.unwrap(),
                        Size::Large => config.title_font.unwrap(),
                    };
                    let mut direction_return = 0;
                    let mut font_ascent_return = 0;
                    let mut font_descent_return = 0;
                    let mut overall_return = std::mem::MaybeUninit::<x11::XCharStruct>::uninit();
                    let s = CString::new(AsRef::<str>::as_ref(text))?;
                    let b = s.as_bytes();
                    if x11::ffi::XTextExtents(
                        font,
                        b.as_ptr() as *const i8,
                        b.len() as i32,
                        &mut direction_return,
                        &mut font_ascent_return,
                        &mut font_descent_return,
                        overall_return.as_mut_ptr(),
                    ) == 0
                    {
                        let overall_return = overall_return.assume_init();
                        x11::ffi::XFillRectangle(
                            **display,
                            window,
                            gc,
                            scale_x(*x, config.width) as i32 + overall_return.lbearing as i32,
                            scale_y(*y, config.height) as i32 - overall_return.ascent as i32,
                            (overall_return.rbearing - overall_return.lbearing)
                                .try_into()
                                .unwrap(),
                            (overall_return.ascent + overall_return.descent)
                                .try_into()
                                .unwrap(),
                        );
                    }
                }
            }
        }
    }
    for (_, graphic) in graphics.iter() {
        unsafe {
            let set_color = |color: &graphics_data::Color| {
                let color = x11::XColor::from_rgba(
                    **display,
                    x11::ffi::XDefaultScreen(**display),
                    color.red,
                    color.green,
                    color.blue,
                    255,
                );
                x11::ffi::XSetForeground(**display, gc, color.pixel)
            };
            match &graphic.as_ref().unwrap().drawable.as_ref().unwrap() {
                Drawable::Rectangle {
                    shape: _,
                    x,
                    y,
                    w,
                    h,
                    fill,
                    color,
                } => {
                    set_color(fill);
                    x11::ffi::XFillRectangle(
                        **display,
                        window,
                        gc,
                        scale_x(*x, config.width) as i32,
                        scale_y(*y, config.height) as i32,
                        scale_w(*w, config.width) as u32,
                        scale_h(*h, config.height) as u32,
                    );
                    set_color(color);
                    x11::ffi::XDrawRectangle(
                        **display,
                        window,
                        gc,
                        scale_x(*x, config.width) as i32,
                        scale_y(*y, config.height) as i32,
                        scale_w(*w, config.width) as u32,
                        scale_h(*h, config.height) as u32,
                    );
                }
                Drawable::Vector {
                    shape: _,
                    color,
                    vector,
                } => {
                    set_color(color);
                    let points: Vec<_> = vector
                        .iter()
                        .map(|p| x11::XPoint {
                            x: p.x as i16,
                            y: p.y as i16,
                        })
                        .collect();
                    x11::ffi::XDrawLines(
                        **display,
                        window,
                        gc,
                        points.as_ptr(),
                        points.len() as i32,
                        x11::coord_mode::CoordModeOrigin,
                    );
                }
                Drawable::Text {
                    text,
                    size,
                    color,
                    x,
                    y,
                } => {
                    set_color(color);
                    match size {
                        Size::Normal => {
                            x11::ffi::XSetFont(**display, gc, (*config.body_font.unwrap()).fid)
                        }
                        Size::Large => {
                            x11::ffi::XSetFont(**display, gc, (*config.title_font.unwrap()).fid)
                        }
                    };
                    let s = CString::new(std::convert::AsRef::<str>::as_ref(text))?;
                    let b = s.as_bytes();
                    x11::ffi::XDrawString(
                        **display,
                        window,
                        gc,
                        scale_x(*x, config.width) as i32,
                        scale_y(*y, config.height) as i32,
                        b.as_ptr() as *const i8,
                        b.len() as i32,
                    );
                }
            }
        }
    }
    unsafe {
        x11::ffi::XFreeGC(**display, gc);
        x11::ffi::XFlush(**display);
    };
    Ok(())
}

#[instrument(skip(opt, rx))]
async fn renderer(opt: Opt, mut rx: mpsc::Receiver<Command>) -> eyre::Result<()> {
    info!("alive");
    // open the display
    let display;
    let screen_number;
    unsafe {
        display = x11::XOpenDisplay(None).wrap_err("Failed to open display")?;
        screen_number = x11::ffi::XDefaultScreen(*display);
        let mut shape_event_base = MaybeUninit::uninit();
        let mut shape_error_base = MaybeUninit::uninit();
        if x11::ffi::XShapeQueryExtension(
            *display,
            shape_event_base.as_mut_ptr(),
            shape_error_base.as_mut_ptr(),
        ) == 0
        {
            bail!("Shape extension unavailable")
        }
        shape_event_base.assume_init();
        shape_error_base.assume_init();
    }

    // create the window
    debug!("creating window");
    let window;
    unsafe {
        let background_color = x11::XColor::from_rgba(*display, screen_number, 0, 0, 0, 0);

        let root = x11::ffi::XDefaultRootWindow(*display);

        let mut visual_info = MaybeUninit::uninit();
        x11::ffi::XMatchVisualInfo(
            *display,
            x11::ffi::XDefaultScreen(*display),
            32,
            x11::display_class::TrueColor,
            visual_info.as_mut_ptr(),
        );
        let visual_info = visual_info.assume_init();
        let colormap = x11::ffi::XCreateColormap(
            *display,
            x11::ffi::XDefaultRootWindow(*display),
            visual_info.visual,
            x11::create_colormap_alloc::AllocNone,
        );

        let mut attr = x11::XSetWindowAttributes {
            background_pixmap: 0,
            background_pixel: background_color.pixel,
            border_pixel: 0,
            win_gravity: x11::gravity::NorthWestGravity,
            bit_gravity: x11::gravity::ForgetGravity,
            save_under: 1,
            event_mask: {
                use x11::event_masks::*;
                StructureNotifyMask
                    | ExposureMask
                    | PropertyChangeMask
                    | EnterWindowMask
                    | LeaveWindowMask
                    | KeyPressMask
                    | KeyReleaseMask
                    | KeymapStateMask
            },
            do_not_propagate_mask: {
                use x11::event_masks::*;
                KeyPressMask
                    | KeyReleaseMask
                    | ButtonPressMask
                    | ButtonReleaseMask
                    | PointerMotionMask
                    | ButtonMotionMask
            },
            override_redirect: 1,
            colormap,
            backing_pixel: 0,
            backing_places: 0,
            backing_store: 0,
            border_pixmap: 0,
            cursor: 0,
        };

        window = x11::ffi::XCreateWindow(
            *display,
            root,
            opt.x_position,
            opt.y_position,
            opt.width,
            opt.height,
            0,
            visual_info.depth,
            x11::create_window_class::InputOutput,
            visual_info.visual,
            {
                use x11::window_attributes::*;
                CWColormap
                    | CWBorderPixel
                    | CWBackPixel
                    | CWEventMask
                    | CWWinGravity
                    | CWBitGravity
                    | CWSaveUnder
                    | CWDontPropagate
                    | CWOverrideRedirect
            },
            &mut attr,
        );

        x11::ffi::XShapeCombineMask(
            *display,
            window,
            x11::shape_dest_kind::ShapeInput,
            0,
            0,
            0,
            x11::shape_op::ShapeSet,
        );
        x11::ffi::XShapeSelectInput(*display, window, x11::shape_notify::ShapeNotifyMask);

        let region = x11::XFixesCreateRegion(*display, std::ptr::null_mut(), 0);
        x11::XFixesSetWindowShapeRegion(
            *display,
            window,
            x11::shape_dest_kind::ShapeInput,
            0,
            0,
            region,
        );
        x11::XFixesDestroyRegion(*display, region);

        x11::ffi::XMapWindow(*display, window);
    }

    // allocate fonts
    // TODO: do these just get leaked right now? whoops
    let mut config = Config {
        x_position: opt.x_position,
        y_position: opt.y_position,
        width: opt.width,
        height: opt.height,
        title_font: None,
        body_font: None,
    };
    unsafe {
        let s = CString::new("9x15bold")?;
        let body_font = x11::ffi::XLoadQueryFont(*display, s.as_ptr());
        if body_font.is_null() {
            error!("fug");
            return Err(eyre!("Failed to load font: 9x15bold"));
        }
        config.body_font = Some(body_font);
        let s = CString::new("12x24")?;
        let title_font = x11::ffi::XLoadQueryFont(*display, s.as_ptr());
        if title_font.is_null() {
            return Err(eyre!("Failed to load font: 12x24"));
        }
        config.title_font = Some(title_font);
    }

    // draw something!
    // debug!("drawing a square");
    // unsafe {
    //     let gc = x11::ffi::XCreateGC(*display, window, 0, std::ptr::null_mut());
    //     x11::ffi::XSetForeground(*display, gc, red.pixel);
    //     x11::ffi::XFillRectangle(*display, window, gc, 0, 0, 40, 40);
    //     x11::ffi::XFreeGC(*display, gc);
    //     x11::ffi::XFlush(*display);
    // }

    let mut graphics = HashMap::<(usize, String), Option<Graphic>>::new();
    graphics.insert(
        (0, "version-number".to_owned()),
        Some(Graphic {
            id: "test-rect".to_owned(),
            ttl: -1,
            drawable: Some(Drawable::Text {
                x: 1175,
                y: 975,
                color: "#ffffff".try_into().unwrap(),
                size: Size::Normal,
                text: "edmcoverlay CE".to_owned(),
            }),
        }),
    );
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1) / FPS);
    debug!(sample_graphic = ?serde_json::to_string(&Graphic {
        id: "sample-graphic".to_owned(),
        ttl: 12345,
        drawable: Some(Drawable::Text {
            text: "".to_owned(),
            size: Size::Normal,
            color: "#123456".try_into().unwrap(),
            x: 3,
            y: 14,
        }),
    }).unwrap());
    debug!("entering loop");
    do_redraw(&config, &graphics, &[], &display, window)?;
    loop {
        tokio::select! {
            Some(command) = rx.recv() => {
                let mut command = command;
                command.graphic.ttl *= isize::try_from(FPS)?;
                let mut expired = Vec::new();
                if let Some(Some(graphic)) = graphics.insert((command.client_id, command.graphic.id.to_owned()), Some(command.graphic)) {
                    expired.push(graphic);
                }
                do_redraw(&config, &graphics, &expired, &display, window)?;
            },
            _ = interval.tick() => {
                let mut expired = Vec::new();
                for (_, graphic) in graphics.iter_mut() {
                    if let Some(Graphic { ref mut ttl, ref id, .. }) = graphic {
                        if *ttl == 0 {
                            debug!(graphic_id = ?id, "ttl expired");
                            expired.push(graphic.take().unwrap());
                            continue;
                        }
                        if *ttl > 0 {
                            *ttl -= 1;
                        }
                    }
                }
                graphics.retain(|_, v| v.is_some());
                do_redraw(&config, &graphics, &expired, &display, window)?;
            },
        }
    }
}

#[instrument(skip(tx))]
async fn listener(tx: mpsc::Sender<Command>) -> eyre::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:5010").await?;
    let mut client_id: usize = 0;
    println!("server: ready to accept connections"); // load-bearing println, do not remove!
    loop {
        debug!("waiting for connection");
        let (socket, _) = listener.accept().await?;
        client_id = client_id.wrapping_add(1);
        let tx = tx.clone();

        debug!(client_id, "new client");
        tokio::spawn(
            async move {
                debug!("started");
                let stream = BufReader::new(socket);
                let mut lines = stream.lines();
                debug!("waiting for line");
                while let Some(line) = lines.next_line().await? {
                    debug!(
                        client_id,
                        line = std::convert::AsRef::<str>::as_ref(&line),
                        "line received"
                    );
                    match serde_json::from_str::<Graphic>(&line)
                        .wrap_err_with(|| eyre!("could not parse line {:?}", line))
                    {
                        Ok(graphic) => {
                            if graphic.drawable.is_none() {
                                warn!(?line, ?graphic, "invalid drawable");
                            }
                            if graphic.drawable.is_some() || graphic.ttl == 0 {
                                tx.send(Command { client_id, graphic }).await?;
                            }
                        }
                        Err(e) => eprintln!("{:#}", eyre!(e)),
                    };
                }
                debug!("client disconnected");
                Ok::<(), eyre::Report>(())
            }
            .instrument(info_span!("handler", client_id)),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), eyre::Report> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr);
    let filter_layer = tracing_subscriber::EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    color_eyre::install()?;

    let opt = Opt::from_args();
    // TODO: handle SIGINT and SIGTERM

    let (tx, rx): (mpsc::Sender<Command>, _) = mpsc::channel(100);

    let renderer = tokio::spawn(renderer(opt.clone(), rx));
    let listener = tokio::spawn(listener(tx));

    tokio::select! {
        result = renderer => result??,
        result = listener => result??,
    }
    Ok(())
}
