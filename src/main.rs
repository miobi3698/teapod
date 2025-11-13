use std::{error::Error, fs::File, io::BufReader, time::Duration};

use arboard::Clipboard;
use chrono::DateTime;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use rodio::{Decoder, Sink, Source};
use serde::{Deserialize, Serialize};

use crate::components::{
    add_podcast_popup::{AddPodcastPopup, AddPodcastPopupState},
    episode_info_popup::{EpisodeInfoPopup, EpisodeInfoPopupState},
    episode_list::{EpisodeList, EpisodeListState},
    player::Player,
    podcast_info_popup::{PodcastInfoPopup, PodcastInfoPopupState},
    podcast_list::{PodcastList, PodcastListState},
};

mod components;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Podcast {
    title: String,
    description: String,
    url: String,
    episodes: Vec<Episode>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Episode {
    title: String,
    description: String,
    date: String,
    audio_url: String,
}

enum View {
    PodcastList,
    EpisodeList,
}

enum Popup {
    PodcastInfo,
    AddPodcast,
    EpisodeInfo,
}

struct Audio {
    title: String,
    total_duration: Duration,
    sink: Sink,
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

    let player_stream_handle = {
        let mut handle = rodio::OutputStreamBuilder::open_default_stream()?;
        handle.log_on_drop(false);
        handle
    };

    let mut terminal = ratatui::init();

    let mut current_view = View::PodcastList;
    let mut current_popup: Option<Popup> = None;

    let mut podcast_list_state = PodcastListState::default();
    let mut podcast_info_popup_state = PodcastInfoPopupState::default();
    let mut episode_list_state = EpisodeListState::default();
    let mut episode_info_popup_state = EpisodeInfoPopupState::default();
    let mut add_podcast_popup_state = AddPodcastPopupState::default();

    let mut player_audio: Option<Audio> = None;

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

            podcast_list_state.focus(matches!(current_view, View::PodcastList));
            frame.render_stateful_widget(
                PodcastList::new(&podcasts),
                podcast_list_area,
                &mut podcast_list_state,
            );

            let episodes = podcast_list_state
                .selected()
                .map(|index| podcasts[index].episodes.as_slice())
                .unwrap_or_default();
            episode_list_state.focus(matches!(current_view, View::EpisodeList));
            frame.render_stateful_widget(
                EpisodeList::new(episodes),
                episode_list_area,
                &mut episode_list_state,
            );

            frame.render_widget(Player::new(&player_audio), player_area);

            frame.render_widget(
                Paragraph::new("q: quit, a: add, u: update, d: delete"),
                footer_area,
            );

