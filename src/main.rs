//! waybar-crypto-ticker - A scrolling cryptocurrency ticker overlay for Waybar.
//!
//! Displays real-time cryptocurrency prices from Kraken's WebSocket API as a
//! smooth scrolling overlay that integrates with Waybar on Hyprland/Wayland.

use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod config;
mod hyprland;
mod ticker;
mod websocket;

use config::{Anchor, Config};
use ticker::TickerState;

const APP_ID: &str = "io.github.waybar-crypto-ticker";
const PID_FILE: &str = "/tmp/waybar-crypto-ticker.pid";

fn main() -> glib::ExitCode {
    // Ignore real-time signals that Waybar sends to refresh modules
    ignore_realtime_signals();

    // Ensure single instance
    if !acquire_instance_lock() {
        return glib::ExitCode::SUCCESS;
    }

    // Clean up PID file on exit
    let _cleanup = PidFileGuard;

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run()
}

/// Guard that removes PID file when dropped.
struct PidFileGuard;

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(PID_FILE);
    }
}

/// Ignore real-time signals (SIGRTMIN+1 through SIGRTMIN+10).
/// Waybar sends these to refresh custom modules.
fn ignore_realtime_signals() {
    unsafe {
        let rtmin = libc::SIGRTMIN();
        for i in 1..=10 {
            libc::signal(rtmin + i, libc::SIG_IGN);
        }
    }
}

/// Acquire single-instance lock via PID file.
fn acquire_instance_lock() -> bool {
    if std::path::Path::new(PID_FILE).exists() {
        if let Ok(old_pid) = std::fs::read_to_string(PID_FILE) {
            let proc_path = format!("/proc/{}", old_pid.trim());
            if std::path::Path::new(&proc_path).exists() {
                eprintln!("Already running (PID {})", old_pid.trim());
                return false;
            }
        }
    }
    let _ = std::fs::write(PID_FILE, format!("{}", std::process::id()));
    true
}

/// Load cryptocurrency icons from the data directory.
fn load_icons(config: &Config) -> HashMap<String, gtk4::cairo::ImageSurface> {
    let mut icons = HashMap::new();
    let icons_dir = Config::icons_dir();
    let icon_size = config.appearance.icon_size;

    for coin in &config.coins {
        let path = icons_dir.join(&coin.icon);
        if !path.exists() {
            continue;
        }

        if let Some(surface) = render_icon_to_surface(&path, icon_size) {
            icons.insert(coin.icon.clone(), surface);
        }
    }

    icons
}

/// Render an image file to a Cairo surface with circular clipping.
fn render_icon_to_surface(path: &std::path::Path, size: u32) -> Option<gtk4::cairo::ImageSurface> {
    let extension = path.extension()?.to_str()?;

    let pixmap = if extension == "svg" {
        render_svg(path, size)?
    } else {
        render_png(path, size)?
    };

    // Convert tiny_skia RGBA to Cairo ARGB32 (premultiplied BGRA)
    let width = pixmap.width();
    let height = pixmap.height();
    let data = pixmap.data();

    let mut surface = gtk4::cairo::ImageSurface::create(
        gtk4::cairo::Format::ARgb32,
        width as i32,
        height as i32,
    ).ok()?;

    {
        let mut surface_data = surface.data().ok()?;
        for i in 0..(width * height) as usize {
            let r = data[i * 4] as u32;
            let g = data[i * 4 + 1] as u32;
            let b = data[i * 4 + 2] as u32;
            let a = data[i * 4 + 3] as u32;

            // Premultiply and convert to BGRA
            surface_data[i * 4] = ((b * a) / 255) as u8;
            surface_data[i * 4 + 1] = ((g * a) / 255) as u8;
            surface_data[i * 4 + 2] = ((r * a) / 255) as u8;
            surface_data[i * 4 + 3] = a as u8;
        }
    }
    surface.mark_dirty();

    // Apply circular clip
    let clipped = gtk4::cairo::ImageSurface::create(
        gtk4::cairo::Format::ARgb32,
        width as i32,
        height as i32,
    ).ok()?;

    let cr = gtk4::cairo::Context::new(&clipped).ok()?;
    let center = width as f64 / 2.0;
    let radius = center - 1.0;
    cr.arc(center, center, radius, 0.0, 2.0 * std::f64::consts::PI);
    cr.clip();
    cr.set_source_surface(&surface, 0.0, 0.0).ok()?;
    cr.paint().ok()?;

    Some(clipped)
}

