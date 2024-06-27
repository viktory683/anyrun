use std::{cell::RefCell, fs, io, rc::Rc};

use anyrun_interface::{HandleResult, Match, PluginRef as Plugin};
use gtk::{gdk, glib, prelude::*};
use gtk_layer_shell::LayerShell;
use log::error;

use crate::config::{style_names, Edge, PostRunAction, RelativeNum, RuntimeData};

pub fn setup_main_window(
    app: &impl IsA<gtk::Application>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .name(style_names::WINDOW)
        .build();

    setup_layer_shell(&window, runtime_data.clone());

    window
}

fn setup_layer_shell(window: &impl GtkWindowExt, runtime_data: Rc<RefCell<RuntimeData>>) {
    window.init_layer_shell();

    let config = &runtime_data.borrow().config;

    let geometry = runtime_data.borrow().geometry;
    let width = geometry.width().try_into().unwrap();
    let height = geometry.height().try_into().unwrap();

    config
        .edges
        .clone()
        .into_iter()
        .enumerate()
        .for_each(|(i, edge)| {
            window.set_anchor(edge.into(), true);

            window.set_margin(
                edge.into(),
                config
                    .margin
                    .get(i)
                    .unwrap_or(&RelativeNum::default())
                    .to_val(match edge {
                        Edge::Left | Edge::Right => width,
                        Edge::Top | Edge::Bottom => height,
                    }),
            );
        });

    window.set_namespace("anyrun");

    if config.ignore_exclusive_zones {
        window.set_exclusive_zone(-1);
    }

    window.set_keyboard_mode(if config.steal_focus {
        gtk_layer_shell::KeyboardMode::Exclusive
    } else {
        gtk_layer_shell::KeyboardMode::OnDemand
    });
    window.set_layer(config.layer.into());
}

pub fn load_custom_css(runtime_data: Rc<RefCell<RuntimeData>>) {
    let config_dir = &runtime_data.borrow().config_dir;
    let css_path = format!("{}/style.css", config_dir);

    if fs::metadata(&css_path).is_ok() {
        let provider = gtk::CssProvider::new();
        provider.load_from_path(css_path);

        let display = gdk::Display::default().expect("Failed to get GDK display for CSS provider!");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn connect_window_key_press_events(
    window: Rc<impl WidgetExt + GtkWindowExt>,
    event_controller_key: gtk::EventControllerKey,
) {
    window.add_controller(event_controller_key.clone());

    let window_clone = window.clone();
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| {
        use gdk::Key;
        match keyval {
            Key::Escape => {
                window_clone.close();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
}

pub fn connect_entry_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    window: Rc<impl GtkWindowExt>,
) {
    widget.add_controller(event_controller_key.clone());

    let window_clone = window.clone();
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| {
        use gdk::Key;
        match keyval {
            Key::Escape => {
                window_clone.close();
                glib::Propagation::Stop
            }
            Key::Down | Key::Up => {
                widget.emit_move_focus(if keyval == Key::Down {
                    gtk::DirectionType::TabForward
                } else {
                    gtk::DirectionType::TabBackward
                });

                glib::Propagation::Proceed
            }
            _ => glib::Propagation::Proceed,
        }
    });
}

pub fn handle_selection_activation<F>(
    row: impl ObjectExt,
    window: Rc<impl GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    mut on_refresh: F,
) where
    F: FnMut(bool),
{
    let rmatch = (unsafe { (*row.data::<Rc<RefCell<Match>>>("match").unwrap().as_ptr()).clone() })
        .borrow()
        .clone();
    let plugin = unsafe { *row.data::<Plugin>("plugin").unwrap().as_ptr() };

    match plugin.handle_selection()(rmatch) {
        HandleResult::Close => window.close(),
        HandleResult::Refresh(exclusive) => {
            runtime_data.borrow_mut().exclusive = if exclusive { Some(plugin) } else { None };
            on_refresh(exclusive);
        }
        HandleResult::Copy(bytes) => {
            runtime_data.borrow_mut().post_run_action = PostRunAction::Copy(bytes.into());
            window.close();
        }
        HandleResult::Stdout(bytes) => {
            if let Err(why) = io::Write::write_all(&mut io::stdout().lock(), &bytes) {
                error!("Error outputting content to stdout: {}", why);
            }
            window.close();
        }
    };
}

pub fn configure_main_window(
    window: Rc<impl WidgetExt + GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl WidgetExt>,
    main_list: Rc<impl WidgetExt>,
) {
    let runtime_data = runtime_data.borrow();

    let width = runtime_data
        .config
        .width
        .to_val(window.width().try_into().unwrap());
    let height = runtime_data
        .config
        .height
        .to_val(window.height().try_into().unwrap());

    let main_vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .width_request(width)
        .height_request(height)
        .name(style_names::MAIN)
        .build();

    main_vbox.append(&*entry);

    if !runtime_data.error_label.is_empty() {
        main_vbox.append(
            &gtk::Label::builder()
                .label(format!(
                    r#"<span foreground="red">{}</span>"#,
                    runtime_data.error_label
                ))
                .use_markup(true)
                .build(),
        );
    }

    // TODO window needs to be resized on `refresh_matches` if it fits `max_content_height`
    let scroll_window = gtk::ScrolledWindow::builder()
        // .min_content_width(200)
        .min_content_height(400)
        // .max_content_height(800)
        .vexpand(true)
        .hexpand(true)
        .focusable(false)
        .build();

    scroll_window.set_child(Some(&*main_list));
    main_vbox.append(&scroll_window);
    window.set_child(Some(&main_vbox));
    window.present();
    entry.grab_focus();
}
