//! Draws the rows panel of `rex watch`: one row per tree, job or agent, with as
//! many columns as the width holds, or a note when the screen has no row.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, TableState, Wrap};

use super::data::WatchData;
use super::draw::panel;
use super::nav::{Level, Nav, Node};
use super::theme;
use super::view::{NOTHING_YET, RowText, ellipsis, text_width, tree_guides};

/// A column of the rows table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Name,
    State,
    Model,
    Time,
    Spend,
    Hit,
    Detail,
}

impl Column {
    fn header(self) -> &'static str {
        match self {
            Column::Name => "name",
            Column::State => "state",
            Column::Model => "model",
            Column::Time => "time",
            Column::Spend => "spend",
            Column::Hit => "hit",
            Column::Detail => "detail",
        }
    }
}

const COLUMN_SPACING: u16 = 1;
/// The name grows from the least to the most it wants before the detail gets room.
const NAME_LEAST: u16 = 22;
const NAME_MOST: u16 = 34;
const DETAIL_LEAST: u16 = 12;

/// The columns that fit in `inner` cells, with the width of each, in the order
/// shown. The name and the state always show; the others give way in the
/// order detail, hit, time, model, spend as the terminal narrows.
fn columns(inner: u16) -> Vec<(Column, u16)> {
    let fixed = [
        (Column::State, 12),
        (Column::Spend, 15),
        (Column::Model, 10),
        (Column::Time, 11),
        (Column::Hit, 4),
    ];
    let mut kept = Vec::new();
    let mut used = NAME_LEAST;
    for (column, width) in fixed {
        let needs = width + COLUMN_SPACING;
        if used + needs <= inner || column == Column::State {
            kept.push((column, width));
            used += needs;
        }
    }
    let mut room = inner.saturating_sub(used);
    let name_growth = room.min(NAME_MOST - NAME_LEAST);
    room -= name_growth;
    let mut name = NAME_LEAST + name_growth;
    if room >= DETAIL_LEAST + COLUMN_SPACING {
        kept.push((Column::Detail, room - COLUMN_SPACING));
    } else {
        name += room;
    }
    kept.push((Column::Name, name));
    let order = [
        Column::Name,
        Column::State,
        Column::Model,
        Column::Time,
        Column::Spend,
        Column::Hit,
        Column::Detail,
    ];
    order
        .into_iter()
        .filter_map(|column| kept.iter().find(|(kept, _)| *kept == column).copied())
        .collect()
}

fn state_span(row: &RowText) -> Span<'static> {
    let (glyph, style) = match row.level {
        Level::Master => theme::tree_state(&row.state),
        Level::Builder => theme::status(&row.state),
        _ => theme::run_state(&row.state),
    };
    Span::styled(format!("{glyph} {}", row.state), style)
}

/// The style of plain text on a row: bright when the row is selected.
fn plain(is_selected: bool) -> Style {
    if is_selected {
        theme::selected_text()
    } else {
        Style::new()
    }
}

/// The name cell: the origin mark, the guide, the level and the name, with `?`
/// when a question waits, cut to `width`.
fn name_cell(text: &RowText, guide: String, width: u16, is_selected: bool) -> Line<'static> {
    let (mark, mark_style) = theme::origin(text.origin);
    let level = text.level.name();
    let waiting = if text.waiting > 0 { " ?" } else { "" };
    let room = usize::from(width).saturating_sub(
        text_width(mark) + 1 + text_width(&guide) + level.len() + 1 + waiting.len(),
    );
    let mut spans = vec![
        Span::styled(format!("{mark} "), mark_style),
        Span::styled(guide, theme::faint()),
        Span::styled(format!("{level} "), theme::level(text.level)),
        Span::styled(ellipsis(&text.name, room), plain(is_selected)),
    ];
    if !waiting.is_empty() {
        spans.push(Span::styled(waiting, theme::question().1));
    }
    Line::from(spans)
}

