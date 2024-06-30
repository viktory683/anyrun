use anyrun_interface::PluginRef as Plugin;
use clap::{Parser, ValueEnum};
use gtk::{gdk::Rectangle, gio, glib};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

// Config struct and its implementation
#[anyrun_macros::config_args]
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_width")]
    pub width: RelativeNum,

    #[serde(default = "Config::default_height")]
    pub height: RelativeNum,

    #[serde(default = "Config::default_edges")]
    pub edges: Vec<Edge>,

    #[serde(default)]
    pub margin: Vec<RelativeNum>,

    #[serde(default = "Config::default_plugins")]
    pub plugins: Vec<PathBuf>,

    #[serde(default)]
    pub hide_match_icons: bool,
    #[serde(default)]
    pub hide_plugins_icons: bool,
    #[serde(default)]
    pub hide_plugin_info: bool,
    #[serde(default)]
    pub steal_focus: bool,
    #[serde(default)]
    pub ignore_exclusive_zones: bool,
    #[serde(default)]
    pub show_results_immediately: bool,

    #[serde(default)]
    pub save_entry_state: bool,

    #[serde(default)]
    pub layer: Layer,
    #[serde(default)]
    pub bottom_entry: bool,
}

impl Config {
    fn default_width() -> RelativeNum {
        RelativeNum::Fraction(0.5)
    }

    fn default_height() -> RelativeNum {
        RelativeNum::Absolute(0)
    }

    fn default_edges() -> Vec<Edge> {
        vec![Edge::Top]
    }

    fn default_plugins() -> Vec<PathBuf> {
        vec![
            "libapplications.so".into(),
            "libsymbols.so".into(),
            "libshell.so".into(),
            "libtranslate.so".into(),
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: Self::default_width(),
            height: Self::default_height(),
            edges: Self::default_edges(),
            margin: Vec::default(),
            plugins: Self::default_plugins(),
            hide_match_icons: false,
            hide_plugins_icons: true,
            hide_plugin_info: false,
            ignore_exclusive_zones: false,
            steal_focus: false,
            show_results_immediately: false,
            layer: Layer::default(),
            bottom_entry: false,
            save_entry_state: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl From<Edge> for gtk_layer_shell::Edge {
    fn from(val: Edge) -> Self {
        match val {
            Edge::Left => gtk_layer_shell::Edge::Left,
            Edge::Right => gtk_layer_shell::Edge::Right,
            Edge::Top => gtk_layer_shell::Edge::Top,
            Edge::Bottom => gtk_layer_shell::Edge::Bottom,
        }
    }
}

// Layer enum and its implementation
#[derive(Deserialize, Clone, Copy, ValueEnum)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl From<Layer> for gtk_layer_shell::Layer {
    fn from(val: Layer) -> Self {
        match val {
            Layer::Background => gtk_layer_shell::Layer::Background,
            Layer::Bottom => gtk_layer_shell::Layer::Bottom,
            Layer::Top => gtk_layer_shell::Layer::Top,
            Layer::Overlay => gtk_layer_shell::Layer::Overlay,
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::Top
    }
}

// RelativeNum enum and its implementation
#[derive(Deserialize, Clone, Copy)]
pub enum RelativeNum {
    Absolute(i32),
    Fraction(f32),
}

impl Default for RelativeNum {
    fn default() -> Self {
        RelativeNum::Absolute(0)
    }
}

impl RelativeNum {
    pub fn to_val(self, val: u32) -> i32 {
        match self {
            RelativeNum::Absolute(num) => num,
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

// Args struct for command line arguments
#[derive(Parser)]
pub struct Args {
    /// Override the path to the config directory
    #[arg(short, long)]
    pub config_dir: Option<String>,
    #[command(flatten)]
    pub config: ConfigArgs,
}

// Enum for positions
#[derive(Deserialize, Clone, Copy, ValueEnum)]
enum Position {
    Top,
    Center,
}

// Enum for actions after GTK has finished
pub enum PostRunAction {
    Copy(Vec<u8>),
    None,
}

// Struct for runtime data
pub struct RuntimeData {
    pub exclusive: Option<Plugin>,
    pub plugins: Vec<Plugin>,
    pub post_run_action: PostRunAction,
    pub config: Config,
    pub error_label: String,
    pub config_dir: PathBuf,
    pub geometry: Rectangle,
    pub list_store: gio::ListStore,
    pub app_state: gio::Settings,
}

/// The naming scheme for CSS styling
///
/// Refer to [GTK 3.0 CSS Overview](https://docs.gtk.org/gtk3/css-overview.html)
/// and [GTK 3.0 CSS Properties](https://docs.gtk.org/gtk3/css-properties.html) for how to style.
pub mod style_names {
    pub const ENTRY: &str = "entry";
    pub const MAIN: &str = "main";
    pub const WINDOW: &str = "window";
    pub const MATCH: &str = "match";
    pub const MATCH_TITLE: &str = "match-title";
    pub const MATCH_DESC: &str = "match-desc";
}

pub const APP_ID: &str = "com.kirottu.anyrun";

pub fn default_config_dir() -> PathBuf {
    let dirs = glib::system_config_dirs();
    if let Some(dir) = dirs
        .iter()
        .map(|dir| dir.join("anyrun"))
        .find(|dir| dir.exists())
    {
        return dir;
    }

    panic!(
        "Can't find default config dir. Please check {:?} for \"anyrun\" subdirectory",
        dirs
    );
}

// Function to load config from file or use defaults
pub fn load_config(config_dir: &Path) -> (Config, String) {
    let config_path = config_dir.join("config.ron");

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

// Function to determine config directory
pub fn determine_config_dir(config_dir_arg: &Option<String>) -> PathBuf {
    if let Some(config_dir) = config_dir_arg {
        return PathBuf::from(config_dir);
    }

    let user_dir = glib::user_config_dir().join("anyrun");

    if user_dir.exists() {
        return user_dir;
    }
    default_config_dir()
}
