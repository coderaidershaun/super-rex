//! The colours and glyphs of `rex watch`: one style for each status, state,
//! work type, model and effort, so both screens colour the same value the same way.

use ratatui::style::{Color, Modifier, Style};

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

pub fn warning() -> Style {
    fg(AMBER)
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
        "finished" => ("✓", fg(GREEN)),
        _ => ("?", dim()),
    }
}

pub fn work_type(work_type: &str) -> Style {
    match work_type {
        "feature" => fg(BLUE),
        "fix" => fg(ORANGE),
        "refactor" => fg(PURPLE),
        "research" => fg(TEAL),
        _ => dim(),
    }
}

/// Green to red as the cost of a mistake rises.
pub fn consequence(consequence: &str) -> Style {
    match consequence {
        "low" => fg(GREEN),
        "medium" => fg(AMBER),
        "high" => fg(RED).add_modifier(Modifier::BOLD),
        _ => dim(),
    }
}

/// Cool to warm as the size rises. Not green to red: a large job is not a fault.
pub fn size(size: &str) -> Style {
    match size {
        "small" => fg(SLATE),
        "medium" => fg(BLUE),
        "large" => fg(PURPLE),
        _ => dim(),
    }
}

/// Cool to warm as the effort rises.
pub fn effort(effort: &str) -> Style {
    match effort {
        "low" => fg(SLATE),
        "medium" => fg(BLUE),
        "high" => fg(PURPLE),
        "xhigh" => fg(ORCHID),
        "max" => fg(RED),
        _ => faint(),
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

/// The styles of the open, finished and unconfirmed run counts.
pub fn run_counts() -> [Style; 3] {
    [fg(AMBER), fg(GREEN), dim()]
}
