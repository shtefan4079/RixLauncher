//! Title bar and topbar / sidebar navigation components.
//! Matching Ronner's native glassmorphic styling and layout metrics.

use crate::icons::{
    SVG_CLOSE, SVG_HOME, SVG_LIBRARY, SVG_LOGO, SVG_MAXIMIZE, SVG_MINIMIZE, SVG_SETTINGS,
    svg_handle,
};
use crate::launcher::RixLauncher;
use crate::launcher::constants;
use crate::launcher::types::{Message, NavLayout, Page};
use crate::style::{
    TextBoldExt, avatar_style_for_name, horizontal_space, nav_tab_button_style,
    window_control_style,
};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, column, container, image, row, stack, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

pub fn is_sidebar(launcher: &RixLauncher) -> bool {
    launcher.nav_layout == NavLayout::Sidebar
}

pub fn view_title_bar(launcher: &RixLauncher) -> Element<'_, Message> {
    let radius = launcher.corner_radius;

    let logo = row![
        svg(svg_handle(SVG_LOGO)).width(22).height(22),
        text(constants::APP_NAME)
            .size(14)
            .bold()
            .color(Color::WHITE),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let min_btn = window_button(SVG_MINIMIZE, Message::MinimizeWindow, false);
    let max_btn = window_button(SVG_MAXIMIZE, Message::MaximizeWindow, false);
    let close_btn = window_button(SVG_CLOSE, Message::CloseWindow, true);

    let background_drag =
        iced::widget::mouse_area(container("").width(Length::Fill).height(Length::Fill))
            .on_press(Message::TitleBarPressed);

    let content = container(
        row![
            logo,
            horizontal_space(Length::Fill),
            row![min_btn, max_btn, close_btn]
                .spacing(4)
                .align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::new(0.0).right(12.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Vertical::Center)
    .padding(Padding::new(0.0).left(14.0).right(6.0));

    container(stack![background_drag, content])
        .width(Length::Fill)
        .height(44)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.20))),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: iced::border::Radius::new(0.0)
                    .top_left(radius)
                    .top_right(radius),
            },
            ..container::Style::default()
        })
        .into()
}

/// Top horizontal navigation bar.
pub fn view_navigation(launcher: &RixLauncher) -> Element<'_, Message> {
    let nav = row![
        nav_button(launcher, Page::Home, "Home", SVG_HOME),
        nav_button(launcher, Page::Instances, "Instances", SVG_LIBRARY),
        nav_button(launcher, Page::Settings, "Settings", SVG_SETTINGS),
        horizontal_space(Length::Fill),
        account_badge(launcher),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(nav)
        .width(Length::Fill)
        .padding(Padding::new(6.0).bottom(10.0).left(14.0).right(14.0))
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.15))),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// Left vertical sidebar navigation — 80px compact glass bar matching Ronner.
pub fn view_sidebar(launcher: &RixLauncher) -> Element<'_, Message> {
    let radius = launcher.corner_radius;

    let nav = column![
        sidebar_nav_button(launcher, Page::Home, "Home", SVG_HOME),
        sidebar_nav_button(launcher, Page::Instances, "Instances", SVG_LIBRARY),
        sidebar_nav_button(launcher, Page::Settings, "Settings", SVG_SETTINGS),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_x(Horizontal::Center);

    container(
        column![
            nav,
            iced::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fill),
            account_badge_vertical(launcher),
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .width(80)
    .height(Length::Fill)
    .padding(Padding::new(8.0).top(10.0).bottom(12.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: iced::border::Radius::new(0.0).bottom_left(radius),
        },
        ..Default::default()
    })
    .into()
}

fn window_button(
    icon: &'static str,
    message: Message,
    is_close: bool,
) -> Element<'static, Message> {
    Button::new(
        svg(svg_handle(icon))
            .width(12)
            .height(12)
            .style(|_, _| svg::Style {
                color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            }),
    )
    .on_press(message)
    .style(move |_, status| window_control_style(status, is_close))
    .padding(8)
    .into()
}

