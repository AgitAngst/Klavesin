//! Окно программы: шапка, список клавиатур, карта клавиш, профили, строка состояния.
//! Вид — по docs/ui-guideline.md и доске первого экрана (решения владельца 31.10.2026).

use anvil_ui::{CommonSettings, Icon, Kind, Lang, Palette, ThemeChoice, chrome, semibold, widgets as w};
use eframe::egui::{self, Align, Color32, Layout, Margin, RichText, Stroke, Ui, Vec2};

use crate::i18n::tr;
use crate::model::{Assignment, Keyboard};

/// Сцена окна для снимков: пустое состояние или обычный экран.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scene {
    Normal,
    Empty,
}

/// Разбор аргументов командной строки (снимки, отладка темы и языка).
#[derive(Clone, Copy, Default)]
pub struct Config {
    pub scene: Option<Scene>,
    pub lang: Option<Lang>,
    pub theme: Option<ThemeChoice>,
}

pub fn parse_args(args: impl Iterator<Item = String>) -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let mut flag = arg.as_str();
        // Разрешаем --flag=value рядом с --flag value.
        let inline = flag.split_once('=').map(|(f, v)| (f, Some(v.to_string())));
        let (name, value) = match inline {
            Some((f, v)) if f.starts_with("--") => (f, v),
            _ => (flag, args.next_if(|a| !a.starts_with("--"))),
        };
        flag = name;
        match flag {
            "--scene" => {
                let v = value.as_deref().ok_or_else(|| format!("missing value for {flag}"))?;
                config.scene = Some(match v {
                    "normal" | "main" => Scene::Normal,
                    "empty" => Scene::Empty,
                    other => return Err(format!("unknown scene: {other} (normal|empty)")),
                });
            }
            "--lang" => {
                let v = value.as_deref().ok_or_else(|| format!("missing value for {flag}"))?;
                config.lang = Some(match v {
                    "ru" => Lang::Ru,
                    "en" => Lang::En,
                    other => return Err(format!("unknown language: {other} (ru|en)")),
                });
            }
            "--theme" => {
                let v = value.as_deref().ok_or_else(|| format!("missing value for {flag}"))?;
                config.theme = Some(match v {
                    "light" => ThemeChoice::Light,
                    "dark" => ThemeChoice::Dark,
                    other => return Err(format!("unknown theme: {other} (light|dark)")),
                });
            }
            "--help" | "-h" => {
                return Err("usage: klavesin [--scene normal|empty] [--lang ru|en] [--theme light|dark]".into());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(config)
}

const STORAGE_KEY: &str = "klavesin-v1";

/// Устройство, о котором помним между запусками: раскладка и профили.
/// Ключ — подпись vid:pid (поле detail модели).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SavedDevice {
    key: String,
    layout: crate::model::Layout,
    layers: usize,
    profiles: Vec<crate::model::Profile>,
    profile: usize,
}

/// Сохраняемое между запусками состояние программы.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SavedState {
    common: CommonSettings,
    kb: Option<usize>,
    draft: bool,
    led_mode: u8,
    devices: Vec<SavedDevice>,
}

