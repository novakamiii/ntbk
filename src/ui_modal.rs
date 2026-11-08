use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::{App, Screen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalState {
    NameEntry,
    Committed,
}

pub struct SaveModal {
    pub state: ModalState,
    pub input: String,
}

impl SaveModal {
    pub fn new() -> Self {
        Self {
            state: ModalState::NameEntry,
            input: String::new(),
        }
    }

    pub fn name_input(&self) -> &str {
        &self.input
    }
}

#[derive(PartialEq)]
enum ModalAction {
    Ignore,
    CommitName,
    EditAgain,
    Cancel,
    SaveToMenu,
    SaveAndQuit,
    DiscardToMenu,
    DiscardAndQuit,
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let Some(modal) = app.modal.as_ref() else {
        return;
    };
    let m = key.modifiers;
    let action = match (modal.state, key.code) {
        (ModalState::NameEntry, KeyCode::Enter) => ModalAction::CommitName,
        (ModalState::NameEntry, KeyCode::Esc) => ModalAction::Cancel,
        (ModalState::NameEntry, KeyCode::Backspace) => {
            if let Some(modal) = app.modal.as_mut() {
                modal.input.pop();
            }
            return;
        }
        (ModalState::NameEntry, KeyCode::Char(ch)) if !m.contains(KeyModifiers::CONTROL) => {
            if let Some(modal) = app.modal.as_mut() {
                modal.input.push(ch);
            }
            return;
        }
        (ModalState::Committed, KeyCode::Char('s')) if m == KeyModifiers::NONE => {
            ModalAction::SaveToMenu
        }
        (ModalState::Committed, KeyCode::Char('S'))
            if m.contains(KeyModifiers::SHIFT) && !m.contains(KeyModifiers::CONTROL) =>
        {
            ModalAction::SaveAndQuit
        }
        (ModalState::Committed, KeyCode::Char('c')) if m == KeyModifiers::NONE => {
            ModalAction::Cancel
        }
        (ModalState::Committed, KeyCode::Char('d')) if m == KeyModifiers::NONE => {
            ModalAction::DiscardToMenu
        }
        (ModalState::Committed, KeyCode::Char('D'))
            if m.contains(KeyModifiers::SHIFT) && !m.contains(KeyModifiers::CONTROL) =>
        {
            ModalAction::DiscardAndQuit
        }
        (ModalState::Committed, KeyCode::Enter) if m == KeyModifiers::NONE => {
            ModalAction::EditAgain
        }
        (_, KeyCode::Esc) => ModalAction::Cancel,
        _ => ModalAction::Ignore,
    };

    match action {
        ModalAction::Ignore => {}
        ModalAction::CommitName => {
            if let Some(modal) = app.modal.as_mut() {
                modal.state = ModalState::Committed;
            }
        }
        ModalAction::EditAgain => {
            if let Some(modal) = app.modal.as_mut() {
                modal.state = ModalState::NameEntry;
            }
        }
        ModalAction::Cancel => app.modal = None,
        ModalAction::SaveToMenu => {
            app.save_from_modal();
            app.goto_menu();
        }
        ModalAction::SaveAndQuit => {
            app.save_from_modal();
            app.editor = None;
            app.screen = Screen::Quit;
        }
        ModalAction::DiscardToMenu => app.goto_menu(),
        ModalAction::DiscardAndQuit => {
            app.editor = None;
            app.screen = Screen::Quit;
        }
    }
}

pub fn draw(app: &App, frame: &mut Frame) {
    let Some(modal) = app.modal.as_ref() else {
        return;
    };
    let area = frame.area();
    if area.width < 14 || area.height < 7 {
        return;
    }
    let w = 44u16.min(area.width - 4);
    let h = 7u16.min(area.height - 2);
    let rect = Rect {
        x: (area.width - w) / 2,
        y: (area.height - h) / 2,
        width: w,
        height: h,
    };
    frame.render_widget(Clear, rect);
    let block = Block::bordered().title(" File Name ");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let name_row = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: 1,
    };
    frame.render_widget(Paragraph::new(modal.input.as_str()), name_row);

    let (hint1, hint2) = match modal.state {
        ModalState::NameEntry => ("enter: confirm · esc: cancel", ""),
        ModalState::Committed => (
            "s save · S save + quit · c cancel",
            "d/D discard · enter: edit name",
        ),
    };
    let h1_row = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(hint1), h1_row);
    if !hint2.is_empty() {
        let h2_row = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(hint2), h2_row);
    }

    let cx = name_row.x + modal.input.chars().count() as u16;
    if cx < name_row.right() {
        frame.set_cursor_position((cx, name_row.y));
    }
}
