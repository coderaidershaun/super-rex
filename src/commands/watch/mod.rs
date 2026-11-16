//! `rex watch`: a live view of master trees, jobs and their agents for people,
//! read from the sessions' live snapshots and from the store when there is one.

pub mod data;
mod draw;
mod facts;
pub mod live;
mod live_form;
mod nav;
mod once;
mod rows;
mod stored;
mod theme;
mod view;

use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use self::data::WatchData;
use self::live::LiveReader;
use self::nav::{Action, Key, Nav, on_key};
use super::job::JobError;
use super::jobs::JobsError;
use super::stdout::print_to_stdout;
use crate::error::{CommandError, FailureClass};
use crate::store::{Store, StoreError, StorePaths};

/// How long the data stays as read before the next refresh.
const REFRESH_SECONDS: u64 = 2;

/// Show a live dashboard of master trees, jobs and their agents for people.
#[derive(clap::Args)]
#[command(
    after_help = "Reads the live snapshots of the sessions in .rex/live/ and the store .rex/rex.db when there is one. Up and Down (or k and j) move, Enter (or Right, l) goes into a row, Esc (or Left, h, Backspace) goes up one level, Home or g goes home, ? shows the keys, q quits. Without --once it needs a terminal: with its output redirected it draws nothing and exits 1."
)]
pub struct WatchOptions {
    /// Print the whole view once as plain text, a tree with one line per node, and exit.
    #[arg(long)]
    once: bool,
}

/// Failure of `rex watch`.
#[derive(thiserror::Error, Debug)]
pub enum WatchError {
    #[error(
        "rex watch needs a terminal; run it without redirecting its output, or run rex watch --once"
    )]
    NotATerminal,

    #[error("cannot use the terminal")]
    Terminal(#[source] io::Error),

    #[error("rex watch: cannot read the live snapshots in {}", path.display())]
    Live {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Jobs(#[from] JobsError),

    #[error(transparent)]
    Job(#[from] JobError),
}

impl CommandError for WatchError {
    fn failure_class(&self) -> FailureClass {
        match self {
            WatchError::NotATerminal => FailureClass::Reject,
            WatchError::Terminal(_) | WatchError::Live { .. } => FailureClass::Retry,
            WatchError::Store(error) => error.failure_class(),
            WatchError::Jobs(error) => error.failure_class(),
            WatchError::Job(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            WatchError::Store(error) => error.exit_code(),
            WatchError::Jobs(error) => error.exit_code(),
            WatchError::Job(error) => error.exit_code(),
            _ => {
                if self.is_usage() {
                    2
                } else {
                    1
                }
            }
        }
    }
}

/// The two places `rex watch` reads, and what it kept from the last read.
struct Sources {
    live_dir: PathBuf,
    store: Option<Store>,
    live: LiveReader,
}

impl Sources {
    /// The live snapshots sit in `.rex/live/` of the main checkout in every
    /// storage mode; the store is wherever `rex jobs` finds it, if anywhere.
    fn open(cwd: &Path) -> Result<Sources, WatchError> {
        let live_dir = StorePaths::locate(cwd)?.dir.join("live");
        let store = match StorePaths::discover(cwd) {
            Ok(paths) => Some(Store::open(&paths)?),
            Err(StoreError::NotFound { .. }) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Sources {
            live_dir,
            store,
            live: LiveReader::default(),
        })
    }

    fn load(&mut self) -> Result<WatchData, WatchError> {
        let now = SystemTime::now();
        let now_ms = millis(now);
        let snapshots = self
            .live
            .read(&self.live_dir)
            .map_err(|source| WatchError::Live {
                path: self.live_dir.clone(),
                source,
            })?;
        let parts = snapshots
            .iter()
            .map(|snapshot| snapshot.part(now_ms))
            .collect();
        let store = match &mut self.store {
            Some(store) => Some(stored::load(store, now)?),
            None => None,
        };
        Ok(WatchData::merge(parts, store))
    }
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| {
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    })
}

/// Puts the terminal back when it goes out of scope, on every way out of
/// `run`: a normal quit, an error, or a panic that unwinds.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

/// # Errors
/// - [`WatchError::NotATerminal`] without `--once` if standard output is not a terminal.
/// - [`WatchError::Store`] if the folder is in no Git repository, or the store cannot be opened.
/// - [`WatchError::Live`] if the folder of live snapshots cannot be listed.
/// - [`WatchError::Jobs`], [`WatchError::Job`] if a refresh cannot read the store.
/// - [`WatchError::Terminal`] if the terminal cannot be drawn on or read.
pub fn run(cwd: &Path, options: WatchOptions) -> Result<(), WatchError> {
    if !options.once && !io::stdout().is_terminal() {
        return Err(WatchError::NotATerminal);
    }
    let mut sources = Sources::open(cwd)?;
    live::remove_old(&sources.live_dir, millis(SystemTime::now())).map_err(|source| {
        WatchError::Live {
            path: sources.live_dir.clone(),
            source,
        }
    })?;
    let first = sources.load()?;
    if options.once {
        print_to_stdout(&once::once_text(&first)).map_err(JobError::from)?;
        return Ok(());
    }

    // The guard comes first: a start that fails half way can leave raw mode on.
    let _restore = TerminalGuard;
    let terminal = ratatui::try_init().map_err(WatchError::Terminal)?;
    event_loop(terminal, &mut sources, first)
}

fn event_loop(
    mut terminal: DefaultTerminal,
    sources: &mut Sources,
    first: WatchData,
) -> Result<(), WatchError> {
    let refresh = Duration::from_secs(REFRESH_SECONDS);
    let mut data = first;
    let mut refreshed = Instant::now();
    let mut nav = Nav::default().refreshed(&data);
    loop {
        let age = refreshed.elapsed().as_secs();
        terminal
            .draw(|frame| draw::draw(frame, &data, &nav, age))
            .map_err(WatchError::Terminal)?;

        let wait = refresh
            .saturating_sub(refreshed.elapsed())
            .min(Duration::from_secs(1));
        if event::poll(wait).map_err(WatchError::Terminal)?
            && let Event::Key(key) = event::read().map_err(WatchError::Terminal)?
        {
            let (next, action) = on_key(&nav, Key::of(&key), &data);
            if action == Action::Quit {
                return Ok(());
            }
            nav = next;
        }
        if refreshed.elapsed() >= refresh {
            data = sources.load()?;
            nav = nav.refreshed(&data);
            refreshed = Instant::now();
        }
    }
}
