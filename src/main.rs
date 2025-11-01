use std::error::Error;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    'main_loop: loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Paragraph::default()
                    .block(Block::bordered().title("Teapod").title_bottom("q: quit")),
                frame.area(),
            );

            let [podcast_list_area, podcast_episode_list_area] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)])
                    .margin(1)
                    .areas(frame.area());

            frame.render_widget(
                Paragraph::default().block(Block::bordered().title("Podcasts")),
                podcast_list_area,
            );

            frame.render_widget(
                Paragraph::default().block(Block::bordered().title("Episodes")),
                podcast_episode_list_area,
            );
        })?;

        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => break 'main_loop,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    ratatui::restore();
    Ok(())
}
