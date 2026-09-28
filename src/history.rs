use std::collections::VecDeque;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetPropW, GetWindowLongPtrW, GetWindowPlacement, RemovePropW,
    SHOW_WINDOW_CMD, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SetPropW, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, ShowWindow, WINDOWPLACEMENT,
};
use windows::core::w;

// The property disappears with the window, so a recycled HWND cannot inherit its history.
const PROPERTY: windows::core::PCWSTR = w!("Orbit.History.BDF8B61C");
const LIMIT: usize = 128;

#[derive(Clone)]
struct Entry {
    window: HWND,
    token: usize,
    placement: WINDOWPLACEMENT,
    style: isize,
    ex_style: isize,
}

#[derive(Default)]
pub struct History {
    entries: VecDeque<Entry>,
    initial: Vec<Entry>,
    next_token: usize,
}

pub fn placement(window: HWND) -> Result<WINDOWPLACEMENT, String> {
    let mut value = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe { GetWindowPlacement(window, &mut value) }
        .map_err(|error| format!("cannot read window placement: {error}"))?;
    Ok(value)
}

/// Stable per-window token used to reject animation or recovery work after HWND reuse.
pub fn identity_token(window: HWND) -> usize {
    unsafe { GetPropW(window, PROPERTY) }.0 as usize
}

impl History {
    pub fn capture_initial(&mut self, window: HWND) -> Result<(), String> {
        let mut token = unsafe { GetPropW(window, PROPERTY) }.0 as usize;
        if token == 0 {
            self.next_token = self.next_token.wrapping_add(1).max(1);
            token = self.next_token;
            unsafe { SetPropW(window, PROPERTY, Some(HANDLE(token as *mut _))) }
                .map_err(|error| format!("cannot track initial window position: {error}"))?;
        }
        if self
            .initial
            .iter()
            .any(|entry| entry.window == window && entry.token == token)
        {
            return Ok(());
        }
        let placement = placement(window)?;
        self.initial.push(Entry {
            window,
            token,
            placement,
            style: unsafe { GetWindowLongPtrW(window, GWL_STYLE) },
            ex_style: unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) },
        });
        if self.initial.len() > LIMIT {
            let old = self.initial.remove(0);
            self.release_if_unused(&old);
        }
        Ok(())
    }

    pub fn record(&mut self, window: HWND, placement: WINDOWPLACEMENT) -> Result<(), String> {
        let mut token = unsafe { GetPropW(window, PROPERTY) }.0 as usize;
        if token == 0 {
            self.next_token = self.next_token.wrapping_add(1).max(1);
            token = self.next_token;
            unsafe { SetPropW(window, PROPERTY, Some(HANDLE(token as *mut _))) }
                .map_err(|error| format!("cannot track window history: {error}"))?;
        }
        let entry = Entry {
            window,
            token,
            placement,
            style: unsafe { GetWindowLongPtrW(window, GWL_STYLE) },
            ex_style: unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) },
        };
        if !self
            .initial
            .iter()
            .any(|old| old.window == window && old.token == token)
        {
            self.initial.push(entry.clone());
        }
        self.entries.push_back(entry);
        if self.entries.len() > LIMIT {
            let old = self.entries.pop_front().unwrap();
            self.release_if_unused(&old);
        }
        if self.initial.len() > LIMIT {
            let old = self.initial.remove(0);
            self.release_if_unused(&old);
        }
        Ok(())
    }

    pub fn undo(&mut self, window: Option<HWND>) -> Result<(), String> {
        self.prune_destroyed();
        let index = self
            .entries
            .iter()
            .rposition(|entry| window.is_none_or(|w| w == entry.window))
            .ok_or("No previous window position is available.")?;
        let entry = &self.entries[index];
        restore(entry).map_err(|error| format!("cannot restore window position: {error}"))?;
        let entry = self.entries.remove(index).unwrap();
        self.release_if_unused(&entry);
        Ok(())
    }

    pub fn initial_frame(&mut self, window: HWND) -> Result<(), String> {
        self.prune_destroyed();
        let entry = self
            .initial
            .iter()
            .rfind(|entry| entry.window == window)
            .ok_or("No initial window position is available yet.")?;
        restore(entry).map_err(|error| format!("cannot restore initial window position: {error}"))
    }

    pub fn has_initial_frame(&mut self, window: HWND) -> bool {
        self.prune_destroyed();
        self.initial.iter().any(|entry| entry.window == window)
    }

    pub fn last_window(&mut self) -> Option<HWND> {
        self.prune_destroyed();
        self.entries.back().map(|entry| entry.window)
    }

    fn prune_destroyed(&mut self) {
        self.entries
            .retain(|entry| unsafe { GetPropW(entry.window, PROPERTY).0 as usize == entry.token });
        self.initial
            .retain(|entry| unsafe { GetPropW(entry.window, PROPERTY).0 as usize == entry.token });
    }

    fn referenced(&self, entry: &Entry) -> bool {
        self.entries
            .iter()
            .chain(&self.initial)
            .any(|other| other.window == entry.window && other.token == entry.token)
    }

    fn release_if_unused(&self, entry: &Entry) {
        if !self.referenced(entry)
            && unsafe { GetPropW(entry.window, PROPERTY) }.0 as usize == entry.token
        {
            let _ = unsafe { RemovePropW(entry.window, PROPERTY) };
        }
    }
}

fn restore(entry: &Entry) -> windows::core::Result<()> {
    restore_style(entry)?;
    unsafe {
        SetWindowPlacement(entry.window, &entry.placement)?;
    }
    unsafe {
        let _ = ShowWindow(
            entry.window,
            SHOW_WINDOW_CMD(entry.placement.showCmd as i32),
        );
    }
    Ok(())
}

fn restore_style(entry: &Entry) -> windows::core::Result<()> {
    unsafe {
        let _ = SetWindowLongPtrW(entry.window, GWL_STYLE, entry.style);
        let _ = SetWindowLongPtrW(entry.window, GWL_EXSTYLE, entry.ex_style);
        SetWindowPos(
            entry.window,
            None,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
        )?;
    }
    Ok(())
}

impl Drop for History {
    fn drop(&mut self) {
        let entries: Vec<_> = self
            .entries
            .drain(..)
            .chain(self.initial.drain(..))
            .collect();
        for entry in entries {
            if unsafe { GetPropW(entry.window, PROPERTY) }.0 as usize == entry.token {
                let _ = unsafe { RemovePropW(entry.window, PROPERTY) };
            }
        }
    }
}
