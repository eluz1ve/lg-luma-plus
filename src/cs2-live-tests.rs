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

const NOW: i64 = 1_794_171_823;

fn index_at(updated: i64) -> (i64, HashMap<String, RoundScore>) {
    let score = RoundScore {
        map: "Anubis".into(),
        round: 23,
        score: [10, 12],
    };
    (updated, HashMap::from([("m1".to_owned(), score)]))
}

fn match_at(updated: i64) -> Live {
    let mut live = parse_live(&live()).unwrap();
    live.id = "m1".into();
    live.updated = updated;
    live
}

fn index_id(commands: &[LiveCommand]) -> Option<u64> {
    commands.iter().find_map(|command| match command {
        LiveCommand::Index { id } => Some(*id),
        _ => None,
    })
}

fn match_id(commands: &[LiveCommand]) -> Option<(u64, String)> {
    commands.iter().find_map(|command| match command {
        LiveCommand::Match { id, match_id } => Some((*id, match_id.clone())),
        _ => None,
    })
}

#[test]
fn cs2_live_asks_for_the_index_once_a_minute_while_home_shows() {
    let mut live = LiveScores::default();
    assert!(live.sync(true, false, None, false, NOW).is_empty());
    assert!(live.sync(false, true, None, false, NOW).is_empty());
    let id = index_id(&live.sync(true, true, None, false, NOW)).unwrap();
    assert!(
        live.sync(true, true, None, false, NOW + 1).is_empty(),
        "one request at a time"
    );
    assert!(live.accept(
        LiveReply::Index {
            id,
            result: Ok(index_at(NOW))
        },
        NOW + 1
    ));
    assert!(live.sync(true, true, None, false, NOW + 30).is_empty());
    assert!(index_id(&live.sync(true, true, None, false, NOW + 1 + INDEX_SECONDS)).is_some());
}

#[test]
fn cs2_live_asks_for_the_open_live_match_every_two_seconds() {
    let mut live = LiveScores::default();
    let (id, wanted) = match_id(&live.sync(true, false, Some("m1"), true, NOW)).unwrap();
    assert_eq!(wanted, "m1");
    assert!(live.accept(
        LiveReply::Match {
            id,
            result: Ok(match_at(NOW))
        },
        NOW
    ));
    assert!(match_id(&live.sync(true, false, Some("m1"), true, NOW + 1)).is_none());
    assert!(match_id(&live.sync(true, false, Some("m1"), true, NOW + DETAIL_SECONDS)).is_some());
}

#[test]
fn cs2_live_asks_nothing_for_a_closed_or_finished_match() {
    let mut live = LiveScores::default();
    assert!(match_id(&live.sync(true, false, None, false, NOW)).is_none());
    assert!(match_id(&live.sync(true, false, Some("m1"), false, NOW)).is_none());
    assert!(match_id(&live.sync(false, false, Some("m1"), true, NOW)).is_none());
}

#[test]
fn cs2_live_ignores_a_reply_for_a_card_closed_since() {
    let mut live = LiveScores::default();
    let (old, _) = match_id(&live.sync(true, false, Some("m1"), true, NOW)).unwrap();
    live.sync(true, false, None, false, NOW);
    let (new, wanted) = match_id(&live.sync(true, false, Some("m2"), true, NOW)).unwrap();
    assert_eq!(wanted, "m2");
    assert!(!live.accept(
        LiveReply::Match {
            id: old,
            result: Ok(match_at(NOW))
        },
        NOW
    ));
    assert!(live.detail("m1", NOW).is_none());
    assert!(live.detail("m2", NOW).is_none());
    let mut other = match_at(NOW);
    other.id = "m2".into();
    assert!(live.accept(
        LiveReply::Match {
            id: new,
            result: Ok(other)
        },
        NOW
    ));
    assert!(live.detail("m2", NOW).is_some());
}

#[test]
fn cs2_live_drops_an_open_match_older_than_thirty_seconds() {
    let mut live = LiveScores::default();
    let (id, _) = match_id(&live.sync(true, false, Some("m1"), true, NOW)).unwrap();
    live.accept(
        LiveReply::Match {
            id,
            result: Ok(match_at(NOW - 5)),
        },
        NOW,
    );
    assert!(live.detail("m1", NOW).is_some());
    assert!(live.detail("m1", NOW - 5 + DETAIL_MAX_AGE).is_some());
    assert!(live.detail("m1", NOW - 4 + DETAIL_MAX_AGE).is_none());
    assert!(live.detail("m2", NOW).is_none());
}

