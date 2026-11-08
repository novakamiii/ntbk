# AGENTS.md — ntbk

Project-specific instructions for agents working in this repository. The global file at
`/home/nova/AGENTS.md` also applies (working code only, no flattery, no fabricated results,
surgical changes, verify everything). On conflict, this file wins for project matters.

---

## 1. What this is

**ntbk** — a minimal, keyboard-first, full-screen TUI note-taking app written in pure Rust.
Its entire reason to exist is to let a person open it, type, and save, with nothing else on
screen. If a proposed change makes the app heavier rather than faster-to-use, it is probably
out of scope.

### Explicitly out of scope (do not add unless the user asks)

- Mouse interaction, scrollbars, menus with icons or colors-as-features
- Markdown preview / rendering / syntax highlighting
- Plugins, themes, settings screens, sync, backup, encryption
- Line numbers, split panes, sidebar file trees
- Any new keybinding, HUD element, or prompt not in section 2

Bias is strong: this app is defined by its absence of features. "It could also do X" is not
an argument. Ship what is specified.

---

## 2. Product specification

This is the source of truth for behavior. When implementing, match this text; if the text is
unclear, ask (see section 8) — do not silently choose.

### 2.1 Main menu (app entry)

The app launches straight into a full-screen TUI main menu. No splash screen, no banner.

Layout, top to bottom:

1. **New Note** — always the first item
2. **Previous note sessions** — a flat list below the first item, most recently edited first,
   sourced from the recents index (see 2.6). Shows the 5 most recent entries, then a
   **Show more** entry; opening it expands to the complete list (same ordering, the five
   included) and the entry becomes **Show less** to collapse back to 5.

Selecting `New Note` opens the editor with blank, unnamed content (see 2.2). Selecting a
previous session opens that note in the editor with its text loaded (see 2.2 and 2.6).

Menu keys (confirmed): arrows or `j`/`k` to move, `Enter` to open, `q` and `Escape` both
quit the app from the menu.

### 2.2 Editor screen (typewriter mode)

