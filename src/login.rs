use std::os::windows::ffi::OsStrExt;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
};
use windows::core::w;

const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let path =
            std::env::current_exe().map_err(|e| format!("cannot locate Orbit executable: {e}"))?;
        let command = format!("\"{}\" --resident", path.display());
        let data: Vec<u16> = std::ffi::OsStr::new(&command)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let result = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                w!("Orbit"),
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            )
        };
        if result.0 != 0 {
            return Err(format!(
                "cannot enable launch at login: Windows error {}",
                result.0
            ));
        }
    } else {
        let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Orbit")) };
        if result.0 != 0 && result.0 != 2 {
            return Err(format!(
                "cannot disable launch at login: Windows error {}",
                result.0
            ));
        }
    }
    Ok(())
}
