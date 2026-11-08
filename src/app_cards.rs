//! The cards the TV's apps publish for LG's Home (YouTube, Netflix, Prime Video, ...), shown in
//! the Lampa row after Lampa's own. The application manager's `getAllContents` answers with
//! every app's list: `{ allContents: [{ launchPointId, contents: [{ type: "content",
//! titleText, category, image: { url }, onClick: { targetAppId, contentId, launchParam } }] }] }`.
//! A card opens its app at the title the way LG's Home does: with `launchParam` when the card
//! has one, else `contentTarget` set to `contentId`.
use crate::app_artwork::Artwork;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

pub const URI: &str = "luna://com.webos.service.applicationmanager/getAllContents";
/// Cards kept from one app, so one app's twelve suggestions do not fill the row.
pub const PER_APP: usize = 6;
const MAX_TEXT: usize = 200;
const MAX_ID: usize = 512;
/// How long a read stays fresh while Home is shown.
const FRESH: i64 = 5 * 60;

#[derive(Clone, Debug, PartialEq)]
pub struct AppCard {
    pub app: String,
    pub title: String,
    pub category: String,
    pub image: Option<String>,
    pub content_id: String,
    pub launch_param: Option<Value>,
}

impl AppCard {
    /// What `applicationManager/launch` gets as `params` to open this title.
    pub fn params(&self) -> Value {
        match &self.launch_param {
            Some(param @ Value::Object(_)) => param.clone(),
            _ => json!({ "contentTarget": self.content_id }),
        }
    }
}

fn clean(value: Option<&Value>) -> String {
    let text: String = value
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TEXT)
        .collect()
}

/// Pictures come from the apps' own servers; only plain web addresses are fetched.
pub fn image_allowed(url: &str) -> bool {
    url.len() <= 2048
        && (url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://")))
        .is_some_and(|rest| {
            !rest.is_empty()
                && !rest.starts_with('/')
                && !rest.chars().any(|c| c.is_whitespace() || c.is_control())
        })
}

/// A card's picture: the larger `image_4k`, as the poster crops it, else `image.url`; a protocol-relative address
/// taken as https.
fn picture(card: &Value) -> Option<String> {
    ["/image_4k/url", "/image/url", "/image/uri", "/imageUrl"]
        .iter()
        .filter_map(|pointer| card.pointer(pointer).and_then(Value::as_str))
        .map(|url| match url.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => url.trim().to_owned(),
        })
        .find(|url| image_allowed(url))
}

/// Every app's cards in the order each app gave them, at most `PER_APP` an app; Luma's own
/// and LG Home's entries are left out.
pub fn parse(reply: &Value) -> Result<Vec<AppCard>, String> {
    if reply.get("returnValue") == Some(&Value::Bool(false)) {
        return Err(clean(reply.get("errorText")));
    }
    let lists = reply
        .get("allContents")
        .and_then(Value::as_array)
        .ok_or("Invalid app contents reply")?;
    let mut out: Vec<AppCard> = Vec::new();
    let mut count: HashMap<String, usize> = HashMap::new();
    for list in lists.iter().take(64) {
        let Some(contents) = list.get("contents").and_then(Value::as_array) else {
            continue;
        };
        for card in contents.iter().take(64) {
            if card.get("type").and_then(Value::as_str) != Some("content") {
                continue;
            }
            let Some(click) = card.get("onClick") else {
                continue;
            };
            let app = click
                .get("targetAppId")
                .and_then(Value::as_str)
                .unwrap_or("");
            let content_id = click.get("contentId").and_then(Value::as_str).unwrap_or("");
            let title = clean(card.get("titleText"));
            if !crate::model::valid_app_id(app)
                || app == crate::platform::APP_ID
                || app.starts_with("com.webos.app.home")
                || content_id.is_empty()
                || content_id.len() > MAX_ID
                || title.is_empty()
            {
                continue;
            }
            let seen = count.entry(app.to_owned()).or_default();
            if *seen >= PER_APP
                || out
                    .iter()
                    .any(|other| other.app == app && other.content_id == content_id)
            {
                continue;
            }
            *seen += 1;
            let image = picture(card);
            let launch_param = click
                .get("launchParam")
                .filter(|param| param.as_object().is_some_and(|map| !map.is_empty()))
                .filter(|param| param.to_string().len() <= 4096)
                .cloned();
            out.push(AppCard {
                app: app.to_owned(),
                title,
                category: clean(card.get("category")),
                image,
                content_id: content_id.to_owned(),
                launch_param,
            });
        }
    }
    Ok(out)
}

