//! The screen of `rex watch` as plain values, and the code that draws them. This
//! file never reads the store: the caller fills a [`ViewModel`] and `draw` only
//! lays it out.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};

use super::model::{Screen, State};
use crate::capture::TokenTotal;
use crate::commands::jobs::{JobSummary, TokenSummary};

/// One line of the job list: the columns of the plain `rex jobs` line, then
/// the run counts and the token text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobListRow {
    pub id: String,
    pub status: String,
    pub work_type: String,
    pub size: String,
    pub consequence: String,
    pub languages: String,
    pub created_at: String,
    pub excerpt: String,
    /// Open, finished and unconfirmed runs, as `2/1/0`.
    pub runs: String,
    pub tokens: String,
}

impl JobListRow {
    pub fn of(summary: &JobSummary) -> JobListRow {
        let counts = &summary.runs;
        JobListRow {
            id: summary.id.clone(),
            status: summary.status.clone(),
            work_type: summary.work_type.clone(),
            size: summary.size.clone(),
            consequence: summary.consequence.clone(),
            languages: summary.languages.join(","),
            created_at: summary.created_at.clone(),
            excerpt: summary.excerpt_on_one_line(),
            runs: format!("{}/{}/{}", counts.open, counts.finished, counts.unconfirmed),
            tokens: summary.tokens.as_ref().map(tokens_text).unwrap_or_default(),
        }
    }
}

/// One sub-agent run in the tree, shown at `depth` levels below the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub depth: usize,
    pub role: String,
    /// The model that ran, else `-`.
    pub model: String,
    pub effort: String,
    pub state: &'static str,
    /// `-` when the run has no duration yet.
    pub duration: String,
    /// Empty when the tokens are unknown.
    pub tokens: String,
    /// The job named by the run's own tag, when it differs from the job shown.
    pub tag_job: Option<String>,
}

/// Everything `draw` needs. `tree` holds the runs of the selected job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewModel {
    pub jobs: Vec<JobListRow>,
    pub tree: Vec<TreeRow>,
}

/// `950 tokens`, `61.0k tokens`, `1.2M tokens`, with `~` after the word when
/// the total can still grow. Counts are cut, not rounded, so `999999` never
/// shows as `1000.0k`.
pub fn tokens_text(summary: &TokenSummary) -> String {
    let sum = summary.sum();
    let number = if sum < 1_000 {
        sum.to_string()
    } else if sum < 1_000_000 {
        let tenths = sum / 100;
        format!("{}.{}k", tenths / 10, tenths % 10)
    } else {
        let tenths = sum / 100_000;
        format!("{}.{}M", tenths / 10, tenths % 10)
    };
    let mark = if summary.provisional { "~" } else { "" };
    format!("{number} tokens{mark}")
}

/// [`tokens_text`] of a run's total; empty when the total is unknown.
pub fn total_text(total: &TokenTotal) -> String {
    TokenSummary::of(total)
        .as_ref()
        .map(tokens_text)
        .unwrap_or_default()
}

/// `41s`, `2m05s`, `1h02m`. `-` when the run has no duration.
pub fn duration_text(duration_ms: Option<u64>) -> String {
    let Some(milliseconds) = duration_ms else {
        return "-".to_owned();
    };
    let seconds = milliseconds / 1_000;
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h{:02}m", seconds / 3_600, seconds % 3_600 / 60)
    }
}

pub fn draw(frame: &mut Frame, view: &ViewModel, state: &State) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    match state.screen {
        Screen::List => draw_jobs(frame, body, view, state),
        Screen::Tree => draw_tree(frame, body, view, state),
    }
    let hint = match state.screen {
        Screen::List => "up/down select   enter show runs   q quit",
        Screen::Tree => "up/down select   enter back to jobs   q quit",
    };
    frame.render_widget(Paragraph::new(hint), footer);
}

fn selected_style() -> Style {
    Style::new().add_modifier(Modifier::REVERSED)
}

fn header_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

