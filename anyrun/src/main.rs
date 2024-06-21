mod plugin;
mod types;
mod ui;

use std::{cell::RefCell, rc::Rc};

use clap::Parser;
use gtk::{gio, prelude::*};
use nix::unistd;

use plugin::refresh_matches;
use types::{determine_config_dir, load_config, style_names, Args, PostRunAction, RuntimeData};
use ui::*;
use wl_clipboard_rs::copy;

fn main() {
    let app = gtk::Application::new(Some("com.kirottu.anyrun"), Default::default());
    app.register(gio::Cancellable::NONE)
        .expect("Failed to register application");

    if app.is_remote() {
        return;
    }

    let args = Args::parse();
    let config_dir = determine_config_dir(&args.config_dir);
    let (config, error_label) = load_config(&config_dir);

    let runtime_data = Rc::new(RefCell::new(RuntimeData {
        exclusive: None,
        plugins: Vec::new(),
        post_run_action: PostRunAction::None,
        config,
        error_label,
        config_dir,
    }));

    let runtime_data_clone = runtime_data.clone();
    app.connect_activate(move |app| activate(app, runtime_data_clone.clone()));
    app.run_with_args::<String>(&[]);

    handle_post_run_action(runtime_data);
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

fn activate(app: &gtk::Application, runtime_data: Rc<RefCell<RuntimeData>>) {
    let window = setup_main_window(app, runtime_data.clone());
    load_custom_css(runtime_data.clone());
    let main_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .name(style_names::MAIN)
        .build();

    let plugins = load_plugins(runtime_data.clone(), &main_list);
    runtime_data.borrow_mut().plugins = plugins;

    connect_selection_events(runtime_data.clone());

    let entry = setup_entry(runtime_data.clone());
    let entry_rc = Rc::new(entry);

    let main_list_rc = Rc::new(main_list);

    let runtime_data_clone = runtime_data.clone().clone();
    entry_rc.clone().connect_changed(move |entry| {
        refresh_matches(entry.text().to_string(), runtime_data_clone.clone())
    });

    connect_key_press_events(&window, runtime_data.clone().clone(), entry_rc.clone());
    if runtime_data.borrow().config.close_on_click {
        handle_close_on_click(&window);
    }

    setup_configure_event(
        &window,
        runtime_data.clone(),
        entry_rc.clone(),
        main_list_rc.clone(),
    );
    window.show_all();
}
