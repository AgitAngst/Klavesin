//! Данные программы: клавиатуры, раскладки, профили, назначения клавиш.
//!
//! Список клавиатур даёт опрос USB (`usb.rs`); раскладка устройства пока
//! условная — её заменит редактор раскладок (0.4).

/// Что делает клавиша на выбранном слое.
#[derive(Clone, Debug, PartialEq)]
pub enum Assignment {
    Empty,
    /// Одна клавиша: Enter, F5, Mute.
    Key(String),
    /// Сочетание: Ctrl+Shift+S.
    Shortcut(String),
    /// Вставить текст.
    Text(String),
    /// Макрос по имени.
    Macro(String),
    /// Переключить или удерживать слой.
    Layer {
        layer: usize,
        hold: bool,
    },
}

impl Assignment {
    pub fn is_empty(&self) -> bool {
        matches!(self, Assignment::Empty)
    }
}

/// Геометрия одного элемента карты: сколько колонок занимает (1 или 2) и номер
/// управления устройства (клавиша 1..=6, ручка 13/14/15; 0 — не привязан).
#[derive(Clone, Copy)]
pub struct KeyDef {
    pub units: u8,
    pub control: u8,
}

/// Раскладка: число колонок и клавиши по порядку, слева направо, сверху вниз.
#[derive(Clone)]
pub struct Layout {
    pub cols: u8,
    pub keys: Vec<KeyDef>,
}

/// Профиль: именованный набор назначений для всех слоев клавиатуры.
pub struct Profile {
    pub name: String,
    /// Основной профиль — метка словом, одна клавиатура хранит один такой.
    pub main: bool,
    /// [слой][клавиша].
    pub layers: Vec<Vec<Assignment>>,
}

pub struct Keyboard {
    pub name: String,
    /// Техническая подпись устройства (vid:pid), пусто — неизвестно.
    pub detail: String,
    /// Устройство знакомо: умеем писать конфигурацию (протокол `device.rs`).
    pub known: bool,
    pub layout: Layout,
    pub profiles: Vec<Profile>,
    /// Выбранный профиль.
    pub profile: usize,
    /// Выбранный слой (вкладка).
    pub layer: usize,
    /// Число слоёв (у всех профилей одинаковое).
    pub layers: usize,
    /// Выбранная клавиша на карте.
    pub sel: Option<usize>,
}

impl Keyboard {
    pub fn key_count(&self) -> usize {
        self.layout.keys.len()
    }

    /// Назначение выбранного слоя для клавиши.
    pub fn assignment(&self, key: usize) -> &Assignment {
        &self.profiles[self.profile].layers[self.layer][key]
    }

    pub fn assignment_mut(&mut self, key: usize) -> &mut Assignment {
        &mut self.profiles[self.profile].layers[self.layer][key]
    }

    /// Добавить слой во все профили (потолок — `max`).
    pub fn add_layer(&mut self, max: usize) {
        if self.layers >= max {
            return;
        }
        let count = self.key_count();
        for p in &mut self.profiles {
            p.layers.push(vec![Assignment::Empty; count]);
        }
        self.layers += 1;
    }

    /// Новый профиль: пустые назначения на всех слоях.
    pub fn add_profile(&mut self, name: String) {
        let layers = (0..self.layers).map(|_| vec![Assignment::Empty; self.key_count()]).collect();
        self.profiles.push(Profile { name, main: false, layers });
        self.profile = self.profiles.len() - 1;
    }
}

/// Клавиатура из найденного USB-устройства. Знакомое (1189:8890) получает свою
/// карту: три клавиши и ручка (поворот/нажатие/поворот), писать в него умеем.
/// Остальным — условный ряд из трёх клавиш без записи, до редактора раскладок.
pub fn from_found(name: String, detail: String, vid: u16, pid: u16) -> Keyboard {
    let known = vid == crate::device::VID && pid == crate::device::PID;
    let (cols, keys) = if known {
        // Клавиши 1–3; ручка: 13 поворот, 14 нажатие, 15 поворот.
        (
            3,
            vec![
                KeyDef { units: 1, control: 1 },
                KeyDef { units: 1, control: 2 },
                KeyDef { units: 1, control: 3 },
                KeyDef { units: 1, control: 13 },
                KeyDef { units: 1, control: 14 },
                KeyDef { units: 1, control: 15 },
            ],
        )
    } else {
        (3, vec![KeyDef { units: 1, control: 0 }; 3])
    };
    let count = keys.len();
    Keyboard {
        name,
        detail,
        known,
        layout: Layout { cols, keys },
        profiles: vec![Profile {
            name: "Основной".to_string(),
            main: true,
            layers: vec![vec![Assignment::Empty; count]],
        }],
        profile: 0,
        layer: 0,
        layers: 1,
        sel: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_keyboard_is_consistent() {
        let kb = from_found("Test Pad".to_string(), "1234:5678".to_string(), 0x1234, 0x5678);
        assert_eq!(kb.key_count(), 3);
        assert!(!kb.known);
        assert_eq!(kb.layers, 1);
        assert_eq!(kb.profiles.iter().filter(|p| p.main).count(), 1);
        for p in &kb.profiles {
            for l in &p.layers {
                assert_eq!(l.len(), kb.key_count());
            }
        }
    }

    #[test]
    fn known_device_has_keys_and_knob() {
        let kb = from_found("Pad".to_string(), String::new(), 0x1189, 0x8890);
        assert!(kb.known);
        assert_eq!(kb.key_count(), 6);
        assert_eq!(kb.layout.keys[3].control, 13);
        assert_eq!(kb.layout.keys[5].control, 15);
    }

    #[test]
    fn add_layer_extends_every_profile() {
        let mut kb = from_found("Test Pad".to_string(), String::new(), 0, 0);
        let before = kb.layers;
        kb.add_layer(8);
        assert_eq!(kb.layers, before + 1);
        for p in &kb.profiles {
            assert_eq!(p.layers.len(), kb.layers);
            assert_eq!(p.layers.last().unwrap().len(), kb.key_count());
        }
        // Потолок держит.
        for _ in 0..16 {
            kb.add_layer(8);
        }
        assert_eq!(kb.layers, 8);
    }
}
