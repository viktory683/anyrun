use std::{cell::RefCell, env, path::PathBuf, rc::Rc, time::Duration};

use anyrun_interface::{Match, PluginRef as Plugin, PollResult};
#[allow(unused_imports)]
use log::*;

use crate::{config::*, types::GMatch};

use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
};

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
        .pixel_size(32)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start);

    let path = PathBuf::from(icon);

    match_image = if path.is_absolute() {
        match_image.file(path.to_string_lossy())
    } else {
        match_image.icon_name(icon)
    };
    match_image.build()
}

pub fn build_match_box(runtime_data: Rc<RefCell<RuntimeData>>, gmatch: GMatch) -> gtk::Widget {
    let runtime_data = runtime_data.borrow();
    let plugin = runtime_data
        .plugins
        .get(gmatch.get_plugin_id() as usize)
        .expect("Can't get plugin by id");

    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();

    if !runtime_data.config.hide_plugin_info {
        let plugin_info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .width_request(200)
            .spacing(10)
            .sensitive(false)
            .build();

        let plugin_info = plugin.info()();
        if !runtime_data.config.hide_plugins_icons && gmatch.get_first() {
            plugin_info_box.append(&build_image(&plugin_info.icon));
        }

        let plugin_label = gtk::Label::builder()
            .halign(gtk::Align::End)
            .label(if gmatch.get_first() {
                &plugin_info.name
            } else {
                ""
            })
            .build();

        plugin_info_box.append(&plugin_label);
        hbox.append(&plugin_info_box);
    }

    let match_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();

    if !runtime_data.config.hide_match_icons {
        if let Some(icon) = gmatch.get_icon() {
            match_box.append(&build_image(&icon));
        }
    }

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();

    vbox.append(&build_label(
        style_names::MATCH_TITLE,
        gmatch.get_use_pango(),
        &gmatch.get_title(),
        true,
    ));

    if let Some(desc) = gmatch.get_description() {
        vbox.append(&build_label(
            style_names::MATCH_DESC,
            gmatch.get_use_pango(),
            &desc,
            false,
        ));
    }

    match_box.append(&vbox);
    hbox.append(&match_box);

    hbox.into()
}

pub fn handle_matches(plugin_id: u64, matches: &[Match], list_store: gio::ListStore) {
    for (index, rmatch) in matches.iter().enumerate() {
        let gmatch = GMatch::from(rmatch.clone());
        gmatch.set_plugin_id(plugin_id);
        gmatch.set_first(index == 0);
        list_store.append(&gmatch);
    }
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
pub fn load_plugin(plugin_path: &PathBuf, config_dir: &PathBuf) -> Plugin {
    let plugins_paths: Vec<PathBuf> = match env::var_os("ANYRUN_PLUGINS") {
        Some(paths) => env::split_paths(&paths).collect(),
        None => [config_dir, &default_config_dir()]
            .iter()
            .map(|plugins_path| plugins_path.join("plugins"))
            .collect(),
    };

    let path = if plugin_path.is_absolute() {
        plugin_path.clone()
    } else {
        plugins_paths
            .iter()
            .map(|dir| dir.join(plugin_path))
            .find(|path| path.exists())
            .unwrap_or_else(|| panic!("Invalid plugin path: {}", plugin_path.to_string_lossy()))
    };

    let plugin = abi_stable::library::lib_header_from_path(&path)
        .and_then(|header| header.init_root_module::<Plugin>())
        .unwrap_or_else(|_| panic!("Failed to load plugin: {}", path.to_string_lossy()));
    plugin.init()(config_dir.to_string_lossy().into());
    plugin
}

pub fn refresh_matches(input: &str, plugins: &[Plugin], runtime_data: Rc<RefCell<RuntimeData>>) {
    let list_store = runtime_data.borrow().list_store.clone();
    list_store.remove_all();

    let mut exclusive_plugin_id = None;

    let plugins_to_use = if let Some(exclusive_plugin) = runtime_data.borrow().exclusive.as_ref() {
        exclusive_plugin_id = plugins
            .iter()
            .position(|p| p.info() == exclusive_plugin.info());
        trace!("CUSTOM {:?}", exclusive_plugin_id);
        vec![*exclusive_plugin]
    } else {
        plugins.to_vec()
    };

    for (plugin_id, plugin) in plugins_to_use.iter().enumerate() {
        let id = plugin.get_matches()(input.into());

        glib::timeout_add_local(
            Duration::from_millis(1),
            clone!(@strong list_store, @strong plugin => move || {
                async_match(&plugin, id, |matches| {
                    handle_matches(
                        exclusive_plugin_id.unwrap_or(plugin_id) as u64,
                        matches,
                        list_store.clone(),
                    )
                })
            }),
        );
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
