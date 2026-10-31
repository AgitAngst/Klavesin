//! Протокол конфигурации макропада 1189:8890 («MINI KeyBoard» и варианты).
//!
//! Прошивка хранит назначения во флеше; читать их обратно нельзя — конфигурацию
//! хранит Klavesin и пишет в устройство по кнопке «Записать на клавиатуру».
//! Протокол восстановлен сообществом (реверс vendor app «MINI KeyBoard.exe»):
//! <https://github.com/jgt87/Macropad> (custom/PROTOCOL.md) и
//! <https://github.com/nonatofabio/macrokey-driver>.
//!
//! Канал конфигурации — vendor-defined интерфейс (usage page 0xFF00, usage 0x01,
//! MI_01). Каждая команда — выходной отчёт `[0x03][8 значимых байт][56 нулей]`.
//! Клавиши: 1..=6, ручка: 13/14/15 (поворот/нажатие/поворот). Слои нумеруются с 1.

use hidapi::{HidApi, HidDevice};

pub const VID: u16 = 0x1189;
pub const PID: u16 = 0x8890;

const REPORT_ID: u8 = 0x03;
const REPORT_LEN: usize = 65; // 1 байт Report ID + 64 байта данных

const OP_SAVE: u8 = 0xAA;
const OP_SAVE_LED: u8 = 0xA1;
const OP_SWITCH_LAYER: u8 = 0xA1;
const OP_LED: u8 = 0xB0;

const TYPE_KEY: u8 = 0x01;
const TYPE_MEDIA: u8 = 0x02;
const TYPE_LED: u8 = 0x08;

pub const MOD_CTRL: u8 = 0x01;
pub const MOD_SHIFT: u8 = 0x02;
pub const MOD_ALT: u8 = 0x04;
pub const MOD_GUI: u8 = 0x08;

/// Открытый канал конфигурации устройства.
pub struct Device {
    dev: HidDevice,
}

impl Device {
    /// Найти устройство по vid:pid и открыть его канал конфигурации (0xFF00/0x01).
    pub fn open() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("hid init failed: {e}"))?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0xFF00)
            .ok_or_else(|| "config interface (FF00) not found".to_string())?;
        let dev = api.open_path(info.path()).map_err(|e| format!("open failed: {e}"))?;
        Ok(Self { dev })
    }

    fn send(&self, frame: &[u8; 8]) -> Result<(), String> {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1..9].copy_from_slice(frame);
        self.dev.write(&buf).map(|_| ()).map_err(|e| format!("write failed: {e}"))
    }

    /// Собрать кадры назначения клавише набора нажатий (1..=5: модификатор, код).
    pub fn strokes_frames(control: u8, layer: u8, strokes: &[(u8, u8)]) -> Vec<[u8; 8]> {
        let layer = layer.max(1);
        let lt = (layer << 4) | TYPE_KEY;
        let count = strokes.len().min(5) as u8;
        let mut frames = vec![[OP_SWITCH_LAYER, layer, 0, 0, 0, 0, 0, 0]];
        let first_mod = strokes.first().map(|s| s.0).unwrap_or(0);
        frames.push([control, lt, count, 0, first_mod, 0, 0, 0]);
        for (i, &(m, k)) in strokes.iter().take(5).enumerate() {
            frames.push([control, lt, count, i as u8 + 1, m, k, 0, 0]);
        }
        frames.push([OP_SAVE, OP_SAVE, 0, 0, 0, 0, 0, 0]);
        frames
    }

    /// Собрать кадры мультимедийного назначения (consumer control).
    pub fn media_frame(control: u8, layer: u8, code: u8) -> Vec<[u8; 8]> {
        let layer = layer.max(1);
        vec![
            [OP_SWITCH_LAYER, layer, 0, 0, 0, 0, 0, 0],
            [control, (layer << 4) | TYPE_MEDIA, code, 0, 0, 0, 0, 0],
            [OP_SAVE, OP_SAVE, 0, 0, 0, 0, 0, 0],
        ]
    }

    /// Собрать кадры режима подсветки (только пресеты прошивки, 0..=3).
    pub fn led_frames(layer: u8, mode: u8) -> Vec<[u8; 8]> {
        let layer = layer.max(1);
        vec![
            [OP_SWITCH_LAYER, layer, 0, 0, 0, 0, 0, 0],
            [OP_LED, (layer << 4) | TYPE_LED, mode, 0, 0, 0, 0, 0],
            [OP_SAVE, OP_SAVE_LED, 0, 0, 0, 0, 0, 0],
        ]
    }

    /// Записать кадры устройству.
    pub fn send_frames(&self, frames: &[[u8; 8]]) -> Result<(), String> {
        for f in frames {
            self.send(f)?;
        }
        Ok(())
    }
}