/// A few cards for the desktop preview, which never calls the TV.
pub fn demo() -> Vec<AppCard> {
    [
        ("jfKfPfyJRdk", "Lo-fi beats to relax to"),
        ("LXb3EKWsInQ", "Costa Rica in 4K"),
        ("aqz-KE-bpKQ", "Big Buck Bunny"),
    ]
    .into_iter()
    .map(|(id, title)| AppCard {
        app: "youtube.leanback.v4".into(),
        title: title.into(),
        category: "Recommended".into(),
        image: Some(format!("https://i.ytimg.com/vi/{id}/maxresdefault.jpg")),
        content_id: id.into(),
        launch_param: None,
    })
    .collect()
}

/// The apps' cards and their pictures, read while Home is on screen.
#[derive(Default)]
pub struct AppCards {
    pub list: Vec<AppCard>,
    /// When the last read was asked for, answered or not.
    asked: Option<i64>,
    pending: bool,
    images: HashMap<String, Option<Arc<Artwork>>>,
}

impl AppCards {
    /// Whether to read the cards now: on Home, when none were read for a while.
    pub fn due(&mut self, wanted: bool, now: i64) -> bool {
        if !wanted || self.pending || self.asked.is_some_and(|at| now - at < FRESH) {
            return false;
        }
        self.asked = Some(now);
        self.pending = true;
        true
    }

    /// The read's answer; whether anything shown changed. A failure keeps the cards shown.
    pub fn accept(&mut self, result: Result<Vec<AppCard>, String>) -> bool {
        self.pending = false;
        match result {
            Ok(list) if list != self.list => {
                self.images
                    .retain(|url, _| list.iter().any(|card| card.image.as_deref() == Some(url)));
                self.list = list;
                true
            }
            Ok(_) => false,
            Err(error) => {
                eprintln!("Luma app cards: {error}");
                false
            }
        }
    }

    /// The apps that published cards, in the order they first appear.
    pub fn apps(&self) -> Vec<String> {
        let mut apps: Vec<String> = Vec::new();
        for card in &self.list {
            if !apps.contains(&card.app) {
                apps.push(card.app.clone());
            }
        }
        apps
    }

    /// The cards of the apps not turned off, taking one from each app in turn.
    pub fn shown(&self, off: &[String]) -> Vec<&AppCard> {
        let apps: Vec<String> = self
            .apps()
            .into_iter()
            .filter(|app| !off.contains(app))
            .collect();
        let lists: Vec<Vec<&AppCard>> = apps
            .iter()
            .map(|app| self.list.iter().filter(|card| &card.app == app).collect())
            .collect();
        let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
        (0..longest)
            .flat_map(|round| {
                lists
                    .iter()
                    .filter_map(move |list| list.get(round).copied())
            })
            .collect()
    }

    /// Pictures of `cards` not yet asked for, marked as loading.
    pub fn wanted_images(&mut self, cards: &[&AppCard]) -> Vec<String> {
        let mut wanted = Vec::new();
        for url in cards.iter().filter_map(|card| card.image.as_ref()) {
            if !self.images.contains_key(url) {
                self.images.insert(url.clone(), None);
                wanted.push(url.clone());
            }
        }
        wanted
    }

