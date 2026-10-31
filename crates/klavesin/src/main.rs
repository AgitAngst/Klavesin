//! Klavesin — настройка мини-клавиатур по USB.

// В релизе не поднимаем консольное окно на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod device;
mod i18n;
mod model;
mod usb;

use anvil_ui::{Accent, theme::Swatch};
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
    // Всё, что печатается в консоль, — по-английски.
    let config = match app::parse_args(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Klavesin")
            .with_inner_size([1160.0, 780.0])
            .with_min_inner_size([980.0, 600.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native("Klavesin", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, &config)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse_scenes() {
        let config = app::parse_args(
            ["--scene".to_string(), "empty".to_string(), "--lang".to_string(), "en".to_string()].into_iter(),
        )
        .unwrap();
        assert_eq!(config.scene, Some(app::Scene::Empty));
        assert_eq!(config.lang, Some(anvil_ui::Lang::En));
        // Значение можно прицепить к флагу через '='.
        let config = app::parse_args(["--theme=dark".to_string()].into_iter()).unwrap();
        assert_eq!(config.theme, Some(anvil_ui::ThemeChoice::Dark));
        // Неизвестный флаг — ошибка.
        assert!(app::parse_args(["--nope".to_string()].into_iter()).is_err());
    }
}
