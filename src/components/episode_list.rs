use ratatui::{
    layout::Constraint,
    style::{Style, Stylize},
    widgets::{
        Block, BorderType, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Table, TableState,
    },
};

use crate::Episode;

#[derive(Default)]
pub struct EpisodeListState {
    is_focused: bool,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl EpisodeListState {
    pub fn focus(&mut self, value: bool) {
        self.is_focused = value;
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn first(&mut self) {
        self.table_state.select_first();
        self.scroll_state.first();
    }

    pub fn prev(&mut self) {
        self.table_state.select_previous();
        self.scroll_state.prev();
    }

    pub fn next(&mut self) {
        self.table_state.select_next();
        self.scroll_state.next();
    }
}

pub struct EpisodeList<'a> {
    episodes: &'a [Episode],
}

impl<'a> EpisodeList<'a> {
    pub fn new(episodes: &'a [Episode]) -> Self {
        Self { episodes }
    }
}

impl<'a> StatefulWidget for EpisodeList<'a> {
    type State = EpisodeListState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if !self.episodes.is_empty() && state.table_state.selected() == None {
            state.table_state.select_first();
        }
        state.scroll_state = state.scroll_state.content_length(self.episodes.len());
        let episode_list_border = if state.is_focused {
            Block::bordered()
                .title("Episodes")
                .title_bottom("i: info")
                .border_type(BorderType::Thick)
        } else {
            Block::bordered()
                .title("Episodes")
                .border_type(BorderType::Plain)
        };

        Table::new(
            self.episodes
                .iter()
                .map(|episode| Row::new(vec![episode.title.as_str(), episode.date.as_str()])),
            [Constraint::Fill(1), Constraint::Length(10)],
        )
        .header(Row::new(vec!["Title", "Date"]).underlined())
        .block(episode_list_border)
        .row_highlight_style(Style::new().reversed())
        .render(area, buf, &mut state.table_state);

        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area,
            buf,
            &mut state.scroll_state,
        );
    }
}
