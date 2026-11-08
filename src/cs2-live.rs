//! CS2 live: round scores and the open match from the feeds server's `cs2/live/` files, which
//! it builds from HLTV's scorebot. Team 0 and 1 are in Liquipedia's order, as in `cs2.rs`.
use serde_json::Value;
use std::collections::HashMap;

/// Strings from the feed are cut to this many characters, as in `cs2.rs`.
const TEXT: usize = 64;
const PLAYERS: usize = 5;
const HISTORY: usize = 60;
const LOG: usize = 40;
const STREAMS: usize = 16;
const MAPS: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ct,
    T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win {
    Elimination,
    Bomb,
    Defuse,
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundState {
    Started,
    Freeze,
    Ended,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LivePlayer {
    pub nick: String,
    pub alive: bool,
    pub hp: u32,
    pub money: u32,
    pub kills: u32,
    pub hs: u32,
    pub assists: u32,
    pub deaths: u32,
    pub adr: f32,
    /// Percent of the rounds with a kill, assist, survival or trade; HLTV sometimes has none.
    pub kast: Option<f32>,
    /// Opening duels won and lost.
    pub entry: Option<[u32; 2]>,
    pub weapon: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Kill {
        killer: String,
        killer_team: usize,
        assister: Option<String>,
        victim: String,
        victim_team: usize,
        weapon: String,
        headshot: bool,
        smoke: bool,
        wallbang: bool,
        noscope: bool,
        blind: bool,
        air: bool,
    },
    RoundEnd {
        winner: usize,
        score: [u32; 2],
        reason: Win,
    },
    RoundStart,
    BombPlanted {
        player: String,
        team: usize,
        site: Option<char>,
    },
    BombDefused {
        player: String,
        team: usize,
    },
    Suicide {
        player: String,
        team: usize,
        weapon: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Twitch,
    Youtube,
    Kick,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    pub platform: Platform,
    pub channel: String,
    /// The stream's title on HLTV, such as "ESL TV".
    pub name: String,
    pub url: String,
    pub language: Option<String>,
    pub viewers: u32,
}

/// A map of the series as HLTV lists it after the veto.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMap {
    pub name: String,
    /// The team that picked it; None for the decider.
    pub picked: Option<usize>,
    pub score: Option<[u32; 2]>,
    /// None until decided, and for the map being played.
    pub winner: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    pub id: String,
    pub updated: i64,
    pub map: String,
    pub round: u32,
    pub state: RoundState,
    pub score: [u32; 2],
    pub sides: [Side; 2],
    pub bomb_planted: bool,
    pub players: [Vec<LivePlayer>; 2],
    /// Played rounds: number, winning team, how.
    pub history: Vec<(u32, usize, Win)>,
    /// Newest first.
    pub log: Vec<Event>,
    pub streams: Vec<Stream>,
    /// In play order.
    pub maps: Vec<LiveMap>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoundScore {
    pub map: String,
    pub round: u32,
    pub score: [u32; 2],
}

fn text(value: &Value) -> String {
    value
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(TEXT)
        .collect()
}

fn optional(value: &Value) -> Option<String> {
    Some(text(value)).filter(|text| !text.is_empty())
}

fn number(value: &Value) -> u32 {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

fn pair(value: &Value) -> Option<[u32; 2]> {
    match value.as_array()?.as_slice() {
        [a, b] => Some([
            u32::try_from(a.as_u64()?).ok()?,
            u32::try_from(b.as_u64()?).ok()?,
        ]),
        _ => None,
    }
}

fn team(value: &Value) -> Option<usize> {
    match value.as_u64()? {
        0 => Some(0),
        1 => Some(1),
        _ => None,
    }
}

fn win(value: &Value) -> Option<Win> {
    match value.as_str()? {
        "elimination" => Some(Win::Elimination),
        "bomb" => Some(Win::Bomb),
        "defuse" => Some(Win::Defuse),
        "time" => Some(Win::Time),
        _ => None,
    }
}

fn side(value: &Value) -> Option<Side> {
    match value.as_str()? {
        "ct" => Some(Side::Ct),
        "t" => Some(Side::T),
        _ => None,
    }
}

fn player(value: &Value) -> Option<LivePlayer> {
    let nick = text(&value["nick"]);
    if nick.is_empty() {
        return None;
    }
    Some(LivePlayer {
        nick,
        alive: value["alive"].as_bool().unwrap_or(false),
        hp: number(&value["hp"]).min(100),
        money: number(&value["money"]),
        kills: number(&value["kills"]),
        hs: number(&value["hs"]),
        assists: number(&value["assists"]),
        deaths: number(&value["deaths"]),
        adr: value["adr"].as_f64().unwrap_or(0.) as f32,
        kast: value["kast"]
            .as_f64()
            .map(|kast| kast.clamp(0., 100.) as f32),
        entry: pair(&value["entry"]),
        weapon: optional(&value["weapon"]),
    })
}

fn event(value: &Value) -> Option<Event> {
    let flag = |name: &str| value[name].as_bool().unwrap_or(false);
    Some(match value["type"].as_str()? {
        "kill" => Event::Kill {
            killer: optional(&value["killer"])?,
            killer_team: team(&value["killer_team"])?,
            assister: optional(&value["assister"]),
            victim: optional(&value["victim"])?,
            victim_team: team(&value["victim_team"])?,
            weapon: text(&value["weapon"]),
            headshot: flag("headshot"),
            smoke: flag("smoke"),
            wallbang: flag("wallbang"),
            noscope: flag("noscope"),
            blind: flag("blind"),
            air: flag("air"),
        },
        "round_end" => Event::RoundEnd {
            winner: team(&value["winner"])?,
            score: pair(&value["score"])?,
            reason: win(&value["reason"]).unwrap_or(Win::Elimination),
        },
        "round_start" => Event::RoundStart,
        "bomb_planted" => Event::BombPlanted {
            player: optional(&value["player"])?,
            team: team(&value["team"])?,
            site: value["site"]
                .as_str()
                .and_then(|site| site.chars().next())
                .filter(|site| site.is_ascii_alphabetic()),
        },
        "bomb_defused" => Event::BombDefused {
            player: optional(&value["player"])?,
            team: team(&value["team"])?,
        },
        "suicide" => Event::Suicide {
            player: optional(&value["player"])?,
            team: team(&value["team"])?,
            weapon: text(&value["weapon"]),
        },
        _ => return None,
    })
}

fn stream(value: &Value) -> Option<Stream> {
    let platform = match value["platform"].as_str()? {
        "twitch" => Platform::Twitch,
        "youtube" => Platform::Youtube,
        "kick" => Platform::Kick,
        _ => return None,
    };
    let url = value["url"].as_str()?;
    if !url.starts_with("https://") || url.len() > 256 || url.chars().any(char::is_whitespace) {
        return None;
    }
    Some(Stream {
        platform,
        channel: text(&value["channel"]),
        name: text(&value["name"]),
        url: url.to_owned(),
        language: optional(&value["language"]),
        viewers: number(&value["viewers"]),
    })
}

fn list<T>(value: &Value, limit: usize, read: impl Fn(&Value) -> Option<T>) -> Vec<T> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(read)
        .take(limit)
        .collect()
}

pub fn parse_live(value: &Value) -> Result<Live, String> {
    const BAD: &str = "The live match file is unreadable.";
    let sides = match value["sides"].as_array().map(Vec::as_slice) {
        Some([a, b]) => [side(a).ok_or(BAD)?, side(b).ok_or(BAD)?],
        _ => return Err(BAD.into()),
    };
    let state = match value["state"].as_str() {
        Some("freezePeriod") => RoundState::Freeze,
        Some("ended") => RoundState::Ended,
        _ => RoundState::Started,
    };
    let team_players = |index: usize| list(&value["players"][index], PLAYERS, player);
    Ok(Live {
        id: text(&value["id"]),
        updated: value["updated"].as_i64().ok_or(BAD)?,
        map: text(&value["map"]),
        round: number(&value["round"]),
        state,
        score: pair(&value["score"]).ok_or(BAD)?,
        sides,
        bomb_planted: value["bomb_planted"].as_bool().unwrap_or(false),
        players: [team_players(0), team_players(1)],
        history: list(&value["history"], HISTORY, |round| {
            Some((
                number(&round["round"]),
                team(&round["winner"])?,
                win(&round["type"])?,
            ))
        }),
        log: list(&value["log"], LOG, event),
        streams: list(&value["streams"], STREAMS, stream),
        maps: list(&value["maps"], MAPS, |map| {
            let name = text(&map["map"]);
            (!name.is_empty()).then(|| LiveMap {
                name,
                picked: team(&map["picked"]),
                score: pair(&map["score"]),
                winner: team(&map["winner"]),
            })
        }),
    })
}

pub fn parse_index(value: &Value) -> Result<(i64, HashMap<String, RoundScore>), String> {
    const BAD: &str = "The live scores are unreadable.";
    let updated = value["updated"].as_i64().ok_or(BAD)?;
    let matches = value["matches"]
        .as_object()
        .ok_or(BAD)?
        .iter()
        .take(200)
        .filter_map(|(id, item)| {
            let score = pair(&item["score"])?;
            let map = text(&item["map"]);
            Some((
                id.chars().take(TEXT).collect(),
                RoundScore {
                    map,
                    round: number(&item["round"]),
                    score,
                },
            ))
        })
        .collect();
    Ok((updated, matches))
}

/// Round scores for Home are asked for this often while Home shows the widget.
pub const INDEX_SECONDS: i64 = 60;
/// A failed request for them is tried again after this long.
const INDEX_RETRY: i64 = 30;
/// The open match is asked for this often.
pub const DETAIL_SECONDS: i64 = 2;
/// The open match's file counts as live this long after it last changed; the server rewrites
/// it at least every 10 s.
pub const DETAIL_MAX_AGE: i64 = 30;
/// Round scores count this long after the index last changed; it is written every 30 s.
pub const INDEX_MAX_AGE: i64 = 180;
/// The largest live file read.
pub const LIVE_LIMIT: usize = 128 * 1024;
/// A TV clock this far from the server's is wrong; ages are then counted from arrival.
const CLOCK_SKEW: i64 = 3600;

pub enum LiveCommand {
    Index { id: u64 },
    Match { id: u64, match_id: String },
}

pub enum LiveReply {
    Index {
        id: u64,
        result: Result<(i64, HashMap<String, RoundScore>), String>,
    },
    Match {
        id: u64,
        result: Result<Live, String>,
    },
}

/// When data written at `updated` by the server's clock last changed, by the TV's clock.
fn changed_at(updated: i64, now: i64) -> i64 {
    let age = now - updated;
    if age.abs() > CLOCK_SKEW {
        now
    } else {
        now - age.max(0)
    }
}

/// Only names the server writes as files reach a URL.
fn file_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "index"
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// What the TV fetches from `cs2/live/`: the index of round scores while Home shows the
/// widget, and the file of the match whose card is open while it is live.
/// The app that plays a stream on the TV and what to launch it with, found on the TV
/// (2026-11-08). Kick has an app, but it does not play streams properly, so it gets nothing.
pub fn launch_params(stream: &Stream) -> Option<(&'static str, Value)> {
    let rest = stream.url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    match stream.platform {
        Platform::Twitch => {
            if !matches!(host, "twitch.tv" | "www.twitch.tv" | "m.twitch.tv") {
                return None;
            }
            let login = path
                .split(['?', '#'])
                .next()
                .unwrap_or("")
                .trim_end_matches('/');
            let valid = (1..=25).contains(&login.len())
                && login
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_');
            valid.then(|| {
                (
                    "twitch.adamffdev.v1",
                    serde_json::json!({"channel": login.to_ascii_lowercase()}),
                )
            })
        }
        Platform::Youtube => {
            let (path, query) = path.split_once('?').unwrap_or((path, ""));
            let id = match host {
                "youtu.be" => Some(path),
                "youtube.com" | "www.youtube.com" | "m.youtube.com" => match path {
                    "watch" => query.split('&').find_map(|pair| pair.strip_prefix("v=")),
                    _ => path
                        .strip_prefix("live/")
                        .or_else(|| path.strip_prefix("embed/")),
                },
                _ => None,
            }?;
            let valid = id.len() == 11
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
            valid.then(|| {
                (
                    "youtube.leanback.v4",
                    serde_json::json!({"contentTarget": format!("https://www.youtube.com/tv?v={id}")}),
                )
            })
        }
        Platform::Kick => None,
    }
}

#[derive(Default)]
pub struct LiveScores {
    id: u64,
    index: HashMap<String, RoundScore>,
    index_updated: i64,
    index_changed: Option<i64>,
    index_pending: Option<u64>,
    next_index: i64,
    open: Option<String>,
    detail: Option<Live>,
    detail_changed: i64,
    detail_pending: Option<u64>,
    next_detail: i64,
}
impl LiveScores {
    /// The requests now due.
    pub fn sync(
        &mut self,
        enabled: bool,
        home_visible: bool,
        open: Option<&str>,
        open_is_live: bool,
        now: i64,
    ) -> Vec<LiveCommand> {
        let mut commands = Vec::new();
        if !enabled || !home_visible {
            self.index_pending = None;
        } else if self.index_pending.is_none() && now >= self.next_index {
            self.id += 1;
            self.index_pending = Some(self.id);
            commands.push(LiveCommand::Index { id: self.id });
        }
        let open = open.filter(|id| enabled && open_is_live && file_name(id));
        if open != self.open.as_deref() {
            self.open = open.map(str::to_owned);
            self.detail = None;
            self.detail_pending = None;
            self.next_detail = 0;
        }
        if let Some(match_id) = &self.open
            && self.detail_pending.is_none()
            && now >= self.next_detail
        {
            self.id += 1;
            self.detail_pending = Some(self.id);
            commands.push(LiveCommand::Match {
                id: self.id,
                match_id: match_id.clone(),
            });
        }
        commands
    }
    /// Takes the reply to a request in flight; others are ignored. Returns whether it was taken.
    pub fn accept(&mut self, reply: LiveReply, now: i64) -> bool {
        match reply {
            LiveReply::Index { id, result } => {
                if self.index_pending != Some(id) {
                    return false;
                }
                self.index_pending = None;
                match result {
                    Ok((updated, matches)) => {
                        if self.index_changed.is_none() || updated > self.index_updated {
                            self.index_changed = Some(changed_at(updated, now));
                        }
                        self.index_updated = updated;
                        self.index = matches;
                        self.next_index = now + INDEX_SECONDS;
                    }
                    Err(_) => self.next_index = now + INDEX_RETRY,
                }
            }
            LiveReply::Match { id, result } => {
                if self.detail_pending != Some(id) {
                    return false;
                }
                self.detail_pending = None;
                self.next_detail = now + DETAIL_SECONDS;
                if let Ok(live) = result {
                    let moved = self
                        .detail
                        .as_ref()
                        .is_none_or(|old| live.updated > old.updated);
                    if moved {
                        self.detail_changed = changed_at(live.updated, now);
                    }
                    self.detail = Some(live);
                }
            }
        }
        true
    }
    /// The round score of a match being played, while the index is fresh.
    pub fn round(&self, id: &str, now: i64) -> Option<&RoundScore> {
        let changed = self.index_changed?;
        (now - changed <= INDEX_MAX_AGE).then(|| self.index.get(id))?
    }
    /// The open match's live file, while it is fresh.
    pub fn detail(&self, id: &str, now: i64) -> Option<&Live> {
        let live = self.detail.as_ref()?;
        (self.open.as_deref() == Some(id)
            && live.id == id
            && now - self.detail_changed <= DETAIL_MAX_AGE)
            .then_some(live)
    }
    /// The open match's maps as last read, however old.
    pub fn maps(&self, id: &str) -> &[LiveMap] {
        match &self.detail {
            Some(live) if self.open.as_deref() == Some(id) && live.id == id => &live.maps,
            _ => &[],
        }
    }
    /// The open match's streams as last read, however old: they change rarely.
    pub fn streams(&self, id: &str) -> &[Stream] {
        match &self.detail {
            Some(live) if self.open.as_deref() == Some(id) && live.id == id => &live.streams,
            _ => &[],
        }
    }
}

#[cfg(test)]
#[path = "cs2-live-tests.rs"]
mod tests;
