use super::*;
use crate::{renderer::TextStyle, state::Rect};

fn with_feed(state: &mut State) {
    let feed = crate::cs2::parse_feed(&serde_json::json!({
        "updated": 0,
        "staff": {"Team_Vitality": [{"name": "XTQZZZ", "role": "Head Coach"}]},
        "matches": [
            {"id": "a", "begin_at": "1970-01-01T00:00:00Z", "state": "live", "best_of": 3,
             "tournament": "IEM Chengdu", "tier": 1, "score": [1, 0],
             "round": {"key": "final", "bracket": "upper"},
             "teams": [{"name": "Team Vitality", "short": "Vitality", "page": "Team_Vitality"}, {"name": "FaZe Clan", "short": "FaZe"}],
             "maps": [{"map": "Mirage", "score": [13, 9], "winner": 0},
                      {"map": "Inferno", "winner": null}],
             "lineups": [[{"name": "ZywOo", "flag": "France"}], []]},
            {"id": "b", "begin_at": "1970-01-02T10:00:00Z", "state": "upcoming", "best_of": 3,
             "tournament": "PGL Masters", "stage": "Playoffs", "tier": 1,
             "teams": [{"name": "MOUZ"}, null], "maps": []}
        ]
    }))
    .unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, 0) else {
        panic!();
    };
    state.cs2.accept(command.id, Ok(feed), 0);
}

#[test]
fn cs2_score_is_switched_in_widgets() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(6);
    state.activate();
    assert_eq!(state.subpage, Some(crate::state::Subpage::Cs2));
    assert_eq!(state.focus, Target::Cs2Toggle);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_cs2);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Layout);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Major);
    state.activate();
    assert!(!state.preferences.cs2_major_only);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Widget(6));
}

#[test]
fn cs2_cards_stand_above_the_favorites_and_open_their_match() {
    let mut state = ready();
    let positions = |scene: &view::Scene| -> Vec<_> {
        scene
            .controls
            .iter()
            .filter(|c| matches!(c.target, Target::App(_) | Target::Input(_)))
            .map(|c| (c.target.clone(), c.rect.x, c.rect.y))
            .collect()
    };
    let before = scene(&state);
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    with_feed(&mut state);
    let after = scene(&state);
    assert_eq!(positions(&before), positions(&after));
    let text: Vec<String> = after
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect();
    for expected in [
        "CS2 Scores",
        "Powered by Liquipedia",
        "LIVE",
        "Team Vitality",
        "Inferno",
        "ZywOo",
        "Tomorrow 10:00 AM",
        "TBD",
        "BO3",
        "Playoffs",
    ] {
        assert!(text.iter().any(|t| t == expected), "{expected} in {text:?}");
    }
    let favorites_top = after
        .controls
        .iter()
        .filter(|c| matches!(c.target, Target::App(_)))
        .map(|c| c.rect.y)
        .fold(f32::INFINITY, f32::min);
    let cards: Vec<_> = after
        .controls
        .iter()
        .filter(|c| matches!(c.target, Target::Cs2Match(_)))
        .collect();
    assert_eq!(cards.len(), 2);
    assert!(
        cards
            .iter()
            .all(|c| c.rect.y > 400. && c.rect.y + c.rect.h < favorites_top)
    );

    state.focus = Target::App("netflix".into());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Match(0));
    let card = controls(&state)
        .into_iter()
        .find(|c| c.target == Target::Cs2Match(0))
        .unwrap();
    assert!(card.rect.y + card.rect.h <= 1080.);
    // OK opens the match: its series, maps and lineups, with Close focused.
    state.activate();
    assert!(matches!(
        &state.overlay,
        Some(crate::state::Overlay::Cs2Match { id, .. }) if id == "a"
    ));
    assert_eq!(state.focus, Target::Cs2Close);
    let dialog = scene(&state);
    let text: Vec<String> = dialog
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect();
    for expected in [
        "IEM Chengdu",
        "Counter-Strike 2 · Upper bracket · Final · BO3 · Tier 1",
        "Mirage",
        "13:9",
        "Vitality",
        "ZywOo",
        "No lineup yet",
        "COACH",
        "XTQZZZ",
        "Liquipedia has no rounds for this map yet",
    ] {
        assert!(text.iter().any(|t| t == expected), "{expected} in {text:?}");
    }
    assert!(dialog.controls.iter().any(|c| c.target == Target::Cs2Close));
    state.back();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::Cs2Match(0));
}

