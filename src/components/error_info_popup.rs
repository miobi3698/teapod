use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Clear, Paragraph, StatefulWidget, Widget},
};

#[derive(Default)]
pub struct ErrorInfoPopupState {
    pub error_msg: String,
}

pub struct ErrorInfoPopup {}

impl ErrorInfoPopup {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ErrorInfoPopup {
    type State = ErrorInfoPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        Clear.render(popup_area, buf);
        Paragraph::new(state.error_msg.as_str())
            .block(
                Block::bordered()
                    .title("Error")
                    .border_type(BorderType::Thick),
            )
            .render(popup_area, buf);
    }
}
