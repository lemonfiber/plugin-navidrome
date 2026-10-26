//! What Navidrome holds and plays, read as its administrator reads it. A member of the
//! household is somebody `identity.source` names, and Navidrome speaks for none of them,
//! so everything asked for one member is unsupported.

use std::collections::HashMap;

use async_trait::async_trait;
use lemonfiber_adapter_kit::endpoint::form_encoded;
use lemonfiber_ports::service::{
    Failure, Holds, HowFar, Image, Item, ItemDetail, ItemProgress, Medium, Picture, Playback,
    Screening, PICTURE_MOST,
};
use serde::Deserialize;

use super::{an_id, Navidrome, ABSENT};

const NO_MEMBERS: &str =
    "Navidrome speaks for none of the household's accounts, so nothing is asked for one";

const NO_DEVICES: &str = "Navidrome opens and ends no device's session for lemonfiber";

const ALBUMS: &str = "/api/album";

const USERS: &str = "/api/user";

const COVER: &str = "getCoverArt";

const NOW_PLAYING: &str = "getNowPlaying";

const ALBUM_COVER: &str = "al-";

const ARTWORK_ABSENT: u32 = 70;

const PICTURE_PREFIX: &str = "image/";

const SECONDS_IN_A_MINUTE: f64 = 60.0;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Album {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    max_year: Option<u16>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    song_count: u32,
    #[serde(default)]
    image_hash: Option<String>,
    #[serde(default)]
    genres: Option<Vec<Genre>>,
}

#[derive(Deserialize)]
struct Genre {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NowPlaying {
    now_playing: Entries,
}

#[derive(Deserialize, Default)]
struct Entries {
    #[serde(default)]
    entry: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    #[serde(default)]
    username: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    player_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    user_name: String,
}

impl Album {
    fn pictured(&self) -> bool {
        self.image_hash
            .as_deref()
            .is_some_and(|hash| !hash.is_empty())
    }

    fn item(&self) -> Item {
        Item {
            id: self.id.clone(),
            title: self.name.clone(),
            year: self.max_year.filter(|year| *year > 0),
            medium: Medium::Other,
            holds: Holds {
                poster: self.pictured(),
                backdrop: false,
                plays: self.song_count > 0,
            },
        }
    }

    fn detail(self) -> ItemDetail {
        let item = self.item();
        ItemDetail {
            item,
            overview: self.comment.filter(|said| !said.trim().is_empty()),
            minutes: self.duration.and_then(whole_minutes),
            genres: self
                .genres
                .unwrap_or_default()
                .into_iter()
                .map(|genre| genre.name)
                .collect(),
            certificate: None,
            released: self.date.filter(|date| a_date(date)),
            seasons: Vec::new(),
        }
    }
}

/// `seconds` in whole minutes, nothing where it is not a length.
pub(super) fn whole_minutes(seconds: f64) -> Option<u32> {
    format!("{:.0}", seconds / SECONDS_IN_A_MINUTE).parse().ok()
}

/// Whether `said` is a calendar date written as `YYYY-MM-DD`.
pub(super) fn a_date(said: &str) -> bool {
    said.len() == 10
        && said.bytes().enumerate().all(|(at, mark)| match at {
            4 | 7 => mark == b'-',
            _ => mark.is_ascii_digit(),
        })
}

impl Navidrome {
    async fn album(&self, id: &str) -> Result<Option<Album>, Failure> {
        if !an_id(id) {
            return Ok(None);
        }
        let response = self.as_admin(&format!("{ALBUMS}/{id}")).await?;
        if response.status == ABSENT {
            return Ok(None);
        }
        self.endpoint
            .decode(&response, "the album Navidrome holds could not be read")
            .map(Some)
    }

