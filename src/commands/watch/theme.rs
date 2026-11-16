//! The colours and glyphs of `rex watch`: one style for each status, state, level,
//! work type, model and effort, so every screen colours the same value the same way.

use ratatui::style::{Color, Modifier, Style};

use super::nav::Level;

// Colours from the 256-colour palette, which nearly every terminal shows the
// same way. Full RGB colour is not used: some terminals, such as older macOS
// Terminal, turn it into the nearest wrong colour.
const BLUE: Color = Color::Indexed(75);
const SKY: Color = Color::Indexed(111);
const LAVENDER: Color = Color::Indexed(147);
const PURPLE: Color = Color::Indexed(141);
const ORCHID: Color = Color::Indexed(176);
const TEAL: Color = Color::Indexed(80);
const GREEN: Color = Color::Indexed(114);
const MINT: Color = Color::Indexed(78);
const AMBER: Color = Color::Indexed(214);
const PEACH: Color = Color::Indexed(215);
const ORANGE: Color = Color::Indexed(209);
const RED: Color = Color::Indexed(203);
const SLATE: Color = Color::Indexed(110);
const GREY: Color = Color::Indexed(245);
const DARK_GREY: Color = Color::Indexed(240);
const MUTED_BORDER: Color = Color::Indexed(60);
const SELECTED_BACKGROUND: Color = Color::Indexed(237);
const BRIGHT: Color = Color::Indexed(255);
const INK: Color = Color::Indexed(16);

fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

pub fn accent() -> Style {
    fg(SKY).add_modifier(Modifier::BOLD)
}

pub fn border() -> Style {
    fg(MUTED_BORDER)
}

pub fn dim() -> Style {
    fg(GREY)
}

pub fn faint() -> Style {
    fg(DARK_GREY)
}

pub fn header() -> Style {
    fg(LAVENDER).add_modifier(Modifier::BOLD)
}

pub fn id() -> Style {
    fg(SKY)
}

/// The badge at the left of the top bar.
pub fn badge() -> Style {
    Style::new().fg(INK).bg(SKY).add_modifier(Modifier::BOLD)
}

/// A key name in the footer.
pub fn key() -> Style {
    fg(SKY).add_modifier(Modifier::BOLD)
}

/// The selected row gets a background only, so the colours of its cells stay.
pub fn selected_row() -> Style {
    Style::new()
        .bg(SELECTED_BACKGROUND)
        .add_modifier(Modifier::BOLD)
}

/// Plain text on the selected row. Without it, a light terminal theme would
/// draw dark text on the dark selection background.
pub fn selected_text() -> Style {
    fg(BRIGHT)
}

/// The glyph and style of a job status as `rex jobs` prints it.
pub fn status(status: &str) -> (&'static str, Style) {
    match status {
        "running" => ("▶", fg(AMBER).add_modifier(Modifier::BOLD)),
        "created" => ("○", fg(SLATE)),
        "planned" => ("◇", fg(PURPLE)),
        "blocked" => ("■", fg(ORANGE).add_modifier(Modifier::BOLD)),
        "delivered" => ("✓", fg(GREEN)),
        "accepted" => ("★", fg(MINT).add_modifier(Modifier::BOLD)),
        "rejected" => ("✗", fg(RED)),
        "failed" => ("✗", fg(RED).add_modifier(Modifier::BOLD)),
        "analyzed" => ("◆", fg(ORCHID)),
        "cancelled" => ("–", dim()),
        _ => ("·", dim()),
    }
}

/// The glyph and style of a sub-agent run's state.
pub fn run_state(state: &str) -> (&'static str, Style) {
    match state {
        "open" => ("▶", fg(AMBER).add_modifier(Modifier::BOLD)),
        "done" => ("✓", fg(GREEN)),
        _ => ("?", dim()),
    }
}

/// One colour for each model family, whatever the version.
pub fn model(model: &str) -> Style {
    if model.contains("opus") {
        fg(ORCHID)
    } else if model.contains("sonnet") {
        fg(BLUE)
    } else if model.contains("haiku") {
        fg(GREEN)
    } else if model.contains("fable") {
        fg(PEACH)
    } else {
        faint()
    }
}

/// The glyph and style of a master tree's state.
pub fn tree_state(state: &str) -> (&'static str, Style) {
    match state {
        "running" => ("▶", fg(AMBER).add_modifier(Modifier::BOLD)),
        "paused" => ("‖", fg(SLATE)),
        "blocked" => ("■", fg(ORANGE).add_modifier(Modifier::BOLD)),
        "done" => ("✓", fg(GREEN)),
        "lost" => ("✗", fg(RED)),
        _ => ("·", dim()),
    }
}

/// The mark of a question that waits for the person.
pub fn question() -> (&'static str, Style) {
    ("?", fg(AMBER).add_modifier(Modifier::BOLD))
}

/// The mark in front of a tree or a job: filled while its session is live,
/// hollow once the session has ended, none when it comes from the store.
pub fn origin(origin: &str) -> (&'static str, Style) {
    match origin {
        "live" => ("●", fg(MINT)),
        "ended" => ("○", dim()),
        _ => (" ", dim()),
    }
}

/// One colour for each level, from the master down to a helper.
pub fn level(level: Level) -> Style {
    match level {
        Level::Master => fg(ORCHID).add_modifier(Modifier::BOLD),
        Level::Builder => fg(PURPLE).add_modifier(Modifier::BOLD),
        Level::Lead => fg(BLUE),
        Level::Worker => fg(TEAL),
        Level::Helper => fg(SLATE),
    }
}
