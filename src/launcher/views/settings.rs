//! Launcher settings view.
//! Clean, responsive, glassmorphic configuration layout matching Ronner.

use std::path::Path;

use crate::icons::{
    SVG_CHECK, SVG_FOLDER, SVG_ROCKET, SVG_SCROLL, svg_handle,
};
use crate::launcher::RixLauncher;
use crate::launcher::constants;
use crate::launcher::types::{Message, NavLayout};
use crate::style::{
    ACCENT_GREEN, TextBoldExt, action_button_style, card_style,
    custom_menu_style, custom_scrollbar_style, glass_panel_style, horizontal_space,
    nav_tab_button_style, outline_button_style, pick_list_style, play_button_style,
    subtle_panel_style, text_input_style,
};
use iced::widget::{
    Button, column, container, pick_list, progress_bar, row, scrollable, slider, svg, text,
    text_input,
};
use iced::alignment::{Horizontal, Vertical};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

pub fn view_settings(launcher: &RixLauncher) -> Element<'_, Message> {
    let is_narrow = launcher.window_width < 850;

    // Header section
    let header = column![
        text("Settings").size(22).bold().color(Color::WHITE),
        text("Configure launcher preferences, runtime options, and game storage")
            .size(12)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
    ]
    .spacing(4);

    // 1. Navigation Layout Card
    let is_topbar = launcher.nav_layout == NavLayout::TopBar;
    let is_sidebar = launcher.nav_layout == NavLayout::Sidebar;
    let nav_card = container(
        column![
            column![
                text("Navigation Layout")
                    .size(14)
                    .bold()
                    .color(Color::WHITE),
                text("Choose how navigation and window controls are displayed.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .spacing(3),
            row![
                Button::new(
                    container(text("Top bar").size(12).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::NavLayoutChanged(NavLayout::TopBar))
                .style(move |_, status| nav_tab_button_style(status, is_topbar))
                .width(Length::Fixed(130.0))
                .height(Length::Fixed(36.0))
                .padding(0),
                Button::new(
                    container(text("Sidebar").size(12).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::NavLayoutChanged(NavLayout::Sidebar))
                .style(move |_, status| nav_tab_button_style(status, is_sidebar))
                .width(Length::Fixed(130.0))
                .height(Length::Fixed(36.0))
                .padding(0),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(14),
    )
    .padding(18)
    .width(Length::Fill)
    .style(|_| card_style());

    // 2. Storage & Game Directory Card
    let root_path = Path::new(&launcher.game_root);
    let root_exists = root_path.is_dir();
    let root_status_color = if root_exists {
        ACCENT_GREEN
    } else {
        Color::from_rgb(1.0, 0.65, 0.25)
    };
    let root_status_label = if root_exists {
        "Directory exists"
    } else {
        "Directory will be created"
    };

    let make_dir_btn = |icon: &'static str, label: &'static str, is_primary: bool, msg: Message| {
        Button::new(
            container(
                row![
                    svg(svg_handle(icon))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style { color: Some(Color::WHITE) }),
                    text(label).size(12).bold(),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
        )
        .on_press(msg)
        .style(move |_, status| {
            if is_primary {
                action_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .height(Length::Fixed(36.0))
        .padding(Padding::new(0.0).left(16.0).right(16.0))
    };

    let dir_actions: Element<'_, Message> = if is_narrow {
        column![
            make_dir_btn(SVG_FOLDER, "Browse Folder", true, Message::BrowseGameRoot)
                .width(Length::Fill),
            make_dir_btn(SVG_FOLDER, "Open in File Manager", false, Message::OpenDataFolder)
                .width(Length::Fill),
            make_dir_btn(SVG_SCROLL, "Open Logs Folder", false, Message::OpenLogsFolder)
                .width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill)
        .into()
    } else {
        row![
            make_dir_btn(SVG_FOLDER, "Browse Folder", true, Message::BrowseGameRoot),
            make_dir_btn(SVG_FOLDER, "Open in File Manager", false, Message::OpenDataFolder),
            make_dir_btn(SVG_SCROLL, "Open Logs Folder", false, Message::OpenLogsFolder),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let storage_card = container(
        column![
            column![
                text("Storage & Data Directory")
                    .size(14)
                    .bold()
                    .color(Color::WHITE),
                text("Instances, downloaded assets, configurations, and logs are kept in this location.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .spacing(3),
            container(
                row![
                    text(&launcher.game_root)
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.85))
                        .width(Length::Fill),
                    horizontal_space(10),
                    container(
                        text(root_status_label)
                            .size(10)
                            .bold()
                            .color(root_status_color),
                    )
                    .padding(Padding::new(3.0).left(8.0).right(8.0))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(Color::from_rgba(
                            root_status_color.r,
                            root_status_color.g,
                            root_status_color.b,
                            0.12,
                        ))),
                        border: Border {
                            color: Color::from_rgba(
                                root_status_color.r,
                                root_status_color.g,
                                root_status_color.b,
                                0.28,
                            ),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }),
                ]
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(12)
            .style(|_| subtle_panel_style()),
            dir_actions,
        ]
        .spacing(14),
    )
    .padding(18)
    .width(Length::Fill)
    .style(|_| card_style());

    // 3. Java Runtime Card
    let java_options = launcher.java_labels();
    let selected_java = launcher.selected_java_label.clone();
    let java_badge = java_hint_badge(launcher);

    let java_manual_row: Element<'_, Message> = if is_narrow {
        column![
            text_input("Executable path or command name", &launcher.java_path)
                .on_input(Message::JavaPathChanged)
                .style(|_, status| text_input_style(status))
                .padding(9)
                .size(12)
                .width(Length::Fill),
            Button::new(
                container(
                    row![
                        svg(svg_handle(SVG_FOLDER))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style { color: Some(Color::WHITE) }),
                        text("Browse...").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            )
            .on_press(Message::BrowseJavaPath)
            .style(|_, status| outline_button_style(status))
            .height(Length::Fixed(36.0))
            .width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill)
        .into()
    } else {
        row![
            text_input("Executable path or command name", &launcher.java_path)
                .on_input(Message::JavaPathChanged)
                .style(|_, status| text_input_style(status))
                .padding(9)
                .size(12)
                .width(Length::Fill),
            Button::new(
                container(
                    row![
                        svg(svg_handle(SVG_FOLDER))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style { color: Some(Color::WHITE) }),
                        text("Browse...").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            )
            .on_press(Message::BrowseJavaPath)
            .style(|_, status| outline_button_style(status))
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(36.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let java_card = container(
        column![
            row![
                column![
                    text("Java Runtime")
                        .size(14)
                        .bold()
                        .color(Color::WHITE),
                    text("Select a detected JVM installation or specify a manual executable path.")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                ]
                .spacing(3),
                horizontal_space(Length::Fill),
                java_badge,
            ]
            .align_y(Alignment::Center),
            pick_list(java_options, selected_java, Message::JavaSelected)
                .style(|_, status| pick_list_style(status))
                .menu_style(custom_menu_style)
                .width(Length::Fill),
            container(
                column![
                    column![
                        text("Manual Java Override")
                            .size(11)
                            .bold()
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
                        text("Leave set to 'java' to use system PATH, or specify full path to binary.")
                            .size(10)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                    ]
                    .spacing(2),
                    java_manual_row,
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Length::Fill)
            .style(|_| subtle_panel_style()),
        ]
        .spacing(14),
    )
    .padding(18)
    .width(Length::Fill)
    .style(|_| card_style());

    // 4. Memory & Arguments Card
    let memory = launcher
        .memory_gb
        .clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
    let memory_percent = ((memory - constants::MIN_MEMORY_GB)
        / (constants::MAX_MEMORY_GB - constants::MIN_MEMORY_GB))
        .clamp(0.0, 1.0);
    let memory_color = if memory <= 8.0 {
        ACCENT_GREEN
    } else if memory <= 12.0 {
        Color::from_rgb(0.95, 0.80, 0.20)
    } else {
        Color::from_rgb(1.0, 0.40, 0.18)
    };

    let memory_card = container(
        column![
            row![
                column![
                    text("Memory Allocation")
                        .size(14)
                        .bold()
                        .color(Color::WHITE),
                    text("Maximum heap size (-Xmx) allocated to Minecraft instances.")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                ]
                .spacing(3),
                horizontal_space(Length::Fill),
                text(format!("{memory:.0} GB"))
                    .size(18)
                    .bold()
                    .color(memory_color),
            ]
            .align_y(Alignment::Center),
            container(progress_bar(0.0..=1.0, memory_percent).style(move |_| {
                let mut style = crate::style::progress_bar_style();
                style.bar = Background::Color(memory_color);
                style
            }))
            .width(Length::Fill)
            .height(6),
            slider(
                constants::MIN_MEMORY_GB..=constants::MAX_MEMORY_GB,
                memory,
                Message::MemoryChanged,
            )
            .step(1.0_f32),
            row![
                text(format!("{:.0} GB", constants::MIN_MEMORY_GB))
                    .size(10)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.36)),
                horizontal_space(Length::Fill),
                text(format!("{:.0} GB", constants::MAX_MEMORY_GB))
                    .size(10)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.36)),
            ],
            container(
                column![
                    column![
                        text("Advanced JVM & Game Arguments")
                            .size(12)
                            .bold()
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.80)),
                        text("Optional flags passed before the main class or directly to the client.")
                            .size(10)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                    ]
                    .spacing(2),
                    column![
                        text("Extra JVM Flags")
                            .size(11)
                            .bold()
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                        text_input("-XX:+UseZGC -XX:+UnlockExperimentalVMOptions", &launcher.extra_jvm_args)
                            .on_input(Message::ExtraJvmArgsChanged)
                            .style(|_, status| text_input_style(status))
                            .padding(9)
                            .size(12),
                    ]
                    .spacing(4),
                    column![
                        text("Extra Game Arguments")
                            .size(11)
                            .bold()
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                        text_input("--fullscreen --width 1920 --height 1080", &launcher.extra_game_args)
                            .on_input(Message::ExtraGameArgsChanged)
                            .style(|_, status| text_input_style(status))
                            .padding(9)
                            .size(12),
                    ]
                    .spacing(4),
                ]
                .spacing(12),
            )
            .padding(14)
            .width(Length::Fill)
            .style(|_| subtle_panel_style()),
        ]
        .spacing(14),
    )
    .padding(18)
    .width(Length::Fill)
    .style(|_| card_style());

    // 5. System Integration Card
    let system_card = container(
        column![
            column![
                text("System Integration")
                    .size(14)
                    .bold()
                    .color(Color::WHITE),
                text("Install RixLauncher shortcut to your desktop environment and system application menu.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .spacing(3),
            Button::new(
                container(
                    row![
                        svg(svg_handle(SVG_ROCKET))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style { color: Some(Color::WHITE) }),
                        text("Install RixLauncher to System").size(12).bold(),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .height(Length::Fill)
                .align_y(Vertical::Center),
            )
            .on_press(Message::InstallToSystem)
            .style(|_, status| play_button_style(status))
            .height(Length::Fixed(38.0))
            .padding(Padding::new(0.0).left(20.0).right(20.0)),
        ]
        .spacing(14),
    )
    .padding(18)
    .width(Length::Fill)
    .style(|_| card_style());

    // Layout assembly: constrained to max-width 780, centered
    let content = column![
        header,
        nav_card,
        storage_card,
        java_card,
        memory_card,
        system_card,
    ]
    .spacing(16)
    .max_width(780)
    .width(Length::Fill);

    container(
        scrollable(
            container(content)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .padding(Padding::new(24.0).top(16.0).bottom(24.0))
        )
        .height(Length::Fill)
        .style(|_, _| custom_scrollbar_style()),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| glass_panel_style())
    .into()
}

fn java_hint_badge(launcher: &RixLauncher) -> Element<'_, Message> {
    let required = launcher.required_java_major();
    let installed = launcher
        .selected_java_label
        .as_deref()
        .and_then(|label| label.split_whitespace().nth(1))
        .and_then(|major| major.parse::<u32>().ok());

    let (label, color) = match installed {
        Some(major) if major >= required => (
            format!("Java {major} ready (requires {required}+)"),
            ACCENT_GREEN,
        ),
        Some(major) => (
            format!("Java {major} (requires {required}+)"),
            Color::from_rgb(1.0, 0.65, 0.25),
        ),
        None => (
            format!("Java {required}+ required"),
            Color::from_rgba(1.0, 1.0, 1.0, 0.60),
        ),
    };

    container(
        row![
            svg(svg_handle(SVG_CHECK))
                .width(10)
                .height(10)
                .style(move |_, _| svg::Style { color: Some(color) }),
            text(label).size(10).bold().color(color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding(Padding::new(3.0).left(8.0).right(8.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(
            color.r, color.g, color.b, 0.12,
        ))),
        border: Border {
            color: Color::from_rgba(color.r, color.g, color.b, 0.30),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
