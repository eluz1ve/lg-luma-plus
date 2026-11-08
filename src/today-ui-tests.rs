use super::*;
use crate::shows::Show;

/// 1 November 2026, 20:00 UTC; the TV is on UTC.
const NOW: i64 = 1_793_563_200;

fn at(state: &State) -> view::Scene {
    let assets = view::Assets {
        root: PathBuf::new(),
        version: "test".into(),
        metrics: BTreeMap::new(),
        artwork: crate::app_artwork::Service::new(false),
    };
    view::scene(
        state,
        &assets,
        time::OffsetDateTime::from_unix_timestamp(NOW).unwrap(),
    )
}

/// Home with CS2 Scores and TV Shows in Today: a live match, and a series with a new episode;
/// Lampa is installed.
fn today_state() -> State {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.show_shows = true;
    let feed = crate::cs2::parse_feed(&serde_json::json!({
        "updated": NOW,
        "matches": [{
            "id": "m1", "begin_at": "2026-11-01T19:00:00Z", "state": "live", "best_of": 3,
            "tournament": "IEM", "tier": 1, "score": [1, 0],
            "teams": [
                {"name": "Natus Vincere", "short": "NAVI",
                 "image": "https://liquipedia.net/commons/images/navi.png"},
                {"name": "FaZe Clan", "short": "FaZe",
                 "image": "https://liquipedia.net/commons/images/faze.png"}
            ],
            "maps": [{"map": "Mirage", "winner": null}]
        }]
    }))
    .unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, NOW) else {
        panic!("a fetch is due");
    };
    state.cs2.accept(command.id, Ok(feed), NOW);
    state.shows.list = vec![series(7, "Severance")];
    state
        .inventory
        .apps
        .push(crate::model::App::new("com.lampa.app", "Lampa"));
    state
}

/// A series of TV Shows with a new episode on the TV's day.
fn series(id: i64, name: &str) -> Show {
    Show {
        id,
        name: name.into(),
        original: name.into(),
        poster: None,
        imdb: None,
        season: Some(2),
        aired: time::OffsetDateTime::from_unix_timestamp(NOW)
            .unwrap()
            .date(),
        rating: Some(8.3),
    }
}

fn texts(scene: &view::Scene) -> Vec<String> {
    scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(label, ..) => Some(label.clone()),
            _ => None,
        })
        .collect()
}

fn first_tile_y(scene: &view::Scene) -> Option<f32> {
    scene
        .controls
        .iter()
        .find(|c| matches!(c.target, Target::App(_)))
        .map(|c| c.rect.y)
}

/// The opened card's tabs, in order.
fn tabs(scene: &view::Scene) -> Vec<Target> {
    scene
        .controls
        .iter()
        .filter(|c| matches!(c.target, Target::TodayTab(_)))
        .map(|c| c.target.clone())
        .collect()
}

fn today_controls(scene: &view::Scene) -> Vec<(Target, crate::state::Rect)> {
    scene
        .controls
        .iter()
        .filter(|c| {
            matches!(
                c.target,
                Target::TodayCard
                    | Target::TodayMatch(_)
                    | Target::TodayShow(_)
                    | Target::TodayLampa(_)
            )
        })
        .map(|c| (c.target.clone(), c.rect))
        .collect()
}

/// The card opened with OK, its first line focused as `compose` reveals it.
fn opened() -> State {
    let mut state = today_state();
    state.focus = Target::TodayCard;
    assert_eq!(state.activate(), vec![]);
    assert!(matches!(state.overlay, Some(Overlay::Today { .. })));
    state.reveal_focus(&at(&state).controls);
    state
}

type Ink = crate::renderer::Color;

/// What stands right under the text `label`, last drawn when it is drawn twice: the two
/// fills of its capsule, if it has one, and the text's own colour.
fn under(scene: &view::Scene, label: &str) -> (Option<Ink>, Ink, Ink) {
    use crate::renderer::Draw;
    let at = scene
        .draw
        .iter()
        .rposition(|draw| matches!(draw, Draw::Text(text, ..) if text == label))
        .unwrap_or_else(|| panic!("{label} is drawn"));
    let Draw::Text(_, _, _, ink, _) = &scene.draw[at] else {
        unreachable!()
    };
    let Draw::Shape(_, _, _, tint) = &scene.draw[at - 1] else {
        panic!("{label} has no pill");
    };
    let veil = match &scene.draw[at - 2] {
        Draw::Shape(_, _, _, veil) => Some(*veil),
        _ => None,
    };
    (veil, *tint, *ink)
}

