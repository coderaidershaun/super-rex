//! The `rex` binary: reads the command line and hands each command to its
//! handler in the library.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};

use super_rex::commands::{
    catalog, codebase, feedback, hook, init, job, jobs, oms, recall, skills, smells, status, watch,
};

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = PathBuf::from(".");

    match cli.command {
        Command::Init(options) => init::run(&cwd, options).context("rex init failed")?,
        Command::Job(options) => job::run(&cwd, options).context("rex job failed")?,
        Command::Hook(options) => hook::run(&cwd, options).context("rex hook failed")?,
        Command::Status(options) => status::run(&cwd, options).context("rex status failed")?,
        Command::Oms(options) => oms::run(&cwd, options).context("rex oms failed")?,
        Command::Skills(options) => skills::run(&cwd, options).context("rex skills failed")?,
        Command::Recall(options) => recall::run(&cwd, options).context("rex recall failed")?,
        Command::Jobs(options) => jobs::run(&cwd, options).context("rex jobs failed")?,
        Command::Watch(options) => watch::run(&cwd, options).context("rex watch failed")?,
        Command::Feedback(options) => {
            feedback::run(&cwd, options).context("rex feedback failed")?;
        }
        Command::Codebase(options) => {
            codebase::run(&cwd, options).context("rex codebase failed")?;
        }
        Command::Smells(options) => smells::run(&cwd, options).context("rex smells failed")?,
        Command::Commands(options) => {
            catalog::run(&Cli::command(), options).context("rex commands failed")?;
        }
    }

    Ok(())
}