/// Разобрать имя клавиши в код HID (page 0x07). None — не знаем такой клавиши.
pub fn keycode(name: &str) -> Option<u8> {
    let s = name.trim().to_lowercase();
    match s.as_str() {
        "enter" => Some(0x28),
        "esc" | "escape" => Some(0x29),
        "backspace" => Some(0x2A),
        "tab" => Some(0x2B),
        "space" => Some(0x2C),
        "minus" => Some(0x2D),
        "equal" | "=" => Some(0x2E),
        "leftbrace" => Some(0x2F),
        "rightbrace" => Some(0x30),
        "backslash" => Some(0x31),
        "semicolon" | ";" => Some(0x33),
        "apostrophe" | "'" => Some(0x34),
        "grave" | "`" => Some(0x35),
        "comma" | "," => Some(0x36),
        "dot" | "." => Some(0x37),
        "slash" | "/" => Some(0x38),
        "capslock" => Some(0x39),
        "printscreen" => Some(0x46),
        "scrolllock" => Some(0x47),
        "pause" => Some(0x48),
        "insert" => Some(0x49),
        "home" => Some(0x4A),
        "pageup" => Some(0x4B),
        "delete" => Some(0x4C),
        "end" => Some(0x4D),
        "pagedown" => Some(0x4E),
        "right" | "arrowright" => Some(0x4F),
        "left" | "arrowleft" => Some(0x50),
        "down" | "arrowdown" => Some(0x51),
        "up" | "arrowup" => Some(0x52),
        _ => None,
    }
    .or_else(|| {
        if s.len() == 1 {
            let c = s.chars().next().unwrap();
            match c {
                'a'..='z' => Some(0x04 + (c as u8 - b'a')),
                '1'..='9' => Some(0x1E + (c as u8 - b'1')),
                '0' => Some(0x27),
                _ => None,
            }
        } else if let Some(n) = s.strip_prefix('f') {
            n.parse::<u8>().ok().filter(|&n| (1..=12).contains(&n)).map(|n| 0x39 + n)
        } else {
            None
        }
    })
}

/// Мультимедийный код (consumer control) по имени. None — не медиаклавиша.
pub fn media_code(name: &str) -> Option<u8> {
    match name.trim().to_lowercase().as_str() {
        "play" | "playpause" | "play/pause" => Some(0xCD),
        "prev" | "previous" => Some(0xB6),
        "next" => Some(0xB5),
        "stop" => Some(0xB7),
        "mute" => Some(0xE2),
        "volumeup" | "vol+" | "volup" => Some(0xE9),
        "volumedown" | "vol-" | "voldown" => Some(0xEA),
        _ => None,
    }
}

/// Разобрать сочетание вида «Ctrl+Shift+S» в (модификаторы, код клавиши).
/// Медиаклавиши отдельно — их [`media_code`].
pub fn parse_shortcut(text: &str) -> Option<(u8, u8)> {
    let mut mods = 0u8;
    let mut key: Option<String> = None;
    for part in text.split('+') {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "ctrl" | "control" => mods |= MOD_CTRL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" | "gui" | "meta" => mods |= MOD_GUI,
            "" => {}
            _ => {
                if key.is_some() {
                    return None; // больше одной обычной клавиши — не сочетание
                }
                key = Some(p);
            }
        }
    }
    key.as_deref().and_then(keycode).map(|k| (mods, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycodes_cover_letters_digits_and_named() {
        assert_eq!(keycode("a"), Some(0x04));
        assert_eq!(keycode("Z"), Some(0x1D));
        assert_eq!(keycode("1"), Some(0x1E));
        assert_eq!(keycode("0"), Some(0x27));
        assert_eq!(keycode("Enter"), Some(0x28));
        assert_eq!(keycode("F5"), Some(0x3E));
        assert_eq!(keycode("F12"), Some(0x45));
        assert_eq!(keycode("ArrowUp"), Some(0x52));
        assert_eq!(keycode("nope"), None);
    }

    #[test]
    fn media_names() {
        assert_eq!(media_code("Mute"), Some(0xE2));
        assert_eq!(media_code("vol+"), Some(0xE9));
        assert_eq!(media_code("Enter"), None);
    }

    #[test]
    fn shortcuts_parse() {
        assert_eq!(parse_shortcut("Ctrl+C"), Some((MOD_CTRL, 0x06)));
        assert_eq!(parse_shortcut("ctrl+shift+s"), Some((MOD_CTRL | MOD_SHIFT, 0x16)));
        assert_eq!(parse_shortcut("F5"), Some((0, 0x3E)));
        assert_eq!(parse_shortcut("Ctrl+Alt+Delete"), Some((MOD_CTRL | MOD_ALT, 0x4C)));
        assert_eq!(parse_shortcut("Ctrl+C+Z"), None);
        assert_eq!(parse_shortcut("Ctrl+Noise"), None);
    }

    #[test]
    fn strokes_frames_follow_protocol() {
        let frames = Device::strokes_frames(0x02, 1, &[(MOD_CTRL, 0x06)]);
        assert_eq!(frames[0], [0xA1, 1, 0, 0, 0, 0, 0, 0], "layer switch");
        assert_eq!(frames[1], [0x02, 0x11, 1, 0, MOD_CTRL, 0, 0, 0], "header");
        assert_eq!(frames[2], [0x02, 0x11, 1, 1, MOD_CTRL, 0x06, 0, 0], "keystroke");
        assert_eq!(frames[3], [0xAA, 0xAA, 0, 0, 0, 0, 0, 0], "save");
        assert_eq!(frames.len(), 4);
    }

    #[test]
    fn layer_nibble_goes_high() {
        let frames = Device::strokes_frames(0x01, 3, &[(0, 0x28)]);
        assert_eq!(frames[1][1], 0x31, "layer 3, type 1");
    }
}
