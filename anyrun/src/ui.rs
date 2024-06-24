use std::{cell::RefCell, io, rc::Rc, sync::Once};

use anyrun_interface::{HandleResult, Match, PluginRef as Plugin};
use gtk::{gdk, glib, prelude::*};
use gtk_layer_shell::LayerShell;

use crate::config::{style_names, PostRunAction, RuntimeData};

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

    for edge in &[
        gtk_layer_shell::Edge::Top,
        gtk_layer_shell::Edge::Bottom,
        gtk_layer_shell::Edge::Left,
        gtk_layer_shell::Edge::Right,
    ] {
        window.set_anchor(*edge, true);
    }

    window.set_namespace("anyrun");

    if runtime_data.borrow().config.ignore_exclusive_zones {
        window.set_exclusive_zone(-1);
    }

    window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::Exclusive);
    window.set_layer(runtime_data.borrow().config.layer.to_g_layer());
}

pub fn load_custom_css(runtime_data: Rc<RefCell<RuntimeData>>) {
    let provider = gtk::CssProvider::new();
    let config_dir = &runtime_data.borrow().config_dir;
    let css_path = format!("{}/style.css", config_dir);

    if let Err(why) = provider.load_from_path(&css_path) {
        eprintln!("Failed to load custom CSS: {}", why);
        provider
            .load_from_data(include_bytes!("../res/style.css"))
            .expect("Failed to load embedded CSS data");
    }

    let screen = gdk::Screen::default().expect("Failed to get GDK screen for CSS provider!");
    gtk::StyleContext::add_provider_for_screen(
        &screen,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn connect_key_press_events(window: Rc<impl WidgetExt + GtkWindowExt>) {
    window.connect_key_press_event(move |window, event| {
        use gdk::keys::constants as Key;
        match event.keyval() {
            Key::Escape => {
                window.close();
                glib::Propagation::Stop
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
                eprintln!("Error outputting content to stdout: {}", why);
            }
            window.close();
        }
    };
}

pub fn handle_close_on_click(window: Rc<impl WidgetExt + GtkWindowExt>) {
    window.connect_button_press_event(move |window, event| {
        if event.window() != window.window() {
            return glib::Propagation::Proceed;
        }

        window.close();
        glib::Propagation::Stop
    });
}

pub fn setup_configure_event(
    window: Rc<impl WidgetExt + ContainerExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl WidgetExt>,
    main_list: Rc<impl WidgetExt>,
) {
    let configure_once = Once::new();

    window.connect_configure_event(move |window, event| {
        let runtime_data = runtime_data.clone();
        let entry = entry.clone();
        let main_list = main_list.clone();

        configure_once.call_once(move || {
            configure_main_window(window, event, runtime_data, entry, main_list);
        });

        false
    });
}

fn configure_main_window(
    window: &(impl WidgetExt + ContainerExt),
    event: &gdk::EventConfigure,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl WidgetExt>,
    main_list: Rc<impl WidgetExt>,
) {
    let runtime_data = runtime_data.borrow();

    let width = runtime_data.config.width.to_val(event.size().0);
    let height = runtime_data.config.height.to_val(event.size().1);

    let main_vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .width_request(width)
        .height_request(height)
        .name(style_names::MAIN)
        .build();

    main_vbox.add(&*entry);

    if !runtime_data.error_label.is_empty() {
        main_vbox.add(
            &gtk::Label::builder()
                .label(format!(
                    r#"<span foreground="red">{}</span>"#,
                    runtime_data.error_label
                ))
                .use_markup(true)
                .build(),
        );
    }

    let fixed = gtk::Fixed::builder().build();
    let x = runtime_data.config.x.to_val(event.size().0) - width / 2;
    let y = runtime_data.config.y.to_val(event.size().1) - height / 2;
    fixed.put(&main_vbox, x, y);

    window.add(&fixed);
    window.show_all();

    main_vbox.add(&*main_list);
    main_list.show();
    entry.grab_focus();
}
