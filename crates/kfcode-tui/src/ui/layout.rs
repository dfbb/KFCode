//! Convenience wrappers around ratatui layout primitives.
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Split `area` horizontally according to the given constraints.
pub fn horizontal_split(area: Rect, constraints: Vec<Constraint>) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

/// Split `area` vertically according to the given constraints.
pub fn vertical_split(area: Rect, constraints: Vec<Constraint>) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

/// Return a centered rect of the given `width` and `height` within `area`.
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height as u16) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height as u16) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width as u16) / 2),
            Constraint::Length(width),
            Constraint::Percentage((100 - width as u16) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Return the bottom `height` rows of `area` as a footer rect.
pub fn footer(area: Rect, height: u16) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(height)])
        .split(area);
    layout[1]
}

/// Return the top `height` rows of `area` as a header rect.
pub fn header(area: Rect, height: u16) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(height), Constraint::Min(0)])
        .split(area);
    layout[0]
}

/// Split `area` into a left sidebar of `width` columns and the remaining content area.
pub fn sidebar_left(area: Rect, width: u16) -> (Rect, Rect) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(width), Constraint::Min(0)])
        .split(area);
    (layout[0], layout[1])
}

/// Split `area` into the content area and a right sidebar of `width` columns.
pub fn sidebar_right(area: Rect, width: u16) -> (Rect, Rect) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(width)])
        .split(area);
    (layout[0], layout[1])
}