pub struct App {
    common: CommonSettings,
    keyboards: Vec<Keyboard>,
    /// Выбранная клавиатура; None — пустое состояние.
    kb: Option<usize>,
    /// Черновик: изменения действуют, пока открыта программа.
    draft: bool,
    /// Есть изменения, не записанные на устройство.
    dirty: bool,
    open_settings: bool,
    open_about: bool,
    /// Редактор раскладки выбранного устройства.
    open_layout: bool,
    /// Режим подсветки знакомого устройства (0..=3, пресеты прошивки).
    led_mode: u8,
    /// Последний опрос ничего не нашёл.
    scanned_empty: bool,
    /// Итог последней записи на устройство: пусто — не записывали.
    write_note: String,
    /// Память устройств между запусками: раскладки и профили по vid:pid.
    saved: Vec<SavedDevice>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: &Config) -> Self {
        // Оформление набора: палитра с акцентом, шрифты; тема и язык — из общих настроек.
        // Состояние прошлого запуска: тема, язык, профили устройств.
        let loaded: Option<SavedState> =
            cc.storage.and_then(|s| s.get_string(STORAGE_KEY)).and_then(|json| serde_json::from_str(&json).ok());
        let mut common = loaded.as_ref().map(|s| s.common.clone()).unwrap_or_default();
        if let Some(lang) = config.lang {
            common.language = lang;
        }
        if let Some(theme) = config.theme {
            common.theme = theme;
        }
        anvil_ui::install(&cc.egui_ctx, crate::ACCENT, common.theme);
        common.apply(&cc.egui_ctx);
        let (saved, kb0, draft0, led0) = loaded.map(|s| (s.devices, s.kb, s.draft, s.led_mode)).unwrap_or_default();
        let mut app = Self {
            common,
            keyboards: Vec::new(),
            kb: None,
            draft: draft0,
            dirty: false,
            open_settings: false,
            open_about: false,
            open_layout: false,
            led_mode: led0,
            scanned_empty: false,
            write_note: String::new(),
            saved,
        };
        if config.scene != Some(Scene::Empty) {
            app.rescan();
            // Выбор клавиатуры из прошлого запуска, если она ещё на месте.
            if app.kb.is_some()
                && let Some(k) = kb0
                && k < app.keyboards.len()
            {
                app.kb = Some(k);
            }
        }
        app
    }

    fn lang(&self) -> Lang {
        self.common.language
    }

    /// Опросить USB и перестроить список клавиатур из найденного. Если среди
    /// устройств есть знакомое (умеем писать конфигурацию) — выбираем его.
    fn rescan(&mut self) {
        let found = crate::usb::scan();
        self.keyboards = found.into_iter().map(|f| crate::model::from_found(f.name, f.detail, f.vid, f.pid)).collect();
        // Возвращаем устройствам их раскладки и профили из памяти.
        for kb in &mut self.keyboards {
            if let Some(s) = self.saved.iter().find(|s| s.key == kb.detail) {
                kb.layout = s.layout.clone();
                kb.layers = s.layers;
                kb.profiles = s.profiles.clone();
                kb.profile = s.profile.min(kb.profiles.len().saturating_sub(1));
                kb.layer = kb.layer.min(kb.layers.saturating_sub(1));
            }
        }
        self.kb = if self.keyboards.is_empty() {
            None
        } else {
            Some(self.keyboards.iter().position(|k| k.known).unwrap_or(0))
        };
        self.scanned_empty = self.keyboards.is_empty();
    }

    /// Редактор раскладки: колонки, ширина клавиш, добавление и удаление.
    /// Изменения применяются сразу, назначения подравниваются под новую карту.
    fn layout_editor(&mut self, ui: &mut Ui, i: usize, lang: Lang) {
        let cols = self.keyboards[i].layout.cols;
        let mut new_cols = cols;
        let col_labels: Vec<String> = (1..=8).map(|n| n.to_string()).collect();
        let col_options: Vec<(u8, &str)> =
            col_labels.iter().enumerate().map(|(n, s)| ((n + 1) as u8, s.as_str())).collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "Колонок")).color(Palette::of(ui).text));
            if w::select(ui, &mut new_cols, &col_options, 72.0).changed() {
                self.keyboards[i].set_cols(new_cols);
                self.dirty = true;
            }
        });
        ui.add_space(6.0);
        let count = self.keyboards[i].layout.keys.len();
        for idx in 0..count {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", tr(lang, "Клавиша"), idx + 1)).color(Palette::of(ui).text));
                let mut units = self.keyboards[i].layout.keys[idx].units;
                if w::segmented(ui, &mut units, &[(1u8, None, "1"), (2u8, None, "2")]).changed() {
                    self.keyboards[i].set_key_units(idx, units);
                    self.dirty = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if w::icon_button(ui, Icon::Trash, tr(lang, "Удалить клавишу")).clicked() {
                        self.keyboards[i].remove_key(idx);
                        self.dirty = true;
                    }
                });
            });
        }
        if w::button(ui, Kind::Ghost, Some(Icon::Plus), tr(lang, "Добавить клавишу")).clicked() {
            self.keyboards[i].add_key();
            self.dirty = true;
        }
    }

    /// Применить режим подсветки к знакомому устройству (пресеты прошивки).
    fn set_led_mode(&mut self, mode: u8) {
        let lang = self.lang();
        self.led_mode = mode;
        match crate::device::Device::open() {
            Ok(dev) => {
                let frames = crate::device::Device::led_frames(1, mode);
                self.write_note = match dev.send_frames(&frames) {
                    Ok(()) => tr(lang, "Подсветка записана на устройство.").to_string(),
                    Err(e) => format!("{}: {e}", tr(lang, "Ошибка записи")),
                };
            }
            Err(e) => self.write_note = format!("{}: {e}", tr(lang, "Не удалось открыть устройство")),
        }
    }

    /// Записать назначения выбранного профиля в устройство.
    ///
    /// Знакомые устройства (протокол `device.rs`) принимают конфигурацию;
    /// текст и макросы прошивка не поддерживает — честно пропускаем и считаем.
    fn write_device(&mut self) {
        let lang = self.lang();
        let Some(i) = self.kb else { return };
        if !self.keyboards[i].known {
            self.write_note = tr(lang, "Это устройство не поддерживает запись конфигурации.").to_string();
            return;
        }
        let dev = match crate::device::Device::open() {
            Ok(dev) => dev,
            Err(e) => {
                self.write_note = format!("{}: {e}", tr(lang, "Не удалось открыть устройство"));
                return;
            }
        };
        let kb = &self.keyboards[i];
        let mut skipped = 0usize;
        let mut failed: Option<String> = None;
        'outer: for layer in 0..kb.layers {
            for (slot, key) in kb.layout.keys.iter().enumerate() {
                if key.control == 0 {
                    continue;
                }
                let a = &kb.profiles[kb.profile].layers[layer][slot];
                match assignment_frames(key.control, layer as u8 + 1, a) {
                    Ok(frames) if frames.is_empty() => {}
                    Ok(frames) => {
                        if let Err(e) = dev.send_frames(&frames) {
                            failed = Some(e);
                            break 'outer;
                        }
                    }
                    Err(()) => skipped += 1,
                }
            }
        }
        self.write_note = if let Some(e) = failed {
            format!("{}: {e}", tr(lang, "Ошибка записи"))
        } else if skipped > 0 {
            format!("{} {skipped}", tr(lang, "Не записано назначений (устройство не поддерживает):"))
        } else {
            tr(lang, "Записано на клавиатуру").to_string()
        };
        self.dirty = false;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let lang = self.lang();

        chrome::top_bar(ui, |ui| {
            chrome::brand(ui, Icon::Keyboard, "Klavesin");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if w::icon_button(ui, Icon::Info, tr(lang, "О программе")).clicked() {
                    self.open_about = true;
                }
                if w::icon_button(ui, Icon::Gear, tr(lang, "Настройки")).clicked() {
                    self.open_settings = true;
                }
            });
        });
        self.status_bar(ui);

        // Панели справа и слева — до центральной области.
        if self.kb.is_some() {
            self.profiles_panel(ui);
        }
        self.keyboards_panel(ui);
        chrome::content(ui, |ui| self.central(ui));
        self.dialogs(ui.ctx());
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        chrome::clear_color(visuals, crate::ACCENT)
    }

    /// Сохранение между запусками: тема, язык, выбор, черновик, устройства
    /// с их раскладками и профилями. Устройство конфигурацию не отдаёт —
    /// хранит программа.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.saved = self
            .keyboards
            .iter()
            .map(|kb| SavedDevice {
                key: kb.detail.clone(),
                layout: kb.layout.clone(),
                layers: kb.layers,
                profiles: kb.profiles.clone(),
                profile: kb.profile,
            })
            .collect();
        let state = SavedState {
            common: self.common.clone(),
            kb: self.kb,
            draft: self.draft,
            led_mode: self.led_mode,
            devices: self.saved.clone(),
        };
        if let Ok(json) = serde_json::to_string(&state) {
            storage.set_string(STORAGE_KEY, json);
        }
    }
}

