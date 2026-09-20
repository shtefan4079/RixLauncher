#![allow(dead_code)]
use crate::launcher::RixLauncher;
use crate::launcher::types::{Message, MinecraftInstance};
use crate::style::{
    TextBoldExt, avatar_style_for_name, glass_panel_style, horizontal_space, instance_button_style,
    loader_badge_style, loader_color, outline_button_style, pick_list_style, subtle_panel_style,
    text_input_style,
};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Button, Column, container, pick_list, row, scrollable, stack, svg, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length};

const SVG_SEARCH: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>"##;

pub const INSTANCE_CARD_SIZE: f32 = 165.0;
pub const INSTANCE_GRID_SPACING: f32 = 14.0;

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts_a: Vec<u32> = a.split('.').filter_map(|p| p.parse::<u32>().ok()).collect();
    let parts_b: Vec<u32> = b.split('.').filter_map(|p| p.parse::<u32>().ok()).collect();

    for i in 0..std::cmp::max(parts_a.len(), parts_b.len()) {
        let val_a = parts_a.get(i).copied().unwrap_or(0);
        let val_b = parts_b.get(i).copied().unwrap_or(0);
        if val_a != val_b {
            return val_a.cmp(&val_b);
        }
    }
    a.cmp(b)
}

pub fn view_instances(launcher: &RixLauncher) -> Element<'_, Message> {
    let search_bar = container(
        row![
            svg(svg::Handle::from_memory(SVG_SEARCH.as_bytes()))
                .width(14)
                .height(14)
                .style(|_, _| svg::Style {
                    color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.4))
                }),
            text_input("Search instances...", &launcher.instance_search)
                .on_input(Message::InstanceSearchChanged)
                .style(|_theme, status| {
                    let mut s = text_input_style(status);
                    s.background = iced::Background::Color(Color::TRANSPARENT);
                    s.border.width = 0.0;
                    s.border.color = Color::TRANSPARENT;
                    s
                })
                .padding(0)
                .size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::new(10.0).left(14.0).right(14.0))
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.18,
        ))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill);

    let sort_options = crate::launcher::types::InstanceSort::ALL.to_vec();
    let sort_picker = pick_list(
        sort_options,
        Some(launcher.instance_sort),
        Message::InstanceSortChanged,
    )
    .style(|_theme, status| pick_list_style(status))
    .menu_style(crate::style::custom_menu_style)
    .width(140);

    let header_row = row![
        search_bar,
        horizontal_space(10),
        text("Sort by:")
            .size(12)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
        sort_picker,
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let mut filtered: Vec<&MinecraftInstance> = launcher
        .instances
        .iter()
        .filter(|inst| {
            launcher.instance_search.is_empty()
                || inst
                    .name
                    .to_lowercase()
                    .contains(&launcher.instance_search.to_lowercase())
        })
        .collect();

    match launcher.instance_sort {
        crate::launcher::types::InstanceSort::Name => {
            filtered.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        crate::launcher::types::InstanceSort::Version => {
            filtered.sort_by(|a, b| {
                let ord = compare_versions(&b.version, &a.version);
                if ord == std::cmp::Ordering::Equal {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                } else {
                    ord
                }
            });
        }
        crate::launcher::types::InstanceSort::Loader => {
            filtered.sort_by(|a, b| {
                let ord = a.loader.to_lowercase().cmp(&b.loader.to_lowercase());
                if ord == std::cmp::Ordering::Equal {
                    compare_versions(&b.version, &a.version)
                } else {
                    ord
                }
            });
        }
    }

    let mut grid_items: Vec<Element<'_, Message>> = Vec::new();
    for instance in &filtered {
        grid_items.push(instance_card(launcher, instance));
    }

    let add_card = Button::new(
        container(
            column![
                container(
                    text("+")
                        .size(24)
                        .bold()
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
                )
                .width(46)
                .height(46)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }),
                text("Add Instance")
                    .size(12)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center),
    )
    .on_press(Message::OpenCreateDialog)
    .padding(10)
    .style(|_theme, status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            _ => Color::from_rgba(1.0, 1.0, 1.0, 0.02),
        };
        let border_color = match status {
            iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.18),
            _ => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 10.0.into(),
            },
            shadow: iced::Shadow::default(),
            ..Default::default()
        }
    })
    .width(Length::Fixed(INSTANCE_CARD_SIZE))
    .height(Length::Fixed(INSTANCE_CARD_SIZE));

    grid_items.push(add_card.into());

    let nav_width = if launcher.nav_layout == crate::launcher::types::NavLayout::Sidebar {
        80.0
    } else {
        0.0
    };
    let available_width = (launcher.window_width as f32 - nav_width - 64.0).max(INSTANCE_CARD_SIZE);
    let columns = ((available_width + INSTANCE_GRID_SPACING) / (INSTANCE_CARD_SIZE + INSTANCE_GRID_SPACING))
        .floor()
        .max(1.0) as usize;

    let mut grid_col = Column::new().spacing(INSTANCE_GRID_SPACING);
    let mut current_row = row![].spacing(INSTANCE_GRID_SPACING).align_y(Alignment::Start);
    let mut item_count = 0;

    for item in grid_items {
        current_row = current_row.push(item);
        item_count += 1;
        if item_count == columns {
            grid_col = grid_col.push(current_row);
            current_row = row![].spacing(INSTANCE_GRID_SPACING).align_y(Alignment::Start);
            item_count = 0;
        }
    }
    if item_count > 0 {
        grid_col = grid_col.push(current_row);
    }

    let content = scrollable(grid_col)
        .height(Length::Fill)
        .style(|_theme, _status| crate::style::custom_scrollbar_style());

    let page_heading = row![
        column![
            text("Instances").size(22).bold().color(Color::WHITE),
            text(format!(
                "{} profiles · choose one to manage or launch",
                launcher.instances.len()
            ))
            .size(12)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.48)),
        ]
        .spacing(4),
        horizontal_space(Length::Fill),
        Button::new(text("+ New instance").size(12).bold())
            .on_press(Message::OpenCreateDialog)
            .style(|_theme, status| crate::style::play_button_style(status))
            .padding(iced::Padding::new(9.0).left(14.0).right(14.0)),
    ]
    .align_y(Alignment::Center);

    let main_view = container(column![page_heading, header_row, content,].spacing(16))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(16)
        .style(|_| glass_panel_style());

    if let Some(id) = launcher.active_context_menu {
        let modal = view_context_modal(launcher, id);
        stack![main_view, modal,].into()
    } else {
        main_view.into()
    }
}

