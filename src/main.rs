use std::{error::Error, sync::Arc, time::Duration};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};
use tokio::sync::mpsc;

type AnyError = Box<dyn Send + Sync + Error>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let mut app = App::default();
    let mut app_ui_state = AppUIState::default();

    let urls = ["https://changelog.fm/rss", "https://feed.syntax.fm/"];
    let client = Arc::new(reqwest::Client::new());
    let (tx, mut rx) = mpsc::channel(urls.len());
    for url in urls {
        let tx = tx.clone();
        let client = client.clone();
        tokio::spawn(async move {
            let res = client.get(url).send().await?;
            let text = res.text().await?;
            let doc = roxmltree::Document::parse(text.as_str())?;
            if let Some(channel) = doc.descendants().find(|n| n.has_tag_name("channel")) {
                let title = channel
                    .descendants()
                    .find(|n| n.has_tag_name("title"))
                    .map(|n| n.text())
                    .flatten()
                    .unwrap_or_default()
                    .to_string();
                let description = channel
                    .descendants()
                    .find(|n| n.has_tag_name("description"))
                    .map(|n| n.text())
                    .flatten()
                    .unwrap_or_default()
                    .to_string();
                let podcast = Podcast { title, description };
                tx.send(podcast).await?;
            }

            Ok::<_, AnyError>(())
        });
    }

    let mut terminal = ratatui::init();

    while !app.should_quit {
        terminal.draw(|frame| {
            frame.render_stateful_widget(&app, frame.area(), &mut app_ui_state);
        })?;

        if event::poll(Duration::from_millis(250))? {
            app.handle_event(event::read()?);
        }

        if app.podcasts.len() != urls.len() {
            if let Some(podcast) = rx.recv().await {
                app.podcasts.push(podcast);
            }
        }
    }

    ratatui::restore();
    Ok(())
}

struct Podcast {
    title: String,
    description: String,
}

enum ViewKind {
    PodcastInfo(usize),
}

#[derive(Default)]
struct App {
    should_quit: bool,
    view_stack: Vec<ViewKind>,
    podcasts: Vec<Podcast>,
}

#[derive(Default)]
struct AppUIState {
    podcast_list_state: ListState,
}

impl App {
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('i') => self.view_stack.push(ViewKind::PodcastInfo(0)),
                    KeyCode::Esc => _ = self.view_stack.pop(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl StatefulWidget for &App {
    type State = AppUIState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(5),
            ])
            .split(area);

        Paragraph::new(Span::styled("Teapod", Style::default().bold())).render(main_layout[0], buf);
        match self.view_stack.last() {
            Some(view) => match view {
                ViewKind::PodcastInfo(index) => {
                    let podcast = &self.podcasts[*index];
                    let title = Line::from(vec![
                        Span::styled("Info: ", Style::new().bold()),
                        Span::raw(podcast.title.as_str()),
                    ]);
                    Paragraph::new(podcast.description.as_str())
                        .wrap(Wrap { trim: true })
                        .block(Block::bordered().title(title))
                        .render(main_layout[1], buf);
                }
            },
            None => {
                StatefulWidget::render(
                    List::new(self.podcasts.iter().map(|podcast| podcast.title.as_str())).block(
                        Block::bordered().title(Span::raw("Browser").style(Style::new().bold())),
                    ),
                    main_layout[1],
                    buf,
                    &mut state.podcast_list_state,
                );
            }
        }
        Block::bordered()
            .title(Span::raw("Player").style(Style::new().bold()))
            .render(main_layout[2], buf);
    }
}
