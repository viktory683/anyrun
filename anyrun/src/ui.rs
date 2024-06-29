use std::{cell::RefCell, fs, io, rc::Rc};

use anyrun_interface::{HandleResult, Match};
use gtk::{
    gdk::{self, Key},
    glib,
    prelude::*,
};
use gtk_layer_shell::LayerShell;
use log::*;

use crate::{
    config::{style_names, Edge, PostRunAction, RelativeNum, RuntimeData},
    types::GMatch,
};

pub fn setup_main_window(
    app: &impl IsA<gtk::Application>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .name(style_names::WINDOW)
        .default_height(500) // TODO: move to config. Yes, let window be static size
        .build();

    setup_layer_shell(&window, runtime_data.clone());
    window.present();
    window
}

fn setup_layer_shell(window: &impl GtkWindowExt, runtime_data: Rc<RefCell<RuntimeData>>) {
    window.init_layer_shell();

    let config = &runtime_data.borrow().config;
    let geometry = runtime_data.borrow().geometry;
    let width = geometry.width().try_into().unwrap();
    let height = geometry.height().try_into().unwrap();

    for (i, edge) in config.edges.clone().into_iter().enumerate() {
        let margin = config
            .margin
            .get(i)
            .unwrap_or(&RelativeNum::default())
            .to_val(match edge {
                Edge::Left | Edge::Right => width,
                Edge::Top | Edge::Bottom => height,
            });
        window.set_anchor(edge.into(), true);
        window.set_margin(edge.into(), margin);
    }

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
        info!("Applying custom CSS from {}", css_path);
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

fn connect_key_press_events<F>(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    handler: F,
) where
    F: Fn(Key) -> glib::Propagation + 'static,
{
    widget.add_controller(event_controller_key.clone());
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| handler(keyval));
}

pub fn connect_window_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    window: Rc<impl GtkWindowExt>,
) {
    connect_key_press_events(widget, event_controller_key, move |keyval| match keyval {
        Key::Escape => {
            window.close();
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
}

pub fn connect_entry_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    window: Rc<impl GtkWindowExt>,
) {
    connect_key_press_events(
        widget.clone(),
        event_controller_key,
        move |keyval| match keyval {
            Key::Escape => {
                window.close();
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
        },
    );
}

pub fn handle_selection_activation<F>(
    row_id: usize,
    window: Rc<impl GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    mut on_refresh: F,
) where
    F: FnMut(bool),
{
    let gmatch = runtime_data
        .borrow()
        .list_store
        .item(row_id.try_into().unwrap())
        .unwrap_or_else(|| panic!("Failed to get list_store item at {} position", row_id))
        .downcast::<GMatch>()
        .expect("Failed to downcast Object to MatchRow");

    let rmatch: Match = gmatch.clone().into();
    let plugin = *runtime_data
        .borrow()
        .plugins
        .get(gmatch.get_plugin_id() as usize)
        .expect("Can't get plugin");

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
    }
}

pub fn configure_main_window(
    window: Rc<impl WidgetExt + GtkWindowExt + NativeExt>,
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

    let scroll_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .focusable(false)
        .build();

    scroll_window.set_child(Some(&*main_list));
    main_vbox.append(&scroll_window);
    window.set_child(Some(&main_vbox));
    entry.grab_focus();
}

pub fn resize_window(
    runtime_data: Rc<RefCell<RuntimeData>>,
    widget: Rc<impl WidgetExt>,
    entry_height: i32,
) {
    fn get_window(widget: Rc<impl WidgetExt>) -> Option<gtk::Window> {
        let parent = widget.parent();
        if let Some(parent) = parent {
            let window = parent.clone().downcast::<gtk::Window>();
            if let Ok(w) = window {
                return Some(w);
            }
            return get_window(Rc::new(parent));
        }
        None
    }

    if let Some(window) = get_window(widget.clone()) {
        let natural_size = widget.preferred_size().1;
        let widget_height = natural_size.height() + entry_height;

        let monitor_height = runtime_data.borrow().geometry.height();
        // TODO move workaround to config to something like max_height or height_adjustment
        window.set_default_height(widget_height.min(monitor_height - 100));
    }
}
