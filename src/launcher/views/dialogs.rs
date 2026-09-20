#![allow(dead_code)]
use crate::icons::SVG_LIBRARY;
use crate::launcher::RixLauncher;
use crate::launcher::constants;
use crate::launcher::types::Message;
use crate::launcher::views::instances::{column, empty_panel, section_title};
use crate::style::{
    ACCENT_GREEN, ACCENT_ORANGE, TextBoldExt, action_button_style, custom_scrollbar_style,
    dialog_panel_style, glass_panel_style, horizontal_space, outline_button_style, pick_list_style,
    play_button_style, subtle_panel_style, text_input_style,
};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Button, Column, Row, container, pick_list, row, scrollable, svg, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Padding};

const SVG_CHEVRON_UP: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="18 15 12 9 6 15"></polyline></svg>"##;
const SVG_CHEVRON_DOWN: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>"##;
const SVG_EYE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"></path><circle cx="12" cy="12" r="3"></circle></svg>"##;

pub fn view_create_dialog(launcher: &RixLauncher) -> Element<'_, Message> {
    let loader_row = constants::SUPPORTED_LOADERS.iter().fold(
        Row::new().spacing(8).align_y(Alignment::Center),
        |row, loader| row.push(loader_chip(launcher, loader)),
    );

    let icon_preview = container(
        svg(svg::Handle::from_memory(SVG_LIBRARY.as_bytes()))
            .width(42)
            .height(42)
            .style(|_theme, _status| svg::Style {
                color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.64)),
            }),
    )
    .width(82)
    .height(82)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .style(|_| subtle_panel_style());

    let dialog = container(
        column![
            row![
                text("Create instance").size(20).bold().color(Color::WHITE),
                horizontal_space(Length::Fill),
                Button::new(text("Import from Modrinth").size(12).bold())
                    .on_press(Message::ShowImportDialog(true))
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(8.0).left(12.0).right(12.0)),
                horizontal_space(10),
                Button::new(
                    container(text("X").size(14).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::CloseCreateDialog)
                .style(|_theme, status| outline_button_style(status))
                .width(38)
                .height(38),
            ]
            .align_y(Alignment::Center),
            row![
                icon_preview,
                text("The profile icon is generated from the selected loader.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.46)),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            form_label("Name"),
            text_input("Instance name", &launcher.create_name)
                .on_input(Message::CreateNameChanged)
                .on_submit(Message::CreateInstance)
                .style(|_theme, status| text_input_style(status))
                .padding(12)
                .size(15),
            form_label("Loader"),
            loader_row,
            form_label("Game version"),
            custom_version_selector(
                launcher,
                &launcher.create_version,
                launcher.create_version_expanded,
                Message::ToggleCreateVersionExpanded,
                Message::CreateVersionSelected,
            ),
            row![
                text(&launcher.create_status)
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                horizontal_space(Length::Fill),
                Button::new(text("< Back").size(13).bold())
                    .on_press(Message::CloseCreateDialog)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(10.0).left(16.0).right(16.0)),
                Button::new(text("+ Create instance").size(13).bold())
                    .on_press(Message::CreateInstance)
                    .style(|_theme, status| play_button_style(status))
                    .padding(Padding::new(10.0).left(18.0).right(18.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .width(520)
    .padding(22)
    .style(|_| dialog_panel_style());

    container(
        row![
            horizontal_space(Length::Fill),
            dialog,
            horizontal_space(Length::Fill),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Vertical::Center)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.45,
        ))),
        ..Default::default()
    })
    .into()
}

fn loader_chip<'a>(launcher: &RixLauncher, loader: &'static str) -> Element<'a, Message> {
    let active = launcher.create_loader == loader;
    Button::new(text(loader).size(13).bold())
        .on_press(Message::CreateLoaderSelected(loader.to_string()))
        .style(move |_theme, status| {
            if active {
                play_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .padding(Padding::new(9.0).left(16.0).right(16.0))
        .into()
}

fn form_label<'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(13)
        .bold()
        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.82))
        .into()
}

