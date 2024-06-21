use std::{cell::RefCell, env, io, mem, path::PathBuf, rc::Rc, sync::Once};

use anyrun_interface::{HandleResult, Match, PluginInfo};
use gtk::{gdk, glib, prelude::*};
use gtk_layer_shell::LayerShell;

use crate::{
    plugin::{load_plugin, refresh_matches},
    types::{style_names, PluginView, PostRunAction, RuntimeData, DEFAULT_CONFIG_DIR},
};

pub fn setup_main_window(
    app: &gtk::Application,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .name(style_names::WINDOW)
        .build();

    window.init_layer_shell();

    [
        gtk_layer_shell::Edge::Top,
        gtk_layer_shell::Edge::Bottom,
        gtk_layer_shell::Edge::Left,
        gtk_layer_shell::Edge::Right,
    ]
    .iter()
    .for_each(|edge| window.set_anchor(*edge, true));

    window.set_namespace("anyrun");

    if runtime_data.borrow().config.ignore_exclusive_zones {
        window.set_exclusive_zone(-1);
    }

    window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::Exclusive);

    window.set_layer(runtime_data.borrow().config.layer.to_g_layer());

    window
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

pub fn load_plugins(
    runtime_data: Rc<RefCell<RuntimeData>>,
    main_list: &gtk::ListBox,
) -> Vec<PluginView> {
    // TODO maybe we should not add some default paths to parsed ANYRUN_PLUGINS
    // like defaults used only if ANYRUN_PLUGINS is None
    let mut plugins_paths: Vec<PathBuf> = match env::var_os("ANYRUN_PLUGINS") {
        Some(paths) => env::split_paths(&paths).collect(),
        None => vec![],
    };

    plugins_paths.append(
        &mut [
            runtime_data.borrow().config_dir.clone(),
            DEFAULT_CONFIG_DIR.to_string(),
        ]
        .iter()
        .map(|plugins_path| PathBuf::from(format!("{}/plugins", plugins_path)))
        .collect(),
    );

    runtime_data
        .borrow()
        .config
        .plugins
        .iter()
        .map(|plugin_path| {
            let plugin = load_plugin(plugin_path, &plugins_paths);
            plugin.init()(runtime_data.borrow().config_dir.clone().into());

            let plugin_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(10)
                .name(style_names::PLUGIN)
                .build();

            if !runtime_data.borrow().config.hide_plugin_info {
                plugin_box.add(&create_info_box(
                    &plugin.info()(),
                    runtime_data.borrow().config.hide_icons,
                ));
                plugin_box.add(
                    &gtk::Separator::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .name(style_names::PLUGIN)
                        .build(),
                );
            }

            let list = gtk::ListBox::builder()
                .name(style_names::PLUGIN)
                .hexpand(true)
                .build();
            plugin_box.add(&list);

            let row = gtk::ListBoxRow::builder()
                .activatable(false)
                .selectable(false)
                .can_focus(false)
                .name(style_names::PLUGIN)
                .build();
            row.add(&plugin_box);
            main_list.add(&row);

            PluginView { plugin, row, list }
        })
        .collect::<Vec<PluginView>>()
}

