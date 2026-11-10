//! `rex watch`: a live view of jobs and their agent trees, for people. The job
//! data comes from the function `rex jobs` uses, so the screen and the list agree.

mod model;
mod view;

use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use self::model::{Action, Key, Sizes, Snapshot, State, on_key};
use super::jobs::JobsError;
use crate::error::CommandError;
use crate::store::{Store, StoreError, StorePaths};

/// How long the data stays as read before the next refresh.
const REFRESH_SECONDS: u64 = 2;

/// Arguments of `rex watch`.
#[derive(clap::Args)]
pub struct WatchOptions {}

#[derive(thiserror::Error, Debug)]
pub enum WatchError {
    #[error("rex watch needs a terminal; run it without redirecting its output")]
    NotATerminal,

    #[error("cannot use the terminal")]
    Terminal(#[source] io::Error),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Jobs(#[from] JobsError),
}

impl CommandError for WatchError {}

/// Puts the terminal back when it goes out of scope, on every way out of
/// `run`: a normal quit, an error, or a panic that unwinds.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

/// # Errors
/// - [`WatchError::NotATerminal`] if standard output is not a terminal. Nothing is drawn.
/// - [`WatchError::Store`] if the store is missing or cannot be opened.
/// - [`WatchError::Jobs`] if a refresh cannot read the jobs or their runs.
/// - [`WatchError::Terminal`] if the terminal cannot be drawn on or read.
pub fn run(cwd: &Path, _options: WatchOptions) -> Result<(), WatchError> {
    if !io::stdout().is_terminal() {
        return Err(WatchError::NotATerminal);
    }
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let first = Snapshot::load(&mut store, SystemTime::now())?;

    // The guard comes first: a start that fails half way can leave raw mode on.
    let _restore = TerminalGuard;
    let terminal = ratatui::try_init().map_err(WatchError::Terminal)?;
    event_loop(terminal, &mut store, first)
}

fn event_loop(
    mut terminal: DefaultTerminal,
    store: &mut Store,
    first: Snapshot,
) -> Result<(), WatchError> {
    let refresh = Duration::from_secs(REFRESH_SECONDS);
    let mut snapshot = first;
    let mut refreshed = Instant::now();
    let mut state = State::default();
    loop {
        let view = snapshot.view(state.job);
        terminal
            .draw(|frame| view::draw(frame, &view, &state))
            .map_err(WatchError::Terminal)?;

        let wait = refresh.saturating_sub(refreshed.elapsed());
        if event::poll(wait).map_err(WatchError::Terminal)?
            && let Event::Key(key) = event::read().map_err(WatchError::Terminal)?
        {
            let key = if is_interrupt(&key) {
                Key::Quit
            } else {
                Key::of(&key)
            };
            let sizes = Sizes {
                jobs: view.jobs.len(),
                tree_rows: view.tree.len(),
            };
            let (next, action) = on_key(state, key, sizes);
            if action == Action::Quit {
                return Ok(());
            }
            state = next;
        }
        if refreshed.elapsed() >= refresh {
            let selected = view.jobs.get(state.job).map(|row| row.id.clone());
            snapshot = Snapshot::load(store, SystemTime::now())?;
            refreshed = Instant::now();
            if let Some(index) = selected.and_then(|id| snapshot.position(&id)) {
                state.job = index;
            }
        }
    }
}

/// `Ctrl-C` reaches the program as a key in raw mode, not as a signal.
fn is_interrupt(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
}
