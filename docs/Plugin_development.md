# Plugin development

The plugin API is intentionally very simple to use. This is all you need for a
plugin:

`Cargo.toml`:

```toml
#[package] omitted
[lib]
crate-type = ["cdylib"] # Required to build a dynamic library that can be loaded by anyrun

[dependencies]
anyrun-plugin = { git = "https://github.com/Kirottu/anyrun" }
abi_stable = "0.11.1"
# Any other dependencies you may have
```

`lib.rs`:

```rust
use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;

#[init]
fn init(config_dir: RString) {
    // Your initialization code. This is run in another thread.
    // The return type is the data you want to share between functions
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Demo".into(),
        icon: "help-about".into(), // Icon from the icon theme
    }
}

#[get_matches]
fn get_matches(input: RString) -> RVec<Match> {
    // The logic to get matches from the input text in the `input` argument.
    // The `data` is a mutable reference to the shared data type later specified.
    vec![Match {
        title: "Test match".into(),
        icon: ROption::RSome("help-about".into()),
        use_pango: false,
        description: ROption::RSome("Test match for the plugin API demo".into()),
        id: ROption::RNone, // The ID can be used for identifying the match later, is not required
    }]
    .into()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    // Handle the selected match and return how anyrun should proceed
    HandleResult::Close
}

```

And that's it! That's all of the API needed to make runners. Refer to the
plugins in the [plugins](plugins) folder for more examples.