pub mod state;
pub mod toolbar;
pub mod library;
pub mod instances;
pub mod control_panel;
pub mod settings;

use crate::anima::AnimaWindow;
use crate::db::Db;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Label, ListBox, Notebook, Orientation, Paned, Scale, ScrolledWindow, Separator, Adjustment};
use gdk_pixbuf::Pixbuf;
use std::cell::RefCell;
use std::rc::Rc;
use state::{AppContext, AppState};

static APP_ICON_BYTES: &[u8] = include_bytes!("../../icon.png");

fn load_app_icon() -> Option<Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(APP_ICON_BYTES).ok()?;
    loader.close().ok()?;
    loader.pixbuf()
}

pub fn build_ui(app: &Application) {
    println!("Building UI...");
    let db = Db::new().expect("Failed to init DB");

    let state = Rc::new(RefCell::new(AppState {
        db,
        animas: Vec::new(),
        global_opacity: 1.0,
        instance_counter: 0,
    }));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Anima Management")
        .default_width(1000)
        .default_height(800)
        .build();

    if let Some(icon) = load_app_icon() {
        window.set_icon(Some(&icon));
    }

    let ctx = AppContext {
        window: window.clone(),
        state: state.clone(),
        refresh_library: Rc::new(RefCell::new(None)),
        refresh_active_spawns: Rc::new(RefCell::new(None)),
        update_control_panel: Rc::new(RefCell::new(None)),
        current_edit_target: Rc::new(RefCell::new(None)),
    };

    let main_vbox = GtkBox::new(Orientation::Vertical, 0);

    let toolbar = toolbar::build(&ctx);
    main_vbox.pack_start(&toolbar, false, false, 0);
    main_vbox.pack_start(&Separator::new(Orientation::Horizontal), false, false, 0);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_position(350);

    let left_vbox = GtkBox::new(Orientation::Vertical, 5);
    left_vbox.set_margin(5);

    let notebook = Notebook::new();

    let library_list = ListBox::new();
    library_list.set_activate_on_single_click(true);
    let library_scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    library_scroll.add(&library_list);
    library::build(&ctx, &library_list);

    let active_spawns_list = ListBox::new();
    active_spawns_list.set_activate_on_single_click(true);
    let spawns_scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    spawns_scroll.add(&active_spawns_list);

    notebook.append_page(&library_scroll, Some(&Label::new(Some("Library"))));
    notebook.append_page(&spawns_scroll, Some(&Label::new(Some("Instances"))));

    let settings_scroll = settings::build(&ctx);
    notebook.append_page(&settings_scroll, Some(&Label::new(Some("Settings"))));

    left_vbox.pack_start(&notebook, true, true, 0);

    let opacity_box = GtkBox::new(Orientation::Vertical, 2);
    opacity_box.set_margin(5);
    opacity_box.add(&Label::new(Some("Global Opacity")));
    let opacity_adj = Adjustment::new(1.0, 0.1, 1.0, 0.05, 0.0, 0.0);
    let opacity_scale = Scale::new(Orientation::Horizontal, Some(&opacity_adj));
    opacity_box.add(&opacity_scale);
    left_vbox.pack_start(&opacity_box, false, false, 0);

    paned.pack1(&left_vbox, false, false);

    let right_scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    let control_panel = GtkBox::new(Orientation::Vertical, 15);
    control_panel.set_margin(20);
    right_scroll.add(&control_panel);
    paned.pack2(&right_scroll, true, false);

    instances::build(&ctx, &active_spawns_list, &control_panel);
    control_panel::build(&ctx, &right_scroll, &control_panel);

    main_vbox.pack_start(&paned, true, true, 0);
    window.add(&main_vbox);

    let state_for_opacity = state.clone();
    opacity_scale.connect_value_changed(move |scale| {
        let val = scale.value();
        let mut s = state_for_opacity.borrow_mut();
        s.global_opacity = val;
        let instances = s.db.get_all_instances().unwrap_or_default();
        for anima in s.animas.iter() {
            if let Some(inst) = instances.iter().find(|i| i.id == anima.instance_db_id) {
                anima.window.set_opacity(inst.opacity * val);
            }
        }
    });

    // Initial Load & Auto-spawn
    if let Some(f) = ctx.refresh_library.borrow().as_ref() { f(); }
    {
        let mut s = state.borrow_mut();
        let max = s.db.get_max_spawns().unwrap_or(10) as usize;
        let instances = s.db.get_all_instances().unwrap_or_default();
        let anims = s.db.get_all_animations().unwrap_or_default();

        let mut to_spawn = Vec::new();
        let mut count = 0;
        for inst in instances.iter().filter(|a| a.auto_spawn) {
            if let Some(anim) = anims.iter().find(|a| a.id == inst.animation_id) {
                if count < max {
                    s.instance_counter += 1;
                    to_spawn.push((
                        s.instance_counter, inst.id, anim.name.clone(), anim.file_path.clone(),
                        inst.scale, inst.opacity * s.global_opacity, inst.x, inst.y,
                        inst.mirror, inst.flip_v, inst.roll, inst.pitch, inst.yaw,
                        inst.temperature, inst.contrast, inst.brightness, inst.saturation, inst.hue
                    ));
                    count += 1;
                }
            }
        }
        drop(s);

        for args in to_spawn {
            let anima = AnimaWindow::new(
                args.0, args.1, args.2, &args.3, args.4, args.5, args.6, args.7,
                args.8, args.9, args.10, args.11, args.12, args.13, args.14, args.15, args.16, args.17
            );
            let ctx_clone = ctx.clone();
            state::register_anima_window(&ctx_clone, anima);
        }
    }
    if let Some(f) = ctx.refresh_active_spawns.borrow().as_ref() { f(); }

    // Persistent Position Timer
    let state_t = state.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
        let s = state_t.borrow();
        for anima in &s.animas {
            let (x, y) = anima.position();
            let _ = s.db.update_instance_position(anima.instance_db_id, x, y);
        }
        gtk::glib::ControlFlow::Continue
    });

    // When the "tray" feature is compiled in, clicking X hides the window and
    // the app lives in the system tray. Without the feature the window closes
    // normally (default GTK behaviour).
    #[cfg(feature = "tray")]
    setup_tray(app, &window);

    window.show_all();
}