#[test]
fn cs2_cards_scroll_three_at_a_time() {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    state.preferences.cs2_major_only = false;
    let matches: Vec<_> = (0..5)
        .map(|i| {
            serde_json::json!({"id": format!("m{i}"), "begin_at": "1970-01-02T10:00:00Z",
                "state": "upcoming", "tournament": format!("Cup {i}"),
                "teams": [{"name": format!("Team {i}A")}, {"name": format!("Team {i}B")}]})
        })
        .collect();
    let feed =
        crate::cs2::parse_feed(&serde_json::json!({"updated": 0, "matches": matches})).unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, 0) else {
        panic!();
    };
    state.cs2.accept(command.id, Ok(feed), 0);
    let cards = |state: &State| -> Vec<usize> {
        controls(state)
            .iter()
            .filter_map(|c| match c.target {
                Target::Cs2Match(i) => Some(i),
                _ => None,
            })
            .collect()
    };
    assert_eq!(cards(&state), [0, 1, 2]);
    state.focus = Target::Cs2Match(2);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Match(3));
    assert_eq!(cards(&state), [1, 2, 3]);
    let text: Vec<String> = scene(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(text.iter().any(|t| t == "2–4 / 5"), "{text:?}");
    state.direction(Direction::Right, &controls(&state));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(
        state.focus,
        Target::Cs2Match(4),
        "the last card keeps focus"
    );
    for _ in 0..4 {
        state.direction(Direction::Left, &controls(&state));
    }
    assert_eq!(state.focus, Target::Cs2Match(0));
    assert_eq!(cards(&state), [0, 1, 2]);
    // OK opens the card it is on, not the one in the same place before scrolling.
    state.focus = Target::Cs2Match(4);
    state.cs2_reveal(4);
    state.activate();
    assert!(matches!(
        &state.overlay,
        Some(crate::state::Overlay::Cs2Match { id, .. }) if id == "m4"
    ));
}

#[test]
fn up_from_favorites_reaches_cs2_score_without_inputs() {
    let mut state = ready();
    state.inventory.inputs.clear();
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    with_feed(&mut state);
    let favorite = state.apps()[2].id.clone();
    state.focus = Target::App(favorite);
    state.direction(Direction::Up, &controls(&state));
    assert!(
        matches!(state.focus, Target::Cs2Match(_)),
        "{:?}",
        state.focus
    );
    let card = controls(&state)
        .into_iter()
        .find(|c| c.target == state.focus)
        .unwrap();
    assert!(card.rect.y + card.rect.h <= 1080.);
}

#[test]
fn cs2_and_shows_take_turns_in_the_center_from_widgets() {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.show_shows = true;
    let heading = |state: &State, label: &str| {
        scene(state)
            .draw
            .iter()
            .any(|draw| matches!(draw, crate::renderer::Draw::Text(text, ..) if text == label))
    };
    // In Today by default: no rows of their own.
    assert!(state.preferences.cs2_today && state.preferences.shows_today);
    assert!(!heading(&state, "CS2 Scores") && !heading(&state, "Watching now"));
    state.open(2);
    for (widget, layout) in [(6, Target::Cs2Layout), (7, Target::ShowsLayout)] {
        state.focus = Target::Widget(widget);
        state.activate();
        state.focus = layout.clone();
        assert!(controls(&state).iter().any(|c| c.target == layout));
        assert_eq!(state.activate(), vec![Effect::Save]);
        state.direction(Direction::Left, &controls(&state));
    }
    // One widget at a time: Shows in the center sent CS2 back into Today.
    assert!(state.preferences.cs2_today && !state.preferences.shows_today);
    state.open(0);
    assert!(!heading(&state, "CS2 Scores") && heading(&state, "Watching now"));
    // Outside Russian its posters are TVmaze's, credited beside Lampa.
    assert!(heading(&state, "Lampa · TVmaze"));
    state.preferences.language = crate::i18n::Language::Ru;
    assert!(heading(&state, "Lampa") && !heading(&state, "Lampa · TVmaze"));
}