    /// A picture's answer; whether it was asked for.
    pub fn accept_image(&mut self, url: String, image: Option<Arc<Artwork>>) -> bool {
        match self.images.get_mut(&url) {
            Some(slot) => {
                *slot = image;
                true
            }
            None => false,
        }
    }

    pub fn image(&self, url: &str) -> Option<Arc<Artwork>> {
        self.images.get(url)?.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(app: &str, id: &str, title: &str) -> Value {
        json!({"type": "content", "titleText": title, "category": "Top picks",
            "image": {"url": format!("https://img.example/{id}.jpg")},
            "onClick": {"targetAppId": app, "contentId": id}})
    }

    #[test]
    fn parses_cards_and_skips_headers_and_bad_entries() {
        let reply = json!({"returnValue": true, "allContents": [
            {"launchPointId": "com.webos.app.browser_default", "contents": []},
            {"launchPointId": "netflix_default", "contents": [
                {"type": "title", "titleText": "Netflix"},
                card("netflix", "80100172", "  Dark \n Season 1 "),
                card("netflix", "80100172", "Dark again"),
                card("bad id!", "1", "Nope"),
                card("netflix", "", "No id"),
                {"type": "content", "titleText": "Plain http", "image": {"url": "http://x/y.jpg"},
                    "onClick": {"targetAppId": "netflix", "contentId": "7",
                        "launchParam": {"contentTarget": "7", "source": "home"}}},
            ]},
        ]});
        let cards = parse(&reply).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].title, "Dark Season 1");
        assert_eq!(
            cards[0].image.as_deref(),
            Some("https://img.example/80100172.jpg")
        );
        assert_eq!(cards[0].params(), json!({"contentTarget": "80100172"}));
        assert_eq!(cards[1].image.as_deref(), Some("http://x/y.jpg"));
        assert_eq!(
            cards[1].params(),
            json!({"contentTarget": "7", "source": "home"})
        );
    }

    #[test]
    fn keeps_at_most_six_cards_an_app() {
        let contents: Vec<Value> = (0..12)
            .map(|i| card("youtube.leanback.v4", &i.to_string(), "Video"))
            .collect();
        let reply = json!({"allContents": [{"launchPointId": "youtube.leanback.v4_default", "contents": contents}]});
        assert_eq!(parse(&reply).unwrap().len(), PER_APP);
    }

    #[test]
    fn a_refused_call_is_an_error() {
        assert!(parse(&json!({"returnValue": false, "errorText": "Denied"})).is_err());
        assert!(parse(&json!({})).is_err());
    }

    #[test]
    fn shown_takes_apps_in_turn_and_leaves_out_turned_off_ones() {
        let reply = json!({"allContents": [
            {"contents": [card("youtube.leanback.v4", "a", "A"), card("youtube.leanback.v4", "b", "B")]},
            {"contents": [card("netflix", "c", "C"), card("netflix", "d", "D")]},
        ]});
        let mut cards = AppCards::default();
        assert!(cards.due(true, 0));
        assert!(!cards.due(true, 10));
        assert!(cards.accept(parse(&reply)));
        let titles = |off: &[String]| {
            cards
                .shown(off)
                .iter()
                .map(|c| c.title.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(titles(&[]), ["A", "C", "B", "D"]);
        assert_eq!(titles(&["netflix".into()]), ["A", "B"]);
        assert_eq!(cards.apps(), ["youtube.leanback.v4", "netflix"]);
        assert!(!cards.due(true, 100));
        assert!(cards.due(true, 400));
    }

    #[test]
    fn only_plain_https_pictures_are_fetched() {
        assert!(image_allowed("https://i.ytimg.com/vi/x/hq.jpg"));
        assert!(image_allowed("http://i.ytimg.com/vi/x/hq.jpg"));
        assert!(!image_allowed("ftp://i.ytimg.com/x.jpg"));
        assert!(!image_allowed("https:///etc"));
        assert!(!image_allowed("https://a b"));
    }
}