- Full-screen text area. Nothing else visible while typing. Pure text entry.
- Text flows from the left edge of the page. (Revised 2026-11-09 after first live use: the
  centered-carriage reading produced the cramped right-half layout the user rejected —
  "the spacing is kinda weird… it should automatically make it go left rather than getting
  stuck".)
- Typewriter lines:
  - The page has typewriter margins: text begins `margin` columns in from the left edge
    and wraps at the same distance from the right edge, where
    `margin = clamp(width / 8, 2, 16)` — roughly a paper margin at any terminal size.
    Margins are spacing only: no visible rules or guides. The HUD, menu, and modal are
    screen furniture and keep the full width.
  - A line holds whatever fits between the margins; there is no soft wrap.
  - When the caret reaches the end of a full line, the app continues on the next line by
    itself — the automatic carriage return — ringing one bell ping as it wraps. Typing is
    never blocked; nothing is clipped or discarded. This replaces the first session's
    bell-then-wait-for-Enter rule.
  - `Enter` is for deliberate line breaks (paragraphs) and is silent.
  - Pasted text flows under the same rule and wraps as needed; the whole paste lands.
- A blank note opens with the cursor at the top-left of the page.
- When lines exceed the visible height, the view scrolls up as new lines roll in; the HUD
  stays pinned.
- Existing/loaded notes open with the cursor at the end of the text (proposed default).
- Typing is plain text entry; no autocorrect, autocomplete, or transformations.

### 2.3 HUD (bottom bar)

One line, pinned to the bottom of the screen, present while the editor is open.

- **Left:** text count (character count) and word count, in that order.
- **Right:** session time (elapsed since entering the editor) and the full timestamp date.

Timestamp format (confirmed): `DD/MM/YYYY HH:MM:SS`.

The HUD stays put. It does not pulse, animate, or change color state. It exists so the user
can measure their writing; it does not decorate.

### 2.4 Exit and the save modal

`Escape` inside the editor is the exit command. What happens next depends on the note:

**New/unnamed note** — Escape opens a centered TUI modal: a rectangular box containing a
label **`File Name`** and a text input field. The modal has two states:

1. **Name entry** (the state it opens in): every typed character goes into the input field
   — the modal is focused on the name only, and the action keys below do not fire. `Enter`
   commits the name; `Escape` cancels the modal back to the editor (confirmed).
2. **Committed** (after `Enter`): the modal shows the filename as typed and the action keys
   below become live. Case matters by convention — a lowercase key performs the action and
   returns to the main menu; the Shift variant performs the same action and quits the app.

| Key | Action |
|-----|--------|
| `s` | Save with the entered filename, then return to the main menu |
| `S` | Save with the entered filename, then quit the app |
| `c` | Cancel the modal, return to the editor with all content intact |
| `d` | Discard the note (no save), then return to the main menu |
| `D` | Discard the note (no save), then quit the app |

Notes:

- `d`/`D` discard immediately — no confirmation dialog. Do not add one.
- Saving with an empty filename falls back to a timestamped name like `2026-10-25.txt`
  (confirmed); the rule applies to both save keys.
- Pressing `Enter` again from the committed state re-enters name editing to fix a typo
  (assumed; the only assumption in the modal flow).
- The modal is an overlay on the editor; the editor text and HUD remain visible beneath it
  (proposed visual default).

**Existing/named note** — Escape saves silently and returns to the main menu (confirmed).
No prompt, no modal.

### 2.5 Clipboard and standard keys

`Ctrl+C` / `Ctrl+V` and the other standard combos (table below) work in the editor.
Bracketed paste is additionally supported so middle-click/terminal-native paste works
cleanly while the terminal is in raw mode.

Context for implementers: in raw mode, `Ctrl+C` does not reach the system clipboard for
free — the app must own it. Clipboard access in pure Rust via the `arboard` crate;
paste-event support via crossterm's bracketed-paste events. Also note: `crossterm` in raw
mode traps `Ctrl+C` as a key event, so the app handling decides what it means — copy first
(with a selection), and only consider a quit-unrelated fallback if no selection exists.

Standard key set (confirmed):

| Combo | Action |
|-------|--------|
| `Ctrl+C` | Copy selection to system clipboard; with no selection, a plain no-op |
| `Ctrl+V` | Paste from system clipboard at cursor |
| `Ctrl+X` | Cut selection |
| `Ctrl+A` | Select all |
| `Ctrl+S` | Save: named notes save silently; new/unnamed notes open the save modal |
| `Shift+Arrow` | Extend or contract the selection (confirmed selection mechanism) |

Selection semantics exist only for copy/cut. There is no click-selection; there is no mouse.

### 2.6 Storage and file formats

- Supported formats: `.txt` and `.md`. Nothing else.
- Notes are stored in the current working directory the app is launched from — no
  subdirectory (confirmed).
- Filenames are what the user types; a name without an extension gets `.txt` appended, so
  every file written is `.txt` or `.md`. To save markdown, type the `.md` explicitly.
- An empty filename at save time falls back to a timestamped name like `2026-10-25.txt`
  (see 2.4).
- The recents list is backed by a dedicated index file (e.g. `.ntbk`) kept alongside the
  notes in the same directory — the menu never scans the directory (confirmed). The index
  records each note's filename and last-edited time, and is written on every save.
- Open previous: opening a session from the menu loads the full file content into the editor.

---

## 3. Architecture (proposed)

Pure Rust. Crates chosen in the first implementation session (pinned in `Cargo.toml`):

- **TUI/rendering/events:** `ratatui` 0.30 + `crossterm` 0.29
- **Clipboard:** `arboard` 3.7
- **Time:** `chrono` 0.4 (HUD timestamp and timestamped fallback filename);
  `std::time::Instant` for elapsed session time
- **Index:** `serde` + `serde_json` for the recents index file — the one justified use of
  serialization in this app
- **Sound:** none added — the typewriter ping is a `BEL` byte (`0x07`) written to stdout,
  so audibility follows the user's terminal bell settings; no audio crate

Module layout (as built):

```
src/
  main.rs          entry point, terminal init/teardown, event loop (raw mode, alternate screen)
  app.rs           App state machine, save/open flows, key dispatch
  ui_menu.rs       main menu rendering + input (recents, show more/less)
  ui_editor.rs     text buffer (typewriter rules, selection) + editor rendering and input
  ui_hud.rs        status bar rendering
  ui_modal.rs      File Name save-modal states, rendering + input
  io_notes.rs      filename resolution, reading/writing notes
  io_index.rs      recents index file read/write
```

Keep it flat. The app has exactly four screens/states: **Menu**, **Editor**, **SaveModal**
(overlaying Editor), and **Quit**. Every rendering edge case should fit without deep module
nests. Inventing intermediate abstractions ("editor traits", "storage driver interfaces") is
wasted effort for an app that writes files with one name at a time.

---

## 4. State machine summary

```
Menu --[Enter: New Note]-------> Editor(unnamed)
Menu --[Enter: recents row]----> Editor(named, loaded from disk)
Menu --[q or Escape]-----------> Quit

Editor(unnamed) --[Escape]-----> SaveModal
Editor(named)   --[Escape]-----> write file silently ------> Menu

SaveModal (two states; action keys are case-sensitive):
  [name entry] typing -> goes to input field only
    Enter  --> [committed]
    Escape --> back to Editor (content preserved)
  [committed]:
    s --> write file (empty name -> timestamped fallback) --> Menu
    S --> write file --> Quit
    c --> back to Editor (content preserved)
    d --> discard --> Menu
    D --> discard --> Quit
    Enter --> back to [name entry]

Editor --[Ctrl+S]--> named: write file silently; unnamed: open SaveModal
```

---

## 5. Commands

Assumes `cargo` is installed (Rust toolchain present).

- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`

During iteration, run a single test file rather than the whole suite
(`cargo test name_of_test --lib`). Full suite is for the final verification pass.

---

## 6. Verification rules specific to this project

- This is a TUI, so visual correctness cannot be proven by unit tests alone. Before claiming
  a screen works, **run the app and look at it**. Scripted capture (e.g. dump of the rendered
  frame, or `tmux capture-pane`) is acceptable; a bare "looks plausible" is not.
- Minimum regression coverage expected as tests:
  - word count and character count against edge inputs (empty, whitespace-only, tabs, newlines)
  - editor buffer operations: insert, backspace, delete, selection boundaries
  - typewriter wrap: typing past a full line auto-wraps to the next line with one ping per
    wrap and nothing rejected; paste flows and wraps without discarding; `Enter` breaks
    lines silently
  - save modal state transitions: name entry, commit via `Enter`, then `s`/`S`/`c`/`d`/`D`
    from the committed state, plus `Escape`-cancel from both states
  - file IO round-trip for `.txt` and `.md`, including the no-extension filename case
- `cargo clippy` clean and `cargo fmt` applied before declaring a task done, matching the
  global file's "finish the job" requirement.

---

## 7. Repo conventions

- Language: Rust, edition found in the actual `Cargo.toml` once created (do not guess a
  version in code comments or docs).
- Error handling: return `Result` up to `main`; avoid `unwrap()`/`expect()` in anything
  reachable from user input. A note app must never crash mid-typing — that is the baseline
  promise of using it.
- No panics from user input paths (malformed filenames, unreadable files => visible error
  line in HUD or modal message, not abort).
- Comments only where the reason is non-obvious; the code should read like the spec in
  section 2 where possible, so spec drift is visible.
- Keep `Cargo.toml` dependencies minimal; every addition must trace to section 2 behavior.

---

## 8. Open decisions — live questions only

Numbered; answer in a session, then fold the decision into section 2 or 4 and delete the
entry. This list holds only unresolved items.

- (empty)

---

## 9. Project learnings

Append one-line, concrete rules here when a session reveals a correction worth carrying
forward ("Use X for Y", never "be careful with Y"). Prune lines whose underlying issue has
disappeared.

- Validate metaphor-based spec readings ("typewriter", "cursor centered") against a live
  run before building on them: the centered-carriage reading survived two Q&A rounds and
  failed at first real use (2026-11-09).
