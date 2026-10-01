#![cfg_attr(not(test), windows_subsystem = "windows")]

mod configuration;
mod history;
mod login;
mod platform;
#[cfg(test)]
mod preview;
mod radial;
mod settings_window;

use orbit::command::{self, Command};
use orbit::geometry::Action;
use orbit::settings::Settings;
use windows::core::w;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let interactive = args.is_empty() || args == ["--resident"];
    if !interactive {
        unsafe {
            let _ = windows::Win32::System::Console::AttachConsole(
                windows::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
    let command = match command::parse(&args) {
        Ok(command) => command,
        Err(error) => {
            print_error(&error, "Run orbit --help for supported commands.");
            std::process::exit(2);
        }
    };
    if let Err(error) = execute(command) {
        if interactive {
            let message: Vec<u16> = error.encode_utf16().chain(Some(0)).collect();
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    None,
                    windows::core::PCWSTR(message.as_ptr()),
                    w!("Orbit could not start"),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
        } else {
            print_error(
                &error,
                "Check the error, correct the settings or target, and retry.",
            );
        }
        std::process::exit(1);
    }
}

fn print_error(error: &str, help: &str) {
    println!(
        "error: {}\nhelp: {}",
        serde_json::to_string(error).unwrap(),
        serde_json::to_string(help).unwrap()
    );
}

fn running() -> bool {
    platform::running_status()
}

fn execute(command: Command) -> Result<(), String> {
    match command {
        Command::Resident => platform::run(),
        Command::Quit => {
            platform::quit_running()?;
            println!("status: stopped");
            Ok(())
        }
        Command::Settings => {
            open_settings()?;
            println!("status: opened\nwindow: settings");
            Ok(())
        }
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Status => {
            let settings = Settings::load()?;
            println!(
                "version: {}\nrunning: {}\nsettings: {}\nupdates_configured: {}\nupdates_enabled: {}",
                env!("CARGO_PKG_VERSION"),
                running(),
                serde_json::to_string(&Settings::path()?.display().to_string()).unwrap(),
                orbit::update::configured().is_some(),
                settings.updates_enabled
            );
            Ok(())
        }
        Command::ListActions => {
            let actions: Vec<String> = Action::ALL
                .iter()
                .map(|action| {
                    serde_json::to_value(action)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .replace('_', "-")
                })
                .collect();
            println!(
                "actions[{}]: {}\ncustom_action: custom:INDEX",
                actions.len(),
                actions.join(",")
            );
            Ok(())
        }
        Command::Apply(action) => {
            platform::dispatch_foreground(action)?;
            println!(
                "status: applied\naction: {}",
                serde_json::to_string(&action).unwrap()
            );
            Ok(())
        }
        Command::ExportSettings(path) => {
            configuration::export(&Settings::load()?, &path)?;
            println!(
                "status: exported\npath: {}",
                serde_json::to_string(&path.display().to_string()).unwrap()
            );
            Ok(())
        }
        Command::ImportSettings(path) => {
            configuration::save(&configuration::read_import(&path)?)?;
            println!("status: imported");
            Ok(())
        }
        Command::ResetSettings => {
            configuration::reset()?;
            println!("status: reset");
            Ok(())
        }
        Command::CheckUpdates => {
            if let Some(manifest) = orbit::update::check()? {
                println!(
                    "status: update_available\nversion: {}\nhelp: Open Orbit settings to download and install the update.",
                    manifest.version
                );
            } else {
                println!("status: current");
            }
            Ok(())
        }
        Command::Help => {
            println!(
                "Orbit {}\nNative Windows window management.\n\nUsage: orbit [ACTION | FLAG]\n\nActions: left, right, top, bottom, maximize, undo, custom:0, and more.\n  --list-actions             List all action names\n  --resident                 Run in the notification area\n  --settings                 Open settings, starting Orbit if necessary\n  --status                   Report resident and update status\n  --quit                     Close Orbit and restore stashed windows\n  --export-settings FILE     Export preferences to a new JSON file\n  --import-settings FILE     Validate and apply a JSON preferences file\n  --reset-settings           Restore default preferences\n  --check-updates            Check for an update without installing it\n  --version                  Print version\n  --help                     Show this help\n\nDefault trigger: hold Ctrl+Alt, point, release. Change the modifiers in Settings. Escape cancels.\nDefault undo: Ctrl+Alt+Z. Bindings are editable in Settings.\nURLs: orbit://settings or orbit://action/left-half\n\nExamples:\n  orbit left-half\n  orbit --export-settings preferences.json\n  orbit --import-settings preferences.json",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
    }
}

fn open_settings() -> Result<(), String> {
    if !running() {
        use std::os::windows::process::CommandExt;
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut child = std::process::Command::new(exe)
            .arg("--resident")
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|error| format!("Cannot start Orbit: {error}"))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !running() {
            if child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err("Orbit exited before settings could open.".into());
            }
            if std::time::Instant::now() >= deadline {
                return Err("Orbit did not start within five seconds.".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
    platform::open_running_settings()
}
