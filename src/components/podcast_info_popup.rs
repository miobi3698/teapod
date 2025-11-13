use ratatui::{
    prelude::{Buffer, Rect, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget, Wrap,
    },
};

use crate::Podcast;

#[derive(Default)]
pub struct PodcastInfoPopupState {
    scroll_offset: u16,
    scroll_state: ScrollbarState,
}

impl PodcastInfoPopupState {
    pub fn first(&mut self) {
        self.scroll_offset = 0;
        self.scroll_state.first();
    }

    pub fn prev(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.scroll_state.next();
    }

    pub fn next(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.scroll_state.next();
    }
}

pub struct PodcastInfoPopup<'a> {
    podcast: Option<&'a Podcast>,
}

impl<'a> PodcastInfoPopup<'a> {
    pub fn new(podcast: Option<&'a Podcast>) -> Self {
        Self { podcast }
    }
}

impl<'a> StatefulWidget for PodcastInfoPopup<'a> {
    type State = PodcastInfoPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };
        Clear.render(area, buf);
        if let Some(podcast) = self.podcast {
            let lines = vec![
                Line::from(vec!["Title: ".bold().into(), podcast.title.as_str().into()]),
                Line::from(vec![
                    "Description: ".bold().into(),
                    podcast.description.as_str().into(),
                ]),
                Line::from(vec!["Source: ".bold().into(), podcast.url.as_str().into()]),
            ];
            state.scroll_state = state.scroll_state.content_length(lines.len());
            Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .scroll((state.scroll_offset, 0))
                .block(
                    Block::bordered()
                        .title("Podcast Info")
                        .border_type(BorderType::Thick),
                )
                .render(area, buf);

            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                area,
                buf,
                &mut state.scroll_state,
            );
        } else {
            Paragraph::new("No info")
                .block(
                    Block::bordered()
                        .title("Podcast Info")
                        .border_type(BorderType::Thick),
                )
                .render(area, buf);
        }
    }
}
