use crate::icons::{SVG_PLAY, svg_handle};
use crate::launcher::RixLauncher;
use crate::launcher::constants;
use crate::launcher::types::{Message, ModDetailsTab};
use crate::launcher::views::instances::{empty_panel, section_title};
use crate::style::{
    ACCENT_GREEN, ACCENT_ORANGE, TextBoldExt, action_button_style, avatar_style_for_name,
    custom_menu_style, custom_scrollbar_style, dialog_panel_style, glass_panel_style,
    hero_panel_style, horizontal_space, instance_button_style, loader_badge_style,
    outline_button_style, pick_list_style, play_button_style, subtle_panel_style,
    text_input_style, vertical_space,
};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Button, Column, column, container, image, pick_list, progress_bar, row, scrollable, slider,
    stack, svg, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Padding};

pub fn view_home(launcher: &RixLauncher) -> Element<'_, Message> {
    let main: Element<'_, Message> = if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
        column![
            view_instance_select_panel(launcher),
            view_account_manager(launcher),
        ]
        .spacing(14)
        .height(Length::Fill)
        .into()
    } else {
        row![
            view_instance_select_panel(launcher),
            view_account_manager(launcher),
        ]
        .spacing(14)
        .height(Length::Fill)
        .into()
    };

    if launcher.microsoft_login.is_some() {
        let modal = view_microsoft_login_modal(launcher);
        stack![main, modal,].into()
    } else {
        main.into()
    }
}

