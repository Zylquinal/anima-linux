use crate::ui::state::AppContext;
use crate::db::Db;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, Entry, Label, MessageDialog, Orientation, ResponseType, SpinButton, Adjustment, ScrolledWindow, DialogFlags, MessageType, ButtonsType};

const AUTOSTART_FILENAME: &str = "anima-linux.desktop";

fn autostart_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~"))
        .join(".config/autostart")
}

fn is_autostart_enabled() -> bool {
    autostart_dir().join(AUTOSTART_FILENAME).exists()
}

fn enable_autostart() {
    let dir = autostart_dir();
    let _ = std::fs::create_dir_all(&dir);
    let exe = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("anima-linux"))
        .to_string_lossy()
        .to_string();

    let exec_line = match crate::env_detect::detect() {
        crate::env_detect::DisplayEnv::XWaylandExplicit
        | crate::env_detect::DisplayEnv::XWaylandImplicit => {
            format!("env GDK_BACKEND=x11 {}", exe)
        }
        _ => exe,
    };

    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Anima\nExec={exec}\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\n",
        exec = exec_line
    );
    let _ = std::fs::write(dir.join(AUTOSTART_FILENAME), content);
}

fn disable_autostart() {
    let _ = std::fs::remove_file(autostart_dir().join(AUTOSTART_FILENAME));
}

