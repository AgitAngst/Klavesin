//! Klavesin — настройка мини-клавиатур по USB.

// В релизе не поднимаем консольное окно на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod device;
mod i18n;
mod input;
mod model;
mod usb;

use anvil_ui::Accent;
use eframe::egui;

/// Акцент программы — сливовый Plum набора (решение владельца 31.10.2026:
/// вместо BLUE). В наборе с тега `kit-v1.1.0` (`Accent::PLUM`).
const ACCENT: Accent = Accent::PLUM;

fn main() -> eframe::Result<()> {
    // Всё, что печатается в консоль, — по-английски.
    if std::env::args().any(|a| a == "--probe") {
        crate::usb::probe();
        return Ok(());
    }
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
            .with_min_inner_size([980.0, 600.0])
            // Знак программы: квадрат акцента со значком клавиатуры (STYLE.md §6).
            .with_icon(anvil_ui::appicon::icon_data(ACCENT, anvil_ui::Icon::Keyboard)),
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
