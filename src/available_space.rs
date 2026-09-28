//! Adapted from Loop's WindowFrameResolver.getFillAvailableSpaceFrame.
//! Original work by Kai Azim and Loop contributors, GPL-3.0.
use crate::geometry::Rect;

fn intersects(a: Rect, b: Rect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// Expand around the current window without crossing other nonoverlapping windows.
/// Already-overlapping windows do not obstruct expansion, matching the source behavior.
pub fn largest_frame(bounds: Rect, current: Rect, obstacles: &[Rect]) -> Rect {
    let obstacles: Vec<Rect> = obstacles
        .iter()
        .copied()
        .filter(|rect| !intersects(*rect, current))
        .map(|rect| Rect {
            left: rect.left.max(bounds.left),
            top: rect.top.max(bounds.top),
            right: rect.right.min(bounds.right),
            bottom: rect.bottom.min(bounds.bottom),
        })
        .filter(|rect| rect.width() > 0 && rect.height() > 0)
        .collect();
    let mut limits = bounds;
    for rect in &obstacles {
        if rect.right <= current.left {
            limits.left = limits.left.max(rect.right);
        }
        if rect.bottom <= current.top {
            limits.top = limits.top.max(rect.bottom);
        }
        if rect.left >= current.right {
            limits.right = limits.right.min(rect.left);
        }
        if rect.top >= current.bottom {
            limits.bottom = limits.bottom.min(rect.top);
        }
    }
    let horizontal = [
        (limits.left, limits.right),
        (current.left, limits.right),
        (limits.left, current.right),
        (current.left, bounds.right),
        (bounds.left, current.right),
        (bounds.left, bounds.right),
    ];
    let vertical = [
        (limits.top, limits.bottom),
        (current.top, limits.bottom),
        (limits.top, current.bottom),
        (current.top, bounds.bottom),
        (bounds.top, current.bottom),
        (bounds.top, bounds.bottom),
    ];
    let area = |rect: Rect| i64::from(rect.width()) * i64::from(rect.height());
    let mut best = current;
    for (left, right) in horizontal {
        for (top, bottom) in vertical {
            let candidate = Rect {
                left,
                top,
                right,
                bottom,
            };
            if candidate.width() <= 0
                || candidate.height() <= 0
                || left < bounds.left
                || right > bounds.right
                || top < bounds.top
                || bottom > bounds.bottom
                || obstacles.iter().any(|rect| intersects(candidate, *rect))
            {
                continue;
            }
            if area(candidate) > area(best) {
                best = candidate;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    const BOUNDS: Rect = Rect {
        left: -1000,
        top: 0,
        right: 0,
        bottom: 800,
    };
    const CURRENT: Rect = Rect {
        left: -800,
        top: 200,
        right: -500,
        bottom: 500,
    };

    #[test]
    fn expands_to_work_area_without_obstacles() {
        assert_eq!(largest_frame(BOUNDS, CURRENT, &[]), BOUNDS);
    }

    #[test]
    fn stops_at_a_nonoverlapping_neighbor() {
        let neighbor = Rect {
            left: -400,
            top: 0,
            right: 0,
            bottom: 800,
        };
        assert_eq!(
            largest_frame(BOUNDS, CURRENT, &[neighbor]),
            Rect {
                right: -400,
                ..BOUNDS
            }
        );
    }

    #[test]
    fn ignores_overlapping_and_off_monitor_windows() {
        let overlap = Rect {
            left: -900,
            top: 100,
            right: -600,
            bottom: 400,
        };
        let other_monitor = Rect {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 800,
        };
        assert_eq!(
            largest_frame(BOUNDS, CURRENT, &[overlap, other_monitor]),
            BOUNDS
        );
    }

    #[test]
    fn evaluates_corner_obstacles_without_crossing_them() {
        let obstacle = Rect {
            left: -400,
            top: 600,
            right: 0,
            bottom: 800,
        };
        let result = largest_frame(BOUNDS, CURRENT, &[obstacle]);
        assert!(!intersects(result, obstacle));
        assert_eq!(
            result,
            Rect {
                bottom: 600,
                ..BOUNDS
            }
        );
    }
}
