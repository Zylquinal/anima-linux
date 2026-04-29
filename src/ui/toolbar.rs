use crate::ui::state::AppContext;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, FileChooserAction, FileChooserDialog, FileFilter,
          Label, MessageDialog, Orientation, ResponseType, SpinButton, Adjustment, Spinner};
use std::cell::RefCell;

pub fn build(ctx: &AppContext) -> GtkBox {
    let toolbar = GtkBox::new(Orientation::Horizontal, 10);
    toolbar.set_margin(10);

    let import_btn     = Button::with_label("Import Anima");
    let import_spinner = Spinner::new();
    import_spinner.set_no_show_all(true);

    let max_spawns = ctx.state.borrow().db.get_max_spawns().unwrap_or(10);
    let max_spawns_box = GtkBox::new(Orientation::Horizontal, 5);
    max_spawns_box.add(&Label::new(Some("Max Spawns:")));
    let max_spawns_adj = Adjustment::new(max_spawns as f64, 1.0, 100.0, 1.0, 10.0, 0.0);
    let max_spawns_spin = SpinButton::new(Some(&max_spawns_adj), 1.0, 0);
    max_spawns_box.add(&max_spawns_spin);

    toolbar.pack_start(&import_btn, false, false, 0);
    toolbar.pack_start(&import_spinner, false, false, 0);
    toolbar.pack_end(&max_spawns_box, false, false, 0);

    let state_for_max = ctx.state.clone();
    max_spawns_spin.connect_value_changed(move |spin| {
        let val = spin.value() as i32;
        let _ = state_for_max.borrow().db.set_max_spawns(val);
    });

    let win_c   = ctx.window.clone();
    let state_i = ctx.state.clone();
    let ref_l   = ctx.refresh_library.clone();

    import_btn.connect_clicked({
        let import_btn     = import_btn.clone();
        let import_spinner = import_spinner.clone();
        move |_| {
            let fc = FileChooserDialog::new(Some("Import Anima"), Some(&win_c), FileChooserAction::Open);
            fc.add_buttons(&[("Cancel", ResponseType::Cancel), ("Open", ResponseType::Accept)]);

            // ── File filters ──────────────────────────────────────────────────
            let filter_all = FileFilter::new();
            filter_all.set_name(Some("Supported Files (image, GIF, video)"));
            for pat in &[
                "*.gif", "*.webp", "*.png", "*.jpg", "*.jpeg", "*.bmp", "*.tiff", "*.tga",
                "*.mp4", "*.mkv", "*.webm", "*.avi", "*.mov", "*.flv", "*.m4v", "*.wmv",
            ] { filter_all.add_pattern(pat); }
            fc.add_filter(filter_all);

            let filter_gif = FileFilter::new();
            filter_gif.set_name(Some("Animated GIF (*.gif)"));
            filter_gif.add_pattern("*.gif");
            fc.add_filter(filter_gif);

            let filter_img = FileFilter::new();
            filter_img.set_name(Some("Images (png, jpg, webp, bmp…)"));
            for pat in &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.bmp", "*.tiff", "*.tga"] {
                filter_img.add_pattern(pat);
            }
            fc.add_filter(filter_img);

            let filter_vid = FileFilter::new();
            filter_vid.set_name(Some("Video (mp4, mkv, webm, avi…)"));
            for pat in &["*.mp4", "*.mkv", "*.webm", "*.avi", "*.mov", "*.flv", "*.m4v", "*.wmv"] {
                filter_vid.add_pattern(pat);
            }
            fc.add_filter(filter_vid);

            if fc.run() == ResponseType::Accept {
                if let Some(path) = fc.filename() {
                    // Warn on large files
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.len() > 10 * 1024 * 1024 {
                            let warn = MessageDialog::new(
                                Some(&win_c), gtk::DialogFlags::MODAL,
                                gtk::MessageType::Warning, gtk::ButtonsType::YesNo,
                                "This file is very large (>10 MB). Processing may be slow. Continue?",
                            );
                            let res = warn.run();
                            warn.close();
                            if res != ResponseType::Yes { fc.close(); return; }
                        }
                    }

                    // Ask for a display name
                    let nd = MessageDialog::new(
                        Some(&win_c), gtk::DialogFlags::MODAL,
                        gtk::MessageType::Question, gtk::ButtonsType::OkCancel, "Name:",
                    );
                    let entry = Entry::new();
                    entry.set_text(&path.file_stem().unwrap_or_default().to_string_lossy());
                    nd.content_area().pack_start(&entry, true, true, 0);
                    nd.show_all();

                    if nd.run() == ResponseType::Ok {
                        let anim_name = entry.text().to_string();
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        // All imported files are normalised to GIF in the app data dir.
                        let dest = crate::db::Db::app_dir().join(format!("{stem}.gif"));

                        // Disable button + show spinner during potentially slow conversion.
                        import_btn.set_sensitive(false);
                        import_spinner.start();
                        import_spinner.show();

                        // Heavy conversion runs on a background thread.
                        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                        let dest_str = dest.to_str().unwrap_or("").to_string();
                        let dest_thread = dest.clone();
                        std::thread::spawn(move || {
                            let result = crate::anima_resize::import_as_gif(&path, &dest_thread);
                            tx.send(result).ok();
                        });
                        let state_poll   = state_i.clone();
                        let ref_poll     = ref_l.clone();
                        let win_poll     = win_c.clone();
                        let btn_poll     = import_btn.clone();
                        let spinner_poll = import_spinner.clone();
                        let rx_cell      = RefCell::new(rx);
                        gtk::glib::timeout_add_local(
                            std::time::Duration::from_millis(16),
                            move || {
                                use std::sync::mpsc::TryRecvError;
                                match rx_cell.borrow_mut().try_recv() {
                                    Ok(Ok(())) => {
                                        let _ = state_poll.borrow().db.insert_animation(
                                            &anim_name, &dest_str,
                                        );
                                        if let Some(f) = ref_poll.borrow().as_ref() { f(); }
                                        spinner_poll.stop();
                                        spinner_poll.hide();
                                        btn_poll.set_sensitive(true);
                                        gtk::glib::ControlFlow::Break
                                    }
                                    Ok(Err(msg)) => {
                                        let err = MessageDialog::new(
                                            Some(&win_poll), gtk::DialogFlags::MODAL,
                                            gtk::MessageType::Error, gtk::ButtonsType::Ok,
                                            &format!("Import failed:\n{msg}"),
                                        );
                                        err.run();
                                        err.close();
                                        spinner_poll.stop();
                                        spinner_poll.hide();
                                        btn_poll.set_sensitive(true);
                                        gtk::glib::ControlFlow::Break
                                    }
                                    Err(TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                                    Err(_) => {
                                        spinner_poll.stop();
                                        spinner_poll.hide();
                                        btn_poll.set_sensitive(true);
                                        gtk::glib::ControlFlow::Break
                                    }
                                }
                            },
                        );
                    }
                    nd.close();
                }
            }
            fc.close();
        }
    });

    toolbar
}
