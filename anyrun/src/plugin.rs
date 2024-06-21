use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use abi_stable::std_types::{ROption, RVec};
use anyrun_interface::{Match, PluginRef, PollResult};

use crate::types::{style_names, PluginView, RuntimeData};

use gtk::{gdk_pixbuf, glib, prelude::*};

pub fn build_label(name: String, use_markup: bool, label: String) -> gtk::Label {
    gtk::Label::builder()
        .name(name)
        .wrap(true)
        .xalign(0.0)
        .use_markup(use_markup)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .label(label)
        .build()
}

fn handle_matches(
    plugin_view: PluginView,
    runtime_data: Rc<RefCell<RuntimeData>>,
    matches: RVec<Match>,
) {
    // Clear out the old matches from the list
    for widget in plugin_view.list.children() {
        plugin_view.list.remove(&widget);
    }

    // If there are no matches, hide the plugin's results
    if matches.is_empty() {
        plugin_view.row.hide();
        return;
    }

    for _match in matches {
        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .name(style_names::MATCH)
            .hexpand(true)
            .build();

        if !runtime_data.borrow().config.hide_icons {
            if let ROption::RSome(icon) = &_match.icon {
                let mut match_image = gtk::Image::builder()
                    .name(style_names::MATCH)
                    .pixel_size(32);

                let path = PathBuf::from(icon.as_str());

                // If the icon path is absolute, load that file
                if path.is_absolute() {
                    match gdk_pixbuf::Pixbuf::from_file_at_size(icon.as_str(), 32, 32) {
                        Ok(pixbuf) => match_image = match_image.pixbuf(&pixbuf),
                        Err(why) => {
                            println!("Failed to load icon file: {}", why);
                            // Set "broken" icon
                            match_image = match_image.icon_name("image-missing");
                        }
                    }
                } else {
                    match_image = match_image.icon_name(icon);
                }

                hbox.add(&match_image.build());
            }
        }

        let title = build_label(
            style_names::MATCH_TITLE.to_string(),
            _match.use_pango,
            _match.title.to_string(),
        );

        // If a description is present, make a box with it and the title
        match &_match.description {
            ROption::RSome(desc) => {
                let title_desc_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .name(style_names::MATCH)
                    .hexpand(true)
                    .vexpand(true)
                    .build();
                title_desc_box.add(&title);
                title_desc_box.add(&build_label(
                    style_names::MATCH_DESC.to_string(),
                    _match.use_pango,
                    desc.to_string(),
                ));
                hbox.add(&title_desc_box);
            }
            ROption::RNone => {
                hbox.add(&title);
            }
        }

        let row = gtk::ListBoxRow::builder()
            .name(style_names::MATCH)
            .height_request(32)
            .build();
        row.add(&hbox);
        // GTK data setting is not type checked, so it is unsafe.
        // Only `Match` objects are stored though.
        unsafe {
            row.set_data("match", _match);
        }
        plugin_view.list.add(&row);
    }

    // Refresh the items in the view
    plugin_view.row.show_all();

    let binding = runtime_data.borrow();
    let combined_matches = binding
        .plugins
        .iter()
        .flat_map(|view| {
            view.list
                .children()
                .into_iter()
                .map(move |child| (child.dynamic_cast::<gtk::ListBoxRow>().unwrap(), view))
        })
        .collect::<Vec<(gtk::ListBoxRow, &PluginView)>>();

    // If `max_entries` is set, truncate the amount of entries
    if let Some(max_matches) = runtime_data.borrow().config.max_entries {
        for (row, view) in combined_matches.iter().skip(max_matches) {
            view.list.remove(row);
        }
    }

    // Hide the plugins that no longer have any entries
    for (_, view) in &combined_matches {
        if view.list.children().is_empty() {
            view.row.hide();
        }
    }

    if let Some((row, view)) = combined_matches.first() {
        view.list.select_row(Some(row));
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
/// * `PluginRef` - A reference to the loaded plugin.
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
pub fn load_plugin(plugin_path: &PathBuf, plugins_paths: &[PathBuf]) -> PluginRef {
    let path = if plugin_path.is_absolute() {
        plugin_path.clone()
    } else {
        plugins_paths
            .iter()
            .map(|plugins_path| {
                let mut p = plugins_path.clone();
                p.push(plugin_path);
                p
            })
            .find(|path| path.exists())
            .unwrap_or_else(|| panic!("Invalid plugin path: {}", plugin_path.to_string_lossy()))
    };

    abi_stable::library::lib_header_from_path(&path)
        .and_then(|plugin| plugin.init_root_module::<PluginRef>())
        .unwrap_or_else(|_| panic!("Failed to load plugin: {}", path.to_string_lossy()))
}

/// Refresh the matches from the plugins
pub fn refresh_matches(input: String, runtime_data: Rc<RefCell<RuntimeData>>) {
    for plugin_view in runtime_data.borrow().plugins.iter() {
        let id = plugin_view.plugin.get_matches()(input.clone().into());
        let plugin_view = plugin_view.clone();
        let runtime_data_clone = runtime_data.clone();
        // If a plugin has requested exclusivity, respect it
        if let Some(exclusive) = &runtime_data.borrow().exclusive {
            if plugin_view.plugin.info() == exclusive.plugin.info() {
                glib::timeout_add_local(Duration::from_micros(1000), move || {
                    async_match(plugin_view.clone(), runtime_data_clone.clone(), id)
                });
            } else {
                handle_matches(plugin_view.clone(), runtime_data.clone(), RVec::new());
            }
        } else {
            glib::timeout_add_local(Duration::from_micros(1000), move || {
                async_match(plugin_view.clone(), runtime_data_clone.clone(), id)
            });
        }
    }
}

/// Handle the asynchronously running match task
fn async_match(
    plugin_view: PluginView,
    runtime_data: Rc<RefCell<RuntimeData>>,
    id: u64,
) -> glib::ControlFlow {
    match plugin_view.plugin.poll_matches()(id) {
        PollResult::Ready(matches) => {
            handle_matches(plugin_view, runtime_data.clone(), matches);
            glib::ControlFlow::Break
        }
        PollResult::Pending => glib::ControlFlow::Continue,
        PollResult::Cancelled => glib::ControlFlow::Break,
    }
}
