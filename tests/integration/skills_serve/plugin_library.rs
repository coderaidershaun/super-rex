//! The plugin's own library as the plugin's server entry serves it: the name it is given, the vocabulary beside it, and the embedded copy that answers the same version.

use std::ffi::OsStr;
use std::path::PathBuf;

use serde_json::Value;

use super::{Server, fixture};

fn plugin_library() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugin/library")
}

fn head_with(args: &[&OsStr]) -> Value {
    let mut server = Server::start_with(args);
    server.initialize();
    server.head()
}

#[test]
fn the_plugin_server_entry_serves_the_plugin_library_as_super_rex() {
    let library = plugin_library();
    let head = head_with(&[
        OsStr::new("--name"),
        OsStr::new("super-rex"),
        OsStr::new("--library"),
        library.as_os_str(),
    ]);

    assert_eq!(head["library"], "super-rex");
    assert_eq!(head["skills"], 25);
    assert_eq!(head["vocabulary"], "2");
}

#[test]
fn with_no_name_the_first_folder_base_name_stays_the_name() {
    let library = plugin_library();
    let lib_a = fixture("lib-a");

    let plugin = head_with(&[OsStr::new("--library"), library.as_os_str()]);
    let fixture = head_with(&[OsStr::new("--library"), lib_a.as_os_str()]);

    assert_eq!(plugin["library"], "library");
    assert_eq!(fixture["library"], "lib-a");
}

#[test]
fn a_name_does_not_change_the_version() {
    let library = plugin_library();
    let named = head_with(&[
        OsStr::new("--name"),
        OsStr::new("other"),
        OsStr::new("--library"),
        library.as_os_str(),
    ]);
    let unnamed = head_with(&[OsStr::new("--library"), library.as_os_str()]);

    assert_eq!(named["library"], "other");
    assert_eq!(named["version"], unnamed["version"]);
}

#[test]
fn the_embedded_library_answers_the_version_of_the_plugin_library_folder() {
    let library = plugin_library();
    let embedded = head_with(&[]);
    let folder = head_with(&[OsStr::new("--library"), library.as_os_str()]);

    assert_eq!(embedded["library"], "super-rex");
    assert_eq!(embedded["vocabulary"], "2");
    assert_eq!(embedded["vocabulary"], folder["vocabulary"]);
    assert_eq!(embedded["skills"], folder["skills"]);
    assert_eq!(embedded["version"], folder["version"]);
}

#[test]
fn the_embedded_library_takes_a_name_too() {
    let head = head_with(&[OsStr::new("--name"), OsStr::new("team")]);

    assert_eq!(head["library"], "team");
}
