use crate::ui::state::{AppContext, EditTarget};
use crate::anima::AnimaWindow;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, EventBox, Label, ListBox, ListBoxRow, Menu, MenuItem, Orientation};
use std::rc::Rc;

pub fn build(ctx: &AppContext, active_spawns_list: &ListBox, control_panel: &GtkBox) {
    let active_spawns_list_c = active_spawns_list.clone();
    let state = ctx.state.clone();
    let update_p = ctx.update_control_panel.clone();
    let refresh_inner = ctx.refresh_active_spawns.clone();
    let current_edit_target_list = ctx.current_edit_target.clone();
    let cp_list = control_panel.clone();
    let ctx_outer = ctx.clone();
    let win_outer = ctx.window.clone();

    *ctx.refresh_active_spawns.borrow_mut() = Some(Rc::new(move || {
        for child in active_spawns_list_c.children() { active_spawns_list_c.remove(&child); }
        let s = state.borrow();
        let instances = s.db.get_all_instances().unwrap_or_default();
        let anims = s.db.get_all_animations().unwrap_or_default();
        for inst in instances {
            let anim = match anims.iter().find(|a| a.id == inst.animation_id) { Some(a) => a, None => continue };
            let running_opt = s.animas.iter().find(|x| x.instance_db_id == inst.id);
            let is_running  = running_opt.is_some();
            let running_id  = running_opt.map(|x| x.id).unwrap_or(0);

            let db_id = inst.id;
            let anim_name = anim.name.clone();
            let anim_path = anim.file_path.clone();
            let inst_scale = inst.scale;
            let _inst_opacity = inst.opacity;
            let _inst_x = inst.x;
            let _inst_y = inst.y;
            let inst_mirror = inst.mirror;
            let inst_flip_v = inst.flip_v;
            let inst_roll = inst.roll;
            let inst_pitch = inst.pitch;
            let inst_yaw = inst.yaw;
            let inst_temp = inst.temperature;
            let inst_contrast = inst.contrast;
            let inst_bright = inst.brightness;
            let inst_sat = inst.saturation;
            let inst_hue = inst.hue;

            let row = ListBoxRow::new();
            let up  = update_p.clone();

            let ev   = EventBox::new();
            row.add(&ev);
            let hbox = GtkBox::new(Orientation::Horizontal, 10);
            ev.add(&hbox);

            let label_text = format!(
                "{} (ID:{}){}", anim_name, db_id,
                if is_running { " [Running]" } else { "" }
            );
            let lbl = Label::new(Some(&label_text));
            lbl.set_xalign(0.0);
            hbox.pack_start(&lbl, true, true, 5);

            // --- Left-click opens control panel; Right-click shows context menu ---
            {
                let state_ev   = state.clone();
                let ref_ev     = refresh_inner.clone();
                let cur_ev     = current_edit_target_list.clone();
                let cp_ev      = cp_list.clone();
                let _ctx_ev     = ctx_outer.clone();
                let win_ev     = win_outer.clone();
                let path_dl    = anim_path.clone();
                let name_dl    = anim_name.clone();

                ev.connect_button_press_event(move |_widget, event| {
                    match event.button() {
                        1 => {
                            // Left-click: open control panel
                            println!("Active row clicked: {}", db_id);
                            if let Some(f) = up.borrow().as_ref() { f(EditTarget::Instance(db_id)); }
                        }
                        3 => {
                            // Right-click: context menu
                            let menu = Menu::new();

                            if is_running {
                                // --- Locate ---
                                let loc_item  = MenuItem::with_label("Locate");
                                let state_loc = state_ev.clone();
                                loc_item.connect_activate(move |_| {
                                    if let Some(a) = state_loc.borrow().animas.iter().find(|x| x.id == running_id) {
                                        a.locate();
                                    }
                                });
                                menu.append(&loc_item);
                            }

                            // --- Download (export processed GIF) ---
                            let dl_item  = MenuItem::with_label("Download / Export");
                            let win_dl2  = win_ev.clone();
                            let path_dl2 = path_dl.clone();
                            let name_dl2 = name_dl.clone();
                            dl_item.connect_activate(move |_| {
                                // Compute processed gif path (may already be cached)
                                let processed = crate::anima_resize::ensure_processed_gif(
                                    &path_dl2,
                                    inst_scale, inst_mirror, inst_flip_v,
                                    inst_roll, inst_pitch, inst_yaw,
                                    inst_temp, inst_contrast, inst_bright, inst_sat, inst_hue,
                                );
                                let fc = gtk::FileChooserDialog::new(
                                    Some("Save Processed Animation As"),
                                    Some(&win_dl2),
                                    gtk::FileChooserAction::Save,
                                );
                                fc.add_button("Cancel", gtk::ResponseType::Cancel);
                                fc.add_button("Save",   gtk::ResponseType::Accept);
                                fc.set_current_name(&format!("{}.gif", name_dl2));
                                fc.set_do_overwrite_confirmation(true);
                                if fc.run() == gtk::ResponseType::Accept {
                                    if let Some(dest) = fc.filename() {
                                        if let Err(e) = std::fs::copy(&processed, &dest) {
                                            eprintln!("Download failed: {e}");
                                        }
                                    }
                                }
                                fc.close();
                            });
                            menu.append(&dl_item);

                            // --- Delete Instance ---
                            let del_item  = MenuItem::with_label("Delete Instance");
                            let state_del = state_ev.clone();
                            let ref_del   = ref_ev.clone();
                            let cur_del   = cur_ev.clone();
                            let cp_del    = cp_ev.clone();
                            del_item.connect_activate(move |_| {
                                let mut st = state_del.borrow_mut();
                                if let Some(idx) = st.animas.iter().position(|x| x.instance_db_id == db_id) {
                                    let win = st.animas[idx].window.clone();
                                    st.animas.remove(idx);
                                    let _ = st.db.delete_instance(db_id);
                                    drop(st);
                                    win.close();
                                } else {
                                    let _ = st.db.delete_instance(db_id);
                                    drop(st);
                                }
                                if let Some(EditTarget::Instance(id)) = *cur_del.borrow() {
                                    if id == db_id {
                                        for child in cp_del.children() { cp_del.remove(&child); }
                                    }
                                }
                                if let Some(f) = ref_del.borrow().as_ref() { f(); }
                            });
                            menu.append(&del_item);

                            menu.show_all();
                            menu.popup_at_pointer(Some(event));
                        }
                        _ => {}
                    }
                    gtk::glib::Propagation::Proceed
                });
            }

            // Spawn button
            let btn_box = GtkBox::new(Orientation::Horizontal, 5);
            if is_running {
                // Despawn
                let des_btn   = Button::with_label("Despawn");
                let state_des = state.clone();
                let ref_des   = refresh_inner.clone();
                des_btn.connect_clicked(move |_| {
                    let mut st = state_des.borrow_mut();
                    if let Some(idx) = st.animas.iter().position(|x| x.instance_db_id == db_id) {
                        let win = st.animas[idx].window.clone();
                        st.animas.remove(idx);
                        drop(st);
                        win.close();
                    } else {
                        drop(st);
                    }
                    if let Some(f) = ref_des.borrow().as_ref() { f(); }
                });
                btn_box.pack_start(&des_btn, false, false, 0);
            } else {
                let spawn_btn   = Button::with_label("Spawn");
                let state_spawn = state.clone();
                let ref_spawn   = refresh_inner.clone();
                let ctx_spawn   = ctx_outer.clone();
                spawn_btn.connect_clicked(move |_| {
                    let (inst_data, anim_path_s, anim_name_s) = {
                        let st = state_spawn.borrow();
                        let insts = st.db.get_all_instances().unwrap_or_default();
                        let anims = st.db.get_all_animations().unwrap_or_default();
                        let inst = match insts.into_iter().find(|i| i.id == db_id) {
                            Some(i) => i,
                            None => return,
                        };
                        let anim = match anims.into_iter().find(|a| a.id == inst.animation_id) {
                            Some(a) => a,
                            None => return,
                        };
                        (inst, anim.file_path, anim.name)
                    };
                    let (counter, g_opacity) = {
                        let mut st = state_spawn.borrow_mut();
                        st.instance_counter += 1;
                        (st.instance_counter, st.global_opacity)
                    };
                    let anima = AnimaWindow::new(
                        counter, db_id, anim_name_s, &anim_path_s,
                        inst_data.scale, inst_data.opacity * g_opacity,
                        inst_data.x, inst_data.y,
                        inst_data.mirror, inst_data.flip_v,
                        inst_data.roll, inst_data.pitch, inst_data.yaw,
                        inst_data.temperature, inst_data.contrast,
                        inst_data.brightness, inst_data.saturation, inst_data.hue,
                    );
                    crate::ui::state::register_anima_window(&ctx_spawn, anima);
                    if let Some(f) = ref_spawn.borrow().as_ref() { f(); }
                });
                btn_box.pack_start(&spawn_btn, false, false, 0);
            }
            hbox.pack_end(&btn_box, false, false, 0);

            active_spawns_list_c.add(&row);
        }
        active_spawns_list_c.show_all();
    }));
}
