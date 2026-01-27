use std::{error::Error, time::Duration};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListState, Paragraph, Wrap},
};

use crate::podcast::{Podcast, download_podcast_info_from_url};

mod podcast;

type AnyError = Box<dyn Send + Sync + Error>;

enum ViewKind {
    PodcastInfo,
    AddPodcast,
    EpisodeList,
    EpisodeInfo,
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let mut terminal = ratatui::init();
    let mut clipboard = arboard::Clipboard::new()?;

    let title_style = Style::new().bold();

    let mut podcast_list_state = ListState::default();
    let mut episode_list_state = ListState::default();

    let mut view_stack = Vec::<ViewKind>::new();
    let mut add_podcast_url = String::new();
    let mut podcasts = Vec::<Podcast>::new();

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(5),
                ])
                .split(frame.area());

            frame.render_widget(
                Paragraph::new(Span::styled("Teapod", title_style)),
                main_layout[0],
            );

            match view_stack.last() {
                Some(view_kind) => match view_kind {
                    ViewKind::PodcastInfo => {
                        let podcast = &podcasts[podcast_list_state.selected().unwrap()];
                        frame.render_widget(
                            Paragraph::new(vec![
                                Line::from(vec![
                                    Span::styled("Description: ", title_style),
                                    Span::raw(podcast.description.as_str()),
                                ]),
                                Line::from(vec![
                                    Span::styled("Url: ", title_style),
                                    Span::raw(podcast.url.as_str()),
                                ]),
                            ])
                            .block(Block::bordered().title(Line::from(vec![
                                Span::styled("Podcast info: ", title_style),
                                Span::raw(podcast.title.as_str()),
                            ])))
                            .wrap(Wrap { trim: true }),
                            main_layout[1],
                        );
                    }
                    ViewKind::AddPodcast => frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::styled("Podcast url: ", title_style),
                            Span::raw(add_podcast_url.as_str()),
                        ]))
                        .block(Block::bordered().title(Span::styled("Add a podcast", title_style)))
                        .wrap(Wrap { trim: true }),
                        main_layout[1],
                    ),
                    ViewKind::EpisodeList => {
                        let podcast = &podcasts[podcast_list_state.selected().unwrap()];
                        if episode_list_state.selected().is_none() && podcast.episodes.len() > 0 {
                            episode_list_state.select_first();
                        }

                        frame.render_stateful_widget(
                            List::new(
                                podcast
                                    .episodes
                                    .iter()
                                    .map(|episode| episode.title.as_str())
                                    .collect::<Vec<_>>(),
                            )
                            .block(Block::bordered().title(Line::from(vec![
                                Span::styled("Episodes: ", title_style),
                                Span::raw(podcast.title.as_str()),
                            ])))
                            .highlight_style(Style::new().reversed()),
                            main_layout[1],
                            &mut episode_list_state,
                        )
                    }
                    ViewKind::EpisodeInfo => {
                        let podcast = &podcasts[podcast_list_state.selected().unwrap()];
                        let episode = &podcast.episodes[episode_list_state.selected().unwrap()];

                        frame.render_widget(
                            Paragraph::new(vec![Line::from(vec![
                                Span::styled("Description: ", title_style),
                                Span::raw(episode.description.as_str()),
                            ])])
                            .block(Block::bordered().title(Line::from(vec![
                                Span::styled("Episode info: ", title_style),
                                Span::raw(podcast.title.as_str()),
                                Span::raw(" | "),
                                Span::raw(episode.title.as_str()),
                            ])))
                            .wrap(Wrap { trim: true }),
                            main_layout[1],
                        );
                    }
                },
                None => {
                    if podcast_list_state.selected().is_none() && podcasts.len() > 0 {
                        podcast_list_state.select_first();
                    }

                    frame.render_stateful_widget(
                        List::new(
                            podcasts
                                .iter()
                                .map(|podcast| podcast.title.as_str())
                                .collect::<Vec<_>>(),
                        )
                        .block(Block::bordered().title(Span::styled("Podcasts", title_style)))
                        .highlight_style(Style::new().reversed()),
                        main_layout[1],
                        &mut podcast_list_state,
                    )
                }
            }

            frame.render_widget(
                Block::bordered().title(Span::styled("Player", title_style)),
                main_layout[2],
            );
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match view_stack.last() {
                        Some(view_kind) => match view_kind {
                            ViewKind::PodcastInfo => match key_event.code {
                                KeyCode::Esc => _ = view_stack.pop(),
                                _ => {}
                            },
                            ViewKind::AddPodcast => match key_event.code {
                                KeyCode::Esc => _ = view_stack.pop(),
                                KeyCode::Char('p') => {
                                    add_podcast_url = clipboard.get_text()?;
                                }
                                KeyCode::Enter => {
                                    let podcast =
                                        download_podcast_info_from_url(&add_podcast_url).await?;
                                    podcasts.push(podcast);
                                    add_podcast_url.clear();
                                    _ = view_stack.pop();
                                }
                                _ => {}
                            },
                            ViewKind::EpisodeList => match key_event.code {
                                KeyCode::Esc => _ = view_stack.pop(),
                                KeyCode::Char('i') => {
                                    if episode_list_state.selected().is_some() {
                                        view_stack.push(ViewKind::EpisodeInfo);
                                    }
                                }
                                KeyCode::Char('k') => episode_list_state.select_previous(),
                                KeyCode::Char('j') => episode_list_state.select_next(),
                                _ => {}
                            },
                            ViewKind::EpisodeInfo => match key_event.code {
                                KeyCode::Esc => _ = view_stack.pop(),
                                _ => {}
                            },
                        },
                        None => match key_event.code {
                            KeyCode::Char('q') => should_quit = true,
                            KeyCode::Char('i') => {
                                if podcast_list_state.selected().is_some() {
                                    view_stack.push(ViewKind::PodcastInfo);
                                }
                            }
                            KeyCode::Char('a') => view_stack.push(ViewKind::AddPodcast),
                            KeyCode::Char('k') => podcast_list_state.select_previous(),
                            KeyCode::Char('j') => podcast_list_state.select_next(),
                            KeyCode::Enter => {
                                if podcast_list_state.selected().is_some() {
                                    view_stack.push(ViewKind::EpisodeList);
                                }
                            }
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