fn get_installed_files(
    instance: &crate::launcher::types::MinecraftInstance,
    kind: crate::launcher::types::ContentKind,
    search: &str,
) -> Vec<String> {
    let folder = instance.path.join(kind.install_folder());
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let filename = entry.file_name().to_string_lossy().into_owned();
                    if search.is_empty() || filename.to_lowercase().contains(&search.to_lowercase())
                    {
                        files.push(filename);
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    files
}

fn content_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn content_file_token(filename: &str) -> String {
    let filename = crate::launcher::utils::strip_disabled_suffix(filename).unwrap_or(filename);
    let stem = filename
        .strip_suffix(".jar")
        .or_else(|| filename.strip_suffix(".JAR"))
        .unwrap_or(filename);
    let stem = stem
        .char_indices()
        .find(|(index, character)| {
            *character == '-'
                && stem
                    .get(index + character.len_utf8()..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|next| next.is_ascii_digit())
        })
        .map(|(index, _)| &stem[..index])
        .unwrap_or(stem);
    content_token(stem)
}

fn content_is_installed(
    item: &crate::launcher::types::ContentItem,
    installed_project_ids: &std::collections::HashSet<String>,
    installed_files: &[String],
) -> bool {
    if installed_project_ids.contains(&item.id)
        || (!item.slug.is_empty() && installed_project_ids.contains(&item.slug))
    {
        return true;
    }

    // Older installations have no registry yet. Modrinth filenames normally
    // contain the slug, so use a conservative token match as a migration
    // fallback while keeping the registry authoritative for new installs.
    let mut tokens = vec![content_token(&item.id), content_token(&item.title)];
    if !item.slug.is_empty() {
        tokens.push(content_token(&item.slug));
    }
    tokens.retain(|token| token.len() >= 3);
    if tokens.is_empty() {
        return false;
    }

    installed_files.iter().any(|filename| {
        let file_token = content_file_token(filename);
        !file_token.is_empty()
            && tokens
                .iter()
                .any(|token| file_token == *token || file_token.starts_with(token))
    })
}

fn view_instance_select_panel(launcher: &RixLauncher) -> Element<'_, Message> {
    if launcher.instances.is_empty() {
        return container(
            column![
                section_title("Instances", "Create an instance to start installing mods."),
                empty_panel("No instances", "Create one to get started.", false),
                Button::new(text("Create Instance").size(13).bold())
                    .on_press(Message::OpenCreateDialog)
                    .style(|_theme, status| play_button_style(status))
                    .padding(Padding::new(10.0).left(16.0).right(16.0)),
            ]
            .spacing(20)
            .align_x(Alignment::Center),
        )
        .width(if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
            Length::Fill
        } else {
            Length::FillPortion(10)
        })
        .height(Length::Fill)
        .padding(16)
        .style(|_| glass_panel_style())
        .into();
    }

    let selected_instance = launcher
        .selected_instance()
        .or_else(|| launcher.instances.first());

    let instance_names: Vec<String> = launcher.instances.iter().map(|i| i.name.clone()).collect();
    let selected_name = selected_instance.map(|i| i.name.clone());

    let header_row = row![
        section_title("Instance", ""),
        horizontal_space(10),
        pick_list(
            instance_names,
            selected_name,
            Message::InstanceDropdownSelected
        )
        .style(|_theme, status| pick_list_style(status))
        .menu_style(custom_menu_style)
        .width(Length::Fixed(180.0)),
        horizontal_space(10),
        Button::new(text("+").size(14).bold())
            .on_press(Message::OpenCreateDialog)
            .style(|_theme, status| play_button_style(status))
            .padding(Padding::new(6.0).left(12.0).right(12.0)),
        horizontal_space(Length::Fill),
        Button::new(text("Refresh").size(12).bold())
            .on_press(Message::RefreshProfiles)
            .style(|_t, status| outline_button_style(status))
            .padding(Padding::new(8.0).left(14.0).right(14.0)),
    ]
    .align_y(Alignment::Center);

    let Some(instance) = selected_instance else {
        return container(text("Error loading selected instance").color(Color::WHITE))
            .width(if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
                Length::Fill
            } else {
                Length::FillPortion(10)
            })
            .height(Length::Fill)
            .padding(16)
            .style(|_| glass_panel_style())
            .into();
    };

    // Tabs row: Installed Content | Instance Settings | Open Directory
    let content_tab_active = !launcher.instance_settings_active;
    let settings_tab_active = launcher.instance_settings_active;

    let tabs = row![
        Button::new(
            container(text("Content").size(12).bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .on_press(Message::SetBrowseContentActive(false))
        .style(move |_theme, status| {
            if content_tab_active {
                action_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(36.0))
        .padding(0),
        Button::new(
            container(text("Settings").size(12).bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .on_press(Message::SetInstanceSettingsActive(true))
        .style(move |_theme, status| {
            if settings_tab_active {
                action_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(36.0))
        .padding(0),
        Button::new(
            container(text("Open Directory").size(12).bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .on_press(Message::OpenInstanceFolder(instance.id))
        .style(|_theme, status| outline_button_style(status))
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(36.0))
        .padding(0),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    // Kind Sub-tabs row: [Mods], [Resource Packs], [Shaders]
    let kind_tab = |kind: crate::launcher::types::ContentKind, label: &'static str| {
        let active = launcher.content_kind == kind;
        Button::new(
            container(text(label).size(12).bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .on_press(Message::ContentKindSelected(kind))
        .style(move |_theme, status| {
            if active {
                action_button_style(status)
            } else {
                outline_button_style(status)
            }
        })
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(36.0))
        .padding(0)
    };

    let kind_tabs_row = row![
        kind_tab(crate::launcher::types::ContentKind::Mods, "Mods"),
        kind_tab(
            crate::launcher::types::ContentKind::ResourcePacks,
            "Resource Packs",
        ),
        kind_tab(crate::launcher::types::ContentKind::Shaders, "Shaders"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let main_content: Element<'_, Message> = if launcher.instance_settings_active {
        // Per-instance settings (memory, Java runtime)
        view_instance_settings(launcher)
    } else if !launcher.browse_content_active {
        // Installed local files view
        let files = get_installed_files(
            &instance,
            launcher.content_kind,
            &launcher.installed_search_query,
        );
        let mut list = Column::new().spacing(8);

        if files.is_empty() {
            list = list.push(
                container(
                    text("No files installed matching query.")
                        .size(12)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                )
                .padding(20)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
            );
        } else {
            for filename in files {
                let name_clone = filename.clone();

                let icon_element: Element<'_, Message> =
                    if launcher.content_kind == crate::launcher::types::ContentKind::Mods {
                        if let Some(handle) = launcher.local_mod_icons.get(&filename) {
                            container(image(handle.clone()).width(32).height(32))
                                .clip(true)
                                .style(|_| container::Style {
                                    border: iced::Border {
                                        radius: 8.0.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .into()
                        } else {
                            let letter = filename
                                .chars()
                                .next()
                                .unwrap_or('?')
                                .to_uppercase()
                                .to_string();
                            let fn1 = filename.clone();
                            container(text(letter).size(12).bold().color(Color::WHITE))
                                .width(32)
                                .height(32)
                                .align_x(Horizontal::Center)
                                .align_y(Vertical::Center)
                                .style(move |_| avatar_style_for_name(&fn1))
                                .into()
                        }
                    } else {
                        let letter = filename
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string();
                        let fn2 = filename.clone();
                        container(text(letter).size(12).bold().color(Color::WHITE))
                            .width(32)
                            .height(32)
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .style(move |_| avatar_style_for_name(&fn2))
                            .into()
                    };

                let display_name = crate::launcher::utils::strip_disabled_suffix(&filename)
                    .unwrap_or(&filename)
                    .to_string();
                let is_enabled = !crate::launcher::utils::is_disabled_mod_filename(&filename);
                let toggle_switch = Button::new(
                    container(
                        container("")
                            .width(14)
                            .height(14)
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::WHITE)),
                                border: Border {
                                    radius: 7.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                    )
                    .width(36)
                    .height(20)
                    .padding(2)
                    .align_x(if is_enabled {
                        Horizontal::Right
                    } else {
                        Horizontal::Left
                    })
                    .align_y(Vertical::Center)
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(if is_enabled {
                            ACCENT_GREEN
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.15)
                        })),
                        border: Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                )
                .on_press(Message::ToggleModEnabled(name_clone.clone(), !is_enabled))
                .style(|_, _| iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                });

                let update_btn = if let Some(update_info) = launcher.update_results.get(&filename) {
                    if launcher.updating_files.contains(&filename) {
                        Some(
                            Button::new(text("Updating...").size(11).bold())
                                .style(|_t, status| outline_button_style(status))
                                .padding(Padding::new(5.0).left(10.0).right(10.0)),
                        )
                    } else {
                        Some(
                            Button::new(
                                row![
                                    text("Update").size(11).bold(),
                                    text(format!(" (v{})", update_info.new_version))
                                        .size(9)
                                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.6))
                                ]
                                .align_y(Alignment::Center),
                            )
                            .on_press(Message::UpdateMod(filename.clone()))
                            .style(|_t, status| {
                                let mut s = play_button_style(status);
                                s.text_color = Color::BLACK;
                                s
                            })
                            .padding(Padding::new(5.0).left(10.0).right(10.0)),
                        )
                    }
                } else {
                    None
                };

                let delete_btn = Button::new(text("Delete").size(13).bold())
                    .on_press(Message::RequestDeleteInstalledFile(name_clone.clone()))
                    .style(|_t, status| {
                        let mut s = outline_button_style(status);
                        s.text_color = Color::from_rgba(0.9, 0.3, 0.3, 1.0);
                        s
                    })
                    .padding(Padding::new(8.0).left(16.0).right(16.0));

                let mut right_row = row![].spacing(8).align_y(Alignment::Center);
                if let Some(btn) = update_btn {
                    right_row = right_row.push(btn);
                }
                if launcher.content_kind == crate::launcher::types::ContentKind::Mods {
                    right_row = right_row.push(toggle_switch);
                }
                right_row = right_row.push(delete_btn);

                let card_content: Element<'_, Message> =
                    if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
                        column![
                            row![
                                icon_element,
                                text(display_name).size(13).bold().color(Color::WHITE),
                            ]
                            .spacing(12)
                            .align_y(Alignment::Center),
                            right_row,
                        ]
                        .spacing(8)
                        .into()
                    } else {
                        row![
                            icon_element,
                            text(display_name).size(13).bold().color(Color::WHITE),
                            horizontal_space(Length::Fill),
                            right_row,
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .into()
                    };
                let card = container(card_content)
                    .padding(10)
                    .style(|_| subtle_panel_style());
                list = list.push(card);
            }
        }

        let installed_list = scrollable(list)
            .id(iced::widget::Id::new("installed_mods_scrollable"))
            .height(Length::Fill)
            .style(|_theme, _status| custom_scrollbar_style());

        let search_input = text_input(
            "Search installed files...",
            &launcher.installed_search_query,
        )
        .on_input(Message::InstalledSearchQueryChanged)
        .style(|_theme, status| text_input_style(status))
        .padding(10)
        .size(13)
        .width(Length::Fill);

        let mut action_row = row![].spacing(8).align_y(Alignment::Center);

        if launcher.content_kind == crate::launcher::types::ContentKind::Mods {
            let check_btn = Button::new(
                text(if launcher.checking_updates {
                    "Checking..."
                } else {
                    "Check Updates"
                })
                .size(12)
                .bold(),
            )
            .on_press(Message::CheckModUpdates)
            .style(|_theme, status| outline_button_style(status))
            .padding(Padding::new(9.0).left(14.0).right(14.0));
            action_row = action_row.push(check_btn);

            let update_count = launcher.update_results.len();
            if update_count > 0 {
                let update_all_btn = Button::new(
                    text(format!("Update All ({})", update_count))
                        .size(12)
                        .bold(),
                )
                .on_press(Message::UpdateAllMods)
                .style(|_theme, status| {
                    let mut s = play_button_style(status);
                    s.text_color = Color::BLACK;
                    s
                })
                .padding(Padding::new(9.0).left(14.0).right(14.0));
                action_row = action_row.push(update_all_btn);
            }
        }

        action_row = action_row
            .push(
                Button::new(text("Import file").size(12).bold())
                    .on_press(Message::SelectImportFile)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(9.0).left(14.0).right(14.0)),
            )
            .push(
                Button::new(text("Browse remote content").size(12).bold())
                    .on_press(Message::SetBrowseContentActive(true))
                    .style(|_theme, status| play_button_style(status))
                    .padding(Padding::new(9.0).left(14.0).right(14.0)),
            );

        let action_buttons: Element<'_, Message> = action_row.into();
        let tools: Element<'_, Message> =
            if launcher.window_width < constants::STACKED_LAYOUT_WIDTH + 150 {
                column![search_input, action_buttons]
                    .spacing(8)
                    .width(Length::Fill)
                    .into()
            } else {
                row![search_input, action_buttons]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .into()
            };

        column![kind_tabs_row, tools, installed_list,]
            .spacing(12)
            .into()
    } else {
        // Remote content downloader view
        if launcher.project_details.is_some() || launcher.loading_project_details {
            view_mod_details(launcher, instance)
        } else {
            let provider_row = row![
                container(
                    text("Modrinth catalog")
                        .size(11)
                        .bold()
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.58))
                )
                .padding(Padding::new(6.0).left(12.0).right(12.0))
                .style(|_| subtle_panel_style()),
                horizontal_space(Length::Fill),
                Button::new(text("< Back to Installed").size(12).bold())
                    .on_press(Message::SetBrowseContentActive(false))
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(9.0).left(16.0).right(16.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let search_input = text_input("Search remote projects...", &launcher.content_query)
                .on_input(Message::ContentQueryChanged)
                .on_submit(Message::SearchContent)
                .style(|_theme, status| text_input_style(status))
                .padding(10)
                .size(13)
                .width(Length::Fill);
            let search_button = Button::new(text("Search").size(13).bold())
                .on_press(Message::SearchContent)
                .style(|_theme, status| play_button_style(status))
                .padding(Padding::new(9.0).left(18.0).right(18.0));
            let search_row: Element<'_, Message> =
                if launcher.window_width < constants::NARROW_LAYOUT_WIDTH {
                    column![search_input, search_button]
                        .spacing(8)
                        .width(Length::Fill)
                        .into()
                } else {
                    row![search_input, search_button]
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .into()
                };

            let installed_project_ids =
                crate::launcher::installed_content_project_ids(&instance, launcher.content_kind);
            let installed_files = get_installed_files(&instance, launcher.content_kind, "");
            let mut results = Column::new().spacing(8);
            for item in &launcher.content_results {
                let item_id = item.id.clone();
                let is_installed = content_is_installed(item, &installed_project_ids, &installed_files);
                let is_installing = launcher.content_installing_instance_id == Some(instance.id)
                    && launcher.content_installing_project_id.as_deref() == Some(item.id.as_str());

                // Build icon widget: use cached PNG bytes if available, otherwise a colored letter tile
                let icon_elem: Element<'_, Message> = if let Some(url) = &item.icon_url {
                    if let Some(handle) = launcher.icon_cache.get(url) {
                        container(image(handle.clone()).width(42).height(42))
                            .clip(true)
                            .style(|_| container::Style {
                                border: Border {
                                    radius: 8.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .into()
                    } else {
                        let title_for_placeholder = item.title.clone();
                        container(
                            text("…")
                                .size(12)
                                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
                        )
                        .width(42)
                        .height(42)
                        .center_x(Length::Fixed(42.0))
                        .center_y(Length::Fixed(42.0))
                        .style(move |_| avatar_style_for_name(&title_for_placeholder))
                        .into()
                    }
                } else {
                    let letter = item
                        .title
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase()
                        .to_string();
                    let title_clone = item.title.clone();
                    container(text(letter).size(14).bold().color(Color::WHITE))
                        .width(42)
                        .height(42)
                        .center_x(Length::Fixed(42.0))
                        .center_y(Length::Fixed(42.0))
                        .style(move |_| avatar_style_for_name(&title_clone))
                        .into()
                };

                let details = column![
                    text(&item.title).size(13).bold().color(Color::WHITE),
                    text(&item.description)
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
                ]
                .spacing(3)
                .width(Length::Fill);

                let install_button: Element<'_, Message> = if is_installing {
                    container(
                        row![
                            text("Downloading").size(11).bold(),
                            text(format!("{:.0}%", launcher.content_install_progress * 100.0))
                                .size(10)
                                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.68)),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding(Padding::new(7.0).left(14.0).right(14.0))
                    .style(|_| crate::style::status_panel_style(0.30, 0.62, 1.0))
                    .into()
                } else if is_installed {
                    container(
                        row![
                            text("✓").size(13).bold().color(ACCENT_GREEN),
                            text("Downloaded")
                                .size(11)
                                .bold()
                                .color(Color::from_rgba(0.65, 1.0, 0.72, 0.92)),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding(Padding::new(7.0).left(14.0).right(14.0))
                    .style(|_| crate::style::status_panel_style(0.27, 0.78, 0.46))
                    .into()
                } else {
                    Button::new(text("Install").size(11).bold())
                        .on_press(Message::InstallContentDirect(item_id.clone()))
                        .style(|_theme, status| play_button_style(status))
                        .padding(Padding::new(7.0).left(16.0).right(16.0))
                        .into()
                };

                let clickable_info = iced::widget::mouse_area(
                    container(
                        row![icon_elem, details]
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .width(Length::Fill),
                )
                .on_press(Message::OpenProjectDetails(item_id))
                .interaction(iced::mouse::Interaction::Pointer);

                let card_content: Element<'_, Message> =
                    if launcher.window_width < constants::NARROW_LAYOUT_WIDTH {
                        column![
                            clickable_info,
                            install_button,
                        ]
                        .spacing(10)
                        .into()
                    } else {
                        row![
                            clickable_info,
                            horizontal_space(12),
                            install_button,
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .into()
                    };

                let mut card_body = column![card_content].spacing(8);
                if is_installing {
                    card_body = card_body.push(
                        column![
                            progress_bar(0.0..=1.0, launcher.content_install_progress)
                                .style(|_| crate::style::progress_bar_style()),
                            text(&launcher.content_install_phase)
                                .size(10)
                                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.48)),
                        ]
                        .spacing(5),
                    );
                }
                let card = container(card_body)
                    .padding(12)
                    .style(|_| subtle_panel_style());
                results = results.push(card);
            }

            let results_body: Element<'_, Message> =
                if launcher.content_searching && launcher.content_results.is_empty() {
                    let pulse = 0.5 + 0.5 * (launcher.anim_time / 400.0).sin().abs();
                    container(
                        column![
                            row![
                                container(iced::widget::Space::new().width(10).height(10))
                                    .width(10)
                                    .height(10)
                                    .style(move |_| container::Style {
                                        background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, pulse))),
                                        border: Border {
                                            radius: 5.0.into(),
                                            ..Default::default()
                                        },
                                        shadow: iced::Shadow {
                                            color: Color::from_rgba(0.27, 0.85, 0.46, pulse * 0.7),
                                            offset: iced::Vector::new(0.0, 0.0),
                                            blur_radius: 10.0,
                                        },
                                        ..Default::default()
                                    }),
                                text("Searching Modrinth...")
                                    .size(14)
                                    .bold()
                                    .color(Color::WHITE),
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                            text(format!(
                                "Finding compatible {} for Minecraft {} ({})",
                                launcher.content_kind.label(),
                                instance.version,
                                instance.loader
                            ))
                            .size(12)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
                        ]
                        .spacing(8)
                        .align_x(Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .padding(40)
                    .align_x(Horizontal::Center)
                    .style(|_| subtle_panel_style())
                    .into()
                } else if launcher.content_results.is_empty() {
                    container(
                        column![
                            text("No projects found")
                                .size(14)
                                .bold()
                                .color(Color::WHITE),
                            text("Search Modrinth by name or keyword to find compatible content.")
                                .size(11)
                                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.48)),
                        ]
                        .spacing(6)
                        .align_x(Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .padding(30)
                    .into()
                } else {
                    results.into()
                };

            let browse_list = scrollable(results_body)
                .id(iced::widget::Id::new("browse_mods_scrollable"))
                .height(Length::Fill)
                .style(|_theme, _status| custom_scrollbar_style());

            column![
                kind_tabs_row,
                provider_row,
                search_row,
                container(
                    row![
                        container(iced::widget::Space::new().width(6).height(6))
                            .width(6)
                            .height(6)
                            .style(|_| crate::style::status_dot_style(launcher.content_searching)),
                        text(if launcher.content_installing_project_id.is_some() {
                            &launcher.content_install_phase
                        } else {
                            &launcher.content_status
                        })
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding(Padding::new(7.0).left(10.0).right(10.0))
                .style(|_| crate::style::inset_panel_style()),
                browse_list,
            ]
            .spacing(12)
            .into()
        }
    };

    container(column![header_row, tabs, main_content, view_launch_strip(launcher),].spacing(14))
        .width(if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
            Length::Fill
        } else {
            Length::FillPortion(10)
        })
        .height(Length::Fill)
        .padding(16)
        .style(|_| glass_panel_style())
        .into()
}

/// Per-instance settings: memory allocation and Java runtime override.
fn view_instance_settings(launcher: &RixLauncher) -> Element<'_, Message> {
    let Some(instance) = launcher.selected_instance() else {
        return empty_panel("No instance selected", "Select an instance first.", true).into();
    };
    let instance = instance.clone();

    let using_default_memory = instance.memory_gb.is_none();
    let effective_memory = launcher.effective_memory_gb(&instance);

    // ── Memory card ────────────────────────────────────────────────────────
    let mem_color = if effective_memory <= 8.0 {
        ACCENT_GREEN
    } else if effective_memory <= 12.0 {
        Color::from_rgb(0.95, 0.80, 0.20)
    } else {
        Color::from_rgb(1.0, 0.40, 0.18)
    };
    let mem_pct = ((effective_memory - constants::MIN_MEMORY_GB)
        / (constants::MAX_MEMORY_GB - constants::MIN_MEMORY_GB))
        .clamp(0.0, 1.0);

    let memory_bar = container(row![
        container(
            iced::widget::Space::new()
                .width(Length::FillPortion((mem_pct * 100.0) as u16))
                .height(6),
        )
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(mem_color)),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
        iced::widget::Space::new().width(Length::Fill).height(6),
    ])
    .width(Length::Fill)
    .height(6)
    .style(|_| subtle_panel_style());

    let scope_note = if using_default_memory {
        format!("Using launcher default ({:.0} GB)", launcher.memory_gb)
    } else {
        "Instance override active".to_string()
    };

    let memory_card = container(
        column![
            row![
                text("Memory Allocation")
                    .size(12)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
                horizontal_space(Length::Fill),
                text(format!("{:.0} GB", effective_memory))
                    .size(14)
                    .bold()
                    .color(mem_color),
            ]
            .align_y(Alignment::Center),
            memory_bar,
            slider(
                constants::MIN_MEMORY_GB..=constants::MAX_MEMORY_GB,
                effective_memory,
                Message::InstanceMemoryChanged,
            )
            .step(1.0_f32),
            row![
                text(scope_note).size(10).color(Color::from_rgba(
                    1.0,
                    1.0,
                    1.0,
                    if using_default_memory {
                        0.38
                    } else {
                        ACCENT_GREEN.a * 0.9 + 0.1
                    }
                )),
                horizontal_space(Length::Fill),
                Button::new(text("Use default").size(10).bold())
                    .on_press(Message::InstanceMemoryReset)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(4.0).left(8.0).right(8.0)),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Length::Fill)
    .style(|_| subtle_panel_style());

    // ── Java runtime card ──────────────────────────────────────────────────
    let mut java_options = vec![constants::AUTO_JAVA_LABEL.to_string()];
    java_options.extend(launcher.java_labels());

    let selected_java_label = match &instance.java_path {
        Some(path) => launcher
            .java_runtimes
            .iter()
            .find(|r| &r.path == path)
            .map(|r| r.label.clone())
            .unwrap_or_else(|| format!("Java - {path}")),
        None => constants::AUTO_JAVA_LABEL.to_string(),
    };

    let required_major = launcher.required_java_major();
    let java_hint = match &instance.java_path {
        Some(path) => launcher
            .java_runtimes
            .iter()
            .find(|runtime| runtime.path == *path)
            .map(|runtime| {
                if runtime.major < required_major {
                    format!(
                        "⚠ Selected Java {} is too old — this version needs Java {required_major}+",
                        runtime.major
                    )
                } else {
                    "Instance-specific Java runtime".to_string()
                }
            })
            .unwrap_or_else(|| "The runtime will be validated before launch".to_string()),
        None => "The launcher picks the best installed Java automatically".to_string(),
    };

    let java_card = container(
        column![
            row![
                text("Java Runtime")
                    .size(12)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
                horizontal_space(Length::Fill),
                text(format!("Required: Java {required_major}+"))
                    .size(10)
                    .bold()
                    .color(Color::from_rgba(0.55, 0.85, 0.60, 0.9)),
            ]
            .align_y(Alignment::Center),
            pick_list(
                java_options,
                Some(selected_java_label),
                Message::InstanceJavaSelected
            )
            .style(|_theme, status| pick_list_style(status))
            .menu_style(custom_menu_style)
            .width(Length::Fill),
            text(java_hint)
                .size(10)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.38)),
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Length::Fill)
    .style(|_| subtle_panel_style());

    // ── Layout ─────────────────────────────────────────────────────────────
    let subtitle = format!(
        "{} · {} / {}",
        instance.name, instance.version, instance.loader
    );
    let runtime_area: Element<'_, Message> =
        if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
            column![memory_card, java_card]
                .spacing(12)
                .width(Length::Fill)
                .into()
        } else {
            row![
                memory_card.width(Length::FillPortion(1)),
                java_card.width(Length::FillPortion(1)),
            ]
            .spacing(12)
            .width(Length::Fill)
            .into()
        };
    container(
        column![
            text("Instance Settings")
                .size(15)
                .bold()
                .color(Color::WHITE),
            text(subtitle)
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.46)),
            runtime_area,
            text(
                "These settings apply only to this instance and are saved in its launch.properties."
            )
            .size(10)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.35)),
        ]
        .spacing(12)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn view_account_manager(launcher: &RixLauncher) -> Element<'_, Message> {
    let mut accounts = Column::new().spacing(8);
    for (index, account) in launcher.accounts.iter().enumerate() {
        let active = index == launcher.active_account;

        let account_type_label = if account.is_microsoft {
            container(text("Microsoft").size(9).bold().color(Color::WHITE))
                .padding(Padding::new(2.0).left(6.0).right(6.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        0.2, 0.6, 0.2, 0.4,
                    ))),
                    border: iced::Border {
                        color: Color::from_rgba(0.2, 0.6, 0.2, 0.6),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        } else {
            container(
                text("Offline")
                    .size(9)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.5)),
            )
            .padding(Padding::new(2.0).left(6.0).right(6.0))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    1.0, 1.0, 1.0, 0.05,
                ))),
                border: iced::Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
        };

        let name_for_avatar = account.name.clone();
        let avatar: Element<'_, Message> =
            if let Some(handle) = launcher.player_skins.get(&account.name) {
                container(image(handle.clone()).width(34).height(34))
                    .clip(true)
                    .style(move |_| avatar_style_for_name(&name_for_avatar))
                    .into()
            } else {
                let name_for_text = account.name.clone();
                container(
                    text(crate::launcher::utils::player_initials(&account.name))
                        .size(13)
                        .bold()
                        .color(Color::WHITE),
                )
                .width(34)
                .height(34)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(move |_| avatar_style_for_name(&name_for_text))
                .into()
            };

        let item = Button::new(
            row![
                avatar,
                column![
                    text(&account.name).size(13).bold().color(Color::WHITE),
                    account_type_label,
                ]
                .spacing(2)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectAccount(index))
        .style(move |_theme, status| instance_button_style(status, active))
        .width(Length::Fill)
        .padding(10);

        accounts = accounts.push(
            iced::widget::mouse_area(item).on_right_press(Message::AccountRightClicked(index)),
        );
    }

    let mut account_actions = Column::new().spacing(8);
    account_actions = account_actions.push(
        Button::new(text("Add Offline Account").size(12).bold())
            .on_press(Message::AddOfflineAccount)
            .style(|_t, status| action_button_style(status))
            .width(Length::Fill)
            .padding(10),
    );
    account_actions = account_actions.push(
        Button::new(text("Add Microsoft Account").size(12).bold())
            .on_press(Message::StartMicrosoftLogin)
            .style(|_t, status| play_button_style(status))
            .width(Length::Fill)
            .padding(10),
    );
    if launcher
        .active_account_object()
        .is_some_and(|account| account.is_microsoft)
    {
        account_actions = account_actions.push(
            Button::new(
                text(if launcher.microsoft_refreshing {
                    "Refreshing Microsoft session…"
                } else {
                    "Refresh Microsoft session"
                })
                .size(11)
                .bold(),
            )
            .on_press_maybe(
                (!launcher.microsoft_refreshing).then_some(Message::RefreshMicrosoftAccount),
            )
            .style(|_t, status| outline_button_style(status))
            .width(Length::Fill)
            .padding(9),
        );
    }

    container(
        iced::widget::column![
            section_title("Accounts", "Choose who will play"),
            scrollable(accounts)
                .height(Length::Fill)
                .style(|_theme, _status| custom_scrollbar_style()),
            text_input("Offline name", &launcher.new_account_name)
                .on_input(Message::NewAccountNameChanged)
                .on_submit(Message::AddOfflineAccount)
                .style(|_theme, status| text_input_style(status))
                .padding(10)
                .size(13),
            account_actions,
        ]
        .spacing(12),
    )
    .width(if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
        Length::Fill
    } else {
        Length::FillPortion(5)
    })
    .height(if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
        Length::Fixed(constants::ACCOUNT_PANEL_COMPACT_HEIGHT)
    } else {
        Length::Fill
    })
    .padding(16)
    .style(|_| glass_panel_style())
    .into()
}

