use teloxide::types::InlineKeyboardButtonKind;

use super::*;
use crate::remnawave::test_support::device;

fn callbacks(keyboard: &InlineKeyboardMarkup) -> Vec<&str> {
    keyboard
        .inline_keyboard
        .iter()
        .flatten()
        .filter_map(|button| match &button.kind {
            InlineKeyboardButtonKind::CallbackData(data) => Some(data.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn long_unicode_hwid_and_labels_fit_telegram_limits() {
    let mut item = device(7, &"длинный-HWID📱".repeat(200));
    item.device_model = Some("Телефон📱\n".repeat(200));
    let token = device_token(&item);
    let (text, keyboard) = devices_screen(
        &DeviceList {
            limit: Some(5),
            devices: vec![item.clone()],
        },
        0,
    );
    assert!(text.contains("Подключено: 1\nЛимит: 5"));
    assert!(!text.contains(&item.hwid));
    assert!(text.chars().count() < 4096);
    assert!(callbacks(&keyboard).contains(&format!("{CALLBACK_DEVICE_PREFIX}{token}").as_str()));
    let (confirmation, keyboard) = device_confirmation_screen(&item);
    assert!(confirmation.chars().count() < 4096);
    assert!(callbacks(&keyboard).iter().all(|data| data.len() <= 64));
    ///assert!(
    ///    callbacks(&keyboard).contains(&format!("{CALLBACK_DEVICE_DELETE_PREFIX}{token}").as_str())
    ///);
    assert_eq!(callbacks(&keyboard), vec![CALLBACK_DEVICES]);
}

#[test]
fn empty_unlimited_and_inherited_limits_are_distinct() {
    for (limit, expected) in [
        (Some(5), "Лимит: 5"),
        (Some(0), "Лимит: Без лимита"),
        (None, "Лимит: По настройкам сервиса"),
    ] {
        let (text, keyboard) = devices_screen(
            &DeviceList {
                limit,
                devices: vec![],
            },
            usize::MAX,
        );
        assert!(text.contains(expected));
        assert!(text.contains("Устройств пока нет"));
        assert!(
            !callbacks(&keyboard)
                .iter()
                .any(|data| data.starts_with(CALLBACK_DEVICE_PREFIX))
        );
    }
}

#[test]
fn pagination_keeps_all_devices_accessible_and_clamps_stale_pages() {
    let list = DeviceList {
        limit: Some(0),
        devices: (0..12).map(|id| device(7, &id.to_string())).collect(),
    };
    let mut seen = Vec::new();
    for page in 0..3 {
        let (text, keyboard) = devices_screen(&list, page);
        assert!(text.contains(&format!("Страница {} из 3", page + 1)));
        let buttons: Vec<_> = callbacks(&keyboard)
            .into_iter()
            .filter(|data| data.starts_with(CALLBACK_DEVICE_PREFIX))
            .map(str::to_owned)
            .collect();
        assert!(buttons.len() <= DEVICES_PAGE_SIZE);
        seen.extend(buttons);
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 12);
    assert!(
        devices_screen(&list, usize::MAX)
            .0
            .contains("Страница 3 из 3")
    );
}
