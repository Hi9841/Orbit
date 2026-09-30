use crate::geometry::Action;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkey {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: u16,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            control: true,
            alt: true,
            shift: false,
            win: false,
            key: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub hotkey: Hotkey,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomFrame {
    pub name: String,
    /// Fractions of the monitor work area, in the range 0.0 to 1.0.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn default_radial_actions() -> [Action; 8] {
    Action::RADIAL
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgePadding {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSide {
    #[default]
    Either,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStart {
    ScreenCenter,
    RadialMenu,
    #[default]
    ActionCenter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub padding: i32,
    pub radial_menu_visible: bool,
    pub preview_visible: bool,
    pub launch_at_login: bool,
    pub snap_on_drag: bool,
    pub excluded_processes: Vec<String>,
    pub trigger: Hotkey,
    pub shortcuts: Vec<Shortcut>,
    pub radial_actions: [Action; 8],
    pub custom_frames: Vec<CustomFrame>,
    pub radial_size: u32,
    pub radial_thickness: u32,
    pub radial_corner_radius: u32,
    pub accent_color: u32,
    pub preview_opacity: u8,
    pub preview_padding: i32,
    pub preview_corner_radius: u32,
    pub cycle_timeout_ms: u32,
    pub trigger_delay_ms: u32,
    pub reverse_scroll: bool,
    pub size_increment: i32,
    pub use_screen_with_cursor: bool,
    pub resize_window_under_cursor: bool,
    pub focus_window_on_resize: bool,
    pub move_cursor_with_window: bool,
    pub ignore_fullscreen: bool,
    pub disable_cursor_interaction: bool,
    pub lock_radial_menu_to_center: bool,
    pub snap_threshold: i32,
    pub stash_visible_padding: i32,
    pub updates_enabled: bool,
    pub cycle_backwards_on_shift: bool,
    pub use_system_accent: bool,
    pub use_gradient: bool,
    pub gradient_color: u32,
    pub preview_border_thickness: u32,
    pub preview_use_window_corner_radius: bool,
    pub edge_padding: Option<EdgePadding>,
    pub padding_minimum_screen_inches: f64,
    pub animation_duration_ms: u32,
    pub animate_window_resizes: bool,
    pub animate_stashed_windows: bool,
    pub ignore_low_power_mode: bool,
    pub preview_start: PreviewStart,
    pub restore_window_frame_on_drag: bool,
    pub shift_focus_when_stashed: bool,
    pub cycle_restart: bool,
    pub trigger_side: TriggerSide,
    pub double_tap_to_trigger: bool,
    pub middle_click_triggers: bool,
    pub middle_click_uses_delay: bool,
    pub trigger_timeout_ms: u32,
    pub hide_on_no_selection: bool,
    pub hide_tray_icon: bool,
    pub include_development_versions: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            padding: 0,
            radial_menu_visible: true,
            preview_visible: true,
            launch_at_login: false,
            snap_on_drag: false,
            excluded_processes: Vec::new(),
            trigger: Hotkey::default(),
            shortcuts: vec![Shortcut {
                hotkey: Hotkey {
                    key: u16::from(b'Z'),
                    ..Hotkey::default()
                },
                actions: vec![Action::Undo],
            }],
            radial_actions: default_radial_actions(),
            custom_frames: Vec::new(),
            radial_size: 100,
            radial_thickness: 14,
            radial_corner_radius: 20,
            accent_color: 0xF4F4F0,
            preview_opacity: 170,
            preview_padding: 10,
            preview_corner_radius: 8,
            cycle_timeout_ms: 1000,
            trigger_delay_ms: 0,
            reverse_scroll: false,
            size_increment: 20,
            use_screen_with_cursor: true,
            resize_window_under_cursor: false,
            focus_window_on_resize: true,
            move_cursor_with_window: false,
            ignore_fullscreen: true,
            disable_cursor_interaction: false,
            lock_radial_menu_to_center: false,
            snap_threshold: 12,
            stash_visible_padding: 20,
            updates_enabled: true,
            cycle_backwards_on_shift: true,
            use_system_accent: false,
            use_gradient: false,
            gradient_color: 0x3b82f6,
            preview_border_thickness: 2,
            preview_use_window_corner_radius: true,
            edge_padding: None,
            padding_minimum_screen_inches: 0.0,
            animation_duration_ms: 180,
            animate_window_resizes: false,
            animate_stashed_windows: false,
            ignore_low_power_mode: false,
            preview_start: PreviewStart::ActionCenter,
            restore_window_frame_on_drag: false,
            shift_focus_when_stashed: true,
            cycle_restart: false,
            trigger_side: TriggerSide::Either,
            double_tap_to_trigger: false,
            middle_click_triggers: false,
            middle_click_uses_delay: false,
            trigger_timeout_ms: 0,
            hide_on_no_selection: false,
            hide_tray_icon: false,
            include_development_versions: false,
        }
    }
}

impl Settings {
    pub fn path() -> Result<PathBuf, String> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
        Ok(PathBuf::from(base).join("Orbit").join("settings.json"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        let mut settings: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid settings at {}: {error}", path.display()))?;
        if settings.version != 1 {
            return Err(format!("unsupported settings version {}", settings.version));
        }
        migrate_loaded(&mut settings);
        settings.padding = settings.padding.clamp(0, 100);
        settings
            .excluded_processes
            .retain(|name| !name.trim().is_empty());
        settings.validate()?;
        Ok(settings)
    }

    /// Validate imported settings without silently rewriting invalid values.
    pub fn validate(&self) -> Result<(), String> {
        if self.accent_color > 0xffffff || self.gradient_color > 0xffffff {
            return Err("colors must be valid RGB values".into());
        }
        if self.preview_border_thickness > 32 {
            return Err("preview_border_thickness must be between 0 and 32".into());
        }
        if self.animation_duration_ms > 2000 {
            return Err("animation_duration_ms must be between 0 and 2000".into());
        }
        if self.trigger_timeout_ms > 600000 {
            return Err("trigger_timeout_ms must be between 0 and 600000".into());
        }
        if !self.padding_minimum_screen_inches.is_finite()
            || !(0.0..=200.0).contains(&self.padding_minimum_screen_inches)
        {
            return Err("padding_minimum_screen_inches must be between 0 and 200".into());
        }
        if self.edge_padding.is_some_and(|p| {
            [p.top, p.right, p.bottom, p.left]
                .iter()
                .any(|x| !(0..=200).contains(x))
        }) {
            return Err("edge padding must be between 0 and 200 pixels per edge".into());
        }
        if self.version != 1 {
            return Err(format!("unsupported settings version {}", self.version));
        }
        if !(0..=100).contains(&self.padding) {
            return Err("padding must be between 0 and 100".into());
        }
        if self.radial_size == 0 || self.radial_size > 512 {
            return Err("radial_size must be between 1 and 512".into());
        }
        if self.radial_thickness == 0 || self.radial_thickness > self.radial_size / 2 {
            return Err("radial_thickness must be between 1 and half the radial size".into());
        }
        if self.radial_corner_radius > self.radial_size / 2 {
            return Err("radial_corner_radius cannot exceed half the radial size".into());
        }
        if self.preview_padding < 0 || self.preview_padding > 200 {
            return Err("preview_padding must be between 0 and 200".into());
        }
        if self.preview_corner_radius > 200 {
            return Err("preview_corner_radius must be between 0 and 200".into());
        }
        if self.size_increment <= 0 || self.size_increment > 2000 {
            return Err("size_increment must be between 1 and 2000".into());
        }
        if !(50..=60000).contains(&self.cycle_timeout_ms) {
            return Err("cycle_timeout_ms must be between 50 and 60000".into());
        }
        if self.trigger_delay_ms > 1000 {
            return Err("trigger_delay_ms must be between 0 and 1000".into());
        }
        if !(0..=500).contains(&self.snap_threshold)
            || !(0..=500).contains(&self.stash_visible_padding)
        {
            return Err("snap and stash padding must be between 0 and 500".into());
        }
        validate_hotkey(self.trigger, "trigger", true)?;
        if self.shortcuts.len() > 2048 {
            return Err("at most 2048 global shortcuts are supported".into());
        }
        if self.excluded_processes.len() > 256
            || self
                .excluded_processes
                .iter()
                .any(|name| name.len() > 260 || name.contains('\0'))
        {
            return Err(
                "excluded_processes must contain at most 256 names of 260 bytes or fewer".into(),
            );
        }
        let mut hotkeys = HashSet::new();
        hotkeys.insert(self.trigger);
        for (index, shortcut) in self.shortcuts.iter().enumerate() {
            validate_hotkey(
                shortcut.hotkey,
                &format!("shortcuts[{index}].hotkey"),
                false,
            )?;
            if !hotkeys.insert(shortcut.hotkey) {
                return Err("shortcut hotkeys must be unique".into());
            }
            if self.cycle_backwards_on_shift && shortcut.actions.len() > 1 {
                let mut reverse = shortcut.hotkey;
                reverse.shift = !reverse.shift;
                if !hotkeys.insert(reverse) {
                    return Err("shift-reversed shortcut hotkeys must be unique and cannot overlap another binding".into());
                }
            }
            if shortcut.actions.is_empty() {
                return Err(format!(
                    "shortcuts[{index}] must contain at least one action"
                ));
            }
            if shortcut.actions.len() > 64 {
                return Err(format!(
                    "shortcuts[{index}] may contain at most 64 cycle actions"
                ));
            }
        }
        if self.custom_frames.len() > u16::MAX as usize {
            return Err("too many custom frames".into());
        }
        let mut names = HashSet::new();
        for (index, frame) in self.custom_frames.iter().enumerate() {
            if frame.name.trim().is_empty() || !names.insert(frame.name.to_lowercase()) {
                return Err(format!(
                    "custom_frames[{index}] needs a unique non-empty name"
                ));
            }
            if ![frame.x, frame.y, frame.width, frame.height]
                .iter()
                .all(|value| value.is_finite())
                || frame.x < 0.0
                || frame.y < 0.0
                || frame.width <= 0.0
                || frame.height <= 0.0
                || frame.x + frame.width > 1.0
                || frame.y + frame.height > 1.0
            {
                return Err(format!(
                    "custom_frames[{index}] must fit within the monitor work area using fractions from 0 to 1"
                ));
            }
        }
        for action in self.radial_actions {
            validate_action(action, self.custom_frames.len())?;
        }
        for shortcut in &self.shortcuts {
            for &action in &shortcut.actions {
                validate_action(action, self.custom_frames.len())?;
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        self.validate()?;
        let path = Self::path()?;
        let parent = path.parent().ok_or("settings path has no parent")?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create settings directory: {error}"))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| format!("cannot create settings temp file: {error}"))?;
        serde_json::to_writer_pretty(&mut temp, self).map_err(|error| error.to_string())?;
        temp.write_all(b"\n").map_err(|error| error.to_string())?;
        temp.as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        temp.persist(&path)
            .map_err(|error| format!("cannot replace settings file: {error}"))?;
        Ok(())
    }
}

fn migrate_loaded(settings: &mut Settings) {
    // The first public default was Ctrl+Alt+Space. The trigger is now the modifiers alone.
    if settings.trigger
        == (Hotkey {
            control: true,
            alt: true,
            shift: false,
            win: false,
            key: 0x20,
        })
    {
        settings.trigger.key = 0;
    }
}

fn validate_hotkey(hotkey: Hotkey, field: &str, allow_modifiers_only: bool) -> Result<(), String> {
    let modifiers_only = allow_modifiers_only && hotkey.key == 0;
    if !modifiers_only
        && (hotkey.key == 0
            || hotkey.key > 0xff
            || matches!(hotkey.key, 0x10 | 0x11 | 0x12 | 0x5b | 0x5c | 0xa0..=0xa5))
    {
        return Err(format!("{field} requires a valid non-modifier virtual key"));
    }
    if !(hotkey.control || hotkey.alt || hotkey.shift || hotkey.win) {
        return Err(format!("{field} requires at least one modifier"));
    }
    Ok(())
}

fn validate_action(action: Action, frame_count: usize) -> Result<(), String> {
    if let Action::Custom(index) = action
        && usize::from(index) >= frame_count
    {
        return Err(format!("custom frame index {index} is out of range"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_config_gains_new_defaults() {
        let settings: Settings = serde_json::from_str(r#"{"version":1,"padding":12}"#).unwrap();
        assert_eq!(settings.padding, 12);
        assert!(settings.radial_menu_visible);
        assert!(!settings.snap_on_drag);
        assert!(settings.trigger.control && settings.trigger.alt);
        assert_eq!(settings.trigger.key, 0);
        let mut previous_default: Settings =
            serde_json::from_str(r#"{"version":1,"trigger":{"control":true,"alt":true,"key":32}}"#)
                .unwrap();
        migrate_loaded(&mut previous_default);
        assert_eq!(previous_default.trigger.key, 0);
        assert_eq!(settings.radial_size, 100);
        assert_eq!(settings.edge_padding, None);
        assert_eq!(settings.trigger_side, TriggerSide::Either);
        assert!(!settings.include_development_versions);
        assert!(!settings.double_tap_to_trigger);
        settings.validate().unwrap();
    }

    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            padding: 17,
            preview_visible: false,
            ..Default::default()
        };
        let parsed: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(parsed.padding, 17);
        assert!(!parsed.preview_visible);
        parsed.validate().unwrap();
    }

    #[test]
    fn rejects_invalid_custom_frames_and_hotkeys() {
        let mut settings = Settings::default();
        settings.custom_frames.push(CustomFrame {
            name: "bad".into(),
            x: 0.8,
            y: 0.0,
            width: 0.4,
            height: 1.0,
        });
        assert!(settings.validate().is_err());
        settings.custom_frames.clear();
        settings.trigger.key = 0;
        settings.validate().unwrap();
        settings.trigger.control = false;
        settings.trigger.alt = false;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn advanced_preferences_round_trip_and_reject_invalid_ranges() {
        let settings = Settings {
            edge_padding: Some(EdgePadding {
                top: 2,
                right: 4,
                bottom: 8,
                left: 16,
            }),
            trigger_side: TriggerSide::Right,
            preview_start: PreviewStart::RadialMenu,
            include_development_versions: true,
            double_tap_to_trigger: true,
            padding_minimum_screen_inches: 24.0,
            ..Default::default()
        };
        settings.validate().unwrap();
        let parsed: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(parsed.edge_padding, settings.edge_padding);
        assert_eq!(parsed.trigger_side, TriggerSide::Right);
        assert_eq!(parsed.preview_start, PreviewStart::RadialMenu);
        assert!(parsed.include_development_versions && parsed.double_tap_to_trigger);
        for mutate in [
            |s: &mut Settings| s.edge_padding.as_mut().unwrap().left = -1,
            |s: &mut Settings| s.preview_border_thickness = 33,
            |s: &mut Settings| s.animation_duration_ms = 2001,
            |s: &mut Settings| s.trigger_timeout_ms = 600001,
            |s: &mut Settings| s.padding_minimum_screen_inches = f64::NAN,
            |s: &mut Settings| s.gradient_color = 0x1000000,
        ] {
            let mut invalid = settings.clone();
            mutate(&mut invalid);
            assert!(invalid.validate().is_err());
        }
    }
}
