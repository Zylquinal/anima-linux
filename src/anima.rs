use gdk_pixbuf::PixbufAnimation;
use gtk::prelude::*;
use gtk::{CssProvider, Image, StyleContext, Window};
use std::process;
use std::rc::Rc;
use std::cell::RefCell;

#[allow(dead_code)]
pub struct AnimaWindow {
    pub id: usize,
    pub instance_db_id: i32,
    pub name: String,
    pub window: Window,
    pub scale: f64,
    pub mirror: bool,
    pub flip_v: bool,
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub temp: f64,
    pub contrast: f64,
    pub brightness: f64,
    pub saturation: f64,
    pub hue: f64,
}

impl AnimaWindow {
    pub fn new(
        id: usize,
        instance_db_id: i32,
        name: String,
        gif_path: &str,
        scale: f64,
        opacity: f64,
        x: i32,
        y: i32,
        mirror: bool,
        flip_v: bool,
        roll: f64,
        pitch: f64,
        yaw: f64,
        temp: f64,
        contrast: f64,
        brightness: f64,
        saturation: f64,
        hue: f64,
    ) -> Self {
        // Create a standard Window instead of an ApplicationWindow
        let window = Window::builder()
            .title("Desktop Mascot")
            .decorated(false)
            .skip_taskbar_hint(true)
            .skip_pager_hint(true)
            .type_hint(gdk::WindowTypeHint::Utility)
            .build();

        let display_env = crate::env_detect::detect();
        if display_env.is_x11_or_xwayland() {
            // On X11/XWayland, EWMH hints work
            window.set_keep_above(true);
        }
        // Note: on native Wayland, skip_taskbar_hint and keep_above are set in the builder
        // but will be silently ignored by the compositor. That is expected behaviour.
        window.stick();
        window.set_app_paintable(true);
        window.set_widget_name("anima-mascot");
        window.set_role("mascot");

        if let Some(screen) = gtk::prelude::WidgetExt::screen(&window) {
            if let Some(visual) = screen.rgba_visual() {
                if screen.is_composited() {
                    window.set_visual(Some(&visual));
                }
            }
        }

        window.set_opacity(opacity);

        let provider = CssProvider::new();
        provider
            .load_from_data(b"#anima-mascot { background-color: transparent; }")
            .expect("Failed to load CSS");

        if let Some(screen) = gtk::prelude::WidgetExt::screen(&window) {
            StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let processed_gif_path = crate::anima_resize::ensure_processed_gif(
            gif_path, scale, mirror, flip_v, roll, pitch, yaw, temp, contrast, brightness, saturation, hue,
        );

        let animation = PixbufAnimation::from_file(&processed_gif_path).unwrap_or_else(|e| {
            eprintln!("Error loading GIF: {}", e);
            process::exit(1);
        });
        let image = Image::from_animation(&animation);

        window.add(&image);

        window.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        window.connect_button_press_event(|win, event| {
            if event.button() == 1 { // Left mouse button
                let (root_x, root_y) = event.root();
                win.begin_move_drag(
                    event.button() as i32,
                    root_x as i32,
                    root_y as i32,
                    event.time(),
                );
            }
            gtk::glib::Propagation::Proceed
        });

        if x != 0 || y != 0 {
            window.move_(x, y);
        }

        window.show_all();

        Self {
            id,
            instance_db_id,
            name,
            window,
            scale,
            mirror,
            flip_v,
            roll,
            pitch,
            yaw,
            temp,
            contrast,
            brightness,
            saturation,
            hue,
        }
    }

    pub fn locate(&self) {
        let win = self.window.clone();

        let step = Rc::new(RefCell::new(0));
        let start_opacity = win.opacity();

        gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let mut s = step.borrow_mut();
            if *s >= 6 {
                win.set_opacity(start_opacity);
                return gtk::glib::ControlFlow::Break;
            }

            if *s % 2 == 0 {
                win.set_opacity(start_opacity * 0.3);
            } else {
                win.set_opacity(start_opacity);
            }

            *s += 1;
            gtk::glib::ControlFlow::Continue
        });
    }

    pub fn position(&self) -> (i32, i32) {
        self.window.position()
    }
}
