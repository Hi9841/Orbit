use orbit::geometry::{Rect, parse_action};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    revision: String,
    frames: Vec<Frame>,
}
#[derive(Deserialize)]
struct Frame {
    action: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[test]
fn frames_match_pinned_loop_fractions_across_monitor_origins() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/loop-frames.json")).unwrap();
    assert_eq!(
        reference.revision,
        "df26d565e07c82e156b8f1c361bdcf428f32e3a4"
    );
    assert!(reference.frames.len() >= 25);
    for bounds in [
        Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        },
        Rect {
            left: -1601,
            top: -201,
            right: 0,
            bottom: 799,
        },
        Rect {
            left: 1920,
            top: 48,
            right: 4480,
            bottom: 1440,
        },
    ] {
        for reference in &reference.frames {
            let action = parse_action(&reference.action).unwrap();
            let actual = action.frame(bounds, bounds, 0);
            let x = f64::from(bounds.left) + reference.x * f64::from(bounds.width());
            let y = f64::from(bounds.top) + reference.y * f64::from(bounds.height());
            let expected = [
                x,
                y,
                x + reference.width * f64::from(bounds.width()),
                y + reference.height * f64::from(bounds.height()),
            ];
            for (actual, expected) in [actual.left, actual.top, actual.right, actual.bottom]
                .into_iter()
                .zip(expected)
            {
                assert!(
                    (f64::from(actual) - expected).abs() <= 1.01,
                    "{} on {bounds:?}: actual edge {actual}, source edge {expected}",
                    reference.action
                );
            }
        }
    }
}

#[test]
fn every_upstream_action_has_a_command_or_an_explicit_platform_classification() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/loop-actions.json")).unwrap();
    let actions = reference["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 94);
    let mut windows_exceptions = 0;
    let mut configured_or_internal = 0;
    for value in actions {
        let action = value.as_str().unwrap();
        if action == "nextSpace" || action == "previousSpace" || action.starts_with("moveToSpace") {
            windows_exceptions += 1;
            continue;
        }
        if ["noSelection", "custom", "cycle"].contains(&action) {
            // noSelection is Option::None; custom and cycle have configuration data.
            configured_or_internal += 1;
            continue;
        }
        let mut command = String::new();
        for character in action.chars() {
            if character.is_ascii_uppercase() {
                command.push('_');
            }
            command.push(character.to_ascii_lowercase());
        }
        if action == "macOSCenter" {
            command = "mac_os_center".into();
        }
        assert!(
            parse_action(&command).is_ok(),
            "unclassified source action {action}, command {command}"
        );
    }
    assert_eq!(windows_exceptions, 18);
    assert_eq!(configured_or_internal, 3);
}
