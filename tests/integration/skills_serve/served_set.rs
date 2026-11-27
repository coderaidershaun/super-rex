//! `get` gives exactly the files that `head` hashes: a symbolic link inside a skill folder is in neither.

use std::fs;
use std::os::unix::fs::symlink;

use serde_json::json;

use super::{Server, copy_tree, fixture};

#[test]
fn get_of_a_symlink_to_a_file_of_the_same_skill_is_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("lib");
    copy_tree(&fixture("lib-a"), &library);
    let before = {
        let mut server = Server::start(&[&library]);
        server.initialize();
        server.head()["version"].clone()
    };
    symlink(
        library.join("alpha/SKILL.md"),
        library.join("alpha/link.md"),
    )
    .unwrap();
    symlink(library.join("alpha/sub"), library.join("alpha/sub-link")).unwrap();

    let mut server = Server::start(&[&library]);
    server.initialize();

    assert_eq!(server.head()["version"], before, "head hashes no link");
    for file in ["link.md", "sub-link/note.md"] {
        let (code, _) = server.get_error_code("alpha", file);
        assert_eq!(code, "not_found", "get alpha {file}");
    }
    let (is_error, _) = server.call("get", &json!({ "skill": "alpha", "file": "sub/note.md" }));
    assert!(!is_error, "the real file is still served");
}

#[test]
fn a_skill_whose_skill_file_is_a_symlink_is_not_served() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("lib");
    copy_tree(&fixture("lib-a"), &library);
    fs::create_dir_all(library.join("linked")).unwrap();
    symlink(
        library.join("beta/SKILL.md"),
        library.join("linked/SKILL.md"),
    )
    .unwrap();

    let mut server = Server::start(&[&library]);
    server.initialize();

    assert_eq!(server.head()["skills"], 2);
    let (code, _) = server.get_error_code("linked", "SKILL.md");
    assert_eq!(code, "not_found");
}
