//! Lists every `rex` command with its one-line description, so an agent learns
//! them all in one call instead of reading the help of each command.

use clap::Command;

use super::stdout::print_to_stdout;
use crate::error::RexError;

/// Arguments of `rex commands`.
#[derive(clap::Args)]
pub struct CommandsOptions {}

/// Print every command in `root`'s tree to stdout.
///
/// Output is one line per command, in aligned columns: the full invocation
/// path (for example `rex skills`) followed by its description.
///
/// Clap's injected `help` subcommand and hidden subcommands are skipped.
/// The root itself is omitted — only invokable paths appear.
///
/// # Errors
/// [`RexError::Stdout`] if the list cannot be written to stdout.
pub fn run(root: &Command, _options: CommandsOptions) -> Result<(), RexError> {
    let root_name = root.get_name();
    let mut entries: Vec<(String, String)> = Vec::new();

    for sub in root.get_subcommands() {
        collect(sub, &[root_name], &mut entries);
    }

    let max_path_len = entries.iter().map(|(p, _)| p.len()).max().unwrap_or(0);

    let text: String = entries
        .iter()
        .map(|(path, about)| format!("{path:<max_path_len$}  — {about}\n"))
        .collect();
    print_to_stdout(&text)
}

fn collect(cmd: &Command, path_parts: &[&str], out: &mut Vec<(String, String)>) {
    let name = cmd.get_name();

    if name == "help" || cmd.is_hide_set() {
        return;
    }

    let mut current_parts: Vec<&str> = path_parts.to_vec();
    current_parts.push(name);

    if let Some(about) = cmd.get_about() {
        out.push((current_parts.join(" "), about.to_string()));
    }

    for sub in cmd.get_subcommands() {
        collect(sub, &current_parts, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_root() -> Command {
        Command::new("rex")
            .about("Rex skills installer")
            .subcommand(Command::new("skills").about("Install the skill bundle."))
            .subcommand(Command::new("codebase").about("Write a CODEBASE.md tree outline."))
    }

    #[test]
    fn walker_produces_non_empty_output_and_includes_rex_skills() {
        let root = synthetic_root();
        let mut entries: Vec<(String, String)> = Vec::new();
        for sub in root.get_subcommands() {
            collect(sub, &["rex"], &mut entries);
        }

        assert!(
            !entries.is_empty(),
            "walker must produce at least one entry"
        );

        let has_skills = entries.iter().any(|(path, _)| path == "rex skills");
        assert!(
            has_skills,
            "expected 'rex skills' in entries; got: {entries:?}"
        );
    }
}