pub fn view_create_instance(launcher: &RixLauncher) -> Element<'_, Message> {
    let loader_options = constants::SUPPORTED_LOADERS
        .iter()
        .map(|loader| loader.to_string())
        .collect::<Vec<_>>();

    container(
        column![
            row![
                section_title("Create Instance", ""),
                horizontal_space(Length::Fill),
                Button::new(
                    text(if launcher.versions_loading {
                        "Loading..."
                    } else {
                        "Reload Versions"
                    })
                    .size(12)
                    .bold()
                )
                .on_press(Message::ReloadVersions)
                .style(|_t, status| outline_button_style(status))
                .padding(Padding::new(8.0).left(12.0).right(12.0)),
            ]
            .align_y(Alignment::Center),
            text_input("Instance name", &launcher.create_name)
                .on_input(Message::CreateNameChanged)
                .on_submit(Message::CreateInstance)
                .style(|_theme, status| text_input_style(status))
                .padding(10)
                .size(13),
            custom_version_selector(
                launcher,
                &launcher.create_version,
                launcher.create_version_expanded,
                Message::ToggleCreateVersionExpanded,
                Message::CreateVersionSelected,
            ),
            pick_list(
                loader_options,
                Some(launcher.create_loader.clone()),
                Message::CreateLoaderSelected
            )
            .style(|_theme, status| pick_list_style(status))
            .menu_style(crate::style::custom_menu_style)
            .width(Length::Fill),
            Button::new(text("Create").size(13).bold())
                .on_press(Message::CreateInstance)
                .style(|_t, status| action_button_style(status))
                .width(Length::Fill)
                .padding(11),
            text(&launcher.create_status)
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
        ]
        .spacing(12),
    )
    .padding(16)
    .style(|_| glass_panel_style())
    .into()
}

pub fn view_java_card(launcher: &RixLauncher) -> Element<'_, Message> {
    let required = launcher.required_java_major_for_create();
    let java_options = launcher.java_labels();
    let selected = launcher.selected_java_label.clone();
    let ok = launcher
        .selected_java_runtime()
        .is_some_and(|runtime| runtime.major >= required);

    container(
        column![
            row![
                section_title("Java", ""),
                horizontal_space(Length::Fill),
                text(format!("Required {}", required))
                    .size(11)
                    .bold()
                    .color(if ok { ACCENT_GREEN } else { ACCENT_ORANGE }),
            ]
            .align_y(Alignment::Center),
            pick_list(java_options, selected, Message::JavaSelected)
                .style(|_theme, status| pick_list_style(status))
                .menu_style(crate::style::custom_menu_style)
                .width(Length::Fill),
            Button::new(
                text(
                    launcher
                        .java_installing
                        .map(|major| format!("Installing Java {major}..."))
                        .unwrap_or_else(|| format!("Install Java {required}"))
                )
                .size(12)
                .bold()
            )
            .on_press(Message::InstallRequiredJava)
            .style(|_t, status| outline_button_style(status))
            .width(Length::Fill)
            .padding(10),
        ]
        .spacing(12),
    )
    .padding(16)
    .style(|_| glass_panel_style())
    .into()
}

pub fn view_selected_instance_settings(launcher: &RixLauncher) -> Element<'_, Message> {
    let Some(instance) = launcher.selected_instance() else {
        return empty_panel(
            "No instance selected",
            "Select or create an instance first.",
            false,
        );
    };

    let loader_options = constants::SUPPORTED_LOADERS
        .iter()
        .map(|loader| loader.to_string())
        .collect::<Vec<_>>();

    container(
        column![
            section_title(&instance.name, ""),
            form_label("Game version"),
            custom_version_selector(
                launcher,
                &instance.version,
                launcher.settings_version_expanded,
                Message::ToggleSettingsVersionExpanded,
                Message::InstanceVersionSelected,
            ),
            form_label("Loader type"),
            pick_list(
                loader_options,
                Some(instance.loader.clone()),
                Message::InstanceLoaderSelected
            )
            .style(|_theme, status| pick_list_style(status))
            .menu_style(crate::style::custom_menu_style)
            .width(Length::Fill),
            row![
                crate::launcher::views::instances::metric("Mods", instance.mod_count),
                crate::launcher::views::instances::metric(
                    "Resource Packs",
                    instance.resource_pack_count
                ),
                crate::launcher::views::instances::metric("Shaders", instance.shader_count),
            ]
            .spacing(10),
        ]
        .spacing(12),
    )
    .padding(16)
    .style(|_| glass_panel_style())
    .into()
}

