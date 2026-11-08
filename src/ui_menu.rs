use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

use crate::app::{App, Screen};

pub struct MenuState {
    pub selected: usize,
    pub expanded: bool,
    pub recents: Vec<String>,
}

impl MenuState {
    pub fn new(recents: Vec<String>) -> Self {
        Self {
            selected: 0,
            expanded: false,
            recents,
        }
    }

    /// Five most recent, or everything once expanded (spec 2.1).
    pub fn shown(&self) -> &[String] {
        if self.expanded {
            &self.recents
        } else {
            &self.recents[..self.recents.len().min(5)]
        }
    }

    pub fn show_toggle(&self) -> bool {
        self.expanded || self.recents.len() > 5
    }

    pub fn item_count(&self) -> usize {
        1 + self.shown().len() + usize::from(self.show_toggle())
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::Quit,
        KeyCode::Up | KeyCode::Char('k') => app.menu.selected = app.menu.selected.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.menu.item_count().saturating_sub(1);
            app.menu.selected = (app.menu.selected + 1).min(max);
        }
        KeyCode::Enter => activate(app),
        _ => {}
    }
}

fn activate(app: &mut App) {
    if app.menu.selected == 0 {
        app.open_new_note();
        return;
    }
    let toggle_index = 1 + app.menu.shown().len();
    if app.menu.show_toggle() && app.menu.selected == toggle_index {
        app.menu.expanded = !app.menu.expanded;
        // Keep the selection on the toggle entry so repeated Enter expands/collapses.
        app.menu.selected = 1 + app.menu.shown().len();
        return;
    }
    let idx = app.menu.selected - 1;
    if let Some(name) = app.menu.shown().get(idx).cloned() {
        app.open_note(name);
    }
}

pub fn draw(app: &App, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 9 || area.height < 3 {
        return;
    }
    let mut y = 1u16;
    let mut idx = 0usize;
    item(frame, area.width, y, "New Note", app.menu.selected == idx);
    y += 2;
    idx += 1;
    for name in app.menu.shown() {
        if y + 1 >= area.height.saturating_sub(1) {
            break;
        }
        item(frame, area.width, y, name, app.menu.selected == idx);
        y += 1;
        idx += 1;
    }
    if app.menu.show_toggle() && y + 1 < area.height.saturating_sub(1) {
        let label = if app.menu.expanded {
            "Show less"
        } else {
            "Show more"
        };
        item(frame, area.width, y, label, app.menu.selected == idx);
    }
    if let Some(msg) = app.status.as_deref() {
        let row = Rect {
            x: 0,
            y: area.height - 1,
            width: area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(msg.to_string()), row);
    }
}

fn item(frame: &mut Frame, width: u16, y: u16, label: &str, selected: bool) {
    let w = label.chars().count() as u16;
    let x = width.saturating_sub(w) / 2;
    let rect = Rect {
        x,
        y,
        width: w.max(1),
        height: 1,
    };
    let style = if selected {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    frame.render_widget(Paragraph::new(label.to_string()).style(style), rect);
}
