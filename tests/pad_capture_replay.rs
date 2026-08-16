use std::fs;
use std::path::{Path, PathBuf};

use poe_wayfinder_core::controller::gamepad_match::{describe_for, PadFamily};
use poe_wayfinder_core::controller::sony_pad::{expected_bit, parse_report};

struct Capture {
    name: String,
    product: u16,
    presses: Vec<(String, Vec<u8>)>,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn read_captures() -> Vec<Capture> {
    let Ok(entries) = fs::read_dir(fixtures()) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("hex") {
            continue;
        }

        let body = fs::read_to_string(&path).expect("reading a capture");
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("capture")
            .to_string();

        out.push(parse_capture(&name, &body));
    }

    out
}

fn parse_capture(name: &str, body: &str) -> Capture {
    let mut product = None;
    let mut presses = Vec::new();

    for line in body.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("# product ") {
            let text = rest.trim().trim_start_matches("0x");

            product = u16::from_str_radix(text, 16).ok();

            continue;
        }

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let label = parts.next().expect("a label").to_string();
        let bytes = parts
            .map(|byte| u8::from_str_radix(byte, 16).expect("a hex byte"))
            .collect();

        presses.push((label, bytes));
    }

    Capture {
        name: name.to_string(),
        product: product.unwrap_or_else(|| panic!("{name} has no product header")),
        presses,
    }
}

#[test]
fn every_captured_press_decodes_to_the_button_that_was_pressed() {
    for capture in read_captures() {
        assert!(
            !capture.presses.is_empty(),
            "{} holds no presses, so it proves nothing",
            capture.name
        );

        for (label, bytes) in &capture.presses {
            let read = parse_report(capture.product, bytes);
            let wanted = expected_bit(label);

            assert_eq!(
                read,
                Some(wanted),
                "{}: {label} decoded as {}",
                capture.name,
                describe_for(PadFamily::PlayStation, read.unwrap_or(0))
            );
        }
    }
}

#[test]
fn a_capture_is_named_by_the_pad_that_produced_it() {
    for capture in read_captures() {
        assert_ne!(
            poe_wayfinder_core::controller::sony_pad::product_name(capture.product),
            "unknown",
            "{} names a product this build does not know",
            capture.name
        );
    }
}

#[test]
fn the_fixture_directory_exists_so_a_capture_has_somewhere_to_land() {
    assert!(
        fixtures().is_dir(),
        "hardware captures live in {}. Run --pad-walkthrough to make one.",
        fixtures().display()
    );
}
