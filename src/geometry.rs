use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }
    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
    pub fn center(self) -> (i32, i32) {
        (
            (i64::from(self.left) + i64::from(self.width()) / 2)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            (i64::from(self.top) + i64::from(self.height()) / 2)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        )
    }
    pub fn inset(self, padding: i32) -> Self {
        let p = padding
            .max(0)
            .min((self.width().min(self.height()) - 1).max(0) / 2);
        Self {
            left: self.left.saturating_add(p),
            top: self.top.saturating_add(p),
            right: self.right.saturating_sub(p),
            bottom: self.bottom.saturating_sub(p),
        }
    }
    /// Inset a work area independently on each edge while retaining at least one pixel.
    pub fn inset_edges(self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        let width = self.width().max(1);
        let height = self.height().max(1);
        let left = left.max(0).min(width - 1);
        let right = right.max(0).min(width - 1 - left);
        let top = top.max(0).min(height - 1);
        let bottom = bottom.max(0).min(height - 1 - top);
        Self {
            left: self.left.saturating_add(left),
            top: self.top.saturating_add(top),
            right: self.right.saturating_sub(right),
            bottom: self.bottom.saturating_sub(bottom),
        }
    }
    fn columns(self, parts: i32, first: i32, count: i32) -> Self {
        let x = |n| {
            (i64::from(self.left) + i64::from(self.width()) * i64::from(n) / i64::from(parts))
                as i32
        };
        Self {
            left: x(first),
            right: x(first + count),
            ..self
        }
    }
    fn rows(self, parts: i32, first: i32, count: i32) -> Self {
        let y = |n| {
            (i64::from(self.top) + i64::from(self.height()) * i64::from(n) / i64::from(parts))
                as i32
        };
        Self {
            top: y(first),
            bottom: y(first + count),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    NoAction,
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    LeftThird,
    CenterThird,
    RightThird,
    LeftTwoThirds,
    RightTwoThirds,
    TopThird,
    MiddleThird,
    BottomThird,
    TopTwoThirds,
    BottomTwoThirds,
    FirstFourth,
    SecondFourth,
    ThirdFourth,
    FourthFourth,
    LeftThreeFourths,
    RightThreeFourths,
    HorizontalCenterHalf,
    VerticalCenterHalf,
    Center,
    AlmostMaximize,
    Maximize,
    Fullscreen,
    FillAvailableSpace,
    MacOSCenter,
    MaximizeWidth,
    MaximizeHeight,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    GrowLeft,
    GrowRight,
    GrowTop,
    GrowBottom,
    ShrinkLeft,
    ShrinkRight,
    ShrinkTop,
    ShrinkBottom,
    ShrinkHorizontal,
    ShrinkVertical,
    GrowHorizontal,
    GrowVertical,
    Minimize,
    Restore,
    Hide,
    MinimizeOthers,
    Undo,
    InitialFrame,
    NextMonitor,
    PreviousMonitor,
    MoveToMonitorLeft,
    MoveToMonitorRight,
    MoveToMonitorUp,
    MoveToMonitorDown,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    FocusNextInStack,
    Larger,
    Smaller,
    ScaleUp,
    ScaleDown,
    Stash,
    StashLeft,
    StashRight,
    StashUp,
    StashDown,
    Unstash,
    Custom(u16),
}

impl Action {
    pub const RADIAL: [Self; 8] = [
        Self::RightHalf,
        Self::BottomRight,
        Self::BottomHalf,
        Self::BottomLeft,
        Self::LeftHalf,
        Self::TopLeft,
        Self::TopHalf,
        Self::TopRight,
    ];

    pub const ALL: &'static [Self] = &[
        Self::NoAction,
        Self::LeftHalf,
        Self::RightHalf,
        Self::TopHalf,
        Self::BottomHalf,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
        Self::LeftThird,
        Self::CenterThird,
        Self::RightThird,
        Self::LeftTwoThirds,
        Self::RightTwoThirds,
        Self::TopThird,
        Self::MiddleThird,
        Self::BottomThird,
        Self::TopTwoThirds,
        Self::BottomTwoThirds,
        Self::FirstFourth,
        Self::SecondFourth,
        Self::ThirdFourth,
        Self::FourthFourth,
        Self::LeftThreeFourths,
        Self::RightThreeFourths,
        Self::HorizontalCenterHalf,
        Self::VerticalCenterHalf,
        Self::Center,
        Self::MacOSCenter,
        Self::AlmostMaximize,
        Self::Maximize,
        Self::Fullscreen,
        Self::FillAvailableSpace,
        Self::MaximizeWidth,
        Self::MaximizeHeight,
        Self::MoveLeft,
        Self::MoveRight,
        Self::MoveUp,
        Self::MoveDown,
        Self::GrowLeft,
        Self::GrowRight,
        Self::GrowTop,
        Self::GrowBottom,
        Self::ShrinkLeft,
        Self::ShrinkRight,
        Self::ShrinkTop,
        Self::ShrinkBottom,
        Self::GrowHorizontal,
        Self::GrowVertical,
        Self::ShrinkHorizontal,
        Self::ShrinkVertical,
        Self::Larger,
        Self::Smaller,
        Self::ScaleUp,
        Self::ScaleDown,
        Self::Minimize,
        Self::Restore,
        Self::Hide,
        Self::MinimizeOthers,
        Self::Undo,
        Self::InitialFrame,
        Self::NextMonitor,
        Self::PreviousMonitor,
        Self::MoveToMonitorLeft,
        Self::MoveToMonitorRight,
        Self::MoveToMonitorUp,
        Self::MoveToMonitorDown,
        Self::FocusLeft,
        Self::FocusRight,
        Self::FocusUp,
        Self::FocusDown,
        Self::FocusNextInStack,
        Self::Stash,
        Self::StashLeft,
        Self::StashRight,
        Self::StashUp,
        Self::StashDown,
        Self::Unstash,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::NoAction => "No action",
            Self::LeftHalf => "Left half",
            Self::RightHalf => "Right half",
            Self::TopHalf => "Top half",
            Self::BottomHalf => "Bottom half",
            Self::TopLeft => "Top left",
            Self::TopRight => "Top right",
            Self::BottomLeft => "Bottom left",
            Self::BottomRight => "Bottom right",
            Self::LeftThird => "Left third",
            Self::CenterThird => "Center third",
            Self::RightThird => "Right third",
            Self::Center => "Center",
            Self::MacOSCenter => "Center",
            Self::LeftTwoThirds => "Left two thirds",
            Self::RightTwoThirds => "Right two thirds",
            Self::TopThird => "Top third",
            Self::MiddleThird => "Middle third",
            Self::BottomThird => "Bottom third",
            Self::TopTwoThirds => "Top two thirds",
            Self::BottomTwoThirds => "Bottom two thirds",
            Self::FirstFourth => "First fourth",
            Self::SecondFourth => "Second fourth",
            Self::ThirdFourth => "Third fourth",
            Self::FourthFourth => "Fourth fourth",
            Self::LeftThreeFourths => "Left three fourths",
            Self::RightThreeFourths => "Right three fourths",
            Self::HorizontalCenterHalf => "Horizontal center half",
            Self::VerticalCenterHalf => "Vertical center half",
            Self::AlmostMaximize => "Almost maximize",
            Self::Maximize => "Maximize",
            Self::Fullscreen => "Fullscreen",
            Self::FillAvailableSpace => "Fill available space",
            Self::MaximizeWidth => "Maximize width",
            Self::MaximizeHeight => "Maximize height",
            Self::MoveLeft => "Move left",
            Self::MoveRight => "Move right",
            Self::MoveUp => "Move up",
            Self::MoveDown => "Move down",
            Self::GrowLeft => "Grow left",
            Self::GrowRight => "Grow right",
            Self::GrowTop => "Grow top",
            Self::GrowBottom => "Grow bottom",
            Self::ShrinkLeft => "Shrink left",
            Self::ShrinkRight => "Shrink right",
            Self::ShrinkTop => "Shrink top",
            Self::ShrinkBottom => "Shrink bottom",
            Self::ShrinkHorizontal => "Shrink horizontal",
            Self::ShrinkVertical => "Shrink vertical",
            Self::GrowHorizontal => "Grow horizontal",
            Self::GrowVertical => "Grow vertical",
            Self::Minimize => "Minimize",
            Self::Restore => "Restore",
            Self::Hide => "Hide",
            Self::MinimizeOthers => "Minimize others",
            Self::Undo => "Undo",
            Self::InitialFrame => "Initial frame",
            Self::NextMonitor => "Next monitor",
            Self::PreviousMonitor => "Previous monitor",
            Self::MoveToMonitorLeft => "Move to monitor left",
            Self::MoveToMonitorRight => "Move to monitor right",
            Self::MoveToMonitorUp => "Move to monitor above",
            Self::MoveToMonitorDown => "Move to monitor below",
            Self::FocusLeft => "Focus left",
            Self::FocusRight => "Focus right",
            Self::FocusUp => "Focus up",
            Self::FocusDown => "Focus down",
            Self::FocusNextInStack => "Focus next window",
            Self::Larger => "Larger",
            Self::Smaller => "Smaller",
            Self::ScaleUp => "Scale up",
            Self::ScaleDown => "Scale down",
            Self::Stash => "Stash",
            Self::StashLeft => "Stash left",
            Self::StashRight => "Stash right",
            Self::StashUp => "Stash up",
            Self::StashDown => "Stash down",
            Self::Unstash => "Unstash",
            Self::Custom(_) => "Custom frame",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        parse_action(value)
    }

    pub fn frame(self, bounds: Rect, current: Rect, padding: i32) -> Rect {
        self.frame_with_increment(bounds, current, padding, 20)
    }

    pub fn frame_with_increment(
        self,
        bounds: Rect,
        current: Rect,
        padding: i32,
        step: i32,
    ) -> Rect {
        let w = bounds.width();
        let h = bounds.height();
        let mid_x = bounds.left + w / 2;
        let mid_y = bounds.top + h / 2;
        let step = step.max(1);
        let raw = match self {
            Self::NoAction => current,
            Self::LeftHalf => bounds.columns(2, 0, 1),
            Self::RightHalf => bounds.columns(2, 1, 1),
            Self::TopHalf => bounds.rows(2, 0, 1),
            Self::BottomHalf => bounds.rows(2, 1, 1),
            Self::TopLeft => Rect {
                right: mid_x,
                bottom: mid_y,
                ..bounds
            },
            Self::TopRight => Rect {
                left: mid_x,
                bottom: mid_y,
                ..bounds
            },
            Self::BottomLeft => Rect {
                right: mid_x,
                top: mid_y,
                ..bounds
            },
            Self::BottomRight => Rect {
                left: mid_x,
                top: mid_y,
                ..bounds
            },
            Self::LeftThird => bounds.columns(3, 0, 1),
            Self::CenterThird => bounds.columns(3, 1, 1),
            Self::RightThird => bounds.columns(3, 2, 1),
            Self::LeftTwoThirds => bounds.columns(3, 0, 2),
            Self::RightTwoThirds => bounds.columns(3, 1, 2),
            Self::TopThird => bounds.rows(3, 0, 1),
            Self::MiddleThird => bounds.rows(3, 1, 1),
            Self::BottomThird => bounds.rows(3, 2, 1),
            Self::TopTwoThirds => bounds.rows(3, 0, 2),
            Self::BottomTwoThirds => bounds.rows(3, 1, 2),
            Self::FirstFourth => bounds.columns(4, 0, 1),
            Self::SecondFourth => bounds.columns(4, 1, 1),
            Self::ThirdFourth => bounds.columns(4, 2, 1),
            Self::FourthFourth => bounds.columns(4, 3, 1),
            Self::LeftThreeFourths => bounds.columns(4, 0, 3),
            Self::RightThreeFourths => bounds.columns(4, 1, 3),
            Self::HorizontalCenterHalf => bounds.columns(4, 1, 2),
            Self::VerticalCenterHalf => bounds.rows(4, 1, 2),
            Self::Center => {
                let width = current.width().min(w);
                let height = current.height().min(h);
                let (cx, cy) = bounds.center();
                Rect {
                    left: cx - width / 2,
                    top: cy - height / 2,
                    right: cx - width / 2 + width,
                    bottom: cy - height / 2 + height,
                }
            }
            Self::AlmostMaximize => Rect {
                left: bounds.left + w / 20,
                top: bounds.top + h / 20,
                right: bounds.right - w / 20,
                bottom: bounds.bottom - h / 20,
            },
            Self::Maximize | Self::Fullscreen | Self::FillAvailableSpace => bounds,
            Self::MacOSCenter => {
                let width = current.width().min(w);
                let height = current.height().min(h);
                let (cx, cy) = bounds.center();
                Rect {
                    left: cx - width / 2,
                    top: cy - height / 2,
                    right: cx - width / 2 + width,
                    bottom: cy - height / 2 + height,
                }
            }
            Self::MaximizeWidth => Rect {
                left: bounds.left,
                right: bounds.right,
                ..current
            },
            Self::MaximizeHeight => Rect {
                top: bounds.top,
                bottom: bounds.bottom,
                ..current
            },
            Self::MoveLeft => Rect {
                left: current.left.saturating_sub(step),
                right: current.right.saturating_sub(step),
                ..current
            },
            Self::MoveRight => Rect {
                left: current.left.saturating_add(step),
                right: current.right.saturating_add(step),
                ..current
            },
            Self::MoveUp => Rect {
                top: current.top.saturating_sub(step),
                bottom: current.bottom.saturating_sub(step),
                ..current
            },
            Self::MoveDown => Rect {
                top: current.top.saturating_add(step),
                bottom: current.bottom.saturating_add(step),
                ..current
            },
            Self::GrowLeft => Rect {
                left: current.left.saturating_sub(step),
                ..current
            },
            Self::GrowRight => Rect {
                right: current.right.saturating_add(step),
                ..current
            },
            Self::GrowTop => Rect {
                top: current.top.saturating_sub(step),
                ..current
            },
            Self::GrowBottom => Rect {
                bottom: current.bottom.saturating_add(step),
                ..current
            },
            Self::ShrinkLeft => Rect {
                left: if current.width() <= 100 {
                    current.left
                } else {
                    current
                        .left
                        .saturating_add(step)
                        .min(current.right.saturating_sub(100))
                },
                ..current
            },
            Self::ShrinkRight => Rect {
                right: if current.width() <= 100 {
                    current.right
                } else {
                    current
                        .right
                        .saturating_sub(step)
                        .max(current.left.saturating_add(100))
                },
                ..current
            },
            Self::ShrinkTop => Rect {
                top: if current.height() <= 80 {
                    current.top
                } else {
                    current
                        .top
                        .saturating_add(step)
                        .min(current.bottom.saturating_sub(80))
                },
                ..current
            },
            Self::ShrinkBottom => Rect {
                bottom: if current.height() <= 80 {
                    current.bottom
                } else {
                    current
                        .bottom
                        .saturating_sub(step)
                        .max(current.top.saturating_add(80))
                },
                ..current
            },
            Self::GrowHorizontal => Rect {
                left: current.left.saturating_sub(step),
                right: current.right.saturating_add(step),
                ..current
            },
            Self::GrowVertical => Rect {
                top: current.top.saturating_sub(step),
                bottom: current.bottom.saturating_add(step),
                ..current
            },
            Self::ShrinkHorizontal => {
                let amount = step.min((current.width() - 100).max(0) / 2);
                Rect {
                    left: current.left.saturating_add(amount),
                    right: current.right.saturating_sub(amount),
                    ..current
                }
            }
            Self::ShrinkVertical => {
                let amount = step.min((current.height() - 100).max(0) / 2);
                Rect {
                    top: current.top.saturating_add(amount),
                    bottom: current.bottom.saturating_sub(amount),
                    ..current
                }
            }
            Self::Larger => Rect {
                left: current.left.saturating_sub(step),
                top: current.top.saturating_sub(step),
                right: current.right.saturating_add(step),
                bottom: current.bottom.saturating_add(step),
            },
            Self::Smaller => {
                let dx = step.min((current.width() - 100).max(0) / 2);
                let dy = step.min((current.height() - 100).max(0) / 2);
                Rect {
                    left: current.left.saturating_add(dx),
                    top: current.top.saturating_add(dy),
                    right: current.right.saturating_sub(dx),
                    bottom: current.bottom.saturating_sub(dy),
                }
            }
            Self::ScaleUp | Self::ScaleDown => {
                let width = f64::from(current.width().max(1));
                let height = f64::from(current.height().max(1));
                let delta = if self == Self::ScaleUp {
                    f64::from(step) * 2.0
                } else {
                    -f64::from(step) * 2.0
                };
                let scale = ((width + delta) / width).min((height + delta) / height);
                let minimum = (100.0 / width).max(100.0 / height).min(1.0);
                let scale = scale.max(minimum);
                let new_width = (width * scale).round().clamp(1.0, f64::from(i32::MAX)) as i32;
                let new_height = (height * scale).round().clamp(1.0, f64::from(i32::MAX)) as i32;
                let (cx, cy) = current.center();
                let left = cx.saturating_sub(new_width / 2);
                let top = cy.saturating_sub(new_height / 2);
                Rect {
                    left,
                    top,
                    right: left.saturating_add(new_width),
                    bottom: top.saturating_add(new_height),
                }
            }
            _ => current,
        };
        if matches!(
            self,
            Self::NoAction
                | Self::Restore
                | Self::Hide
                | Self::MinimizeOthers
                | Self::Undo
                | Self::InitialFrame
                | Self::NextMonitor
                | Self::PreviousMonitor
                | Self::MoveToMonitorLeft
                | Self::MoveToMonitorRight
                | Self::MoveToMonitorUp
                | Self::MoveToMonitorDown
                | Self::FocusLeft
                | Self::FocusRight
                | Self::FocusUp
                | Self::FocusDown
                | Self::FocusNextInStack
                | Self::Center
                | Self::MacOSCenter
                | Self::Stash
                | Self::StashLeft
                | Self::StashRight
                | Self::StashUp
                | Self::StashDown
                | Self::Unstash
                | Self::Custom(_)
                | Self::Minimize
                | Self::MoveLeft
                | Self::MoveRight
                | Self::MoveUp
                | Self::MoveDown
                | Self::GrowLeft
                | Self::GrowRight
                | Self::GrowTop
                | Self::GrowBottom
                | Self::ShrinkLeft
                | Self::ShrinkRight
                | Self::ShrinkTop
                | Self::ShrinkBottom
                | Self::GrowHorizontal
                | Self::GrowVertical
                | Self::ShrinkHorizontal
                | Self::ShrinkVertical
                | Self::Larger
                | Self::Smaller
                | Self::ScaleUp
                | Self::ScaleDown
        ) {
            raw
        } else {
            raw.inset(padding)
        }
    }
}