fn cell(
    column: Column,
    width: u16,
    text: &RowText,
    guide: &str,
    is_selected: bool,
) -> Cell<'static> {
    let fit = |value: &str| ellipsis(value, usize::from(width));
    match column {
        Column::Name => Cell::from(name_cell(text, guide.to_owned(), width, is_selected)),
        Column::State => Cell::from(state_span(text)),
        Column::Model => Cell::from(Span::styled(fit(&text.model), theme::model(&text.model))),
        Column::Time => Cell::from(Span::styled(fit(&text.time), theme::dim())),
        Column::Spend => Cell::from(Span::styled(fit(&text.cost), plain(is_selected))),
        Column::Hit => Cell::from(Span::styled(fit(&text.hit), plain(is_selected))),
        Column::Detail => Cell::from(Span::styled(fit(&text.detail), plain(is_selected))),
    }
}

pub fn draw_rows(frame: &mut Frame, area: Rect, data: &WatchData, nav: &Nav) {
    let title = match nav.current() {
        None => "trees and jobs",
        Some(Node::Tree(_)) => "jobs",
        Some(_) => "agents",
    };
    let block = panel(Line::from(vec![
        Span::raw(" "),
        Span::styled(title, theme::accent()),
        Span::raw(" "),
    ]));
    let rows = data.rows(nav.current());
    if rows.is_empty() {
        let note = match nav.current() {
            None => NOTHING_YET,
            Some(Node::Tree(_)) => "The master has built no job yet.",
            Some(_) => "No agent started below this one.",
        };
        let paragraph = Paragraph::new(vec![Line::styled(
            note,
            theme::dim().add_modifier(Modifier::ITALIC),
        )])
        .centered()
        .wrap(Wrap { trim: true });
        frame.render_widget(paragraph.block(block), area);
        return;
    }
    let selected = nav
        .selected_node()
        .and_then(|node| rows.iter().position(|row| &row.node == node));
    // Two borders, two of padding and the selection mark.
    let columns = columns(area.width.saturating_sub(6));
    let depths: Vec<usize> = rows.iter().map(|row| row.indent).collect();
    let table_rows: Vec<Row> = rows
        .iter()
        .zip(tree_guides(&depths))
        .enumerate()
        .filter_map(|(index, (row, guide))| {
            let text = data.row_text(&row.node)?;
            let is_selected = selected == Some(index);
            let cells = columns
                .iter()
                .map(|(column, width)| cell(*column, *width, &text, &guide, is_selected));
            Some(Row::new(cells.collect::<Vec<_>>()))
        })
        .collect();
    let widths: Vec<Constraint> = columns
        .iter()
        .map(|(_, width)| Constraint::Length(*width))
        .collect();
    let header: Vec<&str> = columns.iter().map(|(column, _)| column.header()).collect();
    let table = Table::new(table_rows, widths)
        .header(Row::new(header).style(theme::header()))
        .block(block)
        .column_spacing(COLUMN_SPACING)
        .row_highlight_style(theme::selected_row())
        .highlight_symbol(Text::from(Span::styled("▌ ", theme::accent())))
        .highlight_spacing(HighlightSpacing::Always);
    let mut table_state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut table_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(inner: u16) -> Vec<Column> {
        columns(inner)
            .into_iter()
            .map(|(column, _)| column)
            .collect()
    }

    #[test]
    fn columns_give_way_from_the_detail_down_and_fill_the_width() {
        use Column::*;
        assert_eq!(shown(114), [Name, State, Model, Time, Spend, Hit, Detail]);
        assert_eq!(shown(94), [Name, State, Model, Time, Spend, Hit]);
        assert_eq!(shown(54), [Name, State, Spend]);
        assert_eq!(shown(20), [Name, State]);
        for inner in [114, 94, 54] {
            let used: u16 = columns(inner)
                .iter()
                .map(|(_, width)| width + COLUMN_SPACING)
                .sum();
            assert_eq!(
                used - COLUMN_SPACING,
                inner,
                "at {inner} the columns fill the row"
            );
        }
    }
}