pub fn instance_row<'a>(
    launcher: &RixLauncher,
    instance: &'a MinecraftInstance,
    with_delete: bool,
) -> Element<'a, Message> {
    let active = Some(instance.id) == launcher.selected_instance_id;
    let is_running = launcher.active_processes.contains_key(&instance.id);
    let pulse = if is_running {
        0.55 + 0.45 * (launcher.anim_time * 0.004).sin().abs()
    } else {
        0.0
    };

    let loader = instance.loader.as_str();

    // Running dot indicator
    let run_dot = container(iced::widget::Space::new().width(6).height(6))
        .width(6)
        .height(6)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.27, 0.85, 0.46, pulse,
            ))),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let select = Button::new(
        row![
            container(
                text(crate::launcher::utils::instance_icon_text(instance))
                    .size(14)
                    .bold()
                    .color(Color::WHITE)
            )
            .width(36)
            .height(36)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(move |_| avatar_style_for_name(&instance.name)),
            column![
                row![
                    text(&instance.name).size(13).bold().color(Color::WHITE),
                    iced::widget::Space::new().width(4),
                    run_dot,
                ]
                .align_y(Alignment::Center),
                row![
                    text(&instance.version)
                        .size(10)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    iced::widget::Space::new().width(5),
                    container(text(loader).size(9).bold())
                        .padding(iced::Padding::new(1.5).left(5.0).right(5.0))
                        .style(move |_| crate::style::loader_badge_style(loader)),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .on_press(Message::SelectInstance(instance.id))
    .style(move |_theme, status| instance_button_style(status, active))
    .width(Length::Fill)
    .padding(10);

    let mut content = row![select].spacing(8).align_y(Alignment::Center);

    if with_delete {
        content = content.push(
            Button::new(
                container(text("X").size(12).bold())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .on_press(Message::DeleteInstance(instance.id))
            .style(|_theme, status| outline_button_style(status))
            .width(38)
            .height(58),
        );
    }

    content.into()
}

pub fn empty_panel<'a>(title: &'a str, description: &'a str, fill: bool) -> Element<'a, Message> {
    let height = if fill { Length::Fill } else { Length::Shrink };
    container(
        column![
            text(title).size(18).bold().color(Color::WHITE),
            text(description)
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.56)),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(height)
    .padding(20)
    .style(|_| subtle_panel_style())
    .into()
}

pub fn section_title<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(title).size(15).bold().color(Color::WHITE),
        text(subtitle)
            .size(11)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.46)),
    ]
    .spacing(2)
    .into()
}

