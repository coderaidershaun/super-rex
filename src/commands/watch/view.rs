//! The screen of `rex watch` as plain values, and the code that draws them. This
//! file never reads the store: the caller fills a [`ViewModel`] and `draw` only
//! lays it out, with the colours of [`super::theme`].

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Cell, HighlightSpacing, Padding, Paragraph, Row, Table, TableState, Wrap,
};

use super::model::{Screen, State};
use super::theme;
use crate::capture::TokenTotal;
use crate::commands::jobs::{JobSummary, RunCounts, TokenSummary};

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
    pub runs: RunCounts,
    pub tokens: String,
}

impl JobListRow {
    pub fn of(summary: &JobSummary) -> JobListRow {
        JobListRow {
            id: summary.id.clone(),
            status: summary.status.clone(),
            work_type: summary.work_type.clone(),
            size: summary.size.clone(),
            consequence: summary.consequence.clone(),
            languages: summary.languages.join(","),
            created_at: summary.created_at.clone(),
            excerpt: summary.excerpt_on_one_line(),
            runs: summary.runs,
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

/// `950`, `61.0k`, `1.2M`, with `~` after the number when the total can still
/// grow. The column header names the unit. Counts are cut, not rounded, so
/// `999999` never shows as `1000.0k`.
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
    format!("{number}{mark}")
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

/// `2026-11-10T09:05:00.000Z` as `2026-11-10 09:05`. Other text stays as it is.
fn created_text(created_at: &str) -> String {
    match (
        created_at.get(..10),
        created_at.get(10..11),
        created_at.get(11..16),
    ) {
        (Some(date), Some("T"), Some(time)) => format!("{date} {time}"),
        _ => created_at.to_owned(),
    }
}

/// The guide in front of each run's role, such as `│  └─ `, from the depth of
/// every row in the order they are shown.
fn tree_guides(depths: &[usize]) -> Vec<String> {
    (0..depths.len())
        .map(|row| {
            let depth = depths[row];
            (1..=depth)
                .map(
                    |level| match (level == depth, has_later_sibling(depths, row, level)) {
                        (true, true) => "├─ ",
                        (true, false) => "└─ ",
                        (false, true) => "│  ",
                        (false, false) => "   ",
                    },
                )
                .collect()
        })
        .collect()
}

/// Whether a row at `level` comes after `row` before the tree climbs above `level`.
fn has_later_sibling(depths: &[usize], row: usize, level: usize) -> bool {
    depths[row + 1..]
        .iter()
        .take_while(|depth| **depth >= level)
        .any(|depth| *depth == level)
}

pub fn draw(frame: &mut Frame, view: &ViewModel, state: &State) {
    let [top, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(top_bar(view, state), top);
    match state.screen {
        Screen::List => draw_jobs(frame, body, view, state),
        Screen::Tree => draw_tree(frame, body, view, state),
    }
    frame.render_widget(footer_bar(state.screen), footer);
}

/// The name of the tool, where the person is, and how many jobs need a look.
fn top_bar<'a>(view: &'a ViewModel, state: &State) -> Line<'a> {
    let mut spans = vec![
        Span::styled(" ◆ rex watch ", theme::badge()),
        Span::raw("  "),
    ];
    match (state.screen, view.jobs.get(state.job)) {
        (Screen::Tree, Some(job)) => spans.extend([
            Span::styled("jobs", theme::dim()),
            Span::styled(" › ", theme::faint()),
            Span::styled(job.id.as_str(), theme::accent()),
        ]),
        _ => spans.push(Span::styled("jobs", theme::header())),
    }
    let total = view.jobs.len();
    let noun = if total == 1 { "job" } else { "jobs" };
    spans.push(Span::styled(format!("   {total} {noun}"), theme::dim()));
    for status in ["running", "blocked", "failed"] {
        let count = view.jobs.iter().filter(|job| job.status == status).count();
        if count > 0 {
            let (glyph, style) = theme::status(status);
            spans.push(Span::styled("  ·  ", theme::faint()));
            spans.push(Span::styled(format!("{glyph} {count} {status}"), style));
        }
    }
    Line::from(spans)
}

fn footer_bar(screen: Screen) -> Line<'static> {
    let enter = match screen {
        Screen::List => "show runs",
        Screen::Tree => "back to jobs",
    };
    let mut spans = vec![Span::raw(" ")];
    for (key, label) in [("↑↓", "select"), ("enter", enter), ("q", "quit")] {
        spans.push(Span::styled(key, theme::key()));
        spans.push(Span::styled(format!(" {label}    "), theme::dim()));
    }
    Line::from(spans)
}

fn panel(title: Line<'_>) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(title)
        .padding(Padding::horizontal(1))
}

fn panel_title(text: &str) -> Line<'_> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(text, theme::accent()),
        Span::raw(" "),
    ])
}

