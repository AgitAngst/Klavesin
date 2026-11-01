//! Чтение нажатий с устройства: фоновый поток слушает клавиатурный
//! HID-интерфейс (boot keyboard, usage page 1 / usage 6) и шлёт в интерфейс
//! пары (модификаторы, код клавиши) для каждого вновь нажатого ключа.
//! Медиаклавиши идут по другому интерфейсу — их здесь нет.

use std::sync::mpsc::Sender;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::device::{PID, VID};

/// Одно нажатие: модификаторы (битовая маска) и HID-код клавиши.
pub type Press = (u8, u8);

/// Открыть клавиатурный интерфейс устройства (не канал конфигурации).
fn open_input(api: &HidApi) -> Option<HidDevice> {
    let info = api
        .device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0x01 && d.usage() == 0x06)?;
    api.open_path(info.path()).ok()
}

/// Кадры boot keyboard-отчёта: [id, mods, reserved, 6 кодов].
fn parse_report(buf: &[u8]) -> (u8, Vec<u8>) {
    if buf.len() < 9 {
        return (0, Vec::new());
    }
    let mods = buf[1];
    let keys = buf[3..9].iter().copied().filter(|&k| k != 0).collect();
    (mods, keys)
}

/// Поток чтения: переподключается сам, шлёт только вновь нажатые клавиши.
pub fn spawn(tx: Sender<Press>) {
    std::thread::Builder::new()
        .name("klavesin-input".into())
        .spawn(move || {
            let mut prev: Vec<u8> = Vec::new();
            loop {
                let Ok(api) = HidApi::new() else {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                };
                let Some(dev) = open_input(&api) else {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                };
                let mut buf = [0u8; 9];
                loop {
                    match dev.read_timeout(&mut buf, 50) {
                        Ok(n) if n >= 9 => {
                            let (mods, keys) = parse_report(&buf);
                            for k in keys.iter().copied().filter(|k| !prev.contains(k)) {
                                let _ = tx.send((mods, k));
                            }
                            prev = keys;
                        }
                        Ok(_) => {}
                        Err(_) => {
                            // Устройство отключили: переподключимся.
                            break;
                        }
                    }
                }
                prev.clear();
                std::thread::sleep(Duration::from_millis(300));
            }
        })
        .expect("spawn input thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_parses_mods_and_keys() {
        // id 1, mods Ctrl, reserved 0, коды F5 (0x3E) и Enter (0x28).
        let buf = [0x01, 0x01, 0x00, 0x3E, 0x28, 0, 0, 0, 0];
        let (mods, keys) = parse_report(&buf);
        assert_eq!(mods, 0x01);
        assert_eq!(keys, vec![0x3E, 0x28]);
    }

    #[test]
    fn empty_report_has_no_keys() {
        let buf = [0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0];
        let (mods, keys) = parse_report(&buf);
        assert_eq!(mods, 0x00);
        assert!(keys.is_empty());
    }
}
