//! Klavesin — настройка мини-клавиатур по USB.

// В релизе не поднимаем консольное окно на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anvil_ui::{Accent, CommonSettings, Icon, Kind, ThemeChoice, chrome, widgets as w};
use eframe::egui;

/// Акцент программы — синий набора по умолчанию, пока владелец не выбрал свой.
const ACCENT: Accent = Accent::BLUE;

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

struct Klavesin {
    /// Общие настройки набора: тема, язык строк набора. Хранение — с экраном настроек.
    common: CommonSettings,
}

impl Klavesin {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let common = CommonSettings::default();
        common.apply(&cc.egui_ctx);
        Self { common }
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
