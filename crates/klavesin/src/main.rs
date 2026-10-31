//! Klavesin — настройка мини-клавиатур по USB.

// В релизе не поднимаем консольное окно на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anvil_ui::{Accent, CommonSettings, Icon, Kind, chrome, theme::Swatch, widgets as w};
use eframe::egui;

/// Акцент программы — сливовый, 290° (решение владельца 31.10.2026: вместо BLUE).
/// Контраст не ниже 5.2:1 в обеих темах. Дом здесь, пока не переехал в набор
/// (`Accent::PLUM`, тест контраста и строка STYLE.md §2.2 — с выпуском набора).
const ACCENT: Accent = Accent {
    name: "Plum",
    dark: Swatch {
        fill: egui::Color32::from_rgb(0xCB, 0x81, 0xD9),
        on_fill: egui::Color32::from_rgb(0x17, 0x0A, 0x1A),
        text: egui::Color32::from_rgb(0xE1, 0xA0, 0xEE),
    },
    light: Swatch {
        fill: egui::Color32::from_rgb(0xAA, 0x39, 0xC0),
        on_fill: egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
        text: egui::Color32::from_rgb(0x79, 0x29, 0x89),
    },
};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Klavesin")
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([520.0, 360.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native("Klavesin", options, Box::new(|cc| Ok(Box::new(Klavesin::new(cc)))))
}

struct Klavesin;

impl Klavesin {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Оформление набора: палитра с акцентом, шрифты; тема и язык — из общих настроек.
        // Хранение настроек между запусками — с экраном настроек.
        let common = CommonSettings::default();
        anvil_ui::install(&cc.egui_ctx, ACCENT, common.theme);
        common.apply(&cc.egui_ctx);
        Self
    }
}

impl eframe::App for Klavesin {
    /// Каркас окна — из набора: шапка, содержимое, строка состояния.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        chrome::top_bar(ui, |ui| chrome::brand(ui, Icon::Keyboard, "Klavesin"));
        chrome::content(ui, |ui| {
            w::card(ui, |ui| {
                w::card_title(ui, Icon::Keyboard, "Мини-клавиатуры по USB");
                w::note(ui, "Подключите клавиатуру — и меняйте назначение клавиш, слои и подсветку.");
                ui.add_space(8.0);
                // Пока ничего не ищет: первый настоящий экран — по доске и слову владельца.
                let _ = w::button(ui, Kind::Primary, None, "Найти клавиатуру…").clicked();
            });
        });
        chrome::status_bar(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                w::note(ui, format!("v{}", env!("CARGO_PKG_VERSION")));
            });
        });
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        chrome::clear_color(visuals, ACCENT)
    }
}
