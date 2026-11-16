//! Draws the screen of `rex watch` from its plain values, with the colours of
//! [`super::theme`]. It only lays the data and the place out and reads nothing.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph, Wrap};

use super::data::WatchData;
use super::facts::{answer_command, options_text, waiting_questions};
use super::nav::{Nav, Node};
use super::rows::draw_rows;
use super::theme;
use super::view::{breadcrumb, counted, ellipsis, fit_crumbs, rail_text, text_width};

/// The keys and what they do, for the `?` screen.
const KEYS: [(&str, &str); 7] = [
    ("↑ ↓  k j", "move the selection"),
    ("enter → l", "go into the selected row"),
    ("esc ← h ⌫", "go up one level"),
    ("home g", "go to the home screen"),
    ("?", "show or hide these keys"),
    ("q  ctrl-c", "quit"),
    (
        "",
        "Answer a question in its session with the command shown.",
    ),
];

/// The width of a label in the facts panel, with the space after it.
const LABEL_WIDTH: usize = 15;

/// `age_seconds` is the time since the data was last read.
pub fn draw(frame: &mut Frame, data: &WatchData, nav: &Nav, age_seconds: u64) {
    let [top, rail, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(top_bar(data, nav, top.width), top);
    frame.render_widget(rail_bar(data, nav), rail);
    let rows_area = match nav.current() {
        Some(node) => {
            let lines = facts_lines(data, node, body.width);
            let (facts_height, rows_height) =
                split_heights(body.height, lines.len(), data.rows(Some(node)).len());
            let [facts_area, rows_area] = Layout::vertical([
                Constraint::Length(facts_height),
                Constraint::Length(rows_height),
            ])
            .areas(body);
            let room = usize::from(facts_height.saturating_sub(2));
            frame.render_widget(
                Paragraph::new(cut_lines(lines, room)).block(panel(node_title(data, node))),
                facts_area,
            );
            rows_area
        }
        None => body,
    };
    draw_rows(frame, rows_area, data, nav);
    frame.render_widget(footer_bar(data, nav, age_seconds, footer.width), footer);
    if nav.is_help_shown {
        draw_help(frame, frame.area());
    }
}

/// The heights of the facts panel and the rows panel in a body `body` lines
/// high. The facts take what they need while the rows still fit; on a short
/// terminal the rows keep at least half, so the person can still move.
fn split_heights(body: u16, fact_lines: usize, rows: usize) -> (u16, u16) {
    let as_height = |count: usize| u16::try_from(count).unwrap_or(u16::MAX);
    // Two borders and the header; a screen with no row shows a note instead.
    let rows_need = as_height(rows.max(1)).saturating_add(3);
    let facts_need = as_height(fact_lines).saturating_add(2);
    if facts_need.saturating_add(rows_need) <= body {
        return (facts_need, body - facts_need);
    }
    let rows_height = rows_need.min((body / 2).max(4)).min(body);
    (body - rows_height, rows_height)
}

/// The first lines that fit in `room`, the last of them saying how many more
/// there are when some do not.
fn cut_lines(mut lines: Vec<Line<'static>>, room: usize) -> Vec<Line<'static>> {
    if lines.len() <= room {
        return lines;
    }
    let kept = room.saturating_sub(1);
    let hidden = lines.len() - kept;
    lines.truncate(kept);
    if room > 0 {
        lines.push(Line::styled(
            format!("… {hidden} more lines; a taller terminal shows them"),
            theme::faint(),
        ));
    }
    lines
}

/// The badge, then the path from home to the node in view, the last part bright.
fn top_bar(data: &WatchData, nav: &Nav, width: u16) -> Line<'static> {
    let badge = " ◆ rex watch ";
    let mut spans = vec![Span::styled(badge, theme::badge()), Span::raw("  ")];
    let room = usize::from(width).saturating_sub(text_width(badge) + 2);
    let crumbs = fit_crumbs(&breadcrumb(data, nav), room);
    let last = crumbs.len().saturating_sub(1);
    for (index, crumb) in crumbs.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" › ", theme::faint()));
        }
        let style = if index == last {
            theme::accent()
        } else {
            theme::dim()
        };
        spans.push(Span::styled(crumb, style));
    }
    Line::from(spans)
}

/// How deep the node in view is, then the levels of the run with the current
/// one marked. The depth comes first, so a narrow terminal cuts the rail and
/// not the depth.
fn rail_bar(data: &WatchData, nav: &Nav) -> Line<'static> {
    let Some(at) = nav.current() else {
        let waiting = data.waiting_count();
        let mut spans = vec![Span::styled(
            format!(
                " {}  ·  {}",
                counted(data.trees.len(), "tree", "trees"),
                counted(data.jobs.len(), "job", "jobs")
            ),
            theme::dim(),
        )];
        if waiting > 0 {
            let (glyph, style) = theme::question();
            spans.push(Span::styled(
                format!("  ·  {glyph} {waiting} waiting"),
                style,
            ));
        }
        return Line::from(spans);
    };
    let rail = rail_text(data, nav);
    let pieces: Vec<&str> = rail.split(" › ").collect();
    let depth = pieces
        .iter()
        .position(|piece| piece.starts_with('['))
        .map_or(0, |index| index + 1);
    let mut spans = vec![Span::styled(
        format!(" level {depth} of {}   ", pieces.len()),
        theme::dim(),
    )];
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" › ", theme::faint()));
        }
        let style = if piece.starts_with('[') {
            theme::level(data.level(at)).add_modifier(Modifier::REVERSED)
        } else {
            theme::faint()
        };
        spans.push(Span::styled((*piece).to_owned(), style));
    }
    Line::from(spans)
}