fn nav_button<'a>(
    launcher: &RixLauncher,
    page: Page,
    label: &'a str,
    svg_icon: &'static str,
) -> Element<'a, Message> {
    let is_active = launcher.active_page == page;

    let icon = svg(svg_handle(svg_icon))
        .width(15)
        .height(15)
        .style(move |_, _| svg::Style {
            color: Some(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            }),
        });

    Button::new(
        row![icon, text(label).size(13).bold()]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .on_press(Message::PageChanged(page))
    .style(move |_, status| nav_tab_button_style(status, is_active))
    .padding(Padding::new(6.0).left(14.0).right(14.0))
    .into()
}

fn sidebar_nav_button<'a>(
    launcher: &RixLauncher,
    page: Page,
    label: &'a str,
    svg_icon: &'static str,
) -> Element<'a, Message> {
    let is_active = launcher.active_page == page;

    let icon = svg(svg_handle(svg_icon))
        .width(20)
        .height(20)
        .style(move |_, _| svg::Style {
            color: Some(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            }),
        });

    Button::new(
        column![
            icon,
            text(label).size(10).bold().color(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.50)
            }),
        ]
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .spacing(4),
    )
    .on_press(Message::PageChanged(page))
    .style(move |_, status| nav_tab_button_style(status, is_active))
    .padding(Padding::new(8.0).top(10.0).bottom(10.0))
    .width(Length::Fill)
    .into()
}

fn account_badge<'a>(launcher: &'a RixLauncher) -> Element<'a, Message> {
    let player = launcher.active_player();

    let avatar: Element<'a, Message> = if let Some(handle) = launcher.player_skins.get(player) {
        container(image(handle.clone()).width(24).height(24))
            .clip(true)
            .style(move |_| avatar_style_for_name(player))
            .into()
    } else {
        container(
            text(crate::launcher::utils::player_initials(player))
                .size(11)
                .bold()
                .color(Color::WHITE),
        )
        .width(24)
        .height(24)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_| avatar_style_for_name(player))
        .into()
    };

    let is_ms = launcher
        .active_account_object()
        .is_some_and(|a| a.is_microsoft);

    let type_dot = container(iced::widget::Space::new().width(6).height(6))
        .width(6)
        .height(6)
        .style(move |_| container::Style {
            background: Some(Background::Color(if is_ms {
                Color::from_rgb(0.24, 0.75, 0.45)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.40)
            })),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    Button::new(
        row![
            avatar,
            text(player).size(12).bold().color(Color::WHITE),
            type_dot,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::PageChanged(Page::Home))
    .padding(Padding::new(4.0).left(8.0).right(12.0))
    .style(|_, status| match status {
        iced::widget::button::Status::Hovered => iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                width: 1.0,
            },
            shadow: Default::default(),
            snap: false,
            text_color: Color::WHITE,
        },
        _ => iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
            },
            shadow: Default::default(),
            snap: false,
            text_color: Color::WHITE,
        },
    })
    .into()
}

fn account_badge_vertical<'a>(launcher: &'a RixLauncher) -> Element<'a, Message> {
    let player = launcher.active_player();

    let avatar: Element<'a, Message> = if let Some(handle) = launcher.player_skins.get(player) {
        container(image(handle.clone()).width(28).height(28))
            .clip(true)
            .style(move |_| avatar_style_for_name(player))
            .into()
    } else {
        container(
            text(crate::launcher::utils::player_initials(player))
                .size(12)
                .bold()
                .color(Color::WHITE),
        )
        .width(28)
        .height(28)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_| avatar_style_for_name(player))
        .into()
    };

    Button::new(
        column![
            avatar,
            text(player)
                .size(10)
                .bold()
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
        ]
        .spacing(4)
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .on_press(Message::PageChanged(Page::Home))
    .padding(Padding::new(6.0))
    .style(|_, status| match status {
        iced::widget::button::Status::Hovered => iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                width: 1.0,
            },
            shadow: Default::default(),
            snap: false,
            text_color: Color::WHITE,
        },
        _ => iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                width: 1.0,
            },
            shadow: Default::default(),
            snap: false,
            text_color: Color::WHITE,
        },
    })
    .width(Length::Fill)
    .into()
}
