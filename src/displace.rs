//! Move windows that a half or corner placement covers.
use crate::geometry::{Action, Rect};

/// The tile opposite a half or a corner. Other placements have none.
pub fn opposite_tile(action: Action) -> Option<Action> {
    Some(match action {
        Action::LeftHalf => Action::RightHalf,
        Action::RightHalf => Action::LeftHalf,
        Action::TopHalf => Action::BottomHalf,
        Action::BottomHalf => Action::TopHalf,
        Action::TopLeft => Action::BottomRight,
        Action::BottomRight => Action::TopLeft,
        Action::TopRight => Action::BottomLeft,
        Action::BottomLeft => Action::TopRight,
        _ => return None,
    })
}

pub fn overlaps(a: Rect, b: Rect) -> bool {
    overlap_area(a, b) > 0
}

pub fn overlap_area(a: Rect, b: Rect) -> i64 {
    let width = (a.right.min(b.right) - a.left.max(b.left)).max(0) as i64;
    let height = (a.bottom.min(b.bottom) - a.top.max(b.top)).max(0) as i64;
    width * height
}

/// A window counts when the placement covers a real part of it, not a sliver or a toolbar.
pub fn substantially_covered(candidate: Rect, placed: Rect) -> bool {
    if candidate.width() < 160 || candidate.height() < 120 {
        return false;
    }
    let area = i64::from(candidate.width()) * i64::from(candidate.height());
    area > 0 && overlap_area(candidate, placed) * 5 >= area
}

/// Equal columns across `region`, left to right.
pub fn column_frames(region: Rect, count: usize) -> Vec<Rect> {
    let count = count.max(1) as i32;
    (0..count)
        .map(|index| {
            let x = |n| {
                (i64::from(region.left)
                    + i64::from(region.width()) * i64::from(n) / i64::from(count))
                    as i32
            };
            Rect {
                left: x(index),
                right: x(index + 1),
                top: region.top,
                bottom: region.bottom,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(action: Action, work: Rect) -> Rect {
        action.frame(work, work, 0)
    }

    fn area(rect: Rect) -> i64 {
        i64::from(rect.width()) * i64::from(rect.height())
    }

    #[test]
    fn every_standard_tile_fills_its_opposite_without_a_gap() {
        let work = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let halves = [
            (Action::LeftHalf, Action::RightHalf),
            (Action::RightHalf, Action::LeftHalf),
            (Action::TopHalf, Action::BottomHalf),
            (Action::BottomHalf, Action::TopHalf),
        ];
        for (placed, opposite) in halves {
            assert_eq!(opposite_tile(placed), Some(opposite));
            let here = tile(placed, work);
            let there = tile(opposite, work);
            assert!(!overlaps(here, there), "{placed:?} overlaps its opposite");
            assert_eq!(here.left.min(there.left), work.left);
            assert_eq!(here.top.min(there.top), work.top);
            assert_eq!(here.right.max(there.right), work.right);
            assert_eq!(here.bottom.max(there.bottom), work.bottom);
            assert_eq!(area(here) + area(there), area(work));
        }
        let corners = [
            (Action::TopLeft, Action::BottomRight),
            (Action::TopRight, Action::BottomLeft),
            (Action::BottomLeft, Action::TopRight),
            (Action::BottomRight, Action::TopLeft),
        ];
        for (placed, opposite) in corners {
            assert_eq!(opposite_tile(placed), Some(opposite));
            let here = tile(placed, work);
            let there = tile(opposite, work);
            assert!(!overlaps(here, there), "{placed:?} overlaps its opposite");
            assert_eq!(area(here), area(work) / 4);
            assert_eq!(area(there), area(work) / 4);
            assert!(there.left >= work.left && there.right <= work.right);
            assert!(there.top >= work.top && there.bottom <= work.bottom);
        }
    }

    #[test]
    fn non_tiles_cannot_move_another_window() {
        for action in [
            Action::NoAction,
            Action::Center,
            Action::Maximize,
            Action::Custom(0),
            Action::LeftThird,
            Action::FillAvailableSpace,
            Action::Stash,
            Action::Undo,
        ] {
            assert_eq!(opposite_tile(action), None, "{action:?}");
        }
    }

    #[test]
    fn halves_and_corners_have_opposites() {
        assert_eq!(opposite_tile(Action::LeftHalf), Some(Action::RightHalf));
        assert_eq!(opposite_tile(Action::RightHalf), Some(Action::LeftHalf));
        assert_eq!(opposite_tile(Action::TopHalf), Some(Action::BottomHalf));
        assert_eq!(opposite_tile(Action::TopLeft), Some(Action::BottomRight));
        assert_eq!(opposite_tile(Action::BottomLeft), Some(Action::TopRight));
        assert_eq!(opposite_tile(Action::Center), None);
        assert_eq!(opposite_tile(Action::Maximize), None);
        assert_eq!(opposite_tile(Action::Custom(1)), None);
    }

    #[test]
    fn any_overlap_counts_and_a_gap_does_not() {
        let placed = Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };
        assert!(overlaps(
            placed,
            Rect {
                left: 99,
                top: 0,
                right: 120,
                bottom: 10,
            }
        ));
        assert!(!overlaps(
            placed,
            Rect {
                left: 100,
                top: 0,
                right: 140,
                bottom: 40,
            }
        ));
        let big = Rect {
            left: 0,
            top: 0,
            right: 400,
            bottom: 300,
        };
        assert!(substantially_covered(
            Rect {
                left: 50,
                top: 40,
                right: 350,
                bottom: 280,
            },
            big
        ));
        assert!(!substantially_covered(
            Rect {
                left: 380,
                top: 0,
                right: 700,
                bottom: 300,
            },
            big
        ));
    }

    #[test]
    fn columns_share_the_region_without_gaps() {
        let region = Rect {
            left: 100,
            top: 10,
            right: 400,
            bottom: 210,
        };
        let frames = column_frames(region, 3);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].left, 100);
        assert_eq!(frames[2].right, 400);
        assert_eq!(frames[0].right, frames[1].left);
        assert_eq!(frames[1].right, frames[2].left);
        assert!(
            frames
                .iter()
                .all(|frame| frame.top == 10 && frame.bottom == 210)
        );
    }
}
