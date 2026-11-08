use super::*;
use crate::{
    model::{Preferences, demo_inventory},
    shows::Show,
};

/// 1 November 2026, 20:00 UTC; the TV's day is that UTC day.
const NOW: i64 = 1_793_563_200;
const DATE: time::Date = match time::Date::from_calendar_date(2026, time::Month::November, 1) {
    Ok(date) => date,
    Err(_) => panic!(),
};

fn state() -> State {
    let mut state = State::new(Preferences {
        show_cs2: true,
        show_shows: true,
        ..Preferences::default()
    });
    state.set_inventory(demo_inventory(), None);
    state
}

/// A feed of `(id, state, begins)` matches, received just now.
fn feed(state: &mut State, matches: &[(&str, &str, i64)]) {
    let iso = |unix: i64| {
        time::OffsetDateTime::from_unix_timestamp(unix)
            .unwrap()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    };
    let matches: Vec<_> = matches
        .iter()
        .map(|(id, kind, begins)| {
            serde_json::json!({
                "id": id, "begin_at": iso(*begins), "state": kind, "best_of": 3,
                "finished_at": NOW - 60,
                "tournament": "IEM", "tier": 1,
                "teams": [{"name": "Vitality"}, {"name": "FaZe"}], "maps": []
            })
        })
        .collect();
    let feed =
        crate::cs2::parse_feed(&serde_json::json!({"updated": NOW, "matches": matches})).unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, NOW) else {
        panic!("a fetch is due");
    };
    state.cs2.accept(command.id, Ok(feed), NOW);
}

/// A series of TV Shows whose latest episode came out `days` before today.
fn show(id: i64, days: i64) -> Show {
    Show {
        id,
        name: format!("Show {id}"),
        original: format!("Show {id}"),
        poster: None,
        imdb: None,
        season: Some(1),
        aired: DATE - time::Duration::days(days),
        rating: None,
    }
}

fn episode(show: i64) -> Line {
    Line::Episode { show }
}

fn matched(id: &str) -> Line {
    Line::Match { id: id.into() }
}

#[test]
fn live_matches_come_first_then_matches_by_time_then_new_episodes() {
    let mut state = state();
    feed(
        &mut state,
        &[
            ("soon", "upcoming", NOW + 2 * 3600),
            ("live", "live", NOW - 1800),
            ("done", "finished", NOW - 7200),
            ("later", "upcoming", NOW + 13 * 3600),
        ],
    );
    state.shows.list = vec![show(1, 1), show(2, 0)];
    let today = today(&state, NOW, DATE);
    assert_eq!(
        today.lines,
        [matched("live"), matched("soon"), episode(1)],
        "the series in the feed's order"
    );
    assert_eq!(today.more, 1, "three lines at most");
}

#[test]
fn crypto_beside_is_always_the_last_line_and_the_rest_counts_as_more() {
    let mut state = state();
    state.preferences.show_crypto = true;
    state.preferences.crypto_beside = true;
    feed(
        &mut state,
        &[
            ("a", "live", NOW),
            ("b", "live", NOW),
            ("c", "live", NOW),
            ("d", "live", NOW),
            ("e", "live", NOW),
        ],
    );
    let today = today(&state, NOW, DATE);
    assert_eq!(today.lines, [matched("a"), matched("b"), Line::Crypto]);
    assert_eq!(today.more, 3);
}

#[test]
fn the_center_widget_or_one_switched_off_adds_nothing() {
    let mut state = state();
    feed(&mut state, &[("a", "live", NOW)]);
    state.shows.list = vec![show(1, 0)];
    state.preferences.show_crypto = true;
    state.preferences.crypto_beside = false;
    state.preferences.cs2_today = false;
    state.preferences.shows_today = false;
    // Only one widget stands in the center, CS2 first; the others stay in Today.
    assert_eq!(today(&state, NOW, DATE).lines, [episode(1), Line::Crypto]);
    state.preferences.show_crypto = false;
    state.preferences.cs2_today = true;
    state.preferences.shows_today = true;
    state.preferences.show_cs2 = false;
    state.preferences.show_shows = false;
    assert_eq!(today(&state, NOW, DATE), Today::default());
}

#[test]
fn series_need_an_episode_out_today() {
    let mut state = state();
    state.shows.list = vec![show(1, 2), show(2, 1), show(3, 9), show(4, 0)];
    assert_eq!(
        today(&state, NOW, DATE).lines,
        [episode(2), episode(4)],
        "dated yesterday may be out today"
    );
}

#[test]
fn a_stale_feed_adds_no_matches() {
    let mut state = state();
    feed(&mut state, &[("a", "live", NOW)]);
    let later = NOW + crate::cs2::MAX_AGE + 1;
    assert!(today(&state, later, DATE).lines.is_empty());
}

#[test]
fn old_preferences_without_today_flags_put_cs2_and_shows_in_today() {
    let preferences: Preferences = serde_json::from_str("{}").unwrap();
    assert!(preferences.cs2_today);
    assert!(preferences.shows_today);
    assert_eq!(preferences.lampa_feed, crate::model::LampaFeed::Watching);
}

/// Signed in to CUB, the Lampa widget set to the history.
fn signed_in(state: &mut State) {
    state.preferences.lampa_feed = crate::model::LampaFeed::Continue;
    state.cub.sign_in_with(
        crate::cub::Tokens {
            access: "a".into(),
            refresh: "r".into(),
            expires: NOW + 86_400,
        },
        None,
    );
}

/// A release of `card` `days` before today.
fn release(card: &str, days: i64, watched: bool) -> crate::cub::Release {
    crate::cub::Release {
        card: card.into(),
        title: card.into(),
        poster: None,
        imdb: None,
        date: DATE - time::Duration::days(days),
        season: 1,
        episodes: vec![1],
        badge: crate::cub::Badge::None,
        watched,
    }
}

