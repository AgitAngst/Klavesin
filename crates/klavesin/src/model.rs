//! Данные программы: клавиатуры, раскладки, профили, назначения клавиш.
//!
//! Сейчас список клавиатур — выдуманный демо-набор (публичный репозиторий:
//! только выдуманные данные). С 0.3 его заменит опрос USB, демо уберут.

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

/// Геометрия одной клавиши: сколько колонок занимает (1 или 2).
#[derive(Clone, Copy)]
pub struct KeyDef {
    pub units: u8,
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

/// Состояние связи с клавиатурой.
#[derive(Clone, Copy, PartialEq)]
pub enum Conn {
    Online,
    /// На связи не отвечает: жёлтая точка и слово рядом.
    Silent,
}

pub struct Keyboard {
    pub name: String,
    pub conn: Conn,
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

    fn new(name: &str, conn: Conn, cols: u8, units: &[u8], layers: usize, profile: Profile) -> Self {
        let mut kb = Keyboard {
            name: name.to_string(),
            conn,
            layout: Layout { cols, keys: units.iter().map(|&u| KeyDef { units: u }).collect() },
            profiles: Vec::new(),
            profile: 0,
            layer: 0,
            layers: 0,
            sel: None,
        };
        kb.layers = layers;
        kb.profiles.push(profile);
        kb
    }
}

/// Профиль из готовых назначений первого слоя; остальные слои пустые.
fn profile(name: &str, main: bool, first: Vec<Assignment>, keys: usize, layers: usize) -> Profile {
    let mut layers_v = vec![first];
    layers_v.extend((1..layers).map(|_| vec![Assignment::Empty; keys]));
    Profile { name: name.to_string(), main, layers: layers_v }
}

/// Выдуманный демо-набор клавиатур.
pub fn demo() -> Vec<Keyboard> {
    // Studio Pad: три ряда по четыре и последний ряд — широкая клавиша и две
    // обычных: 15 клавиш, 16 колонок.
    let studio_units = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1];
    let mut studio = Keyboard::new(
        "Studio Pad",
        Conn::Online,
        4,
        &studio_units,
        3,
        profile(
            "Стрим",
            true,
            vec![
                Assignment::Key("Esc".into()),
                Assignment::Shortcut("Ctrl+C".into()),
                Assignment::Shortcut("Ctrl+V".into()),
                Assignment::Key("F5".into()),
                Assignment::Layer { layer: 1, hold: true },
                Assignment::Empty,
                Assignment::Text("подпись".into()),
                Assignment::Shortcut("Alt+F4".into()),
                Assignment::Macro("Экспорт".into()),
                Assignment::Key("Space".into()),
                Assignment::Key("Enter".into()),
                Assignment::Key("Mute".into()),
                Assignment::Key("Shift".into()),
                Assignment::Key("FN".into()),
                Assignment::Key("ArrowUp".into()),
            ],
            studio_units.len(),
            3,
        ),
    );
    // Как на доске первого экрана: выбрана пятая клавиша (переключение слоя).
    studio.sel = Some(4);

    let deck = Keyboard::new(
        "Macro Deck",
        Conn::Online,
        4,
        &[1, 1, 1, 1, 1, 1, 1, 1],
        2,
        profile("Работа", true, vec![Assignment::Empty; 8], 8, 2),
    );

    let field = Keyboard::new(
        "Field Pad",
        Conn::Silent,
        3,
        &[1, 1, 1, 1, 1, 1],
        2,
        profile("Монтаж", true, vec![Assignment::Empty; 6], 6, 2),
    );

    vec![studio, deck, field]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_consistent() {
        for kb in demo() {
            assert!(kb.key_count() > 0);
            for p in &kb.profiles {
                assert_eq!(p.layers.len(), kb.layers, "{}: layer count", kb.name);
                for l in &p.layers {
                    assert_eq!(l.len(), kb.key_count(), "{}: key count", kb.name);
                }
            }
            assert_eq!(kb.profiles.iter().filter(|p| p.main).count(), 1, "{}: one main profile", kb.name);
        }
    }

    #[test]
    fn add_layer_extends_every_profile() {
        let mut kb = demo().into_iter().next().unwrap();
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
