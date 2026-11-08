//! TV Shows on Home: series with a new episode that Lampa's viewers watch now, from the CUB feed
//! behind Lampa's «Сейчас смотрят» (no account needed). OK opens Lampa. Requests and scheduling
//! mirror CS2 Scores: one job in flight, generation IDs, posters on their own worker.
use crate::{app_artwork::Artwork, cub, i18n::Language};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use time::Date;

/// TMDB's posters a size above the cards, and the mirror Lampa uses where TMDB is blocked.
const POSTER_ORIGINS: [&str; 2] = [
    "https://image.tmdb.org/t/p/w342",
    "https://imagetmdb.com/t/p/w342",
];
/// The Lampa plugin that opens the cards Luma asks for (Lampa on webOS ignores launch
/// parameters); it lives beside Luma+ in `lampa/` and is served from the fork's main branch.
pub const LAMPA_PLUGIN: &str =
    "https://cdn.jsdelivr.net/gh/eluz1ve/lg-luma-plus@main/lampa/luma.js";
/// The row lists; six cards show at a time, the favorites' width and pitch.
pub const SHOWN: usize = 18;
pub const VISIBLE: usize = 6;
/// The feed's pages read, twenty series each; about half have a new episode.
pub const PAGES: u32 = 3;
/// A series is new while its latest episode is this many days old at most.
const FRESH_DAYS: i64 = 21;
/// CUB keeps the feed for hours; a failed fetch is retried sooner.
pub const REFRESH_SECONDS: i64 = 3 * 3600;
const RETRY_SECONDS: i64 = 5 * 60;
/// Posters kept in memory; more are forgotten and fetched again if needed.
const POSTER_CACHE: usize = 48;
/// CUB has its posters only in Russian; elsewhere a series' poster comes from TVmaze (free, no
/// key, CC BY-SA), found by its IMDb id. Such a poster's key is "tvmaze:{imdb}{tmdb path}".
const TVMAZE: &str = "tvmaze:";
const TVMAZE_SHOWS: &str = "https://api.tvmaze.com/shows/";
const TVMAZE_IMAGES: &str = "https://static.tvmaze.com/uploads/images/";

#[derive(Clone, Debug, PartialEq)]
pub struct Show {
    /// TMDB's id, as Lampa's card "tv_{id}".
    pub id: i64,
    /// Its Russian title, CUB's only translation.
    pub name: String,
    pub original: String,
    /// TMDB's poster path, e.g. "/1CGw.jpg".
    pub poster: Option<String>,
    /// IMDb's id, e.g. "tt2531336", for its English poster.
    pub imdb: Option<String>,
    /// The latest season.
    pub season: Option<u32>,
    /// The day its latest episode came out.
    pub aired: Date,
    /// TMDB's rating out of 10, when enough people voted.
    pub rating: Option<f32>,
}

impl Show {
    pub fn title(&self, language: Language) -> &str {
        if language == Language::Ru {
            &self.name
        } else {
            &self.original
        }
    }

    pub fn poster_key(&self, language: Language) -> Option<String> {
        poster_key(self.poster.as_deref(), self.imdb.as_deref(), language)
    }

    /// Whether a new episode is out on `date`: dated that day, or the day before where it aired
    /// west of the TV.
    pub fn new_on(&self, date: Date) -> bool {
        self.aired == date || date.previous_day() == Some(self.aired)
    }
}

/// The poster to show in `language`: TVmaze's English one where there is an IMDb id, falling back
/// to CUB's (TMDB `path`), which is CUB's only one, in Russian.
pub fn poster_key(path: Option<&str>, imdb: Option<&str>, language: Language) -> Option<String> {
    match imdb {
        Some(imdb) if language != Language::Ru => {
            Some(format!("{TVMAZE}{imdb}{}", path.unwrap_or_default()))
        }
        _ => path.map(str::to_owned),
    }
}

/// A poster key's IMDb id to ask TVmaze for, and its TMDB path.
pub fn poster_source(key: &str) -> (Option<&str>, Option<&str>) {
    match key.strip_prefix(TVMAZE) {
        Some(rest) => match rest.find('/') {
            Some(slash) => (Some(&rest[..slash]), Some(&rest[slash..])),
            None => (Some(rest), None),
        },
        None => (None, Some(key)),
    }
}

/// An IMDb title id: "tt" and digits.
pub fn imdb_id(value: Option<&Value>) -> Option<String> {
    let id = value?.as_str()?;
    let digits = id.strip_prefix("tt")?;
    ((5..=10).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| id.to_owned())
}

/// TVmaze's lookup, which redirects to the series' own address.
pub fn tvmaze_lookup(imdb: &str) -> String {
    format!("https://api.tvmaze.com/lookup/shows?imdb={imdb}")
}

/// Whether a lookup's redirect is to a series on TVmaze.
pub fn tvmaze_show(url: &str) -> bool {
    url.strip_prefix(TVMAZE_SHOWS)
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
}

/// The series' full-size poster, from TVmaze's own image host.
pub fn tvmaze_image(show: &Value) -> Option<String> {
    let url = show.get("image")?.get("original")?.as_str()?;
    (url.starts_with(TVMAZE_IMAGES) && !url.contains("..") && url.len() <= 300)
        .then(|| url.to_owned())
}

/// Where a poster `path` may be fetched, in order.
pub fn poster_urls(path: &str) -> Vec<String> {
    if cub::poster_path(Some(&Value::from(path))).is_none() {
        return Vec::new();
    }
    POSTER_ORIGINS
        .iter()
        .map(|origin| format!("{origin}{path}"))
        .collect()
}

