mod config;
mod plugins;
mod types;
mod ui;

use std::{cell::RefCell, rc::Rc};

use anyrun_interface::PluginRef as Plugin;
use clap::Parser;
use gtk::{
    gdk, gio,
    glib::{self, clone},
    prelude::*,
};
use log::*;
use nix::unistd;

use config::*;
use plugins::*;
use types::*;
use ui::*;
use wl_clipboard_rs::copy;

fn main() -> Result<glib::ExitCode, glib::Error> {
    env_logger::init();
    gtk::init().expect("Failed to initialize GTK.");

    let app = gtk::Application::new(Some(APP_ID), Default::default());
    app.register(gio::Cancellable::NONE)?;

    if app.is_remote() {
        return Ok(glib::ExitCode::SUCCESS);
    }

    let app_state = gio::Settings::new(APP_ID);

    let args = Args::parse();
    let config_dir = determine_config_dir(&args.config_dir);
    let (mut config, error_label) = load_config(&config_dir);
    config.merge_opt(args.config);

    let display = gdk::Display::default().expect("No display found");
    let monitor = display
        .monitors()
        .into_iter()
        .filter_map(|m| m.ok())
        .peekable()
        .peek()
        .expect("No monitor found")
        .clone()
        .downcast::<gdk::Monitor>()
        .expect("Can't downcast Object to Monitor");
    let geometry = monitor.geometry();

    let list_store = gio::ListStore::builder()
        .item_type(GMatch::static_type())
        .build();

    let plugins = config
        .plugins
        .iter()
        .map(|filename| load_plugin(filename, &config_dir))
        .collect();

    let runtime_data = Rc::new(RefCell::new(RuntimeData {
        exclusive: None,
        post_run_action: PostRunAction::None,
        config,
        error_label,
        config_dir,
        geometry,
        list_store,
        plugins,
        app_state,
    }));

    app.connect_activate(
        clone!(@weak runtime_data => move |app| activate(app, runtime_data.clone())),
    );
    let exit_code = app.run_with_args::<String>(&[]);

    handle_post_run_action(runtime_data);

    Ok(exit_code)
}

fn handle_post_run_action(runtime_data: Rc<RefCell<RuntimeData>>) {
    if let PostRunAction::Copy(bytes) = &runtime_data.borrow().post_run_action {
        match unsafe { unistd::fork() } {
            Ok(unistd::ForkResult::Parent { .. }) => {
                info!("Child spawned to serve copy requests.");
            }
            Ok(unistd::ForkResult::Child) => {
                serve_copy_requests(bytes);
            }
            Err(why) => {
                error!("Failed to fork for copy sharing: {}", why);
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
            .selection_mode(gtk::SelectionMode::Single)
            .name(style_names::MAIN)
            .build(),
    );

    let list_store = runtime_data.clone().borrow().list_store.clone();

    main_list.bind_model(
        Some(&list_store),
        clone!(@strong runtime_data => move |match_row| {
            build_match_box(
                runtime_data.clone(),
                match_row
                    .clone()
                    .downcast::<GMatch>()
                    .expect("Can't downcast glib::Object to GMatch"),
            )
        }),
    );

    let app_state = runtime_data.borrow().app_state.clone();

    let entry = Rc::new(
        gtk::SearchEntry::builder()
            .hexpand(true)
            .name(style_names::ENTRY)
            .height_request(32)
            .build(),
    );
    if runtime_data.borrow().config.save_entry_state {
        entry.set_text(&app_state.string("entry-state"));
        app_state.bind("entry-state", &*entry, "text").build();
    }

    list_store.connect_items_changed(
        clone!(@weak entry, @weak main_list, @weak runtime_data => move |_, _, _, _| {
            main_list.select_row(main_list.row_at_index(0).as_ref());

            resize_window(
                runtime_data.clone(),
                main_list.clone(),
                entry.height_request(),
            );
        }),
    );

    let window = Rc::new(setup_main_window(app, runtime_data.clone()));

    let entry_eck = gtk::EventControllerKey::new();
    connect_entry_key_press_events(entry.clone(), entry_eck, window.clone());

    let window_eck = gtk::EventControllerKey::new();
    connect_window_key_press_events(window.clone(), window_eck, window.clone());

    let plugins = runtime_data.clone().borrow().plugins.clone();

    setup_entry_changed(entry.clone(), runtime_data.clone(), plugins.clone());
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
        refresh_matches(&entry.text(), &plugins, runtime_data.clone());
    }

    configure_main_window(
        window.clone(),
        runtime_data.clone(),
        entry.clone(),
        main_list.clone(),
    );

    window.present();
}

fn setup_entry_changed(
    entry: Rc<gtk::SearchEntry>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    plugins: Vec<Plugin>,
) {
    entry.connect_changed(move |e| {
        runtime_data.borrow_mut().exclusive = None;
        refresh_matches(&e.text(), &plugins, runtime_data.clone());
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
        if let Some(row) = main_list.selected_row() {
            handle_selection_activation(
                row.index().try_into().unwrap(),
                window.clone(),
                runtime_data.clone(),
                |_| refresh_matches(&e.text(), &plugins, runtime_data.clone()),
            )
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
    main_list.connect_row_activated(move |_, row| {
        handle_selection_activation(
            row.index().try_into().unwrap(),
            window.clone(),
            runtime_data.clone(),
            |_| refresh_matches(&entry.text(), &plugins, runtime_data.clone()),
        )
    });
}
