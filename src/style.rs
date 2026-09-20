//! Modern glassmorphic theme styling, color tokens, and layout helpers for RixLauncher.
//! Matching Ronner's native frosted-glass design language across Linux and Windows.

#![allow(dead_code)]

use iced::widget::{button, container, pick_list, progress_bar, text_input};
use iced::{Background, Border, Color, Shadow, Vector};

// Brand accent colors — emerald green is the primary accent, matching Ronner
pub const ACCENT_GREEN: Color = Color::from_rgb(0.14, 0.78, 0.42);
pub const ACCENT_GREEN_HOVER: Color = Color::from_rgb(0.18, 0.88, 0.48);
pub const ACCENT_GREEN_GLOW: Color = Color::from_rgba(0.14, 0.78, 0.42, 0.35);

pub const ACCENT_ORANGE: Color = Color::from_rgb(0.98, 0.48, 0.18);
pub const ACCENT_BLUE: Color = Color::from_rgb(0.24, 0.55, 0.96);
pub const ACCENT_RED: Color = Color::from_rgb(0.95, 0.32, 0.32);

// Glass panel backgrounds and borders
pub const GLASS_BG: Color = Color::from_rgba(0.06, 0.07, 0.10, 0.65);
pub const GLASS_CARD_BG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.035);
pub const GLASS_CARD_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const GLASS_EDGE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.10);

// Typography extension trait
pub trait TextBoldExt {
    fn bold(self) -> Self;
}

impl TextBoldExt for iced::widget::Text<'_> {
    fn bold(self) -> Self {
        self.font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::default()
        })
    }
}

// Spacing helpers
pub fn horizontal_space(width: impl Into<iced::Length>) -> iced::widget::Space {
    iced::widget::Space::new().width(width)
}

pub fn vertical_space(height: impl Into<iced::Length>) -> iced::widget::Space {
    iced::widget::Space::new().height(height)
}

/// Root window glass styling — translucent to let compositor blur shine through.
pub fn root_window_style(corner_radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.02, 0.02, 0.03, 0.01))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            width: 1.0,
            radius: corner_radius.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Main outer card / surface container.
pub fn glass_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.05, 0.06, 0.08, 0.40))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.16),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Frosted card panel for content sections.
pub fn card_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(GLASS_CARD_BG)),
        border: Border {
            color: GLASS_CARD_BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.22),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Subtle panel for secondary grouping and inset items.
pub fn subtle_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.025))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Inset panel for terminal/code-like elements or nested lists.
pub fn inset_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.28))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Hero launch / status card — emerald for ready, ruby for error.
pub fn status_hero_style(ready: bool) -> container::Style {
    let (bg, border, glow) = if ready {
        (
            Color::from_rgba(0.04, 0.16, 0.09, 0.55),
            Color::from_rgba(0.14, 0.78, 0.42, 0.35),
            Color::from_rgba(0.14, 0.78, 0.42, 0.12),
        )
    } else {
        (
            Color::from_rgba(0.20, 0.05, 0.05, 0.55),
            Color::from_rgba(0.95, 0.32, 0.32, 0.35),
            Color::from_rgba(0.95, 0.32, 0.32, 0.12),
        )
    };

    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: glow,
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Hero panel style for top banners.
pub fn hero_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.05, 0.07, 0.10, 0.60))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 20.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Dialog panel style for modals and confirmation popups.
pub fn dialog_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.09, 0.10, 0.14))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Toast notification style matching Ronner (1=success, 2=error, 3=info).
pub fn toast_panel_style(kind: u8) -> container::Style {
    let (bg, border) = match kind {
        1 => (
            Color::from_rgba(0.05, 0.18, 0.09, 0.90),
            Color::from_rgba(0.14, 0.78, 0.42, 0.40),
        ),
        2 => (
            Color::from_rgba(0.22, 0.06, 0.06, 0.90),
            Color::from_rgba(0.95, 0.32, 0.32, 0.40),
        ),
        _ => (
            Color::from_rgba(0.06, 0.12, 0.22, 0.90),
            Color::from_rgba(0.24, 0.55, 0.96, 0.40),
        ),
    };

    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Accent-outlined panel.
pub fn accent_panel_style(r: f32, g: f32, b: f32, alpha: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            r * 0.15,
            g * 0.15,
            b * 0.15,
            0.55,
        ))),
        border: Border {
            color: Color::from_rgba(r, g, b, alpha),
            width: 1.0,
            radius: 10.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

/// Coloured status banner pill.
pub fn status_panel_style(r: f32, g: f32, b: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            r * 0.10,
            g * 0.10,
            b * 0.10,
            0.60,
        ))),
        border: Border {
            color: Color::from_rgba(r, g, b, 0.40),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn pill_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn brand_mark_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.14, 0.78, 0.42, 0.20))),
        border: Border {
            color: Color::from_rgba(0.14, 0.78, 0.42, 0.40),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: ACCENT_GREEN_GLOW,
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn avatar_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.14, 0.78, 0.42, 0.22))),
        border: Border {
            color: Color::from_rgba(0.14, 0.78, 0.42, 0.45),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn avatar_style_for_name(name: &str) -> container::Style {
    let hash: u32 = name.bytes().fold(0, |acc, b| acc.wrapping_add(b as u32));
    let colors = [
        Color::from_rgb(0.20, 0.60, 0.85),
        Color::from_rgb(0.85, 0.45, 0.20),
        Color::from_rgb(0.40, 0.75, 0.30),
        Color::from_rgb(0.70, 0.30, 0.80),
        Color::from_rgb(0.85, 0.70, 0.20),
    ];
    let base = colors[(hash as usize) % colors.len()];

    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            base.r, base.g, base.b, 0.25,
        ))),
        border: Border {
            color: Color::from_rgba(base.r, base.g, base.b, 0.50),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn progress_track_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn sidebar_container_style(corner_radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22))),
        border: Border {
            radius: iced::border::Radius::new(0.0).bottom_left(corner_radius),
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
        },
        ..container::Style::default()
    }
}

