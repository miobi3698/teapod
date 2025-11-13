use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Clear, Paragraph, StatefulWidget, Widget},
};

#[derive(Default)]
pub struct AddPodcastPopupState {
    pub url: String,
}

pub struct AddPodcastPopup {}

impl AddPodcastPopup {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for AddPodcastPopup {
    type State = AddPodcastPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = Rect {
            x: area.width / 4,
            y: (area.height - 3) / 2,
            width: area.width / 2,
            height: 3,
        };
        Clear.render(area, buf);
        Paragraph::new(state.url.as_str())
            .block(
                Block::bordered()
                    .title("Add Podcast")
                    .title_bottom("p: paste")
                    .border_type(BorderType::Thick),
            )
            .render(area, buf);
    }
}
