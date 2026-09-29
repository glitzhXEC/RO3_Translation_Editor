# RO3 Translation Editor

Native desktop editor for translating RO3 TSV localization files by hand. This is a standalone project and is not bundled into the game patch repository.

## Design

- **Dracula + minimal light themes:** readable semantic colors, clear focus, and no accidental text selection in row navigation.
- **Workspace first:** open a folder once and browse every `.tsv` file recursively in Explorer.
- **Editor workflow:** keep multiple files open as tabs, search rows, and filter by untranslated, issues, or completed rows.
- **Live game preview:** strips `^{…}`, inserts sample values for `${…}`/`@{…}`, highlights formatted values, and wraps the completed Thai text like an in-game tooltip.
- **Safe translation:** English is selectable and copyable but read-only on the left; Thai is editable on the right. `${…}`, `@{…}`, `^{…}`, bracketed names, escaped characters, and HTML-like tags are validated live.
- **Native saving:** `Ctrl+S` writes the current TSV directly; Save all writes every dirty tab. UTF-8 BOM and normal TSV quoting are preserved.
- **Folder-wide replace:** `Ctrl+H` scans the Thai column across every TSV, previews occurrence and file counts, then replaces and saves all affected files with adjacent `.bak` backups.
- **Offline:** no file content leaves the computer.

## Build

```bash
cargo run
cargo test
cargo build --release
```

A translated sample with valid and intentionally broken markers is included at `examples/RO3_Translation_Example.tsv`.

The Windows executable is produced at `target/release/ro3-translation-editor.exe`.

## Keyboard shortcuts

- `Ctrl+O` — open folder
- `Ctrl+S` — save active file
- `Ctrl+Shift+S` — save all files
- `Ctrl+H` — find and replace Thai text across the opened folder
- `Ctrl+PageDown` / `Ctrl+PageUp` — select the next / previous TSV file in the current folder
- `Ctrl+B` — toggle Explorer
- `Ctrl+J` — toggle row navigator

## License

MIT. The bundled Noto Sans Thai font is licensed under the SIL Open Font License; see `assets/OFL.txt`.
