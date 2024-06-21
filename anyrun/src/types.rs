use std::{env, fs, path::PathBuf};

use anyrun_interface::PluginRef;
use clap::{Parser, ValueEnum};
use serde::Deserialize;

#[anyrun_macros::config_args]
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_x")]
    pub x: RelativeNum,

    #[serde(default = "Config::default_y")]
    pub y: RelativeNum,

    #[serde(default = "Config::default_width")]
    pub width: RelativeNum,

    #[serde(default = "Config::default_height")]
    pub height: RelativeNum,

    #[serde(default = "Config::default_plugins")]
    pub plugins: Vec<PathBuf>,

    #[serde(default)]
    pub hide_icons: bool,
    #[serde(default)]
    pub hide_plugin_info: bool,
    #[serde(default)]
    pub ignore_exclusive_zones: bool,
    #[serde(default)]
    pub close_on_click: bool,
    #[serde(default)]
    pub show_results_immediately: bool,
    #[serde(default)]
    pub max_entries: Option<usize>,
    #[serde(default = "Config::default_layer")]
    pub layer: Layer,
}

impl Config {
    fn default_x() -> RelativeNum {
        RelativeNum::Fraction(0.5)
    }

    fn default_y() -> RelativeNum {
        RelativeNum::Absolute(0)
    }

    fn default_width() -> RelativeNum {
        RelativeNum::Fraction(0.5)
    }

    fn default_height() -> RelativeNum {
        RelativeNum::Absolute(0)
    }

    fn default_plugins() -> Vec<PathBuf> {
        vec![
            "libapplications.so".into(),
            "libsymbols.so".into(),
            "libshell.so".into(),
            "libtranslate.so".into(),
        ]
    }

    fn default_layer() -> Layer {
        Layer::Overlay
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: Self::default_x(),
            y: Self::default_y(),
            width: Self::default_width(),
            height: Self::default_height(),
            plugins: Self::default_plugins(),
            hide_icons: false,
            hide_plugin_info: false,
            ignore_exclusive_zones: false,
            close_on_click: false,
            show_results_immediately: false,
            max_entries: None,
            layer: Self::default_layer(),
        }
    }
}

#[derive(Deserialize, Clone, ValueEnum)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl Layer {
    pub fn to_g_layer(&self) -> gtk_layer_shell::Layer {
        match self {
            Layer::Background => gtk_layer_shell::Layer::Background,
            Layer::Bottom => gtk_layer_shell::Layer::Bottom,
            Layer::Top => gtk_layer_shell::Layer::Top,
            Layer::Overlay => gtk_layer_shell::Layer::Overlay,
        }
    }
}

// Could have a better name
#[derive(Deserialize, Clone)]
pub enum RelativeNum {
    Absolute(i32),
    Fraction(f32),
}

impl Default for RelativeNum {
    fn default() -> Self {
        RelativeNum::Fraction(0.5)
    }
}

impl RelativeNum {
    pub fn to_val(&self, val: u32) -> i32 {
        match self {
            RelativeNum::Absolute(num) => *num,
            RelativeNum::Fraction(frac) => (frac * val as f32) as i32,
        }
    }
}

impl From<&str> for RelativeNum {
    fn from(value: &str) -> Self {
        let (ty, val) = value.split_once(':').expect("Invalid RelativeNum value");

        match ty {
            "absolute" => Self::Absolute(val.parse().unwrap()),
            "fraction" => Self::Fraction(val.parse().unwrap()),
            _ => panic!("Invalid type of value"),
        }
    }
}

/// A "view" of plugin's info and matches
#[derive(Clone)]
pub struct PluginView {
    pub plugin: PluginRef,
    pub row: gtk::ListBoxRow,
    pub list: gtk::ListBox,
}

#[derive(Parser)]
pub struct Args {
    /// Override the path to the config directory
    #[arg(short, long)]
    pub config_dir: Option<String>,
    #[command(flatten)]
    config: ConfigArgs,
}

#[derive(Deserialize, Clone, ValueEnum)]
enum Position {
    Top,
    Center,
}

/// Actions to run after GTK has finished
pub enum PostRunAction {
    Copy(Vec<u8>),
    None,
}

/// Some data that needs to be shared between various parts
pub struct RuntimeData {
    /// A plugin may request exclusivity which is set with this
    pub exclusive: Option<PluginView>,
    pub plugins: Vec<PluginView>,
    pub post_run_action: PostRunAction,
    pub config: Config,
    /// Used for displaying errors later on
    pub error_label: String,
    pub config_dir: String,
}

/// The naming scheme for CSS styling
///
/// Refer to [GTK 3.0 CSS Overview](https://docs.gtk.org/gtk3/css-overview.html)
/// and [GTK 3.0 CSS Properties](https://docs.gtk.org/gtk3/css-properties.html) for how to style.
pub mod style_names {
    /// The text entry box
    pub const ENTRY: &str = "entry";
    /// "Main" widgets (main GtkListBox, main GtkBox)
    pub const MAIN: &str = "main";
    /// The window
    pub const WINDOW: &str = "window";
    /// Widgets related to the whole plugin. Including the info box
    pub const PLUGIN: &str = "plugin";
    /// Widgets for the specific match `MATCH_*` names are for more specific parts.
    pub const MATCH: &str = "match";

    pub const MATCH_TITLE: &str = "match-title";
    pub const MATCH_DESC: &str = "match-desc";
}

/// Default config directory
pub const DEFAULT_CONFIG_DIR: &str = "/etc/anyrun";

pub fn load_config(config_dir: &str) -> (Config, String) {
    let config_path = format!("{}/config.ron", config_dir);

    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(why) => {
            return (
                Config::default(),
                format!(
                    "Failed to read Anyrun config file, using default config: {}",
                    why
                ),
            )
        }
    };

    match ron::from_str(&content) {
        Ok(config) => (config, String::new()),
        Err(why) => (
            Config::default(),
            format!(
                "Failed to parse Anyrun config file, using default config: {}",
                why
            ),
        ),
    }
}

pub fn determine_config_dir(config_dir_arg: &Option<String>) -> String {
    config_dir_arg.clone().unwrap_or_else(|| {
        let user_dir = format!(
            "{}/.config/anyrun",
            env::var("HOME").expect("Could not determine home directory! Is $HOME set?")
        );

        if PathBuf::from(&user_dir).exists() {
            user_dir
        } else {
            DEFAULT_CONFIG_DIR.to_string()
        }
    })
}