fn view_microsoft_login_modal(launcher: &RixLauncher) -> Element<'_, Message> {
    let Some(state) = launcher.microsoft_login.as_ref() else {
        return iced::widget::Space::new().into();
    };

    let code_area = if !state.user_code.is_empty() {
        column![
            text("1. Visit verification page:")
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
            row![
                text(&state.verification_uri)
                    .size(13)
                    .bold()
                    .color(Color::from_rgba(0.2, 0.7, 1.0, 1.0)),
                horizontal_space(10),
                Button::new(text("Open").size(11).bold())
                    .on_press(Message::OpenMicrosoftVerificationUri(
                        state.verification_uri.clone()
                    ))
                    .style(|_theme, status| play_button_style(status))
                    .padding(Padding::new(6.0).left(12.0).right(12.0))
            ]
            .align_y(Alignment::Center),
            vertical_space(4),
            text("2. Enter this code:")
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
            row![
                container(text(&state.user_code).size(26).bold().color(Color::WHITE))
                    .padding(10)
                    .style(|_| subtle_panel_style())
                    .align_x(Horizontal::Center)
                    .width(Length::FillPortion(7)),
                horizontal_space(8),
                Button::new(text("Copy").size(12).bold())
                    .on_press(Message::CopyMicrosoftUserCode)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(12.0).left(16.0).right(16.0))
                    .height(Length::Fixed(48.0))
                    .width(Length::FillPortion(3))
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8)
    } else {
        column![
            text("Requesting login code from Microsoft...")
                .size(13)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
        ]
    };

    let dialog = container(
        column![
            text("Microsoft Auth").size(20).bold().color(Color::WHITE),
            text("Log in with your official Mojang/Microsoft account to play Minecraft on public multiplayer servers.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            vertical_space(4),
            code_area,
            vertical_space(4),
            row![
                text("Status:").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                text(&state.status).size(11).bold().color(Color::WHITE),
            ]
            .spacing(6),
            row![
                horizontal_space(Length::Fill),
                Button::new(text("Cancel").size(13).bold())
                    .on_press(Message::CloseMicrosoftLogin)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(10.0).left(18.0).right(18.0)),
            ]
        ]
        .spacing(12),
    )
    .width(420)
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
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.45,
        ))),
        ..Default::default()
    })
    .into()
}

