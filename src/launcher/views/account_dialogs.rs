use crate::launcher::RixLauncher;
use crate::launcher::types::Message;
use crate::style::{TextBoldExt, action_button_style, dialog_panel_style, outline_button_style};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, column, container, row, stack, text};
use iced::{Alignment, Border, Color, Element, Length};

const SVG_DELETE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>"##;

pub fn view_account_context_modal<'a>(
    launcher: &'a RixLauncher,
    account_index: usize,
) -> Element<'a, Message> {
    let _account = match launcher.accounts.get(account_index) {
        Some(a) => a,
        None => return container(text("")).into(),
    };

    let menu_item = Button::new(
        row![
            iced::widget::svg(iced::widget::svg::Handle::from_memory(
                SVG_DELETE.as_bytes()
            ))
            .width(14)
            .height(14)
            .style(move |_, _| iced::widget::svg::Style {
                color: Some(Color::from_rgba(0.85, 0.25, 0.25, 0.9)),
            }),
            text("Delete Account")
                .size(13)
                .bold()
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.9)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ConfirmDeleteAccount(account_index))
    .padding(iced::Padding::new(10.0).left(16.0).right(16.0))
    .style(move |_theme, status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => Color::from_rgba(0.85, 0.25, 0.25, 0.2),
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
    .width(Length::Fill);

    let modal_card = container(column![menu_item].spacing(4))
        .width(160)
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
        .on_press(Message::CloseAccountContextMenu)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let mut x = launcher.context_menu_position.x;
    let mut y = (launcher.context_menu_position.y - 105.0).max(0.0);

    if x + 170.0 > launcher.window_width as f32 {
        x = (launcher.window_width as f32 - 170.0).max(0.0);
    }
    let max_y = (launcher.window_height as f32 - 105.0 - 60.0).max(0.0);
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

pub fn view_account_delete_confirmation_modal<'a>(
    launcher: &'a RixLauncher,
    account_index: usize,
) -> Element<'a, Message> {
    let account_name = launcher
        .accounts
        .get(account_index)
        .map(|acc| acc.name.as_str())
        .unwrap_or("");

    let dialog = container(
        column![
            text("Delete Account").size(18).bold().color(Color::WHITE),
            text(format!(
                "Are you sure you want to delete account \"{}\"?\nThis action cannot be undone.",
                account_name
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
                .on_press(Message::CancelDeleteAccount)
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
                .on_press(Message::DeleteAccountConfirmed(account_index))
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
        .on_press(Message::CancelDeleteAccount)
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
