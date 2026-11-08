# ntbk

A minimal, keyboard-first note-taking TUI in pure Rust. Open it, type, save — nothing
else on screen. The editor behaves like a typewriter: text flows between page margins,
and when a line reaches the right margin the carriage returns by itself to the next
line, with a bell ping.

## Build

Requires the [Rust toolchain](https://rustup.rs).

```sh
cargo build --release
```

The binary lands in `target/release/ntbk`. Run it from the directory where you keep
your notes — notes are read from and written to the current working directory.

## Usage

The app opens straight into the main menu: **New Note** on top, your previous sessions
below (the 5 most recent, then `Show more`).

### Editor

| Key | Action |
|-----|--------|
| typing | plain text; automatic carriage return at the right margin (one bell ping per return) |
| Enter | deliberate line break (silent) |
| Shift+Arrows | select text |
| Ctrl+C / Ctrl+X / Ctrl+V | copy / cut / paste |
| Ctrl+A | select all |
| Ctrl+S | save — named notes save silently, new notes open the save modal |
| Escape | exit — new note: save modal; named note: silent save, back to the menu |

The bottom HUD shows character and word count on the left, session time and the
timestamp (`DD/MM/YYYY HH:MM:SS`) on the right.

### Save modal (new notes)

`Escape` on an unnamed note opens a centered **File Name** box:

| Key | Action |
|-----|--------|
| Enter | commit the name, enable the actions below |
| s | save, return to the menu |
| S | save, quit the app |
| c | cancel, back to the editor |
| d | discard, return to the menu |
| D | discard, quit the app |

Saving with an empty name falls back to a timestamped name (`YYYY-MM-DD.txt`). A name
without an extension gets `.txt`; type the `.md` yourself to save markdown.

### Menu keys

Arrows or `j`/`k` to move, `Enter` to open, `q` or `Escape` to quit.

## Notes storage

- Supported formats: `.txt` and `.md`.
- Notes are saved in the directory the app was launched from.
- The recents list is kept in a small index file (`.ntbk`) next to the notes.

## Stack

`ratatui` + `crossterm` for the TUI, `arboard` for the system clipboard, `chrono` for
timestamps, `serde_json` for the recents index. No other runtime dependencies.
