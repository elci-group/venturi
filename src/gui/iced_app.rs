// iced backend — retained-mode GUI using iced 0.13's functional builder API.

use iced::widget::{button, column, container, horizontal_space, row, text};
use iced::{Element, Length, Task};

use super::ContactCard;

#[derive(Debug, Clone)]
pub enum Message {
    New,
    Edit,
    Share,
    Search,
}

pub struct VenturiCardsIced {
    card: ContactCard,
    contact_index: usize,
    total_contacts: usize,
}

impl VenturiCardsIced {
    pub fn new(card: ContactCard) -> Self {
        Self {
            card,
            contact_index: 1,
            total_contacts: 3,
        }
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // ── Title bar ──────────────────────────────────────────────────
        let titlebar = container(
            column![
                text("V E N T U R I   C A R D S").size(22),
                text("v 1.0").size(13),
            ]
            .spacing(4),
        )
        .padding(16)
        .width(Length::Fill)
        .center_x(Length::Fill);

        // ── Contact card ──────────────────────────────────────────────
        let field = |label: String, value: String| -> Element<'static, Message> {
            row![
                text(label).size(14).width(Length::Fixed(60.0)),
                text(value).size(14),
            ]
            .spacing(12)
            .into()
        };

        let card_content = container(
            column![
                field("Name".into(), self.card.name.clone()),
                field("Role".into(), self.card.role.clone()),
                field("Email".into(), self.card.email.clone()),
                field("Tags".into(), self.card.tags.clone()),
            ]
            .spacing(10),
        )
        .padding(20)
        .width(Length::Fill);

        // ── Status / action bar ───────────────────────────────────────
        let statusbar = container(
            row![
                button("New").on_press(Message::New),
                button("Edit").on_press(Message::Edit),
                button("Share").on_press(Message::Share),
                horizontal_space(),
                text(format!(
                    "Contact {} of {}",
                    self.contact_index, self.total_contacts
                ))
                .size(13),
                button("Search").on_press(Message::Search),
            ]
            .spacing(8),
        )
        .padding(10)
        .width(Length::Fill);

        column![titlebar, card_content, statusbar].into()
    }
}

pub fn run(card: ContactCard) -> crate::error::Result<()> {
    iced::application(
        "VenturiCards",
        VenturiCardsIced::update,
        VenturiCardsIced::view,
    )
    .window_size((500.0, 340.0))
    .run_with(move || (VenturiCardsIced::new(card), Task::none()))
    .map_err(|e| crate::error::VenturiError::Gui(e.to_string()))
}
