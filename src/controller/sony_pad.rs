use crate::controller::gamepad_match as pad;

pub const SONY: u16 = 0x054C;

pub const DUALSENSE: u16 = 0x0CE6;
pub const DUALSENSE_EDGE: u16 = 0x0DF2;
pub const DUALSHOCK4_V1: u16 = 0x05C4;
pub const DUALSHOCK4_V2: u16 = 0x09CC;
pub const DUALSHOCK4_ADAPTER: u16 = 0x0BA0;

pub const KNOWN_PRODUCTS: &[u16] = &[
    DUALSENSE,
    DUALSENSE_EDGE,
    DUALSHOCK4_V1,
    DUALSHOCK4_V2,
    DUALSHOCK4_ADAPTER,
];

pub const GAMEPAD_USAGE_PAGE: u16 = 0x01;
pub const GAMEPAD_USAGE: u16 = 0x05;

pub fn is_known(vendor: u16, product: u16) -> bool {
    vendor == SONY && KNOWN_PRODUCTS.contains(&product)
}

pub fn product_name(product: u16) -> &'static str {
    match product {
        DUALSENSE => "DualSense",
        DUALSENSE_EDGE => "DualSense Edge",
        DUALSHOCK4_V1 => "DualShock 4",
        DUALSHOCK4_V2 => "DualShock 4 v2",
        DUALSHOCK4_ADAPTER => "DualShock 4 wireless adapter",
        _ => "unknown",
    }
}

fn is_dualsense(product: u16) -> bool {
    product == DUALSENSE || product == DUALSENSE_EDGE
}

fn button_offset(product: u16, report: &[u8]) -> Option<usize> {
    let id = *report.first()?;
    let len = report.len();

    let offset = match (is_dualsense(product), id) {
        (true, 0x01) if len >= 64 => 8,
        (true, 0x01) if len >= 10 => 5,
        (true, 0x31) if len >= 78 => 9,
        (false, 0x01) if len >= 10 => 5,
        (false, 0x11..=0x19) if len >= 78 => 7,
        _ => return None,
    };

    match len >= offset + 3 {
        true => Some(offset),
        false => None,
    }
}

fn stick_offset(product: u16, report: &[u8]) -> Option<usize> {
    let buttons = button_offset(product, report)?;

    let sticks = match is_dualsense(product) && report.len() >= 64 {
        true => buttons.checked_sub(7)?,
        false => buttons.checked_sub(4)?,
    };

    match report.len() >= sticks + 4 {
        true => Some(sticks),
        false => None,
    }
}

pub fn parse_left_stick(product: u16, report: &[u8]) -> Option<(f32, f32)> {
    let at = stick_offset(product, report)?;

    Some((centred(report[at]), centred(report[at + 1])))
}

fn centred(byte: u8) -> f32 {
    (f32::from(byte) - 128.0) / 127.0
}

pub fn parse_report(product: u16, report: &[u8]) -> Option<u16> {
    let offset = button_offset(product, report)?;

    let faces = report[offset] >> 4;
    let shoulders = report[offset + 1];

    let mut mask = hat_to_mask(report[offset] & 0x0F);

    for (bit, button) in [(0, pad::X), (1, pad::A), (2, pad::B), (3, pad::Y)] {
        if faces & (1 << bit) != 0 {
            mask |= button;
        }
    }

    for (bit, button) in [
        (0, pad::LEFT_SHOULDER),
        (1, pad::RIGHT_SHOULDER),
        (2, pad::LEFT_TRIGGER),
        (3, pad::RIGHT_TRIGGER),
        (4, pad::BACK),
        (5, pad::START),
        (6, pad::LEFT_THUMB),
        (7, pad::RIGHT_THUMB),
    ] {
        if shoulders & (1 << bit) != 0 {
            mask |= button;
        }
    }

    Some(mask)
}

pub fn hat_to_mask(hat: u8) -> u16 {
    match hat {
        0 => pad::DPAD_UP,
        1 => pad::DPAD_UP | pad::DPAD_RIGHT,
        2 => pad::DPAD_RIGHT,
        3 => pad::DPAD_DOWN | pad::DPAD_RIGHT,
        4 => pad::DPAD_DOWN,
        5 => pad::DPAD_DOWN | pad::DPAD_LEFT,
        6 => pad::DPAD_LEFT,
        7 => pad::DPAD_UP | pad::DPAD_LEFT,
        _ => 0,
    }
}