pub fn build(ctx: &AppContext) -> ScrolledWindow {
    let settings_scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    let settings_vbox = GtkBox::new(Orientation::Vertical, 10);
    settings_vbox.set_margin(10);

    // ─── Desktop Environment Status ────────────────────────────────────────
    let de_header = Label::new(None);
    de_header.set_markup("<b>Desktop Environment</b>");
    de_header.set_xalign(0.0);
    settings_vbox.pack_start(&de_header, false, false, 0);

    let display_env = crate::env_detect::detect();
    let is_gnome = crate::env_detect::is_gnome();

    let (status_markup, show_cmd) = match &display_env {
        crate::env_detect::DisplayEnv::X11 =>
            ("<span foreground='#4caf50'>X11 — Taskbar hiding and Always-on-Top are active.</span>", false),
        crate::env_detect::DisplayEnv::XWaylandExplicit =>
            ("<span foreground='#4caf50'>XWayland (configured) — Taskbar hiding and Always-on-Top are active.</span>", false),
        crate::env_detect::DisplayEnv::XWaylandImplicit =>
            ("<span foreground='#ff9800'>XWayland detected — window hints work, but for best results launch with GDK_BACKEND=x11.</span>", true),
        crate::env_detect::DisplayEnv::NativeWayland =>
            ("<span foreground='#f44336'>Native Wayland — mascot windows will appear in your taskbar. Launch with GDK_BACKEND=x11 to fix this.</span>", true),
        crate::env_detect::DisplayEnv::Unknown =>
            ("<span foreground='#9e9e9e'>Unknown environment — hints applied but results may vary.</span>", false),
    };

    let status_label = Label::new(None);
    status_label.set_markup(status_markup);
    status_label.set_xalign(0.0);
    status_label.set_line_wrap(true);
    settings_vbox.pack_start(&status_label, false, false, 0);

    if show_cmd {
        let cmd = "GDK_BACKEND=x11 ./anima-linux";
        let cmd_row = GtkBox::new(Orientation::Horizontal, 5);
        let cmd_label = Label::new(None);
        cmd_label.set_markup(&format!("<tt>{}</tt>", cmd));
        cmd_label.set_selectable(true);
        cmd_label.set_xalign(0.0);
        cmd_row.pack_start(&cmd_label, true, true, 0);

        let copy_btn = Button::with_label("📋 Copy");
        let cmd_str = cmd.to_string();
        copy_btn.connect_clicked(move |_| {
            let clipboard = gtk::Clipboard::get(&gdk::Atom::intern("CLIPBOARD"));
            clipboard.set_text(&cmd_str);
        });
        cmd_row.pack_start(&copy_btn, false, false, 0);
        settings_vbox.pack_start(&cmd_row, false, false, 0);
    }

    if is_gnome && !display_env.is_x11_or_xwayland() {
        let gnome_header = Label::new(None);
        gnome_header.set_markup("<b>Always-on-Top Keybinding (GNOME)</b>");
        gnome_header.set_xalign(0.0);
        gnome_header.set_margin_top(10);
        settings_vbox.pack_start(&gnome_header, false, false, 0);

        let gnome_info = Label::new(Some(
            "Focus a mascot window and press this shortcut to toggle Always-on-Top in GNOME Shell."
        ));
        gnome_info.set_xalign(0.0);
        gnome_info.set_line_wrap(true);
        settings_vbox.pack_start(&gnome_info, false, false, 0);

        let saved_key = ctx.state.borrow().db.get_gnome_always_on_top_key()
            .unwrap_or_else(|_| "<Control><Super>t".to_string());
        let gsettings_current = crate::env_detect::read_gnome_always_on_top_key()
            .unwrap_or_else(|| "not set".to_string());

        let current_label = Label::new(None);
        current_label.set_markup(&format!(
            "Current in GNOME: <tt>{}</tt>",
            gsettings_current.replace('<', "&lt;").replace('>', "&gt;")
        ));
        current_label.set_xalign(0.0);
        settings_vbox.pack_start(&current_label, false, false, 0);

        let key_row = GtkBox::new(Orientation::Horizontal, 5);
        key_row.pack_start(&Label::new(Some("Keybinding:")), false, false, 0);

        let key_entry = Entry::new();
        key_entry.set_text(&saved_key);
        key_entry.set_placeholder_text(Some("<Control><Super>t"));
        key_entry.set_hexpand(true);
        key_row.pack_start(&key_entry, true, true, 0);

        let apply_btn = Button::with_label("Apply");
        let state_key = ctx.state.clone();
        let key_entry_a = key_entry.clone();
        let current_label_a = current_label.clone();
        let win_gnome = ctx.window.clone();
        apply_btn.connect_clicked(move |_| {
            let key_text = key_entry_a.text().to_string();
            if key_text.is_empty() {
                let err = MessageDialog::new(Some(&win_gnome), DialogFlags::MODAL, MessageType::Error, ButtonsType::Ok, "Please enter a keybinding.");
                err.run(); err.close(); return;
            }
            let _ = state_key.borrow().db.set_gnome_always_on_top_key(&key_text);
            if crate::env_detect::set_gnome_always_on_top_key(&key_text) {
                current_label_a.set_markup(&format!(
                    "Current in GNOME: <tt>{}</tt>",
                    key_text.replace('<', "&lt;").replace('>', "&gt;")
                ));
                let ok = MessageDialog::new(Some(&win_gnome), DialogFlags::MODAL, MessageType::Info, ButtonsType::Ok,
                    &format!("Keybinding set to '{}'.\nFocus a mascot window and press it to toggle Always-on-Top.", key_text));
                ok.run(); ok.close();
            } else {
                let err = MessageDialog::new(Some(&win_gnome), DialogFlags::MODAL, MessageType::Error, ButtonsType::Ok,
                    "Failed to apply keybinding via gsettings. Make sure gsettings is available.");
                err.run(); err.close();
            }
        });
        key_row.pack_start(&apply_btn, false, false, 0);

        let reset_btn = Button::with_label("↺");
        let state_key2 = ctx.state.clone();
        let key_entry_r = key_entry.clone();
        reset_btn.connect_clicked(move |_| {
            let default = "<Control><Super>t";
            let _ = state_key2.borrow().db.set_gnome_always_on_top_key(default);
            let _ = crate::env_detect::set_gnome_always_on_top_key(default);
            key_entry_r.set_text(default);
        });
        key_row.pack_start(&reset_btn, false, false, 0);

        settings_vbox.pack_start(&key_row, false, false, 0);
    }

    let lu_header = Label::new(None);
    lu_header.set_markup("<b>Live Update</b>");
    lu_header.set_xalign(0.0);
    lu_header.set_margin_top(10);
    settings_vbox.pack_start(&lu_header, false, false, 0);

    let delay_box = GtkBox::new(Orientation::Horizontal, 5);
    delay_box.add(&Label::new(Some("Delay (ms):")));
    let delay_adj = Adjustment::new(ctx.state.borrow().db.get_live_update_delay().unwrap_or(300) as f64, 50.0, 2000.0, 50.0, 100.0, 0.0);
    let delay_spin = SpinButton::new(Some(&delay_adj), 1.0, 0);
    delay_box.add(&delay_spin);
    settings_vbox.pack_start(&delay_box, false, false, 0);

    let state_settings = ctx.state.clone();
    delay_spin.connect_value_changed(move |spin| {
        let _ = state_settings.borrow().db.set_live_update_delay(spin.value() as u64);
    });

    let live_update_check = CheckButton::with_label("Enable Live Update");
    live_update_check.set_active(ctx.state.borrow().db.get_live_update_enabled().unwrap_or(true));
    settings_vbox.pack_start(&live_update_check, false, false, 0);

    let state_settings_toggle = ctx.state.clone();
    live_update_check.connect_toggled(move |btn| {
        let _ = state_settings_toggle.borrow().db.set_live_update_enabled(btn.is_active());
    });

    let maintenance_header = Label::new(None);
    maintenance_header.set_markup("<b>Maintenance</b>");
    maintenance_header.set_xalign(0.0);
    maintenance_header.set_margin_top(10);
    settings_vbox.pack_start(&maintenance_header, false, false, 0);

    let clear_cache_btn = Button::with_label("Clear Cache");
    settings_vbox.pack_start(&clear_cache_btn, false, false, 0);
    let win_c3 = ctx.window.clone();
    clear_cache_btn.connect_clicked(move |_| {
        if crate::anima_resize::clear_cache().is_ok() {
            let info = MessageDialog::new(Some(&win_c3), DialogFlags::MODAL, MessageType::Info, ButtonsType::Ok, "Cache cleared successfully!");
            info.run(); info.close();
        } else {
            let err = MessageDialog::new(Some(&win_c3), DialogFlags::MODAL, MessageType::Error, ButtonsType::Ok, "Failed to clear cache.");
            err.run(); err.close();
        }
    });

    let clear_data_btn = Button::with_label("Clear All Data");
    settings_vbox.pack_start(&clear_data_btn, false, false, 0);
    let win_c2 = ctx.window.clone();
    let state_clear = ctx.state.clone();
    let ref_l_clear = ctx.refresh_library.clone();
    let ref_a_clear = ctx.refresh_active_spawns.clone();
    clear_data_btn.connect_clicked(move |_| {
        let dialog = MessageDialog::new(Some(&win_c2), DialogFlags::MODAL, MessageType::Warning, ButtonsType::OkCancel, "Are you sure you want to clear ALL data?");
        if dialog.run() == ResponseType::Ok {
            let mut st = state_clear.borrow_mut();
            let windows: Vec<_> = st.animas.iter().map(|a| a.window.clone()).collect();
            st.animas.clear();
            let _ = st.db.clear_all_data();
            drop(st);
            for w in windows { w.close(); }
            if let Some(f) = ref_l_clear.borrow().as_ref() { f(); }
            if let Some(f) = ref_a_clear.borrow().as_ref() { f(); }
        }
        dialog.close();
    });

    let open_db_btn = Button::with_label("Open DB Folder");
    settings_vbox.pack_start(&open_db_btn, false, false, 0);
    open_db_btn.connect_clicked(move |_| {
        let _ = std::process::Command::new("xdg-open").arg(Db::app_dir()).spawn();
    });

    // License
    let license_btn = Button::with_label("Show License");
    settings_vbox.pack_start(&license_btn, false, false, 0);
    let win_c4 = ctx.window.clone();
    license_btn.connect_clicked(move |_| {
        let license_text = include_str!("../../LICENSE");
        let dialog = gtk::Dialog::with_buttons(
            Some("License"), Some(&win_c4), DialogFlags::MODAL,
            &[("Close", ResponseType::Close)]
        );
        let content_area = dialog.content_area();
        let scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
        scroll.set_size_request(400, 300);
        let label = Label::new(Some(license_text));
        label.set_margin(10);
        label.set_line_wrap(true);
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        scroll.add(&label);
        content_area.pack_start(&scroll, true, true, 0);
        dialog.show_all();
        dialog.run();
        dialog.close();
    });

    // Autostart
    let autostart_header = Label::new(None);
    autostart_header.set_markup("<b>Autostart</b>");
    autostart_header.set_xalign(0.0);
    autostart_header.set_margin_top(10);
    settings_vbox.pack_start(&autostart_header, false, false, 0);

    let autostart_check = CheckButton::with_label("Launch Anima on login");
    autostart_check.set_active(is_autostart_enabled());
    settings_vbox.pack_start(&autostart_check, false, false, 0);

    autostart_check.connect_toggled(move |btn| {
        if btn.is_active() {
            enable_autostart();
        } else {
            disable_autostart();
        }
    });

    // Version Footer
    let version_label = Label::new(Some(&format!("Version: {}", env!("CARGO_PKG_VERSION"))));
    version_label.set_margin_top(20);
    version_label.set_xalign(0.5);
    settings_vbox.pack_end(&version_label, false, false, 0);

    settings_scroll.add(&settings_vbox);
    settings_scroll
}
