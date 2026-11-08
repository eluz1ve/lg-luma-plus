//! The TV Shows worker: CUB's feed of series off the SDL thread, newest request wins, and posters
//! one at a time on their own worker, from TVmaze in English or from TMDB as CUB has them.
use crate::app_artwork::Artwork;
use crate::shows::{
    Command, PAGES, Show, parse_feed, poster_source, poster_urls, tvmaze_image, tvmaze_lookup,
    tvmaze_show,
};
use serde_json::Value;
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

/// A page of twenty series is about fifty kilobytes.
const LIMIT: usize = 1024 * 1024;
const POSTER_LIMIT: usize = 1024 * 1024;
/// TVmaze's series are a few kilobytes; its full-size posters, about 1400×2100, under a megabyte.
const TVMAZE_LIMIT: usize = 256 * 1024;
const TVMAZE_POSTER_LIMIT: usize = 4 * 1024 * 1024;
/// Posters are drawn about 256 px wide; a little more keeps them sharp when the card lifts.
const POSTER_WIDTH: u32 = 320;

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
}
impl Client {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(8)))
            .max_redirects(0)
            .proxy(None)
            .build()
            .new_agent();
        Self { agent }
    }
    fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String> {
        let mut response = self
            .agent
            .get(url)
            .header("User-Agent", "LumaNative")
            .call()
            .map_err(|_| "CUB is unavailable.")?;
        if !response.status().is_success() {
            return Err("CUB is unavailable.".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The CUB reply could not be read")?;
        if bytes.len() > limit {
            return Err("The CUB reply is too large".into());
        }
        Ok(bytes)
    }
    /// The feed's first pages, from the first of CUB's hosts that answers.
    pub fn shows(&self, command: &Command) -> Result<Vec<Show>, String> {
        let mut error = None;
        for host in crate::cub::HOSTS {
            let pages: Result<Vec<Value>, String> = (1..=PAGES)
                .map(|page| {
                    let bytes = self.get(&feed_url(host, page), LIMIT)?;
                    serde_json::from_slice(&bytes)
                        .map_err(|_| "CUB returned unreadable data".into())
                })
                .collect();
            match pages {
                Ok(pages) => return Ok(parse_feed(&pages, command.date)),
                Err(message) => error = Some(message),
            }
        }
        Err(error.unwrap_or_else(|| "CUB is unavailable.".into()))
    }
    /// A poster for its key (see `shows::poster_key`), decoded and scaled on this worker.
    pub fn poster(&self, key: &str) -> Option<Arc<Artwork>> {
        poster(&self.agent, key, "show-poster")
    }
}

fn download(
    agent: &ureq::Agent,
    url: &str,
    limit: usize,
) -> Option<(u16, Vec<u8>, Option<String>)> {
    let mut response = agent
        .get(url)
        .header("User-Agent", "LumaNative")
        .call()
        .ok()?;
    let status = response.status().as_u16();
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= limit).then_some((status, bytes, location))
}

fn ok(reply: Option<(u16, Vec<u8>, Option<String>)>) -> Option<Vec<u8>> {
    reply
        .filter(|(status, ..)| (200..300).contains(status))
        .map(|(_, bytes, _)| bytes)
}

/// TVmaze's English poster of the series with this IMDb id: the lookup redirects to the series,
/// which names its poster.
fn tvmaze_poster(agent: &ureq::Agent, imdb: &str) -> Option<Vec<u8>> {
    let (status, _, location) = download(agent, &tvmaze_lookup(imdb), TVMAZE_LIMIT)?;
    let show = location.filter(|url| (300..400).contains(&status) && tvmaze_show(url))?;
    let show: Value = serde_json::from_slice(&ok(download(agent, &show, TVMAZE_LIMIT))?).ok()?;
    ok(download(agent, &tvmaze_image(&show)?, TVMAZE_POSTER_LIMIT))
}

/// A poster for `key`: TVmaze's where the key asks for it, else, or failing that, TMDB's (or its
/// mirror's), scaled to the cards on the calling worker. Shared with the CUB account's posters.
pub fn poster(agent: &ureq::Agent, key: &str, kind: &str) -> Option<Arc<Artwork>> {
    let (imdb, path) = poster_source(key);
    let bytes = imdb
        .and_then(|imdb| tvmaze_poster(agent, imdb))
        .or_else(|| {
            poster_urls(path?)
                .iter()
                .find_map(|url| ok(download(agent, url, POSTER_LIMIT)))
        })?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let pixels = if image.width() > POSTER_WIDTH {
        image.thumbnail(POSTER_WIDTH, POSTER_WIDTH * 2).into_rgba8()
    } else {
        image.into_rgba8()
    };
    Some(Arc::new(Artwork {
        key: format!("{kind}:{key}"),
        pixels,
        backdrop: [[0; 3]; 2],
        wash: None,
    }))
}

/// A page of the series Lampa's viewers watch now, as Lampa itself asks for it.
fn feed_url(host: &str, page: u32) -> String {
    format!("https://tmdb.{host}/?cat=tv&sort=now_playing&page={page}&email=")
}

pub enum Response {
    Shows {
        id: u64,
        result: Result<Vec<Show>, String>,
    },
    Poster {
        path: String,
        image: Option<Arc<Artwork>>,
    },
}

pub struct Service {
    sender: Option<mpsc::Sender<Command>>,
    posters: Option<mpsc::Sender<String>>,
    generation: Arc<AtomicU64>,
    pub receiver: mpsc::Receiver<Response>,
}
impl Service {
    pub fn new(client: Client) -> Self {
        let (sender, jobs) = mpsc::channel::<Command>();
        let (responses, receiver) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        let (posters, wanted) = mpsc::channel::<String>();
        let poster_client = client.clone();
        let poster_responses = responses.clone();
        thread::spawn(move || {
            for path in wanted {
                let image = poster_client.poster(&path);
                if poster_responses
                    .send(Response::Poster { path, image })
                    .is_err()
                {
                    break;
                }
            }
        });
        thread::spawn(move || {
            while let Ok(mut job) = jobs.recv() {
                // Only the newest waiting request is worth answering.
                while let Ok(newer) = jobs.try_recv() {
                    job = newer;
                }
                if current.load(Ordering::SeqCst) != job.id {
                    continue;
                }
                let result = client.shows(&job);
                if current.load(Ordering::SeqCst) == job.id
                    && responses
                        .send(Response::Shows { id: job.id, result })
                        .is_err()
                {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            posters: Some(posters),
            generation,
            receiver,
        }
    }
    /// Requests older than `id` are dropped before or after they run.
    pub fn generation(&self, id: u64) {
        self.generation.store(id, Ordering::SeqCst);
    }
    pub fn poster(&self, path: String) {
        if let Some(posters) = &self.posters {
            let _ = posters.send(path);
        }
    }
    pub fn send(&self, command: Command) {
        self.generation(command.id);
        if let Some(sender) = &self.sender {
            let _ = sender.send(command);
        }
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.generation.store(u64::MAX, Ordering::SeqCst);
        self.sender.take();
        self.posters.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feed_is_asked_for_as_lampa_asks_for_it() {
        assert_eq!(
            feed_url("cub.red", 2),
            "https://tmdb.cub.red/?cat=tv&sort=now_playing&page=2&email="
        );
    }
}