pub const BUTTON_PAGE: u16 = 0x09;
pub const HAT_USAGE: u16 = 0x39;

pub fn bit_for_button(number: u16) -> u16 {
    match number {
        1 => pad::X,
        2 => pad::A,
        3 => pad::B,
        4 => pad::Y,
        5 => pad::LEFT_SHOULDER,
        6 => pad::RIGHT_SHOULDER,
        7 => pad::LEFT_TRIGGER,
        8 => pad::RIGHT_TRIGGER,
        9 => pad::BACK,
        10 => pad::START,
        11 => pad::LEFT_THUMB,
        12 => pad::RIGHT_THUMB,
        _ => 0,
    }
}

pub const WALKTHROUGH: &[&str] = &[
    "Square", "Cross", "Circle", "Triangle", "L1", "R1", "L2", "R2", "Create", "Options", "L3",
    "R3", "Up", "Down", "Left", "Right",
];

pub fn expected_bit(label: &str) -> u16 {
    pad::parse_chord(label).map_or(0, |chord| chord.mask)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Usb,
    BluetoothBasic,
    BluetoothFull,
}

#[cfg(test)]
fn mask_to_hat(mask: u16) -> Option<u8> {
    let directions = mask & (pad::DPAD_UP | pad::DPAD_DOWN | pad::DPAD_LEFT | pad::DPAD_RIGHT);

    for hat in 0..8u8 {
        if hat_to_mask(hat) == directions {
            return Some(hat);
        }
    }

    match directions {
        0 => Some(8),
        _ => None,
    }
}