fn show(value: &Value, date: Date) -> Option<Show> {
    let id = value.get("id")?.as_i64().filter(|id| *id > 0)?;
    let name = cub::text(value.get("name"), 120)?;
    let poster = Some(cub::poster_path(value.get("poster_path"))?);
    let aired = cub::day(value.get("last_air_date"))?;
    let age = (date - aired).whole_days();
    if !(-1..=FRESH_DAYS).contains(&age) {
        return None;
    }
    let season = value
        .get("seasons")
        .and_then(Value::as_object)
        .and_then(|seasons| seasons.keys().filter_map(|key| key.parse().ok()).max())
        .filter(|season: &u32| *season > 0);
    let rating = value
        .get("vote_average")
        .and_then(Value::as_f64)
        .filter(|rating| *rating > 0. && *rating <= 10.)
        .filter(|_| value.get("vote_count").and_then(Value::as_u64) >= Some(10))
        .map(|rating| (rating * 10.).round() as f32 / 10.);
    Some(Show {
        id,
        original: cub::text(value.get("original_name"), 120).unwrap_or_else(|| name.clone()),
        name,
        poster,
        imdb: imdb_id(value.get("imdb_id")),
        season,
        aired,
        rating,
    })
}

/// The series of the feed's pages with a new episode by the TV's `date`, in the feed's order
/// (what is watched most first), without repeats or posterless ones. At most `SHOWN`.
pub fn parse_feed(pages: &[Value], date: Date) -> Vec<Show> {
    let mut shows: Vec<Show> = Vec::new();
    for value in pages
        .iter()
        .filter_map(|page| page.get("results")?.as_array())
        .flatten()
    {
        if shows.len() == SHOWN {
            break;
        }
        if let Some(show) = show(value, date)
            && !shows.iter().any(|seen| seen.id == show.id)
        {
            shows.push(show);
        }
    }
    shows
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: u64,
    /// The TV's local date, which decides what is new.
    pub date: Date,
}

pub enum Poster {
    Loading,
    Ready(Arc<Artwork>),
    Missing,
}

#[derive(Default)]
pub struct Shows {
    pub list: Vec<Show>,
    pub loading: bool,
    pub failed: bool,
    pub posters: HashMap<String, Poster>,
    id: u64,
    pending: Option<u64>,
    next_fetch: i64,
}
impl Shows {
    pub fn generation(&self) -> u64 {
        self.id
    }
    /// Starts a fetch when Home shows the widget and the list is due. Returns the request to
    /// send and whether anything changed.
    pub fn sync(&mut self, enabled: bool, home_visible: bool, now: i64) -> (Option<Command>, bool) {
        let before = (self.loading, self.failed);
        let mut command = None;
        if !enabled || !home_visible {
            if self.pending.take().is_some() {
                self.id += 1;
            }
            self.loading = false;
        } else if self.pending.is_none() && now >= self.next_fetch {
            self.id += 1;
            self.pending = Some(self.id);
            self.loading = self.list.is_empty();
            command = Some(Command {
                id: self.id,
                date: time::OffsetDateTime::from_unix_timestamp(now)
                    .map_or(Date::MIN, |t| t.date()),
            });
        }
        (command, before != (self.loading, self.failed))
    }
    /// Takes the reply to the request in flight; replies to older requests are ignored.
    pub fn accept(&mut self, id: u64, result: Result<Vec<Show>, String>, now: i64) -> bool {
        if self.pending != Some(id) {
            return false;
        }
        self.pending = None;
        self.loading = false;
        match result {
            Ok(list) => {
                self.list = list;
                self.failed = false;
                self.next_fetch = now + REFRESH_SECONDS;
            }
            Err(_) => {
                self.failed = true;
                self.next_fetch = now + RETRY_SECONDS;
            }
        }
        true
    }
    /// Posters in `language` for the cards around `first` not fetched yet, marked as loading.
    pub fn wanted_posters(&mut self, first: usize, language: Language) -> Vec<String> {
        let wanted: Vec<String> = self
            .list
            .iter()
            .skip(first.saturating_sub(VISIBLE))
            .take(VISIBLE * 3)
            .filter_map(|show| show.poster_key(language))
            .filter(|key| !self.posters.contains_key(key))
            .collect();
        if self.posters.len() + wanted.len() > POSTER_CACHE {
            let keep: Vec<String> = self
                .list
                .iter()
                .filter_map(|s| s.poster_key(language))
                .collect();
            self.posters.retain(|key, _| keep.contains(key));
        }
        for path in &wanted {
            self.posters.insert(path.clone(), Poster::Loading);
        }
        wanted
    }
    pub fn accept_poster(&mut self, path: String, image: Option<Arc<Artwork>>) -> bool {
        if !matches!(self.posters.get(&path), Some(Poster::Loading)) {
            return false;
        }
        self.posters
            .insert(path, image.map_or(Poster::Missing, Poster::Ready));
        true
    }
    pub fn poster(&self, show: &Show, language: Language) -> Option<Arc<Artwork>> {
        match self.posters.get(&show.poster_key(language)?) {
            Some(Poster::Ready(image)) => Some(image.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
impl Show {
    /// A series for tests elsewhere, with a new episode on `aired`.
    pub fn sample(id: i64, title: &str, aired: Date) -> Self {
        Self {
            id,
            name: title.into(),
            original: title.into(),
            poster: None,
            imdb: None,
            season: Some(1),
            aired,
            rating: None,
        }
    }
}

#[cfg(test)]
#[path = "shows-tests.rs"]
mod tests;
