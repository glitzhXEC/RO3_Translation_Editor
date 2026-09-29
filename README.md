# RO3 Translation Editor

Native desktop editor for translating RO3 TSV localization files by hand. This is a standalone project and is not bundled into the game patch repository.

## Design

- **Workspace first:** open a folder once and browse every `.tsv` file recursively in Explorer.
- **Editor workflow:** keep multiple files open as tabs, search rows, and filter by untranslated, issues, or completed rows.
- **Safe translation:** English is read-only on the left; Thai is editable on the right. `${…}`, `@{…}`, `^{…}`, bracketed names, escaped characters, and HTML-like tags are validated live.
- **Native saving:** `Ctrl+S` writes the current TSV directly; Save all writes every dirty tab. UTF-8 BOM and normal TSV quoting are preserved.
- **Offline:** no file content leaves the computer.

## Build

```bash
cargo run
cargo test
cargo build --release
```

The Windows executable is produced at `target/release/ro3-translation-editor.exe`.

## Keyboard shortcuts

- `Ctrl+O` — open folder
- `Ctrl+S` — save active file
- `Ctrl+Shift+S` — save all files

## License

MIT. The bundled Noto Sans Thai font is licensed under the SIL Open Font License; see `assets/OFL.txt`.