fn custom_version_selector<'a, F>(
    launcher: &'a RixLauncher,
    current_val: &'a str,
    expanded: bool,
    toggle_msg: Message,
    on_select: F,
) -> Element<'a, Message>
where
    F: Fn(String) -> Message + 'a,
{
    let top_radius = if expanded {
        iced::border::Radius {
            top_left: 8.0,
            top_right: 8.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    } else {
        8.0.into()
    };

    let header = Button::new(
        container(
            row![
                text(current_val).size(13).bold().color(Color::WHITE),
                horizontal_space(Length::Fill),
                svg(svg::Handle::from_memory(if expanded {
                    SVG_CHEVRON_UP.as_bytes()
                } else {
                    SVG_CHEVRON_DOWN.as_bytes()
                }))
                .width(10)
                .height(10)
                .style(|_, _| svg::Style {
                    color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
                }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill),
    )
    .on_press(toggle_msg)
    .padding(12)
    .style(move |_theme, status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => Color::from_rgba(0.18, 0.38, 0.28, 0.8),
            _ => Color::from_rgba(0.18, 0.38, 0.28, 0.6),
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: top_radius,
            },
            ..Default::default()
        }
    })
    .width(Length::Fill);

    if !expanded {
        return header.into();
    }

    let mut list_col = Column::new().spacing(4);
    for version in &launcher.minecraft_versions {
        let is_selected = version == current_val;
        let version_clone = version.clone();
        let item_btn = Button::new(
            row![
                text(version)
                    .size(13)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.85)),
                horizontal_space(Length::Fill),
                if is_selected {
                    text("●").size(10).color(Color::from_rgb(0.2, 0.8, 0.4))
                } else {
                    text("")
                }
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_select(version_clone))
        .padding(10)
        .style(move |_theme, status| {
            let bg = if is_selected {
                Color::from_rgba(0.18, 0.38, 0.28, 0.25)
            } else {
                match status {
                    iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    _ => Color::TRANSPARENT,
                }
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: Color::WHITE,
                border: iced::Border::default(),
                ..Default::default()
            }
        })
        .width(Length::Fill);

        list_col = list_col.push(item_btn);
    }

    let scroll_list = scrollable(list_col)
        .height(160)
        .style(|_theme, _status| custom_scrollbar_style());

    let bottom_text = if launcher.show_snapshots {
        "Hide snapshots"
    } else {
        "Show all versions"
    };

    let bottom_btn = Button::new(
        container(
            row![
                svg(svg::Handle::from_memory(SVG_EYE.as_bytes()))
                    .width(14)
                    .height(14)
                    .style(|_, _| svg::Style {
                        color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.85)),
                    }),
                horizontal_space(6),
                text(bottom_text).size(12).bold(),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .on_press(Message::ShowSnapshotsToggled(!launcher.show_snapshots))
    .padding(10)
    .style(|_theme, status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            _ => Color::from_rgba(1.0, 1.0, 1.0, 0.04),
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.70),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: iced::border::Radius {
                    top_left: 0.0,
                    top_right: 0.0,
                    bottom_right: 8.0,
                    bottom_left: 8.0,
                },
            },
            ..Default::default()
        }
    })
    .width(Length::Fill);

    container(column![header, scroll_list, bottom_btn,])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.08, 0.08, 0.10, 0.6,
            ))),
            border: iced::Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