/// The keys of the screen in view with what each does, then the source and
/// the age of the data. A narrow terminal drops what the keys do first, then
/// the source.
fn footer_bar(data: &WatchData, nav: &Nav, age_seconds: u64, width: u16) -> Line<'static> {
    let mut keys = Vec::new();
    if !data.rows(nav.current()).is_empty() {
        keys.push(("↑↓", "move"));
    }
    if nav.selected_node().is_some() {
        keys.push(("enter", "open"));
    }
    if nav.current().is_some() {
        keys.extend([("esc", "up"), ("g", "home")]);
    }
    keys.extend([("?", "keys"), ("q", "quit")]);
    let source = format!(
        " source {}  ·  read {age_seconds}s ago",
        data.sources.text()
    );
    let line = |with_labels: bool, with_source: bool| {
        let mut spans = vec![Span::raw(" ")];
        for (key, label) in &keys {
            spans.push(Span::styled(*key, theme::key()));
            let after = if with_labels {
                format!(" {label}   ")
            } else {
                " ".to_owned()
            };
            spans.push(Span::styled(after, theme::dim()));
        }
        if with_source {
            spans.push(Span::styled(source.clone(), theme::faint()));
        }
        Line::from(spans)
    };
    [(true, true), (false, true), (false, false)]
        .into_iter()
        .map(|(with_labels, with_source)| line(with_labels, with_source))
        .find(|line| line.width() <= usize::from(width))
        .unwrap_or_else(|| line(false, false))
}

/// A rounded panel with `title`, as every panel of the screen has.
pub fn panel(title: Line<'_>) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(title)
        .padding(Padding::horizontal(1))
}

/// `master t0geh4lh3i`, `builder 0geh5fafkb` or `lead engineer-lead`, in the level's colour.
fn node_title(data: &WatchData, node: &Node) -> Line<'static> {
    let level = data.level(node);
    let name = data.row_text(node).map_or_else(String::new, |row| row.name);
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(level.name(), theme::level(level)),
        Span::raw(" "),
    ];
    if !name.is_empty() {
        spans.extend([
            Span::styled(ellipsis(&name, 40), theme::id()),
            Span::raw(" "),
        ]);
    }
    Line::from(spans)
}

fn fact_line(label: &str, label_style: Style, value: String, value_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<LABEL_WIDTH$}"), label_style),
        Span::styled(value, value_style),
    ])
}

/// The facts of the node, each cut to one line. The waiting questions come
/// right after the first fact, so a short terminal still shows them.
fn facts_lines(data: &WatchData, node: &Node, width: u16) -> Vec<Line<'static>> {
    let value_width = usize::from(width).saturating_sub(4 + LABEL_WIDTH);
    let mut facts = data.facts(node).into_iter();
    let mut lines: Vec<Line> = facts
        .next()
        .map(|(label, value)| {
            fact_line(
                label,
                theme::dim(),
                ellipsis(&value, value_width),
                Style::new(),
            )
        })
        .into_iter()
        .collect();
    let (glyph, style) = theme::question();
    for question in waiting_questions(data, node) {
        lines.extend([
            fact_line(
                &format!("{glyph} waiting"),
                style,
                ellipsis(&question.question, value_width),
                Style::new(),
            ),
            fact_line(
                "  options",
                theme::dim(),
                ellipsis(&options_text(question), value_width),
                Style::new(),
            ),
            fact_line(
                "  answer",
                theme::dim(),
                ellipsis(&answer_command(question), value_width),
                theme::key(),
            ),
        ]);
    }
    lines.extend(facts.map(|(label, value)| {
        fact_line(
            label,
            theme::dim(),
            ellipsis(&value, value_width),
            Style::new(),
        )
    }));
    lines
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let width = area.width.min(76);
    let lines: Vec<Line> = KEYS
        .iter()
        .map(|(key, what)| {
            Line::from(vec![
                Span::styled(format!("{key:<12}"), theme::key()),
                Span::raw(*what),
            ])
        })
        .collect();
    // The borders and the padding take four columns and two lines; a line
    // wider than the rest wraps onto as many lines as it needs.
    let inner = usize::from(width.saturating_sub(4)).max(1);
    let text_height: usize = lines
        .iter()
        .map(|line| line.width().div_ceil(inner).max(1))
        .sum();
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let height = area.height.min(
        u16::try_from(text_height)
            .unwrap_or(u16::MAX)
            .saturating_add(2),
    );
    let [_, middle, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, popup, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(middle);
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("keys", theme::accent()),
        Span::raw(" "),
    ]);
    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph.block(panel(title)), popup);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_terminal_keeps_half_the_body_for_the_rows() {
        assert_eq!(split_heights(36, 13, 4), (15, 21), "all fits");
        assert_eq!(split_heights(11, 13, 4), (6, 5));
        assert_eq!(split_heights(11, 3, 0), (5, 6), "a screen with no row");
        assert_eq!(split_heights(3, 13, 4), (0, 3));
    }

    #[test]
    fn cut_lines_says_how_many_lines_it_hid() {
        let lines: Vec<Line> = (0..5).map(|n| Line::raw(n.to_string())).collect();
        assert_eq!(cut_lines(lines.clone(), 5).len(), 5);
        let cut = cut_lines(lines, 3);
        assert_eq!(cut.len(), 3);
        assert_eq!(
            cut[2].to_string(),
            "… 3 more lines; a taller terminal shows them"
        );
    }
}
