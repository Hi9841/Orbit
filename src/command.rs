//! Parsing shared by the desktop executable and scripts. Parsing never changes Windows state.
use crate::geometry::{Action, parse_action};
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Command {
    Resident,
    Settings,
    Quit,
    Help,
    Version,
    Status,
    ListActions,
    Apply(Action),
    ExportSettings(PathBuf),
    ImportSettings(PathBuf),
    ResetSettings,
    CheckUpdates,
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Resident),
        [flag, path] if flag == "--export-settings" => Ok(Command::ExportSettings(path.into())),
        [flag, path] if flag == "--import-settings" => Ok(Command::ImportSettings(path.into())),
        [flag] => match flag.as_str() {
            "--resident" => Ok(Command::Resident),
            "--settings" => Ok(Command::Settings),
            "--quit" => Ok(Command::Quit),
            "--help" | "-h" => Ok(Command::Help),
            "--version" | "-v" | "-V" => Ok(Command::Version),
            "--status" => Ok(Command::Status),
            "--list-actions" => Ok(Command::ListActions),
            "--reset-settings" => Ok(Command::ResetSettings),
            "--check-updates" => Ok(Command::CheckUpdates),
            value if value.starts_with("orbit:") => parse_uri(value),
            value => parse_action(value).map(Command::Apply),
        },
        _ => Err("Use one action or flag. Import and export require exactly one file path.".into()),
    }
}

fn parse_uri(value: &str) -> Result<Command, String> {
    if value.len() > 2048 {
        return Err("Orbit URL is too long.".into());
    }
    let url = url::Url::parse(value).map_err(|_| "Invalid Orbit URL.")?;
    if url.scheme() != "orbit"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err("Use orbit://action/ACTION without credentials, ports, or fragments.".into());
    }
    match url.host_str() {
        Some("settings") if ["", "/"].contains(&url.path()) && url.query().is_none() => {
            Ok(Command::Settings)
        }
        Some("action") => {
            if url.query().is_some() {
                return Err("Orbit action URLs do not accept query parameters.".into());
            }
            let action = url.path().strip_prefix('/').unwrap_or_default();
            if action.is_empty()
                || !action
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':'))
            {
                return Err(
                    "Use orbit://action/ACTION with an action name such as left-half or custom:0."
                        .into(),
                );
            }
            parse_action(action).map(Command::Apply)
        }
        _ => Err("Supported URLs are orbit://settings and orbit://action/ACTION.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn parses_commands_without_desktop_side_effects() {
        assert_eq!(parse(&[]).unwrap(), Command::Resident);
        assert_eq!(
            parse(&args(&["--import-settings", "a file.json"])).unwrap(),
            Command::ImportSettings("a file.json".into())
        );
        assert_eq!(
            parse(&args(&["left-half"])).unwrap(),
            Command::Apply(Action::LeftHalf)
        );
        assert_eq!(
            parse(&args(&["orbit://action/right-half"])).unwrap(),
            Command::Apply(Action::RightHalf)
        );
        assert_eq!(
            parse(&args(&["orbit://settings"])).unwrap(),
            Command::Settings
        );
        assert!(parse(&args(&["left", "right"])).is_err());
    }

    #[test]
    fn rejects_uri_commands_outside_the_allowlist() {
        for value in [
            "orbit://quit",
            "orbit://action/left?command=quit",
            "orbit://user@action/left",
            "orbit://action/left#right",
            "orbit://action/%6ceft",
            "orbit://settings/anything",
            "orbit://action/left/right",
        ] {
            assert!(parse(&args(&[value])).is_err(), "{value}");
        }
    }
}
