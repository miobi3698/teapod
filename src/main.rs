use std::{error::Error, fs::File, io::BufReader, vec};

use arboard::Clipboard;
use chrono::DateTime;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Clear, List, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Widget, Wrap,
    },
};
use rodio::Sink;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Podcast {
    title: String,
    description: String,
    url: String,
    episodes: Vec<Episode>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Episode {
    title: String,
    description: String,
    date: String,
    audio_url: String,
}

enum View {
    PodcastList,
    PodcastInfo,
    AddPodcast,
    EpisodeList,
    EpisodeInfo,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let data_path = std::env::home_dir().unwrap().join(".local/share/teapod");
    if !data_path.exists() {
        std::fs::create_dir(&data_path)?;
    }

    let mut podcasts: Vec<Podcast> = Vec::new();

    for entry in std::fs::read_dir(&data_path)? {
        let entry = entry?;
        let path = entry.path().join("feed.json");
        if path.exists() {
            let contents = tokio::fs::read_to_string(path).await?;
            let podcast = serde_json::from_str(&contents)?;
            podcasts.push(podcast);
        }
    }

    let mut terminal = ratatui::init();

    let mut current_view = View::PodcastList;
    let mut podcast_list_state = ListState::default();
    if !podcasts.is_empty() {
        podcast_list_state.select_first();
    }
    let mut podcast_list_scroll_state = ScrollbarState::default();

    let mut podcast_info_scroll: u16 = 0;
    let mut podcast_info_scroll_state = ScrollbarState::default();

    let mut episode_list_state = TableState::default();
    if let Some(podcast_index) = podcast_list_state.selected() {
        let podcast = &podcasts[podcast_index];
        if !podcast.episodes.is_empty() {
            episode_list_state.select_first();
        }
    }
    let mut episode_list_scroll_state = ScrollbarState::default();

    let mut episode_info_scroll: u16 = 0;
    let mut episode_info_scroll_state = ScrollbarState::default();

    let mut url_to_add = String::new();

    let mut player_title = String::new();
    let player_stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
    let mut player_sink: Option<Sink> = None;

    let mut is_running = true;
    while is_running {
        terminal.draw(|frame| {
            let [header_area, main_area, player_area, footer_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .areas(frame.area());

            frame.render_widget(Paragraph::new("Teapod"), header_area);

            let [podcast_list_area, episode_list_area] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)]).areas(main_area);

            podcast_list_scroll_state = podcast_list_scroll_state.content_length(podcasts.len());
            let podcast_list_border = if matches!(current_view, View::PodcastList) {
                Block::bordered()
                    .title("Podcasts")
                    .title_bottom("i: info")
                    .border_type(BorderType::Thick)
            } else {
                Block::bordered()
                    .title("Podcasts")
                    .border_type(BorderType::Plain)
            };
            frame.render_stateful_widget(
                List::new(podcasts.iter().map(|podcast| podcast.title.as_str()))
                    .block(podcast_list_border)
                    .highlight_style(Style::new().reversed()),
                podcast_list_area,
                &mut podcast_list_state,
            );
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                podcast_list_area,
                &mut podcast_list_scroll_state,
            );

            let episodes = if let Some(index) = podcast_list_state.selected() {
                podcasts[index].episodes.as_slice()
            } else {
                &[]
            };
            episode_list_scroll_state = episode_list_scroll_state.content_length(episodes.len());
            let episode_list_border = if matches!(current_view, View::EpisodeList) {
                Block::bordered()
                    .title("Episodes")
                    .title_bottom("i: info")
                    .border_type(BorderType::Thick)
            } else {
                Block::bordered()
                    .title("Episodes")
                    .border_type(BorderType::Plain)
            };
            frame.render_stateful_widget(
                Table::new(
                    episodes.iter().map(|episode| {
                        Row::new(vec![episode.title.as_str(), episode.date.as_str()])
                    }),
                    [Constraint::Fill(1), Constraint::Length(10)],
                )
                .header(Row::new(vec!["Title", "Date"]).underlined())
                .block(episode_list_border)
                .row_highlight_style(Style::new().reversed()),
                episode_list_area,
                &mut episode_list_state,
            );
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                episode_list_area,
                &mut episode_list_scroll_state,
            );

            let player_status = if let Some(sink) = &player_sink {
                sink.get_pos();
                if sink.is_paused() {
                    "Paused"
                } else {
                    "Playing"
                }
            } else {
                "Stopped"
            };
            frame.render_widget(
                Paragraph::new(vec![Line::from(vec![
                    format!("[{}] ", player_status).as_str().into(),
                    player_title.as_str().into(),
                ])])
                .block(Block::bordered().title("Player")),
                player_area,
            );

            frame.render_widget(Paragraph::new("q: quit, a: add, u: update"), footer_area);

            if matches!(current_view, View::PodcastInfo) {
                let area = frame.area();
                let area = Rect {
                    x: area.width / 4,
                    y: area.height / 4,
                    width: area.width / 2,
                    height: area.height / 2,
                };
                Clear.render(area, frame.buffer_mut());
                let widget = if let Some(index) = podcast_list_state.selected() {
                    let podcast = &podcasts[index];
                    let lines = vec![
                        Line::from(vec!["Title: ".bold().into(), podcast.title.as_str().into()]),
                        Line::from(vec![
                            "Description: ".bold().into(),
                            podcast.description.as_str().into(),
                        ]),
                        Line::from(vec!["Source: ".bold().into(), podcast.url.as_str().into()]),
                    ];
                    podcast_info_scroll_state =
                        podcast_info_scroll_state.content_length(lines.len());
                    Paragraph::new(lines)
                        .wrap(Wrap { trim: true })
                        .scroll((podcast_info_scroll, 0))
                } else {
                    Paragraph::new("No info")
                }
                .block(
                    Block::bordered()
                        .title("Podcast Info")
                        .border_type(BorderType::Thick),
                );

                frame.render_widget(widget, area);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight),
                    area,
                    &mut podcast_info_scroll_state,
                );
            }

            if matches!(current_view, View::EpisodeInfo) {
                let area = frame.area();
                let area = Rect {
                    x: area.width / 4,
                    y: area.height / 4,
                    width: area.width / 2,
                    height: area.height / 2,
                };
                Clear.render(area, frame.buffer_mut());
                let widget = if let Some(podcast_index) = podcast_list_state.selected() {
                    let podcast = &podcasts[podcast_index];
                    if let Some(episode_index) = episode_list_state.selected() {
                        let episode = &podcast.episodes[episode_index];
                        let lines = vec![
                            Line::from(vec![
                                "Podcast: ".bold().into(),
                                podcast.title.as_str().into(),
                            ]),
                            Line::from(vec![
                                "Title: ".bold().into(),
                                episode.title.as_str().into(),
                            ]),
                            Line::from(vec!["Date: ".bold().into(), episode.date.as_str().into()]),
                            Line::from(vec![
                                "Audio Source: ".bold().into(),
                                episode.audio_url.as_str().into(),
                            ]),
                            Line::from(vec![
                                "Description: ".bold().into(),
                                episode.description.as_str().into(),
                            ]),
                        ];
                        episode_info_scroll_state =
                            episode_info_scroll_state.content_length(lines.len());
                        Paragraph::new(lines)
                            .wrap(Wrap { trim: true })
                            .scroll((episode_info_scroll, 0))
                    } else {
                        Paragraph::new("No info")
                    }
                } else {
                    Paragraph::new("No info")
                }
                .block(
                    Block::bordered()
                        .title("Episode Info")
                        .border_type(BorderType::Thick),
                );

                frame.render_widget(widget, area);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight),
                    area,
                    &mut episode_info_scroll_state,
                );
            }

            if matches!(current_view, View::AddPodcast) {
                let area = frame.area();
                let area = Rect {
                    x: area.width / 4,
                    y: (area.height - 3) / 2,
                    width: area.width / 2,
                    height: 3,
                };
                Clear.render(area, frame.buffer_mut());
                frame.render_widget(
                    Paragraph::new(url_to_add.as_str()).block(
                        Block::bordered()
                            .title("Add Podcast")
                            .title_bottom("p: paste")
                            .border_type(BorderType::Thick),
                    ),
                    area,
                );
            }
        })?;

        match event::read()? {
            Event::Key(key_event) if key_event.is_press() => {
                match key_event.code {
                    KeyCode::Char('q') => is_running = false,
                    KeyCode::Char('a') => {
                        url_to_add.clear();
                        current_view = View::AddPodcast;
                    }
                    KeyCode::Char('u') => {
                        for podcast in podcasts.iter_mut() {
                            let podcast_text = reqwest::get(&podcast.url).await?.text().await?;
                            *podcast = parse_podcast_data(&podcast.url, &podcast_text).await?;
                            let feed_path =
                                data_path.clone().join(&podcast.title).join("feed.json");
                            let contents = serde_json::to_string_pretty(&podcast)?;
                            tokio::fs::write(feed_path, contents).await?;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(sink) = &player_sink {
                            if sink.is_paused() {
                                sink.play();
                            } else {
                                sink.pause();
                            }
                        }
                    }
                    _ => {}
                }

                match current_view {
                    View::PodcastList => match key_event.code {
                        // TODO(miobi): support delete podcast
                        KeyCode::Enter => {
                            if let Some(index) = podcast_list_state.selected() {
                                if !podcasts[index].episodes.is_empty() {
                                    episode_list_state.select_first();
                                }
                            }
                            current_view = View::EpisodeList;
                        }
                        KeyCode::Char('k') => {
                            podcast_list_state.select_previous();
                            podcast_list_scroll_state.prev();
                            episode_list_state.select_first();
                            episode_list_scroll_state.first();
                        }
                        KeyCode::Char('j') => {
                            podcast_list_state.select_next();
                            podcast_list_scroll_state.next();
                            episode_list_state.select_first();
                            episode_list_scroll_state.first();
                        }
                        KeyCode::Char('i') => {
                            podcast_info_scroll = 0;
                            podcast_info_scroll_state.first();
                            current_view = View::PodcastInfo;
                        }
                        _ => {}
                    },
                    View::PodcastInfo => match key_event.code {
                        KeyCode::Esc => current_view = View::PodcastList,
                        KeyCode::Char('k') => {
                            podcast_info_scroll = podcast_info_scroll.saturating_sub(1);
                            podcast_info_scroll_state.prev();
                        }
                        KeyCode::Char('j') => {
                            podcast_info_scroll = podcast_info_scroll.saturating_add(1);
                            podcast_info_scroll_state.next();
                        }
                        _ => {}
                    },
                    View::AddPodcast => match key_event.code {
                        KeyCode::Esc => current_view = View::PodcastList,
                        KeyCode::Char('p') => {
                            let clipboard = Clipboard::new()?.get_text()?;
                            url_to_add = clipboard;
                        }
                        KeyCode::Enter => {
                            match podcasts.iter().find(|podcast| podcast.url == url_to_add) {
                                Some(_) => {
                                    // TODO(miobi): notify duplicate
                                }
                                None => {
                                    let podcast_text =
                                        reqwest::get(&url_to_add).await?.text().await?;
                                    let podcast =
                                        parse_podcast_data(&url_to_add, &podcast_text).await?;
                                    let podcast_path = data_path.clone().join(&podcast.title);
                                    tokio::fs::create_dir(&podcast_path).await?;
                                    let feed_path = podcast_path.join("feed.json");
                                    let contents = serde_json::to_string_pretty(&podcast)?;
                                    tokio::fs::write(feed_path, contents).await?;

                                    podcasts.push(podcast);
                                    podcast_list_state.select_next();
                                    current_view = View::PodcastList;
                                }
                            }
                        }
                        _ => {}
                    },
                    View::EpisodeList => match key_event.code {
                        KeyCode::Esc => current_view = View::PodcastList,
                        KeyCode::Enter => {
                            if let Some(episode_index) = episode_list_state.selected() {
                                let podcast = &podcasts[podcast_list_state.selected().unwrap()];
                                let episode = &podcast.episodes[episode_index];

                                // TODO(miobi): support other audio mimetype
                                let podcast_audio_path = data_path
                                    .clone()
                                    .join(&podcast.title)
                                    .join(&episode.title)
                                    .with_extension("mp3");
                                if !podcast_audio_path.exists() {
                                    let podcast_episode_audio =
                                        reqwest::get(&episode.audio_url).await?.bytes().await?;
                                    tokio::fs::write(&podcast_audio_path, podcast_episode_audio)
                                        .await?;
                                }

                                let audio = BufReader::new(File::open(podcast_audio_path)?);
                                player_sink =
                                    Some(rodio::play(&player_stream_handle.mixer(), audio)?);
                                player_title = episode.title.clone();
                            }
                        }
                        KeyCode::Char('k') => {
                            episode_list_state.select_previous();
                            episode_list_scroll_state.prev();
                        }
                        KeyCode::Char('j') => {
                            episode_list_state.select_next();
                            episode_list_scroll_state.next();
                        }
                        KeyCode::Char('i') => {
                            episode_info_scroll = 0;
                            episode_info_scroll_state.first();
                            current_view = View::EpisodeInfo;
                        }
                        _ => {}
                    },
                    View::EpisodeInfo => match key_event.code {
                        KeyCode::Esc => current_view = View::EpisodeList,
                        KeyCode::Char('k') => {
                            episode_info_scroll = episode_info_scroll.saturating_sub(1);
                            episode_info_scroll_state.prev();
                        }
                        KeyCode::Char('j') => {
                            episode_info_scroll = episode_info_scroll.saturating_add(1);
                            episode_info_scroll_state.next();
                        }
                        _ => {}
                    },
                }
            }
            _ => {}
        }
    }

    ratatui::restore();
    Ok(())
}