fn empty_note(text: &str) -> Paragraph<'_> {
    Paragraph::new(vec![
        Line::default(),
        Line::styled(text, theme::dim().add_modifier(Modifier::ITALIC)),
    ])
    .centered()
}

/// A table in the house style: a coloured header, two spaces between columns
/// and a bar in front of the selected row.
fn styled_table<'a>(
    rows: Vec<Row<'a>>,
    widths: &[Constraint],
    header: &[&'a str],
    block: Block<'a>,
) -> Table<'a> {
    Table::new(rows, widths)
        .header(Row::new(header.iter().copied()).style(theme::header()))
        .block(block)
        .column_spacing(2)
        .row_highlight_style(theme::selected_row())
        .highlight_symbol(Text::from(Span::styled("▌ ", theme::accent())))
        .highlight_spacing(HighlightSpacing::Always)
}

/// The style of plain text on a row: bright when the row is selected.
fn plain(selected: bool) -> Style {
    if selected {
        theme::selected_text()
    } else {
        Style::new()
    }
}

fn status_span(status: &str) -> Span<'_> {
    let (glyph, style) = theme::status(status);
    Span::styled(format!("{glyph} {status}"), style)
}

/// `▶2 ✓5 ?1` with each count in the colour of its state, and zeros left out.
fn runs_line(runs: RunCounts) -> Line<'static> {
    let [open, finished, unconfirmed] = theme::run_counts();
    let parts = [
        ("▶", runs.open, open),
        ("✓", runs.finished, finished),
        ("?", runs.unconfirmed, unconfirmed),
    ];
    let spans: Vec<_> = parts
        .into_iter()
        .filter(|(_, count, _)| *count > 0)
        .map(|(glyph, count, style)| Span::styled(format!("{glyph}{count} "), style))
        .collect();
    if spans.is_empty() {
        Line::styled("–", theme::faint())
    } else {
        Line::from(spans)
    }
}

/// The token count, with the `~` of a total that can still grow in amber.
fn tokens_line(text: &str, selected: bool) -> Line<'_> {
    if text.is_empty() {
        return Line::styled("–", theme::faint());
    }
    match text.strip_suffix('~') {
        Some(number) => Line::from(vec![
            Span::styled(number, plain(selected)),
            Span::styled("~", theme::warning()),
        ]),
        None => Line::styled(text, plain(selected)),
    }
}

fn draw_jobs(frame: &mut Frame, area: Rect, view: &ViewModel, state: &State) {
    let block = panel(panel_title("jobs"));
    if view.jobs.is_empty() {
        let note =
            empty_note("No jobs yet. A job shows here as soon as `rex job create` opens it.");
        frame.render_widget(note.block(block), area);
        return;
    }
    let rows = view
        .jobs
        .iter()
        .enumerate()
        .map(|(index, job)| {
            let selected = index == state.job;
            Row::new([
                Cell::from(Span::styled(job.id.as_str(), theme::id())),
                Cell::from(status_span(&job.status)),
                Cell::from(Span::styled(
                    job.work_type.as_str(),
                    theme::work_type(&job.work_type),
                )),
                Cell::from(Span::styled(job.size.as_str(), theme::size(&job.size))),
                Cell::from(Span::styled(
                    job.consequence.as_str(),
                    theme::consequence(&job.consequence),
                )),
                Cell::from(Span::styled(job.languages.as_str(), theme::dim())),
                Cell::from(Span::styled(created_text(&job.created_at), theme::dim())),
                Cell::from(runs_line(job.runs)),
                Cell::from(tokens_line(&job.tokens, selected)),
                Cell::from(Span::styled(job.excerpt.as_str(), plain(selected))),
            ])
        })
        .collect();
    // The fixed columns fit in 110 terminal columns; the request takes the rest.
    let widths = [
        Constraint::Length(10),
        Constraint::Length(11),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Length(16),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Fill(1),
    ];
    let header = [
        "id",
        "status",
        "type",
        "size",
        "conseq",
        "languages",
        "created",
        "runs",
        "tokens",
        "request",
    ];
    let table = styled_table(rows, &widths, &header, block);
    let mut table_state = TableState::default().with_selected(Some(state.job));
    frame.render_stateful_widget(table, area, &mut table_state);
}

