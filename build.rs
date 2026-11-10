// Do not delete this file. Cargo does not know the embedded bundle is built
// from `.claude`, so without this line an edit under `.claude` would not
// trigger a rebuild and the binary would keep shipping a stale bundle.
fn main() {
    println!("cargo:rerun-if-changed=.claude");
}
