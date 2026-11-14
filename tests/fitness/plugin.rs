//! The plugin and the binary ship as one version, so the plugin that a session
//! loads never meets a `rex` whose reads or event batches it was not built for.

use std::fs;
use std::path::Path;

use serde_json::Value;

// This test must never be deleted. The mod reads the binary's JSON and sends it
// event batches by the contract of one release; a plugin of another version
// would read keys that are not there and the record would go wrong silently.
#[test]
fn plugin_version_is_the_crate_version() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugin/.claude-plugin/plugin.json");
    let text = fs::read_to_string(&manifest).expect("read plugin.json");
    let plugin: Value = serde_json::from_str(&text).expect("plugin.json is JSON");

    assert_eq!(plugin["name"], "super-rex");
    assert_eq!(
        plugin["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "plugin/.claude-plugin/plugin.json and Cargo.toml must name one version"
    );
}
