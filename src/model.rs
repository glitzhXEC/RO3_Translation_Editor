use anyhow::{Context, Result};
use csv::{ReaderBuilder, Terminator, WriterBuilder};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

static TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\{[^}\r\n]+\}|@\{[^}\r\n]+\}|\^\{[^}\r\n]+\}|(?:\[|【)[^\]】\r\n]+(?:\]|】)|\\u[0-9A-Fa-f]{4}|\\[nrt]|</?[A-Za-z][^>]*>"#).unwrap()
});
static STYLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\^\{[^}\r\n]+\}").unwrap());
static BRACKET_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:\[|【)[^\]】\r\n]+(?:\]|】)").unwrap());
static BROKEN_ARROW_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"↑\{\d+\}").unwrap());
static PREVIEW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:\^\{[^}\r\n]+\})+|\$\{[^}\r\n]+\}|@\{[^}\r\n]+\}|\\[nrt]").unwrap()
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowFilter {
    All,
    Untranslated,
    Issues,
    Translated,
}

#[derive(Clone, Debug, Default)]
pub struct Validation {
    pub missing: Vec<String>,
    pub extra: Vec<String>,
    pub messages: Vec<String>,
}
impl Validation {
    pub fn ok(&self) -> bool {
        self.messages.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RowState {
    pub translated: bool,
    pub issue: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Columns {
    pub id: usize,
    pub english: usize,
    pub thai: usize,
}

pub struct Document {
    pub path: PathBuf,
    pub rows: Vec<Vec<String>>,
    pub columns: Columns,
    pub selected: usize,
    pub row_query: String,
    pub filter: RowFilter,
    pub had_bom: bool,
    pub dirty: bool,
    pub states: Vec<RowState>,
}

impl Document {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("อ่านไฟล์ไม่ได้: {}", path.display()))?;
        let had_bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
        let text = std::str::from_utf8(if had_bom { &bytes[3..] } else { &bytes })
            .context("ไฟล์ต้องเป็น UTF-8")?;
        let mut reader = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .flexible(true)
            .from_reader(text.as_bytes());
        let mut rows: Vec<Vec<String>> = Vec::new();
        for record in reader.records() {
            rows.push(record?.iter().map(str::to_owned).collect());
        }
        anyhow::ensure!(rows.len() >= 2, "ไฟล์ไม่มีแถวข้อมูล");
        anyhow::ensure!(rows[0].len() >= 3, "ต้องมีอย่างน้อย 3 คอลัมน์");
        let columns = detect_columns(&rows[0]);
        let mut doc = Self {
            path: path.to_owned(),
            rows,
            columns,
            selected: 0,
            row_query: String::new(),
            filter: RowFilter::All,
            had_bom,
            dirty: false,
            states: Vec::new(),
        };
        doc.refresh_states();
        Ok(doc)
    }
    pub fn len(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
    pub fn row(&self, index: usize) -> &[String] {
        &self.rows[index + 1]
    }
    pub fn value(&self, index: usize, column: usize) -> &str {
        self.row(index)
            .get(column)
            .map(String::as_str)
            .unwrap_or("")
    }
    pub fn id(&self, index: usize) -> &str {
        self.value(index, self.columns.id)
    }
    pub fn english(&self, index: usize) -> &str {
        self.value(index, self.columns.english)
    }
    pub fn thai(&self, index: usize) -> &str {
        self.value(index, self.columns.thai)
    }
    pub fn set_thai(&mut self, index: usize, value: String) {
        let row = &mut self.rows[index + 1];
        if row.len() <= self.columns.thai {
            row.resize(self.columns.thai + 1, String::new());
        }
        if row[self.columns.thai] != value {
            row[self.columns.thai] = value;
            self.dirty = true;
            self.refresh_state(index);
        }
    }
    pub fn refresh_states(&mut self) {
        self.states = (0..self.len()).map(|i| self.compute_state(i)).collect();
    }
    pub fn refresh_state(&mut self, index: usize) {
        let state = self.compute_state(index);
        if let Some(slot) = self.states.get_mut(index) {
            *slot = state;
        }
    }
    fn compute_state(&self, index: usize) -> RowState {
        let en = self.english(index);
        let th = self.thai(index);
        RowState {
            translated: !th.trim().is_empty() && th.trim() != en.trim(),
            issue: !validate(en, th).ok(),
        }
    }
    pub fn stats(&self) -> (usize, usize) {
        (
            self.states.iter().filter(|x| x.translated).count(),
            self.states.iter().filter(|x| x.issue).count(),
        )
    }
    pub fn filtered_indices(&self) -> Vec<usize> {
        let query = self.row_query.trim().to_lowercase();
        (0..self.len())
            .filter(|&i| {
                let state = self.states[i];
                let filter_ok = match self.filter {
                    RowFilter::All => true,
                    RowFilter::Untranslated => !state.translated,
                    RowFilter::Issues => state.issue,
                    RowFilter::Translated => state.translated,
                };
                filter_ok
                    && (query.is_empty()
                        || self.id(i).to_lowercase().contains(&query)
                        || self.english(i).to_lowercase().contains(&query)
                        || self.thai(i).to_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn save(&mut self) -> Result<()> {
        let mut writer = WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .terminator(Terminator::CRLF)
            .from_writer(Vec::new());
        for row in &self.rows {
            writer.write_record(row)?;
        }
        let mut output = writer.into_inner()?;
        if self.had_bom {
            output.splice(0..0, [0xEF, 0xBB, 0xBF]);
        }
        fs::write(&self.path, output)
            .with_context(|| format!("บันทึกไม่ได้: {}", self.path.display()))?;
        self.dirty = false;
        Ok(())
    }
}

fn detect_columns(header: &[String]) -> Columns {
    let normalized: Vec<String> = header.iter().map(|x| x.trim().to_lowercase()).collect();
    let find = |names: &[&str], fallback| {
        normalized
            .iter()
            .position(|x| names.contains(&x.as_str()))
            .unwrap_or(fallback)
    };
    Columns {
        id: find(&["id", "key", "localization_id"], 0),
        english: find(&["english", "source", "en"], 1),
        thai: find(&["thai_translation", "thai", "translation", "target"], 2),
    }
}

pub fn tokens(text: &str) -> Vec<String> {
    TOKEN_RE
        .find_iter(text)
        .map(|m| m.as_str().to_owned())
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPart {
    pub text: String,
    pub emphasized: bool,
}

pub fn preview_parts(text: &str) -> Vec<PreviewPart> {
    let mut parts = Vec::new();
    let mut cursor = 0;
    let mut emphasized = false;
    for token in PREVIEW_RE.find_iter(text) {
        push_preview(&mut parts, &text[cursor..token.start()], emphasized);
        let value = token.as_str();
        if value.starts_with("^{") {
            // Consecutive game-style markers form one visual boundary.
            emphasized = !emphasized;
        } else if value == r"\n" {
            push_preview(&mut parts, "\n", false);
        } else if value == r"\t" {
            push_preview(&mut parts, "    ", false);
        } else if value == r"\r" {
            // Ignore a standalone escaped carriage return in preview.
        } else {
            push_preview(&mut parts, sample_value(value), true);
        }
        cursor = token.end();
    }
    push_preview(&mut parts, &text[cursor..], emphasized);
    parts
}

fn sample_value(token: &str) -> &'static str {
    let number = token
        .trim_start_matches(['$', '@'])
        .trim_matches(['{', '}'])
        .parse::<usize>()
        .unwrap_or(1);
    const VALUES: [&str; 8] = ["50", "10", "3", "5", "2", "8", "20", "1"];
    VALUES[(number.saturating_sub(1)) % VALUES.len()]
}

fn push_preview(parts: &mut Vec<PreviewPart>, text: &str, emphasized: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = parts.last_mut() {
        if last.emphasized == emphasized {
            last.text.push_str(text);
            return;
        }
    }
    parts.push(PreviewPart {
        text: text.to_owned(),
        emphasized,
    });
}
fn counts(items: &[String]) -> HashMap<&str, usize> {
    let mut map = HashMap::new();
    for item in items {
        *map.entry(item.as_str()).or_default() += 1;
    }
    map
}

pub fn validate(source: &str, target: &str) -> Validation {
    if target.trim().is_empty() {
        return Validation::default();
    }
    let src = tokens(source);
    let dst = tokens(target);
    let a = counts(&src);
    let b = counts(&dst);
    let mut out = Validation::default();
    for (token, n) in &a {
        for _ in 0..n.saturating_sub(*b.get(token).unwrap_or(&0)) {
            out.missing.push((*token).to_owned());
        }
    }
    for (token, n) in &b {
        for _ in 0..n.saturating_sub(*a.get(token).unwrap_or(&0)) {
            out.extra.push((*token).to_owned());
        }
    }
    if !out.missing.is_empty() {
        out.messages.push(format!("ขาด {}", out.missing.join(" ")));
    }
    if !out.extra.is_empty() {
        out.messages.push(format!("เกิน {}", out.extra.join(" ")));
    }
    let styles = |s: &str| {
        STYLE_RE
            .find_iter(s)
            .map(|m| m.as_str().to_owned())
            .collect::<Vec<String>>()
    };
    if styles(source) != styles(target) {
        out.messages.push("ลำดับ ^{…} ไม่ตรงต้นฉบับ".into());
    }
    let brackets = |s: &str| {
        BRACKET_RE
            .find_iter(s)
            .map(|m| m.as_str().to_owned())
            .collect::<Vec<String>>()
    };
    if brackets(source) != brackets(target) {
        out.messages.push("ชื่อหรือลำดับในวงเล็บไม่ตรงต้นฉบับ".into());
    }
    if BROKEN_ARROW_RE.is_match(target) {
        out.messages.push("พบ ↑{…} ซึ่งควรเป็น ^{…}".into());
    }
    let without_styles = STYLE_RE.replace_all(target, "");
    if without_styles.contains('^') {
        out.messages.push("พบเครื่องหมาย ^ ที่ไม่สมบูรณ์".into());
    }
    out
}

pub fn scan_tsv_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("tsv"))
        })
        .map(|e| e.into_path())
        .collect();
    files.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catches_broken_markers() {
        assert!(
            validate("A ^{1}${1}%^{2} 【Poring】", "ก ^{1}${1}% 【Poring】")
                .messages
                .len()
                > 0
        );
        assert!(validate("A ^{1}${1}%^{2} 【Poring】", "ก ^{1}${1}%^{2} 【Poring】").ok());
    }
    #[test]
    fn preserves_duplicate_tokens() {
        let v = validate("${1} ${1}", "${1}");
        assert_eq!(v.missing, vec!["${1}"]);
    }
    #[test]
    fn creates_game_style_preview() {
        let parts = preview_parts(r"DMG *^{1}${1}%+${2}^{2}\nRange ${3}m");
        let rendered = parts.iter().map(|p| p.text.as_str()).collect::<String>();
        assert_eq!(rendered, "DMG *50%+10\nRange 3m");
        assert!(parts.iter().any(|p| p.emphasized && p.text.contains("50")));
    }
}
