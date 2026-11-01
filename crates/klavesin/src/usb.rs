//! Опрос USB HID: поиск подключённых мини-клавиатур.
//!
//! Windows выдаёт за устройства ещё «корневые коллекции» с нулевыми vid:pid —
//! их пропускаем. По умолчанию список короткий: знакомые устройства (умеем
//! писать конфигурацию) и vendor-defined коллекции (0xFF00+, так живут
//! макропады) — без системной клавиатуры, тачпада и прочего HID мышинки.
//! Режим «показать все HID» выводит и остальное, кроме мышей и тачскринов.
//! Строк у макропадов часто нет — тогда имя собираем из vid:pid.

/// Найденное устройство.
pub struct Found {
    /// Имя для списка: продукт, иначе производитель, иначе «USB HID vid:pid».
    pub name: String,
    /// Техническая подпись: vid:pid моноширинным.
    pub detail: String,
    pub vid: u16,
    pub pid: u16,
}

/// Опросить USB HID. `show_all` — показать и HID без признаков клавиатуры.
/// Без доступа к HID — пустой список, не ошибка.
pub fn scan(show_all: bool) -> Vec<Found> {
    let Ok(api) = hidapi::HidApi::new() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for d in api.device_list() {
        // Корневые коллекции Windows — не устройства.
        if d.vendor_id() == 0 && d.product_id() == 0 {
            continue;
        }
        if !keep(d, show_all) {
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

/// Входит ли устройство в список. Мыши, указатели и тачскрины (digitizer)
/// скрыты всегда; прочий «не-клавиатурный» HID — только в режиме show_all.
fn keep(d: &hidapi::DeviceInfo, show_all: bool) -> bool {
    let page = d.usage_page();
    if matches!((page, d.usage()), (0x01, 0x01) | (0x01, 0x02)) {
        return false; // pointer / mouse
    }
    if page == 0x0D {
        return false; // digitizer: тачпады и тачскрины
    }
    if show_all {
        return true;
    }
    // Короткий список: знакомое устройство или vendor-defined коллекция.
    (d.vendor_id() == crate::device::VID && d.product_id() == crate::device::PID) || page >= 0xFF00
}

/// Отладочный дамп всех HID-устройств (флаг --probe): виден состав usage
/// page/usage у каждого — по нему строим фильтр списка.
pub fn probe() {
    let Ok(api) = hidapi::HidApi::new() else {
        eprintln!("probe: hid init failed");
        return;
    };
    eprintln!("probe: {} hid entries", api.device_list().count());
    for d in api.device_list() {
        eprintln!(
            "probe: vid={:04X} pid={:04X} page={:04X} usage={:02X} if={:?} prod={:?} mfr={:?}",
            d.vendor_id(),
            d.product_id(),
            d.usage_page(),
            d.usage(),
            d.interface_number(),
            d.product_string(),
            d.manufacturer_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    // Фильтр проверяем через scan нельзя (нужен hidapi), поэтому тут — сами правила
    // на тестовых коллекциях через замыкание с теми же ветками.
    #[test]
    fn filter_rules() {
        let rules = |page: u16, usage: u16, known: bool, show_all: bool| {
            if matches!((page, usage), (0x01, 0x01) | (0x01, 0x02)) {
                return false;
            }
            if page == 0x0D {
                return false;
            }
            show_all || known || page >= 0xFF00
        };
        assert!(!rules(0x01, 0x06, false, false), "системная клавиатура скрыта в коротком списке");
        assert!(rules(0x01, 0x06, false, true), "show_all открывает её");
        assert!(rules(0x01, 0x06, true, false), "знакомое устройство всегда видно");
        assert!(rules(0xFF31, 0x76, false, false), "vendor-defined виден");
        assert!(!rules(0x01, 0x02, false, true), "мышь скрыта всегда");
        assert!(!rules(0x0D, 0x05, false, true), "digitizer скрыт всегда");
    }
}