pub fn view_import_dialog(launcher: &RixLauncher) -> Element<'_, Message> {
    let mut list_col = Column::new().spacing(8);

    if launcher.modrinth_profiles.is_empty() {
        list_col = list_col.push(
            container(
                text("No Modrinth App profiles found to import.")
                    .size(13)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.4)),
            )
            .padding(20)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        );
    } else {
        for profile in &launcher.modrinth_profiles {
            let p_clone = profile.clone();
            let card = container(
                row![
                    column![
                        text(&profile.name).size(14).bold().color(Color::WHITE),
                        text(format!(
                            "Minecraft {} ({})",
                            profile.game_version, profile.mod_loader
                        ))
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.54)),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    Button::new(text("Import").size(12).bold())
                        .on_press(Message::ImportProfileSelected(p_clone))
                        .style(|_theme, status| play_button_style(status))
                        .padding(Padding::new(6.0).left(12.0).right(12.0))
                ]
                .align_y(Alignment::Center),
            )
            .padding(10)
            .style(|_| subtle_panel_style());
            list_col = list_col.push(card);
        }
    }

    let dialog = container(
        column![
            row![
                text("Import from Modrinth App")
                    .size(20)
                    .bold()
                    .color(Color::WHITE),
                horizontal_space(Length::Fill),
                Button::new(
                    container(text("X").size(14).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::ShowImportDialog(false))
                .style(|_theme, status| outline_button_style(status))
                .width(38)
                .height(38),
            ]
            .align_y(Alignment::Center),
            text(&launcher.import_status)
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
            scrollable(list_col)
                .height(Length::Fixed(260.0))
                .style(|_theme, _status| custom_scrollbar_style()),
            row![
                horizontal_space(Length::Fill),
                Button::new(text("< Back").size(13).bold())
                    .on_press(Message::ShowImportDialog(false))
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(10.0).left(16.0).right(16.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .width(520)
    .padding(22)
    .style(|_| dialog_panel_style());

    container(
        row![
            horizontal_space(Length::Fill),
            dialog,
            horizontal_space(Length::Fill),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Vertical::Center)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.45,
        ))),
        ..Default::default()
    })
    .into()
}

pub fn view_file_delete_confirmation_modal<'a>(
    _launcher: &'a RixLauncher,
    filename: &'a str,
) -> Element<'a, Message> {
    use crate::style::{action_button_style, dialog_panel_style, outline_button_style};
    use iced::alignment::{Horizontal, Vertical};
    use iced::widget::{Button, column, container, row, stack, text};
    use iced::{Alignment, Color, Length};

    let display_name = crate::launcher::utils::strip_disabled_suffix(filename).unwrap_or(filename);

    let dialog = container(
        column![
            text("Delete File").size(18).bold().color(Color::WHITE),
            text(format!(
                "Are you sure you want to delete \"{}\"?\nThis action cannot be undone.",
                display_name
            ))
            .size(13)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            row![
                Button::new(
                    container(text("Cancel").size(12).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::CancelDeleteInstalledFile)
                .style(|_t, status| outline_button_style(status))
                .width(100)
                .height(38),
                Button::new(
                    container(text("Delete").size(12).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::DeleteInstalledFileConfirmed(filename.to_string()))
                .style(|_t, status| {
                    let mut s = action_button_style(status);
                    s.background = Some(iced::Background::Color(Color::from_rgba(
                        0.85, 0.25, 0.25, 0.8,
                    )));
                    s
                })
                .width(100)
                .height(38),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(20),
    )
    .width(360)
    .padding(20)
    .style(|_| dialog_panel_style());

    let backdrop = Button::new(container(text("")).width(Length::Fill).height(Length::Fill))
        .on_press(Message::CancelDeleteInstalledFile)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.5,
            ))),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill);

    stack![
        backdrop,
        container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
    ]
    .into()
}

pub fn view_instance_delete_confirmation_modal(
    launcher: &RixLauncher,
    instance_id: usize,
) -> Element<'_, Message> {
    let name = launcher
        .instances
        .iter()
        .find(|instance| instance.id == instance_id)
        .map(|instance| instance.name.as_str())
        .unwrap_or("this instance");
    let dialog = container(
        column![
            text("Delete instance?").size(19).bold().color(Color::WHITE),
            text(format!(
                "Delete \"{name}\" and all its mods, worlds and settings?\nThis cannot be undone."
            ))
            .size(13)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.68)),
            row![
                Button::new(text("Cancel").size(12).bold())
                    .on_press(Message::CancelDeleteInstance)
                    .style(|_, status| outline_button_style(status))
                    .padding(Padding::new(10.0).left(18.0).right(18.0)),
                Button::new(text("Delete instance").size(12).bold())
                    .on_press(Message::DeleteInstanceConfirmed(instance_id))
                    .style(|_, status| {
                        let mut style = action_button_style(status);
                        style.background =
                            Some(iced::Background::Color(Color::from_rgb(0.72, 0.18, 0.22)));
                        style
                    })
                    .padding(Padding::new(10.0).left(18.0).right(18.0)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(18),
    )
    .width(420)
    .padding(24)
    .style(|_| dialog_panel_style());

    let backdrop = Button::new(container(text("")))
        .on_press(Message::CancelDeleteInstance)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.62,
            ))),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill);

    iced::widget::stack![
        backdrop,
        container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
    ]
    .into()
}

