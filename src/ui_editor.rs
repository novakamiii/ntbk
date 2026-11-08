use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::ui_modal::SaveModal;

pub struct EditorBuffer {
    lines: Vec<String>,
    /// (line, col); col counted in chars.
    cursor: (usize, usize),
    anchor: Option<(usize, usize)>,
}

pub struct EditorState {
    pub buffer: EditorBuffer,
    pub name: Option<String>,
    pub session_start: Instant,
}

impl EditorState {
    pub fn blank() -> Self {
        Self {
            buffer: EditorBuffer::blank(),
            name: None,
            session_start: Instant::now(),
        }
    }

    pub fn from_text(text: &str, name: Option<String>) -> Self {
        Self {
            buffer: EditorBuffer::from_text(text),
            name,
            session_start: Instant::now(),
        }
    }

    /// Typewriter margin (spec 2.2): inset from each edge, roughly a paper margin.
    pub fn margin(area_width: u16) -> u16 {
        (area_width / 8).clamp(2, 16)
    }

    /// Line capacity: whatever fits between the margins.
    pub fn capacity(&self, area_width: u16) -> usize {
        let m = Self::margin(area_width) as usize;
        (area_width as usize).saturating_sub(2 * m).max(1)
    }
}

impl EditorBuffer {
    pub fn blank() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: (0, 0),
            anchor: None,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n')
                .map(|l| {
                    l.strip_suffix('\r')
                        .map(str::to_string)
                        .unwrap_or_else(|| l.to_string())
                })
                .collect()
        };
        let last = lines.len() - 1;
        let cursor = (last, lines[last].chars().count());
        Self {
            lines,
            cursor,
            anchor: None,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.anchor.map(|_| self.normalized())
    }

    fn line_len(&self, l: usize) -> usize {
        self.lines[l].chars().count()
    }

    fn col_to_byte(&self, l: usize, col: usize) -> usize {
        self.lines[l]
            .char_indices()
            .nth(col)
            .map(|(b, _)| b)
            .unwrap_or(self.lines[l].len())
    }

    /// Typewriter wrap (spec 2.2): when the line is full the app continues on the next
    /// line by itself and rings once. Returns true when a wrap (ping) happened.
    pub fn insert_char(&mut self, ch: char, capacity: usize) -> bool {
        let (l, _) = self.cursor;
        let mut rang = false;
        if self.line_len(l) >= capacity {
            self.newline();
            rang = true;
        }
        let (l, c) = self.cursor;
        let b = self.col_to_byte(l, c);
        self.lines[l].insert(b, ch);
        self.cursor = (l, c + 1);
        rang
    }

    pub fn newline(&mut self) {
        let (l, c) = self.cursor;
        let b = self.col_to_byte(l, c);
        let rest = self.lines[l].split_off(b);
        self.lines.insert(l + 1, rest);
        self.cursor = (l + 1, 0);
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        let (l, c) = self.cursor;
        if c > 0 {
            let b = self.col_to_byte(l, c);
            let pb = self.col_to_byte(l, c - 1);
            self.lines[l].replace_range(pb..b, "");
            self.cursor = (l, c - 1);
        } else if l > 0 {
            // Structural merge; the wrap rule only governs live typing at the margin.
            let prev_len = self.line_len(l - 1);
            let cur = self.lines.remove(l);
            self.lines[l - 1].push_str(&cur);
            self.cursor = (l - 1, prev_len);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        let (l, c) = self.cursor;
        if c < self.line_len(l) {
            let b = self.col_to_byte(l, c);
            let nb = self.col_to_byte(l, c + 1);
            self.lines[l].replace_range(b..nb, "");
        } else if l + 1 < self.lines.len() {
            let next = self.lines.remove(l + 1);
            self.lines[l].push_str(&next);
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        self.set_anchor(extend);
        let (l, c) = self.cursor;
        if c > 0 {
            self.cursor = (l, c - 1);
        } else if l > 0 {
            self.cursor = (l - 1, self.line_len(l - 1));
        }
    }

    pub fn move_right(&mut self, extend: bool) {
        self.set_anchor(extend);
        let (l, c) = self.cursor;
        if c < self.line_len(l) {
            self.cursor = (l, c + 1);
        } else if l + 1 < self.lines.len() {
            self.cursor = (l + 1, 0);
        }
    }

    pub fn move_up(&mut self, extend: bool) {
        self.set_anchor(extend);
        let (l, c) = self.cursor;
        if l > 0 {
            self.cursor = (l - 1, c.min(self.line_len(l - 1)));
        }
    }

    pub fn move_down(&mut self, extend: bool) {
        self.set_anchor(extend);
        let (l, c) = self.cursor;
        if l + 1 < self.lines.len() {
            self.cursor = (l + 1, c.min(self.line_len(l + 1)));
        }
    }

    fn set_anchor(&mut self, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
    }

    pub fn select_all(&mut self) {
        let last = self.lines.len() - 1;
        self.anchor = Some((0, 0));
        self.cursor = (last, self.line_len(last));
    }

    fn normalized(&self) -> ((usize, usize), (usize, usize)) {
        let a = self.anchor.unwrap_or(self.cursor);
        let b = self.cursor;
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn selected_text(&self) -> Option<String> {
        let ((sl, sc), (el, ec)) = self.selection()?;
        if (sl, sc) == (el, ec) {
            return None;
        }
        if sl == el {
            return Some(
                self.lines[sl][self.col_to_byte(sl, sc)..self.col_to_byte(sl, ec)].to_string(),
            );
        }
        let mut parts = vec![self.lines[sl][self.col_to_byte(sl, sc)..].to_string()];
        for l in sl + 1..el {
            parts.push(self.lines[l].clone());
        }
        parts.push(self.lines[el][..self.col_to_byte(el, ec)].to_string());
        Some(parts.join("\n"))
    }

    /// Returns true when a non-empty selection was removed.
    pub fn delete_selection(&mut self) -> bool {
        let Some(((sl, sc), (el, ec))) = self.selection() else {
            return false;
        };
        if (sl, sc) == (el, ec) {
            self.anchor = None;
            return false;
        }
        if sl == el {
            let start_b = self.col_to_byte(sl, sc);
            let end_b = self.col_to_byte(sl, ec);
            self.lines[sl].replace_range(start_b..end_b, "");
        } else {
            let head = self.lines[sl][..self.col_to_byte(sl, sc)].to_string();
            let tail = self.lines[el][self.col_to_byte(el, ec)..].to_string();
            self.lines.splice(sl..=el, [format!("{head}{tail}")]);
        }
        self.cursor = (sl, sc);
        self.anchor = None;
        true
    }

    /// Pasted text flows under the same rule as typing (spec 2.2): wraps at the margin,
    /// breaks on newlines, discards nothing. Returns true when any wrap happened.
    pub fn paste(&mut self, text: &str, capacity: usize) -> bool {
        let mut rang = false;
        for ch in text.chars() {
            if ch == '\r' {
                continue;
            }
            if ch == '\n' {
                self.newline();
                continue;
            }
            let (l, _) = self.cursor;
            if self.line_len(l) >= capacity {
                self.newline();
                rang = true;
            }
            let (l, c) = self.cursor;
            let b = self.col_to_byte(l, c);
            self.lines[l].insert(b, ch);
            self.cursor = (l, c + 1);
        }
        rang
    }

    /// Character count over the whole note (newlines excluded).
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.chars().count()).sum()
    }

    pub fn word_count(&self) -> usize {
        self.text().split_whitespace().count()
    }
}