pub fn parse_action(value: &str) -> Result<Action, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    if let Some(index) = normalized
        .strip_prefix("custom_")
        .or_else(|| normalized.strip_prefix("custom:"))
    {
        return index
            .parse::<u16>()
            .map(Action::Custom)
            .map_err(|_| format!("invalid custom frame index '{index}'"));
    }
    let action = match normalized.as_str() {
        "no_action" | "noaction" => Action::NoAction,
        "mac_os_center" | "macoscenter" => Action::MacOSCenter,
        "top_left_quarter" => Action::TopLeft,
        "top_right_quarter" => Action::TopRight,
        "bottom_left_quarter" => Action::BottomLeft,
        "bottom_right_quarter" => Action::BottomRight,
        "horizontal_center_third" => Action::CenterThird,
        "vertical_center_third" => Action::MiddleThird,
        "left_screen" => Action::MoveToMonitorLeft,
        "right_screen" => Action::MoveToMonitorRight,
        "top_screen" => Action::MoveToMonitorUp,
        "bottom_screen" => Action::MoveToMonitorDown,
        "next_screen" => Action::NextMonitor,
        "previous_screen" => Action::PreviousMonitor,
        "move_to_monitor_left" => Action::MoveToMonitorLeft,
        "move_to_monitor_right" => Action::MoveToMonitorRight,
        "move_to_monitor_up" => Action::MoveToMonitorUp,
        "move_to_monitor_down" => Action::MoveToMonitorDown,
        "fill_available_space" => Action::FillAvailableSpace,
        "fullscreen" => Action::Fullscreen,
        "minimize_others" => Action::MinimizeOthers,
        "focus_next_in_stack" => Action::FocusNextInStack,
        "larger" => Action::Larger,
        "smaller" => Action::Smaller,
        "scale_up" => Action::ScaleUp,
        "scale_down" => Action::ScaleDown,
        "shrink_horizontal" => Action::ShrinkHorizontal,
        "shrink_vertical" => Action::ShrinkVertical,
        "grow_horizontal" => Action::GrowHorizontal,
        "grow_vertical" => Action::GrowVertical,
        "initial_frame" => Action::InitialFrame,
        "move_left" => Action::MoveLeft,
        "move_right" => Action::MoveRight,
        "move_up" => Action::MoveUp,
        "move_down" => Action::MoveDown,
        "grow_left" => Action::GrowLeft,
        "grow_right" => Action::GrowRight,
        "grow_top" => Action::GrowTop,
        "grow_bottom" => Action::GrowBottom,
        "shrink_left" => Action::ShrinkLeft,
        "shrink_right" => Action::ShrinkRight,
        "shrink_top" => Action::ShrinkTop,
        "shrink_bottom" => Action::ShrinkBottom,
        _ => serde_json::from_value(serde_json::Value::String(normalized.clone()))
            .map_err(|_| format!("unknown action '{value}'"))?,
    };
    Ok(action)
}

