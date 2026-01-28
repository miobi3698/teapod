use std::path::{Path, PathBuf};

use crate::AnyError;
use chrono::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Podcast {
    pub title: String,
    pub description: String,
    pub url: String,
    pub episodes: Vec<Episode>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub title: String,
    pub description: String,
    pub pub_date: String,
    pub url: String,
    pub mime_type: String,
}

pub const PODCAST_FEED_FILE: &str = "feed.json";

pub async fn download_podcast_info_from_url(url: &str) -> Result<Podcast, AnyError> {
    let res = reqwest::get(url).await?;
    let text = res.text().await?;
    let doc = roxmltree::Document::parse(text.as_str())?;

    let channel = doc
        .descendants()
        .find(|n| n.has_tag_name("channel"))
        .ok_or("missing channel tag")?;

    let title = channel
        .children()
        .find(|n| n.has_tag_name("title"))
        .ok_or("missing title tag")?
        .text()
        .unwrap_or_default()
        .to_string();
    let description = channel
        .children()
        .find(|n| n.has_tag_name("description"))
        .ok_or("missing description tag")?
        .text()
        .unwrap_or_default()
        .to_string();
    let url = url.to_string();

    let mut episodes = Vec::new();
    for item in channel.children().filter(|n| n.has_tag_name("item")) {
        let title = item
            .children()
            .find(|n| n.has_tag_name("title"))
            .ok_or("missing title tag")?
            .text()
            .unwrap_or_default()
            .to_string();
        let description = item
            .children()
            .find(|n| n.has_tag_name("description"))
            .ok_or("missing description tag")?
            .text()
            .unwrap_or_default()
            .to_string();
        let pub_date = DateTime::parse_from_rfc2822(
            item.children()
                .find(|n| n.has_tag_name("pubDate"))
                .ok_or("missing pubDate tag")?
                .text()
                .unwrap_or_default(),
        )?
        .date_naive()
        .to_string();

        let enclosure = item
            .children()
            .find(|n| n.has_tag_name("enclosure"))
            .ok_or("missing enclosure tag")?;
        let url = enclosure
            .attribute("url")
            .ok_or("missing url attr")?
            .to_string();
        let mime_type = enclosure
            .attribute("type")
            .ok_or("missing type attr")?
            .to_string();

        episodes.push(Episode {
            title,
            description,
            pub_date,
            url,
            mime_type,
        });
    }

    Ok(Podcast {
        title,
        description,
        url,
        episodes,
    })
}

pub async fn save_podcast_info_to_path(podcast: &Podcast, path: &Path) -> Result<(), AnyError> {
    let feed_dir = path.join(&podcast.title);
    if !feed_dir.exists() {
        tokio::fs::create_dir(&feed_dir).await?;
    }

    let feed_file = feed_dir.join(PODCAST_FEED_FILE);
    let json = serde_json::to_string(podcast)?;
    tokio::fs::write(feed_file, json).await?;
    Ok(())
}

pub async fn download_podcast_audio_to_path(podcast: &Podcast, episode: &Episode, path: &Path) -> Result<PathBuf, AnyError> {
    let mut audio_file = path.join(&podcast.title).join(&episode.title);
    match episode.mime_type.as_str() {
        "audio/mpeg" => {
            audio_file = audio_file.with_extension("mp3");
            if !audio_file.exists() {
                let res = reqwest::get(&episode.url).await?;
                let contents = res.bytes().await?;
                tokio::fs::write(&audio_file, contents).await?;
            }

            Ok(audio_file)
        }
        _ => Err("audio format not supported".into())
    }
}
