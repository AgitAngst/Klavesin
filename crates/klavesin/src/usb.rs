//! Опрос USB HID: поиск подключённых мини-клавиатур.
//!
//! Windows выдаёт за устройства ещё «корневые коллекции» с нулевыми vid:pid —
//! их пропускаем. Системные клавиатуры и мыши (usage page Generic Desktop,
//! usage Pointer/Mouse) не считаем мини-клавиатурами. Макропады обычно живут
//! на странице Keyboard (1, 6) или на vendor-defined (0xFF00+) — их берём;
//! строк у таких устройств часто нет — тогда имя собираем из vid:pid.

/// Найденное устройство.
pub struct Found {
    /// Имя для списка: продукт, иначе производитель, иначе «USB HID vid:pid».
    pub name: String,
    /// Техническая подпись: vid:pid моноширинным.
    pub detail: String,
    pub vid: u16,
    pub pid: u16,
}

/// Опросить USB HID. Без доступа к HID — пустой список, не ошибка.
pub fn scan() -> Vec<Found> {
    let Ok(api) = hidapi::HidApi::new() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for d in api.device_list() {
        // Корневые коллекции Windows — не устройства.
        if d.vendor_id() == 0 && d.product_id() == 0 {
            continue;
        }
        if !keep(d.usage_page(), d.usage()) {
            continue;
        }
        let detail = format!("{:04X}:{:04X}", d.vendor_id(), d.product_id());
        let name = d
            .product_string()
            .or_else(|| d.manufacturer_string())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("USB HID {detail}"));
        found.push(Found { name, detail, vid: d.vendor_id(), pid: d.product_id() });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found.dedup_by(|a, b| a.name == b.name && a.detail == b.detail);
    found
}

/// Входит ли устройство в список: не системный указатель и не мышь.
fn keep(usage_page: u16, usage: u16) -> bool {
    !matches!((usage_page, usage), (0x01, 0x01) | (0x01, 0x02))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_keeps_keyboards_and_vendor_devices() {
        assert!(keep(0x01, 0x06), "keyboard (macropads live here too)");
        assert!(keep(0xFF60, 0x01), "vendor-defined page");
        assert!(!keep(0x01, 0x01), "pointer");
        assert!(!keep(0x01, 0x02), "mouse");
    }
}
