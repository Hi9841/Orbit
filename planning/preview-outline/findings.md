# Findings

## Loop mechanism
Source: mrkai77/Loop branch `develop`, fetched 2026-09-29.

- `Loop/Window Action Indicators/WindowActionIndicatorService.swift` opens the preview when preview visibility is on, then the radial menu.
- `PreviewController.swift` creates one `NSPanel` (borderless, nonactivating). `ignoresMouseEvents = true`. `hasShadow = false`. `backgroundColor = .clear`. Level is screensaver minus 1. The panel frame is the **whole screen**, not the target window. `orderFrontRegardless()`.
- `PreviewView.swift` draws inside that screen:
  1. `VisualEffectView` material `.hudWindow`, blending `.behindWindow` (the frosted square).
  2. Accent linear gradient at `previewBackgroundAccentOpacity`.
  3. Clip to the corner radii.
  4. 1 px `.quinary` stroke.
  5. Accent stroke at `previewBorderThickness`.
- `PreviewViewModel.swift` places that plate at the padded target frame in screen-local coordinates and fades it in. Starting position can be screen center, radial menu, or action center.

## Why Orbit shows nothing and breaks hover
Orbit sizes one layered HWND to the target rectangle and calls `UpdateLayeredWindow` before the window is moved. An empty topmost popup then sits on the desktop. It eats hover, and with border thickness 0 there is no outline.

## Contract for the new renderer
`render_screen_preview(settings, action, screen_size, frame, dpi, system_accent) -> PreviewBitmap`

- `screen_size` is `(width, height)` of the monitor bitmap.
- `frame` is `(x, y, width, height)` of the plate in that bitmap.
- Pixels outside the plate are alpha 0.
- Plate fill is dark glass, about `(32, 34, 38)` at alpha 180, then a src-over accent wash at `preview_opacity`.
- A 1 px light stroke and an accent stroke of `preview_border_thickness` (DPI-scaled) sit on the plate edge.
- `update_layered_window(window, bitmap, destination: Option<(i32, i32)>)` passes that point as `pptDst`.

Tests to keep green: outside corner alpha 0, interior alpha at least 160, accent border alpha 255 when thickness is greater than 0.
