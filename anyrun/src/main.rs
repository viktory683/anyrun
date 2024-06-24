mod config;
mod plugins;
mod ui;

use std::{cell::RefCell, rc::Rc};

use anyrun_interface::PluginRef as Plugin;
use clap::Parser;
use gtk::{gio, glib, prelude::*};
use nix::unistd;

use config::{determine_config_dir, load_config, style_names, Args, PostRunAction, RuntimeData};
use plugins::{load_plugin, refresh_matches};
use ui::*;
use wl_clipboard_rs::copy;

fn main() -> Result<glib::ExitCode, glib::Error> {
    gtk::init().expect("Failed to initialize GTK.");

    let app = gtk::Application::new(Some("com.kirottu.anyrun"), Default::default());
    app.register(gio::Cancellable::NONE)?;

    if app.is_remote() {
        return Ok(glib::ExitCode::SUCCESS);
    }

    let args = Args::parse();
    let config_dir = determine_config_dir(&args.config_dir);
    let (config, error_label) = load_config(&config_dir);

    let runtime_data = Rc::new(RefCell::new(RuntimeData {
        exclusive: None,
        post_run_action: PostRunAction::None,
        config,
        error_label,
        config_dir,
    }));

    let runtime_data_clone = runtime_data.clone();
    app.connect_activate(move |app| activate(app, runtime_data_clone.clone()));
    let exit_code = app.run_with_args::<String>(&[]);

    handle_post_run_action(runtime_data);

    Ok(exit_code)
}

fn handle_post_run_action(runtime_data: Rc<RefCell<RuntimeData>>) {
    if let PostRunAction::Copy(bytes) = &runtime_data.borrow().post_run_action {
        match unsafe { unistd::fork() } {
            Ok(unistd::ForkResult::Parent { .. }) => {
                println!("Child spawned to serve copy requests.");
            }
            Ok(unistd::ForkResult::Child) => {
                serve_copy_requests(bytes);
            }
            Err(why) => {
                eprintln!("Failed to fork for copy sharing: {}", why);
            }
        }
    }
}

fn serve_copy_requests(bytes: &[u8]) {
    let mut opts = copy::Options::new();
    opts.foreground(true);
    opts.copy(
        copy::Source::Bytes(bytes.to_vec().into_boxed_slice()),
        copy::MimeType::Autodetect,
    )
    .expect("Failed to serve copy bytes");
}

fn activate(app: &impl IsA<gtk::Application>, runtime_data: Rc<RefCell<RuntimeData>>) {
    load_custom_css(runtime_data.clone());

    let main_list = Rc::new(
        gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .name(style_names::MAIN)
            .build(),
    );

    let plugins: Vec<_> = runtime_data
        .borrow()
        .config
        .plugins_paths
        .iter()
        .map(|filename| load_plugin(filename, runtime_data.clone()))
        .collect();

    let window = Rc::new(setup_main_window(app, runtime_data.clone()));

    let entry = Rc::new(
        gtk::SearchEntry::builder()
            .hexpand(true)
            .name(style_names::ENTRY)
            .build(),
    );
    setup_entry_changed(
        entry.clone(),
        runtime_data.clone(),
        plugins.clone(),
        main_list.clone(),
    );
    setup_entry_activated(
        entry.clone(),
        main_list.clone(),
        window.clone(),
        runtime_data.clone(),
        plugins.clone(),
    );

    setup_row_activated(
        main_list.clone(),
        window.clone(),
        runtime_data.clone(),
        entry.clone(),
        plugins.clone(),
    );

    if runtime_data.borrow().config.show_results_immediately {
        refresh_matches("", &plugins, main_list.clone(), runtime_data.clone());
    }

    connect_key_press_events(window.clone());
    if runtime_data.borrow().config.close_on_click {
        handle_close_on_click(window.clone());
    }

    setup_configure_event(
        window.clone(),
        runtime_data.clone(),
        entry.clone(),
        main_list.clone(),
    );
    window.show_all();
}

fn setup_entry_changed(
    entry: Rc<gtk::SearchEntry>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    plugins: Vec<Plugin>,
    main_list: Rc<gtk::ListBox>,
) {
    entry.connect_changed(move |e| {
        runtime_data.borrow_mut().exclusive = None;
        refresh_matches(&e.text(), &plugins, main_list.clone(), runtime_data.clone());
    });
}

fn setup_entry_activated(
    entry: Rc<gtk::SearchEntry>,
    main_list: Rc<gtk::ListBox>,
    window: Rc<gtk::ApplicationWindow>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    plugins: Vec<Plugin>,
) {
    entry.connect_activate(move |e| {
        if let Some(row) = main_list.children().first() {
            handle_selection_activation(row.clone(), window.clone(), runtime_data.clone(), |_| {
                refresh_matches(&e.text(), &plugins, main_list.clone(), runtime_data.clone())
            })
        }
    });
}

fn setup_row_activated(
    main_list: Rc<gtk::ListBox>,
    window: Rc<gtk::ApplicationWindow>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<gtk::SearchEntry>,
    plugins: Vec<Plugin>,
) {
    let main_list_clone = main_list.clone();
    main_list.connect_row_activated(move |_, row| {
        handle_selection_activation(row.clone(), window.clone(), runtime_data.clone(), |_| {
            refresh_matches(
                &entry.text(),
                &plugins,
                main_list_clone.clone(),
                runtime_data.clone(),
            )
        })
    });
}
