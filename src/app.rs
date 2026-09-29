use crate::model::{scan_tsv_files, tokens, validate, Document, RowFilter};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(189, 147, 249)
    } else {
        Color32::from_rgb(39, 131, 222)
    }
}
fn success(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(80, 250, 123)
    } else {
        Color32::from_rgb(46, 142, 99)
    }
}
fn danger(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(255, 85, 85)
    } else {
        Color32::from_rgb(220, 82, 75)
    }
}
fn warning(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(255, 184, 108)
    } else {
        Color32::from_rgb(190, 111, 40)
    }
}
fn primary_text(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(248, 248, 242)
    } else {
        Color32::from_rgb(44, 44, 43)
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Settings {
    recent_folder: Option<PathBuf>,
    dark: bool,
}
#[derive(Clone)]
struct Notice {
    message: String,
    error: bool,
    until: f64,
}

pub struct EditorApp {
    root: Option<PathBuf>,
    files: Vec<PathBuf>,
    file_query: String,
    docs: HashMap<PathBuf, Document>,
    tabs: Vec<PathBuf>,
    active: Option<PathBuf>,
    settings: Settings,
    notice: Option<Notice>,
    confirm_close: bool,
    confirm_folder: Option<PathBuf>,
}
impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let settings = load_settings();
        apply_theme(&cc.egui_ctx, settings.dark);
        let mut app = Self {
            root: None,
            files: vec![],
            file_query: String::new(),
            docs: HashMap::new(),
            tabs: vec![],
            active: None,
            settings,
            notice: None,
            confirm_close: false,
            confirm_folder: None,
        };
        let startup = std::env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .or_else(|| app.settings.recent_folder.clone().filter(|p| p.exists()));
        if let Some(path) = startup {
            app.open_folder_now(path);
            if let Some(first) = app.files.first().cloned() {
                app.open_file(&cc.egui_ctx, first);
            }
        }
        app
    }
    fn dirty_count(&self) -> usize {
        self.docs.values().filter(|d| d.dirty).count()
    }
    fn toast(&mut self, ctx: &egui::Context, msg: impl Into<String>, error: bool) {
        self.notice = Some(Notice {
            message: msg.into(),
            error,
            until: ctx.input(|i| i.time) + 3.5,
        });
    }
    fn choose_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if self.dirty_count() > 0 {
                self.confirm_folder = Some(path)
            } else {
                self.open_folder_now(path)
            }
        }
    }
    fn open_folder_now(&mut self, path: PathBuf) {
        self.files = scan_tsv_files(&path);
        self.root = Some(path.clone());
        self.docs.clear();
        self.tabs.clear();
        self.active = None;
        self.settings.recent_folder = Some(path);
        save_settings(&self.settings);
    }
    fn open_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        if !self.docs.contains_key(&path) {
            match Document::load(&path) {
                Ok(doc) => {
                    self.docs.insert(path.clone(), doc);
                    self.tabs.push(path.clone())
                }
                Err(e) => {
                    self.toast(ctx, e.to_string(), true);
                    return;
                }
            }
        }
        self.active = Some(path);
    }
    fn save_active(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.active.clone() {
            if let Some(doc) = self.docs.get_mut(&path) {
                match doc.save() {
                    Ok(_) => self.toast(ctx, "บันทึกไฟล์แล้ว", false),
                    Err(e) => self.toast(ctx, e.to_string(), true),
                }
            }
        }
    }
    fn save_all(&mut self, ctx: &egui::Context) {
        let mut saved = 0;
        let mut err = None;
        for doc in self.docs.values_mut().filter(|d| d.dirty) {
            match doc.save() {
                Ok(_) => saved += 1,
                Err(e) => {
                    err = Some(e.to_string());
                    break;
                }
            }
        }
        if let Some(e) = err {
            self.toast(ctx, e, true)
        } else {
            self.toast(ctx, format!("บันทึก {} ไฟล์แล้ว", saved), false)
        }
    }
    fn close_tab(&mut self, path: &PathBuf) {
        if self.docs.get(path).is_some_and(|d| d.dirty) {
            return;
        }
        self.docs.remove(path);
        self.tabs.retain(|p| p != path);
        if self.active.as_ref() == Some(path) {
            self.active = self.tabs.last().cloned();
        }
    }
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .exact_height(58.0)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .stroke(Stroke::new(
                        1.0_f32,
                        ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
                    )),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    egui::Frame::new()
                        .fill(accent(ui))
                        .corner_radius(8)
                        .inner_margin(8)
                        .show(ui, |ui| {
                            ui.label(RichText::new("TH").strong().color(Color32::WHITE));
                        });
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("RO3 Translation Editor")
                                .size(18.0)
                                .strong()
                                .color(primary_text(ui)),
                        );
                        ui.label(
                            RichText::new("Native TSV workspace")
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                    ui.add_space(18.0);
                    if ui.button("📁  Open folder").clicked() {
                        self.choose_folder()
                    }
                    ui.separator();
                    let dirty = self.dirty_count();
                    if ui
                        .add_enabled(self.active.is_some(), egui::Button::new("Save  Ctrl+S"))
                        .clicked()
                    {
                        self.save_active(ctx)
                    }
                    if ui
                        .add_enabled(
                            dirty > 0,
                            egui::Button::new(format!(
                                "Save all{}",
                                if dirty > 0 {
                                    format!(" ({dirty})")
                                } else {
                                    String::new()
                                }
                            )),
                        )
                        .clicked()
                    {
                        self.save_all(ctx)
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if self.settings.dark {
                                "☀ Light"
                            } else {
                                "☾ Dark"
                            })
                            .clicked()
                        {
                            self.settings.dark = !self.settings.dark;
                            apply_theme(ctx, self.settings.dark);
                            save_settings(&self.settings)
                        }
                        if let Some(root) = &self.root {
                            ui.label(
                                RichText::new(
                                    root.file_name().unwrap_or_default().to_string_lossy(),
                                )
                                .color(ui.visuals().weak_text_color()),
                            );
                        }
                    });
                });
            });
    }
    fn explorer(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("explorer")
            .default_width(270.0)
            .min_width(220.0)
            .max_width(400.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("EXPLORER")
                            .size(12.0)
                            .strong()
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} TSV", self.files.len()));
                    });
                });
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.file_query)
                        .hint_text("Search files…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                if self.root.is_none() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No folder opened").strong());
                        ui.label(
                            RichText::new("Open a folder to browse every .tsv file recursively.")
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        );
                        ui.add_space(10.0);
                        if ui.button("Open folder").clicked() {
                            self.choose_folder()
                        }
                    });
                    return;
                }
                let root = self.root.clone().unwrap();
                let q = self.file_query.to_lowercase();
                let visible: Vec<_> = self
                    .files
                    .iter()
                    .filter(|p| {
                        q.is_empty()
                            || p.strip_prefix(&root)
                                .unwrap_or(p)
                                .to_string_lossy()
                                .to_lowercase()
                                .contains(&q)
                    })
                    .cloned()
                    .collect();
                let mut open = None;
                egui::ScrollArea::vertical()
                    .id_salt("files")
                    .show(ui, |ui| {
                        let mut last_dir = PathBuf::new();
                        for path in visible {
                            let rel = path.strip_prefix(&root).unwrap_or(&path);
                            let dir = rel.parent().unwrap_or(Path::new(""));
                            if dir != last_dir {
                                last_dir = dir.to_owned();
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(if dir.as_os_str().is_empty() {
                                        "/".into()
                                    } else {
                                        dir.to_string_lossy().into_owned()
                                    })
                                    .size(11.0)
                                    .strong()
                                    .color(ui.visuals().weak_text_color()),
                                );
                            }
                            let dirty = self.docs.get(&path).is_some_and(|d| d.dirty);
                            let active = self.active.as_ref() == Some(&path);
                            let name = format!(
                                "{}{}",
                                if dirty { "● " } else { "" },
                                rel.file_name().unwrap_or_default().to_string_lossy()
                            );
                            if ui
                                .selectable_label(active, name)
                                .on_hover_text(rel.display().to_string())
                                .clicked()
                            {
                                open = Some(path)
                            }
                        }
                    });
                if let Some(path) = open {
                    self.open_file(ctx, path)
                }
            });
    }
    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ctx.style().visuals.extreme_bg_color))
            .show(ctx, |ui| {
                let mut activate = None;
                let mut close = None;
                ui.horizontal(|ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for path in self.tabs.clone() {
                                    let doc = &self.docs[&path];
                                    let active = self.active.as_ref() == Some(&path);
                                    let label = format!(
                                        "{}{}",
                                        doc.file_name(),
                                        if doc.dirty { "  ●" } else { "" }
                                    );
                                    if ui.selectable_label(active, label).clicked() {
                                        activate = Some(path.clone())
                                    }
                                    if ui
                                        .small_button("×")
                                        .on_hover_text(if doc.dirty {
                                            "Save before closing"
                                        } else {
                                            "Close tab"
                                        })
                                        .clicked()
                                    {
                                        close = Some(path.clone())
                                    }
                                    ui.separator();
                                }
                            })
                        });
                });
                if let Some(p) = activate {
                    self.active = Some(p)
                }
                if let Some(p) = close {
                    self.close_tab(&p)
                }
                ui.separator();
                let Some(path) = self.active.clone() else {
                    welcome(ui, self.root.as_deref());
                    return;
                };
                let available = ui.available_size();
                let mut selected_new = None;
                let mut target_change = None;
                let mut copy_token = None;
                let mut move_delta = 0isize;
                let mut go_issue = false;
                let doc = self.docs.get_mut(&path).unwrap();
                let (translated, issues) = doc.stats();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(doc.file_name())
                            .size(16.0)
                            .strong()
                            .color(primary_text(ui)),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} rows  ·  {} translated  ·  {} issues",
                            doc.len(),
                            translated,
                            issues
                        ))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Next issue").clicked() {
                            go_issue = true
                        }
                        if ui.button("↓").clicked() {
                            move_delta = 1
                        }
                        if ui.button("↑").clicked() {
                            move_delta = -1
                        }
                    });
                });
                ui.add_space(6.0);
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(320.0))
                    .size(egui_extras::Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            egui::Frame::new()
                                .fill(ui.visuals().panel_fill)
                                .corner_radius(8)
                                .inner_margin(10)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut doc.row_query)
                                            .hint_text("Search ID, English, Thai…")
                                            .desired_width(f32::INFINITY),
                                    );
                                    ui.horizontal_wrapped(|ui| {
                                        for (f, label) in [
                                            (RowFilter::All, "All"),
                                            (RowFilter::Untranslated, "To do"),
                                            (RowFilter::Issues, "Issues"),
                                            (RowFilter::Translated, "Done"),
                                        ] {
                                            if ui.selectable_label(doc.filter == f, label).clicked()
                                            {
                                                doc.filter = f
                                            }
                                        }
                                    });
                                    ui.separator();
                                    let indices = doc.filtered_indices();
                                    let row_h = 58.0;
                                    egui::ScrollArea::vertical().id_salt("rows").show_rows(
                                        ui,
                                        row_h,
                                        indices.len(),
                                        |ui, range| {
                                            for pos in range {
                                                let i = indices[pos];
                                                let state = doc.states[i];
                                                let selected = doc.selected == i;
                                                let dot = if state.issue {
                                                    danger(ui)
                                                } else if state.translated {
                                                    success(ui)
                                                } else {
                                                    ui.visuals().weak_text_color()
                                                };
                                                let response = ui
                                                    .allocate_ui_with_layout(
                                                        Vec2::new(ui.available_width(), row_h),
                                                        egui::Layout::left_to_right(
                                                            egui::Align::TOP,
                                                        ),
                                                        |ui| {
                                                            ui.add_space(3.0);
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new("●").color(dot),
                                                                )
                                                                .selectable(false),
                                                            );
                                                            ui.vertical(|ui| {
                                                                ui.add(
                                                                    egui::Label::new(
                                                                    RichText::new(doc.id(i))
                                                                        .monospace()
                                                                        .size(11.0)
                                                                        .color(
                                                                            ui.visuals()
                                                                                .weak_text_color(),
                                                                        ),
                                                                    )
                                                                    .selectable(false),
                                                                );
                                                                let preview = if doc
                                                                    .thai(i)
                                                                    .trim()
                                                                    .is_empty()
                                                                {
                                                                    doc.english(i)
                                                                } else {
                                                                    doc.thai(i)
                                                                };
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(truncate(
                                                                            preview, 48,
                                                                        ))
                                                                        .size(13.0),
                                                                    )
                                                                    .selectable(false),
                                                                );
                                                            });
                                                        },
                                                    )
                                                    .response;
                                                let response = ui.interact(
                                                    response.rect,
                                                    ui.id().with(("translation-row", i)),
                                                    egui::Sense::click(),
                                                );
                                                if selected {
                                                    ui.painter().rect_stroke(
                                                        response.rect,
                                                        6.0,
                                                        Stroke::new(1.5_f32, accent(ui)),
                                                        egui::StrokeKind::Inside,
                                                    );
                                                }
                                                if response.clicked() {
                                                    selected_new = Some(i)
                                                }
                                            }
                                        },
                                    );
                                });
                        });
                        strip.cell(|ui| {
                            egui::Frame::new()
                                .fill(ui.visuals().panel_fill)
                                .corner_radius(8)
                                .inner_margin(14)
                                .show(ui, |ui| {
                                    let i = doc.selected.min(doc.len().saturating_sub(1));
                                    let id = doc.id(i).to_owned();
                                    let en = doc.english(i).to_owned();
                                    let mut th = doc.thai(i).to_owned();
                                    let validation = validate(&en, &th);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("ID {id}"))
                                                .monospace()
                                                .color(ui.visuals().weak_text_color()),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(format!("{} / {}", i + 1, doc.len()));
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);
                                    ui.columns(2, |panes| {
                                        let mut en_view = en.clone();
                                        language_pane(
                                            &mut panes[0],
                                            "English",
                                            "SOURCE · READ ONLY",
                                            &mut en_view,
                                            false,
                                            &validation,
                                            &mut copy_token,
                                        );
                                        language_pane(
                                            &mut panes[1],
                                            "ภาษาไทย",
                                            "TRANSLATION",
                                            &mut th,
                                            true,
                                            &validation,
                                            &mut copy_token,
                                        );
                                    });
                                    if th != doc.thai(i) {
                                        target_change = Some((i, th.clone()))
                                    }
                                    ui.add_space(10.0);
                                    validation_banner(ui, &validation, th.trim().is_empty());
                                });
                        });
                    });
                if let Some(i) = selected_new {
                    doc.selected = i
                }
                if let Some((i, value)) = target_change {
                    doc.set_thai(i, value)
                }
                if move_delta != 0 {
                    doc.selected = (doc.selected as isize + move_delta)
                        .clamp(0, doc.len().saturating_sub(1) as isize)
                        as usize
                }
                if go_issue {
                    if let Some(i) = next_issue(doc) {
                        doc.selected = i
                    }
                }
                if let Some(token) = copy_token {
                    ctx.copy_text(token.clone());
                    self.toast(ctx, format!("Copied {token}"), false)
                }
                let _ = available;
            });
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ))
        }) {
            self.save_active(ctx)
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                },
                egui::Key::S,
            ))
        }) {
            self.save_all(ctx)
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::O,
            ))
        }) {
            self.choose_folder()
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.confirm_folder.is_some() {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Opening another folder will discard unsaved edits.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_folder = None
                        }
                        if ui
                            .button(RichText::new("Discard and open").color(danger(ui)))
                            .clicked()
                        {
                            let p = self.confirm_folder.take().unwrap();
                            self.open_folder_now(p)
                        }
                    });
                });
        }
        if self.confirm_close {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "{} file(s) still have unsaved edits.",
                        self.dirty_count()
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Keep editing").clicked() {
                            self.confirm_close = false
                        }
                        if ui.button("Save all and quit").clicked() {
                            self.save_all(ctx);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                        if ui
                            .button(RichText::new("Quit without saving").color(danger(ui)))
                            .clicked()
                        {
                            self.confirm_close = false;
                            self.docs.values_mut().for_each(|d| d.dirty = false);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                    });
                });
        }
    }
}
impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.shortcuts(ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && self.dirty_count() > 0
            && !self.confirm_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true
        }
        self.top_bar(ctx);
        self.explorer(ctx);
        self.central(ctx);
        self.dialogs(ctx);
        if let Some(n) = &self.notice {
            if ctx.input(|i| i.time) < n.until {
                egui::Area::new("toast".into())
                    .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -18.0])
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(if n.error {
                                Color32::from_rgb(252, 233, 231)
                            } else {
                                Color32::from_rgb(232, 241, 236)
                            })
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&n.message)
                                        .color(if n.error { danger(ui) } else { success(ui) })
                                        .strong(),
                                );
                            });
                    });
                ctx.request_repaint()
            } else {
                self.notice = None
            }
        }
    }
}
fn language_pane(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    text: &mut String,
    editable: bool,
    validation: &crate::model::Validation,
    copy: &mut Option<String>,
) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(
            1.0_f32,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(8)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .size(15.0)
                        .strong()
                        .color(primary_text(ui)),
                );
                ui.label(
                    RichText::new(subtitle)
                        .size(10.0)
                        .color(ui.visuals().weak_text_color()),
                );
            });
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                let list = tokens(text);
                if list.is_empty() {
                    ui.label(
                        RichText::new("No protected symbols")
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                for token in list {
                    let missing = validation.missing.contains(&token);
                    let chip_bg = if missing {
                        if ui.visuals().dark_mode {
                            Color32::from_rgb(92, 45, 55)
                        } else {
                            Color32::from_rgb(252, 233, 231)
                        }
                    } else if ui.visuals().dark_mode {
                        Color32::from_rgb(68, 71, 90)
                    } else {
                        Color32::from_rgb(229, 242, 252)
                    };
                    let chip_text = if missing {
                        danger(ui)
                    } else if ui.visuals().dark_mode {
                        Color32::from_rgb(139, 233, 253)
                    } else {
                        Color32::from_rgb(23, 95, 157)
                    };
                    let btn = egui::Button::new(
                        RichText::new(&token)
                            .monospace()
                            .size(11.0)
                            .color(chip_text),
                    )
                    .fill(if missing { chip_bg } else { chip_bg })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if missing {
                            danger(ui)
                        } else {
                            Color32::from_rgb(160, 206, 240)
                        },
                    ));
                    if ui.add(btn).on_hover_text("Click to copy").clicked() {
                        *copy = Some(token)
                    }
                }
            });
            ui.separator();
            if editable {
                ui.add_sized(
                    [
                        ui.available_width(),
                        ui.available_height().clamp(260.0, 480.0),
                    ],
                    egui::TextEdit::multiline(text)
                        .font(egui::TextStyle::Body)
                        .desired_width(f32::INFINITY)
                        .lock_focus(true),
                );
            } else {
                ui.add_sized(
                    [
                        ui.available_width(),
                        ui.available_height().clamp(260.0, 480.0),
                    ],
                    egui::TextEdit::multiline(text)
                        .font(egui::TextStyle::Body)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            }
        });
}
fn validation_banner(ui: &mut egui::Ui, v: &crate::model::Validation, empty: bool) {
    let (color, title, detail) = if empty {
        (
            warning(ui),
            "Not translated",
            "Type the Thai translation in the right pane.",
        )
    } else if v.ok() {
        (
            success(ui),
            "Symbols are valid",
            "Variables, style markers, and bracketed names match the source.",
        )
    } else {
        (
            danger(ui),
            "Fix protected symbols",
            v.messages
                .first()
                .map(String::as_str)
                .unwrap_or("Formatting mismatch"),
        )
    };
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            24,
        ))
        .stroke(Stroke::new(1.0_f32, color))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if v.ok() { "✓" } else { "!" })
                        .size(20.0)
                        .strong()
                        .color(color),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).strong());
                    ui.label(
                        RichText::new(detail)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                });
            });
        });
}
fn next_issue(doc: &Document) -> Option<usize> {
    (doc.selected + 1..doc.len())
        .chain(0..=doc.selected)
        .find(|&i| doc.states[i].issue)
}
fn truncate(s: &str, n: usize) -> String {
    let mut out = s.chars().take(n).collect::<String>();
    if s.chars().count() > n {
        out.push('…')
    }
    out.replace(['\r', '\n'], " ")
}
fn welcome(ui: &mut egui::Ui, root: Option<&Path>) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);
        ui.label(RichText::new("RO3 Translation Editor").size(28.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new(if root.is_some() {
                "Choose a TSV file from Explorer to start translating."
            } else {
                "Open a folder once, then browse every TSV file without reopening dialogs."
            })
            .size(15.0)
            .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(18.0);
        ui.label("English source on the left  ·  Thai translation on the right  ·  Ctrl+S to save");
    });
}
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto".into(),
        FontData::from_static(include_bytes!("../assets/NotoSans-Variable.ttf")).into(),
    );
    fonts.font_data.insert(
        "thai".into(),
        FontData::from_static(include_bytes!("../assets/NotoSansThai-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "cjk".into(),
        FontData::from_static(include_bytes!("../assets/NotoSansJP-Regular.otf")).into(),
    );
    let proportional = fonts.families.get_mut(&FontFamily::Proportional).unwrap();
    proportional.insert(0, "cjk".into());
    proportional.insert(0, "thai".into());
    proportional.insert(0, "noto".into());
    let monospace = fonts.families.get_mut(&FontFamily::Monospace).unwrap();
    monospace.push("noto".into());
    monospace.push("thai".into());
    monospace.push("cjk".into());
    ctx.set_fonts(fonts);
}
fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    if dark {
        // Dracula: strong foreground contrast, restrained surfaces, purple focus.
        let bg = Color32::from_rgb(40, 42, 54);
        let raised = Color32::from_rgb(68, 71, 90);
        let foreground = Color32::from_rgb(248, 248, 242);
        let purple = Color32::from_rgb(189, 147, 249);
        visuals.panel_fill = bg;
        visuals.window_fill = bg;
        visuals.extreme_bg_color = Color32::from_rgb(33, 34, 44);
        visuals.faint_bg_color = Color32::from_rgb(49, 51, 65);
        visuals.code_bg_color = raised;
        visuals.override_text_color = Some(foreground);
        visuals.selection.bg_fill = Color32::from_rgb(98, 114, 164);
        visuals.selection.stroke = Stroke::new(1.0_f32, foreground);
        visuals.hyperlink_color = Color32::from_rgb(139, 233, 253);
        visuals.widgets.noninteractive.bg_fill = bg;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, foreground);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, raised);
        visuals.widgets.inactive.bg_fill = raised;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, foreground);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, raised);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(98, 114, 164);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, foreground);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, purple);
        visuals.widgets.active.bg_fill = purple;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, bg);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, purple);
    } else {
        let text = Color32::from_rgb(44, 44, 43);
        let border = Color32::from_rgb(230, 229, 227);
        let blue = Color32::from_rgb(39, 131, 222);
        visuals.panel_fill = Color32::WHITE;
        visuals.window_fill = Color32::WHITE;
        visuals.extreme_bg_color = Color32::from_rgb(247, 247, 245);
        visuals.faint_bg_color = Color32::from_rgb(240, 239, 237);
        visuals.override_text_color = Some(text);
        visuals.selection.bg_fill = Color32::from_rgb(200, 226, 247);
        visuals.selection.stroke = Stroke::new(1.0_f32, text);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(249, 248, 247);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(229, 242, 252);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, blue);
        visuals.widgets.active.bg_fill = blue;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, blue);
    }
    visuals.window_corner_radius = 10.into();
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(11.0, 7.0);
    style
        .text_styles
        .get_mut(&egui::TextStyle::Body)
        .unwrap()
        .size = 15.0;
    ctx.set_style(style);
}
fn settings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "glitzhXEC", "RO3TranslationEditor")
        .map(|d| d.config_dir().join("settings.json"))
}
fn load_settings() -> Settings {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn save_settings(settings: &Settings) {
    if let Some(path) = settings_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            let _ = std::fs::write(path, json);
        }
    }
}
