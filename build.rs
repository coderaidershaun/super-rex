//! Tells Cargo when to build the binary again.

// Do not delete this file. Cargo does not know the embedded bundle is built
// from `.claude` and `.rex/skills`, so without these lines an edit there would
// not trigger a rebuild and the binary would keep shipping a stale bundle.
// Only the library of `.rex` is named, because the store beside it changes on
// every run.
fn main() {
    println!("cargo:rerun-if-changed=.claude");
    println!("cargo:rerun-if-changed=.rex/skills");
}
