use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

use crate::Audio;

pub struct Player<'a> {
    audio: &'a Option<Audio>,
}

impl<'a> Player<'a> {
    pub fn new(audio: &'a Option<Audio>) -> Self {
        Self { audio }
    }
}

impl<'a> Widget for Player<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(audio) = &self.audio {
            let player_status = if audio.sink.is_paused() {
                "Paused"
            } else {
                "Playing"
            };

            Paragraph::new(vec![
                Line::from(format!("[{}] {}", player_status, audio.title)),
                Line::from(format!(
                    "{}/{}",
                    format_audio_duration(audio.sink.get_pos()),
                    format_audio_duration(audio.total_duration)
                )),
            ])
            .block(Block::bordered().title("Player"))
            .render(area, buf);
        } else {
            Paragraph::new("[Stopped]")
                .block(Block::bordered().title("Player"))
                .render(area, buf);
        };
    }
}

fn format_audio_duration(duration: Duration) -> String {
    let mut total_seconds = duration.as_secs();
    let hours = total_seconds / (60 * 60);
    total_seconds %= 60 * 60;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