/// The selected job in full above its runs, so the tree screen says what the
/// runs worked on.
fn job_details(job: &JobListRow) -> Paragraph<'_> {
    let dot = || Span::styled("  ·  ", theme::faint());
    let facts = Line::from(vec![
        status_span(&job.status),
        dot(),
        Span::styled(job.work_type.as_str(), theme::work_type(&job.work_type)),
        dot(),
        Span::styled(job.size.as_str(), theme::size(&job.size)),
        dot(),
        Span::styled(
            format!("{} consequence", job.consequence),
            theme::consequence(&job.consequence),
        ),
        dot(),
        Span::styled(job.languages.as_str(), theme::dim()),
        dot(),
        Span::styled(created_text(&job.created_at), theme::dim()),
    ]);
    let request = Line::from(job.excerpt.as_str());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("job ", theme::accent()),
        Span::styled(job.id.as_str(), theme::id()),
        Span::raw(" "),
    ]);
    Paragraph::new(vec![facts, request])
        .wrap(Wrap { trim: true })
        .block(panel(title))
}

fn draw_tree(frame: &mut Frame, area: Rect, view: &ViewModel, state: &State) {
    let area = match view.jobs.get(state.job) {
        Some(job) => {
            // Two border lines, the facts line, and two lines of request.
            let [details, runs] =
                Layout::vertical([Constraint::Length(5), Constraint::Min(3)]).areas(area);
            frame.render_widget(job_details(job), details);
            runs
        }
        None => area,
    };
    let block = panel(panel_title("runs"));
    if view.tree.is_empty() {
        frame.render_widget(empty_note("No sub-agent runs yet.").block(block), area);
        return;
    }
    let depths: Vec<_> = view.tree.iter().map(|run| run.depth).collect();
    let rows = view
        .tree
        .iter()
        .zip(tree_guides(&depths))
        .enumerate()
        .map(|(index, (run, guide))| {
            let selected = index == state.tree_row;
            let role_style = if run.depth == 0 {
                theme::accent()
            } else {
                plain(selected)
            };
            let model = run.model.strip_prefix("claude-").unwrap_or(&run.model);
            let (glyph, state_style) = theme::run_state(run.state);
            let duration_style = if run.duration == "-" {
                theme::faint()
            } else {
                plain(selected)
            };
            let tag_job = run.tag_job.as_deref().map_or_else(Line::default, |job| {
                Line::styled(format!("⚠ {job}"), theme::warning())
            });
            Row::new([
                Cell::from(Line::from(vec![
                    Span::styled(guide, theme::faint()),
                    Span::styled(run.role.as_str(), role_style),
                ])),
                Cell::from(Span::styled(model, theme::model(model))),
                Cell::from(Span::styled(
                    run.effort.as_str(),
                    theme::effort(&run.effort),
                )),
                Cell::from(Span::styled(format!("{glyph} {}", run.state), state_style)),
                Cell::from(Span::styled(run.duration.as_str(), duration_style)),
                Cell::from(tokens_line(&run.tokens, selected)),
                Cell::from(tag_job),
            ])
        })
        .collect();
    let widths = [
        Constraint::Length(28),
        Constraint::Length(16),
        Constraint::Length(6),
        Constraint::Length(13),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Fill(1),
    ];
    let header = [
        "role", "model", "effort", "state", "time", "tokens", "tag job",
    ];
    let table = styled_table(rows, &widths, &header, block);
    let mut table_state = TableState::default().with_selected(Some(state.tree_row));
    frame.render_stateful_widget(table, area, &mut table_state);
}