pub fn radial_action_from(
    origin: (i32, i32),
    cursor: (i32, i32),
    actions: &[Action; 8],
) -> Option<Action> {
    let dx = f64::from(cursor.0) - f64::from(origin.0);
    let dy = f64::from(cursor.1) - f64::from(origin.1);
    if dx * dx + dy * dy < 30.0 * 30.0 {
        return None;
    }
    let angle = dy.atan2(dx);
    let sector = ((angle / std::f64::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
    Some(actions[sector as usize])
}

pub fn radial_action(origin: (i32, i32), cursor: (i32, i32)) -> Option<Action> {
    radial_action_from(origin, cursor, &Action::RADIAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    const B: Rect = Rect {
        left: -100,
        top: 20,
        right: 901,
        bottom: 821,
    };

    #[test]
    fn halves_cover_odd_width_without_gap() {
        let left = Action::LeftHalf.frame(B, B, 0);
        let right = Action::RightHalf.frame(B, B, 0);
        assert_eq!(left.right, right.left);
        assert_eq!(left.width() + right.width(), B.width());
    }

    #[test]
    fn edge_padding_preserves_offset_and_never_inverts_small_work_areas() {
        let padded = B.inset_edges(10, 20, 30, 40);
        assert_eq!(
            padded,
            Rect {
                left: -60,
                top: 30,
                right: 881,
                bottom: 791
            }
        );
        let narrow = Rect {
            left: -2,
            top: 5,
            right: 0,
            bottom: 6,
        }
        .inset_edges(50, 50, 50, 50);
        assert_eq!(narrow.width(), 1);
        assert_eq!(narrow.height(), 1);
    }

    #[test]
    fn quarters_stay_within_offset_monitor() {
        let r = Action::BottomRight.frame(B, B, 10);
        assert!(r.left >= B.left && r.top >= B.top);
        assert!(r.right <= B.right && r.bottom <= B.bottom);
    }

    #[test]
    fn radial_has_dead_zone_and_eight_directions() {
        let o = (100, 100);
        assert_eq!(radial_action(o, o), None);
        assert_eq!(radial_action(o, (200, 100)), Some(Action::RightHalf));
        assert_eq!(radial_action(o, (100, 0)), Some(Action::TopHalf));
        assert_eq!(radial_action(o, (0, 100)), Some(Action::LeftHalf));
    }

    #[test]
    fn thirds_and_fourths_cover_odd_monitor_width() {
        let thirds = [Action::LeftThird, Action::CenterThird, Action::RightThird]
            .map(|action| action.frame(B, B, 0));
        assert_eq!(thirds[0].left, B.left);
        assert_eq!(thirds[0].right, thirds[1].left);
        assert_eq!(thirds[1].right, thirds[2].left);
        assert_eq!(thirds[2].right, B.right);
        let fourths = [
            Action::FirstFourth,
            Action::SecondFourth,
            Action::ThirdFourth,
            Action::FourthFourth,
        ]
        .map(|action| action.frame(B, B, 0));
        for pair in fourths.windows(2) {
            assert_eq!(pair[0].right, pair[1].left);
        }
    }

    #[test]
    fn relative_move_preserves_size() {
        let current = Rect {
            left: 10,
            top: 10,
            right: 410,
            bottom: 310,
        };
        let moved = Action::MoveRight.frame(B, current, 0);
        assert_eq!(moved.width(), current.width());
        assert_eq!(moved.left, current.left + 20);
    }

    #[test]
    fn large_shrink_steps_keep_a_valid_frame_and_scaling_preserves_ratio() {
        let current = Rect {
            left: -50,
            top: 20,
            right: 350,
            bottom: 220,
        };
        for action in [
            Action::Smaller,
            Action::ScaleDown,
            Action::ShrinkHorizontal,
            Action::ShrinkVertical,
        ] {
            let result = action.frame_with_increment(B, current, 10, 2000);
            assert!(
                result.width() >= 100 && result.height() >= 100,
                "{action:?}: {result:?}"
            );
        }
        let scaled = Action::ScaleUp.frame_with_increment(B, current, 10, 20);
        assert_eq!(scaled.width(), 2 * scaled.height());
        assert_eq!(scaled.center(), current.center());
    }
}