#[cfg(test)]
fn render_report(product: u16, transport: Transport, mask: u16) -> Option<Vec<u8>> {
    let hat = mask_to_hat(mask)?;

    let (id, len, offset) = match (is_dualsense(product), transport) {
        (true, Transport::Usb) => (0x01, 64, 8),
        (true, Transport::BluetoothBasic) => (0x01, 10, 5),
        (true, Transport::BluetoothFull) => (0x31, 78, 9),
        (false, Transport::Usb) => (0x01, 64, 5),
        (false, Transport::BluetoothBasic) => (0x01, 10, 5),
        (false, Transport::BluetoothFull) => (0x11, 78, 7),
    };

    let mut report = vec![0u8; len];
    let mut faces = 0u8;
    let mut shoulders = 0u8;

    for (bit, button) in [(0, pad::X), (1, pad::A), (2, pad::B), (3, pad::Y)] {
        if mask & button == button {
            faces |= 1 << bit;
        }
    }

    for (bit, button) in [
        (0, pad::LEFT_SHOULDER),
        (1, pad::RIGHT_SHOULDER),
        (2, pad::LEFT_TRIGGER),
        (3, pad::RIGHT_TRIGGER),
        (4, pad::BACK),
        (5, pad::START),
        (6, pad::LEFT_THUMB),
        (7, pad::RIGHT_THUMB),
    ] {
        if mask & button == button {
            shoulders |= 1 << bit;
        }
    }

    report[0] = id;
    report[offset] = (faces << 4) | hat;
    report[offset + 1] = shoulders;

    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dualsense_usb(hat: u8, faces: u8, shoulders: u8) -> Vec<u8> {
        let mut report = vec![0u8; 64];

        report[0] = 0x01;
        report[8] = (faces << 4) | hat;
        report[9] = shoulders;

        report
    }

    fn dualsense_bluetooth(hat: u8, faces: u8, shoulders: u8) -> Vec<u8> {
        let mut report = vec![0u8; 10];

        report[0] = 0x01;
        report[5] = (faces << 4) | hat;
        report[6] = shoulders;

        report
    }

    fn dualsense_bluetooth_full(hat: u8, faces: u8, shoulders: u8) -> Vec<u8> {
        let mut report = vec![0u8; 78];

        report[0] = 0x31;
        report[9] = (faces << 4) | hat;
        report[10] = shoulders;

        report
    }

    fn dualshock_usb(hat: u8, faces: u8, shoulders: u8) -> Vec<u8> {
        let mut report = vec![0u8; 64];

        report[0] = 0x01;
        report[5] = (faces << 4) | hat;
        report[6] = shoulders;

        report
    }

    fn dualshock_bluetooth_full(hat: u8, faces: u8, shoulders: u8) -> Vec<u8> {
        let mut report = vec![0u8; 78];

        report[0] = 0x11;
        report[7] = (faces << 4) | hat;
        report[8] = shoulders;

        report
    }

    const NEUTRAL: u8 = 0x8;

    const EVERY_PAD: [u16; 5] = [
        DUALSENSE,
        DUALSENSE_EDGE,
        DUALSHOCK4_V1,
        DUALSHOCK4_V2,
        DUALSHOCK4_ADAPTER,
    ];

    const EVERY_TRANSPORT: [Transport; 3] = [
        Transport::Usb,
        Transport::BluetoothBasic,
        Transport::BluetoothFull,
    ];

    #[test]
    fn every_press_a_pad_can_physically_make_decodes_back_to_itself() {
        for product in EVERY_PAD {
            for transport in EVERY_TRANSPORT {
                for mask in 0..=u16::MAX {
                    let Some(report) = render_report(product, transport, mask) else {
                        continue;
                    };

                    assert_eq!(
                        parse_report(product, &report),
                        Some(mask),
                        "{product:#06x} {transport:?} {mask:#06x}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_dpad_pointing_two_opposite_ways_at_once_cannot_be_rendered() {
        let impossible = pad::DPAD_UP | pad::DPAD_DOWN;

        assert_eq!(mask_to_hat(impossible), None);
        assert_eq!(render_report(DUALSENSE, Transport::Usb, impossible), None);
    }

    #[test]
    fn every_hat_position_survives_the_round_trip() {
        for hat in 0..=15u8 {
            let mask = hat_to_mask(hat);

            assert_eq!(mask_to_hat(mask), Some(if hat > 7 { 8 } else { hat }));
        }
    }

    #[test]
    fn no_report_of_any_shape_makes_the_parser_panic() {
        for product in EVERY_PAD {
            for id in 0..=u8::MAX {
                for len in 0..80usize {
                    let mut report = vec![0xFFu8; len];

                    if len > 0 {
                        report[0] = id;
                    }

                    let _ = parse_report(product, &report);
                }
            }
        }
    }

    #[test]
    fn a_report_shorter_than_its_buttons_is_always_refused() {
        for product in EVERY_PAD {
            for transport in EVERY_TRANSPORT {
                let full = render_report(product, transport, 0).expect("a report");

                for len in 0..full.len() {
                    let short = &full[..len];

                    if parse_report(product, short).is_some() {
                        assert!(
                            len >= 8,
                            "{product:#06x} {transport:?} read {len} bytes as a whole report"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_button_the_descriptor_numbers_is_a_bit_we_decode() {
        for number in 1..=12u16 {
            assert_ne!(bit_for_button(number), 0, "button {number}");
        }
    }

    #[test]
    fn the_descriptor_numbering_matches_the_offsets_we_hardcode() {
        for number in 1..=12u16 {
            let bit = bit_for_button(number);
            let report = render_report(DUALSENSE, Transport::Usb, bit).expect("a report");

            assert_eq!(
                parse_report(DUALSENSE, &report),
                Some(bit),
                "sony button {number} disagrees with our offset table"
            );
        }
    }

    #[test]
    fn a_button_number_beyond_the_ones_we_map_has_no_bit() {
        assert_eq!(bit_for_button(0), 0);
        assert_eq!(bit_for_button(13), 0, "PS button is not offered as a chord");
        assert_eq!(bit_for_button(14), 0, "touchpad is not offered as a chord");
        assert_eq!(bit_for_button(99), 0);
    }

    fn with_left_stick(mut report: Vec<u8>, x: u8, y: u8, at: usize) -> Vec<u8> {
        report[at] = x;
        report[at + 1] = y;

        report
    }

    #[test]
    fn a_stick_at_rest_reads_as_the_middle() {
        let report = with_left_stick(dualsense_usb(NEUTRAL, 0, 0), 0x80, 0x7f, 1);
        let (x, y) = parse_left_stick(DUALSENSE, &report).expect("sticks");

        assert!(x.abs() < 0.6 && y.abs() < 0.6, "{x} {y}");
    }

    #[test]
    fn the_left_stick_is_read_at_its_own_offset_on_every_transport() {
        let cases = [
            (DUALSENSE, dualsense_usb(NEUTRAL, 0, 0), 1),
            (DUALSENSE, dualsense_bluetooth(NEUTRAL, 0, 0), 1),
            (DUALSENSE, dualsense_bluetooth_full(NEUTRAL, 0, 0), 2),
            (DUALSHOCK4_V2, dualshock_usb(NEUTRAL, 0, 0), 1),
            (DUALSHOCK4_V2, dualshock_bluetooth_full(NEUTRAL, 0, 0), 3),
        ];

        for (product, report, at) in cases {
            let pushed = with_left_stick(report, 0xFF, 0x00, at);
            let (x, y) = parse_left_stick(product, &pushed).expect("sticks");

            assert!(x > 0.9, "{product:#06x} x {x}");
            assert!(y < -0.9, "{product:#06x} y {y}");
        }
    }

    #[test]
    fn a_report_this_build_cannot_read_has_no_stick_either() {
        let mut report = dualsense_usb(NEUTRAL, 0, 0);

        report[0] = 0x02;

        assert_eq!(parse_left_stick(DUALSENSE, &report), None);
    }

    #[test]
    fn a_dualsense_over_usb_reads_its_face_buttons() {
        let report = dualsense_usb(NEUTRAL, 0b0001, 0);

        assert_eq!(parse_report(DUALSENSE, &report), Some(pad::X));
    }

    #[test]
    fn square_is_the_xbox_x_and_cross_is_the_xbox_a() {
        let square = dualsense_usb(NEUTRAL, 0b0001, 0);
        let cross = dualsense_usb(NEUTRAL, 0b0010, 0);
        let circle = dualsense_usb(NEUTRAL, 0b0100, 0);
        let triangle = dualsense_usb(NEUTRAL, 0b1000, 0);

        assert_eq!(parse_report(DUALSENSE, &square), Some(pad::X));
        assert_eq!(parse_report(DUALSENSE, &cross), Some(pad::A));
        assert_eq!(parse_report(DUALSENSE, &circle), Some(pad::B));
        assert_eq!(parse_report(DUALSENSE, &triangle), Some(pad::Y));
    }

    #[test]
    fn every_shoulder_and_menu_button_lands_on_its_own_bit() {
        let cases = [
            (0b0000_0001, pad::LEFT_SHOULDER),
            (0b0000_0010, pad::RIGHT_SHOULDER),
            (0b0000_0100, pad::LEFT_TRIGGER),
            (0b0000_1000, pad::RIGHT_TRIGGER),
            (0b0001_0000, pad::BACK),
            (0b0010_0000, pad::START),
            (0b0100_0000, pad::LEFT_THUMB),
            (0b1000_0000, pad::RIGHT_THUMB),
        ];

        for (bits, wanted) in cases {
            let report = dualsense_usb(NEUTRAL, 0, bits);

            assert_eq!(parse_report(DUALSENSE, &report), Some(wanted), "{bits:08b}");
        }
    }

    #[test]
    fn the_chord_we_document_decodes_from_a_real_shaped_report() {
        let report = dualsense_usb(NEUTRAL, 0b1000, 0b0000_0011);
        let wanted = pad::parse_chord("L1+R1+Triangle").expect("a chord").mask;

        assert_eq!(parse_report(DUALSENSE, &report), Some(wanted));
    }

    #[test]
    fn the_hat_reads_as_the_direction_it_points() {
        let cases = [
            (0, pad::DPAD_UP),
            (1, pad::DPAD_UP | pad::DPAD_RIGHT),
            (2, pad::DPAD_RIGHT),
            (3, pad::DPAD_DOWN | pad::DPAD_RIGHT),
            (4, pad::DPAD_DOWN),
            (5, pad::DPAD_DOWN | pad::DPAD_LEFT),
            (6, pad::DPAD_LEFT),
            (7, pad::DPAD_UP | pad::DPAD_LEFT),
        ];

        for (hat, wanted) in cases {
            let report = dualsense_usb(hat, 0, 0);

            assert_eq!(parse_report(DUALSENSE, &report), Some(wanted), "hat {hat}");
        }
    }

    #[test]
    fn a_hat_at_rest_presses_no_direction() {
        for hat in [8u8, 9, 15] {
            let report = dualsense_usb(hat, 0, 0);

            assert_eq!(parse_report(DUALSENSE, &report), Some(0), "hat {hat}");
        }
    }

    #[test]
    fn a_pad_at_rest_decodes_to_nothing_held_rather_than_to_no_report() {
        let report = dualsense_usb(NEUTRAL, 0, 0);

        assert_eq!(parse_report(DUALSENSE, &report), Some(0));
    }

    #[test]
    fn the_same_press_decodes_the_same_over_every_transport() {
        let wanted = pad::parse_chord("L1+R1+Triangle").expect("a chord").mask;

        let usb = dualsense_usb(NEUTRAL, 0b1000, 0b11);
        let basic = dualsense_bluetooth(NEUTRAL, 0b1000, 0b11);
        let full = dualsense_bluetooth_full(NEUTRAL, 0b1000, 0b11);

        assert_eq!(parse_report(DUALSENSE, &usb), Some(wanted));
        assert_eq!(parse_report(DUALSENSE, &basic), Some(wanted));
        assert_eq!(parse_report(DUALSENSE, &full), Some(wanted));
    }

    #[test]
    fn a_dualshock_four_reads_at_its_own_offsets() {
        let wanted = pad::parse_chord("L1+R1+Triangle").expect("a chord").mask;

        let usb = dualshock_usb(NEUTRAL, 0b1000, 0b11);
        let full = dualshock_bluetooth_full(NEUTRAL, 0b1000, 0b11);

        assert_eq!(parse_report(DUALSHOCK4_V2, &usb), Some(wanted));
        assert_eq!(parse_report(DUALSHOCK4_V2, &full), Some(wanted));
    }

    #[test]
    fn every_dualshock_bluetooth_report_id_is_read() {
        for id in 0x11u8..=0x19 {
            let mut report = dualshock_bluetooth_full(NEUTRAL, 0b1000, 0);

            report[0] = id;

            assert_eq!(
                parse_report(DUALSHOCK4_V2, &report),
                Some(pad::Y),
                "{id:#x}"
            );
        }
    }

    #[test]
    fn a_dualsense_report_read_at_the_dualshock_offsets_would_be_wrong() {
        let report = dualsense_usb(NEUTRAL, 0b1000, 0b11);

        assert_ne!(
            parse_report(DUALSENSE, &report),
            parse_report(DUALSHOCK4_V2, &report),
            "the two families must not share an offset by accident"
        );
    }

    #[test]
    fn a_report_id_this_build_does_not_know_is_refused_rather_than_guessed() {
        let mut report = dualsense_usb(NEUTRAL, 0b1000, 0);

        report[0] = 0x02;

        assert_eq!(parse_report(DUALSENSE, &report), None);
    }

    #[test]
    fn a_truncated_report_is_refused_rather_than_read_past_its_end() {
        for len in 0..10 {
            let report = vec![0x01u8; len];

            assert_eq!(parse_report(DUALSENSE, &report), None, "len {len}");
            assert_eq!(parse_report(DUALSHOCK4_V2, &report), None, "len {len}");
        }
    }

    #[test]
    fn a_bluetooth_full_report_that_is_short_is_refused() {
        let report = vec![0x31u8; 40];

        assert_eq!(parse_report(DUALSENSE, &report), None);
    }

    #[test]
    fn only_sony_pads_this_build_knows_are_claimed() {
        assert!(is_known(SONY, DUALSENSE));
        assert!(is_known(SONY, DUALSHOCK4_V1));
        assert!(!is_known(SONY, 0x1234));
        assert!(!is_known(0x045E, DUALSENSE));
    }

    #[test]
    fn every_known_product_has_a_name_for_the_log() {
        for product in KNOWN_PRODUCTS {
            assert_ne!(product_name(*product), "unknown", "{product:#06x}");
        }
    }

    #[test]
    fn every_walkthrough_label_names_a_bit_we_can_decode() {
        for label in WALKTHROUGH {
            assert_ne!(expected_bit(label), 0, "{label}");
        }
    }

    #[test]
    fn the_walkthrough_covers_every_button_a_chord_may_use() {
        let mut covered = 0u16;

        for label in WALKTHROUGH {
            covered |= expected_bit(label);
        }

        for (name, bit) in pad::PLAYSTATION_BUTTONS {
            assert_eq!(covered & bit, *bit, "{name} is never walked through");
        }
    }

    #[test]
    fn a_label_that_is_not_a_button_has_no_bit() {
        assert_eq!(expected_bit("Nonsense"), 0);
    }
}
