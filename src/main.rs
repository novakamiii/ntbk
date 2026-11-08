mod app;
mod io_index;
mod io_notes;
mod ui_editor;
mod ui_hud;
mod ui_menu;
mod ui_modal;

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor::Show, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let result = run_app(&mut stdout);
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableBracketedPaste, Show)?;
    result
}

fn run_app(stdout: &mut io::Stdout) -> io::Result<()> {
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let base = std::env::current_dir()?;
    let mut app = App::new(base);

    loop {
        terminal.draw(|frame| app.draw(frame))?;
        if app.take_bell() {
            write!(terminal.backend_mut(), "\x07")?;
            terminal.backend_mut().flush()?;
        }
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }
        if app.quitting() {
            return Ok(());
        }
    }
}