const SVG_CLONE_ICON: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect><path d="M9 14l2 2 4-4"></path></svg>"##;
const SVG_WARN_TRIANGLE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>"##;

pub fn view_clone_dialog(launcher: &RixLauncher) -> Element<'_, Message> {
    use crate::style::{accent_panel_style, clone_button_style, status_panel_style};

    // ── Loader chips ─────────────────────────────────────────────────────────
    let loader_row = constants::SUPPORTED_LOADERS.iter().fold(
        Row::new().spacing(6).align_y(Alignment::Center),
        |row, loader| row.push(clone_loader_chip(launcher, loader)),
    );

    // ── Source instance badge ─────────────────────────────────────────────────
    let src_badge: Element<'_, Message> = if let Some(src_id) = launcher.show_clone_dialog {
        if let Some(src) = launcher.instances.iter().find(|i| i.id == src_id) {
            container(
                row![
                    // coloured dot
                    container(text("").size(1))
                        .width(8)
                        .height(8)
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba(
                                0.30, 0.62, 1.0, 0.90
                            ))),
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    text("Source  ")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
                    text(&src.name).size(12).bold().color(Color::WHITE),
                    text(format!("  ·  {} / {}", src.version, src.loader))
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(iced::Padding::new(10.0).left(14.0).right(14.0))
            .style(|_| accent_panel_style(0.30, 0.62, 1.0, 0.25))
            .into()
        } else {
            Column::new().into()
        }
    } else {
        Column::new().into()
    };

    // ── Status area ───────────────────────────────────────────────────────────
    let status_area: Element<'_, Message> = if !launcher.clone_status.is_empty() {
        let (r, g, b, icon) = if launcher.clone_status.contains('\u{26a0}')
            || launcher.clone_status.contains("Not found")
            || launcher.clone_status.contains("Could not clone")
        {
            // amber warning
            (1.0f32, 0.78f32, 0.20f32, "⚠")
        } else if launcher.clone_status.to_lowercase().contains("fail")
            || launcher.clone_status.to_lowercase().contains("error")
        {
            // red error
            (1.0f32, 0.30f32, 0.30f32, "✕")
        } else {
            // green success
            (0.30f32, 0.90f32, 0.45f32, "✓")
        };
        container(
            row![
                text(icon)
                    .size(13)
                    .bold()
                    .color(Color::from_rgba(r, g, b, 0.95)),
                text(&launcher.clone_status)
                    .size(12)
                    .color(Color::from_rgba(
                        r * 0.95 + 0.05,
                        g * 0.95 + 0.05,
                        b * 0.95 + 0.05,
                        0.90
                    )),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        )
        .width(Length::Fill)
        .padding(iced::Padding::new(11.0).left(14.0).right(14.0))
        .style(move |_| status_panel_style(r, g, b))
        .into()
    } else {
        Column::new().into()
    };

    // ── Info note ─────────────────────────────────────────────────────────────
    let info_note = container(
        row![
            svg(svg::Handle::from_memory(SVG_WARN_TRIANGLE.as_bytes()))
                .width(13).height(13)
                .style(|_, _| svg::Style {
                    color: Some(Color::from_rgba(1.0, 0.78, 0.20, 0.70))
                }),
            text("Configs & settings are copied. Mods are re-downloaded for the new version via SHA-1 lookup. Shaders & resource packs are copied as-is.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.42)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding::new(10.0).left(12.0).right(12.0))
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(1.0, 0.78, 0.20, 0.04))),
        border: iced::Border {
            color: Color::from_rgba(1.0, 0.78, 0.20, 0.18),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    // ── Toggle message ────────────────────────────────────────────────────────
    let toggle_msg = Message::CloneVersionExpanded(!launcher.clone_version_expanded);

    // ── Clone button ──────────────────────────────────────────────────────────
    let clone_btn = if launcher.clone_in_progress {
        Button::new(
            row![text("⟳").size(14), text("  Cloning...").size(13).bold(),]
                .align_y(Alignment::Center),
        )
        .style(|_theme, status| clone_button_style(status))
        .padding(Padding::new(11.0).left(20.0).right(20.0))
    } else {
        Button::new(
            row![
                svg(svg::Handle::from_memory(SVG_CLONE_ICON.as_bytes()))
                    .width(14)
                    .height(14)
                    .style(|_, _| svg::Style {
                        color: Some(Color::from_rgba(0.04, 0.04, 0.08, 1.0))
                    }),
                text("  Clone Instance").size(13).bold(),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::StartCloneInstance)
        .style(|_theme, status| clone_button_style(status))
        .padding(Padding::new(11.0).left(20.0).right(20.0))
    };

    // ── Thin separator ────────────────────────────────────────────────────────
    let divider = container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                1.0, 1.0, 1.0, 0.07,
            ))),
            ..Default::default()
        });

    // ── Dialog body ───────────────────────────────────────────────────────────
    let dialog = container(
        column![
            // Header
            row![
                container(
                    svg(svg::Handle::from_memory(SVG_CLONE_ICON.as_bytes()))
                        .width(18)
                        .height(18)
                        .style(|_, _| svg::Style {
                            color: Some(Color::from_rgba(0.30, 0.62, 1.0, 0.95))
                        })
                )
                .width(36)
                .height(36)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(|_| accent_panel_style(0.30, 0.62, 1.0, 0.30)),
                column![
                    text("Clone Instance").size(18).bold().color(Color::WHITE),
                    text("Smart copy with version migration")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                ]
                .spacing(2),
                horizontal_space(Length::Fill),
                // Close button
                Button::new(
                    container(text("✕").size(13).bold())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .on_press(Message::CloseCloneInstanceDialog)
                .style(|_theme, status| outline_button_style(status))
                .width(34)
                .height(34),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            // Source badge
            src_badge,
            divider,
            // Name field
            form_label("New Instance Name"),
            text_input("e.g. My 1.21.5 world", &launcher.clone_name)
                .on_input(Message::CloneNameChanged)
                .on_submit(Message::StartCloneInstance)
                .style(|_theme, status| text_input_style(status))
                .padding(12)
                .size(14),
            // Loader chips
            form_label("Loader"),
            loader_row,
            // Version picker
            form_label("Minecraft Version"),
            custom_version_selector(
                launcher,
                &launcher.clone_version,
                launcher.clone_version_expanded,
                toggle_msg,
                Message::CloneVersionSelected,
            ),
            info_note,
            status_area,
            // Action row
            row![
                horizontal_space(Length::Fill),
                Button::new(text("Cancel").size(13))
                    .on_press(Message::CloseCloneInstanceDialog)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(10.0).left(16.0).right(16.0)),
                clone_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(14),
    )
    .width(530)
    .padding(Padding::new(24.0))
    .style(|_| {
        let mut s = dialog_panel_style();
        // Add a blue left-side accent line via border colour blending
        s.border.color = Color::from_rgba(0.30, 0.62, 1.0, 0.22);
        s
    });

    // ── Backdrop ─────────────────────────────────────────────────────────────
    let backdrop = Button::new(container(text("")).width(Length::Fill).height(Length::Fill))
        .on_press(Message::CloseCloneInstanceDialog)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill);

    iced::widget::stack![
        backdrop,
        container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, 0.60
                ))),
                ..Default::default()
            })
    ]
    .into()
}

fn clone_loader_chip<'a>(launcher: &RixLauncher, loader: &'static str) -> Element<'a, Message> {
    use crate::style::clone_button_style;
    let active = launcher.clone_loader == loader;
    Button::new(text(loader).size(13).bold())
        .on_press(Message::CloneLoaderSelected(loader.to_string()))
        .style(move |_theme, status| {
            if active {
                clone_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .padding(Padding::new(9.0).left(15.0).right(15.0))
        .into()
}
