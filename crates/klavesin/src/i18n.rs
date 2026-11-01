//! Строки программы: ключ — русская строка, перевод — английская.
//! Язык — из общих настроек набора; таблицу дополняют вместе с кодом,
//! тест ловит дубликаты и пустые переводы.

use anvil_ui::Lang;

/// Таблица переводов программы.
pub const TABLE: &[(&str, &str)] = &[
    ("Настройки", "Settings"),
    ("О программе", "About"),
    ("Клавиатуры", "Keyboards"),
    ("Профили", "Profiles"),
    ("Обновить список", "Refresh list"),
    ("Ищем клавиатуры…", "Looking for keyboards…"),
    ("Ничего не найдено. Подключите клавиатуру по USB.", "Nothing found. Plug a keyboard in over USB."),
    ("Клавиатур пока нет", "No keyboards yet"),
    (
        "Подключите клавиатуру по USB или нажмите кнопку — Klavesin сам опросит порты и покажет карту клавиш.",
        "Plug a keyboard in over USB or press the button — Klavesin polls the ports itself and shows the key map.",
    ),
    ("Найти клавиатуру…", "Find keyboard…"),
    ("Готов", "Ready"),
    ("подключена", "connected"),
    ("не отвечает", "no reply"),
    ("клавиш", "keys"),
    ("Записать на клавиатуру", "Write to keyboard"),
    ("Записано на клавиатуру", "Written to the keyboard"),
    ("Черновик", "Draft"),
    (
        "Черновик включён: изменения действуют, пока открыта программа. Кнопка справа сохранит их в устройстве.",
        "Draft is on: changes apply while the program is open. The button on the right saves them to the device.",
    ),
    (
        "Черновик выключен: изменения не действуют, пока их не записать на клавиатуру.",
        "Draft is off: changes do not apply until they are written to the keyboard.",
    ),
    ("Слой", "Layer"),
    ("Назначение клавиши", "Key assignment"),
    ("Клавиша", "Key"),
    ("Сочетание", "Shortcut"),
    ("Текст", "Text"),
    ("Макрос", "Macro"),
    ("Мышь", "Mouse"),
    ("Последовательность нажатий, до 5: Ctrl+C; Tab; Enter", "Keystroke sequence, up to 5: Ctrl+C; Tab; Enter"),
    (
        "Кнопка или колесо: Left, Right, Middle, WheelUp, WheelDown",
        "Button or wheel: Left, Right, Middle, WheelUp, WheelDown",
    ),
    ("Убрать назначение", "Clear"),
    ("пока удерживаю", "while held"),
    ("станет постоянным после записи на клавиатуру", "becomes permanent once written to the keyboard"),
    ("Действует на слое", "Active on layer"),
    ("Название клавиши, например Enter", "Key name, e.g. Enter"),
    ("Сочетание, например Ctrl+Shift+S", "Shortcut, e.g. Ctrl+Shift+S"),
    ("Текст для вставки", "Text to type"),
    ("Имя макроса", "Macro name"),
    ("Основной", "Main"),
    ("Новый профиль…", "New profile…"),
    ("Профиль", "Profile"),
    ("Не выбрано", "None"),
    ("Это устройство не поддерживает запись конфигурации.", "This device does not support configuration."),
    ("Не удалось открыть устройство", "Failed to open the device"),
    ("Ошибка записи", "Write error"),
    ("Не записано назначений (устройство не поддерживает):", "Assignments not written (unsupported by the device):"),
    ("Подсветка: выключена", "Backlight: off"),
    ("Подсветка: один светодиод", "Backlight: one LED"),
    ("Подсветка: бегущий огонь", "Backlight: colour cycle"),
    ("Подсветка: пульс", "Backlight: breathing"),
    ("Подсветка записана на устройство.", "Backlight written to the device."),
    ("Раскладка…", "Layout…"),
    ("Редактор раскладки", "Layout editor"),
    ("Колонок", "Columns"),
    ("Удалить клавишу", "Remove key"),
    ("Добавить клавишу", "Add key"),
    ("Показать все HID", "Show all HID"),
    ("Только клавиатуры", "Keyboards only"),
    ("Назначений нет — нечего записывать.", "Nothing to write: no assignments."),
];

/// Перевод ключа на язык интерфейса; перевода нет — возвращается сам ключ.
pub fn tr(lang: Lang, key: &str) -> &str {
    match lang {
        Lang::Ru => key,
        Lang::En => TABLE.iter().find(|(ru, _)| *ru == key).map(|(_, en)| *en).unwrap_or(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for (ru, _) in TABLE {
            assert!(seen.insert(*ru), "duplicate key: {ru}");
        }
    }

    #[test]
    fn translations_are_nonempty_and_differ() {
        for (ru, en) in TABLE {
            assert!(!en.is_empty(), "empty translation for: {ru}");
            assert_ne!(ru, en, "translation copies the key: {ru}");
        }
    }
}
