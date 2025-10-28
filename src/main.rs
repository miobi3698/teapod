use crossterm::event::{self, Event, KeyEventKind};
use ratatui::widgets::Paragraph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    'main_loop: loop {
        terminal.draw(|frame| frame.render_widget(Paragraph::new("Teapod"), frame.area()))?;
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    event::KeyCode::Char('q') => break 'main_loop,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    ratatui::restore();
    Ok(())
}
