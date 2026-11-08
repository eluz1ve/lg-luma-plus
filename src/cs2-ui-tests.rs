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
        "BO3",
        "Tier 1",
        "Mirage",
        "13:9",
        "Vitality",
        "ZywOo",
        "No lineup yet",
        "Upper bracket · Final",
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
