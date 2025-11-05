use std::{error::Error, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{
        Block, BorderType, Cell, Clear, List, Paragraph, Row, Table, TableState, Widget, Wrap,
    },
};

use crate::rss::{Podcast, download_podcast_info};

mod rss;

enum View {
    PodcastList,
    PodcastInfo,
    EpisodeList,
    EpisodeInfo,
    AddPodcast,
    UpdatePodcasts,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut podcasts: Vec<Podcast> = Vec::new();
    let mut selected_podcast = 0;
    let mut podcast_episodes_table_state = TableState::default().with_selected(0);
    let mut current_view = View::PodcastList;
    let mut add_url = String::new();
    let mut clipboard = arboard::Clipboard::new()?;

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

            let podcasts_border = if matches!(current_view, View::PodcastList) {
                Block::bordered()
                    .title("Podcasts")
                    .border_type(BorderType::Double)
                    .title_bottom("Enter: view")
                    .title_bottom("i: info")
            } else {
                Block::bordered().title("Podcasts")
            };
            frame.render_widget(
                List::new(podcasts.iter().enumerate().map(|(index, podcast)| {
                    if selected_podcast == index {
                        Text::from(podcast.title.as_str()).reversed()
                    } else {
                        Text::from(podcast.title.as_str())
                    }
                }))
                .block(podcasts_border),
                podcast_list_area,
            );

            let episodes_border = if matches!(current_view, View::EpisodeList) {
                Block::bordered()
                    .title("Episodes")
                    .border_type(BorderType::Double)
                    .title_bottom("Esc: back")
                    .title_bottom("i: info")
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
                            Cell::from(Text::from(episode.title.as_str())),
                            Cell::from(Text::from(episode.date.as_str())),
                        ])
                    }),
                    [Constraint::Fill(2), Constraint::Length(10)],
                )
                .header(Row::new(vec!["Title", "Date"]).underlined())
                .row_highlight_style(Style::default().reversed())
                .block(episodes_border),
                podcast_episode_list_area,
                &mut podcast_episodes_table_state,
            );

            match current_view {
                View::PodcastList => {}
                View::EpisodeList => {}
                View::PodcastInfo => {
                    let popup_area = Rect {
                        x: frame.area().width / 4,
                        y: frame.area().height / 4,
                        width: frame.area().width / 2,
                        height: frame.area().height / 2,
                    };
                    Clear.render(popup_area, frame.buffer_mut());
                    let podcast_info = if let Some(podcast) = podcasts.get(selected_podcast) {
                        podcast.description.as_str()
                    } else {
                        "No info"
                    };
                    frame.render_widget(
                        Paragraph::new(podcast_info)
                            .block(
                                Block::bordered()
                                    .title("Podcast info")
                                    .title_bottom("Esc: back")
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true }),
                        popup_area,
                    );
                }
                View::EpisodeInfo => {
                    let popup_area = Rect {
                        x: frame.area().width / 4,
                        y: frame.area().height / 4,
                        width: frame.area().width / 2,
                        height: frame.area().height / 2,
                    };
                    Clear.render(popup_area, frame.buffer_mut());
                    let episode_info = if let Some(index) = podcast_episodes_table_state.selected()
                    {
                        podcasts[selected_podcast].episodes[index]
                            .description
                            .as_str()
                    } else {
                        "No info"
                    };
                    frame.render_widget(
                        Paragraph::new(episode_info)
                            .block(
                                Block::bordered()
                                    .title("Episode info")
                                    .title_bottom("Esc: back")
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true }),
                        popup_area,
                    );
                }
                View::AddPodcast => {
                    let popup_area = Rect {
                        x: frame.area().width / 4,
                        y: (frame.area().height - 3) / 2,
                        width: frame.area().width / 2,
                        height: 3,
                    };
                    Clear.render(popup_area, frame.buffer_mut());
                    frame.render_widget(
                        Paragraph::new(add_url.as_str()).block(
                            Block::bordered()
                                .title("Add podcast")
                                .title_bottom("Esc: back")
                                .title_bottom("p: paste")
                                .title_bottom("d: delete")
                                .title_bottom("Enter: add")
                                .border_type(BorderType::Double),
                        ),
                        popup_area,
                    );
                }
                View::UpdatePodcasts => {
                    let popup_area = Rect {
                        x: frame.area().width / 4,
                        y: frame.area().height / 4,
                        width: frame.area().width / 2,
                        height: frame.area().height / 2,
                    };
                    Clear.render(popup_area, frame.buffer_mut());
                    frame.render_widget(
                        Paragraph::default().block(
                            Block::bordered()
                                .title("Update podcasts")
                                .title_bottom("Esc: back")
                                .border_type(BorderType::Double),
                        ),
                        popup_area,
                    );
                }
            }
        })?;

        while event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind.is_press() => {
                    match key_event.code {
                        KeyCode::Char('q') => break 'main_loop,
                        KeyCode::Char('a') => {
                            add_url.clear();
                            current_view = View::AddPodcast;
                        }
                        KeyCode::Char('u') => {
                            current_view = View::UpdatePodcasts;
                            // TODO(miobi): implement this
                            // for podcast in podcasts.iter_mut() {
                            //     *podcast = download_podcast_info(podcast.url.as_str()).await?;
                            // }
                        }
                        _ => {}
                    }

                    match current_view {
                        // TODO(miobi): support delete podcast
                        View::PodcastList => match key_event.code {
                            KeyCode::Char('j') => {
                                selected_podcast =
                                    selected_podcast.saturating_add(1).min(podcasts.len() - 1)
                            }
                            KeyCode::Char('k') => {
                                selected_podcast = selected_podcast.saturating_sub(1)
                            }
                            KeyCode::Char('i') => current_view = View::PodcastInfo,
                            KeyCode::Enter => {
                                current_view = View::EpisodeList;
                                if let Some(podcast) = podcasts.get(selected_podcast) {
                                    if !podcast.episodes.is_empty() {
                                        podcast_episodes_table_state.select(Some(0));
                                    }
                                }
                            }
                            _ => {}
                        },
                        View::PodcastInfo => match key_event.code {
                            KeyCode::Esc => current_view = View::PodcastList,
                            _ => {}
                        },
                        View::EpisodeList => match key_event.code {
                            KeyCode::Esc => current_view = View::PodcastList,
                            KeyCode::Char('j') => podcast_episodes_table_state.select_next(),
                            KeyCode::Char('k') => podcast_episodes_table_state.select_previous(),
                            KeyCode::Char('i') => current_view = View::EpisodeInfo,
                            _ => {}
                        },
                        View::EpisodeInfo => match key_event.code {
                            KeyCode::Esc => current_view = View::EpisodeList,
                            _ => {}
                        },
                        View::AddPodcast => match key_event.code {
                            KeyCode::Esc => current_view = View::PodcastList,
                            KeyCode::Char('p') => {
                                // add_url = "https://changelog.fm/rss".to_string();
                                add_url = clipboard.get_text()?;
                            }
                            KeyCode::Char('d') => {
                                add_url.clear();
                            }
                            KeyCode::Enter => {
                                // TODO(miobi): save to file
                                // TODO(miobi): check for duplicate
                                let podcast = download_podcast_info(&add_url).await?;
                                podcasts.push(podcast);
                                current_view = View::PodcastList;
                            }
                            _ => {}
                        },
                        View::UpdatePodcasts => match key_event.code {
                            KeyCode::Esc => current_view = View::PodcastList,
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
