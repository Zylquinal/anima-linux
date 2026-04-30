use crate::ui::state::{AppContext, EditTarget};
use crate::anima::AnimaWindow;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, Image, Label, Orientation, Scale,
          ScrolledWindow, Adjustment, Spinner};
use std::rc::Rc;
use gdk_pixbuf::PixbufAnimation;
use std::cell::RefCell;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

pub fn build(ctx: &AppContext, right_scroll: &ScrolledWindow, control_panel: &GtkBox) {
    let control_panel_c = control_panel.clone();
    let right_scroll_c = right_scroll.clone();
    let state = ctx.state.clone();
    let refresh_active_spawns = ctx.refresh_active_spawns.clone();
    let current_edit_target = ctx.current_edit_target.clone();

    let ctx_c = ctx.clone();
    *ctx.update_control_panel.borrow_mut() = Some(Rc::new(move |target| {
        println!("Opening control panel for target...");
        *current_edit_target.borrow_mut() = Some(target.clone());
        for child in control_panel_c.children() { control_panel_c.remove(&child); }

        let s = state.borrow();
        let (name, file_path, config) = match target.clone() {
            EditTarget::Library(id) => {
                let anims = s.db.get_all_animations().unwrap_or_default();
                let a = anims.into_iter().find(|x| x.id == id).expect("Anim not found");
                (a.name, a.file_path, crate::db::InstanceConfig {
                    id: -1, animation_id: id, scale: 1.0, opacity: 1.0, x: 0, y: 0,
                    auto_spawn: false, mirror: false, flip_v: false, roll: 0.0, pitch: 0.0,
                    yaw: 0.0, temperature: 0.0, contrast: 0.0, brightness: 0.0,
                    saturation: 0.0, hue: 0.0
                })
            }
            EditTarget::Instance(id) => {
                let insts = s.db.get_all_instances().unwrap_or_default();
                let i = insts.into_iter().find(|x| x.id == id).expect("Instance not found");
                let anims = s.db.get_all_animations().unwrap_or_default();
                let a = anims.into_iter().find(|x| x.id == i.animation_id).expect("Anim not found");
                (a.name, a.file_path, i)
            }
        };
        drop(s);

        let title = Label::new(None);
        let target_str = match target { EditTarget::Library(_) => "Library Default", EditTarget::Instance(_) => "Active Instance" };
        title.set_markup(&format!("<span size='large' weight='bold'>{} - {}</span>", target_str, name));
        title.set_xalign(0.0);
        control_panel_c.add(&title);

        // Preview area
        let preview_box = GtkBox::new(Orientation::Vertical, 4);

        let preview_img = Image::new();
        preview_img.set_size_request(-1, -1);

        // Spinner shown while the preview is being computed on a background thread.
        let preview_spinner = Spinner::new();
        preview_spinner.set_opacity(0.0);

        let info_label = Label::new(None);
        info_label.set_no_show_all(true);
        info_label.set_line_wrap(true);
        info_label.set_max_width_chars(40);
        info_label.set_xalign(0.0);

        preview_box.add(&preview_img);
        preview_box.add(&preview_spinner);
        preview_box.add(&info_label);
        control_panel_c.add(&preview_box);

        let preview_path_orig = file_path.clone();

        // Read natural dimensions once (cheap, for fit-scale computation).
        const PREVIEW_MAX_PX: u32 = 350;
        let (nat_w, nat_h): (u32, u32) = PixbufAnimation::from_file(&preview_path_orig)
            .ok()
            .and_then(|a| a.static_image().map(|p| (p.width() as u32, p.height() as u32)))
            .unwrap_or((0, 0));

        // Generation counter – incremented on every preview request.
        // The poll callback compares its captured generation against this; if they
        // differ the result is discarded (newer request already in flight).
        let preview_gen: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

        // Tracks the active poll SourceId so we can cancel it when a new preview
        // request supersedes the previous one before it has finished.
        let poll_source: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));

        // update_preview: non-blocking. Computation runs on a background thread;
        // result is delivered via std::sync::mpsc polled with timeout_add_local.
        let update_preview = {
            let preview_img   = preview_img.clone();
            let spinner       = preview_spinner.clone();
            let info_label    = info_label.clone();
            let gen_ref       = preview_gen.clone();
            let poll_src_ref  = poll_source.clone();
            let path_for_prev = preview_path_orig.clone();
            move |scale: f64, mirror: bool, flip_v: bool,
                  roll: f64, pitch: f64, yaw: f64,
                  temp: f64, contrast: f64, bright: f64, sat: f64, hue: f64| {
                // Increment generation: the poll closure captures this value and
                // will discard the result if a newer request arrived in the meantime.
                let this_gen = gen_ref.fetch_add(1, Ordering::SeqCst) + 1;

                // Cancel any still-running poll from a previous request.
                if let Some(id) = poll_src_ref.borrow_mut().take() { id.remove(); }

                // Compute fit-scale and update the info label (fast, on main thread).
                let max_dim = nat_w.max(nat_h).max(1) as f64;
                let fit_scale = (PREVIEW_MAX_PX as f64 / max_dim).min(scale);
                let preview_scale = scale.min(fit_scale);
                let is_too_big = fit_scale < scale - 0.005;
                let is_above_1 = scale > 1.005;

                if is_too_big || is_above_1 {
                    let msg = if is_too_big {
                        format!("Preview fit to {}px  ·  Actual: {}×{}px",
                            PREVIEW_MAX_PX,
                            (nat_w as f64 * scale) as u32,
                            (nat_h as f64 * scale) as u32)
                    } else {
                        format!("Preview limited to 1.0×  ·  Actual: {}×{}px",
                            (nat_w as f64 * scale) as u32,
                            (nat_h as f64 * scale) as u32)
                    };
                    info_label.set_markup(&format!("<span size='small' color='gray'>{msg}</span>"));
                    info_label.show();
                } else {
                    info_label.hide();
                }

                let unchanged = !is_too_big
                    && (preview_scale - 1.0).abs() < 0.01
                    && !mirror && !flip_v
                    && roll.abs() < 0.01 && pitch.abs() < 0.01 && yaw.abs() < 0.01
                    && temp.abs() < 0.01 && contrast.abs() < 0.01 && bright.abs() < 0.01
                    && sat.abs() < 0.01 && hue.abs() < 0.01;

                // Show spinner, hide old image while background work runs.
                preview_img.set_opacity(0.0);
                spinner.start();
                spinner.set_opacity(1.0);

                // Background thread
                let (tx, rx) = std::sync::mpsc::channel::<Option<Vec<u8>>>();
                let path = path_for_prev.clone();
                std::thread::spawn(move || {
                    let data: Option<Vec<u8>> = if unchanged {
                        std::fs::read(&path).ok()
                    } else {
                        Some(crate::anima_resize::process_gif_in_memory(
                            &path, preview_scale, mirror, flip_v,
                            roll, pitch, yaw, temp, contrast, bright, sat, hue,
                        ))
                    };
                    tx.send(data).ok();
                });

                // Poll result every ~16 ms on the main thread.
                let preview_img_p = preview_img.clone();
                let spinner_p     = spinner.clone();
                let gen_check     = gen_ref.clone();
                let poll_src_p    = poll_src_ref.clone();
                let rx_cell       = RefCell::new(rx);
                let id = gtk::glib::timeout_add_local(
                    std::time::Duration::from_millis(16),
                    move || {
                        use std::sync::mpsc::TryRecvError;
                        match rx_cell.borrow_mut().try_recv() {
                            Ok(data) => {
                                // Only apply result if still the current generation.
                                if gen_check.load(Ordering::SeqCst) == this_gen {
                                    spinner_p.stop();
                                    spinner_p.set_opacity(0.0);
                                    if let Some(bytes) = data {
                                        let loader = gdk_pixbuf::PixbufLoader::with_type("gif").unwrap();
                                        loader.write(&bytes).ok();
                                        loader.close().ok();
                                        if let Some(anim) = loader.animation() {
                                            preview_img_p.set_from_animation(&anim);
                                            preview_img_p.set_opacity(1.0);
                                        }
                                    }
                                }
                                *poll_src_p.borrow_mut() = None;
                                gtk::glib::ControlFlow::Break
                            }
                            Err(TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                            Err(_) => {
                                *poll_src_p.borrow_mut() = None;
                                gtk::glib::ControlFlow::Break
                            }
                        }
                    },
                );
                *poll_src_ref.borrow_mut() = Some(id);
            }
        };


        // Sliders
        let grid = gtk::Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(20);
        grid.set_margin_top(10);

        let create_slider = |label: &str, min: f64, max: f64, current: f64, step: f64| {
            let l = Label::new(Some(label));
            l.set_xalign(0.0);
            let adj = Adjustment::new(current, min, max, step, step * 10.0, 0.0);
            let sc = Scale::new(Orientation::Horizontal, Some(&adj));
            sc.set_hexpand(true);
            let reset = Button::with_label("↺");
            let adj_c = adj.clone();
            let def_val = if label == "Scale" { 1.0 } else { 0.0 };
            reset.connect_clicked(move |_| adj_c.set_value(def_val));
            (l, sc, adj, reset)
        };

        let (sl_l, sl_s, sl_adj, sl_r) = create_slider("Scale", 0.1, 5.0, config.scale, 0.1);
        grid.attach(&sl_l, 0, 0, 1, 1); grid.attach(&sl_s, 1, 0, 1, 1); grid.attach(&sl_r, 2, 0, 1, 1);

        let (op_l, op_s, op_adj, op_r) = create_slider("Opacity", 0.1, 1.0, config.opacity, 0.05);
        grid.attach(&op_l, 0, 1, 1, 1); grid.attach(&op_s, 1, 1, 1, 1); grid.attach(&op_r, 2, 1, 1, 1);

        let mirror_box = GtkBox::new(Orientation::Horizontal, 5);
        let mirror_check = CheckButton::with_label("Flip H");
        mirror_check.set_active(config.mirror);
        let flip_v_check = CheckButton::with_label("Flip V");
        flip_v_check.set_active(config.flip_v);
        mirror_box.add(&mirror_check);
        mirror_box.add(&flip_v_check);
        grid.attach(&mirror_box, 1, 2, 1, 1);

        let (t_l, t_s, t_adj, t_r) = create_slider("Temp", -100.0, 100.0, config.temperature, 1.0);
        grid.attach(&t_l, 0, 3, 1, 1); grid.attach(&t_s, 1, 3, 1, 1); grid.attach(&t_r, 2, 3, 1, 1);

        let (c_l, c_s, c_adj, c_r) = create_slider("Contrast", -100.0, 100.0, config.contrast, 1.0);
        grid.attach(&c_l, 0, 4, 1, 1); grid.attach(&c_s, 1, 4, 1, 1); grid.attach(&c_r, 2, 4, 1, 1);

        let (b_l, b_s, b_adj, b_r) = create_slider("Brightness", -100.0, 100.0, config.brightness, 1.0);
        grid.attach(&b_l, 0, 5, 1, 1); grid.attach(&b_s, 1, 5, 1, 1); grid.attach(&b_r, 2, 5, 1, 1);

        let (s_l, s_s, s_adj, s_r) = create_slider("Saturation", -100.0, 100.0, config.saturation, 1.0);
        grid.attach(&s_l, 0, 6, 1, 1); grid.attach(&s_s, 1, 6, 1, 1); grid.attach(&s_r, 2, 6, 1, 1);

        let (h_l, h_s, h_adj, h_r) = create_slider("Hue", -180.0, 180.0, config.hue, 1.0);
        grid.attach(&h_l, 0, 7, 1, 1); grid.attach(&h_s, 1, 7, 1, 1); grid.attach(&h_r, 2, 7, 1, 1);

        let (r_l, r_s, r_adj, r_r) = create_slider("Roll (Z)", 0.0, 360.0, config.roll.rem_euclid(360.0), 5.0);
        for deg in [90.0_f64, 180.0, 270.0, 360.0] {
            r_s.add_mark(deg, gtk::PositionType::Bottom, None);
        }
        grid.attach(&r_l, 0, 8, 1, 1); grid.attach(&r_s, 1, 8, 1, 1); grid.attach(&r_r, 2, 8, 1, 1);

        let (p_l, p_s, p_adj, p_r) = create_slider("Pitch (X)", -90.0, 90.0, config.pitch, 5.0);
        for deg in [-90.0_f64, -45.0, 0.0, 45.0, 90.0] {
            p_s.add_mark(deg, gtk::PositionType::Bottom, None);
        }
        grid.attach(&p_l, 0, 9, 1, 1); grid.attach(&p_s, 1, 9, 1, 1); grid.attach(&p_r, 2, 9, 1, 1);

        let (y_l, y_s, y_adj, y_r) = create_slider("Yaw (Y)", -90.0, 90.0, config.yaw, 5.0);
        for deg in [-90.0_f64, -45.0, 0.0, 45.0, 90.0] {
            y_s.add_mark(deg, gtk::PositionType::Bottom, None);
        }
        grid.attach(&y_l, 0, 10, 1, 1); grid.attach(&y_s, 1, 10, 1, 1); grid.attach(&y_r, 2, 10, 1, 1);

        let auto_spawn_check = CheckButton::with_label("Auto-spawn");
        auto_spawn_check.set_active(config.auto_spawn);
        grid.attach(&auto_spawn_check, 1, 11, 1, 1);

        control_panel_c.add(&grid);

        // Live-update debounce
        let live_update_enabled = state.borrow().db.get_live_update_enabled().unwrap_or(true);
        let live_update_delay   = state.borrow().db.get_live_update_delay().unwrap_or(300);

        let debounce_id = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
        let live_update = {
            let up_p = update_preview.clone();
            let sl = sl_adj.clone(); let mir = mirror_check.clone(); let flip_v = flip_v_check.clone();
            let ro = r_adj.clone(); let pi = p_adj.clone(); let ya = y_adj.clone();
            let t = t_adj.clone(); let c = c_adj.clone(); let b = b_adj.clone();
            let s = s_adj.clone(); let h = h_adj.clone();
            let db_id_ref = debounce_id.clone();
            move || {
                if !live_update_enabled { return; }
                if let Some(id) = db_id_ref.borrow_mut().take() { id.remove(); }
                let up_p = up_p.clone();
                let sl_v = sl.value(); let mir_v = mir.is_active(); let fv_v = flip_v.is_active();
                let ro_v = ro.value(); let pi_v = pi.value(); let ya_v = ya.value();
                let t_v = t.value(); let c_v = c.value(); let b_v = b.value();
                let s_v = s.value(); let h_v = h.value();
                let db_id_inner = db_id_ref.clone();
                let id = gtk::glib::timeout_add_local(
                    std::time::Duration::from_millis(live_update_delay),
                    move || {
                        up_p(sl_v, mir_v, fv_v, ro_v, pi_v, ya_v, t_v, c_v, b_v, s_v, h_v);
                        *db_id_inner.borrow_mut() = None;
                        gtk::glib::ControlFlow::Break
                    },
                );
                *db_id_ref.borrow_mut() = Some(id);
            }
        };

        sl_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        mirror_check.connect_toggled({let lu = live_update.clone(); move |_| lu()});
        flip_v_check.connect_toggled({let lu = live_update.clone(); move |_| lu()});
        r_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        p_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        y_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        t_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        c_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        b_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        s_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});
        h_adj.connect_value_changed({let lu = live_update.clone(); move |_| lu()});

        // Initial preview render.
        update_preview(
            config.scale, config.mirror, config.flip_v,
            config.roll, config.pitch, config.yaw,
            config.temperature, config.contrast, config.brightness,
            config.saturation, config.hue,
        );

        // Action buttons
        let action_box = GtkBox::new(Orientation::Horizontal, 10);
        let apply_btn  = Button::with_label("Apply Changes");
        let spawn_btn  = Button::with_label("Spawn with Settings");

        // Spinner shown while the GIF is being pre-computed
        let action_spinner = Spinner::new();
        action_spinner.set_opacity(0.0);

        match target.clone() {
            EditTarget::Library(_) => {
                action_box.add(&spawn_btn);
            }
            EditTarget::Instance(_) => {
                action_box.add(&apply_btn);
            }
        }
        action_box.add(&action_spinner);
        control_panel_c.add(&action_box);

        let state_c   = state.clone();
        let refresh_a = refresh_active_spawns.clone();
        let name_c    = name.clone();
        let path_c    = file_path.clone();

        let effective_target: Rc<RefCell<EditTarget>> = Rc::new(RefCell::new(target.clone()));
        let title_lbl = title.clone();

        let ctx_c2 = ctx_c.clone();
        let on_apply = {
            let effective_target = effective_target.clone();
            let apply_btn_c      = apply_btn.clone();
            let spawn_btn_c      = spawn_btn.clone();
            let action_spinner_c = action_spinner.clone();
            move || {
                let mirror  = mirror_check.is_active();
                let flip_v  = flip_v_check.is_active();
                let roll    = r_adj.value();
                let pitch   = p_adj.value();
                let yaw     = y_adj.value();
                let scale   = sl_adj.value();
                let opacity = op_adj.value();
                let temp    = t_adj.value();
                let cont    = c_adj.value();
                let bright  = b_adj.value();
                let sat     = s_adj.value();
                let hue     = h_adj.value();
                let auto    = auto_spawn_check.is_active();

                let mut st = state_c.borrow_mut();

                let current_target = effective_target.borrow().clone();
                let db_id = match current_target {
                    EditTarget::Instance(id) => id,
                    EditTarget::Library(anim_id) => {
                        let id = st.db.insert_instance(anim_id, scale, opacity, 0, 0, auto).unwrap();
                        *effective_target.borrow_mut() = EditTarget::Instance(id);
                        title_lbl.set_markup(&format!(
                            "<span size='large' weight='bold'>Active Instance - {}</span>",
                            name_c
                        ));
                        id
                    }
                };

                let _ = st.db.update_instance_scale(db_id, scale);
                let _ = st.db.update_instance_auto_spawn(db_id, auto);
                let _ = st.db.update_instance_mirror(db_id, mirror);
                let _ = st.db.update_instance_rotation(db_id, flip_v, roll, pitch, yaw);
                let _ = st.db.update_instance_editing(db_id, temp, cont, bright, sat, hue);
                let _ = st.db.update_instance_opacity(db_id, opacity);

                // Capture current position so the re-spawned mascot stays in place.
                let mut spawn_x = 0i32;
                let mut spawn_y = 0i32;
                if let Some(idx) = st.animas.iter().position(|a| a.instance_db_id == db_id) {
                    let (cx, cy) = st.animas[idx].position();
                    spawn_x = cx;
                    spawn_y = cy;
                    let win = st.animas[idx].window.clone();
                    st.animas.remove(idx);
                    drop(st);
                    win.close();
                    st = state_c.borrow_mut();
                }

                let _ = st.db.update_instance_position(db_id, spawn_x, spawn_y);

                // Pre-compute counter and global opacity on the main thread
                // (state is not Send so we can't access it from the background thread).
                let (counter, g_opacity) = {
                    st.instance_counter += 1;
                    (st.instance_counter, st.global_opacity)
                };
                drop(st);

                // Disable buttons and show spinner while the GIF cache is warmed.
                apply_btn_c.set_sensitive(false);
                spawn_btn_c.set_sensitive(false);
                action_spinner_c.start();
                action_spinner_c.set_opacity(1.0);

                // Background thread: pre-compute (warm) the processed GIF cache.
                // AnimaWindow::new() will then return immediately from cache.
                let (tx, rx) = std::sync::mpsc::channel::<()>();
                {
                    let path = path_c.clone();
                    std::thread::spawn(move || {
                        crate::anima_resize::ensure_processed_gif(
                            &path, scale, mirror, flip_v,
                            roll, pitch, yaw, temp, cont, bright, sat, hue,
                        );
                        tx.send(()).ok();
                    });
                }

                // Poll the channel every ~16 ms on the main thread.
                let ctx_poll      = ctx_c2.clone();
                let refresh_poll  = refresh_a.clone();
                let name_poll     = name_c.clone();
                let path_poll     = path_c.clone();
                let apply_btn_p   = apply_btn_c.clone();
                let spawn_btn_p   = spawn_btn_c.clone();
                let spinner_p     = action_spinner_c.clone();
                let rx_cell       = RefCell::new(rx);
                gtk::glib::timeout_add_local(
                    std::time::Duration::from_millis(16),
                    move || {
                        use std::sync::mpsc::TryRecvError;
                        match rx_cell.borrow_mut().try_recv() {
                            Ok(()) => {
                                // Cache is ready – create the window on the main thread.
                                let anima = AnimaWindow::new(
                                    counter, db_id, name_poll.clone(), &path_poll,
                                    scale, opacity * g_opacity, spawn_x, spawn_y,
                                    mirror, flip_v, roll, pitch, yaw,
                                    temp, cont, bright, sat, hue,
                                );
                                crate::ui::state::register_anima_window(&ctx_poll, anima);
                                if let Some(f) = refresh_poll.borrow().as_ref() { f(); }

                                spinner_p.stop();
                                spinner_p.set_opacity(0.0);
                                apply_btn_p.set_sensitive(true);
                                spawn_btn_p.set_sensitive(true);
                                gtk::glib::ControlFlow::Break
                            }
                            Err(TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                            Err(_) => {
                                spinner_p.stop();
                                spinner_p.set_opacity(0.0);
                                apply_btn_p.set_sensitive(true);
                                spawn_btn_p.set_sensitive(true);
                                gtk::glib::ControlFlow::Break
                            }
                        }
                    },
                );
            }
        };

        let on_apply_rc = Rc::new(on_apply);
        apply_btn.connect_clicked({let oa = on_apply_rc.clone(); move |_| oa()});
        spawn_btn.connect_clicked(move |_| on_apply_rc());

        control_panel_c.show_all();
        right_scroll_c.show_all();
    }));
}
