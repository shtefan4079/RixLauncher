pub mod account_dialogs;
pub mod dialogs;
pub mod home;
pub mod instances;
pub mod navigation;
pub mod settings;

use crate::launcher::RixLauncher;
use crate::launcher::types::{Message, Page};
use crate::style::horizontal_space;
use iced::Element;

pub fn view_main(launcher: &RixLauncher) -> Element<'_, Message> {
    let body = match launcher.active_page {
        Page::Home => home::view_home(launcher),
        Page::Instances => instances::view_instances(launcher),
        Page::Settings => settings::view_settings(launcher),
    };

    let radius = launcher.corner_radius;

    // 1. Background fill — translucent to expose compositor blur
    let bg_panel = iced::widget::container(crate::style::vertical_space(0))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.02, 0.02, 0.03, 0.01,
            ))),
            border: iced::Border {
                radius: radius.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            ..Default::default()
        });

    // 2. Dynamic monochromatic noise overlay for authentic frosted glass feel
    let noise_panel = iced::widget::image(launcher.noise_image.clone())
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .content_fit(iced::ContentFit::None);

    // 3. Layout and controls overlay
    let content_panel = if navigation::is_sidebar(launcher) {
        // Sidebar layout: title bar on top, then [sidebar | body] in a row
        let sidebar = navigation::view_sidebar(launcher);
        let body_area = iced::widget::container(body)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .padding(iced::Padding::new(14.0));

        iced::widget::container(iced::widget::column![
            navigation::view_title_bar(launcher),
            iced::widget::row![sidebar, body_area]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        ])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(1)
    } else {
        // Top-bar layout: title bar → nav bar → body
        iced::widget::container(iced::widget::column![
            navigation::view_title_bar(launcher),
            navigation::view_navigation(launcher),
            iced::widget::container(body)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding(iced::Padding::new(14.0).top(4.0))
        ])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(1) // Keep slightly inset to let the border display nicely
    };

    // 4. Resize handles overlay for borderless window stretching
    let handle_size = 6.0;

    let top_left = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::NorthWest));

    let top_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(iced::Length::Fill)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::North));

    let top_right = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::NorthEast));

    let left_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(iced::Length::Fill),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::West));

    let right_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(iced::Length::Fill),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::East));

    let bottom_left = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::SouthWest));

    let bottom_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(iced::Length::Fill)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::South));

    let bottom_right = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::SouthEast));

    let resize_overlay = iced::widget::column![
        iced::widget::row![top_left, top_edge, top_right].width(iced::Length::Fill),
        iced::widget::row![left_edge, horizontal_space(iced::Length::Fill), right_edge]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        iced::widget::row![bottom_left, bottom_edge, bottom_right].width(iced::Length::Fill),
    ]
    .width(iced::Length::Fill)
    .height(iced::Length::Fill);

    let mut main_view: Element<'_, Message> = iced::widget::container(iced::widget::stack![
        bg_panel,
        noise_panel,
        content_panel,
        resize_overlay
    ])
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    // Clip the whole window content to the rounded shape so nothing bleeds into corners.
    // This container also draws the single authoritative window border.
    .clip(true)
    .style(move |_| iced::widget::container::Style {
        border: iced::Border {
            radius: radius.into(),
            // A single crisp 1px edge — renders on top of the clipped content
            color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            width: 1.0,
        },
        ..Default::default()
    })
    .into();

    if let Some(index) = launcher.active_account_context_menu {
        let modal = account_dialogs::view_account_context_modal(launcher, index);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if let Some(index) = launcher.account_to_delete {
        let modal = account_dialogs::view_account_delete_confirmation_modal(launcher, index);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if let Some(ref filename) = launcher.file_to_delete {
        let modal = dialogs::view_file_delete_confirmation_modal(launcher, filename);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if let Some(instance_id) = launcher.instance_to_delete {
        let modal = dialogs::view_instance_delete_confirmation_modal(launcher, instance_id);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if launcher.show_create_dialog {
        let modal = dialogs::view_create_dialog(launcher);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if launcher.show_import_dialog {
        let modal = dialogs::view_import_dialog(launcher);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if launcher.show_clone_dialog.is_some() {
        let modal = dialogs::view_clone_dialog(launcher);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    // ── Toast notification overlay ────────────────────────────────────────────
    if let Some((ref msg, kind, expiry_time)) = launcher.toast {
        use crate::style::toast_panel_style;
        // Fade out in last 600ms before expiry (expiry_time is anim_time target)
        let remaining = expiry_time - launcher.anim_time;
        let alpha = if remaining < 600.0 {
            (remaining / 600.0).max(0.0)
        } else {
            1.0
        };

        let (accent_r, accent_g, accent_b) = match kind {
            1 => (0.27f32, 0.78, 0.46),
            2 => (0.90, 0.25, 0.25),
            _ => (0.30, 0.62, 1.00),
        };
        let icon = match kind {
            1 => "✓",
            2 => "✕",
            _ => "ℹ",
        };

        let toast_widget = iced::widget::mouse_area(
            iced::widget::container(
                iced::widget::row![
                    iced::widget::container(
                        iced::widget::text(icon)
                            .size(13)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..iced::Font::default()
                            })
                            .color(iced::Color::from_rgba(accent_r, accent_g, accent_b, alpha))
                    )
                    .width(22)
                    .align_x(iced::alignment::Horizontal::Center),
                    iced::widget::text(msg.as_str())
                        .size(12)
                        .color(iced::Color::from_rgba(1.0, 1.0, 1.0, alpha)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding(iced::Padding::new(10.0).left(14.0).right(16.0))
            .style(move |_| {
                let mut s = toast_panel_style(kind);
                // Apply fade alpha to border and background
                if let Some(iced::Background::Color(ref mut c)) = s.background {
                    c.a *= alpha;
                }
                s.border.color.a *= alpha;
                s
            }),
        )
        .on_press(crate::launcher::types::Message::DismissToast);

        // Anchor bottom-right with some margin
        let toast_overlay = iced::widget::container(iced::widget::column![
            crate::style::vertical_space(iced::Length::Fill),
            iced::widget::row![
                crate::style::horizontal_space(iced::Length::Fill),
                toast_widget,
            ],
        ])
        .padding(iced::Padding::new(16.0).right(20.0).bottom(20.0))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);

        main_view = iced::widget::stack![main_view, toast_overlay].into();
    }

    main_view
}