#[test]
fn a_live_match_wears_live_and_its_map_on_a_tinted_pill() {
    let (veil, tint, ink) = under(&at(&today_state()), "LIVE · Mirage");
    let veil = veil.expect("a dark capsule on the card's glass");
    assert!(veil[..3].iter().all(|channel| *channel == 0.) && veil[3] >= 0.3);
    assert_eq!(tint[..3], ink[..3], "tinted its own colour");
    assert!(tint[3] > 0. && tint[3] < 0.4, "{tint:?}");
    assert!(ink[0] > 0.8 && ink[1] < 0.6, "red: {ink:?}");
}

#[test]
fn a_focused_live_line_keeps_its_pill_in_a_deeper_red() {
    let state = opened();
    let (_, tint, ink) = under(&at(&state), "LIVE · Mirage");
    assert_eq!(tint[..3], ink[..3]);
    assert!(
        tint[3] <= 0.2,
        "a light capsule on the focused white: {tint:?}"
    );
    assert!(
        ink[0] > 0.6 && ink[0] < 0.9 && ink[1] < 0.3,
        "deep red: {ink:?}"
    );
}

#[test]
fn today_card_is_one_control_and_ok_opens_it_in_the_center() {
    let state = today_state();
    let scene = at(&state);
    let texts = texts(&scene);
    assert!(texts.iter().any(|t| t == "TODAY"));
    assert!(texts.iter().any(|t| t == "NAVI 1–0 FaZe"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "LIVE · Mirage"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Severance"));
    assert!(texts.iter().any(|t| t == "S2 · New episode"), "{texts:?}");
    let card = today_controls(&scene);
    assert_eq!(card.len(), 1, "{card:?}");
    let (target, rect) = &card[0];
    assert_eq!(*target, Target::TodayCard);
    assert!(rect.x >= 1160. && rect.y + rect.h <= 400.);

    let state = opened();
    assert_eq!(state.focus, Target::TodayMatch("m1".into()));
    let scene = at(&state);
    // As tall as its two lines: it hangs from the panel's top and stops short of its bottom.
    let most = crate::state::TODAY_PANEL;
    let panel = scene.modal.expect("the opened card is modal");
    assert!(panel.x == most.x && panel.y == most.y && panel.w == most.w);
    assert!(panel.h < most.h * 2. / 3., "{panel:?}");
    use crate::today::Tab;
    assert_eq!(
        tabs(&scene),
        [Target::TodayTab(Tab::Cs2), Target::TodayTab(Tab::Shows)]
    );
    let lines = today_controls(&scene);
    assert_eq!(
        lines.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>(),
        vec![Target::TodayMatch("m1".into())],
        "CS2 Scores' tab first, and nothing of Home beneath"
    );
    assert!(lines.iter().all(|(_, r)| r.x >= panel.x
        && r.x + r.w <= panel.x + panel.w
        && r.y >= panel.y
        && r.y + r.h <= panel.y + panel.h));
    let texts = self::texts(&scene);
    for expected in [
        "CS2 Scores",
        "Lampa",
        "LIVE NOW",
        "Natus Vincere 1–0 FaZe Clan",
        "IEM · BO3",
        "1 in all",
    ] {
        assert!(
            texts.iter().any(|t| t.ends_with(expected)),
            "{expected}: {texts:?}"
        );
    }
}

#[test]
fn a_match_opened_from_the_card_returns_to_it_and_back_collapses_it() {
    let mut state = opened();
    state.activate();
    assert!(matches!(
        &state.overlay,
        Some(Overlay::Cs2Match { id, restore })
            if id == "m1" && *restore == Target::TodayMatch("m1".into())
    ));
    state.back();
    assert!(matches!(state.overlay, Some(Overlay::Today { .. })));
    assert_eq!(state.focus, Target::TodayMatch("m1".into()));
    state.direction(Direction::Down, &at(&state).controls);
    assert_eq!(
        state.focus,
        Target::TodayMatch("m1".into()),
        "the tab's last line"
    );
    state.back();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::TodayCard);
}

#[test]
fn a_series_opened_from_the_card_opens_it_in_lampa_and_collapses_it() {
    let mut state = opened();
    state.direction(Direction::Right, &at(&state).controls);
    state.reveal_focus(&at(&state).controls);
    assert_eq!(
        state.activate(),
        vec![Effect::OpenCard {
            app: "com.lampa.app".into(),
            card: "tv_7".into()
        }]
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::TodayCard);
}

#[test]
fn the_opened_card_scrolls_to_keep_the_focused_line_in_view() {
    let mut state = today_state();
    state.shows.list = (0..12)
        .map(|id| series(id, &format!("Show {id}")))
        .collect();
    state.focus = Target::TodayCard;
    state.activate();
    state.reveal_focus(&at(&state).controls);
    state.direction(Direction::Right, &at(&state).controls);
    state.reveal_focus(&at(&state).controls);
    for _ in 0..12 {
        state.direction(Direction::Down, &at(&state).controls);
        state.reveal_focus(&at(&state).controls);
    }
    assert_eq!(state.focus, Target::TodayShow(11));
    assert!(state.today_scroll > 0., "{}", state.today_scroll);
    let list = crate::state::TODAY_LIST;
    let scene = at(&state);
    let (_, last) = today_controls(&scene).pop().unwrap();
    let shown = state
        .motion
        .shown(crate::motion::Lane::Today, state.today_scroll);
    let y = last.y + shown - state.today_scroll;
    assert!(y + last.h <= list.y + list.h, "{y}");
    for _ in 0..13 {
        state.direction(Direction::Up, &at(&state).controls);
        state.reveal_focus(&at(&state).controls);
    }
    assert_eq!(state.focus, Target::TodayShow(0), "the tab's first line");
    assert_eq!(state.today_scroll, 0.);
}

#[test]
fn left_and_right_switch_the_opened_card_between_its_widgets() {
    use crate::today::Tab;
    let mut state = opened();
    state.direction(Direction::Left, &at(&state).controls);
    assert!(
        matches!(state.overlay, Some(Overlay::Today { .. })),
        "the first tab: Left stays"
    );
    assert_eq!(state.focus, Target::TodayMatch("m1".into()));
    state.direction(Direction::Right, &at(&state).controls);
    state.reveal_focus(&at(&state).controls);
    assert_eq!(state.today_tab, Some(Tab::Shows));
    assert_eq!(state.focus, Target::TodayShow(7));
    let texts = texts(&at(&state));
    for expected in ["NEW EPISODES", "Severance", "S2 · ★ 8.3", "New episode"] {
        assert!(texts.iter().any(|t| t.ends_with(expected)), "{expected}");
    }
    state.direction(Direction::Right, &at(&state).controls);
    assert_eq!(
        state.today_tab,
        Some(Tab::Shows),
        "the last tab: Right stays"
    );
    state.direction(Direction::Left, &at(&state).controls);
    state.reveal_focus(&at(&state).controls);
    assert_eq!(state.focus, Target::TodayMatch("m1".into()));
    // A pointer picks a tab too, and the card stays open on it.
    state.focus = Target::TodayTab(Tab::Shows);
    assert!(state.activate().is_empty());
    state.reveal_focus(&at(&state).controls);
    assert!(matches!(state.overlay, Some(Overlay::Today { .. })));
    assert_eq!(state.focus, Target::TodayShow(7));
    // Opened again, the card starts on its first tab.
    state.back();
    state.activate();
    state.reveal_focus(&at(&state).controls);
    assert_eq!(state.focus, Target::TodayMatch("m1".into()));
}

#[test]
fn the_cards_match_line_shows_both_teams_logos() {
    let mut state = today_state();
    let logo = |name: &str| format!("https://liquipedia.net/commons/images/{name}.png");
    state.cs2.logos.insert(
        logo("navi"),
        crate::cs2::Logo::Ready(std::sync::Arc::new(crate::app_artwork::Artwork {
            key: "navi".into(),
            pixels: image::RgbaImage::new(1, 1),
            backdrop: [[0; 3]; 2],
            wash: None,
        })),
    );
    state.cs2.logos.insert(
        logo("faze"),
        crate::cs2::Logo::Ready(std::sync::Arc::new(crate::app_artwork::Artwork {
            key: "faze".into(),
            pixels: image::RgbaImage::new(1, 1),
            backdrop: [[0; 3]; 2],
            wash: None,
        })),
    );
    let pictures: Vec<String> = at(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::AppImage(image, rect, ..)
                if rect.x >= 1160. && rect.y < 300. =>
            {
                Some(image.key.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(pictures, ["navi", "faze"]);
}

#[test]
fn no_today_lines_hide_the_card_without_moving_favorites() {
    let with = today_state();
    let without = ready();
    let (with, without) = (at(&with), at(&without));
    assert!(texts(&with).iter().any(|t| t == "TODAY"));
    assert!(!texts(&without).iter().any(|t| t == "TODAY"));
    assert_eq!(first_tile_y(&with), first_tile_y(&without));
}

#[test]
fn up_from_the_first_favorite_reaches_today_and_stops_there() {
    let mut state = today_state();
    let first = state.apps()[0].id.clone();
    state.focus = Target::App(first.clone());
    state.direction(Direction::Up, &at(&state).controls);
    assert_eq!(state.focus, Target::TodayCard);
    state.direction(Direction::Up, &at(&state).controls);
    assert_eq!(state.focus, Target::TodayCard, "nothing above");
    state.direction(Direction::Down, &at(&state).controls);
    assert_eq!(state.focus, Target::App(first));
    state.direction(Direction::Up, &at(&state).controls);
    // Left from Today: nothing lies to its left but the menu.
    state.direction(Direction::Left, &at(&state).controls);
    assert!(matches!(state.overlay, Some(Overlay::Menu { .. })));
}

#[test]
fn a_card_with_nothing_left_returns_focus_to_favorites() {
    let mut state = opened();
    // The match is over and the episode has aired yesterday: the card has nothing left.
    state.preferences.show_cs2 = false;
    state.shows.list.clear();
    state.reveal_focus(&at(&state).controls);
    assert!(state.overlay.is_none(), "the opened card closes");
    state.reveal_focus(&at(&state).controls);
    assert!(matches!(state.focus, Target::App(_)), "{:?}", state.focus);
}

#[test]
fn format_compact_keeps_the_tickers_coins_on_one_line() {
    use crate::crypto::format_compact;
    assert_eq!(format_compact(77_881.97), "77.9K");
    assert_eq!(format_compact(2_262.98), "2,263");
    assert_eq!(format_compact(94.3), "94.30");
    assert_eq!(format_compact(0.00001042), "0.00001042");
    let mut state = ready();
    state.preferences.show_crypto = true;
    state.preferences.crypto_coins = ["BTC", "ETH", "SOL", "DOGE"].map(String::from).to_vec();
    state.crypto.quotes = ["BTC", "ETH", "SOL", "DOGE"]
        .iter()
        .map(|symbol| crate::crypto::Quote {
            symbol: (*symbol).into(),
            price: 77_881.97,
            change: 0.4,
        })
        .collect();
    let scene = at(&state);
    let coins: Vec<_> = scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(label, rect, ..)
                if ["BTC", "ETH", "SOL", "DOGE"]
                    .iter()
                    .any(|c| label.starts_with(c)) =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert!(coins.len() >= 2, "{coins:?}");
    let y = coins[0].y;
    assert!(coins.iter().all(|r| r.y == y), "one line");
    // Past the right edge a symbol is clipped; the ticker's own tests cover that end.
    assert!(coins.iter().all(|r| r.x >= 1160.));
}

#[test]
fn the_crypto_line_colours_each_change_as_exchanges_do() {
    let mut state = ready();
    state.preferences.show_crypto = true;
    state.preferences.crypto_coins = ["BTC", "ETH"].map(String::from).to_vec();
    state.crypto.quotes = [("BTC", 1.2), ("ETH", -0.8)]
        .iter()
        .map(|(symbol, change)| crate::crypto::Quote {
            symbol: (*symbol).into(),
            price: 1000.,
            change: *change,
        })
        .collect();
    let colour = |label: &str| {
        at(&state).draw.iter().find_map(|draw| match draw {
            crate::renderer::Draw::Text(text, _, _, colour, _) if text == label => Some(*colour),
            _ => None,
        })
    };
    let (up, down) = (colour("▲ 1.2%").unwrap(), colour("▼ 0.8%").unwrap());
    assert!(up[1] > up[0] && up[1] > 0.6, "green: {up:?}");
    assert!(down[0] > down[1] && down[0] > 0.8, "red: {down:?}");
    let price = colour("1,000").unwrap();
    assert!(price[0] > 0.9 && price[1] > 0.9, "the price stays light");
    assert_eq!(
        colour("BTC").unwrap(),
        crate::renderer::WHITE,
        "the symbol leads"
    );
    assert_eq!(
        colour("TODAY").unwrap(),
        crate::renderer::WHITE,
        "the heading is white"
    );
    // Readable on any wallpaper: the change stands on a dark capsule tinted its colour.
    let drawn = at(&state);
    let change = drawn
        .draw
        .iter()
        .position(|draw| matches!(draw, crate::renderer::Draw::Text(text, ..) if text == "▲ 1.2%"))
        .unwrap();
    let (
        crate::renderer::Draw::Shape(veil_rect, _, _, veil),
        crate::renderer::Draw::Shape(_, _, _, tint),
    ) = (&drawn.draw[change - 2], &drawn.draw[change - 1])
    else {
        panic!("the change has no pill");
    };
    assert!(
        veil[..3].iter().all(|ink| *ink == 0.) && veil[3] >= 0.3,
        "{veil:?}"
    );
    assert_eq!(tint[..3], up[..3]);
    assert!(tint[3] > 0. && tint[3] < 0.4, "{tint:?}");
    assert!(veil_rect.h > 28., "{veil_rect:?}");
}

#[test]
fn the_crypto_line_says_when_prices_are_loading_or_unavailable() {
    let mut state = ready();
    state.preferences.show_crypto = true;
    assert!(texts(&at(&state)).iter().any(|t| t == "Loading prices…"));
    state.crypto.failed = true;
    assert!(texts(&at(&state)).iter().any(|t| t == "Prices unavailable"));
}

#[test]
fn a_long_weather_line_stays_in_the_header_beside_today() {
    let mut state = today_state();
    let place = crate::weather::Location {
        name: "Llanfairpwllgwyngyllgogerychwyrndrobwllllantysiliogogogoch ".repeat(2),
        region: String::new(),
        country: "Wales".into(),
        latitude: 53.22,
        longitude: -4.2,
        id: None,
    };
    state.weather.cache = Some(crate::weather::Cache {
        key: place.key(),
        updated_at: NOW,
        reading: crate::weather::Reading {
            temperature: 9.,
            code: 2,
            is_day: true,
        },
    });
    state.preferences.weather_location = Some(place);
    state.preferences.greeting_name = "Maximilian-Alexander".into();
    let scene = at(&state);
    for draw in &scene.draw {
        if let crate::renderer::Draw::Text(label, rect, ..) = draw
            && rect.x < 1120.
            && rect.y > 120.
            && rect.y < 400.
        {
            assert!(
                rect.y + rect.h <= 400.,
                "{label} ends at {}",
                rect.y + rect.h
            );
            assert!(
                rect.x + rect.w <= 1121.,
                "{label} reaches {}",
                rect.x + rect.w
            );
        }
    }
}

#[test]
fn a_finished_match_in_the_opened_card_shows_its_score() {
    let mut state = today_state();
    let feed = crate::cs2::parse_feed(&serde_json::json!({
        "updated": NOW,
        "matches": [{
            "id": "m0", "begin_at": "2026-11-01T16:00:00Z", "state": "finished", "best_of": 3,
            "finished_at": NOW - 60,
            "tournament": "IEM", "tier": 1, "score": [2, 1],
            "teams": [{"name": "Team Vitality", "short": "VIT"}, {"name": "MOUZ", "short": "MOUZ"}]
        }]
    }))
    .unwrap();
    let (Some(command), _) = state.cs2.sync(true, true, NOW + 3600) else {
        panic!("a fetch is due");
    };
    state.cs2.accept(command.id, Ok(feed), NOW + 3600);
    state.focus = Target::TodayCard;
    state.activate();
    state.reveal_focus(&at(&state).controls);
    let texts = texts(&at(&state));
    assert!(
        texts.iter().any(|t| t == "Team Vitality 2–1 MOUZ"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "Final"), "{texts:?}");
}

#[test]
fn the_opened_cards_prices_show_each_coins_own_mark() {
    let mut state = today_state();
    state.preferences.show_crypto = true;
    state.preferences.crypto_coins = ["BTC", "ETH", "PEPE"].map(String::from).to_vec();
    let mut state = {
        state.focus = Target::TodayCard;
        state.activate();
        state.today_tab = Some(crate::today::Tab::Crypto);
        state
    };
    state.reveal_focus(&at(&state).controls);
    let marks: Vec<String> = at(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::TintedImage(path, rect, ..) if rect.x < 1100. => {
                let name = path.rsplit('/').next()?.strip_suffix(".lvec")?;
                name.starts_with("coin").then(|| name.to_owned())
            }
            _ => None,
        })
        .collect();
    assert_eq!(marks, ["coin-btc", "coin-eth", "coins"]);
}

#[test]
fn set_to_continue_the_lampa_row_lists_the_accounts_history() {
    use crate::model::LampaFeed;
    let mut state = ready();
    state.preferences.show_shows = true;
    state.preferences.shows_today = false;
    state.shows.list = vec![series(7, "Severance")];
    state.preferences.lampa_feed = LampaFeed::Continue;
    let shown = texts(&at(&state));
    assert!(shown.iter().any(|t| t == "Continue watching"), "{shown:?}");
    assert!(
        shown
            .iter()
            .any(|t| t == "Sign in to CUB in Settings to see what you watched"),
        "{shown:?}"
    );
    assert!(!shown.iter().any(|t| t == "Severance"), "not the feed");
    state.cub.sign_in_with(
        crate::cub::Tokens {
            access: "a".into(),
            refresh: "r".into(),
            expires: NOW + 86_400,
        },
        None,
    );
    state.cub.watched = vec![crate::cub::Watched {
        card: "tv_5".into(),
        title: "Dark".into(),
        poster: None,
        imdb: None,
        episode: Some((2, 3)),
        at: NOW,
    }];
    let shown = texts(&at(&state));
    for expected in ["Dark", "S2 E3"] {
        assert!(shown.iter().any(|t| t == expected), "{expected}: {shown:?}");
    }
    assert_eq!(state.lampa_cards(), ["tv_5"]);
    state.preferences.lampa_feed = LampaFeed::Watching;
    assert!(texts(&at(&state)).iter().any(|t| t == "Watching now"));
    assert_eq!(state.lampa_cards(), ["tv_7"]);
}

#[test]
fn the_lampa_settings_choose_what_the_widget_lists() {
    use crate::model::LampaFeed;
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(7);
    state.activate();
    assert_eq!(state.subpage, Some(crate::state::Subpage::Shows));
    let page = at(&state);
    for target in [
        Target::ShowsToggle,
        Target::ShowsLayout,
        Target::LampaFeed,
        Target::LampaAccount,
    ] {
        assert!(
            page.controls.iter().any(|c| c.target == target),
            "{target:?}"
        );
    }
    assert!(texts(&page).iter().any(|t| t == "Watching now"));
    state.focus = Target::LampaFeed;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.lampa_feed, LampaFeed::Continue);
    assert!(texts(&at(&state)).iter().any(|t| t == "Continue watching"));
    state.activate();
    assert_eq!(state.preferences.lampa_feed, LampaFeed::Watching);
}

#[test]
fn the_plugin_row_explains_lampas_plugin_and_back_returns_to_it() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(7);
    state.activate();
    state.focus = Target::LampaPlugin;
    state.activate();
    assert!(matches!(state.overlay, Some(Overlay::LampaPlugin)));
    assert_eq!(state.focus, Target::CloseDialog);
    let shown = texts(&at(&state));
    assert!(
        shown.iter().any(|t| t == crate::shows::LAMPA_PLUGIN),
        "{shown:?}"
    );
    assert!(shown.iter().any(|t| t.starts_with("In Lampa: Settings")));
    state.back();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::LampaPlugin);
    // Close does the same.
    state.activate();
    state.activate();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::LampaPlugin);
}

