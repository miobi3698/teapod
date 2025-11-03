use std::{error::Error, fmt::Display, time::Duration};

use chrono::{DateTime, FixedOffset};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Cell, List, Paragraph, Row, Table, TableState},
};

struct Podcast {
    title: String,
    description: String,
    url: String,
    episodes: Vec<Episode>,
}

struct Episode {
    title: String,
    description: String,
    date: DateTime<FixedOffset>,
    audio: Audio,
}

struct Audio {
    mime_type: String,
    length: usize,
    url: String,
}

#[derive(Debug)]
enum RssParseError {
    MissingTag,
    MissingValue,
    MissingAttr,
}

impl Display for RssParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for RssParseError {}

async fn download_podcast_info(url: &str) -> Result<Podcast, Box<dyn Error>> {
    let text = reqwest::get(url).await?.text().await?;
    let rss = roxmltree::Document::parse(text.as_str())?;
    let channel = rss
        .descendants()
        .find(|elem| elem.has_tag_name("channel"))
        .ok_or(RssParseError::MissingTag)?;
    let channel_title = channel
        .children()
        .find(|elem| elem.has_tag_name("title"))
        .ok_or(RssParseError::MissingTag)?
        .text()
        .ok_or(RssParseError::MissingValue)?
        .to_string();
    let channel_description = channel
        .children()
        .find(|elem| elem.has_tag_name("title"))
        .ok_or(RssParseError::MissingTag)?
        .text()
        .ok_or(RssParseError::MissingValue)?
        .to_string();
    let episodes: Result<Vec<_>, Box<dyn Error>> = channel
        .children()
        .filter(|elem| elem.has_tag_name("item"))
        .map(|elem| {
            let episode_title = elem
                .children()
                .find(|e| e.has_tag_name("title"))
                .ok_or(RssParseError::MissingTag)?
                .text()
                .ok_or(RssParseError::MissingValue)?
                .to_string();
            let episode_description = elem
                .children()
                .find(|e| e.has_tag_name("description"))
                .ok_or(RssParseError::MissingTag)?
                .text()
                .ok_or(RssParseError::MissingValue)?
                .to_string();
            let episode_date = DateTime::parse_from_rfc2822(
                elem.children()
                    .find(|e| e.has_tag_name("pubDate"))
                    .ok_or(RssParseError::MissingTag)?
                    .text()
                    .ok_or(RssParseError::MissingValue)?,
            )?;

            let episode_audio = {
                let elem = elem
                    .children()
                    .find(|e| e.has_tag_name("enclosure"))
                    .ok_or(RssParseError::MissingTag)?;

                Audio {
                    mime_type: elem
                        .attribute("type")
                        .ok_or(RssParseError::MissingAttr)?
                        .to_string(),
                    length: elem
                        .attribute("length")
                        .ok_or(RssParseError::MissingAttr)?
                        .parse()?,
                    url: elem
                        .attribute("url")
                        .ok_or(RssParseError::MissingAttr)?
                        .to_string(),
                }
            };

            Ok(Episode {
                title: episode_title,
                description: episode_description,
                date: episode_date,
                audio: episode_audio,
            })
        })
        .collect();

    Ok(Podcast {
        title: channel_title,
        description: channel_description,
        url: url.to_string(),
        episodes: episodes?,
    })
}

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
    let mut table_state = TableState::default().with_selected(0);
    let mut current_view = View::Podcast;

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
                        ])
                    }),
                    [Constraint::Fill(1), Constraint::Length(10)],
                )
                .header(Row::new(vec!["Title", "Date", "Duration"]).underlined())
                .row_highlight_style(Style::default().reversed())
                .block(episodes_border),
                podcast_episode_list_area,
                &mut table_state,
            );
        })?;

        while event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Char('q') => break 'main_loop,
                        KeyCode::Char('a') => {
                            // TODO(miobi): implement this
                            // current_view = View::Add;
                            let url = "https://changelog.fm/rss";
                            let podcast = download_podcast_info(url).await?;
                            podcasts.push(podcast);
                        }
                        KeyCode::Char('u') => {
                            // TODO(miobi): implement this
                            // current_view = View::Update
                            for podcast in podcasts.iter_mut() {
                                *podcast = download_podcast_info(podcast.url.as_str()).await?;
                            }
                        }
                        _ => {}
                    }

                    match current_view {
                        View::Podcast => match key_event.code {
                            KeyCode::Char('j') => {
                                selected_podcast =
                                    selected_podcast.saturating_add(1).min(podcasts.len() - 1)
                            }
                            KeyCode::Char('k') => {
                                selected_podcast = selected_podcast.saturating_sub(1)
                            }
                            KeyCode::Enter => current_view = View::Episode,
                            _ => {}
                        },
                        View::Episode => match key_event.code {
                            KeyCode::Esc => current_view = View::Podcast,
                            KeyCode::Char('j') => table_state.select_next(),
                            KeyCode::Char('k') => table_state.select_previous(),
                            _ => {}
                        },
                        View::Add => match key_event.code {
                            KeyCode::Esc => current_view = View::Podcast,
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
