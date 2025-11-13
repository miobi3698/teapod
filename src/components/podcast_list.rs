use ratatui::{
    layout::Rect,
    prelude::Buffer,
    style::{Style, Stylize},
    widgets::{
        Block, BorderType, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget,
    },
};

use crate::Podcast;

#[derive(Default)]
pub struct PodcastListState {
    is_focused: bool,
    list_state: ListState,
    scroll_state: ScrollbarState,
}

impl PodcastListState {
    pub fn focus(&mut self, value: bool) {
        self.is_focused = value;
    }

    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn next(&mut self) {
        self.list_state.select_next();
        self.scroll_state.next();
    }

    pub fn prev(&mut self) {
        self.list_state.select_previous();
        self.scroll_state.prev();
    }
}

pub struct PodcastList<'a> {
    podcasts: &'a [Podcast],
}

impl<'a> PodcastList<'a> {
    pub fn new(podcasts: &'a [Podcast]) -> Self {
        Self { podcasts }
    }
}

impl<'a> StatefulWidget for PodcastList<'a> {
    type State = PodcastListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.scroll_state = state.scroll_state.content_length(self.podcasts.len());
        if !self.podcasts.is_empty() && state.list_state.selected() == None {
            state.list_state.select_first();
        }

        let podcast_list_border = if state.is_focused {
            Block::bordered()
                .title("Podcasts")
                .title_bottom("i: info")
                .border_type(BorderType::Thick)
        } else {
            Block::bordered()
                .title("Podcasts")
                .border_type(BorderType::Plain)
        };

        StatefulWidget::render(
            List::new(self.podcasts.iter().map(|podcast| podcast.title.as_str()))
                .block(podcast_list_border)
                .highlight_style(Style::new().reversed()),
            area,
            buf,
            &mut state.list_state,
        );
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area,
            buf,
            &mut state.scroll_state,
        );
    }
}
