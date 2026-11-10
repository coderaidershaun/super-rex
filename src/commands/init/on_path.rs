//! Finds out whether a tool is installed by looking in the folders on PATH,
//! without starting it.

use std::env;
use std::ffi::{OsStr, OsString};

/// True when no file named `tool` is in any of the directories on PATH.
///
/// The tool itself is not started. Starting a program to learn that it is
/// installed runs whatever the name points at, on every install, and PATH
/// holds the answer already.
pub(super) fn is_missing(tool: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return true;
    };

    let pathext = env::var_os("PATHEXT");
    let names = executable_names(tool, pathext.as_deref());
    !env::split_paths(&path).any(|dir| names.iter().any(|name| dir.join(name).is_file()))
}

/// The file names that count as `tool` being installed. Windows runs
/// `tool.exe` for `tool` and names the endings it tries in PATHEXT. No other
/// system sets that variable.
fn executable_names(tool: &str, pathext: Option<&OsStr>) -> Vec<OsString> {
    let mut names = vec![OsString::from(tool)];
    let Some(pathext) = pathext else {
        return names;
    };

    names.extend(
        pathext
            .to_string_lossy()
            .split(';')
            .filter(|ending| !ending.is_empty())
            .map(|ending| OsString::from(format!("{tool}{ending}"))),
    );
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_plain_name_is_looked_for_where_pathext_is_not_set() {
        assert_eq!(executable_names("trivy", None), [OsString::from("trivy")]);
    }

    #[test]
    fn the_endings_windows_runs_are_looked_for_as_well_as_the_plain_name() {
        let names = executable_names("trivy", Some(OsStr::new(".COM;.EXE;")));

        assert_eq!(
            names,
            ["trivy", "trivy.COM", "trivy.EXE"].map(OsString::from)
        );
    }
}