pub fn metric<'a>(label: &'a str, value: usize) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            text(value.to_string()).size(18).bold().color(Color::WHITE),
        ]
        .spacing(4),
    )
    .width(Length::Fill)
    .padding(12)
    .style(|_| subtle_panel_style())
    .into()
}

// Re-export column! macro usage locally
#[macro_export]
macro_rules! column {
    ($($x:expr),* $(,)?) => {
        iced::widget::Column::new()$(.push($x))*
    };
}
pub(crate) use column;

const SVG_FOLDER: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>"##;
const SVG_DUPLICATE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>"##;
const SVG_CLONE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect><path d="M9 14l2 2 4-4"></path></svg>"##;
const SVG_DELETE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>"##;
const SVG_CLOSE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>"##;

pub fn instance_card<'a>(
    launcher: &RixLauncher,
    instance: &'a MinecraftInstance,
) -> Element<'a, Message> {
    let active = Some(instance.id) == launcher.selected_instance_id;
    let is_running = launcher.active_processes.contains_key(&instance.id);
    let loader = instance.loader.as_str();
    let (lr, lg, lb) = loader_color(loader);

    // Pulse for running instances
    let glow_alpha = if is_running {
        0.50 + 0.40 * (launcher.anim_time * 0.004).sin().abs()
    } else {
        0.0
    };

    // Top accent stripe — loader color pill centered at the top, cleanly within card bounds
    let top_accent = container(
        container(iced::widget::Space::new().width(52).height(3))
            .width(52)
            .height(3)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(lr, lg, lb, 0.90))),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .width(Length::Fill)
    .align_x(Horizontal::Center)
    .padding(iced::Padding::new(0.0).top(2.0));

    // Running live dot
    let run_dot = container(iced::widget::Space::new().width(7).height(7))
        .width(7)
        .height(7)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.27,
                0.85,
                0.46,
                glow_alpha,
            ))),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let card_content = column![
        top_accent,
        container(
            column![
                // Top area: avatar with running indicator
                stack![
                    container(
                        container(
                            text(crate::launcher::utils::instance_icon_text(instance))
                                .size(18)
                                .bold()
                                .color(Color::WHITE)
                        )
                        .width(46)
                        .height(46)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                        .style(move |_| avatar_style_for_name(&instance.name))
                    )
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                    if is_running {
                        container(run_dot)
                            .width(Length::Fill)
                            .align_x(Horizontal::Right)
                            .padding(iced::Padding::new(0.0).right(4.0))
                    } else {
                        container(iced::widget::Space::new().width(0).height(0))
                            .width(Length::Fill)
                    },
                ],
                // Instance name
                container(
                    text(&instance.name)
                        .size(13)
                        .bold()
                        .color(Color::WHITE)
                )
                .width(Length::Fill)
                .align_x(Horizontal::Center),
                // Version + Loader badge
                row![
                    text(&instance.version)
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
                    container(text(loader).size(9).bold())
                        .padding(iced::Padding::new(2.0).left(6.0).right(6.0))
                        .style(move |_| loader_badge_style(loader)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            ]
            .spacing(8)
            .align_x(Alignment::Center)
            .width(Length::Fill)
        )
        .padding(iced::Padding::new(10.0).top(8.0))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let card_body = Button::new(card_content)
        .on_press(Message::SelectInstance(instance.id))
        .style(move |_theme, status| {
            let mut s = instance_button_style(status, active);
            // Running glow — green shadow
            if is_running {
                s.shadow = iced::Shadow {
                    color: Color::from_rgba(0.27, 0.85, 0.46, glow_alpha),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: 12.0,
                };
                s.border.color = Color::from_rgba(0.27, 0.85, 0.46, glow_alpha * 1.2);
                s.border.width = 1.5;
            }
            s
        })
        .width(Length::Fixed(INSTANCE_CARD_SIZE))
        .height(Length::Fixed(INSTANCE_CARD_SIZE))
        .padding(0);

    container(
        iced::widget::mouse_area(card_body)
            .on_right_press(Message::InstanceRightClicked(instance.id)),
    )
    .width(Length::Fixed(INSTANCE_CARD_SIZE))
    .height(Length::Fixed(INSTANCE_CARD_SIZE))
    .into()
}

fn view_context_modal<'a>(launcher: &'a RixLauncher, instance_id: usize) -> Element<'a, Message> {
    let instance = match launcher.instances.iter().find(|i| i.id == instance_id) {
        Some(i) => i,
        None => return container(text("")).into(),
    };

    let menu_item =
        |icon_svg: &'static str, label: &'static str, msg: Message, hover_color: Color| {
            Button::new(
                row![
                    svg(svg::Handle::from_memory(icon_svg.as_bytes()))
                        .width(14)
                        .height(14)
                        .style(move |_, _| svg::Style {
                            color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.7)),
                        }),
                    text(label)
                        .size(13)
                        .bold()
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.9)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .on_press(msg)
            .padding(iced::Padding::new(10.0).left(16.0).right(16.0))
            .style(move |_theme, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => hover_color,
                    _ => Color::TRANSPARENT,
                };
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .width(Length::Fill)
        };

    let modal_card = container(
        column![
            menu_item(
                SVG_FOLDER,
                "Open Folder",
                Message::OpenInstanceFolder(instance.id),
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            ),
            menu_item(
                SVG_DUPLICATE,
                "Duplicate Profile",
                Message::DuplicateInstance(instance.id),
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            ),
            menu_item(
                SVG_CLONE,
                "Clone Instance...",
                Message::OpenCloneInstanceDialog(instance.id),
                Color::from_rgba(0.35, 0.60, 1.0, 0.15)
            ),
            menu_item(
                SVG_DELETE,
                "Delete Profile",
                Message::RequestDeleteInstance(instance.id),
                Color::from_rgba(0.85, 0.25, 0.25, 0.2)
            ),
        ]
        .spacing(4),
    )
    .width(200)
    .padding(6)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.08, 0.08, 0.10, 0.98,
        ))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    });

    let backdrop = Button::new(container(text("")).width(Length::Fill).height(Length::Fill))
        .on_press(Message::CloseContextMenu)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let mut x = launcher.context_menu_position.x;
    let mut y = (launcher.context_menu_position.y - 140.0).max(0.0);

    if x + 210.0 > launcher.window_width as f32 {
        x = (launcher.window_width as f32 - 210.0).max(0.0);
    }
    let max_y = (launcher.window_height as f32 - 140.0 - 130.0).max(0.0);
    y = y.min(max_y);

    stack![
        backdrop,
        container(modal_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(0.0).left(x).top(y))
    ]
    .into()
}
