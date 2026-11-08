use super::*;
use serde_json::json;

fn player(nick: &str) -> Value {
    json!({"nick": nick, "alive": true, "hp": 100, "money": 2200, "kills": 20, "hs": 9,
           "assists": 2, "deaths": 16, "adr": 72.1, "kast": 69.6, "entry": [1, 2], "weapon": "awp"})
}

/// The shape cs2-live on the feeds server writes (a file taken from it, trimmed).
fn live() -> Value {
    json!({
        "id": "tMoSBihfDY_R01-M001", "hltv": 2400319, "updated": 1_794_171_823,
        "map": "Anubis", "round": 23, "state": "started", "score": [10, 12], "sides": ["t", "ct"],
        "frozen": false, "bomb_planted": false,
        "players": [
            [player("Chucky---"), player("a"), player("b"), player("c"), player("d"), player("extra")],
            [{"nick": "Br4tkO", "alive": false, "hp": 0, "money": 0, "kills": 3, "hs": 1,
              "assists": 0, "deaths": 5, "adr": 40.0, "kast": null, "entry": null, "weapon": null}]
        ],
        "history": [{"round": 1, "winner": 1, "type": "defuse"},
                    {"round": 2, "winner": 0, "type": "elimination"},
                    {"round": 3, "winner": 2, "type": "bomb"}],
        "log": [
            {"type": "kill", "killer": "Br4tkO", "killer_team": 1, "assister": null,
             "victim": "nut nut", "victim_team": 0, "weapon": "fiveseven", "headshot": true,
             "smoke": true, "wallbang": false, "noscope": false, "blind": false, "air": true},
            {"type": "chat", "text": "gg"},
            {"type": "round_start"},
            {"type": "round_end", "winner": 0, "score": [10, 12], "reason": "bomb"},
            {"type": "bomb_planted", "player": "nut nut", "team": 0, "site": "B"},
            {"type": "bomb_defused", "player": "Br4tkO", "team": 1},
            {"type": "suicide", "player": "a", "team": 0, "weapon": "world"}
        ],
        "streams": [
            {"platform": "twitch", "channel": "eslcs", "name": "ESL TV",
             "url": "https://www.twitch.tv/eslcs", "language": "EN", "viewers": 2548},
            {"platform": "afreeca", "channel": "x", "url": "https://example.com", "viewers": 9000},
            {"platform": "kick", "channel": "eslcs", "name": "ESL TV",
             "url": "https://kick.com/eslcs", "language": null, "viewers": 1095}
        ]
    })
}

#[test]
fn cs2_live_reads_a_file_from_the_server() {
    let live = parse_live(&live()).unwrap();
    assert_eq!(live.id, "tMoSBihfDY_R01-M001");
    assert_eq!(live.updated, 1_794_171_823);
    assert_eq!(live.map, "Anubis");
    assert_eq!(live.round, 23);
    assert_eq!(live.state, RoundState::Started);
    assert_eq!(live.score, [10, 12]);
    assert_eq!(live.sides, [Side::T, Side::Ct]);
    assert!(!live.bomb_planted);
    let first = &live.players[0][0];
    assert_eq!(first.nick, "Chucky---");
    assert!(first.alive);
    assert_eq!(
        (
            first.hp,
            first.money,
            first.kills,
            first.hs,
            first.assists,
            first.deaths
        ),
        (100, 2200, 20, 9, 2, 16)
    );
    assert_eq!(first.adr, 72.1);
    assert_eq!(first.kast, Some(69.6));
    assert_eq!(first.entry, Some([1, 2]));
    assert_eq!(first.weapon.as_deref(), Some("awp"));
    let second = &live.players[1][0];
    assert_eq!(
        (second.kast, second.entry, second.weapon.as_deref()),
        (None, None, None)
    );
    assert_eq!(
        live.history,
        vec![(1, 1, Win::Defuse), (2, 0, Win::Elimination)]
    );
    assert_eq!(live.streams[0].name, "ESL TV");
    assert_eq!(live.streams[0].language.as_deref(), Some("EN"));
    assert_eq!(live.streams[1].language, None);
}

#[test]
fn cs2_live_reads_every_event_and_skips_unknown_ones() {
    let live = parse_live(&live()).unwrap();
    assert_eq!(
        live.log,
        vec![
            Event::Kill {
                killer: "Br4tkO".into(),
                killer_team: 1,
                assister: None,
                victim: "nut nut".into(),
                victim_team: 0,
                weapon: "fiveseven".into(),
                headshot: true,
                smoke: true,
                wallbang: false,
                noscope: false,
                blind: false,
                air: true,
            },
            Event::RoundStart,
            Event::RoundEnd {
                winner: 0,
                score: [10, 12],
                reason: Win::Bomb
            },
            Event::BombPlanted {
                player: "nut nut".into(),
                team: 0,
                site: Some('B')
            },
            Event::BombDefused {
                player: "Br4tkO".into(),
                team: 1
            },
            Event::Suicide {
                player: "a".into(),
                team: 0,
                weapon: "world".into()
            },
        ]
    );
}

#[test]
fn cs2_live_skips_unknown_stream_platforms() {
    let live = parse_live(&live()).unwrap();
    let platforms: Vec<_> = live.streams.iter().map(|stream| stream.platform).collect();
    assert_eq!(platforms, vec![Platform::Twitch, Platform::Kick]);
}

#[test]
fn cs2_live_keeps_five_players_a_team() {
    let live = parse_live(&live()).unwrap();
    assert_eq!(live.players[0].len(), 5);
    assert_eq!(live.players[1].len(), 1);
}

#[test]
fn cs2_live_refuses_a_score_or_sides_not_of_two() {
    let mut value = live();
    value["score"] = json!([10]);
    assert!(parse_live(&value).is_err());
    let mut value = live();
    value["sides"] = json!(["t", "ct", "ct"]);
    assert!(parse_live(&value).is_err());
    let mut value = live();
    value["sides"] = json!(["t", "spectator"]);
    assert!(parse_live(&value).is_err());
}

#[test]
fn cs2_live_cuts_long_strings() {
    let mut value = live();
    value["players"][0][0]["nick"] = json!("x".repeat(100));
    value["map"] = json!("m".repeat(100));
    let live = parse_live(&value).unwrap();
    assert_eq!(live.players[0][0].nick.chars().count(), 64);
    assert_eq!(live.map.chars().count(), 64);
}

#[test]
fn cs2_live_reads_the_index() {
    let (updated, matches) = parse_index(&json!({
        "updated": 1_794_171_810,
        "matches": {"a": {"map": "Mirage", "round": 24, "score": [10, 13]},
                    "bad": {"map": "Nuke", "round": 3, "score": [1]}}
    }))
    .unwrap();
    assert_eq!(updated, 1_794_171_810);
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches["a"],
        RoundScore {
            map: "Mirage".into(),
            round: 24,
            score: [10, 13]
        }
    );
    assert!(parse_index(&json!({"matches": {}})).is_err());
}
