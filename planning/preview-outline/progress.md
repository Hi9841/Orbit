# Progress

## 2026-09-29
- Read Loop `PreviewController.swift`, `PreviewView.swift`, `PreviewViewModel.swift`, and `WindowActionIndicatorService.swift` from `develop`.
- Prior Orbit attempts resized a small layered window. The user still saw no border, and hover broke.
- Next: two workers, renderer in `src/preview.rs` and host in `src/platform.rs`.
- Renderer added `render_screen_preview` and a destination argument on `update_layered_window`. Preview tests: 3 passed.
- Host sizes the preview HWND to `monitor.full` and draws the plate inside it. Release build finished. Installed exe replaced and Orbit restarted.