#[derive(PartialEq)]
enum EditOutcome {
    Handled,
    Bell,
    OpenModal,
    SaveSilent,
    SaveAndMenu,
}

fn buffer(app: &mut App) -> Option<&mut EditorBuffer> {
    app.editor.as_mut().map(|e| &mut e.buffer)
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let capacity = app
        .editor
        .as_ref()
        .map(|e| e.capacity(app.area.width))
        .unwrap_or(1);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let outcome = if ctrl {
        match key.code {
            KeyCode::Char('c') => {
                copy_selection(app);
                EditOutcome::Handled
            }
            KeyCode::Char('v') => {
                paste_clipboard(app);
                EditOutcome::Handled
            }
            KeyCode::Char('x') => {
                cut_selection(app);
                EditOutcome::Handled
            }
            KeyCode::Char('a') => {
                if let Some(b) = buffer(app) {
                    b.select_all();
                }
                EditOutcome::Handled
            }
            KeyCode::Char('s') => {
                if app.editor.as_ref().is_some_and(|e| e.name.is_some()) {
                    EditOutcome::SaveSilent
                } else {
                    EditOutcome::OpenModal
                }
            }
            _ => EditOutcome::Handled,
        }
    } else {
        match key.code {
            KeyCode::Right => {
                if let Some(b) = buffer(app) {
                    b.move_right(shift);
                }
                EditOutcome::Handled
            }
            KeyCode::Left => {
                if let Some(b) = buffer(app) {
                    b.move_left(shift);
                }
                EditOutcome::Handled
            }
            KeyCode::Up => {
                if let Some(b) = buffer(app) {
                    b.move_up(shift);
                }
                EditOutcome::Handled
            }
            KeyCode::Down => {
                if let Some(b) = buffer(app) {
                    b.move_down(shift);
                }
                EditOutcome::Handled
            }
            KeyCode::Enter => {
                if let Some(b) = buffer(app) {
                    b.newline();
                }
                EditOutcome::Handled
            }
            KeyCode::Backspace => {
                if let Some(b) = buffer(app) {
                    b.backspace();
                }
                EditOutcome::Handled
            }
            KeyCode::Delete => {
                if let Some(b) = buffer(app) {
                    b.delete();
                }
                EditOutcome::Handled
            }
            KeyCode::Tab => {
                let rung = buffer(app)
                    .map(|b| b.insert_char('\t', capacity))
                    .unwrap_or(false);
                if rung {
                    EditOutcome::Bell
                } else {
                    EditOutcome::Handled
                }
            }
            KeyCode::Esc => {
                if app.editor.as_ref().is_some_and(|e| e.name.is_some()) {
                    EditOutcome::SaveAndMenu
                } else {
                    EditOutcome::OpenModal
                }
            }
            KeyCode::Char(ch) => {
                let rung = buffer(app)
                    .map(|b| b.insert_char(ch, capacity))
                    .unwrap_or(false);
                if rung {
                    EditOutcome::Bell
                } else {
                    EditOutcome::Handled
                }
            }
            _ => EditOutcome::Handled,
        }
    };

    match outcome {
        EditOutcome::Handled => {}
        EditOutcome::Bell => app.ring_bell(),
        EditOutcome::OpenModal => app.modal = Some(SaveModal::new()),
        EditOutcome::SaveSilent => app.save_silent(),
        EditOutcome::SaveAndMenu => {
            app.save_silent();
            app.goto_menu();
        }
    }
}