#[test]
fn a_card_names_the_players_and_scores_each_map() {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    let feed = crate::cs2::parse_feed(&serde_json::json!({
        "updated": 0,
        "matches": [{
            "id": "a", "begin_at": "1970-01-01T00:00:00Z", "state": "live", "best_of": 3,
            "tournament": "IEM Chengdu", "tier": 1, "score": [1, 0],
            "teams": [{"name": "Team Vitality"}, {"name": "FaZe Clan"}],
            "maps": [{"map": "Mirage", "score": [13, 9], "winner": 0},
                     {"map": "Inferno", "score": [4, 6], "winner": null},
                     {"map": "Anubis", "winner": null}],
            "lineups": [[{"name": "ZywOo"}, {"name": "flameZ"}], [{"name": "ropz"}]]
        }]
    }))
    .unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, 0) else {
        panic!();
    };
    state.cs2.accept(command.id, Ok(feed), 0);
    let text: Vec<String> = scene(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect();
    for expected in [
        "ZywOo · flameZ",
        "ropz",
        "Mirage",
        "13:9",
        "Inferno",
        "4:6",
        "Anubis",
        "—",
    ] {
        assert!(text.iter().any(|t| t == expected), "{expected} in {text:?}");
    }
}

/// The page of a live match whose first team has these players and coaches.
fn match_page(players: serde_json::Value, staff: serde_json::Value) -> view::Scene {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    state.preferences.cs2_major_only = false;
    let feed = crate::cs2::parse_feed(&serde_json::json!({
        "updated": 0,
        "staff": {"Team_A": staff},
        "matches": [{
            "id": "a", "begin_at": "1970-01-01T00:00:00Z", "state": "live", "best_of": 3,
            "tournament": "IEM Chengdu",
            "teams": [{"name": "Alpha", "short": "Alpha", "page": "Team_A"}, {"name": "Beta"}],
            "maps": [], "lineups": [players, []]
        }]
    }))
    .unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, 0) else {
        panic!();
    };
    state.cs2.accept(command.id, Ok(feed), 0);
    state.focus = Target::Cs2Match(0);
    state.activate();
    assert!(state.overlay.is_some());
    scene(&state)
}

fn drawn(page: &view::Scene) -> Vec<(String, Rect, TextStyle)> {
    page.draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, rect, style, ..) => {
                Some((text.clone(), *rect, *style))
            }
            _ => None,
        })
        .collect()
}

fn line(page: &view::Scene, wanted: &str) -> (Rect, TextStyle) {
    drawn(page)
        .into_iter()
        // The page is drawn last, over a card that can carry the same words.
        .rfind(|(text, ..)| text == wanted)
        .map(|(_, rect, style)| (rect, style))
        .unwrap_or_else(|| panic!("{wanted} in {:?}", drawn(page)))
}

fn flags(page: &view::Scene) -> Vec<(String, Rect)> {
    page.draw
        .iter()
        .filter_map(|draw| match draw {
            // A page that is still fading in tints its images.
            crate::renderer::Draw::Image(path, rect, ..)
            | crate::renderer::Draw::TintedImage(path, rect, ..)
                if path.contains("flags/") =>
            {
                Some((path.clone(), *rect))
            }
            _ => None,
        })
        .collect()
}

fn players(names: &[&str]) -> serde_json::Value {
    names
        .iter()
        .map(|name| serde_json::json!({"name": name, "flag": "France"}))
        .collect()
}

#[test]
fn a_players_flag_stands_left_of_the_name_instead_of_the_country() {
    let page = match_page(players(&["ZywOo"]), serde_json::json!([]));
    let flags = flags(&page);
    assert_eq!(flags.len(), 1);
    let (path, flag) = &flags[0];
    assert!(path.ends_with("flags/fr.png"), "{path}");
    let (name, _) = line(&page, "ZywOo");
    assert!(flag.x + flag.w <= name.x);
    assert!(flag.y < name.y + name.h && name.y < flag.y + flag.h);
    assert!(!drawn(&page).iter().any(|(text, ..)| text == "FRANCE"));
}

#[test]
fn a_player_without_a_flag_stays_in_line() {
    let page = match_page(
        serde_json::json!([
            {"name": "ZywOo", "flag": "France"},
            {"name": "ghost", "flag": "Atlantis"},
            {"name": "nobody"}
        ]),
        serde_json::json!([]),
    );
    assert_eq!(flags(&page).len(), 1);
    let x = line(&page, "ZywOo").0.x;
    assert_eq!(line(&page, "ghost").0.x, x);
    assert_eq!(line(&page, "nobody").0.x, x);
}

