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

#[cfg(test)]
#[path = "cs2-live-tests.rs"]
mod tests;
