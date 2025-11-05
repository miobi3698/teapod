use std::{error::Error, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Cell, Clear, List, Paragraph, Row, Table, TableState, Widget},
};

use crate::rss::{Podcast, download_podcast_info};

mod rss;

enum View {
    Podcast,
    Episode,
    Add,
    Update,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut podcasts: Vec<Podcast> = Vec::new();
    let mut selected_podcast = 0;
    let mut podcast_episodes_table_state = TableState::default().with_selected(0);
    let mut current_view = View::Podcast;
    let mut add_url = String::new();

    let mut terminal = ratatui::init();
    'main_loop: loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Paragraph::default().block(
                    Block::bordered()
                        .title("Teapod")
                        .title_bottom("q: quit")
                        .title_bottom("a: add")
                        .title_bottom("u: update"),
                ),
                frame.area(),
            );

            let [podcast_list_area, podcast_episode_list_area] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)])
                    .margin(1)
                    .areas(frame.area());

            let podcasts_border = if matches!(current_view, View::Podcast) {
                Block::bordered()
                    .title("Podcasts")
                    .border_type(BorderType::Double)
                    .title_bottom("Enter: view")
                    // TODO(miobi): implement this
                    // .title_bottom("i: info")
                    .title_bottom("k: up")
                    .title_bottom("j: down")
            } else {
                Block::bordered().title("Podcasts")
            };
            frame.render_widget(
                List::new(podcasts.iter().enumerate().map(|(index, podcast)| {
                    if selected_podcast == index {
                        Text::from(podcast.title.clone()).reversed()
                    } else {
                        Text::from(podcast.title.clone())
                    }
                }))
                .block(podcasts_border),
                podcast_list_area,
            );

            let episodes_border = if matches!(current_view, View::Episode) {
                Block::bordered()
                    .title("Episodes")
                    .border_type(BorderType::Double)
                    .title_bottom("Esc: back")
                    // TODO(miobi): implement this
                    // .title_bottom("i: info")
                    .title_bottom("k: up")
                    .title_bottom("j: down")
            } else {
                Block::bordered().title("Episodes")
            };
            let episodes = match podcasts.get(selected_podcast) {
                Some(podcast) => &podcast.episodes,
                None => &Vec::new(),
            };
            frame.render_stateful_widget(
                Table::new(
                    episodes.iter().map(|episode| {
                        Row::new(vec![
                            Cell::from(Text::from(episode.title.clone())),
                            Cell::from(Text::from(episode.date.date_naive().to_string())),
                            Cell::from(Text::from(episode.duration.clone())),
                        ])
                    }),
                    [
                        Constraint::Fill(2),
                        Constraint::Length(10),
                        Constraint::Length(8),
                    ],
                )
                .header(Row::new(vec!["Title", "Date", "Duration"]).underlined())
                .row_highlight_style(Style::default().reversed())
                .block(episodes_border),
                podcast_episode_list_area,
                &mut podcast_episodes_table_state,
            );

            if matches!(current_view, View::Add) {
                let popup_area = Rect {
                    x: frame.area().width / 4,
                    y: (frame.area().height - 3) / 2,
                    width: frame.area().width / 2,
                    height: 3,
                };
                Clear.render(popup_area, frame.buffer_mut());
                frame.render_widget(
                    Paragraph::new(add_url.clone()).block(
                        Block::bordered()
                            .title("Add podcast")
                            .title_bottom("Esc: back")
                            .title_bottom("p: paste")
                            .title_bottom("Enter: add")
                            .border_type(BorderType::Double),
                    ),
                    popup_area,
                );
            }

            if matches!(current_view, View::Update) {
                let popup_area = Rect {
                    x: frame.area().width / 4,
                    y: frame.area().height / 4,
                    width: frame.area().width / 2,
                    height: frame.area().height / 2,
                };
                Clear.render(popup_area, frame.buffer_mut());
                frame.render_widget(
                    Paragraph::new(add_url.clone()).block(
                        Block::bordered()
                            .title("Update podcasts")
                            .title_bottom("Esc: back")
                            .border_type(BorderType::Double),
                    ),
                    popup_area,
                );
            }
        })?;

        while event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Char('q') => break 'main_loop,
                        KeyCode::Char('a') => {
                            add_url.clear();
                            current_view = View::Add;
                        }
                        KeyCode::Char('u') => {
                            current_view = View::Update;
                            // TODO(miobi): implement this
                            // for podcast in podcasts.iter_mut() {
                            //     *podcast = download_podcast_info(podcast.url.as_str()).await?;
                            // }
                        }
                        _ => {}
                    }

                    match current_view {
                        // TODO(miobi): support delete podcast
                        View::Podcast => match key_event.code {
                            KeyCode::Char('j') => {
                                selected_podcast =
                                    selected_podcast.saturating_add(1).min(podcasts.len() - 1)
                            }
                            KeyCode::Char('k') => {
                                selected_podcast = selected_podcast.saturating_sub(1)
                            }
                            KeyCode::Enter => {
                                current_view = View::Episode;
                                if let Some(podcast) = podcasts.get(selected_podcast) {
                                    if !podcast.episodes.is_empty() {
                                        podcast_episodes_table_state.select(Some(0));
                                    }
                                }
                            }
                            _ => {}
                        },
                        View::Episode => match key_event.code {
                            KeyCode::Esc => current_view = View::Podcast,
                            KeyCode::Char('j') => podcast_episodes_table_state.select_next(),
                            KeyCode::Char('k') => podcast_episodes_table_state.select_previous(),
                            _ => {}
                        },
                        View::Add => match key_event.code {
                            KeyCode::Esc => current_view = View::Podcast,
                            KeyCode::Char('p') => {
                                // TODO(miobi): copy from clipboard
                                add_url = "https://changelog.fm/rss".to_string();
                            }
                            KeyCode::Enter => {
                                // TODO(miobi): save to file
                                // TODO(miobi): check for duplicate
                                let podcast = download_podcast_info(&add_url).await?;
                                podcasts.push(podcast);
                            }
                            _ => {}
                        },
                        View::Update => match key_event.code {
                            KeyCode::Esc => current_view = View::Podcast,
                            _ => {}
                        },
                    }
                }
                _ => {}
            }
        }
    }

    ratatui::restore();
    Ok(())
}