pub fn handle_paste(app: &mut App, text: &str) {
    let capacity = app
        .editor
        .as_ref()
        .map(|e| e.capacity(app.area.width))
        .unwrap_or(1);
    let discarded = app
        .editor
        .as_mut()
        .map(|e| e.buffer.paste(text, capacity))
        .unwrap_or(false);
    if discarded {
        app.ring_bell();
    }
}

fn clipboard_set(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_string()))
        .map_err(|e| e.to_string())
}

fn copy_selection(app: &mut App) {
    // No selection: plain no-op (spec 2.5).
    let Some(text) = app.editor.as_ref().and_then(|e| e.buffer.selected_text()) else {
        return;
    };
    if let Err(e) = clipboard_set(&text) {
        app.status = Some(format!("clipboard: {e}"));
    }
}

fn cut_selection(app: &mut App) {
    let Some(text) = app.editor.as_ref().and_then(|e| e.buffer.selected_text()) else {
        return;
    };
    if let Err(e) = clipboard_set(&text) {
        app.status = Some(format!("clipboard: {e}"));
        return;
    }
    if let Some(b) = buffer(app) {
        b.delete_selection();
    }
}

fn paste_clipboard(app: &mut App) {
    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
        Ok(text) => handle_paste(app, &text),
        Err(e) => app.status = Some(format!("clipboard: {e}")),
    }
}

pub fn draw(app: &App, frame: &mut Frame) {
    let Some(ed) = app.editor.as_ref() else {
        return;
    };
    let area = frame.area();
    if area.width == 0 {
        return;
    }
    let mut ed_height = area.height.saturating_sub(1); // HUD row
    if app.status.is_some() {
        ed_height = ed_height.saturating_sub(1);
    }
    let ed_area = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: ed_height,
    };
    let height = ed_area.height as usize;
    let margin = EditorState::margin(ed_area.width);
    let text_width = ed_area.width.saturating_sub(2 * margin);

    let (cursor_line, cursor_col) = ed.buffer.cursor();
    let offset = if cursor_line >= height {
        cursor_line + 1 - height
    } else {
        0
    };
    let sel = ed.buffer.selection();

    for (i, line) in ed
        .buffer
        .lines()
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
    {
        let spans = line_spans(i, line, sel);
        let rect = Rect {
            x: margin,
            y: (i - offset) as u16,
            width: text_width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(spans), rect);
    }

    if app.modal.is_none() {
        let cx = (margin + cursor_col as u16).min(ed_area.width.saturating_sub(1));
        let cy = ((cursor_line - offset) as u16).min(ed_height.saturating_sub(1));
        frame.set_cursor_position((cx, cy));
    }

    if let Some(msg) = app.status.as_deref() {
        let row = Rect {
            x: 0,
            y: area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(msg.to_string()), row);
    }
}

