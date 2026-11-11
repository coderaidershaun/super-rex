//! An installed file that `rex init` cannot read is left alone and named, and
//! the rest of the install still happens.

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::support::{AGENT_MARKER, SKILL_MARKER, installed, repo, rex_init_ok};

/// A file that nobody can read. It is made readable again when dropped, so
/// the temp dir can be removed even if the test fails.
struct Locked(PathBuf);

impl Locked {
    /// `None` when the lock has no effect, as when the tests run as root.
    fn file(path: PathBuf, contents: &str) -> Option<Self> {
        fs::write(&path, contents).expect("write locked file");
        fs::set_permissions(&path, Permissions::from_mode(0o000)).expect("lock file");
        let locked = Self(path);
        fs::read(&locked.0).is_err().then_some(locked)
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, Permissions::from_mode(0o755));
    }
}

// This test must never be deleted. Everything `rex init` does besides the
// bundle — AGENTS.md, the .gitignore block, the pre-commit config and hooks —
// runs after it, so one file rex cannot read must not end the whole install.
#[test]
fn skills_force_skips_an_unreadable_installed_file_and_finishes_the_install() {
    let repo = repo();
    let marker = installed(&repo, SKILL_MARKER);
    fs::create_dir_all(marker.parent().expect("marker has a folder")).expect("create skill folder");
    let Some(_locked) = Locked::file(marker, "stale\n") else {
        return;
    };

    // Without `--force` the file is left alone too, and rex must not
    // suggest `--force` for a file whose contents it cannot know.
    let without_force = rex_init_ok(&repo, &[]);
    assert!(
        !without_force.stdout.contains("--force"),
        "rex must not suggest --force for a file it cannot read, got: {}",
        without_force.stdout
    );

    let printed = rex_init_ok(&repo, &["--force"]);

    assert!(
        printed.stderr.contains(
            "rex init: skipped ./.rex/skills/rex-skill-language/SKILL.md: Permission denied"
        ),
        "warning must name the skipped file, got: {}",
        printed.stderr
    );
    assert!(
        repo.path().join("AGENTS.md").is_file()
            && repo.path().join(".pre-commit-config.yaml").is_file(),
        "the rest of the install must still happen"
    );
    assert!(
        installed(&repo, AGENT_MARKER).is_file(),
        "the other bundle files must still be installed"
    );
    assert!(
        !printed.stdout.contains("differs from this version"),
        "a file rex cannot read is not known to differ, got: {}",
        printed.stdout
    );
}
