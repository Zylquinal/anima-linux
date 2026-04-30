use crate::ui::state::{AppContext, EditTarget};
use gtk::prelude::*;
use gtk::{Box as GtkBox, EventBox, Label, ListBox, ListBoxRow, MenuItem, Menu, Orientation};
use std::rc::Rc;

pub fn build(ctx: &AppContext, library_list: &ListBox) {
    let library_list_c = library_list.clone();
    let state = ctx.state.clone();
    let update_p = ctx.update_control_panel.clone();
    let win_outer = ctx.window.clone();
    let refresh_lib = ctx.refresh_library.clone();

    *ctx.refresh_library.borrow_mut() = Some(Rc::new(move || {
        for child in library_list_c.children() { library_list_c.remove(&child); }
        let anims = state.borrow().db.get_all_animations().unwrap_or_default();
        for anim in anims {
            let row = ListBoxRow::new();
            let anim_id = anim.id;
            let anim_name = anim.name.clone();
            let anim_path = anim.file_path.clone();
            let up = update_p.clone();
            let state_menu = state.clone();
            let refresh_menu = refresh_lib.clone();
            let win_menu = win_outer.clone();

            let ev = EventBox::new();
            row.add(&ev);
            let hbox = GtkBox::new(Orientation::Horizontal, 10);
            ev.add(&hbox);

            let lbl = Label::new(Some(&anim.name));
            lbl.set_xalign(0.0);
            hbox.pack_start(&lbl, true, true, 5);

            // Pre-clone for the closure captures
            let row_for_closure = row.clone();
            let lib_for_closure = library_list_c.clone();

            ev.connect_button_press_event(move |_widget, event| {
                match event.button() {
                    1 => {
                        // Left-click: open control panel
                        println!("Library row clicked: {}", anim_id);
                        if let Some(f) = up.borrow().as_ref() { f(EditTarget::Library(anim_id)); }
                    }
                    3 => {
                        // Right-click: show context menu
                        let menu = Menu::new();

                        // Rename
                        let rename_item = MenuItem::with_label("Rename");
                        let state_rn   = state_menu.clone();
                        let refresh_rn = refresh_menu.clone();
                        let win_rn     = win_menu.clone();
                        let name_rn    = anim_name.clone();
                        rename_item.connect_activate(move |_| {
                            let dialog = gtk::Dialog::with_buttons(
                                Some("Rename Animation"),
                                Some(&win_rn),
                                gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
                                &[("OK", gtk::ResponseType::Ok), ("Cancel", gtk::ResponseType::Cancel)],
                            );
                            dialog.set_default_response(gtk::ResponseType::Ok);

                            let entry = gtk::Entry::new();
                            entry.set_text(&name_rn);
                            entry.set_activates_default(true);
                            entry.set_margin_start(12);
                            entry.set_margin_end(12);
                            entry.set_margin_top(8);
                            entry.set_margin_bottom(8);

                            let content = dialog.content_area();
                            content.add(&entry);
                            content.show_all();

                            if dialog.run() == gtk::ResponseType::Ok {
                                let new_name = entry.text().to_string();
                                if !new_name.trim().is_empty() {
                                    let _ = state_rn.borrow().db.rename_animation(anim_id, new_name.trim());
                                    if let Some(f) = refresh_rn.borrow().as_ref() { f(); }
                                }
                            }
                            dialog.close();
                        });
                        menu.append(&rename_item);

                        // Download (export)
                        let dl_item  = MenuItem::with_label("Download / Export");
                        let win_dl   = win_menu.clone();
                        let path_dl  = anim_path.clone();
                        let name_dl  = anim_name.clone();
                        dl_item.connect_activate(move |_| {
                            let fc = gtk::FileChooserDialog::new(
                                Some("Save Animation As"),
                                Some(&win_dl),
                                gtk::FileChooserAction::Save,
                            );
                            fc.add_button("Cancel", gtk::ResponseType::Cancel);
                            fc.add_button("Save",   gtk::ResponseType::Accept);
                            fc.set_current_name(&format!("{}.gif", name_dl));
                            fc.set_do_overwrite_confirmation(true);

                            if fc.run() == gtk::ResponseType::Accept {
                                if let Some(dest) = fc.filename() {
                                    if let Err(e) = std::fs::copy(&path_dl, &dest) {
                                        eprintln!("Download failed: {e}");
                                    }
                                }
                            }
                            fc.close();
                        });
                        menu.append(&dl_item);

                        // Delete
                        let del_item  = MenuItem::with_label("Delete");
                        let state_del = state_menu.clone();
                        let win_del   = win_menu.clone();
                        let lib_del   = lib_for_closure.clone();
                        let row_del   = row_for_closure.clone();
                        del_item.connect_activate(move |_| {
                            let instances = state_del.borrow().db.get_all_instances().unwrap_or_default();
                            if instances.iter().any(|i| i.animation_id == anim_id) {
                                let msg = gtk::MessageDialog::new(
                                    Some(&win_del),
                                    gtk::DialogFlags::MODAL,
                                    gtk::MessageType::Error,
                                    gtk::ButtonsType::Ok,
                                    "Cannot delete: animation is used by one or more instances. Delete the instances first.",
                                );
                                msg.run();
                                msg.close();
                                return;
                            }
                            let msg = gtk::MessageDialog::new(
                                Some(&win_del),
                                gtk::DialogFlags::MODAL,
                                gtk::MessageType::Warning,
                                gtk::ButtonsType::OkCancel,
                                "Are you sure you want to delete this animation?",
                            );
                            if msg.run() == gtk::ResponseType::Ok {
                                let _ = state_del.borrow().db.delete_animation(anim_id);
                                lib_del.remove(&row_del);
                            }
                            msg.close();
                        });
                        menu.append(&del_item);

                        menu.show_all();
                        menu.popup_at_pointer(Some(event));
                    }
                    _ => {}
                }
                gtk::glib::Propagation::Proceed
            });

            library_list_c.add(&row);
        }
        library_list_c.show_all();
    }));
}
