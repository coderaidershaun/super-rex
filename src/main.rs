//! The `rex` binary: reads the command line and hands each command to its
//! handler in the library.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

use super_rex::commands::{
    catalog, codebase, doctor, feedback, ingest, init, job, jobs, limit, migrate, oms, recall,
    skills, status, test_filter, tree, watch, worktree,
};
use super_rex::error::CommandError;

#[derive(Parser)]
#[command(
    name = "rex",
    version,
    about = "Set a repo up for SuperRex, pick a model for each role, and keep the record of what ran"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// Each command's description is the doc comment of its options struct, which
// clap reads when the variant has none.
#[derive(Subcommand)]
enum Command {
    Init(init::InitOptions),
    Migrate(migrate::MigrateOptions),
    Ingest(ingest::IngestOptions),
    Doctor(doctor::DoctorOptions),
    Job(job::JobOptions),
    TestFilter(test_filter::FilterOptions),
    Limit(limit::LimitOptions),
    Status(status::StatusOptions),
    Oms(oms::OmsOptions),
    Skills(skills::SkillsOptions),
    Recall(recall::RecallOptions),
    Jobs(jobs::JobsOptions),
    Tree(tree::TreeOptions),
    Watch(watch::WatchOptions),
    Feedback(feedback::FeedbackOptions),
    Codebase(codebase::CodebaseOptions),
    Smells(codebase::SmellsOptions),
    Worktree(worktree::WorktreeOptions),
    Commands(catalog::CommandsOptions),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = PathBuf::from(".");

    match cli.command {
        Command::Init(options) => finish(init::run(&cwd, options)),
        Command::Migrate(options) => finish(migrate::run(&cwd, options)),
        Command::Ingest(options) => finish(ingest::run(&cwd, options)),
        Command::Doctor(options) => finish(doctor::run(&cwd, options)),
        Command::Job(options) => finish(job::run(&cwd, options)),
        Command::TestFilter(options) => test_filter::run_filter(options),
        Command::Limit(options) => finish(limit::run(&cwd, options)),
        Command::Status(options) => finish(status::run(&cwd, options)),
        Command::Oms(options) => finish(oms::run(&cwd, options)),
        Command::Skills(options) => finish(skills::run(&cwd, options)),
        Command::Recall(options) => finish(recall::run(&cwd, options)),
        Command::Jobs(options) => finish(jobs::run(&cwd, options)),
        Command::Tree(options) => finish(tree::run(&cwd, options)),
        Command::Watch(options) => finish(watch::run(&cwd, options)),
        Command::Feedback(options) => finish(feedback::run(&cwd, options)),
        Command::Codebase(options) => finish(codebase::run(&cwd, options)),
        Command::Smells(options) => finish(codebase::run_smells(&cwd, options)),
        Command::Worktree(options) => finish(worktree::run(&cwd, options)),
        Command::Commands(options) => finish(catalog::run(&Cli::command(), options)),
    }
}

/// Every exit that is not 0 writes exactly one line on stderr: `error: ` and
/// the error, or a refusal as it is. Lines that the command wrote before it,
/// such as warnings or findings, stay.
///
/// Scripts, agents and the plugin branch on the exit code: 0 done, also when
/// there was nothing to do; 1 the work failed, a check that found something
/// included; 2 the command line is wrong, or the input or the store is refused;
/// `rex ingest` adds 3 and 4 for its own refusals.
fn finish(result: Result<(), impl CommandError>) -> ExitCode {
    let Err(error) = result else {
        return ExitCode::SUCCESS;
    };
    let code = error.exit_code();
    // If stderr cannot be written to, there is nowhere left to report it.
    let _ = writeln!(std::io::stderr(), "{}", error_line(error));
    ExitCode::from(code)
}

/// The error and each of its sources, joined by `: `, as one line. Agents read
/// stderr line by line, so a line end in a tool's output must not split it.
fn error_line(error: impl CommandError) -> String {
    let text = if error.is_bare_line() {
        format!("{:#}", anyhow::Error::new(error))
    } else {
        format!("error: {:#}", anyhow::Error::new(error))
    };
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(thiserror::Error, Debug)]
    enum TestError {
        #[error("a tool failed: {output}")]
        Tool { output: String },
    }

    impl CommandError for TestError {
        fn failure_class(&self) -> super_rex::error::FailureClass {
            super_rex::error::FailureClass::Reject
        }
    }

    #[test]
    fn an_error_text_with_line_ends_prints_as_one_line() {
        let error = TestError::Tool {
            output: "first line\nsecond line\r\nthird line".to_string(),
        };

        let line = error_line(error);

        assert!(line.starts_with("error: "));
        assert!(!line.contains('\n'));
        assert!(!line.contains('\r'));
    }
}
