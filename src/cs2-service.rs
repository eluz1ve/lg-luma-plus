//! The CS2 Scores worker: one HTTP job at a time off the SDL thread, newest request wins.
use crate::app_artwork::Artwork;
use crate::cs2::{Command, Feed, image_allowed, parse_feed};
use crate::cs2_live::{
    LIVE_LIMIT, Live, LiveCommand, LiveReply, RoundScore, parse_index, parse_live,
};
use serde_json::Value;
use std::collections::HashMap;
use std::{
    io::Read,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const LIMIT: usize = 256 * 1024;
const LOGO_LIMIT: usize = 1024 * 1024;
/// Map screenshots stand behind text, so they keep more width than a logo.
const BACKDROP_WIDTH: u32 = 640;
/// The feed from its own server.
const FEEDS: [&str; 1] = ["https://luma.eluzive.dev/cs2/matches.json"];
/// The live match files, from the same server only.
const LIVE: &str = "https://luma.eluzive.dev/cs2/live/";

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    urls: Vec<String>,
    /// The folder of live match files, ending in "/".
    live: String,
}
impl Client {
    pub fn new(test_server: Option<&str>) -> Result<Self, String> {
        let urls = if let Some(server) = test_server {
            let uri: ureq::http::Uri = server
                .parse()
                .map_err(|e| format!("Invalid CS2 test server: {e}"))?;
            let address: SocketAddr = uri
                .authority()
                .ok_or("CS2 test server needs a loopback address and port")?
                .as_str()
                .parse()
                .map_err(|_| "CS2 tests require an IP-literal loopback address and port")?;
            if uri.scheme_str() != Some("http")
                || !address.ip().is_loopback()
                || address.port() == 0
                || uri.path() != "/"
                || uri.query().is_some()
            {
                return Err("CS2 test server must be an HTTP loopback origin".into());
            }
            vec![format!("http://{address}/luma-feeds/cs2/matches.json")]
        } else {
            FEEDS.map(String::from).to_vec()
        };
        let live = match &urls[..] {
            [url] if test_server.is_some() => url.replace("matches.json", "live/"),
            _ => LIVE.to_owned(),
        };
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(8)))
            .max_redirects(0)
            .proxy(None)
            .build()
            .new_agent();
        Ok(Self { agent, urls, live })
    }
    /// A logo or a map's screenshot from an allowed origin, decoded and scaled on this
    /// worker.
    pub fn image(&self, url: &str, backdrop: bool) -> Option<Arc<Artwork>> {
        if !image_allowed(url) {
            return None;
        }
        let mut response = self
            .agent
            .get(url)
            .header("User-Agent", "LumaNative")
            .call()
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(LOGO_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > LOGO_LIMIT {
            return None;
        }
        let mut artwork = if backdrop {
            decode_backdrop(&bytes)?
        } else {
            crate::app_artwork::decode(&bytes)?
        };
        artwork.key = format!("cs2-image:{url}");
        if !backdrop {
            artwork.wash = crate::cs2_wash::paint(&artwork.pixels).map(|pixels| {
                Arc::new(Artwork {
                    key: format!("cs2-wash:{url}"),
                    pixels,
                    backdrop: [[0; 3]; 2],
                    wash: None,
                })
            });
        }
        Some(Arc::new(artwork))
    }
    /// Round scores of the matches being played.
    pub fn live_index(&self) -> Result<(i64, HashMap<String, RoundScore>), String> {
        parse_index(&self.json(&format!("{}index.json", self.live))?)
    }
    /// The live file of one match; `id` is a file name the scheduler checked.
    pub fn live_match(&self, id: &str) -> Result<Live, String> {
        parse_live(&self.json(&format!("{}{id}.json", self.live))?)
    }
    fn json(&self, url: &str) -> Result<Value, String> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .header("User-Agent", "LumaNative")
            .call()
            .map_err(|_| "Live scores are unavailable.")?;
        if !response.status().is_success() {
            return Err("Live scores are unavailable.".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(LIVE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The live scores could not be read")?;
        if bytes.len() > LIVE_LIMIT {
            return Err("The live scores are too large".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "The live scores are unreadable".into())
    }
    pub fn feed(&self) -> Result<Feed, String> {
        let mut failure = String::new();
        for url in &self.urls {
            match self.fetch(url) {
                Ok(feed) => return Ok(feed),
                Err(error) => failure = error,
            }
        }
        Err(failure)
    }
    fn fetch(&self, url: &str) -> Result<Feed, String> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .header("User-Agent", "LumaNative")
            .call()
            .map_err(|_| "Scores are unavailable.")?;
        if !response.status().is_success() {
            return Err("Scores are unavailable.".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The score feed could not be read")?;
        if bytes.len() > LIMIT {
            return Err("The score feed is too large".into());
        }
        parse_feed(
            &serde_json::from_slice(&bytes)
                .map_err(|_| "The score feed returned unreadable data")?,
        )
    }
}

/// A screenshot within the same limits as app artwork, scaled to `BACKDROP_WIDTH`.
fn decode_backdrop(bytes: &[u8]) -> Option<Artwork> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let pixels = if image.width() > BACKDROP_WIDTH {
        image.thumbnail(BACKDROP_WIDTH, BACKDROP_WIDTH).into_rgba8()
    } else {
        image.into_rgba8()
    };
    Some(Artwork {
        key: String::new(),
        pixels,
        backdrop: [[0; 3]; 2],
        wash: None,
    })
}

pub enum Response {
    Feed {
        id: u64,
        result: Result<Feed, String>,
    },
    Logo {
        url: String,
        image: Option<Arc<Artwork>>,
    },
    Live(LiveReply),
}

pub struct Service {
    sender: Option<mpsc::Sender<Command>>,
    logos: Option<mpsc::Sender<(String, bool)>>,
    live: Option<mpsc::Sender<LiveCommand>>,
    generation: Arc<AtomicU64>,
    pub receiver: mpsc::Receiver<Response>,
}
impl Service {
    pub fn new(client: Client) -> Self {
        let (sender, jobs) = mpsc::channel::<Command>();
        let (responses, receiver) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        // Logos come one at a time on their own worker, so a slow one never delays scores.
        let (logos, wanted) = mpsc::channel::<(String, bool)>();
        let logo_client = client.clone();
        let logo_responses = responses.clone();
        thread::spawn(move || {
            for (url, backdrop) in wanted {
                let image = logo_client.image(&url, backdrop);
                if logo_responses.send(Response::Logo { url, image }).is_err() {
                    break;
                }
            }
        });
        // Live files on a worker of their own: the open match every 2 s, the index for Home;
        // of each kind only the newest waiting request runs.
        let (live, live_jobs) = mpsc::channel::<LiveCommand>();
        let live_client = client.clone();
        let live_responses = responses.clone();
        thread::spawn(move || {
            while let Ok(first) = live_jobs.recv() {
                let (mut index, mut detail) = (None, None);
                for job in std::iter::once(first).chain(live_jobs.try_iter()) {
                    match job {
                        LiveCommand::Index { id } => index = Some(id),
                        LiveCommand::Match { id, match_id } => detail = Some((id, match_id)),
                    }
                }
                let mut replies = Vec::new();
                if let Some((id, match_id)) = detail {
                    replies.push(LiveReply::Match {
                        id,
                        result: live_client.live_match(&match_id),
                    });
                }
                if let Some(id) = index {
                    replies.push(LiveReply::Index {
                        id,
                        result: live_client.live_index(),
                    });
                }
                if replies
                    .into_iter()
                    .any(|reply| live_responses.send(Response::Live(reply)).is_err())
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
                let result = client.feed();
                if current.load(Ordering::SeqCst) == job.id
                    && responses
                        .send(Response::Feed { id: job.id, result })
                        .is_err()
                {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            logos: Some(logos),
            live: Some(live),
            generation,
            receiver,
        }
    }
    /// Requests older than `id` are dropped before or after they run.
    pub fn generation(&self, id: u64) {
        self.generation.store(id, Ordering::SeqCst);
    }
    pub fn image(&self, url: String, backdrop: bool) {
        if let Some(logos) = &self.logos {
            let _ = logos.send((url, backdrop));
        }
    }
    pub fn live(&self, command: LiveCommand) {
        if let Some(live) = &self.live {
            let _ = live.send(command);
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
        self.logos.take();
        self.live.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    const GOOD: &str = "HTTP/1.1 200 OK\r\nContent-Length: 26\r\nConnection: close\r\n\r\n\
                        {\"updated\":7,\"matches\":[]}";
    const REFUSED: &str =
        "HTTP/1.1 525 SSL handshake failed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const UNREADABLE: &str =
        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope";
    const NOBODY: &str = "http://127.0.0.1:1/matches.json";

    /// A loopback server that answers one request with `response`.
    fn serve(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://{address}/matches.json")
    }

    fn client(urls: Vec<String>) -> Client {
        let mut client = Client::new(Some("http://127.0.0.1:1")).unwrap();
        client.urls = urls;
        client
    }

    #[test]
    fn the_first_feed_is_used_when_it_answers() {
        let feed = client(vec![serve(GOOD), NOBODY.into()]).feed().unwrap();
        assert_eq!(feed.updated, 7);
    }

    #[test]
    fn a_feed_the_server_refuses_falls_back_to_the_next() {
        let feed = client(vec![serve(REFUSED), serve(GOOD)]).feed().unwrap();
        assert_eq!(feed.updated, 7);
    }

    #[test]
    fn an_unreachable_feed_falls_back_to_the_next() {
        let feed = client(vec![NOBODY.into(), serve(GOOD)]).feed().unwrap();
        assert_eq!(feed.updated, 7);
    }

    #[test]
    fn an_unreadable_feed_falls_back_to_the_next() {
        let feed = client(vec![serve(UNREADABLE), serve(GOOD)]).feed().unwrap();
        assert_eq!(feed.updated, 7);
    }

    #[test]
    fn every_feed_failing_is_an_error() {
        assert!(client(vec![serve(REFUSED), NOBODY.into()]).feed().is_err());
    }

    const INDEX: &str = "HTTP/1.1 200 OK\r\nContent-Length: 69\r\nConnection: close\r\n\r\n\
                         {\"updated\":9,\"matches\":{\"m1\":{\"map\":\"Nuke\",\"round\":4,\"score\":[1,2]}}}";
    const MATCH: &str = "HTTP/1.1 200 OK\r\nContent-Length: 66\r\nConnection: close\r\n\r\n\
                         {\"id\":\"m1\",\"updated\":9,\"score\":[1,2],\"sides\":[\"ct\",\"t\"],\"round\":4}";

    /// A client whose live files come from `url`'s server.
    fn live_client(url: String) -> Client {
        let mut client = client(Vec::new());
        client.live = url.trim_end_matches("matches.json").to_owned();
        client
    }

    #[test]
    fn cs2_live_files_come_from_the_live_folder() {
        let (updated, matches) = live_client(serve(INDEX)).live_index().unwrap();
        assert_eq!((updated, matches["m1"].round), (9, 4));
        let live = live_client(serve(MATCH)).live_match("m1").unwrap();
        assert_eq!((live.id.as_str(), live.score), ("m1", [1, 2]));
        assert!(live_client(serve(REFUSED)).live_match("m1").is_err());
        assert!(live_client(serve(UNREADABLE)).live_index().is_err());
        assert_eq!(
            Client::new(Some("http://127.0.0.1:1234")).unwrap().live,
            "http://127.0.0.1:1234/luma-feeds/cs2/live/"
        );
        assert_eq!(
            Client::new(None).unwrap().live,
            "https://luma.eluzive.dev/cs2/live/"
        );
    }

    #[test]
    fn mock_origins_are_loopback_only() {
        assert!(Client::new(Some("http://127.0.0.1:1234")).is_ok());
        assert!(Client::new(Some("http://[::1]:1234")).is_ok());
        for invalid in [
            "http://example.com:1234",
            "https://127.0.0.1:1234",
            "http://127.0.0.1:0",
            "http://127.0.0.1:1234/path",
            "http://127.0.0.1:1234?query=yes",
        ] {
            assert!(Client::new(Some(invalid)).is_err(), "{invalid}");
        }
    }
}