#[test]
fn cs2_live_judges_age_by_its_own_clock_when_the_tv_clock_is_off() {
    let mut live = LiveScores::default();
    let wrong = NOW + 5 * 3600;
    let (id, _) = match_id(&live.sync(true, false, Some("m1"), true, wrong)).unwrap();
    live.accept(
        LiveReply::Match {
            id,
            result: Ok(match_at(NOW)),
        },
        wrong,
    );
    assert!(live.detail("m1", wrong + 10).is_some());
    // The same file again: the scores have not moved for longer than the limit.
    let (id, _) = match_id(&live.sync(true, false, Some("m1"), true, wrong + 20)).unwrap();
    live.accept(
        LiveReply::Match {
            id,
            result: Ok(match_at(NOW)),
        },
        wrong + 20,
    );
    assert!(live.detail("m1", wrong + DETAIL_MAX_AGE + 1).is_none());
}

#[test]
fn cs2_live_drops_round_scores_older_than_three_minutes() {
    let mut live = LiveScores::default();
    let id = index_id(&live.sync(true, true, None, false, NOW)).unwrap();
    live.accept(
        LiveReply::Index {
            id,
            result: Ok(index_at(NOW)),
        },
        NOW,
    );
    assert_eq!(live.round("m1", NOW + INDEX_MAX_AGE).unwrap().round, 23);
    assert!(live.round("m1", NOW + INDEX_MAX_AGE + 1).is_none());
    assert!(live.round("m2", NOW).is_none());
}

#[test]
fn cs2_live_keeps_the_last_data_when_a_request_fails() {
    let mut live = LiveScores::default();
    let id = index_id(&live.sync(true, true, None, false, NOW)).unwrap();
    live.accept(
        LiveReply::Index {
            id,
            result: Ok(index_at(NOW)),
        },
        NOW,
    );
    let id = index_id(&live.sync(true, true, None, false, NOW + INDEX_SECONDS)).unwrap();
    live.accept(
        LiveReply::Index {
            id,
            result: Err("down".into()),
        },
        NOW + INDEX_SECONDS,
    );
    assert!(live.round("m1", NOW + INDEX_SECONDS).is_some());
    let (id, _) = match_id(&live.sync(true, false, Some("m1"), true, NOW)).unwrap();
    live.accept(
        LiveReply::Match {
            id,
            result: Ok(match_at(NOW)),
        },
        NOW,
    );
    let (id, _) = match_id(&live.sync(true, false, Some("m1"), true, NOW + 2)).unwrap();
    live.accept(
        LiveReply::Match {
            id,
            result: Err("down".into()),
        },
        NOW + 2,
    );
    assert!(live.detail("m1", NOW + 2).is_some());
}

#[test]
fn cs2_live_never_asks_for_an_id_that_is_not_a_file_name() {
    let mut live = LiveScores::default();
    for id in ["../index", "a/b", "", "index", "a b"] {
        assert!(
            match_id(&live.sync(true, false, Some(id), true, NOW)).is_none(),
            "{id}"
        );
    }
}

fn stream(platform: Platform, channel: &str, url: &str) -> Stream {
    Stream {
        platform,
        channel: channel.into(),
        name: channel.into(),
        url: url.into(),
        language: None,
        viewers: 0,
    }
}

#[test]
fn a_twitch_stream_opens_its_channel_in_the_twitch_app() {
    let twitch = stream(Platform::Twitch, "ESL_CSGO", "https://www.twitch.tv/eslcs");
    assert_eq!(
        launch_params(&twitch),
        Some((
            "twitch.adamffdev.v1",
            serde_json::json!({"channel": "eslcs"})
        ))
    );
}

#[test]
fn a_twitch_url_without_a_login_opens_nothing() {
    for url in [
        "https://www.twitch.tv/",
        "https://www.twitch.tv/a/b?c",
        "https://twitch.tv/x y",
    ] {
        assert_eq!(
            launch_params(&stream(Platform::Twitch, "x", url)),
            None,
            "{url}"
        );
    }
}

