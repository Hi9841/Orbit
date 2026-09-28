use crate::{login, platform};
use orbit::settings::Settings;
use std::io::{Read, Write};
use std::path::Path;

const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

pub fn read_import(path: &Path) -> Result<Settings, String> {
    let source = std::fs::File::open(path)
        .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    source
        .take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read settings: {error}"))?;
    if bytes.len() as u64 > MAX_SETTINGS_BYTES {
        return Err("Settings file exceeds 1 MiB.".into());
    }
    let next: Settings = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid settings JSON: {error}"))?;
    next.validate()?;
    Ok(next)
}

pub fn export(settings: &Settings, path: &Path) -> Result<(), String> {
    settings.validate()?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot create export: {error}"))?;
    serde_json::to_writer_pretty(&mut staged, settings).map_err(|error| error.to_string())?;
    staged.write_all(b"\n").map_err(|error| error.to_string())?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    staged.persist_noclobber(path).map_err(|error| {
        format!(
            "Cannot create {} without replacing an existing file: {error}",
            path.display()
        )
    })?;
    Ok(())
}

/// Keep persisted preferences, logon registration, and live shortcuts in agreement.
pub fn save(next: &Settings) -> Result<(), String> {
    next.validate()?;
    let previous = Settings::load()?;
    next.save()?;
    let result =
        login::set_enabled(next.launch_at_login).and_then(|()| platform::reload_running_settings());
    if let Err(error) = result {
        let mut failures = Vec::new();
        if let Err(rollback) = previous.save() {
            failures.push(rollback);
        }
        if let Err(rollback) = login::set_enabled(previous.launch_at_login) {
            failures.push(rollback);
        }
        if let Err(rollback) = platform::reload_running_settings() {
            failures.push(rollback);
        }
        return if failures.is_empty() {
            Err(error)
        } else {
            Err(format!(
                "{error}. Restoring previous settings also failed: {}",
                failures.join("; ")
            ))
        };
    }
    Ok(())
}

pub fn reset() -> Result<(), String> {
    if Settings::load().is_ok() {
        return save(&Settings::default());
    }
    let path = Settings::path()?;
    // Preserve an unreadable configuration rather than discard the only copy.
    let parent = path.parent().ok_or("Settings path has no parent.")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let backup = parent.join(format!("settings-invalid-{stamp}.json"));
    std::fs::rename(&path, &backup)
        .map_err(|error| format!("Cannot preserve the invalid settings file: {error}"))?;
    if let Err(error) = save(&Settings::default()) {
        // save() may have restored defaults while rolling back. Remove only that known settings file.
        if path.exists() {
            std::fs::remove_file(&path).map_err(|restore| {
                format!("{error}; cannot restore original configuration: {restore}")
            })?;
        }
        std::fs::rename(&backup, &path).map_err(|restore| {
            format!(
                "{error}; original settings remain at {}: {restore}",
                backup.display()
            )
        })?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_a_roundtrip_without_replacing_existing_data() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        let settings = Settings {
            padding: 17,
            ..Default::default()
        };
        export(&settings, &path).unwrap();
        assert_eq!(read_import(&path).unwrap().padding, 17);
        assert!(export(&Settings::default(), &path).is_err());
        assert_eq!(read_import(&path).unwrap().padding, 17);
    }

    #[test]
    fn rejects_invalid_and_oversized_imports_before_applying_them() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        for bytes in [
            br#"{"version":99}"#.as_slice(),
            br#"{"padding":-4}"#.as_slice(),
            b"{invalid".as_slice(),
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(read_import(&path).is_err());
        }
        std::fs::write(&path, vec![b' '; MAX_SETTINGS_BYTES as usize + 1]).unwrap();
        assert!(read_import(&path).unwrap_err().contains("1 MiB"));
    }
}