fn draw_jobs(frame: &mut Frame, area: ratatui::layout::Rect, view: &ViewModel, state: &State) {
    let block = Block::bordered().title(" jobs ");
    if view.jobs.is_empty() {
        frame.render_widget(Paragraph::new("no jobs").block(block), area);
        return;
    }
    let rows = view.jobs.iter().map(|job| {
        Row::new([
            job.id.as_str(),
            job.status.as_str(),
            job.work_type.as_str(),
            job.size.as_str(),
            job.consequence.as_str(),
            job.languages.as_str(),
            job.created_at.as_str(),
            job.runs.as_str(),
            job.tokens.as_str(),
            job.excerpt.as_str(),
        ])
    });
    // The fixed columns fit in 110 terminal columns; the request takes the rest.
    let widths = [
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Length(24),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Fill(1),
    ];
    let header = Row::new([
        "id",
        "status",
        "type",
        "size",
        "conseq",
        "languages",
        "created",
        "runs o/f/u",
        "tokens",
        "request",
    ])
    .style(header_style());
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selected_style());
    let mut table_state = TableState::default().with_selected(Some(state.job));
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn draw_tree(frame: &mut Frame, area: ratatui::layout::Rect, view: &ViewModel, state: &State) {
    let title = match view.jobs.get(state.job) {
        Some(job) => format!(" runs of {} ", job.id),
        None => " runs ".to_owned(),
    };
    let block = Block::bordered().title(title);
    if view.tree.is_empty() {
        frame.render_widget(Paragraph::new("no sub-agent runs").block(block), area);
        return;
    }
    let rows = view.tree.iter().map(|run| {
        let branch = if run.depth == 0 { "" } else { "└ " };
        let role = format!(
            "{}{}{}",
            "  ".repeat(run.depth.saturating_sub(1)),
            branch,
            run.role
        );
        Row::new([
            Cell::from(role),
            Cell::from(run.model.as_str()),
            Cell::from(run.effort.as_str()),
            Cell::from(run.state),
            Cell::from(run.duration.as_str()),
            Cell::from(run.tokens.as_str()),
            Cell::from(run.tag_job.as_deref().unwrap_or_default()),
        ])
    });
    let widths = [
        Constraint::Length(24),
        Constraint::Length(26),
        Constraint::Length(6),
        Constraint::Length(11),
        Constraint::Length(5),
        Constraint::Length(14),
        Constraint::Fill(1),
    ];
    let header = Row::new([
        "role", "model", "effort", "state", "time", "tokens", "tag job",
    ])
    .style(header_style());
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selected_style());
    let mut table_state = TableState::default().with_selected(Some(state.tree_row));
    frame.render_stateful_widget(table, area, &mut table_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::TokenCounts;

    fn summary(input: u64, provisional: bool) -> TokenSummary {
        TokenSummary {
            input,
            output: 0,
            cache_read: 0,
            cache_creation: 0,
            provisional,
        }
    }

    #[test]
    fn token_text_follows_the_status_command() {
        assert_eq!(tokens_text(&summary(950, false)), "950 tokens");
        assert_eq!(tokens_text(&summary(61_000, true)), "61.0k tokens~");
        assert_eq!(tokens_text(&summary(999_999, false)), "999.9k tokens");
        assert_eq!(tokens_text(&summary(1_250_000, false)), "1.2M tokens");
        assert_eq!(total_text(&TokenTotal::Unknown), "");
        let counts = TokenCounts {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_creation: 4,
        };
        assert_eq!(total_text(&TokenTotal::Provisional(counts)), "10 tokens~");
    }

    #[test]
    fn duration_text_has_three_forms() {
        assert_eq!(duration_text(Some(41_000)), "41s");
        assert_eq!(duration_text(Some(125_000)), "2m05s");
        assert_eq!(duration_text(Some(3_720_000)), "1h02m");
        assert_eq!(duration_text(None), "-");
    }

    #[test]
    fn both_screens_draw_without_a_terminal() {
        use ratatui::{Terminal, backend::TestBackend};
        let view = ViewModel {
            jobs: vec![],
            tree: vec![],
        };
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        for screen in [Screen::List, Screen::Tree] {
            let state = State {
                screen,
                ..State::default()
            };
            terminal.draw(|frame| draw(frame, &view, &state)).unwrap();
        }
    }
}