fn view_launch_strip(launcher: &RixLauncher) -> Element<'_, Message> {
    let selected = launcher.selected_instance();
    let launch_text = selected
        .map(|instance| {
            if instance.has_version_files {
                format!(
                    "{} ·  {} / {}",
                    instance.name, instance.version, instance.loader
                )
            } else {
                format!("{} — needs version files", instance.name)
            }
        })
        .unwrap_or_else(|| "Select an instance".to_string());

    let btn = if let Some(instance_id) = launcher.selected_instance_id {
        if launcher.launching_instance_id == Some(instance_id) {
            Button::new(
                row![
                    container(iced::widget::Space::new().width(8).height(8))
                        .width(8)
                        .height(8)
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(ACCENT_ORANGE)),
                            border: Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    text("Preparing assets…")
                        .size(12)
                        .bold()
                        .color(Color::from_rgba(1.0, 0.80, 0.3, 1.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .style(|_t, status| {
                let mut s = outline_button_style(status);
                s.text_color = Color::from_rgba(1.0, 0.80, 0.3, 1.0);
                s.border.color = Color::from_rgba(1.0, 0.36, 0.20, 0.60);
                s
            })
            .padding(Padding::new(10.0).left(18.0).right(18.0))
        } else if launcher.active_processes.contains_key(&instance_id) {
            // ── Running state with uptime timer ────────────────────────────────
            // Format uptime from launch_times map
            let uptime = launcher
                .instance_launch_times
                .get(&instance_id)
                .map(|t| {
                    let secs = t.elapsed().as_secs();
                    format!("{:02}:{:02}", secs / 60, secs % 60)
                })
                .unwrap_or_default();
            let label = if uptime.is_empty() {
                "Stop".to_string()
            } else {
                format!("Stop · {}", uptime)
            };
            Button::new(text(label).size(12).bold().color(Color::WHITE))
                .on_press(Message::StopInstance(instance_id))
                .style(|_t, status| {
                    let mut s = action_button_style(status);
                    s.background = Some(iced::Background::Color(Color::from_rgb(0.72, 0.22, 0.22)));
                    s.text_color = Color::WHITE;
                    s.border.color = Color::from_rgba(0.9, 0.2, 0.2, 0.55);
                    s
                })
                .padding(Padding::new(10.0).left(18.0).right(18.0))
        } else {
            Button::new(
                row![
                    svg(svg_handle(SVG_PLAY))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style {
                            color: Some(Color::WHITE),
                        }),
                    text("Launch").size(13).bold().color(Color::WHITE),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(Message::PlayPressed)
            .style(|_t, status| play_button_style(status))
            .padding(Padding::new(10.0).left(22.0).right(22.0))
        }
    } else {
        Button::new(
            row![
                svg(svg_handle(SVG_PLAY))
                    .width(13)
                    .height(13)
                    .style(|_, _| svg::Style {
                        color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
                    }),
                text("Launch").size(13).bold().color(Color::from_rgba(1.0, 1.0, 1.0, 0.4)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .style(|_t, status| outline_button_style(status))
        .padding(Padding::new(10.0).left(22.0).right(22.0))
    };

    let strip_content: Element<'_, Message> =
        if launcher.window_width < constants::STACKED_LAYOUT_WIDTH {
            column![
                text(launch_text)
                    .size(13)
                    .bold()
                    .color(Color::WHITE)
                    .width(Length::Fill),
                btn,
            ]
            .spacing(8)
            .into()
        } else {
            row![
                text(launch_text)
                    .size(13)
                    .bold()
                    .color(Color::WHITE)
                    .width(Length::Fill),
                horizontal_space(Length::Fill),
                btn,
            ]
            .align_y(Alignment::Center)
            .into()
        };

    container(strip_content)
        .padding(12)
        .style(|_| hero_panel_style())
        .into()
}

fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn project_is_installed(
    details: &crate::launcher::types::ModrinthProjectDetails,
    installed_project_ids: &std::collections::HashSet<String>,
    installed_files: &[String],
) -> bool {
    let dummy = crate::launcher::types::ContentItem {
        id: details.id.clone(),
        slug: details.slug.clone(),
        title: details.title.clone(),
        description: details.description.clone(),
        project_type: String::new(),
        icon_url: details.icon_url.clone(),
    };
    content_is_installed(&dummy, installed_project_ids, installed_files)
}

fn unescape_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
}

fn extract_html_attr(tag: &str, attr: &str) -> Option<String> {
    let key = format!("{}=", attr);
    let lower = tag.to_ascii_lowercase();
    let idx = lower.find(&key)?;
    let after_eq = &tag[idx + key.len()..];
    let trimmed = after_eq.trim_start();
    let quote = trimmed.chars().next()?;
    if quote == '"' || quote == '\'' {
        let content = &trimmed[1..];
        let end_idx = content.find(quote)?;
        Some(content[..end_idx].to_string())
    } else {
        let end_idx = trimmed
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(trimmed.len());
        Some(trimmed[..end_idx].to_string())
    }
}

fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag_content = String::new();
            for inner in chars.by_ref() {
                if inner == '>' {
                    break;
                }
                tag_content.push(inner);
            }
            let tag_trim = tag_content.trim();
            let tag_lower = tag_trim.to_ascii_lowercase();

            if tag_lower == "br" || tag_lower == "br/" || tag_lower.starts_with("br ") {
                result.push('\n');
            } else if tag_lower == "hr" || tag_lower == "hr/" || tag_lower.starts_with("hr ") {
                result.push_str("\n---\n");
            } else if tag_lower.starts_with("h1") {
                result.push_str("\n# ");
            } else if tag_lower == "/h1" {
                result.push('\n');
            } else if tag_lower.starts_with("h2") {
                result.push_str("\n## ");
            } else if tag_lower == "/h2" {
                result.push('\n');
            } else if tag_lower.starts_with("h3")
                || tag_lower.starts_with("h4")
                || tag_lower.starts_with("h5")
                || tag_lower.starts_with("h6")
            {
                result.push_str("\n### ");
            } else if tag_lower == "/h3"
                || tag_lower == "/h4"
                || tag_lower == "/h5"
                || tag_lower == "/h6"
            {
                result.push('\n');
            } else if tag_lower.starts_with("p") || tag_lower.starts_with("p ") {
                result.push('\n');
            } else if tag_lower == "/p" {
                result.push('\n');
            } else if tag_lower.starts_with("li") || tag_lower.starts_with("li ") {
                result.push_str("\n- ");
            } else if tag_lower == "/li" {
                result.push('\n');
            } else if tag_lower.starts_with("img") {
                if let Some(alt) = extract_html_attr(tag_trim, "alt") {
                    let alt_trim = alt.trim();
                    let alt_lower = alt_trim.to_ascii_lowercase();
                    let is_badge_or_banner = alt_lower.contains("banner")
                        || alt_lower.contains("badge")
                        || alt_lower.contains("button")
                        || alt_lower.contains("logo")
                        || alt_lower.contains("icon")
                        || alt_lower.contains("discord")
                        || alt_lower.contains("github")
                        || alt_lower.contains("patreon")
                        || alt_lower.contains("ko-fi")
                        || alt_lower.contains("paypal")
                        || alt_lower.contains("modrinth")
                        || alt_lower.contains("curseforge")
                        || alt_lower.is_empty();

                    if !is_badge_or_banner {
                        result.push_str(&format!("\n## {}\n", alt_trim));
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn clean_markdown_inline(s: &str) -> String {
    let unescaped = unescape_html_entities(s);
    let mut result = String::with_capacity(unescaped.len());
    let mut chars = unescaped.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '!' && chars.peek() == Some(&'[') {
            // Markdown image: ![alt](url)
            chars.next(); // consume '['
            let mut alt = String::new();
            for inner in chars.by_ref() {
                if inner == ']' {
                    break;
                }
                alt.push(inner);
            }
            if chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                for inner in chars.by_ref() {
                    if inner == ')' {
                        break;
                    }
                }
            }
            let alt_trim = alt.trim();
            let alt_lower = alt_trim.to_ascii_lowercase();
            let is_badge_or_banner = alt_lower.contains("banner")
                || alt_lower.contains("badge")
                || alt_lower.contains("button")
                || alt_lower.contains("logo")
                || alt_lower.contains("icon")
                || alt_lower.contains("discord")
                || alt_lower.contains("github")
                || alt_lower.contains("patreon")
                || alt_lower.contains("ko-fi")
                || alt_lower.contains("paypal")
                || alt_lower.contains("modrinth")
                || alt_lower.contains("curseforge")
                || alt_lower.is_empty();

            if !is_badge_or_banner {
                result.push_str(alt_trim);
            }
        } else if c == '[' {
            // Markdown link: [text](url)
            let mut link_text = String::new();
            let mut found_close = false;
            for inner in chars.by_ref() {
                if inner == ']' {
                    found_close = true;
                    break;
                }
                link_text.push(inner);
            }
            if found_close && chars.peek() == Some(&'(') {
                chars.next();
                for inner in chars.by_ref() {
                    if inner == ')' {
                        break;
                    }
                }
                result.push_str(&clean_markdown_inline(&link_text));
            } else {
                result.push('[');
                result.push_str(&link_text);
                if found_close {
                    result.push(']');
                }
            }
        } else if c == '*' || c == '_' || c == '`' || c == '~' {
            continue;
        } else {
            result.push(c);
        }
    }

    result
}

fn render_markdown_body<'a>(body: &'a str) -> Element<'a, Message> {
    let cleaned_html = strip_html_tags(body);
    let mut col = Column::new().spacing(8).width(Length::Fill);
    let mut in_code_block = false;
    let mut code_lines = Vec::new();

    for raw_line in cleaned_html.lines() {
        let trimmed = raw_line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                let code_text = code_lines.join("\n");
                col = col.push(
                    container(
                        text(code_text)
                            .size(11)
                            .color(Color::from_rgba(0.9, 0.9, 0.9, 0.9)),
                    )
                    .padding(10)
                    .width(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
                        border: Border {
                            radius: 6.0.into(),
                            color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
                );
                code_lines.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_lines.push(raw_line);
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Horizontal divider
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            col = col.push(
                container(iced::widget::Space::new().width(Length::Fill).height(1))
                    .width(Length::Fill)
                    .height(1)
                    .padding(Padding::new(4.0).top(6.0).bottom(6.0))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
                        ..Default::default()
                    }),
            );
            continue;
        }

        if let Some(h1) = trimmed.strip_prefix("# ") {
            let h_clean = clean_markdown_inline(h1);
            if !h_clean.trim().is_empty() {
                col = col.push(
                    text(h_clean)
                        .size(18)
                        .bold()
                        .color(Color::WHITE),
                );
            }
        } else if let Some(h2) = trimmed.strip_prefix("## ") {
            let h_clean = clean_markdown_inline(h2);
            if !h_clean.trim().is_empty() {
                col = col.push(
                    text(h_clean)
                        .size(15)
                        .bold()
                        .color(Color::from_rgba(0.70, 0.90, 1.0, 0.95)),
                );
            }
        } else if let Some(h3) = trimmed.strip_prefix("### ") {
            let h_clean = clean_markdown_inline(h3);
            if !h_clean.trim().is_empty() {
                col = col.push(
                    text(h_clean)
                        .size(13)
                        .bold()
                        .color(Color::WHITE),
                );
            }
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            let q_clean = clean_markdown_inline(quote);
            if !q_clean.trim().is_empty() {
                col = col.push(
                    container(
                        text(q_clean)
                            .size(12)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
                    )
                    .padding(Padding::new(8.0).left(14.0).right(10.0))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
                        border: Border {
                            radius: 4.0.into(),
                            color: Color::from_rgba(0.27, 0.85, 0.46, 0.60),
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
                );
            }
        } else if let Some(bullet) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            let b_clean = clean_markdown_inline(bullet);
            if !b_clean.trim().is_empty() {
                col = col.push(
                    row![
                        text("•").size(12).bold().color(ACCENT_GREEN),
                        text(b_clean)
                            .size(12)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.82)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
        } else {
            let p_clean = clean_markdown_inline(trimmed);
            if !p_clean.trim().is_empty() {
                col = col.push(
                    text(p_clean)
                        .size(12)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.80)),
                );
            }
        }
    }

    col.into()
}

fn tab_button_style(
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    if active {
        iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, 0.22))),
            text_color: ACCENT_GREEN,
            border: Border {
                radius: 16.0.into(),
                color: ACCENT_GREEN,
                width: 1.5,
            },
            ..Default::default()
        }
    } else {
        let bg_alpha = match status {
            iced::widget::button::Status::Hovered => 0.12,
            iced::widget::button::Status::Pressed => 0.18,
            _ => 0.05,
        };
        let text_alpha = match status {
            iced::widget::button::Status::Hovered => 0.95,
            _ => 0.65,
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, bg_alpha))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, text_alpha),
            border: Border {
                radius: 16.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
            },
            ..Default::default()
        }
    }
}

