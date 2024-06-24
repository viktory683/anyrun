use std::{cell::RefCell, env, path::PathBuf, rc::Rc, time::Duration};

use abi_stable::std_types::ROption;
use anyrun_interface::{Match, PluginRef as Plugin, PollResult};

use crate::config::{style_names, RuntimeData, DEFAULT_CONFIG_DIR};

use gtk::{gdk_pixbuf, glib, prelude::*};

pub fn build_label(name: &str, use_markup: bool, label: &str, sensitive: bool) -> gtk::Label {
    gtk::Label::builder()
        .name(name)
        .wrap(true)
        .xalign(0.0)
        .use_markup(use_markup)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .label(label)
        .sensitive(sensitive)
        .build()
}

pub fn build_image(icon: &str) -> gtk::Image {
    let mut match_image = gtk::Image::builder()
        .name(style_names::MATCH)
        .pixel_size(32);

    let path = PathBuf::from(icon);

    if path.is_absolute() {
        match gdk_pixbuf::Pixbuf::from_file_at_size(icon, 32, 32) {
            Ok(pixbuf) => match_image = match_image.pixbuf(&pixbuf),
            Err(why) => {
                eprintln!("Failed to load icon file: {}", why);
                match_image = match_image.icon_name("image-missing");
            }
        }
    } else {
        match_image = match_image.icon_name(icon);
    }
    match_image.build()
}

pub fn handle_matches(
    main_list: Rc<impl ContainerExt>,
    matches: &[Match],
    plugin: Plugin,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    let mut first_plugin_match = true;

    matches.iter().for_each(|rmatch| {
        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        if !runtime_data.borrow().config.hide_plugin_info {
            // TODO style_name
            // TODO plugin icon?
            let plugin_name = if first_plugin_match {
                &plugin.info()().name
            } else {
                ""
            };
            let plugin_label = gtk::Label::builder()
                .label(plugin_name)
                .width_request(200)
                .sensitive(false)
                .build();
            hbox.add(&plugin_label);

            first_plugin_match = false;
        }

        let match_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .build();

        if !runtime_data.borrow().config.hide_icons {
            if let ROption::RSome(icon) = &rmatch.icon {
                let image = build_image(icon);
                match_box.add(&image);
            }
        }

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        let title = build_label(
            style_names::MATCH_TITLE,
            rmatch.use_pango,
            &rmatch.title,
            true,
        );
        vbox.add(&title);

        if let ROption::RSome(desc) = &rmatch.description {
            let desc = build_label(style_names::MATCH_DESC, rmatch.use_pango, desc, false);
            vbox.add(&desc);
        }

        match_box.add(&vbox);
        hbox.add(&match_box);

        let row = gtk::ListBoxRow::builder().height_request(32).build();
        unsafe {
            row.set_data("match", Rc::new(RefCell::new(rmatch.clone())));
            row.set_data("plugin", plugin);
        }
        row.set_child(Some(&hbox));
        row.show_all();

        main_list.add(&row);
    });
}

/// Loads a plugin from the specified path or from the provided directories if the path is not absolute.
///
/// # Arguments
///
/// * `plugin_path` - A relative or absolute path to the plugin file (e.g., "libapplication.so").
/// * `plugins_paths` - A slice of directory paths where plugin files may be located.
///
/// # Returns
///
/// * `Plugin` - A reference to the loaded plugin.
///
/// # Panics
///
/// This function will panic if:
/// * The provided `plugin_path` does not exist in any of the `plugin_paths` directories.
/// * The plugin fails to load or initialize.
///
/// # Example
///
/// ```
/// let plugin_path = PathBuf::from("libapplication.so");
/// let plugin_dirs = vec![PathBuf::from("/usr/local/lib/plugins"), PathBuf::from("/opt/plugins")];
/// let plugin = load_plugin(&plugin_path, &plugin_dirs);
/// ```
pub fn load_plugin(plugin_path: &PathBuf, runtime_data: Rc<RefCell<RuntimeData>>) -> Plugin {
    let plugins_paths: Vec<PathBuf> = match env::var_os("ANYRUN_PLUGINS") {
        Some(paths) => env::split_paths(&paths).collect(),
        None => [
            runtime_data.borrow().config_dir.clone(),
            DEFAULT_CONFIG_DIR.to_string(),
        ]
        .iter()
        .map(|plugins_path| PathBuf::from(format!("{}/plugins", plugins_path)))
        .collect(),
    };

    let path = if plugin_path.is_absolute() {
        plugin_path.clone()
    } else {
        plugins_paths
            .iter()
            .map(|plugins_path| plugins_path.join(plugin_path))
            .find(|path| path.exists())
            .unwrap_or_else(|| panic!("Invalid plugin path: {}", plugin_path.to_string_lossy()))
    };

    let plugin = abi_stable::library::lib_header_from_path(&path)
        .and_then(|plugin| plugin.init_root_module::<Plugin>())
        .unwrap_or_else(|_| panic!("Failed to load plugin: {}", path.to_string_lossy()));
    plugin.init()(runtime_data.borrow().config_dir.clone().into());
    plugin
}

pub fn refresh_matches(
    input: &str,
    plugins: &[Plugin],
    main_list_rc: Rc<impl ContainerExt + ListBoxExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    main_list_rc
        .children()
        .iter()
        .for_each(|child| main_list_rc.remove(child));

    let handle_async_matches = |plugin: &Plugin, id: u64| {
        let plugin_clone = *plugin;
        let main_list_rc_clone = main_list_rc.clone();
        let runtime_data_clone = runtime_data.clone();

        glib::timeout_add_local(Duration::from_millis(1), move || {
            let main_list_rc_clone_clone = main_list_rc_clone.clone();
            let runtime_data_clone_clone = runtime_data_clone.clone();
            async_match(&plugin_clone, id, move |matches| {
                handle_matches(
                    main_list_rc_clone_clone.clone(),
                    matches,
                    plugin_clone,
                    runtime_data_clone_clone.clone(),
                );
            })
        });
    };

    if let Some(exclusive_plugin) = runtime_data.borrow().exclusive.as_ref() {
        let id = exclusive_plugin.get_matches()(input.into());
        handle_async_matches(exclusive_plugin, id);
        return;
    }

    for plugin in plugins.iter() {
        let id = plugin.get_matches()(input.into());
        handle_async_matches(plugin, id);
    }
}

pub fn async_match<F>(plugin: &Plugin, id: u64, mut func: F) -> glib::ControlFlow
where
    F: FnMut(&[Match]),
{
    match plugin.poll_matches()(id) {
        PollResult::Ready(matches) => {
            func(&matches);
            glib::ControlFlow::Break
        }
        PollResult::Pending => glib::ControlFlow::Continue,
        PollResult::Cancelled => glib::ControlFlow::Break,
    }
}
