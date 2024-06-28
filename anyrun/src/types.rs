use abi_stable::{
    std_types::{ROption, RString},
    traits::IntoReprRust,
};
use gtk::{
    gio::prelude::*,
    glib::{self, subclass::prelude::*, ParamSpec},
};
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MatchRow {
        pub title: RefCell<String>,
        pub description: RefCell<Option<String>>,
        pub use_pango: Cell<bool>,
        pub icon: RefCell<Option<String>>,
        pub id: Cell<u64>,
        id_some: Cell<bool>, // workarond to get something like `Option<u64>` for id with glib because I couldn't find some
        pub plugin_id: Cell<u64>,
        pub first: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MatchRow {
        const NAME: &'static str = "MatchRow";

        type Type = super::MatchRow;
    }

    impl ObjectImpl for MatchRow {
        fn properties() -> &'static [ParamSpec] {
            use std::sync::OnceLock;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("title").build(),
                    glib::ParamSpecString::builder("description").build(),
                    glib::ParamSpecBoolean::builder("use-pango").build(),
                    glib::ParamSpecString::builder("icon").build(),
                    glib::ParamSpecUInt64::builder("id").build(),
                    glib::ParamSpecBoolean::builder("id-some").build(),
                    glib::ParamSpecUInt64::builder("plugin-id").build(),
                    glib::ParamSpecBoolean::builder("first").build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "title" => {
                    let title: String = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.title.replace(title);
                }
                "description" => {
                    let description: Option<String> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.description.replace(description);
                }
                "use-pango" => {
                    let use_pango: bool = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.use_pango.replace(use_pango);
                }
                "icon" => {
                    let icon: Option<String> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.icon.replace(icon);
                }
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "id-some" => {
                    let id_some = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id_some.replace(id_some);
                }
                "plugin-id" => {
                    let plugin_id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.plugin_id.replace(plugin_id);
                }
                "first" => {
                    let first = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.first.replace(first);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "title" => self.title.borrow().to_value(),
                "description" => self.description.borrow().to_value(),
                "use-pango" => self.use_pango.get().to_value(),
                "icon" => self.icon.borrow().to_value(),
                "id" => self.id.get().to_value(),
                "id-some" => self.id_some.get().to_value(),
                "plugin-id" => self.plugin_id.get().to_value(),
                "first" => self.first.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed()
        }
    }
}

glib::wrapper! {
    pub struct MatchRow(ObjectSubclass<imp::MatchRow>);
}

impl MatchRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn get_title(&self) -> String {
        self.property("title")
    }

    pub fn set_title(&self, value: String) {
        self.set_property("title", value)
    }

    pub fn get_description(&self) -> Option<String> {
        self.property("description")
    }

    pub fn set_description(&self, value: Option<String>) {
        self.set_property("description", value)
    }

    pub fn get_use_pango(&self) -> bool {
        self.property("use-pango")
    }

    pub fn set_use_pango(&self, value: bool) {
        self.set_property("use-pango", value)
    }

    pub fn get_icon(&self) -> Option<String> {
        self.property("icon")
    }

    pub fn set_icon(&self, value: Option<String>) {
        self.set_property("icon", value)
    }

    pub fn get_id(&self) -> Option<u64> {
        let id = self.property("id");
        let id_some = self.property("id-some");

        if id_some {
            return Some(id);
        }
        None
    }

    pub fn set_id(&self, value: Option<u64>) {
        if let Some(value) = value {
            self.set_property("id", value);
            self.set_property("id-some", true);
        } else {
            self.set_property("id", 0u64);
            self.set_property("id-some", false);
        }
    }

    pub fn get_plugin_id(&self) -> u64 {
        self.property("plugin-id")
    }

    pub fn set_plugin_id(&self, value: u64) {
        self.set_property("plugin-id", value)
    }

    pub fn get_first(&self) -> bool {
        self.property("first")
    }

    pub fn set_first(&self, value: bool) {
        self.set_property("first", value);
    }
}

impl Default for MatchRow {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MatchRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchRow")
            .field("title", &self.get_title())
            .field("description", &self.get_description())
            .field("use_pango", &self.get_use_pango())
            .field("icon", &self.get_icon())
            .field("id", &self.get_id())
            .field("plugin_id", &self.get_plugin_id())
            .field("first", &self.get_first())
            .finish()
    }
}

impl From<anyrun_interface::Match> for MatchRow {
    fn from(value: anyrun_interface::Match) -> Self {
        fn from_ropt_to_opt(value: ROption<RString>) -> Option<String> {
            if let ROption::RSome(s) = value {
                Some(s.to_string())
            } else {
                None
            }
        }

        let item = Self::new();

        item.set_title(value.title.into());
        item.set_description(from_ropt_to_opt(value.description));
        item.set_use_pango(value.use_pango);
        item.set_icon(from_ropt_to_opt(value.icon));
        item.set_id(value.id.into_rust());

        item.set_plugin_id(0);

        item.set_first(true);

        item
    }
}

impl From<MatchRow> for anyrun_interface::Match {
    fn from(val: MatchRow) -> Self {
        fn from_opt_to_ropt(value: Option<String>) -> ROption<RString> {
            if let Some(s) = value {
                ROption::RSome(s.into())
            } else {
                ROption::RNone
            }
        }

        anyrun_interface::Match {
            title: val.get_title().into(),
            description: from_opt_to_ropt(val.get_description()),
            use_pango: val.get_use_pango(),
            icon: from_opt_to_ropt(val.get_icon()),
            id: val.get_id().into(),
        }
    }
}