fn watched(card: &str, hours: i64) -> crate::cub::Watched {
    crate::cub::Watched {
        card: card.into(),
        title: card.into(),
        poster: None,
        imdb: None,
        episode: Some((1, 1)),
        at: NOW - hours * 3600,
    }
}

fn fresh(card: &str, days: i64) -> Line {
    Line::Release {
        card: card.into(),
        date: DATE - time::Duration::days(days),
    }
}

fn resume(card: &str) -> Line {
    Line::Continue { card: card.into() }
}

#[test]
fn lampa_lists_todays_releases_then_unwatched_ones_then_what_to_continue() {
    let mut state = state();
    signed_in(&mut state);
    state.cub.releases = vec![
        release("tv_today", 0, false),
        release("tv_seen", 0, true),
        release("tv_week", 3, false),
        // An older episode of the same show gives way to the newer one.
        release("tv_week", 5, false),
        release("tv_tomorrow", -1, false),
    ];
    state.cub.watched = vec![
        watched("tv_today", 2),
        watched("tv_last", 30),
        watched("tv_month", 24 * 30),
    ];
    let today = today(&state, NOW, DATE);
    assert_eq!(
        today.lines,
        [fresh("tv_today", 0), fresh("tv_week", 3), resume("tv_last")]
    );
    assert_eq!(today.more, 0);
}

#[test]
fn series_without_a_known_episode_are_continued_and_films_are_not() {
    let mut state = state();
    signed_in(&mut state);
    let mut film = watched("movie_550", 1);
    film.episode = None;
    let mut series = watched("tv_1399", 3);
    series.episode = None;
    state.cub.watched = vec![film, series];
    assert_eq!(today(&state, NOW, DATE).lines, [resume("tv_1399")]);
}

#[test]
fn each_kind_of_line_gets_one_before_any_gets_two() {
    let mut state = state();
    signed_in(&mut state);
    feed(
        &mut state,
        &[
            ("live", "live", NOW - 600),
            ("soon", "upcoming", NOW + 3600),
            ("later", "upcoming", NOW + 7200),
        ],
    );
    state.cub.releases = vec![release("tv_a", 0, false), release("tv_b", 0, false)];
    state.cub.watched = vec![watched("tv_c", 5)];
    let today = today(&state, NOW, DATE);
    assert_eq!(
        today.lines,
        [matched("live"), fresh("tv_a", 0), matched("soon")]
    );
    assert_eq!(today.more, 3);
}

#[test]
fn the_lampa_widget_lists_whichever_its_set_to() {
    let mut state = state();
    state.shows.list = vec![show(1, 0)];
    signed_in(&mut state);
    state.cub.watched = vec![watched("tv_c", 5)];
    assert_eq!(
        today(&state, NOW, DATE).lines,
        [resume("tv_c")],
        "the history, not the feed"
    );
    state.preferences.lampa_feed = crate::model::LampaFeed::Watching;
    assert_eq!(today(&state, NOW, DATE).lines, [episode(1)]);
    // Set to the history but signed out, it has nothing.
    state.preferences.lampa_feed = crate::model::LampaFeed::Continue;
    state.cub.sign_out();
    assert_eq!(today(&state, NOW, DATE), Today::default());
}

#[test]
fn opened_each_widget_has_a_tab_with_all_it_has() {
    let mut state = state();
    state.preferences.show_crypto = true;
    state.preferences.crypto_beside = true;
    feed(
        &mut state,
        &[
            ("a", "live", NOW),
            ("soon", "upcoming", NOW + 3600),
            ("tomorrow", "upcoming", NOW + 30 * 3600),
            ("done", "finished", NOW - 7200),
        ],
    );
    state.shows.list = vec![show(1, 0), show(2, 5)];
    // The card keeps three lines; opened, every widget shows all of its own.
    assert_eq!(today(&state, NOW, DATE).more, 1);
    assert_eq!(
        tabs(&state, NOW, DATE),
        [
            (
                Tab::Cs2,
                vec![
                    (Section::Live, vec![matched("a")]),
                    (Section::Matches, vec![matched("soon"), matched("tomorrow")]),
                    (Section::Results, vec![matched("done")]),
                ]
            ),
            (
                Tab::Shows,
                vec![(Section::Episodes, vec![episode(1), episode(2)])]
            ),
            (Tab::Crypto, vec![(Section::Prices, vec![Line::Crypto])]),
        ]
    );
    state.preferences.cs2_results = false;
    assert_eq!(
        tabs(&state, NOW, DATE)[0].1.last().unwrap().0,
        Section::Matches,
        "results only when CS2 Scores shows them"
    );
}

#[test]
fn opened_with_lampa_its_tab_has_releases_and_series_to_continue() {
    let mut state = state();
    signed_in(&mut state);
    state.shows.list = vec![show(1, 0)];
    state.cub.releases = vec![release("tv_a", 0, false), release("tv_b", 2, false)];
    state.cub.watched = vec![watched("tv_c", 5)];
    assert_eq!(
        tabs(&state, NOW, DATE),
        [(
            Tab::Lampa,
            vec![
                (Section::Episodes, vec![fresh("tv_a", 0), fresh("tv_b", 2)]),
                (Section::Continue, vec![resume("tv_c")]),
            ]
        )],
        "TV Shows give way to Lampa"
    );
}

#[test]
fn a_widget_with_nothing_has_no_tab() {
    let mut state = state();
    feed(&mut state, &[("done", "finished", NOW - 7200)]);
    state.preferences.cs2_results = false;
    assert!(tabs(&state, NOW, DATE).is_empty());
}