fn line_spans(
    line_idx: usize,
    text: &str,
    sel: Option<((usize, usize), (usize, usize))>,
) -> Line<'static> {
    let chars: Vec<char> = text.chars().collect();
    let Some(((sl, sc), (el, ec))) = sel else {
        return Line::from(text.to_string());
    };
    if line_idx < sl || line_idx > el {
        return Line::from(text.to_string());
    }
    let start = if line_idx == sl { sc } else { 0 };
    let end = if line_idx == el { ec } else { chars.len() };
    let sel_style = Style::new().add_modifier(Modifier::REVERSED);
    let mut spans = Vec::new();
    let pre: String = chars[..start.min(chars.len())].iter().collect();
    let mid: String = chars[start.min(chars.len())..end.min(chars.len())]
        .iter()
        .collect();
    let post: String = chars[end.min(chars.len())..].iter().collect();
    if !pre.is_empty() {
        spans.push(Span::raw(pre));
    }
    if !mid.is_empty() {
        spans.push(Span::styled(mid, sel_style));
    }
    if !post.is_empty() {
        spans.push(Span::raw(post));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_edge_inputs() {
        let mut b = EditorBuffer::blank();
        assert_eq!((b.char_count(), b.word_count()), (0, 0));
        for ch in "   ".chars() {
            b.insert_char(ch, 80);
        }
        assert_eq!((b.char_count(), b.word_count()), (3, 0));
        let b = EditorBuffer::from_text("hello world\n  \t x");
        assert_eq!(b.char_count(), 16);
        assert_eq!(b.word_count(), 3);
    }

    #[test]
    fn margins_scale_with_width_and_bound_capacity() {
        assert_eq!(EditorState::margin(8), 2);
        assert_eq!(EditorState::margin(40), 5);
        assert_eq!(EditorState::margin(110), 13);
        let ed = EditorState::blank();
        assert_eq!(ed.capacity(110), 84);
        assert_eq!(ed.capacity(40), 30);
        assert_eq!(ed.capacity(6), 2);
        assert_eq!(ed.capacity(4), 1); // degenerate: never below one column
    }

    #[test]
    fn typing_past_capacity_auto_wraps_with_ping() {
        let mut b = EditorBuffer::blank();
        for ch in "abc".chars() {
            assert!(!b.insert_char(ch, 3));
        }
        // The wrap itself rings the bell; nothing is rejected.
        assert!(b.insert_char('d', 3));
        assert_eq!(b.text(), "abc\nd");
        assert_eq!(b.cursor(), (1, 1));
        assert!(!b.insert_char('e', 3));
        assert_eq!(b.text(), "abc\nde");
    }

    #[test]
    fn enter_after_full_line_opens_next_line() {
        let mut b = EditorBuffer::blank();
        for ch in "abc".chars() {
            b.insert_char(ch, 3);
        }
        b.newline();
        // Deliberate Enter is silent; the fresh line accepts typing normally.
        assert!(!b.insert_char('x', 3));
        assert_eq!(b.text(), "abc\nx");
    }

    #[test]
    fn paste_wraps_instead_of_discarding() {
        let mut b = EditorBuffer::blank();
        assert!(b.paste("abcdefgh", 4));
        assert_eq!(b.text(), "abcd\nefgh");
        let mut b = EditorBuffer::blank();
        assert!(!b.paste("ab\ncd", 80));
        assert_eq!(b.text(), "ab\ncd");
    }

    #[test]
    fn backspace_merges_lines_and_delete_pulls_next_up() {
        let mut b = EditorBuffer::from_text("ab\ncd");
        b.move_left(false);
        b.move_left(false);
        b.backspace(); // col 0 -> merge "cd" onto "ab"
        assert_eq!(b.text(), "abcd");
        assert_eq!(b.cursor(), (0, 2));
        b.delete(); // removes 'c', cursor now on 'd'
        assert_eq!(b.text(), "abd");
        b.move_right(false);
        b.delete(); // end of line, no next line -> no-op
        assert_eq!(b.text(), "abd");
        let mut b2 = EditorBuffer::from_text("ab\ncd");
        b2.move_up(false);
        b2.delete(); // end of line -> pull next line up
        assert_eq!(b2.text(), "abcd");
    }

    #[test]
    fn shift_arrows_select_and_copy_text_extraction() {
        let mut b = EditorBuffer::from_text("hello");
        for _ in 0..3 {
            b.move_left(true);
        }
        assert_eq!(b.selected_text().as_deref(), Some("llo"));
        assert_eq!(b.cursor(), (0, 2));
        b.delete_selection();
        assert_eq!(b.text(), "he");
    }

    #[test]
    fn select_all_covers_whole_note() {
        let mut b = EditorBuffer::from_text("one\ntwo");
        b.select_all();
        assert_eq!(b.selected_text().as_deref(), Some("one\ntwo"));
    }

    #[test]
    fn selection_collapsed_copies_nothing() {
        let mut b = EditorBuffer::from_text("abc");
        b.move_left(true);
        b.move_right(true); // anchor == cursor again
        assert_eq!(b.selected_text(), None);
    }
}
