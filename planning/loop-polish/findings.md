# Findings

- `update_preview()` in `src/platform.rs` computes the destination frame, captures and blurs the destination pixels on cache miss, then immediately positions and paints the plate at that frame. There is no preview animation state or timer.
- `paint_color_key_bitmap()` allocates and converts every pixel on each `WM_PAINT`. A typical half-screen preview is about 944 x 1016 pixels, so animation at 60 Hz would repeatedly convert nearly one million pixels.
- Real target-window animation already has `interpolate_rect()` and a 16 ms timer, but preview updates do not use either. User setting `animation_duration_ms` defaults to 180 ms, and `animations_allowed()` checks Windows animation preferences and power state.
- Current radial uses an angular wedge over a rounded-square SDF. Its body is medium gray with a bright continuous rim; the selected segment has straight angular cutoffs. The GIF shows a darker, softer ring and a shorter pale arc following the square's corner.
- A release timing sample of a 944 x 1016 plate measured about 11.7 ms blur, 13.9 ms render, and 2.7 ms color-key conversion. Conversion repeated on every repaint in the previous path; a cached `PresentedBitmap` moves it to selection changes.
- The pinned Loop source starts an action-center preview at 80% of its target and animates its frame. Orbit's existing duration is 180 ms. The new preview timer eases from the current frame to the next frame and retargets mid-flight.
- A radial cap path based on the rounded-square perimeter gives a continuous corner arc. The first mapping classified inner straight pixels as a corner and produced a stray line; classifying those by the nearer straight side removed it.
- Settings previously painted native white controls over a white shell. The Settings worker added dark DWM chrome, reusable brushes, and owner-drawn neutral sidebar selection while preserving the control IDs and save flow.
