use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::io_index;
use crate::io_notes;
use crate::ui_editor::{self, EditorState};
use crate::ui_hud;
use crate::ui_menu::{self, MenuState};
use crate::ui_modal::{self, SaveModal};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Screen {
    Menu,
    Editor,
    Quit,
}

pub struct App {
    pub screen: Screen,
    pub base: PathBuf,
    pub menu: MenuState,
    pub editor: Option<EditorState>,
    pub modal: Option<SaveModal>,
    pub status: Option<String>,
    /// Last drawn frame area; capacity for the typewriter line rules derives from it.
    pub area: Rect,
    bell: bool,
}

impl App {
    pub fn new(base: PathBuf) -> Self {
        let recents = io_index::load(&base);
        Self {
            screen: Screen::Menu,
            base,
            menu: MenuState::new(recents),
            editor: None,
            modal: None,
            status: None,
            area: Rect::default(),
            bell: false,
        }
    }

    pub fn quitting(&self) -> bool {
        self.screen == Screen::Quit
    }

    pub fn ring_bell(&mut self) {
        self.bell = true;
    }

    /// Consume a pending typewriter ping; the main loop writes the BEL byte.
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.bell)
    }

    pub fn open_new_note(&mut self) {
        self.editor = Some(EditorState::blank());
        self.modal = None;
        self.screen = Screen::Editor;
    }

    pub fn open_note(&mut self, name: String) {
        match io_notes::load_note(&self.base, &name) {
            Ok(text) => {
                self.editor = Some(EditorState::from_text(&text, Some(name)));
                self.modal = None;
                self.screen = Screen::Editor;
            }
            Err(e) => self.status = Some(format!("could not open {name}: {e}")),
        }
    }

    /// Named notes: Escape and Ctrl+S save silently (spec 2.4, 2.5).
    pub fn save_silent(&mut self) {
        let Some(name) = self.editor.as_ref().and_then(|e| e.name.clone()) else {
            return;
        };
        let content = self
            .editor
            .as_ref()
            .map(|e| e.buffer.text())
            .unwrap_or_default();
        match io_notes::save_note(&self.base, &name, &content) {
            Ok(_) => {
                if let Err(e) = io_index::record(&self.base, &name) {
                    self.status = Some(format!("could not update recents: {e}"));
                }
            }
            Err(e) => self.status = Some(format!("could not save {name}: {e}")),
        }
    }

    /// Save from the modal: empty input falls back to the timestamped name (spec 2.4).
    pub fn save_from_modal(&mut self) {
        let raw = self
            .modal
            .as_ref()
            .map(|m| m.name_input().to_string())
            .unwrap_or_default();
        let name = io_notes::resolve_filename(&raw);
        if let Some(ed) = self.editor.as_mut() {
            let content = ed.buffer.text();
            match io_notes::save_note(&self.base, &name, &content) {
                Ok(_) => {
                    ed.name = Some(name.clone());
                    if let Err(e) = io_index::record(&self.base, &name) {
                        self.status = Some(format!("could not update recents: {e}"));
                    }
                }
                Err(e) => self.status = Some(format!("could not save {name}: {e}")),
            }
        }
    }

    pub fn goto_menu(&mut self) {
        self.menu = MenuState::new(io_index::load(&self.base));
        self.editor = None;
        self.modal = None;
        self.screen = Screen::Menu;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.status = None;
        match self.screen {
            Screen::Menu => ui_menu::handle_key(self, key),
            Screen::Editor => {
                if self.modal.is_some() {
                    ui_modal::handle_key(self, key);
                } else {
                    ui_editor::handle_key(self, key);
                }
            }
            Screen::Quit => {}
        }
    }

    pub fn handle_paste(&mut self, text: String) {
        if self.screen == Screen::Editor && self.modal.is_none() {
            ui_editor::handle_paste(self, &text);
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        self.area = frame.area();
        match self.screen {
            Screen::Menu => ui_menu::draw(self, frame),
            Screen::Editor => {
                ui_editor::draw(self, frame);
                ui_hud::draw(self, frame);
                if self.modal.is_some() {
                    ui_modal::draw(self, frame);
                }
            }
            Screen::Quit => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_modal::ModalState;
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;
    use std::path::PathBuf;

    fn temp_base(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ntbk-app-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn type_str(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    #[test]
    fn escape_on_unnamed_note_opens_modal_in_name_entry() {
        let mut app = App::new(temp_base("esc-modal"));
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        let modal = app.modal.as_ref().unwrap();
        assert!(matches!(modal.state, ModalState::NameEntry));
        // Still in the editor; nothing saved.
        assert_eq!(app.screen, Screen::Editor);
        assert!(app.editor.is_some());
    }

    #[test]
    fn escape_on_named_note_saves_silently_and_returns_to_menu() {
        let base = temp_base("esc-named");
        fs::write(base.join("existing.md"), "hello").unwrap();
        let mut app = App::new(base.clone());
        app.open_note("existing.md".into());
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Menu);
        assert_eq!(
            fs::read_to_string(base.join("existing.md")).unwrap(),
            "hello"
        );
        assert_eq!(io_index::load(&base), vec!["existing.md".to_string()]);
    }

    #[test]
    fn modal_typing_goes_to_name_field_action_keys_do_not_fire() {
        let mut app = App::new(temp_base("modal-typing"));
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        type_str(&mut app, "sd"); // these are filename characters in name-entry state
        let modal = app.modal.as_ref().unwrap();
        assert_eq!(modal.input, "sd");
        assert_eq!(app.screen, Screen::Editor);
        assert!(fs::read_dir(app.base.clone()).unwrap().count() <= 1); // no stray save
    }

    #[test]
    fn modal_s_saves_and_returns_to_menu() {
        let base = temp_base("modal-s");
        let mut app = App::new(base.clone());
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        type_str(&mut app, "demo");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // commit name
        app.handle_key(key(KeyCode::Char('s'), KeyModifiers::NONE)); // save + menu
        assert_eq!(app.screen, Screen::Menu);
        assert_eq!(fs::read_to_string(base.join("demo.txt")).unwrap(), "");
        assert_eq!(io_index::load(&base), vec!["demo.txt".to_string()]);
    }

    #[test]
    fn modal_shift_s_saves_and_quits() {
        let base = temp_base("modal-shift-s");
        let mut app = App::new(base.clone());
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // empty name -> committed
        app.handle_key(key(KeyCode::Char('S'), KeyModifiers::SHIFT));
        assert_eq!(app.screen, Screen::Quit);
        // Empty name fell back to the timestamped file.
        assert!(
            fs::read_dir(&base)
                .unwrap()
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().ends_with(".txt"))
        );
    }

    #[test]
    fn modal_c_cancels_back_to_editor_with_content() {
        let mut app = App::new(temp_base("modal-c"));
        app.open_new_note();
        app.area = Rect::new(0, 0, 80, 24); // capacity derives from the drawn frame
        type_str(&mut app, "kept");
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // commit empty name -> action keys live
        app.handle_key(key(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(app.modal.is_none());
        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(app.editor.as_ref().unwrap().buffer.text(), "kept");
    }

    #[test]
    fn modal_d_discards_to_menu_shift_d_quits() {
        let base = temp_base("modal-d");
        let mut app = App::new(base.clone());
        app.open_new_note();
        app.area = Rect::new(0, 0, 80, 24);
        type_str(&mut app, "gone");
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Menu);
        assert!(app.editor.is_none());

        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(app.screen, Screen::Quit);
    }

    #[test]
    fn modal_escape_cancels_from_both_states() {
        let mut app = App::new(temp_base("modal-esc"));
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.modal.is_none());
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // committed state
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.modal.is_none());
        assert_eq!(app.screen, Screen::Editor);
    }

    #[test]
    fn modal_enter_again_reenters_name_editing() {
        let mut app = App::new(temp_base("modal-reedit"));
        app.open_new_note();
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        type_str(&mut app, "typo");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // back into the field
        assert!(matches!(
            app.modal.as_ref().unwrap().state,
            ModalState::NameEntry
        ));
        type_str(&mut app, "2");
        assert_eq!(app.modal.as_ref().unwrap().input, "typo2");
    }

    #[test]
    fn menu_enter_opens_new_note_and_q_quits() {
        let mut app = App::new(temp_base("menu-basic"));
        app.handle_key(key(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.menu.selected, 0); // nothing to move to
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Editor);
        app.goto_menu();
        app.handle_key(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Quit);
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Quit);
    }

    #[test]
    fn menu_show_more_expands_full_list() {
        let base = temp_base("menu-expand");
        for i in 0..6 {
            io_index::record(&base, &format!("n{i}.txt")).unwrap();
        }
        let mut app = App::new(base);
        assert_eq!(app.menu.item_count(), 1 + 5 + 1);
        for _ in 0..6 {
            app.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
        }
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // Show more
        assert!(app.menu.expanded);
        assert_eq!(app.menu.item_count(), 1 + 6 + 1);
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)); // Show less
        assert!(!app.menu.expanded);
    }

    #[test]
    fn menu_enter_on_recent_opens_note() {
        let base = temp_base("menu-open");
        fs::write(base.join("old.txt"), "body").unwrap();
        io_index::record(&base, "old.txt").unwrap();
        let mut app = App::new(base);
        app.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(app.editor.as_ref().unwrap().buffer.text(), "body");
        assert_eq!(
            app.editor.as_ref().unwrap().name.as_deref(),
            Some("old.txt")
        );
    }

    #[test]
    fn menu_enter_on_missing_recent_shows_error_not_crash() {
        let base = temp_base("menu-missing");
        io_index::record(&base, "ghost.txt").unwrap();
        let mut app = App::new(base);
        app.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.screen, Screen::Menu);
        assert!(app.status.as_deref().unwrap().contains("could not open"));
    }
}