fn render_svg(path: &std::path::Path, size: u32) -> Option<resvg::tiny_skia::Pixmap> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&svg_data, &opt).ok()?;

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;

    let x_offset = (size as f32 - svg_size.width() * scale) / 2.0;
    let y_offset = (size as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(x_offset, y_offset);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn render_png(path: &std::path::Path, size: u32) -> Option<resvg::tiny_skia::Pixmap> {
    let png_data = std::fs::read(path).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::decode_png(&png_data).ok()?;

    if pixmap.width() != size || pixmap.height() != size {
        let mut scaled = resvg::tiny_skia::Pixmap::new(size, size)?;

        let scale = (size as f32 / pixmap.width() as f32)
            .min(size as f32 / pixmap.height() as f32);

        let x_offset = (size as f32 - pixmap.width() as f32 * scale) / 2.0;
        let y_offset = (size as f32 - pixmap.height() as f32 * scale) / 2.0;

        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(x_offset, y_offset);

        scaled.draw_pixmap(
            0, 0,
            pixmap.as_ref(),
            &resvg::tiny_skia::PixmapPaint::default(),
            transform,
            None,
        );
        pixmap = scaled;
    }

    Some(pixmap)
}

fn build_ui(app: &Application) {
    let config = Config::load();

    let state = Arc::new(Mutex::new(TickerState::new(&config)));
    let scroll_offset = Rc::new(RefCell::new(0.0f64));
    let cached_width = Rc::new(RefCell::new(0.0f64));
    let icons = Rc::new(load_icons(&config));
    let config = Rc::new(config);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(config.position.width)
        .default_height(config.position.height)
        .decorated(false)
        .build();

    // Transparent background
    if let Some(display) = gtk4::gdk::Display::default() {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data("window { background-color: transparent; }");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Layer shell setup
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);

    // Monitor targeting
    if let Some(ref monitor_name) = config.monitor {
        if let Some(display) = gtk4::gdk::Display::default() {
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(obj) = monitors.item(i) {
                    if let Ok(monitor) = obj.downcast::<gtk4::gdk::Monitor>() {
                        if monitor.connector().as_deref() == Some(monitor_name.as_str()) {
                            window.set_monitor(&monitor);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Position anchoring
    match config.position.anchor {
        Anchor::TopLeft => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Left, true);
        }
        Anchor::TopRight => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, true);
        }
        Anchor::BottomLeft => {
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
        }
        Anchor::BottomRight => {
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Right, true);
        }
    }

    window.set_margin(Edge::Top, config.position.margin_top);
    window.set_margin(Edge::Right, config.position.margin_right);
    window.set_margin(Edge::Bottom, config.position.margin_bottom);
    window.set_margin(Edge::Left, config.position.margin_left);
    window.set_namespace("waybar-crypto-ticker");
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

    // Drawing area
    let drawing_area = DrawingArea::new();
    drawing_area.set_content_width(config.position.width);
    drawing_area.set_content_height(config.position.height);

    let state_draw = Arc::clone(&state);
    let scroll_draw = Rc::clone(&scroll_offset);
    let icons_draw = Rc::clone(&icons);
    let cached_draw = Rc::clone(&cached_width);
    let config_draw = Rc::clone(&config);

    let last_segments: Rc<RefCell<Vec<ticker::Segment>>> = Rc::new(RefCell::new(Vec::new()));
    let segments_draw = Rc::clone(&last_segments);

    drawing_area.set_draw_func(move |_, cr, width, height| {
        let segments = match state_draw.try_lock() {
            Ok(state) => {
                let segs = state.segments.clone();
                *segments_draw.borrow_mut() = segs.clone();
                segs
            }
            Err(_) => segments_draw.borrow().clone(),
        };

        let offset = *scroll_draw.borrow();

        // Clear background
        cr.set_operator(gtk4::cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(gtk4::cairo::Operator::Over);

        // Font setup
        cr.select_font_face(
            &config_draw.appearance.font_family,
            gtk4::cairo::FontSlant::Normal,
            gtk4::cairo::FontWeight::Normal,
        );
        cr.set_font_size(config_draw.appearance.font_size);

        if segments.is_empty() {
            let c = config_draw.appearance.color_neutral;
            cr.set_source_rgb(c.0, c.1, c.2);
            cr.move_to(10.0, (height as f64 + config_draw.appearance.font_size) / 2.0);
            let _ = cr.show_text("Connecting...");
            return;
        }

        // Calculate widths
        let icon_space = config_draw.appearance.icon_size as f64 + 4.0;
        let mut total_width = 0.0;
        let mut widths: Vec<f64> = Vec::with_capacity(segments.len());

        for seg in &segments {
            let w = match cr.text_extents(&seg.text) {
                Ok(ext) => {
                    if seg.icon.is_some() { icon_space + ext.x_advance() } else { ext.x_advance() }
                }
                Err(_) => if seg.icon.is_some() { icon_space } else { 0.0 },
            };
            widths.push(w);
            total_width += w;
        }

        if total_width <= 0.0 {
            return;
        }

        // Cache width for smooth scrolling
        let mut cached = cached_draw.borrow_mut();
        if *cached <= 0.0 || (*cached - total_width).abs() > 1.0 {
            *cached = total_width;
        }
        let use_width = *cached;
        drop(cached);

        let effective_offset = offset % use_width;
        let text_y = (height as f64 + config_draw.appearance.font_size) / 2.0;
        let icon_y = (height as f64 - config_draw.appearance.icon_size as f64) / 2.0;

        let draw_ticker = |start_x: f64| {
            let mut x = start_x;
            for (i, seg) in segments.iter().enumerate() {
                let seg_width = widths[i];

                if x + seg_width > 0.0 && x < width as f64 {
                    if let Some(ref icon_name) = seg.icon {
                        if let Some(surface) = icons_draw.get(icon_name) {
                            let _ = cr.set_source_surface(surface, x, icon_y);
                            let _ = cr.paint();
                        }
                    }

                    let text_x = if seg.icon.is_some() { x + icon_space } else { x };
                    let color = match seg.direction {
                        ticker::Direction::Up => config_draw.appearance.color_up,
                        ticker::Direction::Down => config_draw.appearance.color_down,
                        ticker::Direction::Neutral => config_draw.appearance.color_neutral,
                    };
                    cr.set_source_rgb(color.0, color.1, color.2);
                    cr.move_to(text_x, text_y);
                    let _ = cr.show_text(&seg.text);
                }
                x += seg_width;
            }
        };

        draw_ticker(-effective_offset);
        let end = use_width - effective_offset;
        if end < width as f64 {
            draw_ticker(end);
        }
    });

    window.set_child(Some(&drawing_area));

    // Animation timer
    let scroll_timer = Rc::clone(&scroll_offset);
    let cached_timer = Rc::clone(&cached_width);
    let drawing_timer = drawing_area.clone();
    let fps = config.animation.fps;
    let speed = config.animation.scroll_speed;

    glib::timeout_add_local(Duration::from_millis(1000 / fps as u64), move || {
        let mut off = scroll_timer.borrow_mut();
        *off += speed / fps as f64;

        let cached = *cached_timer.borrow();
        if cached > 0.0 && *off >= cached {
            *off -= cached;
        }

        drawing_timer.queue_draw();
        glib::ControlFlow::Continue
    });

    // WebSocket connection
    let state_ws = Arc::clone(&state);
    let config_ws = (*config).clone();
    std::thread::spawn(move || {
        websocket::run(&state_ws, &config_ws);
    });

    window.present();

    // Hyprland fullscreen detection
    if let Some(ref monitor) = config.monitor {
        let (tx, rx) = std::sync::mpsc::channel();
        hyprland::watch_fullscreen(monitor.clone(), tx);

        let window_vis = window.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            while let Ok(vis) = rx.try_recv() {
                window_vis.set_visible(vis == hyprland::TickerVisibility::Visible);
            }
            glib::ControlFlow::Continue
        });
    }
}
