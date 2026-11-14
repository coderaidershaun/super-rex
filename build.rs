//! Tells Cargo when to build the binary again.

// Do not delete this file. Cargo does not know the embedded bundle is built
// from the plugin's skill, agent and library folders, so without these lines
// an edit there would not trigger a rebuild and the binary would keep a stale
// bundle. The mod's own code is not named, because the bundle holds none of it.
fn main() {
    println!("cargo:rerun-if-changed=plugin/skills");
    println!("cargo:rerun-if-changed=plugin/agents");
    println!("cargo:rerun-if-changed=plugin/library");
}
