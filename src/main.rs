use std::{error::Error, fs::File, io::BufReader, time::Duration};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListState, Paragraph, Row, Table, TableState, Wrap},
};
use rodio::{Sink, Source};

use crate::podcast::{
    PODCAST_FEED_FILE, Podcast, check_podcast_audio_in_path, download_podcast_audio_to_path,
    download_podcast_info_from_url, save_podcast_info_to_path, update_all_podcast_info,
};

mod podcast;

type AnyError = Box<dyn Send + Sync + Error>;

enum ViewKind {
    PodcastInfo,
    AddPodcast,
    EpisodeList,
    EpisodeInfo,
}

struct PlayerState {
    title: String,
    sink: Sink,
    duration: Duration,
}

fn format_audio_duration(duration: Duration) -> String {
    let mut total_seconds = duration.as_secs();
    let hours = total_seconds / (60 * 60);
    total_seconds %= 60 * 60;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let home_path = std::env::home_dir().ok_or("missing home directory")?;
    let data_path = home_path.join(".local/share/teapod");
    if !data_path.exists() {
        tokio::fs::create_dir_all(&data_path).await?;
    }

    let mut podcasts = Vec::<Podcast>::new();
    let mut read_dir = tokio::fs::read_dir(&data_path).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let feed_file = entry.path().join(PODCAST_FEED_FILE);
        if feed_file.exists() {
            let json = tokio::fs::read_to_string(&feed_file).await?;
            let podcast = serde_json::from_str(&json)?;
            podcasts.push(podcast);
        }
    }

    let mut clipboard = arboard::Clipboard::new()?;
    let stream_handle = {
        let mut handle = rodio::OutputStreamBuilder::open_default_stream()?;
        handle.log_on_drop(false);
        handle
    };
    let mut player: Option<PlayerState> = None;

    let mut terminal = ratatui::init();

    let title_style = Style::new().bold();
    let table_header_style = Style::new().underlined();

    let mut podcast_list_state = ListState::default();
    let mut episode_list_table_state = TableState::default();

    let mut view_stack = Vec::<ViewKind>::new();
    let mut add_podcast_url = String::new();

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
                                Span::styled(podcast.title.as_str(), title_style),
                                Span::styled(" / Info", title_style),
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
                        if episode_list_table_state.selected().is_none()
                            && podcast.episodes.len() > 0
                        {
                            episode_list_table_state.select_first();
                        }

                        frame.render_stateful_widget(
                            Table::new(
                                podcast
                                    .episodes
                                    .iter()
                                    .map(|episode| {
                                        let is_downloaded = check_podcast_audio_in_path(
                                            podcast, episode, &data_path,
                                        );

                                        Row::new(vec![
                                            episode.title.as_str(),
                                            episode.pub_date.as_str(),
                                            if is_downloaded { "Yes" } else { "No" },
                                        ])
                                    })
                                    .collect::<Vec<_>>(),
                                [
                                    Constraint::Fill(1),
                                    Constraint::Length(10),
                                    Constraint::Length(10),
                                ],
                            )
                            .header(
                                Row::new(vec!["Title", "Date", "Downloaded"])
                                    .style(table_header_style),
                            )
                            .block(Block::bordered().title(Line::from(vec![
                                Span::styled(podcast.title.as_str(), title_style),
                                Span::styled(" / Episodes", title_style),
                            ])))
                            .row_highlight_style(Style::new().reversed()),
                            main_layout[1],
                            &mut episode_list_table_state,
                        );
                    }
                    ViewKind::EpisodeInfo => {
                        let podcast = &podcasts[podcast_list_state.selected().unwrap()];
                        let episode =
                            &podcast.episodes[episode_list_table_state.selected().unwrap()];

                        frame.render_widget(
                            Paragraph::new(vec![Line::from(vec![
                                Span::styled("Description: ", title_style),
                                Span::raw(episode.description.as_str()),
                            ])])
                            .block(Block::bordered().title(Line::from(vec![
                                Span::styled(podcast.title.as_str(), title_style),
                                Span::raw(" / "),
                                Span::styled(episode.title.as_str(), title_style),
                                Span::styled(" / Info", title_style),
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
                    );
                }
            }

            if let Some(player_state) = &player {
                let status = if player_state.sink.is_paused() {
                    "Paused"
                } else {
                    "Playing"
                };
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(vec![
                            Span::raw("Now playing: "),
                            Span::styled(player_state.title.as_str(), title_style),
                        ]),
                        Line::from(vec![
                            Span::raw("Status: "),
                            Span::styled(status, title_style),
                        ]),
                        Line::from(vec![
                            Span::raw("Duration: "),
                            Span::raw(format_audio_duration(player_state.sink.get_pos()).as_str()),
                            Span::raw("/"),
                            Span::raw(format_audio_duration(player_state.duration).as_str()),
                        ]),
                    ])
                    .block(Block::bordered().title(Span::styled("Player", title_style))),
                    main_layout[2],
                );
            } else {
                frame.render_widget(
                    Block::bordered().title(Span::styled("Player", title_style)),
                    main_layout[2],
                );
            }
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    if key_event.code == KeyCode::Char(' ') {
                        if let Some(player_state) = &player {
                            if player_state.sink.is_paused() {
                                player_state.sink.play();
                            } else {
                                player_state.sink.pause();
                            }
                        }
                    } else {
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
                                            download_podcast_info_from_url(&add_podcast_url)
                                                .await?;
                                        save_podcast_info_to_path(&podcast, &data_path).await?;

                                        podcasts.push(podcast);
                                        add_podcast_url.clear();
                                        _ = view_stack.pop();
                                    }
                                    _ => {}
                                },
                                ViewKind::EpisodeList => match key_event.code {
                                    KeyCode::Esc => _ = view_stack.pop(),
                                    KeyCode::Char('i') => {
                                        if episode_list_table_state.selected().is_some() {
                                            view_stack.push(ViewKind::EpisodeInfo);
                                        }
                                    }
                                    KeyCode::Char('k') => {
                                        episode_list_table_state.select_previous()
                                    }
                                    KeyCode::Char('j') => episode_list_table_state.select_next(),
                                    KeyCode::Enter => {
                                        if episode_list_table_state.selected().is_some() {
                                            if let Some(player_state) = &player {
                                                player_state.sink.clear();
                                            }

                                            let podcast =
                                                &podcasts[podcast_list_state.selected().unwrap()];
                                            let episode = &podcast.episodes
                                                [episode_list_table_state.selected().unwrap()];
                                            let audio_file = download_podcast_audio_to_path(
                                                podcast, episode, &data_path,
                                            )
                                            .await?;
                                            let reader = BufReader::new(File::open(audio_file)?);
                                            let source = rodio::Decoder::try_from(reader)?;

                                            let title =
                                                format!("{} / {}", &podcast.title, &episode.title);
                                            let sink = Sink::connect_new(&stream_handle.mixer());
                                            let duration =
                                                source.total_duration().unwrap_or_default();
                                            sink.append(source);
                                            player = Some(PlayerState {
                                                title,
                                                sink,
                                                duration,
                                            });
                                        }
                                    }
                                    _ => {}
                                },
                                ViewKind::EpisodeInfo => match key_event.code {
                                    KeyCode::Esc => _ = view_stack.pop(),
                                    _ => {}
                                },
                            },
                            None => match key_event.code {
                                KeyCode::Char('q') => should_quit = true,
                                KeyCode::Char('u') => {
                                    podcasts = update_all_podcast_info(
                                        &podcasts
                                            .iter()
                                            .map(|podcast| podcast.url.as_str())
                                            .collect(),
                                        &data_path,
                                    )
                                    .await?;
                                }
                                KeyCode::Char('a') => view_stack.push(ViewKind::AddPodcast),
                                KeyCode::Char('k') => podcast_list_state.select_previous(),
                                KeyCode::Char('j') => podcast_list_state.select_next(),
                                KeyCode::Char('i') => {
                                    if podcast_list_state.selected().is_some() {
                                        view_stack.push(ViewKind::PodcastInfo);
                                    }
                                }
                                KeyCode::Enter => {
                                    if podcast_list_state.selected().is_some() {
                                        view_stack.push(ViewKind::EpisodeList);
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                }
                _ => {}
            }
        }
    }

    ratatui::restore();
    Ok(())
}