#[test]
fn a_youtube_stream_opens_its_video_in_the_youtube_app() {
    let target = serde_json::json!({"contentTarget": "https://www.youtube.com/tv?v=jfKfPfyJRdk"});
    for url in [
        "https://www.youtube.com/watch?v=jfKfPfyJRdk",
        "https://www.youtube.com/watch?feature=x&v=jfKfPfyJRdk",
        "https://youtu.be/jfKfPfyJRdk",
        "https://www.youtube.com/live/jfKfPfyJRdk?si=1",
        "https://www.youtube.com/embed/jfKfPfyJRdk",
    ] {
        assert_eq!(
            launch_params(&stream(Platform::Youtube, "x", url)),
            Some(("youtube.leanback.v4", target.clone())),
            "{url}"
        );
    }
    for url in [
        "https://www.youtube.com/@channel",
        "https://www.youtube.com/watch?v=short",
    ] {
        assert_eq!(
            launch_params(&stream(Platform::Youtube, "x", url)),
            None,
            "{url}"
        );
    }
}

#[test]
fn kick_streams_open_nothing() {
    let kick = stream(Platform::Kick, "x", "https://kick.com/x");
    assert_eq!(launch_params(&kick), None);
}

#[test]
fn the_open_match_keeps_its_streams_after_the_file_goes_stale() {
    let mut scores = LiveScores::default();
    let commands = scores.sync(true, false, Some("a"), true, 0);
    let Some(LiveCommand::Match { id, .. }) = commands.into_iter().last() else {
        panic!("the match is due");
    };
    let live = parse_live(&serde_json::json!({
        "id": "a", "updated": 0, "map": "Inferno", "round": 1, "state": "started",
        "score": [0, 0], "sides": ["ct", "t"], "players": [[], []], "log": [],
        "streams": [{"platform": "twitch", "channel": "eslcs", "name": "ESL",
            "url": "https://www.twitch.tv/eslcs", "language": "en", "viewers": 10}]
    }))
    .unwrap();
    assert!(scores.accept(
        LiveReply::Match {
            id,
            result: Ok(live)
        },
        0
    ));
    assert_eq!(scores.streams("a").len(), 1);
    assert!(scores.streams("b").is_empty());
}

#[test]
fn the_maps_come_with_their_picks() {
    let mut scores = LiveScores::default();
    let commands = scores.sync(true, false, Some("a"), true, 0);
    let Some(LiveCommand::Match { id, .. }) = commands.into_iter().last() else {
        panic!("the match is due");
    };
    let live = parse_live(&serde_json::json!({
        "id": "a", "updated": 0, "map": "Dust II", "round": 1, "state": "started",
        "score": [0, 0], "sides": ["ct", "t"], "players": [[], []], "log": [], "streams": [],
        "maps": [
            {"map": "Cache", "picked": 1, "score": [2, 13], "winner": 1},
            {"map": "Dust II", "picked": 0, "score": null, "winner": null},
            {"map": "Anubis", "picked": null, "score": null, "winner": null},
            {"map": "", "picked": 0},
            {"map": "Nuke", "picked": 5, "score": [1], "winner": 7}
        ]
    }))
    .unwrap();
    assert!(scores.accept(
        LiveReply::Match {
            id,
            result: Ok(live)
        },
        0
    ));
    let maps = scores.maps("a");
    assert_eq!(
        maps[0],
        LiveMap {
            name: "Cache".into(),
            picked: Some(1),
            score: Some([2, 13]),
            winner: Some(1)
        }
    );
    assert_eq!(maps[2].picked, None);
    assert_eq!(maps.len(), 4, "a map needs a name");
    assert_eq!(
        maps[3],
        LiveMap {
            name: "Nuke".into(),
            picked: None,
            score: None,
            winner: None
        }
    );
    assert!(scores.maps("b").is_empty());
}

#[test]
fn a_file_without_maps_reads() {
    let live = parse_live(&json!({
        "id": "a", "updated": 0, "map": "Dust II", "round": 1, "state": "started",
        "score": [0, 0], "sides": ["ct", "t"], "players": [[], []], "log": [], "streams": []
    }))
    .unwrap();
    assert!(live.maps.is_empty());
}