fn create_info_box(info: &PluginInfo, hide_icons: bool) -> gtk::Box {
    let info_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .name(style_names::PLUGIN)
        .width_request(200)
        .height_request(32)
        .expand(false)
        .spacing(10)
        .build();
    if !hide_icons {
        info_box.add(
            &gtk::Image::builder()
                .icon_name(&info.icon)
                .name(style_names::PLUGIN)
                .pixel_size(32)
                .halign(gtk::Align::Start)
                .valign(gtk::Align::Start)
                .build(),
        );
    }
    info_box.add(
        &gtk::Label::builder()
            .label(info.name.to_string())
            .name(style_names::PLUGIN)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    // This is so that we can align the plugin name with the icon. GTK would not let it be properly aligned otherwise.
    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .name(style_names::PLUGIN)
        .build();
    main_box.add(&info_box);
    main_box.add(
        &gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .name(style_names::PLUGIN)
            .build(),
    );
    main_box
}

pub fn connect_selection_events(runtime_data: Rc<RefCell<RuntimeData>>) {
    for plugin_view in runtime_data.borrow().plugins.iter() {
        let plugins_clone = runtime_data.borrow().plugins.clone();
        plugin_view.list.connect_row_selected(move |list, row| {
            if row.is_some() {
                let combined_matches = plugins_clone
                    .iter()
                    .flat_map(|view| {
                        view.list.children().into_iter().map(|child| {
                            (
                                child.dynamic_cast::<gtk::ListBoxRow>().unwrap(),
                                view.list.clone(),
                            )
                        })
                    })
                    .collect::<Vec<(gtk::ListBoxRow, gtk::ListBox)>>();

                for (_, _list) in combined_matches {
                    if _list != *list {
                        _list.select_row(None::<&gtk::ListBoxRow>);
                    }
                }
            }
        });
    }
}

pub fn setup_entry(runtime_data: Rc<RefCell<RuntimeData>>) -> gtk::SearchEntry {
    let entry = gtk::SearchEntry::builder()
        .hexpand(true)
        .name(style_names::ENTRY)
        .build();
    if runtime_data.borrow().config.show_results_immediately {
        refresh_matches(String::new(), runtime_data.clone());
    }
    entry
}

pub fn connect_key_press_events(
    window: &gtk::ApplicationWindow,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl EntryExt>,
) {
    window.connect_key_press_event(move |window, event| {
        use gdk::keys::constants as Key;
        match event.keyval() {
            Key::Escape => {
                window.close();
                glib::Propagation::Stop
            }
            Key::Down | Key::Tab | Key::Up => {
                handle_selection_navigation(event, runtime_data.clone());
                glib::Propagation::Stop
            }
            Key::Return => {
                handle_selection_activation(window, runtime_data.clone(), entry.clone());
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
}

fn handle_selection_navigation(event: &gdk::EventKey, runtime_data: Rc<RefCell<RuntimeData>>) {
    use gdk::keys::constants as Key;

    let combined_matches = runtime_data
        .borrow()
        .plugins
        .iter()
        .flat_map(|view| {
            view.list.children().into_iter().map(|child| {
                (
                    child.dynamic_cast::<gtk::ListBoxRow>().unwrap(),
                    view.list.clone(),
                )
            })
        })
        .collect::<Vec<(gtk::ListBoxRow, gtk::ListBox)>>();

    let (selected_match, selected_list) = match runtime_data
        .borrow()
        .plugins
        .iter()
        .find_map(|view| view.list.selected_row().map(|row| (row, view.list.clone())))
    {
        Some(selected) => selected,
        None => {
            if !combined_matches.is_empty() {
                match event.keyval() {
                    Key::Down | Key::Tab => combined_matches[0]
                        .1
                        .select_row(Some(&combined_matches[0].0)),
                    Key::Up => combined_matches
                        .last()
                        .unwrap()
                        .1
                        .select_row(Some(&combined_matches.last().unwrap().0)),
                    _ => unreachable!(),
                }
            }
            return;
        }
    };

    selected_list.select_row(None::<&gtk::ListBoxRow>);

    let index = combined_matches
        .iter()
        .position(|(row, _list)| *row == selected_match)
        .unwrap();
    match event.keyval() {
        Key::Down | Key::Tab => {
            if index + 1 != combined_matches.len() {
                combined_matches[index + 1]
                    .1
                    .select_row(Some(&combined_matches[index + 1].0));
            } else {
                combined_matches[0]
                    .1
                    .select_row(Some(&combined_matches[0].0));
            }
        }
        Key::Up => {
            if index != 0 {
                combined_matches[index - 1]
                    .1
                    .select_row(Some(&combined_matches[index - 1].0));
            } else {
                combined_matches
                    .last()
                    .unwrap()
                    .1
                    .select_row(Some(&combined_matches.last().unwrap().0));
            }
        }
        _ => unreachable!(),
    }
}

fn handle_selection_activation(
    window: &gtk::ApplicationWindow,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl EntryExt>,
) {
    let mut _runtime_data_clone = runtime_data.borrow_mut();

    let (selected_match, plugin_view) = match _runtime_data_clone
        .plugins
        .iter()
        .find_map(|view| view.list.selected_row().map(|row| (row, view)))
    {
        Some(selected) => selected,
        None => return,
    };

    match plugin_view.plugin.handle_selection()(unsafe {
        (*selected_match.data::<Match>("match").unwrap().as_ptr()).clone()
    }) {
        HandleResult::Close => {
            window.close();
        }
        HandleResult::Refresh(exclusive) => {
            if exclusive {
                _runtime_data_clone.exclusive = Some(plugin_view.clone());
            } else {
                _runtime_data_clone.exclusive = None;
            }
            mem::drop(_runtime_data_clone);
            refresh_matches(entry.text().to_string(), runtime_data.clone());
        }
        HandleResult::Copy(bytes) => {
            _runtime_data_clone.post_run_action = PostRunAction::Copy(bytes.into());
            window.close();
        }
        HandleResult::Stdout(bytes) => {
            if let Err(why) = io::Write::write_all(&mut io::stdout().lock(), &bytes) {
                eprintln!("Error outputting content to stdout: {}", why);
            }
            window.close();
        }
    }
}

pub fn handle_close_on_click(window: &gtk::ApplicationWindow) {
    window.connect_button_press_event(move |window, event| {
        if event.window() != window.window() {
            return glib::Propagation::Proceed;
        }

        window.close();
        glib::Propagation::Stop
    });
}

pub fn setup_configure_event(
    window: &gtk::ApplicationWindow,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl WidgetExt>,
    main_list: Rc<gtk::ListBox>,
) {
    let configure_once = Once::new();

    window.connect_configure_event(move |window, event| {
        let runtime_data = runtime_data.clone();
        let entry = entry.clone();
        let main_list = main_list.clone();

        configure_once.call_once(move || {
            let runtime_data = runtime_data.borrow();

            let width = runtime_data.config.width.to_val(event.size().0);
            let x = runtime_data.config.x.to_val(event.size().0) - width / 2;
            let height = runtime_data.config.height.to_val(event.size().1);
            let y = runtime_data.config.y.to_val(event.size().1) - height / 2;

            let fixed = gtk::Fixed::builder().build();
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

            fixed.put(&main_vbox, x, y);
            window.add(&fixed);
            window.show_all();

            main_vbox.add(&*main_list);
            main_list.show();
            entry.grab_focus();
        });

        false
    });
}