fn link_button_style(status: iced::widget::button::Status) -> iced::widget::button::Style {
    let (bg_alpha, text_alpha) = match status {
        iced::widget::button::Status::Hovered => (0.10, 1.0),
        iced::widget::button::Status::Pressed => (0.16, 1.0),
        _ => (0.04, 0.80),
    };
    iced::widget::button::Style {
        background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, bg_alpha))),
        text_color: Color::from_rgba(1.0, 1.0, 1.0, text_alpha),
        border: Border {
            radius: 6.0.into(),
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn view_mod_details<'a>(
    launcher: &'a RixLauncher,
    instance: &'a crate::launcher::types::MinecraftInstance,
) -> Element<'a, Message> {
    if launcher.loading_project_details {
        let pulse = 0.5 + 0.5 * (launcher.anim_time / 400.0).sin().abs();
        return container(
            column![
                row![
                    container(iced::widget::Space::new().width(10).height(10))
                        .width(10)
                        .height(10)
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, pulse))),
                            border: Border {
                                radius: 5.0.into(),
                                ..Default::default()
                            },
                            shadow: iced::Shadow {
                                color: Color::from_rgba(0.27, 0.85, 0.46, pulse * 0.8),
                                offset: iced::Vector::new(0.0, 0.0),
                                blur_radius: 12.0,
                            },
                            ..Default::default()
                        }),
                    text("Loading project details from Modrinth...")
                        .size(14)
                        .bold()
                        .color(Color::WHITE),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                Button::new(text("< Back to Search").size(12).bold())
                    .on_press(Message::CloseProjectDetails)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(9.0).left(16.0).right(16.0)),
            ]
            .spacing(16)
            .align_x(Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(50)
        .align_x(Horizontal::Center)
        .style(|_| subtle_panel_style())
        .into();
    }

    if let Some(err) = &launcher.project_details_error {
        return container(
            column![
                text("Failed to load project details")
                    .size(15)
                    .bold()
                    .color(Color::from_rgb(0.95, 0.35, 0.35)),
                text(err)
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
                Button::new(text("< Back to Search").size(12).bold())
                    .on_press(Message::CloseProjectDetails)
                    .style(|_theme, status| outline_button_style(status))
                    .padding(Padding::new(9.0).left(16.0).right(16.0)),
            ]
            .spacing(12)
            .align_x(Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(40)
        .align_x(Horizontal::Center)
        .style(|_| subtle_panel_style())
        .into();
    }

    let Some(details) = &launcher.project_details else {
        return container(text("No project selected").size(13).color(Color::WHITE))
            .width(Length::Fill)
            .into();
    };

    let installed_project_ids =
        crate::launcher::installed_content_project_ids(instance, launcher.content_kind);
    let installed_files = get_installed_files(instance, launcher.content_kind, "");

    let is_installing = launcher.content_installing_instance_id == Some(instance.id)
        && launcher.content_installing_project_id.as_deref() == Some(details.id.as_str());

    let is_installed = project_is_installed(details, &installed_project_ids, &installed_files);

    // Top action bar
    let back_btn = Button::new(text("< Back to Search").size(12).bold())
        .on_press(Message::CloseProjectDetails)
        .style(|_theme, status| outline_button_style(status))
        .padding(Padding::new(9.0).left(16.0).right(16.0));

    let top_install_btn: Element<'_, Message> = if is_installing {
        container(
            row![
                text("Downloading").size(12).bold(),
                text(format!("{:.0}%", launcher.content_install_progress * 100.0))
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(Padding::new(9.0).left(18.0).right(18.0))
        .style(|_| crate::style::status_panel_style(0.30, 0.62, 1.0))
        .into()
    } else if is_installed {
        container(
            row![
                text("✓").size(14).bold().color(ACCENT_GREEN),
                text("Downloaded").size(12).bold().color(Color::from_rgba(0.65, 1.0, 0.72, 0.95)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(Padding::new(9.0).left(18.0).right(18.0))
        .style(|_| crate::style::status_panel_style(0.27, 0.78, 0.46))
        .into()
    } else {
        Button::new(text("Install to Instance").size(12).bold())
            .on_press(Message::InstallContentDirect(details.id.clone()))
            .style(|_theme, status| play_button_style(status))
            .padding(Padding::new(9.0).left(20.0).right(20.0))
            .into()
    };

    let top_bar = row![back_btn, horizontal_space(Length::Fill), top_install_btn]
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // Hero Section
    let icon_elem: Element<'_, Message> = if let Some(url) = &details.icon_url {
        if let Some(handle) = launcher.icon_cache.get(url) {
            container(image(handle.clone()).width(64).height(64))
                .clip(true)
                .style(|_| container::Style {
                    border: Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        } else {
            let title_for_placeholder = details.title.clone();
            container(
                text("…")
                    .size(16)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
            )
            .width(64)
            .height(64)
            .center_x(Length::Fixed(64.0))
            .center_y(Length::Fixed(64.0))
            .style(move |_| avatar_style_for_name(&title_for_placeholder))
            .into()
        }
    } else {
        let letter = details
            .title
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase()
            .to_string();
        let title_clone = details.title.clone();
        container(text(letter).size(22).bold().color(Color::WHITE))
            .width(64)
            .height(64)
            .center_x(Length::Fixed(64.0))
            .center_y(Length::Fixed(64.0))
            .style(move |_| avatar_style_for_name(&title_clone))
            .into()
    };

    let mut meta_badges = row![].spacing(8).align_y(Alignment::Center);

    if let Some(downloads) = details.downloads {
        meta_badges = meta_badges.push(
            container(
                row![
                    text("↓").size(12).bold().color(ACCENT_GREEN),
                    text(format!("{} downloads", format_count(downloads)))
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .padding(Padding::new(4.0).left(10.0).right(10.0))
            .style(|_| crate::style::inset_panel_style()),
        );
    }

    if let Some(followers) = details.followers {
        meta_badges = meta_badges.push(
            container(
                row![
                    text("★").size(12).bold().color(Color::from_rgb(1.0, 0.85, 0.3)),
                    text(format!("{} followers", format_count(followers)))
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .padding(Padding::new(4.0).left(10.0).right(10.0))
            .style(|_| crate::style::inset_panel_style()),
        );
    }

    if let Some(categories) = &details.categories {
        for cat in categories.iter().take(3) {
            meta_badges = meta_badges.push(
                container(
                    text(cat.to_uppercase())
                        .size(9)
                        .bold()
                        .color(Color::from_rgba(0.70, 0.88, 1.0, 0.90)),
                )
                .padding(Padding::new(3.0).left(8.0).right(8.0))
                .style(|_| crate::style::status_panel_style(0.30, 0.60, 0.95)),
            );
        }
    }

    let modrinth_url = format!("https://modrinth.com/mod/{}", details.slug);
    let web_btn = Button::new(
        row![
            text("Modrinth Page").size(11).bold(),
            text("↗").size(12).bold(),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .on_press(Message::OpenModrinthUrl(modrinth_url))
    .style(|_theme, status| outline_button_style(status))
    .padding(Padding::new(4.0).left(10.0).right(10.0));

    meta_badges = meta_badges.push(web_btn);

    let hero_info = column![
        text(&details.title).size(22).bold().color(Color::WHITE),
        text(&details.description)
            .size(13)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)),
        meta_badges,
    ]
    .spacing(8)
    .width(Length::Fill);

    let hero_container = container(
        row![icon_elem, hero_info]
            .spacing(16)
            .align_y(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| subtle_panel_style());

    // Navigation Tabs
    let gallery_count = details.gallery.as_ref().map(|g| g.len()).unwrap_or(0);
    let gallery_label = if gallery_count > 0 {
        format!("Gallery ({})", gallery_count)
    } else {
        "Gallery".to_string()
    };

    let desc_active = launcher.mod_details_tab == ModDetailsTab::Description;
    let desc_tab_btn = Button::new(text("Description").size(12).bold())
        .on_press(Message::ModDetailsTabSelected(ModDetailsTab::Description))
        .style(move |_theme, status| tab_button_style(status, desc_active))
        .padding(Padding::new(7.0).left(16.0).right(16.0));

    let gall_active = launcher.mod_details_tab == ModDetailsTab::Gallery;
    let gallery_tab_btn = Button::new(text(gallery_label).size(12).bold())
        .on_press(Message::ModDetailsTabSelected(ModDetailsTab::Gallery))
        .style(move |_theme, status| tab_button_style(status, gall_active))
        .padding(Padding::new(7.0).left(16.0).right(16.0));

    let tabs_row = row![desc_tab_btn, gallery_tab_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    // Left Column Content: Description or Gallery
    let left_content: Element<'_, Message> = match launcher.mod_details_tab {
        ModDetailsTab::Description => {
            if let Some(body) = &details.body {
                if !body.trim().is_empty() {
                    container(render_markdown_body(body))
                        .padding(18)
                        .width(Length::Fill)
                        .style(|_| subtle_panel_style())
                        .into()
                } else {
                    container(
                        text("No description provided for this project.")
                            .size(12)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    )
                    .padding(30)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .style(|_| subtle_panel_style())
                    .into()
                }
            } else {
                container(
                    text("No description provided for this project.")
                        .size(12)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                )
                .padding(30)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .style(|_| subtle_panel_style())
                .into()
            }
        }
        ModDetailsTab::Gallery => {
            if let Some(gallery) = &details.gallery {
                if !gallery.is_empty() {
                    let active_idx = launcher
                        .selected_gallery_index
                        .min(gallery.len().saturating_sub(1));
                    let active_item = &gallery[active_idx];

                    let main_preview: Element<'_, Message> =
                        if let Some(handle) = launcher.gallery_image_cache.get(&active_item.url) {
                            container(
                                image(handle.clone())
                                    .width(Length::Fill)
                                    .height(Length::Fixed(360.0))
                                    .content_fit(iced::ContentFit::Contain),
                            )
                            .width(Length::Fill)
                            .height(360)
                            .center_x(Length::Fill)
                            .center_y(Length::Fixed(360.0))
                            .clip(true)
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
                                border: Border {
                                    radius: 8.0.into(),
                                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                                    width: 1.0,
                                },
                                ..Default::default()
                            })
                            .into()
                        } else {
                            let pulse = 0.5 + 0.5 * (launcher.anim_time / 400.0).sin().abs();
                            container(
                                row![
                                    container(iced::widget::Space::new().width(8).height(8))
                                        .width(8)
                                        .height(8)
                                        .style(move |_| container::Style {
                                            background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, pulse))),
                                            border: Border {
                                                radius: 4.0.into(),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        }),
                                    text("Loading screenshot...")
                                        .size(12)
                                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
                                ]
                                .spacing(10)
                                .align_y(Alignment::Center),
                            )
                            .width(Length::Fill)
                            .height(240)
                            .center_x(Length::Fill)
                            .center_y(Length::Fixed(240.0))
                            .style(|_| crate::style::inset_panel_style())
                            .into()
                        };

                    let mut caption_col = Column::new().spacing(4);
                    if let Some(title) = &active_item.title {
                        if !title.is_empty() {
                            caption_col = caption_col.push(text(title).size(13).bold().color(Color::WHITE));
                        }
                    }
                    if let Some(desc) = &active_item.description {
                        if !desc.is_empty() {
                            caption_col = caption_col.push(
                                text(desc)
                                    .size(11)
                                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
                            );
                        }
                    }

                    let mut gallery_col = column![main_preview].spacing(10);
                    let caption_elem: Element<'_, Message> = caption_col.into();
                    gallery_col = gallery_col.push(caption_elem);

                    if gallery.len() > 1 {
                        let mut thumb_row = row![].spacing(8).align_y(Alignment::Center);
                        for (idx, item) in gallery.iter().enumerate() {
                            let is_active = idx == active_idx;
                            let thumb_content: Element<'_, Message> =
                                if let Some(handle) = launcher.gallery_image_cache.get(&item.url) {
                                    container(
                                        image(handle.clone())
                                            .width(76)
                                            .height(48)
                                            .content_fit(iced::ContentFit::Cover),
                                    )
                                    .clip(true)
                                    .into()
                                } else {
                                    container(
                                        text(format!("{}", idx + 1))
                                            .size(11)
                                            .bold()
                                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
                                    )
                                    .width(76)
                                    .height(48)
                                    .center_x(Length::Fixed(76.0))
                                    .center_y(Length::Fixed(48.0))
                                    .style(|_| crate::style::inset_panel_style())
                                    .into()
                                };

                            let thumb_btn = Button::new(thumb_content)
                                .on_press(Message::SelectGalleryImage(idx))
                                .style(move |_theme, status| {
                                    let mut s = outline_button_style(status);
                                    if is_active {
                                        s.border.color = ACCENT_GREEN;
                                        s.border.width = 2.0;
                                    }
                                    s.border.radius = 6.0.into();
                                    s
                                })
                                .padding(0);

                            thumb_row = thumb_row.push(thumb_btn);
                        }

                        let thumb_scroll = scrollable(thumb_row)
                            .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
                            .style(|_theme, _status| custom_scrollbar_style());

                        gallery_col = gallery_col.push(thumb_scroll);
                    }

                    container(gallery_col)
                        .padding(18)
                        .width(Length::Fill)
                        .style(|_| subtle_panel_style())
                        .into()
                } else {
                    container(
                        text("No gallery images available for this project.")
                            .size(12)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    )
                    .padding(30)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .style(|_| subtle_panel_style())
                    .into()
                }
            } else {
                container(
                    text("No gallery images available for this project.")
                        .size(12)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                )
                .padding(30)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .style(|_| subtle_panel_style())
                .into()
            }
        }
    };

    // Right Sidebar: Compatibility, Links, Tags
    let mut compat_col = column![
        text("Compatibility").size(13).bold().color(Color::WHITE),
        vertical_space(4),
        text("Minecraft: Java Edition")
            .size(11)
            .bold()
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
    ]
    .spacing(6);

    if let Some(versions) = &details.game_versions {
        if !versions.is_empty() {
            let mut shown_versions: Vec<String> = versions.iter().take(10).cloned().collect();
            if !shown_versions.contains(&instance.version) && versions.contains(&instance.version) {
                shown_versions.push(instance.version.clone());
            }

            let mut row_count = 0;
            let mut ver_rows = Column::new().spacing(6);
            let mut cur_row = row![].spacing(6);

            for ver in &shown_versions {
                let is_cur = ver == &instance.version;
                let badge: Element<'_, Message> = if is_cur {
                    container(
                        row![
                            text("✓").size(10).bold().color(ACCENT_GREEN),
                            text(ver.clone()).size(10).bold().color(Color::WHITE),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding(Padding::new(3.0).left(8.0).right(8.0))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, 0.20))),
                        border: Border {
                            radius: 4.0.into(),
                            color: ACCENT_GREEN,
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                    .into()
                } else {
                    container(text(ver.clone()).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)))
                        .padding(Padding::new(3.0).left(7.0).right(7.0))
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
                            border: Border {
                                radius: 4.0.into(),
                                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                                width: 1.0,
                            },
                            ..Default::default()
                        })
                        .into()
                };

                cur_row = cur_row.push(badge);
                row_count += 1;
                if row_count >= 3 {
                    ver_rows = ver_rows.push(cur_row);
                    cur_row = row![].spacing(6);
                    row_count = 0;
                }
            }
            if row_count > 0 {
                ver_rows = ver_rows.push(cur_row);
            }
            compat_col = compat_col.push(ver_rows);
        }
    }

    if let Some(loaders) = &details.loaders {
        if !loaders.is_empty() {
            compat_col = compat_col.push(vertical_space(4));
            compat_col = compat_col.push(
                text("Platforms")
                    .size(11)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
            );
            let mut loader_row = row![].spacing(6);
            for l in loaders {
                let is_cur = l.eq_ignore_ascii_case(&instance.loader);
                let loader_badge: Element<'_, Message> = if is_cur {
                    container(
                        row![
                            text("✓").size(10).bold().color(ACCENT_GREEN),
                            text(l).size(10).bold().color(Color::WHITE),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding(Padding::new(3.0).left(8.0).right(8.0))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(0.27, 0.85, 0.46, 0.20))),
                        border: Border {
                            radius: 4.0.into(),
                            color: ACCENT_GREEN,
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                    .into()
                } else {
                    let l_owned = l.clone();
                    container(text(l).size(10).bold())
                        .padding(Padding::new(3.0).left(8.0).right(8.0))
                        .style(move |_| loader_badge_style(&l_owned))
                        .into()
                };
                loader_row = loader_row.push(loader_badge);
            }
            compat_col = compat_col.push(loader_row);
        }
    }

    let mut env_row = row![].spacing(6);
    if let Some(client) = &details.client_side {
        if !client.is_empty() {
            let label = format!("Client: {}", client);
            env_row = env_row.push(
                container(text(label).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)))
                    .padding(Padding::new(3.0).left(7.0).right(7.0))
                    .style(|_| crate::style::inset_panel_style()),
            );
        }
    }
    if let Some(server) = &details.server_side {
        if !server.is_empty() {
            let label = format!("Server: {}", server);
            env_row = env_row.push(
                container(text(label).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)))
                    .padding(Padding::new(3.0).left(7.0).right(7.0))
                    .style(|_| crate::style::inset_panel_style()),
            );
        }
    }
    if details.client_side.is_some() || details.server_side.is_some() {
        compat_col = compat_col.push(vertical_space(4));
        compat_col = compat_col.push(
            text("Supported environments")
                .size(11)
                .bold()
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
        );
        compat_col = compat_col.push(env_row);
    }

    let compat_card = container(compat_col)
        .padding(14)
        .width(Length::Fill)
        .style(|_| subtle_panel_style());

    // Links Card
    let mut links_col = column![
        text("Links").size(13).bold().color(Color::WHITE),
        vertical_space(4),
    ]
    .spacing(6);

    let mut has_links = false;

    if let Some(url) = &details.issues_url {
        if !url.is_empty() {
            has_links = true;
            let url_clone = url.clone();
            links_col = links_col.push(
                Button::new(
                    row![
                        text("⚠").size(11).color(ACCENT_ORANGE),
                        text("Report issues").size(11).bold(),
                        horizontal_space(Length::Fill),
                        text("↗").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenModrinthUrl(url_clone))
                .style(|_t, status| link_button_style(status))
                .padding(Padding::new(6.0).left(10.0).right(10.0))
                .width(Length::Fill),
            );
        }
    }

    if let Some(url) = &details.source_url {
        if !url.is_empty() {
            has_links = true;
            let url_clone = url.clone();
            links_col = links_col.push(
                Button::new(
                    row![
                        text("<>").size(11).bold().color(Color::from_rgba(0.7, 0.9, 1.0, 0.9)),
                        text("View source").size(11).bold(),
                        horizontal_space(Length::Fill),
                        text("↗").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenModrinthUrl(url_clone))
                .style(|_t, status| link_button_style(status))
                .padding(Padding::new(6.0).left(10.0).right(10.0))
                .width(Length::Fill),
            );
        }
    }

    if let Some(url) = &details.discord_url {
        if !url.is_empty() {
            has_links = true;
            let url_clone = url.clone();
            links_col = links_col.push(
                Button::new(
                    row![
                        text("💬").size(11),
                        text("Join Discord").size(11).bold(),
                        horizontal_space(Length::Fill),
                        text("↗").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenModrinthUrl(url_clone))
                .style(|_t, status| link_button_style(status))
                .padding(Padding::new(6.0).left(10.0).right(10.0))
                .width(Length::Fill),
            );
        }
    }

    if let Some(url) = &details.wiki_url {
        if !url.is_empty() {
            has_links = true;
            let url_clone = url.clone();
            links_col = links_col.push(
                Button::new(
                    row![
                        text("📖").size(11),
                        text("Documentation").size(11).bold(),
                        horizontal_space(Length::Fill),
                        text("↗").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenModrinthUrl(url_clone))
                .style(|_t, status| link_button_style(status))
                .padding(Padding::new(6.0).left(10.0).right(10.0))
                .width(Length::Fill),
            );
        }
    }

    let links_card: Option<Element<'_, Message>> = if has_links {
        Some(
            container(links_col)
                .padding(14)
                .width(Length::Fill)
                .style(|_| subtle_panel_style())
                .into(),
        )
    } else {
        None
    };

    // Tags Card
    let tags_card: Option<Element<'_, Message>> = if let Some(categories) = &details.categories {
        if !categories.is_empty() {
            let mut tags_col = column![
                text("Tags").size(13).bold().color(Color::WHITE),
                vertical_space(4),
            ]
            .spacing(6);

            let mut tag_row = row![].spacing(6);
            let mut count = 0;
            let mut tag_rows = Column::new().spacing(6);

            for cat in categories {
                let tag_badge = container(
                    text(cat.to_uppercase())
                        .size(9)
                        .bold()
                        .color(Color::from_rgba(0.70, 0.88, 1.0, 0.90)),
                )
                .padding(Padding::new(3.0).left(8.0).right(8.0))
                .style(|_| crate::style::status_panel_style(0.30, 0.60, 0.95));

                tag_row = tag_row.push(tag_badge);
                count += 1;
                if count >= 2 {
                    tag_rows = tag_rows.push(tag_row);
                    tag_row = row![].spacing(6);
                    count = 0;
                }
            }
            if count > 0 {
                tag_rows = tag_rows.push(tag_row);
            }

            tags_col = tags_col.push(tag_rows);

            Some(
                container(tags_col)
                    .padding(14)
                    .width(Length::Fill)
                    .style(|_| subtle_panel_style())
                    .into(),
            )
        } else {
            None
        }
    } else {
        None
    };

    let mut sidebar_col = column![compat_card].spacing(12);
    if let Some(lc) = links_card {
        sidebar_col = sidebar_col.push(lc);
    }
    if let Some(tc) = tags_card {
        sidebar_col = sidebar_col.push(tc);
    }
    let sidebar: Element<'_, Message> = sidebar_col.into();

    let is_wide = launcher.window_width >= constants::STACKED_LAYOUT_WIDTH;
    let main_and_sidebar: Element<'_, Message> = if is_wide {
        row![
            container(left_content).width(Length::FillPortion(7)),
            container(sidebar).width(Length::FillPortion(3)),
        ]
        .spacing(14)
        .into()
    } else {
        column![left_content, sidebar].spacing(14).into()
    };

    let full_content = column![
        hero_container,
        tabs_row,
        main_and_sidebar,
    ]
    .spacing(14)
    .width(Length::Fill);

    let scrollable_content = scrollable(full_content)
        .id(iced::widget::Id::new("mod_details_scrollable"))
        .height(Length::Fill)
        .style(|_theme, _status| custom_scrollbar_style());

    column![top_bar, scrollable_content]
        .spacing(12)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_and_markdown_cleaning() {
        let input = r#"
![Entity Culling Banner](https://tr7zw.github.io/uikit/banner.png)
<p align="center" style="text-align: center;">
<a href="https://discord.gg/caW5eXekm"><img src="https://tr7zw.github.io/uikit/socialbuttonsicon/Discord-Button-64.png" alt="Discord" style="margin: 5px 10px;"></a>
<a href="https://github.com/tr7zw/EntityCulling"><img src="https://tr7zw.github.io/uikit/socialbuttonsicon/Github-Button-64.png" alt="GitHub" style="margin: 5px 10px;"></a>
</p>
<hr>
<img alt="About" src="https://tr7zw.github.io/uikit/about.png">
Minecraft skips rendering things that are behind you.
This mod introduces **asynchronous path-tracing** to efficiently determine what's actually visible &amp; fast.
"#;

        let stripped = strip_html_tags(input);
        assert!(!stripped.contains("<p"));
        assert!(!stripped.contains("</p>"));
        assert!(!stripped.contains("<a"));
        assert!(!stripped.contains("</a>"));
        assert!(!stripped.contains("Discord"));
        assert!(!stripped.contains("GitHub"));
        assert!(stripped.contains("## About"));
        assert!(stripped.contains("Minecraft skips rendering things that are behind you."));

        let cleaned_inline = clean_markdown_inline("![Entity Culling Banner](https://foo.bar)");
        assert_eq!(cleaned_inline, "");

        let bold_cleaned = clean_markdown_inline("**asynchronous path-tracing** &amp; fast");
        assert_eq!(bold_cleaned, "asynchronous path-tracing & fast");
    }
}