            if let Some(popup) = &current_popup {
                match popup {
                    Popup::PodcastInfo => frame.render_stateful_widget(
                        PodcastInfoPopup::new(
                            podcast_list_state.selected().map(|index| &podcasts[index]),
                        ),
                        frame.area(),
                        &mut podcast_info_popup_state,
                    ),
                    Popup::AddPodcast => frame.render_stateful_widget(
                        AddPodcastPopup::new(),
                        frame.area(),
                        &mut add_podcast_popup_state,
                    ),
                    Popup::EpisodeInfo => frame.render_stateful_widget(
                        EpisodeInfoPopup::new(
                            episode_list_state.selected().map(|index| &episodes[index]),
                        ),
                        frame.area(),
                        &mut episode_info_popup_state,
                    ),
                }
            }
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.is_press() => {
                    match key_event.code {
                        KeyCode::Char('q') => is_running = false,
                        KeyCode::Char('a') => {
                            add_podcast_popup_state.url.clear();
                            current_popup = Some(Popup::AddPodcast);
                        }
                        KeyCode::Char('u') => {
                            // TODO(miobi): handle error
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
                            if let Some(audio) = &player_audio {
                                if audio.sink.is_paused() {
                                    audio.sink.play();
                                } else {
                                    audio.sink.pause();
                                }
                            }
                        }
                        _ => {}
                    }

                    if let Some(popup) = &current_popup {
                        match popup {
                            Popup::PodcastInfo => match key_event.code {
                                KeyCode::Esc => {
                                    current_popup = None;
                                    current_view = View::PodcastList;
                                }
                                KeyCode::Char('k') => podcast_info_popup_state.prev(),
                                KeyCode::Char('j') => podcast_info_popup_state.next(),
                                _ => {}
                            },
                            Popup::AddPodcast => match key_event.code {
                                KeyCode::Esc => {
                                    current_popup = None;
                                    current_view = View::PodcastList
                                }
                                KeyCode::Char('p') => {
                                    // TODO(miobi): handle error
                                    add_podcast_popup_state.url = Clipboard::new()?.get_text()?;
                                }
                                KeyCode::Enter => {
                                    match podcasts
                                        .iter()
                                        .find(|podcast| podcast.url == add_podcast_popup_state.url)
                                    {
                                        Some(_) => {
                                            // TODO(miobi): notify duplicate
                                        }
                                        None => {
                                            // TODO(miobi): handle error
                                            let podcast_text =
                                                reqwest::get(&add_podcast_popup_state.url)
                                                    .await?
                                                    .text()
                                                    .await?;
                                            let podcast = parse_podcast_data(
                                                &add_podcast_popup_state.url,
                                                &podcast_text,
                                            )
                                            .await?;
                                            let podcast_path =
                                                data_path.clone().join(&podcast.title);
                                            tokio::fs::create_dir(&podcast_path).await?;
                                            let feed_path = podcast_path.join("feed.json");
                                            let contents = serde_json::to_string_pretty(&podcast)?;
                                            tokio::fs::write(feed_path, contents).await?;

                                            podcasts.push(podcast);
                                            podcast_list_state.next();
                                            current_view = View::PodcastList;
                                        }
                                    }
                                }
                                _ => {}
                            },
                            Popup::EpisodeInfo => match key_event.code {
                                KeyCode::Esc => {
                                    current_popup = None;
                                    current_view = View::EpisodeList;
                                }
                                KeyCode::Char('k') => episode_info_popup_state.prev(),
                                KeyCode::Char('j') => episode_info_popup_state.next(),
                                _ => {}
                            },
                        }
                    } else {
                        match &current_view {
                            View::PodcastList => match key_event.code {
                                KeyCode::Enter => current_view = View::EpisodeList,
                                KeyCode::Char('k') => {
                                    podcast_list_state.prev();
                                    episode_list_state.first();
                                }
                                KeyCode::Char('j') => {
                                    podcast_list_state.next();
                                    episode_list_state.first();
                                }
                                KeyCode::Char('i') => {
                                    podcast_info_popup_state.first();
                                    current_popup = Some(Popup::PodcastInfo);
                                }
                                KeyCode::Char('d') => {
                                    // TODO(miobi): support delete podcast
                                }
                                _ => {}
                            },
                            View::EpisodeList => match key_event.code {
                                KeyCode::Esc => current_view = View::PodcastList,
                                KeyCode::Enter => {
                                    if let Some(episode_index) = episode_list_state.selected() {
                                        let podcast =
                                            &podcasts[podcast_list_state.selected().unwrap()];
                                        let episode = &podcast.episodes[episode_index];

                                        // TODO(miobi): support other audio mimetype
                                        let podcast_audio_path = data_path
                                            .clone()
                                            .join(&podcast.title)
                                            .join(&episode.title)
                                            .with_extension("mp3");
                                        if !podcast_audio_path.exists() {
                                            let podcast_episode_audio =
                                                reqwest::get(&episode.audio_url)
                                                    .await?
                                                    .bytes()
                                                    .await?;
                                            tokio::fs::write(
                                                &podcast_audio_path,
                                                podcast_episode_audio,
                                            )
                                            .await?;
                                        }

                                        let audio_data =
                                            BufReader::new(File::open(podcast_audio_path)?);
                                        let source = Decoder::try_from(audio_data)?;
                                        let total_duration = source.total_duration().unwrap();
                                        let sink = Sink::connect_new(player_stream_handle.mixer());
                                        sink.append(source);

                                        player_audio = Some(Audio {
                                            title: episode.title.clone(),
                                            total_duration,
                                            sink,
                                        })
                                    }
                                }
                                KeyCode::Char('k') => episode_list_state.prev(),
                                KeyCode::Char('j') => episode_list_state.next(),
                                KeyCode::Char('i') => {
                                    episode_info_popup_state.first();
                                    current_popup = Some(Popup::EpisodeInfo);
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

async fn parse_podcast_data(
    url: &str,
    text: &str,
) -> Result<Podcast, Box<dyn Error + Send + Sync>> {
    let root = roxmltree::Document::parse(&text)?;
    let channel = root
        .descendants()
        .find(|node| node.has_tag_name("channel"))
        .unwrap();

    let mut podcast = Podcast::default();
    podcast.url = url.to_string();
    for node in channel.children() {
        match node.tag_name().name() {
            "title" => podcast.title = node.text().unwrap().to_string(),
            "description" => podcast.description = node.text().unwrap_or_default().to_string(),
            "item" => {
                let mut episode = Episode::default();
                for subnode in node.children() {
                    match subnode.tag_name().name() {
                        "title" => episode.title = subnode.text().unwrap().to_string(),
                        "description" => {
                            episode.description = subnode.text().unwrap_or_default().to_string()
                        }
                        "pubDate" => {
                            episode.date =
                                DateTime::parse_from_rfc2822(subnode.text().unwrap_or_default())
                                    .unwrap_or_default()
                                    .date_naive()
                                    .to_string()
                        }
                        "enclosure" => {
                            episode.audio_url = subnode.attribute("url").unwrap().to_string()
                        }
                        _ => {}
                    }
                }
                podcast.episodes.push(episode);
            }
            _ => {}
        }
    }

    Ok(podcast)
}
