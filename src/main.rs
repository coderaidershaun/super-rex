//! The `rex` binary: reads the command line and hands each command to its
//! handler in the library.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

use super_rex::commands::{
    catalog, codebase, feedback, hook, init, job, jobs, oms, recall, skills, smells, status, watch,
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

// SMELL: the `///` line on each variant is that command's help text, and clap
// lets it replace any description that the command's own options struct sets.
// So a command's description can only change with an edit to this file.
#[derive(Subcommand)]
enum Command {
    /// Set SuperRex up in a repo. Safe to run twice.
    Init(init::InitOptions),

    /// Open, plan, close and show a job.
    Job(job::JobOptions),

    /// Called by Claude Code, not by people. Append an event to the log.
    Hook(hook::HookOptions),

    /// Print where a job stands: open sub-agents and what has finished.
    Status(status::StatusOptions),

    /// Print the grid agent for a set of inputs.
    Oms(oms::OmsOptions),

    /// List the skills, or check a skill file.
    Skills(skills::SkillsOptions),

    /// Print lessons from similar past jobs and the orchestrators they used.
    Recall(recall::RecallOptions),

    /// List jobs for scripts and agents.
    Jobs(jobs::JobsOptions),

    /// Show a live dashboard of jobs for people.
    Watch(watch::WatchOptions),

    /// Record what the user said about a job, the root cause and the lesson.
    Feedback(feedback::FeedbackOptions),

    /// Print a tree outline of the working directory.
    Codebase(codebase::CodebaseOptions),

    /// List every `SMELL` flag comment.
    Smells(smells::SmellsOptions),

    /// List every command with its full invocation path and description.
    Commands(catalog::CommandsOptions),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = PathBuf::from(".");

    match cli.command {
        Command::Init(options) => finish(init::run(&cwd, options)),
        Command::Job(options) => finish(job::run(&cwd, options)),
        Command::Hook(options) => finish(hook::run(&cwd, options)),
        Command::Status(options) => finish(status::run(&cwd, options)),
        Command::Oms(options) => finish(oms::run(&cwd, options)),
        Command::Skills(options) => finish(skills::run(&cwd, options)),
        Command::Recall(options) => finish(recall::run(&cwd, options)),
        Command::Jobs(options) => finish(jobs::run(&cwd, options)),
        Command::Watch(options) => finish(watch::run(&cwd, options)),
        Command::Feedback(options) => finish(feedback::run(&cwd, options)),
        Command::Codebase(options) => finish(codebase::run(&cwd, options)),
        Command::Smells(options) => finish(smells::run(&cwd, options)),
        Command::Commands(options) => finish(catalog::run(&Cli::command(), options)),
    }
}

fn finish(result: Result<(), impl CommandError>) -> ExitCode {
    let Err(error) = result else {
        return ExitCode::SUCCESS;
    };
    let code = exit_code_for(&error);
    // If stderr cannot be written to, there is nowhere left to report it.
    let _ = writeln!(std::io::stderr(), "error: {:#}", anyhow::Error::new(error));
    ExitCode::from(code)
}

fn exit_code_for(error: &impl CommandError) -> u8 {
    if error.is_usage() { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(thiserror::Error, Debug)]
    enum TestError {
        #[error("the command line is wrong")]
        Usage,

        #[error("the work failed")]
        Work,
    }

    impl CommandError for TestError {
        fn is_usage(&self) -> bool {
            matches!(self, Self::Usage)
        }
    }

    #[test]
    fn a_usage_error_exits_with_2_and_any_other_error_with_1() {
        assert_eq!(exit_code_for(&TestError::Usage), 2);
        assert_eq!(exit_code_for(&TestError::Work), 1);
    }
}
