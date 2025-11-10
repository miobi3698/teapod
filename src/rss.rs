use std::{error::Error, fmt::Display, io, path::PathBuf};

use chrono::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Podcast {
    pub title: String,
    pub description: String,
    pub url: String,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Episode {
    pub title: String,
    pub description: String,
    pub date: String,
    audio: Audio,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Audio {
    mime_type: String,
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
        write!(f, "{:?}", self)
    }
}

impl Error for RssParseError {}

async fn download_podcast_info(url: String) -> Result<Podcast, Box<dyn Error + Send + Sync>> {
    let text = reqwest::get(url.as_str()).await?.text().await?;
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
    let episodes: Result<Vec<_>, Box<dyn Error + Send + Sync>> = channel
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
                .unwrap_or_default()
                .to_string();
            let episode_date = DateTime::parse_from_rfc2822(
                elem.children()
                    .find(|e| e.has_tag_name("pubDate"))
                    .ok_or(RssParseError::MissingTag)?
                    .text()
                    .ok_or(RssParseError::MissingValue)?,
            )?
            .date_naive()
            .to_string();

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

async fn save_podcast_info(podcast: &Podcast, path: PathBuf) -> io::Result<()> {
    let path = path.join(&podcast.title).with_extension("json");
    let contents = serde_json::to_string_pretty(podcast)?;
    tokio::fs::write(path, contents).await?;
    Ok(())
}

pub async fn download_and_save_podcast_info(
    url: String,
    path: PathBuf,
) -> Result<Podcast, Box<dyn Error + Send + Sync>> {
    let podcast = download_podcast_info(url).await?;
    save_podcast_info(&podcast, path).await?;
    Ok(podcast)
}

pub async fn load_podcast_info_from_file(path: PathBuf) -> io::Result<Podcast> {
    let contents = tokio::fs::read_to_string(&path).await?;
    let podcast = serde_json::from_str(&contents)?;
    Ok(podcast)
}
