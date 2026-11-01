//! Данные программы: клавиатуры, раскладки, профили, назначения клавиш.
//!
//! Список клавиатур даёт опрос USB (`usb.rs`); раскладка устройства пока
//! условная — её заменит редактор раскладок (0.4).

/// Что делает клавиша на выбранном слое.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct KeyDef {
    pub units: u8,
    pub control: u8,
}

/// Раскладка: число колонок и клавиши по порядку, слева направо, сверху вниз.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    pub cols: u8,
    pub keys: Vec<KeyDef>,
}

/// Профиль: именованный набор назначений для всех слоев клавиатуры.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub name: String,
    /// Основной профиль — метка словом, одна клавиатура хранит один такой.
    pub main: bool,
    /// [слой][клавиша].
    pub layers: Vec<Vec<Assignment>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
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

    /// Изменить число колонок раскладки.
    pub fn set_cols(&mut self, cols: u8) {
        self.layout.cols = cols.max(1);
    }

    /// Ширина клавиши: 1 или 2 колонки.
    pub fn set_key_units(&mut self, idx: usize, units: u8) {
        if let Some(k) = self.layout.keys.get_mut(idx) {
            k.units = units.clamp(1, 2);
        }
    }

    /// Добавить клавишу в конец раскладки (не привязана к устройству).
    pub fn add_key(&mut self) {
        self.layout.keys.push(KeyDef { units: 1, control: 0 });
        self.sync_layers();
    }

    /// Убрать клавишу из раскладки вместе с её назначениями.
    pub fn remove_key(&mut self, idx: usize) {
        if self.layout.keys.len() > 1 && idx < self.layout.keys.len() {
            self.layout.keys.remove(idx);
            if self.sel.is_some_and(|s| s >= self.layout.keys.len()) {
                self.sel = None;
            }
            self.sync_layers();
        }
    }

    /// Привести назначения всех профилей к числу клавиш: лишнее обрезать,
    /// недостающее дополнить пустыми.
    fn sync_layers(&mut self) {
        let count = self.key_count();
        for p in &mut self.profiles {
            for l in &mut p.layers {
                l.truncate(count);
                l.resize(count, Assignment::Empty);
            }
        }
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
    fn keyboard_survives_json_roundtrip() {
        let mut kb = from_found("Pad".to_string(), "1189:8890".to_string(), 0x1189, 0x8890);
        kb.add_layer(8);
        *kb.assignment_mut(0) = Assignment::Shortcut("Ctrl+C".into());
        let json = serde_json::to_string(&kb).unwrap();
        let back: Keyboard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, kb.name);
        assert_eq!(back.detail, kb.detail);
        assert!(back.known);
        assert_eq!(back.key_count(), kb.key_count());
        assert_eq!(back.layers, kb.layers);
        assert_eq!(back.profiles.len(), kb.profiles.len());
        assert_eq!(back.profiles[0].layers[0][0], Assignment::Shortcut("Ctrl+C".into()));
    }

    #[test]
    fn layout_editing_syncs_assignments() {
        let mut kb = from_found("Pad".to_string(), String::new(), 0x1189, 0x8890);
        let before = kb.key_count();
        kb.add_key();
        assert_eq!(kb.key_count(), before + 1);
        for p in &kb.profiles {
            for l in &p.layers {
                assert_eq!(l.len(), kb.key_count());
            }
        }
        kb.set_key_units(0, 2);
        assert_eq!(kb.layout.keys[0].units, 2);
        kb.set_cols(4);
        assert_eq!(kb.layout.cols, 4);
        kb.remove_key(0);
        assert_eq!(kb.key_count(), before);
        for p in &kb.profiles {
            for l in &p.layers {
                assert_eq!(l.len(), kb.key_count());
            }
        }
        // Последнюю клавишу удалить нельзя: карта не пустеет.
        for _ in 0..16 {
            kb.remove_key(0);
        }
        assert_eq!(kb.key_count(), 1);
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
