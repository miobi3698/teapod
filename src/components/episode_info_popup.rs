use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::style::Stylize,
    text::Line,
    widgets::{
        Block, BorderType, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget, Wrap,
    },
};

use crate::Episode;

#[derive(Default)]
pub struct EpisodeInfoPopupState {
    scroll_offset: u16,
    scroll_state: ScrollbarState,
}

impl EpisodeInfoPopupState {
    pub fn first(&mut self) {
        self.scroll_offset = 0;
        self.scroll_state.first();
    }

    pub fn prev(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.scroll_state.prev();
    }

    pub fn next(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.scroll_state.next();
    }
}

pub struct EpisodeInfoPopup<'a> {
    episode: Option<&'a Episode>,
}

impl<'a> EpisodeInfoPopup<'a> {
    pub fn new(episode: Option<&'a Episode>) -> Self {
        Self { episode }
    }
}

impl<'a> StatefulWidget for EpisodeInfoPopup<'a> {
    type State = EpisodeInfoPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };
        Clear.render(area, buf);
        if let Some(episode) = self.episode {
            let lines = vec![
                Line::from(vec!["Title: ".bold().into(), episode.title.as_str().into()]),
                Line::from(vec!["Date: ".bold().into(), episode.date.as_str().into()]),
                Line::from(vec![
                    "Audio Source: ".bold().into(),
                    episode.audio_url.as_str().into(),
                ]),
                Line::from(vec![
                    "Description: ".bold().into(),
                    episode.description.as_str().into(),
                ]),
            ];
            state.scroll_state = state.scroll_state.content_length(lines.len());
            Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .scroll((state.scroll_offset, 0))
                .block(
                    Block::bordered()
                        .title("Episode Info")
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
                        .title("Episode Info")
                        .border_type(BorderType::Thick),
                )
                .render(area, buf);
        }
    }
}