    async fn poster(&self, id: &str) -> Result<Option<Image>, Failure> {
        if !self.album(id).await?.is_some_and(|album| album.pictured()) {
            return Ok(None);
        }
        let cover = format!("{ALBUM_COVER}{id}");
        let fetched = match self
            .fetched_subsonic(COVER, &[("id", &cover)], PICTURE_MOST)
            .await?
        {
            Ok(fetched) => fetched,
            Err(error) if error.code == ARTWORK_ABSENT => return Ok(None),
            Err(error) => return Err(self.subsonic_refusal(&error)),
        };
        if !fetched.is_success() {
            return Err(self
                .endpoint
                .refused("Navidrome would not answer the cover"));
        }
        let Some(media_type) = fetched
            .header("content-type")
            .filter(|kind| kind.starts_with(PICTURE_PREFIX))
            .map(str::to_owned)
        else {
            return Ok(None);
        };
        Ok(fetched.bytes.map(|bytes| Image { media_type, bytes }))
    }

    async fn albums(&self, most: u32) -> Result<Vec<Item>, Failure> {
        let asked = form_encoded(&[
            ("_start", "0"),
            ("_end", &most.to_string()),
            ("_sort", "name"),
            ("_order", "ASC"),
        ]);
        let albums: Vec<Album> = self
            .read(
                &format!("{ALBUMS}?{asked}"),
                "the albums Navidrome holds could not be read",
            )
            .await?;
        Ok(albums
            .iter()
            .take(usize::try_from(most).unwrap_or(usize::MAX))
            .map(Album::item)
            .collect())
    }

    async fn played(&self) -> Result<Vec<Playback>, Failure> {
        let playing: NowPlaying = self
            .subsonic(
                NOW_PLAYING,
                &[],
                "what Navidrome is playing could not be read",
            )
            .await?;
        if playing.now_playing.entry.is_empty() {
            return Ok(Vec::new());
        }
        let users: Vec<User> = self
            .read(USERS, "the accounts Navidrome holds could not be read")
            .await?;
        let filed: HashMap<String, String> = users
            .into_iter()
            .map(|user| (user.user_name, user.id))
            .collect();
        Ok(playing
            .now_playing
            .entry
            .into_iter()
            .map(|entry| Playback {
                member_id: filed.get(&entry.username).cloned().unwrap_or_default(),
                member: entry.username,
                title: entry.title,
                series: None,
                season: None,
                episode: None,
                medium: Medium::Other,
                paused: false,
                device: entry.player_name,
            })
            .collect())
    }
}

#[async_trait]
impl Screening for Navidrome {
    async fn signed_in(&self, _member: &str, _device: &str) -> Result<Option<String>, Failure> {
        Err(Self::unsupported(NO_MEMBERS))
    }

    async fn title(&self, member: Option<&str>, id: &str) -> Result<Option<ItemDetail>, Failure> {
        if member.is_some() {
            return Err(Self::unsupported(NO_MEMBERS));
        }
        Ok(self.album(id).await?.map(Album::detail))
    }

    async fn part_way(&self, _member: &str, _most: u32) -> Result<Vec<ItemProgress>, Failure> {
        Err(Self::unsupported(NO_MEMBERS))
    }

    async fn progressed(&self, _member: &str, _id: &str, _how_far: &HowFar) -> Result<(), Failure> {
        Err(Self::unsupported(NO_MEMBERS))
    }

    async fn sign_out(&self, _device: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_DEVICES))
    }

    async fn picture(
        &self,
        member: Option<&str>,
        id: &str,
        which: Picture,
    ) -> Result<Option<Image>, Failure> {
        if member.is_some() {
            return Err(Self::unsupported(NO_MEMBERS));
        }
        match which {
            Picture::Poster => self.poster(id).await,
            Picture::Backdrop => Ok(None),
        }
    }

    async fn holdings(&self, member: Option<&str>, most: u32) -> Result<Vec<Item>, Failure> {
        if member.is_some() {
            return Err(Self::unsupported(NO_MEMBERS));
        }
        self.albums(most).await
    }

    async fn playing(&self, member: Option<&str>) -> Result<Vec<Playback>, Failure> {
        if member.is_some() {
            return Err(Self::unsupported(NO_MEMBERS));
        }
        self.played().await
    }
}