// Everything below is only compiled when the "tray" Cargo feature is enabled.

/// Write the embedded icon PNG to a temp directory so libappindicator can
/// reference it by filesystem path (the C library does not accept raw bytes).
#[cfg(feature = "tray")]
fn write_tray_icon() -> Option<std::path::PathBuf> {
    let tmp = std::env::temp_dir().join("anima-linux-tray");
    std::fs::create_dir_all(&tmp).ok()?;
    let icon_path = tmp.join("anima-linux.png");
    std::fs::write(&icon_path, APP_ICON_BYTES).ok()?;
    Some(icon_path)
}

/// Set up the system tray indicator and intercept the window close button.
///
/// Called only when the `tray` feature is enabled. If libappindicator fails
/// to initialise (e.g. no status-notifier host running), the function still
/// hooks the delete-event so the window hides.
#[cfg(feature = "tray")]
fn setup_tray(app: &Application, window: &ApplicationWindow) {
    // Keep the GtkApplication alive even when every window is hidden.
    // ApplicationExtManual::hold() is in scope via gtk::prelude::*.
    // Leak the guard so it is never dropped, the app stays alive until quit().
    std::mem::forget(app.hold());

    let tray_menu = gtk::Menu::new();

    let show_item = gtk::MenuItem::with_label("Show / Hide");
    let quit_item = gtk::MenuItem::with_label("Quit Anima");

    tray_menu.append(&show_item);
    tray_menu.append(&gtk::SeparatorMenuItem::new());
    tray_menu.append(&quit_item);
    tray_menu.show_all();

    // AppIndicator
    let mut indicator = if let Some(icon_path) = write_tray_icon() {
        let theme_dir = icon_path
            .parent()
            .unwrap_or(std::path::Path::new("/tmp"))
            .to_string_lossy()
            .to_string();
        let icon_name = icon_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        libappindicator::AppIndicator::with_path("anima-linux", &icon_name, &theme_dir)
    } else {
        libappindicator::AppIndicator::new("anima-linux", "application-x-executable")
    };

    indicator.set_status(libappindicator::AppIndicatorStatus::Active);
    indicator.set_title("Anima");
    indicator.set_menu(&mut tray_menu.clone());

    // The indicator must live for the entire process lifetime.
    std::mem::forget(indicator);

    // Show / Hide toggle
    let win_toggle = window.clone();
    show_item.connect_activate(move |_| {
        if win_toggle.is_visible() {
            win_toggle.hide();
        } else {
            win_toggle.show_all();
            win_toggle.present();
        }
    });

    // Quit
    let app_quit = app.clone();
    quit_item.connect_activate(move |_| {
        <gtk::Application as gio::prelude::ApplicationExt>::quit(&app_quit);
    });

    // Intercept the window X button to hide instead of destroy
    window.connect_delete_event(move |win, _| {
        win.hide();
        gtk::glib::Propagation::Stop
    });
}