pub fn sidebar_nav_container_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            ..Default::default()
        },
        ..container::Style::default()
    }
}

// ---------------------------------------------------------------------------
// Button Styles
// ---------------------------------------------------------------------------

/// Primary action / Play button — emerald green with glow effect.
pub fn play_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT_GREEN_HOVER)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.18, 0.88, 0.48, 0.50),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 14.0,
            },
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.12, 0.68, 0.36))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(ACCENT_GREEN)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: ACCENT_GREEN_GLOW,
                offset: Vector::new(0.0, 2.0),
                blur_radius: 10.0,
            },
            snap: false,
        },
    }
}

/// Secondary action button with frosted surface.
pub fn action_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.14))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.20))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

/// Subtle outline button with transparent background.
pub fn outline_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.30),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

/// Navigation tab button — active tab is highlighted with subtle translucent surface and white text.
pub fn nav_tab_button_style(status: button::Status, active: bool) -> button::Style {
    let (bg, text_color, border_color) = if active {
        (
            Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            Color::WHITE,
            Color::from_rgba(1.0, 1.0, 1.0, 0.20),
        )
    } else {
        match status {
            button::Status::Hovered => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                Color::WHITE,
                Color::from_rgba(1.0, 1.0, 1.0, 0.10),
            ),
            button::Status::Pressed => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.09),
                Color::WHITE,
                Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            ),
            _ => (
                Color::TRANSPARENT,
                Color::from_rgba(1.0, 1.0, 1.0, 0.60),
                Color::TRANSPARENT,
            ),
        }
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            radius: 8.0.into(),
            color: border_color,
            width: if active { 1.0 } else { 0.0 },
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Window control buttons (minimize, maximize, close).
pub fn window_control_style(status: button::Status, is_close: bool) -> button::Style {
    let (bg, text_color) = match status {
        button::Status::Hovered => {
            if is_close {
                (Color::from_rgb(0.85, 0.20, 0.20), Color::WHITE)
            } else {
                (Color::from_rgba(1.0, 1.0, 1.0, 0.10), Color::WHITE)
            }
        }
        button::Status::Pressed => {
            if is_close {
                (Color::from_rgb(0.70, 0.15, 0.15), Color::WHITE)
            } else {
                (Color::from_rgba(1.0, 1.0, 1.0, 0.15), Color::WHITE)
            }
        }
        _ => (Color::TRANSPARENT, Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Instance and account selector items.
pub fn instance_button_style(status: button::Status, active: bool) -> button::Style {
    let (bg, border_color) = if active {
        (
            Color::from_rgba(0.14, 0.78, 0.42, 0.15),
            Color::from_rgba(0.14, 0.78, 0.42, 0.45),
        )
    } else {
        match status {
            button::Status::Hovered => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                Color::from_rgba(1.0, 1.0, 1.0, 0.16),
            ),
            button::Status::Pressed => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.09),
                Color::from_rgba(1.0, 1.0, 1.0, 0.22),
            ),
            _ => (GLASS_CARD_BG, GLASS_CARD_BORDER),
        }
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 10.0.into(),
            color: border_color,
            width: 1.0,
        },
        shadow: if active {
            Shadow {
                color: Color::from_rgba(0.14, 0.78, 0.42, 0.15),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            }
        } else {
            Shadow::default()
        },
        snap: false,
    }
}