#[test]
fn a_perf_script_opens_today_without_walking_the_focus_there() {
    let mut state = today_state();
    let controls = at(&state).controls;
    assert_ne!(state.focus, Target::TodayCard);
    assert_eq!(crate::script("today", &mut state, &controls), Ok(vec![]));
    assert!(matches!(state.overlay, Some(Overlay::Today { .. })));
}

#[test]
fn the_page_under_today_and_the_menu_is_home_as_it_was() {
    let mut state = today_state();
    let home = at(&state);
    assert_eq!(home.ground, home.draw.len(), "nothing lies over Home");
    crate::script("today", &mut state, &home.controls).unwrap();
    let today = at(&state);
    assert!(today.ground < today.draw.len());
    assert!(crate::damage::same_scene(
        &home.draw,
        &today.draw[..today.ground]
    ));

    let mut state = today_state();
    state.open_menu();
    let menu = at(&state);
    assert!(menu.ground < menu.draw.len());
    assert!(crate::damage::same_scene(
        &home.draw,
        &menu.draw[..menu.ground]
    ));
}

#[test]
fn a_perf_script_can_focus_the_today_card_before_opening_it() {
    let mut state = today_state();
    let controls = at(&state).controls;
    assert_eq!(
        crate::script("focus-today", &mut state, &controls),
        Ok(vec![])
    );
    assert_eq!(state.focus, Target::TodayCard);
    assert!(state.overlay.is_none());
}