#[test]
fn lineups_read_from_the_sofa() {
    let page = match_page(
        players(&["ZywOo"]),
        serde_json::json!([{"name": "XTQZZZ", "role": "Head Coach"}]),
    );
    assert!(line(&page, "ZywOo").1.size >= 24.);
    assert!(line(&page, "Alpha").1.size >= 22.);
    assert!(line(&page, "XTQZZZ").1.size >= 20.);
}

#[test]
fn six_players_and_a_coach_fit_above_the_footer() {
    let page = match_page(
        players(&["p0", "p1", "p2", "p3", "p4", "p5"]),
        serde_json::json!([{"name": "XTQZZZ", "role": "Head Coach"}]),
    );
    let footer = drawn(&page)
        .into_iter()
        .find(|(text, ..)| text.ends_with("CC BY-SA 3.0"))
        .unwrap()
        .1;
    let first = line(&page, "p0").0;
    let second = line(&page, "p1").0;
    let last = line(&page, "p5").0;
    assert!(second.y >= first.y + 30.);
    assert!(last.y + last.h + 20. <= footer.y);
    let coach = line(&page, "XTQZZZ").0;
    assert!(coach.y + coach.h <= first.y);
}

#[test]
fn coaches_who_would_run_into_the_heading_stay_off_the_line() {
    let names: Vec<_> = (0..5)
        .map(|i| serde_json::json!({"name": format!("LongCoachName{i}"), "role": "Coach"}))
        .collect();
    let page = match_page(players(&["ZywOo"]), serde_json::json!(names));
    let text = drawn(&page);
    let (shown, coach, _) = text
        .iter()
        .find(|(text, ..)| text.starts_with("LongCoachName0"))
        .unwrap();
    assert!(!shown.contains("LongCoachName4"), "{shown}");
    let label = line(&page, "COACH").0;
    assert!(label.x < coach.x);
    let heading = line(&page, "Alpha").0;
    assert!(heading.x + heading.w <= label.x);
}

#[test]
fn a_lineup_is_headed_by_the_team_alone() {
    let page = match_page(players(&["ZywOo"]), serde_json::json!([]));
    assert!(!drawn(&page).iter().any(|(text, ..)| text == "Alpha lineup"));
    let heading = line(&page, "Alpha").0;
    let name = line(&page, "ZywOo").0;
    assert!(heading.y + heading.h <= name.y && name.y - heading.y < 60.);
}

/// The feeds server's round score for match `id`, written at `updated` and received at `now`.
pub(crate) fn with_round(state: &mut State, id: &str, map: &str, updated: i64, now: i64) {
    use crate::cs2_live::{LiveCommand, LiveReply, parse_index};
    let commands = state.cs2_live.sync(true, true, None, false, now);
    let Some(LiveCommand::Index { id: request }) = commands.into_iter().next() else {
        panic!("the index is due");
    };
    let index = parse_index(&serde_json::json!({
        "updated": updated,
        "matches": {id: {"map": map, "round": 17, "score": [9, 7]}}
    }))
    .unwrap();
    assert!(state.cs2_live.accept(
        LiveReply::Index {
            id: request,
            result: Ok(index)
        },
        now
    ));
}