async fn parse_podcast_data(
    url: &str,
    text: &str,
) -> Result<Podcast, Box<dyn Error + Send + Sync>> {
    let root = roxmltree::Document::parse(&text)?;
    let channel = root
        .descendants()
        .find(|node| node.has_tag_name("channel"))
        .unwrap();
    let title = channel
        .children()
        .find(|node| node.has_tag_name("title"))
        .unwrap()
        .text()
        .unwrap()
        .to_string();
    let description = channel
        .children()
        .find(|node| node.has_tag_name("description"))
        .unwrap()
        .text()
        .unwrap_or_default()
        .to_string();

    let episodes = channel
        .children()
        .filter(|node| node.has_tag_name("item"))
        .map(|node| {
            let title = node
                .children()
                .find(|node| node.has_tag_name("title"))
                .unwrap()
                .text()
                .unwrap()
                .to_string();
            let description = node
                .children()
                .find(|node| node.has_tag_name("description"))
                .unwrap()
                .text()
                .unwrap_or_default()
                .to_string();
            let date = DateTime::parse_from_rfc2822(
                node.children()
                    .find(|node| node.has_tag_name("pubDate"))
                    .unwrap()
                    .text()
                    .unwrap_or_default(),
            )
            .unwrap_or_default()
            .date_naive()
            .to_string();

            let audio_url = node
                .children()
                .find(|node| node.has_tag_name("enclosure"))
                .unwrap()
                .attribute("url")
                .unwrap()
                .to_string();

            Episode {
                title,
                description,
                date,
                audio_url,
            }
        })
        .collect();

    Ok(Podcast {
        title,
        description,
        url: url.to_string(),
        episodes,
    })
}