/// Sidebar navigation button.
pub fn sidebar_nav_button_style(status: button::Status, active: bool) -> button::Style {
    nav_tab_button_style(status, active)
}

pub fn sidebar_button_style(status: button::Status, active: bool) -> button::Style {
    nav_tab_button_style(status, active)
}

pub fn clone_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.24, 0.55, 0.96, 0.35))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(0.24, 0.55, 0.96, 0.60),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.24, 0.55, 0.96, 0.20))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(0.24, 0.55, 0.96, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

pub fn toggle_active_style(status: button::Status) -> button::Style {
    nav_tab_button_style(status, true)
}

pub fn toggle_inactive_style(status: button::Status) -> button::Style {
    nav_tab_button_style(status, false)
}

// ---------------------------------------------------------------------------
// Inputs & Overlays
// ---------------------------------------------------------------------------

pub fn text_input_style(status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_GREEN,
        text_input::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.25),
        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
    };

    text_input::Style {
        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
        placeholder: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
        value: Color::WHITE,
        selection: Color::from_rgba(0.14, 0.78, 0.42, 0.30),
    }
}

pub fn pick_list_style(status: pick_list::Status) -> pick_list::Style {
    let (border_color, bg) = match status {
        pick_list::Status::Hovered => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        ),
        pick_list::Status::Opened { .. } => (ACCENT_GREEN, Color::from_rgba(1.0, 1.0, 1.0, 0.10)),
        _ => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            Color::from_rgba(1.0, 1.0, 1.0, 0.04),
        ),
    };

    pick_list::Style {
        text_color: Color::WHITE,
        placeholder_color: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
        handle_color: Color::from_rgba(1.0, 1.0, 1.0, 0.60),
        background: Background::Color(bg),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
    }
}

pub fn custom_menu_style(_theme: &iced::Theme) -> iced::widget::overlay::menu::Style {
    iced::widget::overlay::menu::Style {
        text_color: Color::WHITE,
        background: Background::Color(Color::from_rgb(0.08, 0.09, 0.13)),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            width: 1.0,
            radius: 8.0.into(),
        },
        selected_text_color: Color::WHITE,
        selected_background: Background::Color(Color::from_rgba(0.14, 0.78, 0.42, 0.25)),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
    }
}

/// Refined subtle frosted scrollbar matching the glass theme.
pub fn custom_scrollbar_style() -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{Rail, Scroller, Style};
    Style {
        container: container::Style::default(),
        vertical_rail: Rail {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            scroller: Scroller {
                background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.18)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
            },
        },
        horizontal_rail: Rail {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            scroller: Scroller {
                background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.18)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
            },
        },
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: Background::Color(Color::from_rgba(0.08, 0.09, 0.13, 0.90)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            icon: Color::from_rgba(1.0, 1.0, 1.0, 0.60),
        },
    }
}

pub fn progress_bar_style() -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
        bar: Background::Color(ACCENT_GREEN),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
    }
}

pub fn loader_color(loader: &str) -> (f32, f32, f32) {
    match loader.to_lowercase().as_str() {
        "fabric" => (0.95, 0.85, 0.65),
        "quilt" => (0.75, 0.55, 0.95),
        "forge" | "neoforge" => (0.95, 0.55, 0.35),
        _ => (0.80, 0.80, 0.85),
    }
}

pub fn loader_badge_style(loader: &str) -> container::Style {
    let (r, g, b) = loader_color(loader);
    container::Style {
        background: Some(Background::Color(Color::from_rgba(r, g, b, 0.12))),
        border: Border {
            color: Color::from_rgba(r, g, b, 0.35),
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: Some(Color::from_rgb(r, g, b)),
        ..container::Style::default()
    }
}

pub fn status_dot_style(active: bool) -> container::Style {
    let color = if active {
        ACCENT_GREEN
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.30)
    };
    container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..container::Style::default()
    }
}

pub fn loading_skeleton_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..container::Style::default()
    }
}