fn home_texts(state: &State) -> Vec<String> {
    scene(state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn live_card() -> State {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.cs2_today = false;
    with_feed(&mut state);
    state
}

#[test]
fn the_map_being_played_shows_its_round_score_on_home() {
    let mut state = live_card();
    let before = home_texts(&state);
    assert_eq!(
        before.iter().filter(|t| *t == "LIVE").count(),
        2,
        "{before:?}"
    );
    assert!(!before.iter().any(|t| t == "9:7"));
    with_round(&mut state, "a", "Inferno", 0, 0);
    let after = home_texts(&state);
    assert!(after.iter().any(|t| t == "9:7"), "{after:?}");
    assert_eq!(
        after.iter().filter(|t| *t == "LIVE").count(),
        1,
        "the badge stays: {after:?}"
    );
    assert_eq!(
        after.iter().filter(|t| *t == "Inferno").count(),
        1,
        "{after:?}"
    );
}

#[test]
fn a_map_liquipedia_has_not_reached_yet_takes_the_live_tile() {
    let mut state = live_card();
    with_round(&mut state, "a", "Anubis", 0, 0);
    let texts = home_texts(&state);
    assert!(texts.iter().any(|t| t == "Anubis"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "9:7"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t == "Inferno"),
        "no second live tile: {texts:?}"
    );
}

#[test]
fn an_old_round_score_leaves_home_as_it_was() {
    let mut state = live_card();
    let before = home_texts(&state);
    with_round(
        &mut state,
        "a",
        "Inferno",
        -(crate::cs2_live::INDEX_MAX_AGE + 1),
        0,
    );
    assert_eq!(home_texts(&state), before);
    let mut state = live_card();
    with_round(&mut state, "someone-else", "Inferno", 0, 0);
    assert_eq!(home_texts(&state), before);
}

/// Match "a" of `with_feed` open in detail, its stage set.
fn open_match() -> State {
    let mut state = live_card();
    state.cs2.feed.as_mut().unwrap().matches[0].stage = "Group A".into();
    state.focus = Target::Cs2Match(0);
    state.activate();
    assert!(matches!(
        state.overlay,
        Some(crate::state::Overlay::Cs2Match { .. })
    ));
    state
}

/// The feeds server's file for the open match "a", received at 0.
pub(crate) fn with_detail(state: &mut State, extra: serde_json::Value) {
    use crate::cs2_live::{LiveCommand, LiveReply, parse_live};
    let commands = state.cs2_live.sync(true, false, Some("a"), true, 0);
    let Some(LiveCommand::Match { id, .. }) = commands.into_iter().last() else {
        panic!("the match is due");
    };
    let mut value = serde_json::json!({
        "id": "a", "updated": 0, "map": "Inferno", "round": 24, "state": "started",
        "score": [10, 13], "sides": ["ct", "t"], "players": [[], []], "log": [], "streams": []
    });
    for (key, field) in extra.as_object().into_iter().flatten() {
        value[key] = field.clone();
    }
    let live = parse_live(&value).unwrap();
    assert!(state.cs2_live.accept(
        LiveReply::Match {
            id,
            result: Ok(live)
        },
        0
    ));
}

#[test]
fn the_match_header_says_it_all_on_one_line() {
    let state = live_card();
    let on_home = home_texts(&state).iter().filter(|t| *t == "BO3").count();
    let state = open_match();
    let page = scene(&state);
    line(
        &page,
        "Counter-Strike 2 · Group A · Upper bracket · Final · BO3 · Tier 1",
    );
    let texts = home_texts(&state);
    assert_eq!(
        texts.iter().filter(|t| *t == "BO3").count(),
        on_home,
        "no BO3 chip"
    );
    assert!(!texts.iter().any(|t| t == "Tier 1"), "no tier chip");
}

#[test]
fn the_maps_sit_inside_the_teams_block() {
    let page = scene(&open_match());
    let team = line(&page, "Team Vitality").0;
    for map in ["Mirage", "Inferno"] {
        let tile = line(&page, map).0;
        assert!(
            tile.y > team.y + team.h && tile.y + tile.h < team.y + 180.,
            "{map}: {tile:?} under {team:?}"
        );
    }
    assert!(
        !drawn(&page).iter().any(|(text, ..)| text == "MAPS"),
        "no separate maps heading"
    );
}

#[test]
fn the_live_map_shows_its_rounds_in_the_sides_colours() {
    let mut state = open_match();
    with_detail(&mut state, serde_json::json!({}));
    let page = scene(&state);
    let texts: Vec<String> = drawn(&page).into_iter().map(|(text, ..)| text).collect();
    let at = texts
        .windows(3)
        .rposition(|w| w[0] == "10" && w[1] == ":" && w[2] == "13")
        .unwrap_or_else(|| panic!("10 : 13 in {texts:?}"));
    let colour = |index: usize| match &page
        .draw
        .iter()
        .filter(|d| matches!(d, crate::renderer::Draw::Text(..)))
        .nth(index)
    {
        Some(crate::renderer::Draw::Text(_, _, _, colour, ..)) => *colour,
        _ => panic!(),
    };
    let (ct, t) = (colour(at), colour(at + 2));
    assert!(ct[2] > ct[0], "CT is blue: {ct:?}");
    assert!(t[0] > t[2], "T is yellow: {t:?}");
    line(&page, "Round 24");
}

fn live_player(nick: &str, alive: bool) -> serde_json::Value {
    serde_json::json!({"nick": nick, "alive": alive, "hp": if alive { 64 } else { 0 }, "money": 4150,
        "kills": 18, "hs": 10, "assists": 3, "deaths": 16, "adr": 79.9, "kast": 60.9,
        "entry": [0, 2], "weapon": "ak47"})
}

fn team_of(prefix: &str, count: usize) -> serde_json::Value {
    serde_json::Value::Array(
        (0..count)
            .map(|i| live_player(&format!("{prefix}{i}"), i != 3))
            .collect(),
    )
}

/// Match "a" open with the feeds server's file: 23 rounds played.
fn scoreboard_page(players: serde_json::Value) -> view::Scene {
    let mut state = open_match();
    let history: Vec<_> = (1..=23)
        .map(|round| {
            let kind = ["elimination", "bomb", "defuse", "time"][round as usize % 4];
            serde_json::json!({"round": round, "winner": round % 2, "type": kind})
        })
        .collect();
    with_detail(
        &mut state,
        serde_json::json!({"players": players, "history": history, "round": 24}),
    );
    scene(&state)
}

/// Where the scoreboard is drawn: the left part of the card under the header.
fn scoreboard_area() -> Rect {
    Rect::new(296. + 48., 60. + 360., 760., 540.)
}

fn inside(rect: Rect, area: Rect) -> bool {
    rect.x >= area.x
        && rect.x + rect.w <= area.x + area.w
        && rect.y >= area.y
        && rect.y + rect.h <= area.y + area.h
}

#[test]
fn the_scoreboard_lists_both_teams_inside_its_place() {
    let page = scoreboard_page(serde_json::json!([team_of("a", 5), team_of("b", 5)]));
    for nick in ["a0", "a1", "a2", "a3", "a4", "b0", "b1", "b2", "b3", "b4"] {
        let (rect, _) = line(&page, nick);
        assert!(inside(rect, scoreboard_area()), "{nick}: {rect:?}");
    }
    line(&page, "18 (10)");
    line(&page, "60.9%");
    line(&page, "0:2");
    line(&page, "79.9");
    line(&page, "R: 24 · Inferno");
}

#[test]
fn a_long_nick_stays_in_its_column() {
    let page = scoreboard_page(serde_json::json!([
        [live_player("Bukhavez_Bukhave", true)],
        team_of("b", 5)
    ]));
    let (nick, _) = line(&page, "Bukhavez_Bukhave");
    let money = drawn(&page)
        .into_iter()
        .filter(|(text, ..)| text == "4 150")
        .map(|(_, rect, _)| rect)
        .last()
        .unwrap();
    assert!(
        nick.x + nick.w <= money.x - 70.,
        "{nick:?} before HP and {money:?}"
    );
}

#[test]
fn four_players_keep_the_scoreboard_as_tall() {
    let full = scoreboard_page(serde_json::json!([team_of("a", 5), team_of("b", 5)]));
    let short = scoreboard_page(serde_json::json!([team_of("a", 4), team_of("b", 5)]));
    let (full_b0, short_b0) = (line(&full, "b0").0, line(&short, "b0").0);
    assert_eq!((full_b0.x, full_b0.y), (short_b0.x, short_b0.y));
    assert!(!drawn(&short).iter().any(|(text, ..)| text == "a4"));
}

#[test]
fn the_round_history_has_an_icon_a_round() {
    let page = scoreboard_page(serde_json::json!([team_of("a", 5), team_of("b", 5)]));
    let icons = page
        .draw
        .iter()
        .filter(|draw| {
            matches!(draw, crate::renderer::Draw::TintedImage(path, ..) if path.contains("ui-icons/cs-win-"))
        })
        .count();
    assert_eq!(icons, 23);
}

#[test]
fn without_a_live_file_the_card_keeps_its_lineups() {
    let page = scene(&open_match());
    line(&page, "ZywOo");
    assert!(
        !drawn(&page)
            .iter()
            .any(|(text, ..)| text.starts_with("R: "))
    );
}

#[test]
fn a_player_without_health_is_dimmed_even_when_hltv_calls_him_alive() {
    let mut zero = live_player("zero", true);
    zero["hp"] = serde_json::json!(0);
    let page = scoreboard_page(serde_json::json!([
        [live_player("full", true), zero],
        team_of("b", 5)
    ]));
    let alpha = |nick: &str| {
        page.draw
            .iter()
            .rev()
            .find_map(|draw| match draw {
                crate::renderer::Draw::Text(text, _, _, colour, _) if text == nick => {
                    Some(colour[3])
                }
                _ => None,
            })
            .unwrap()
    };
    // The page may still be fading in, so compare the two.
    assert!(
        alpha("zero") < alpha("full") * 0.5,
        "{} against {}",
        alpha("zero"),
        alpha("full")
    );
}

fn kill(weapon: &str, headshot: bool) -> serde_json::Value {
    serde_json::json!({"type": "kill", "killer": "a0", "killer_team": 0, "assister": "a1",
        "victim": "b0", "victim_team": 1, "weapon": weapon, "headshot": headshot,
        "smoke": false, "wallbang": false, "noscope": false, "blind": false, "air": false})
}

fn round_over() -> serde_json::Value {
    serde_json::json!({"type": "round_end", "winner": 0, "score": [12, 10], "reason": "elimination"})
}

/// Match "a" open with these log events, newest first.
fn log_page(log: serde_json::Value) -> view::Scene {
    let mut state = open_match();
    with_detail(
        &mut state,
        serde_json::json!({"players": [team_of("a", 5), team_of("b", 5)], "log": log}),
    );
    scene(&state)
}

/// Where the game log is drawn: right of the scoreboard, under the header.
fn log_area() -> Rect {
    Rect::new(296. + 822., 60. + 380., 458., 462.)
}

fn icons(page: &view::Scene) -> Vec<(String, Rect)> {
    page.draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::TintedImage(path, rect, ..) if path.contains("ui-icons/cs-") => {
                Some((path.clone(), *rect))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_kill_shows_the_weapon_and_how() {
    let page = log_page(serde_json::json!([
        kill("ak47", true),
        kill("m4a1_silencer", false)
    ]));
    let icons = icons(&page);
    for name in ["cs-weapon-ak47", "cs-weapon-m4a1-silencer", "cs-headshot"] {
        let (_, rect) = icons
            .iter()
            .find(|(path, _)| path.contains(&format!("ui-icons/{name}.lvec")))
            .unwrap_or_else(|| panic!("{name} in {icons:?}"));
        assert!(inside(*rect, log_area()), "{name}: {rect:?}");
    }
    assert_eq!(
        icons
            .iter()
            .filter(|(p, _)| p.contains("cs-headshot"))
            .count(),
        1
    );
    line(&page, "a0");
    line(&page, "b0");
}

#[test]
fn a_weapon_without_an_icon_is_named() {
    let page = log_page(serde_json::json!([kill("zeus_new", false)]));
    line(&page, "zeus_new");
    assert!(!icons(&page).iter().any(|(p, _)| p.contains("zeus")));
}

#[test]
fn round_lines_share_a_smaller_size_than_kills() {
    let page = log_page(serde_json::json!([
        {"type": "round_start"}, round_over(), kill("ak47", false)
    ]));
    let started = line(&page, "Round started").1.size;
    let over = drawn(&page)
        .into_iter()
        .rfind(|(text, ..)| text.starts_with("Round over"))
        .unwrap()
        .2
        .size;
    let killer = line(&page, "a0").1.size;
    assert_eq!(started, over);
    assert!(over < killer, "{over} < {killer}");
}

#[test]
fn a_long_round_over_wraps_inside_the_log() {
    let page = log_page(serde_json::json!([round_over()]));
    let parts: Vec<_> = drawn(&page)
        .into_iter()
        .filter(|(_, rect, _)| inside(*rect, log_area()))
        .filter(|(text, ..)| text.starts_with("Round over") || text == "Enemy eliminated")
        .collect();
    assert_eq!(parts.len(), 2, "{parts:?}");
    assert!(parts[1].1.y > parts[0].1.y);
}

#[test]
fn the_log_stops_at_the_bottom_of_its_place() {
    let events: Vec<_> = (0..40).map(|_| kill("ak47", true)).collect();
    let page = log_page(serde_json::Value::Array(events));
    let names: Vec<_> = drawn(&page)
        .into_iter()
        .filter(|(text, rect, _)| text == "b0" && rect.x >= log_area().x)
        .collect();
    assert!(names.len() > 5 && names.len() < 40, "{}", names.len());
    let area = log_area();
    for (_, rect, _) in names {
        assert!(rect.y + rect.h <= area.y + area.h, "{rect:?}");
    }
}

/// Match "a" open with three streams: Twitch with the fewest viewers, YouTube, Twitch with the most.
fn with_streams(apps: &[&str]) -> State {
    let mut state = open_match();
    // The demo TV has YouTube; each test says which stream apps it has.
    state
        .inventory
        .apps
        .retain(|app| !BOTH.contains(&app.id.as_str()));
    for app in apps {
        state
            .inventory
            .apps
            .push(crate::model::App::new(app, "Streaming"));
    }
    with_detail(
        &mut state,
        serde_json::json!({"streams": [
            {"platform": "twitch", "channel": "cct_ru", "name": "CCT RU", "url": "https://www.twitch.tv/cct_ru", "language": "ru", "viewers": 900},
            {"platform": "youtube", "channel": "CCT Counter-Strike", "name": "CCT", "url": "https://www.youtube.com/watch?v=jfKfPfyJRdk", "language": "en", "viewers": 2000},
            {"platform": "twitch", "channel": "CCT_CS2", "name": "CCT", "url": "https://www.twitch.tv/cct_cs2", "language": "en", "viewers": 5000},
            {"platform": "kick", "channel": "cct_cs", "name": "CCT", "url": "https://kick.com/cct_cs", "language": "en", "viewers": 9000}
        ]}),
    );
    state
}

const BOTH: [&str; 2] = ["twitch.adamffdev.v1", "youtube.leanback.v4"];

fn chips(page: &view::Scene) -> Vec<Rect> {
    page.controls
        .iter()
        .filter_map(|c| match c.target {
            Target::Cs2Stream(_) => Some(c.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_streams_line_up_by_viewers_under_the_title() {
    let page = scene(&with_streams(&BOTH));
    line(&page, "Watch");
    let order: Vec<f32> = [
        "Twitch · CCT_CS2",
        "YouTube · CCT Counter-Strike",
        "Twitch · cct_ru",
    ]
    .iter()
    .map(|name| line(&page, name).0.x)
    .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert_eq!(chips(&page).len(), 3, "no Kick chip");
    let chip = chips(&page)[0];
    assert!(
        chip.y >= 60. + 110. && chip.y + chip.h <= 60. + 124. + 60.,
        "{chip:?}"
    );
}

#[test]
fn a_stream_shows_only_with_its_app() {
    let page = scene(&with_streams(&["youtube.leanback.v4"]));
    assert_eq!(chips(&page).len(), 1);
    line(&page, "YouTube · CCT Counter-Strike");
    assert!(
        !drawn(&page)
            .iter()
            .any(|(text, ..)| text.starts_with("Twitch"))
    );
}

#[test]
fn without_a_stream_app_the_header_stays_up() {
    let without = scene(&with_streams(&[]));
    assert!(chips(&without).is_empty());
    assert!(!drawn(&without).iter().any(|(text, ..)| text == "Watch"));
    let with = scene(&with_streams(&BOTH));
    let team = |page: &view::Scene| line(page, "Team Vitality").0.y;
    assert_eq!(team(&with) - team(&without), 60.);
}

#[test]
fn the_streams_take_the_focus_between_close_and_the_card() {
    let mut state = with_streams(&BOTH);
    let controls = |state: &State| scene(state).controls;
    assert_eq!(state.focus, Target::Cs2Close);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Stream(0));
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Stream(0), "Left stops at the first");
    state.direction(Direction::Right, &controls(&state));
    state.direction(Direction::Right, &controls(&state));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Stream(2), "Right stops at the last");
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Stream(2), "nothing to focus below");
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Cs2Close);
}

#[test]
fn ok_on_a_stream_opens_it_in_its_app() {
    let mut state = with_streams(&BOTH);
    state.focus = Target::Cs2Stream(1);
    assert_eq!(
        state.activate(),
        vec![Effect::OpenContent {
            app: "youtube.leanback.v4".into(),
            params: serde_json::json!({"contentTarget": "https://www.youtube.com/tv?v=jfKfPfyJRdk"}),
        }]
    );
    state.focus = Target::Cs2Stream(0);
    assert_eq!(
        state.activate(),
        vec![Effect::OpenContent {
            app: "twitch.adamffdev.v1".into(),
            params: serde_json::json!({"channel": "cct_cs2"}),
        }]
    );
}
