use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::app::App;

/// Minimal HUD (spec 2.3): counts on the left, session time and timestamp on the right.
pub fn draw(app: &App, frame: &mut Frame) {
    let Some(ed) = app.editor.as_ref() else {
        return;
    };
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    let row_y = area.height - 1;

    let chars = ed.buffer.char_count();
    let words = ed.buffer.word_count();
    let left = format!("{chars} chars · {words} words");

    let secs = ed.session_start.elapsed().as_secs();
    let session = format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    );
    let now = chrono::Local::now().format("%d/%m/%Y %H:%M:%S");
    let right = format!("{session} · {now}");

    let right_len = right.chars().count() as u16;
    // Bound the left segment so the two can never overlap on narrow terminals.
    let left_width = area.width.saturating_sub(right_len);
    if left_width > 0 {
        frame.render_widget(
            Paragraph::new(left),
            Rect {
                x: 0,
                y: row_y,
                width: left_width,
                height: 1,
            },
        );
    }
    if right_len < area.width {
        frame.render_widget(
            Paragraph::new(right),
            Rect {
                x: area.width - right_len,
                y: row_y,
                width: right_len,
                height: 1,
            },
        );
    }
}
