use std::{error::Error, fmt::Display};

use chrono::{DateTime, FixedOffset};

pub struct Podcast {
    pub title: String,
    description: String,
    pub url: String,
    pub episodes: Vec<Episode>,
}

pub struct Episode {
    pub title: String,
    description: String,
    pub date: DateTime<FixedOffset>,
    audio: Audio,
    pub duration: String,
}

pub struct Audio {
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

pub async fn download_podcast_info(url: &str) -> Result<Podcast, Box<dyn Error>> {
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
            let episode_duration = elem
                .children()
                .find(|e| e.has_tag_name("itunes:duration"))
                .ok_or(RssParseError::MissingTag)?
                .text()
                .ok_or(RssParseError::MissingValue)?
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
                duration: episode_duration,
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