impl App {
    fn keyboards_panel(&mut self, ui: &mut Ui) {
        let lang = self.lang();
        chrome::side_panel(ui, "keyboards", 236.0, |ui| {
            w::section_label(ui, tr(lang, "Клавиатуры"));
            let mut pick = None;
            for (i, kb) in self.keyboards.iter().enumerate() {
                let tone = w::Tone::Success;
                let badge = format!("{} {}", kb.key_count(), tr(lang, "клавиш"));
                if w::nav_item(ui, self.kb == Some(i), tone, &kb.name, Some((&badge, w::Tone::Neutral))).clicked() {
                    pick = Some(i);
                }
            }
            if let Some(i) = pick {
                self.kb = Some(i);
            }
            ui.add_space(ui.available_height() - 38.0);
            if w::button(ui, Kind::Ghost, None, tr(lang, "Обновить список")).clicked() {
                self.rescan();
            }
        });
    }

    fn profiles_panel(&mut self, ui: &mut Ui) {
        let Some(i) = self.kb else { return };
        let lang = self.lang();
        let p = Palette::of(ui);
        egui::Panel::right(egui::Id::new("profiles"))
            .resizable(false)
            .exact_size(220.0)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin::symmetric(10, 14)))
            .show(ui, |ui| {
                w::section_label(ui, tr(lang, "Профили"));
                let mut pick = None;
                {
                    let kb = &self.keyboards[i];
                    for (n, prof) in kb.profiles.iter().enumerate() {
                        let trailing = prof.main.then(|| (tr(lang, "Основной"), w::Tone::Accent));
                        if w::nav_item(ui, kb.profile == n, w::Tone::Neutral, &prof.name, trailing).clicked() {
                            pick = Some(n);
                        }
                    }
                }
                if let Some(n) = pick {
                    self.keyboards[i].profile = n;
                }
                ui.add_space(ui.available_height() - 38.0);
                if w::button(ui, Kind::Ghost, Some(Icon::Plus), tr(lang, "Новый профиль…")).clicked() {
                    let n = self.keyboards[i].profiles.len() + 1;
                    self.keyboards[i].add_profile(format!("{} {}", tr(lang, "Профиль"), n));
                    self.dirty = true;
                }
            });
    }

    fn central(&mut self, ui: &mut Ui) {
        let Some(i) = self.kb else {
            self.empty(ui);
            return;
        };
        let lang = self.lang();
        self.page_head(ui, i, lang);
        self.layers_tabs(ui, i, lang);
        let touched = w::card(ui, |ui| self.keymap(ui, i, lang));
        if touched {
            self.dirty = true;
        }
        if let Some(key) = self.keyboards[i].sel {
            let touched = w::card(ui, |ui| self.assignment_card(ui, i, key, lang));
            if touched {
                self.dirty = true;
            }
        }
    }

    fn page_head(&mut self, ui: &mut Ui, i: usize, lang: Lang) {
        let (name, count, detail) = {
            let kb = &self.keyboards[i];
            let mut name = kb.name.clone();
            // Длинные имена устройств обрезаем, чтобы шапка не теснила действия.
            if name.chars().count() > 26 {
                name = format!("{}…", name.chars().take(25).collect::<String>());
            }
            (name, kb.key_count(), kb.detail.clone())
        };
        ui.horizontal(|ui| {
            let p = Palette::of(ui);
            ui.label(RichText::new(name).font(semibold(24.0)).color(p.text));
            w::badge(ui, &format!("{count} {}", tr(lang, "клавиш")), w::Tone::Neutral);
            w::badge(ui, "USB", w::Tone::Accent);
            if !detail.is_empty() {
                w::mono(ui, &detail, None);
            }
        });
        // Действия — своей строкой, прижатой вправо: в узком окне не теснят имя и бейджи.
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let dirty = self.dirty;
                ui.add_enabled_ui(!dirty, |ui| {
                    if w::button(ui, Kind::Primary, None, tr(lang, "Записать на клавиатуру")).clicked()
                    {
                        self.write_device();
                    }
                });
                w::switch(ui, &mut self.draft, tr(lang, "Черновик"));
                // Заводские раскладки знакомых устройств не правим.
                if !self.keyboards[i].known && w::button(ui, Kind::Secondary, None, tr(lang, "Раскладка…")).clicked()
                {
                    self.open_layout = true;
                }
            });
        });
        if !self.write_note.is_empty() {
            w::note(ui, self.write_note.clone());
        }
        if self.keyboards[i].known {
            let modes = [
                tr(lang, "Подсветка: выключена"),
                tr(lang, "Подсветка: один светодиод"),
                tr(lang, "Подсветка: бегущий огонь"),
                tr(lang, "Подсветка: пульс"),
            ];
            let options: Vec<(u8, &str)> = modes.iter().enumerate().map(|(n, s)| (n as u8, *s)).collect();
            let mut mode = self.led_mode;
            if w::select(ui, &mut mode, &options, 240.0).changed() {
                self.set_led_mode(mode);
            }
        }
        let note = if self.draft {
            tr(
                lang,
                "Черновик включён: изменения действуют, пока открыта программа. Кнопка справа сохранит их в устройстве.",
            )
        } else {
            tr(lang, "Черновик выключен: изменения не действуют, пока их не записать на клавиатуру.")
        };
        w::note(ui, note);
    }

    fn layers_tabs(&mut self, ui: &mut Ui, i: usize, lang: Lang) {
        let mut add = false;
        ui.horizontal(|ui| {
            let kb = &mut self.keyboards[i];
            let labels: Vec<String> = (0..kb.layers).map(|n| format!("{} {}", tr(lang, "Слой"), n + 1)).collect();
            let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            w::tabs(ui, &mut kb.layer, &refs);
            if kb.layers < 8 && w::button(ui, Kind::Ghost, Some(Icon::Plus), "").clicked() {
                add = true;
            }
        });
        if add {
            self.keyboards[i].add_layer(8);
            self.dirty = true;
        }
    }

    /// Карта клавиш: ряды набираются по колонкам раскладки.
    fn keymap(&mut self, ui: &mut Ui, i: usize, lang: Lang) -> bool {
        let cols = self.keyboards[i].layout.cols as f32;
        let units: Vec<u8> = self.keyboards[i].layout.keys.iter().map(|k| k.units).collect();
        let mut rows: Vec<Vec<(usize, u8)>> = Vec::new();
        let mut row: Vec<(usize, u8)> = Vec::new();
        let mut acc = 0u8;
        for (idx, u) in units.iter().enumerate() {
            row.push((idx, *u));
            acc += u;
            if acc >= self.keyboards[i].layout.cols {
                rows.push(std::mem::take(&mut row));
                acc = 0;
            }
        }
        if !row.is_empty() {
            rows.push(row);
        }

        let height = 64.0;
        let gap = 8.0;
        for r in &rows {
            ui.horizontal(|ui| {
                let total = ui.available_width();
                for (idx, u) in r {
                    let width = (total * *u as f32 / cols - gap).max(44.0);
                    let sel = self.keyboards[i].sel == Some(*idx);
                    let assignment = self.keyboards[i].assignment(*idx).clone();
                    let resp = key_cap(ui, Vec2::new(width, height), sel, &assignment, lang);
                    if resp.clicked() {
                        self.keyboards[i].sel = Some(*idx);
                    }
                }
            });
            ui.add_space(2.0);
        }
        false
    }

    /// Карточка назначения выбранной клавиши. Возвращает, менялось ли что-нибудь.
    fn assignment_card(&mut self, ui: &mut Ui, i: usize, key: usize, lang: Lang) -> bool {
        let mut changed = false;
        w::card_title(ui, Icon::Keyboard, &format!("{} {}", tr(lang, "Назначение клавиши"), key + 1));

        let kinds = ["Клавиша", "Сочетание", "Текст", "Макрос", "Слой"];
        let options: Vec<(Option<usize>, Option<Icon>, &str)> =
            kinds.iter().enumerate().map(|(n, s)| (Some(n), None, tr(lang, s))).collect();
        let mut kind = kind_of(self.keyboards[i].assignment(key));
        if w::segmented(ui, &mut kind, &options).changed()
            && let Some(k) = kind
        {
            set_kind(&mut self.keyboards[i], key, k);
            changed = true;
        }

        ui.add_space(8.0);
        let kb_layers = self.keyboards[i].layers;
        ui.horizontal(|ui| {
            let kb = &mut self.keyboards[i];
            match kb.assignment_mut(key) {
                Assignment::Key(s) => {
                    changed |= field(ui, s, tr(lang, "Название клавиши, например Enter")).changed();
                }
                Assignment::Shortcut(s) => {
                    changed |= field(ui, s, tr(lang, "Сочетание, например Ctrl+Shift+S")).changed();
                }
                Assignment::Text(s) => {
                    changed |= field(ui, s, tr(lang, "Текст для вставки")).changed();
                }
                Assignment::Macro(s) => {
                    changed |= field(ui, s, tr(lang, "Имя макроса")).changed();
                }
                Assignment::Layer { layer, hold } => {
                    let labels: Vec<String> =
                        (0..kb_layers).map(|n| format!("{} {}", tr(lang, "Слой"), n + 1)).collect();
                    let options: Vec<(usize, &str)> = labels.iter().enumerate().map(|(n, s)| (n, s.as_str())).collect();
                    changed |= w::select(ui, layer, &options, 200.0).changed();
                    changed |= w::toggle(ui, hold, tr(lang, "пока удерживаю")).changed();
                }
                Assignment::Empty => {
                    w::note(ui, tr(lang, "Не выбрано"));
                }
            }
        });

        if !self.keyboards[i].assignment(key).is_empty()
            && w::button(ui, Kind::Ghost, None, tr(lang, "Убрать назначение")).clicked()
        {
            *self.keyboards[i].assignment_mut(key) = Assignment::Empty;
            changed = true;
        }
        let layer = self.keyboards[i].layer + 1;
        w::note(
            ui,
            format!(
                "{} {} · {}",
                tr(lang, "Действует на слое"),
                layer,
                tr(lang, "станет постоянным после записи на клавиатуру")
            ),
        );
        changed
    }

    fn empty(&mut self, ui: &mut Ui) {
        let lang = self.lang();
        w::empty_state(
            ui,
            Icon::Keyboard,
            tr(lang, "Клавиатур пока нет"),
            tr(
                lang,
                "Подключите клавиатуру по USB или нажмите кнопку — Klavesin сам опросит порты и покажет карту клавиш.",
            ),
        );
        ui.add_space(12.0);
        if ui.vertical_centered(|ui| w::button(ui, Kind::Primary, None, tr(lang, "Найти клавиатуру…")).clicked()).inner
        {
            self.rescan();
        }
        if self.scanned_empty {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| w::note(ui, tr(lang, "Ничего не найдено. Подключите клавиатуру по USB.")));
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let lang = self.lang();
        chrome::status_bar(ui, |ui| {
            match self.kb {
                Some(i) if !self.keyboards.is_empty() => {
                    let kb = &self.keyboards[i];
                    w::dot(ui, w::Tone::Success);
                    ui.label(
                        RichText::new(format!("{} {}", kb.name, tr(lang, "подключена"))).color(Palette::of(ui).weak),
                    );
                }
                _ => {
                    ui.label(RichText::new(tr(lang, "Готов")).color(Palette::of(ui).weak));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                w::mono(ui, &format!("v{}", env!("CARGO_PKG_VERSION")), None);
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        if self.open_settings {
            let mut open = self.open_settings;
            let mut changed = false;
            chrome::dialog(ctx, "klavesin-settings", tr(lang, "Настройки"), 420.0, &mut open, |ui| {
                changed = chrome::common_settings(ui, &mut self.common);
            });
            self.open_settings = open;
            if changed {
                let common = self.common.clone();
                common.apply(ctx);
            }
        }
        if self.open_layout {
            let mut open = self.open_layout;
            let i = self.kb;
            chrome::dialog(
                ctx,
                "klavesin-layout",
                tr(lang, "Редактор раскладки"),
                460.0,
                &mut open,
                |ui| {
                    if let Some(i) = i {
                        self.layout_editor(ui, i, lang);
                    }
                },
            );
            self.open_layout = open;
        }
        let mut open = self.open_about;
        let info = chrome::AppInfo {
            name: "Klavesin",
            icon: Icon::Keyboard,
            version: env!("CARGO_PKG_VERSION"),
            tagline: tr(lang, "Настройка мини-клавиатур по USB"),
            repository: env!("CARGO_PKG_REPOSITORY"),
        };
        let _ = chrome::about(ctx, &mut open, &info, None);
        self.open_about = open;
    }
}

/// Собрать кадры протокола для назначения. Ok(vec![]) — пустое, не трогаем
/// устройство; Err(()) — прошивка такое не поддерживает (текст, макросы, слои,
/// неразбираемое имя клавиши).
fn assignment_frames(control: u8, layer: u8, a: &Assignment) -> Result<Vec<[u8; 8]>, ()> {
    let strokes = |m: u8, k: u8| crate::device::Device::strokes_frames(control, layer, &[(m, k)]);
    match a {
        Assignment::Empty => Ok(Vec::new()),
        Assignment::Key(s) => {
            if let Some(code) = crate::device::media_code(s) {
                Ok(crate::device::Device::media_frame(control, layer, code))
            } else {
                crate::device::parse_shortcut(s).map(|(m, k)| strokes(m, k)).ok_or(())
            }
        }
        Assignment::Shortcut(s) => crate::device::parse_shortcut(s).map(|(m, k)| strokes(m, k)).ok_or(()),
        Assignment::Text(_) | Assignment::Macro(_) | Assignment::Layer { .. } => Err(()),
    }
}

/// Клавиша на карте: подпись действия и род действия мелко; выбранная — рамка и
/// подложка акцента (STYLE.md §7 «Элементы»). Рисуется на выделенном прямоугольнике,
/// чтобы клавиша держала ширину клетки раскладки.
fn key_cap(ui: &mut Ui, size: Vec2, selected: bool, assignment: &Assignment, lang: Lang) -> egui::Response {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let (fill, stroke, stroke_w, main_color) = if selected {
        (p.soft(p.accent), p.accent, 1.5, p.accent_text)
    } else if hovered {
        (p.hover, p.border_strong, 1.0, p.text)
    } else {
        (p.raised, p.border, 1.0, p.text)
    };
    let painter = ui.painter_at(rect);
    painter.rect(rect, egui::CornerRadius::same(6), fill, Stroke::new(stroke_w, stroke), egui::StrokeKind::Inside);
    let (main, sub) = cap_labels(assignment, lang, main_color, p.weak);
    painter.text(rect.min + Vec2::new(10.0, 8.0), egui::Align2::LEFT_TOP, main.0, main.1, main.2);
    if let Some((text, color)) = sub {
        painter.text(
            rect.min + Vec2::new(10.0, 40.0),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(11.0),
            color,
        );
    }
    response
}

fn cap_labels(
    assignment: &Assignment,
    lang: Lang,
    main: Color32,
    weak: Color32,
) -> ((String, egui::FontId, Color32), Option<(String, Color32)>) {
    let sub = |word: &str| Some((word.to_string(), weak));
    let prop = |s: &str| (s.to_string(), egui::FontId::proportional(13.0), main);
    let mono = |s: &str| (s.to_string(), egui::FontId::monospace(13.0), main);
    match assignment {
        Assignment::Empty => (("—".to_string(), egui::FontId::proportional(13.0), weak), None),
        Assignment::Key(s) => (mono(s), sub(tr(lang, "Клавиша"))),
        Assignment::Shortcut(s) => (mono(s), sub(tr(lang, "Сочетание"))),
        Assignment::Text(s) => (prop(s), sub(tr(lang, "Текст"))),
        Assignment::Macro(s) => (prop(s), sub(tr(lang, "Макрос"))),
        Assignment::Layer { layer, hold } => {
            let text = format!("{} {}", tr(lang, "Слой"), layer + 1);
            let sub = if *hold { sub(tr(lang, "пока удерживаю")) } else { None };
            ((text, semibold(13.0), main), sub)
        }
    }
}

/// Поле ввода набора: фон `field`, рамка `border_strong`, высота 30.
fn field(ui: &mut Ui, text: &mut String, placeholder: &str) -> egui::Response {
    let p = Palette::of(ui);
    egui::Frame::new()
        .fill(p.field)
        .stroke(Stroke::new(1.0, p.border_strong))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(10, 2))
        .show(ui, |ui| {
            ui.add_sized(
                [260.0, 26.0],
                egui::TextEdit::singleline(text).hint_text(placeholder).font(egui::FontId::proportional(14.0)),
            )
        })
        .inner
}

/// Номер вида действия: 0 — клавиша, 1 — сочетание, 2 — текст, 3 — макрос, 4 — слой.
fn kind_of(assignment: &Assignment) -> Option<usize> {
    match assignment {
        Assignment::Empty => None,
        Assignment::Key(_) => Some(0),
        Assignment::Shortcut(_) => Some(1),
        Assignment::Text(_) => Some(2),
        Assignment::Macro(_) => Some(3),
        Assignment::Layer { .. } => Some(4),
    }
}

/// Сменить вид действия, сохранив строку значения, если она была.
fn set_kind(kb: &mut Keyboard, key: usize, kind: usize) {
    let prev = match kb.assignment(key) {
        Assignment::Key(s) | Assignment::Shortcut(s) | Assignment::Text(s) | Assignment::Macro(s) => s.clone(),
        _ => String::new(),
    };
    *kb.assignment_mut(key) = match kind {
        0 => Assignment::Key(prev),
        1 => Assignment::Shortcut(prev),
        2 => Assignment::Text(prev),
        3 => Assignment::Macro(prev),
        _ => Assignment::Layer { layer: 0, hold: true },
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_state_serializes() {
        let state =
            SavedState { common: CommonSettings::default(), kb: Some(0), draft: true, led_mode: 2, devices: vec![] };
        let json = serde_json::to_string(&state).expect("SavedState must serialize");
        let back: SavedState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.led_mode, 2);
        assert_eq!(back.kb, Some(0));
    }
}
