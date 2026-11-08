use super::*;
#[test]
fn crypto_coins_are_chosen_in_widgets_up_to_the_limit() {
    let limit = crate::crypto::MAX_COINS;
    assert!(limit > 4);
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(5);
    state.activate();
    assert_eq!(state.subpage, Some(crate::state::Subpage::Crypto));
    assert_eq!(state.focus, Target::CryptoToggle);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_crypto);
    // BTC, ETH and SOL are chosen by default, and a fourth no longer fills the list.
    state.focus = Target::CryptoCoin(3);
    assert_eq!(state.activate(), vec![Effect::Save]);
    state.focus = Target::CryptoCoin(7);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(
        state.preferences.crypto_coins,
        ["BTC", "ETH", "SOL", "BNB", "TON"]
    );
    assert!(state.notice.is_none());
    // One past the limit is refused.
    state.preferences.crypto_coins = (0..limit).map(|n| format!("COIN{n}")).collect();
    state.focus = Target::CryptoCoin(0);
    assert!(
        state.activate().is_empty(),
        "a coin past the limit is refused"
    );
    assert!(state.notice.is_some());
    assert_eq!(state.preferences.crypto_coins.len(), limit);
    state.focus = Target::CryptoCoin(8);
    state.activate();
    assert_eq!(state.preferences.crypto_coins.len(), limit - 1);
    assert!(
        controls(&state)
            .iter()
            .any(|c| c.target == Target::CryptoCoin(7))
    );
    state.focus = Target::CryptoToggle;
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Widget(5));
}

#[test]
fn crypto_prices_leave_home_as_it_is_and_open_their_settings_from_today() {
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
    state.preferences.show_crypto = true;
    let (Some(command), _) =
        state
            .crypto
            .sync(true, &state.preferences.crypto_coins.clone(), true, 1000)
    else {
        panic!();
    };
    let quote = |symbol: &str, price, change| crate::crypto::Quote {
        symbol: symbol.into(),
        price,
        change,
    };
    state.crypto.accept(
        command.id,
        Ok(vec![
            quote("BTC", 67420.12, 1.23),
            quote("ETH", 2600.5, -3.69),
            quote("SOL", 0.5, 0.),
        ]),
        1000,
    );
    let after = scene(&state);
    assert_eq!(positions(&before), positions(&after));
    assert!(
        !after
            .controls
            .iter()
            .any(|c| matches!(c.target, Target::Coin(_))),
        "no coin cards on Home"
    );
    // Up from a favorite reaches the Today card; its list has the whole price of each coin.
    state.focus = Target::App("netflix".into());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::TodayCard);
    state.activate();
    state.today_motion = None;
    let opened = scene(&state);
    let text = drawn_text(&opened);
    for expected in [
        "BTC",
        "Bitcoin",
        "$67,420.12",
        "▲ 1.23%",
        "$2,600.50",
        "▼ 3.69%",
        "$0.5000",
        "▲ 0.00%",
    ] {
        assert!(text.iter().any(|t| t == expected), "{expected} in {text:?}");
    }
    state.focus = Target::Coin(0);
    state.activate();
    assert_eq!(state.subpage, Some(crate::state::Subpage::Crypto));
    assert_eq!(state.focus, Target::CryptoToggle);
}

fn drawn_text(scene: &view::Scene) -> Vec<String> {
    scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(text, ..) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn any_binance_coin_is_found_by_search_and_listed_after_the_usual_ones() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(5);
    state.activate();
    state.focus = Target::CryptoSearch;
    state.activate();
    assert!(state.text_input_active());
    assert_eq!(state.keyboard_field, crate::state::TextField::Coin);
    assert!(
        state.crypto.search.wanted,
        "the first search loads Binance's coins"
    );
    state.crypto.search.wanted = false;
    state.crypto.search.loading = true;
    state
        .crypto
        .search
        .accept(Ok(["APE", "BTC", "PEPE"].map(String::from).to_vec()));
    state.text("pe");
    assert_eq!(state.editing_text(), "PE");
    assert!(
        state.query.is_empty(),
        "the coin search leaves the app search alone"
    );
    state.overlay = None;
    assert!(
        controls(&state)
            .iter()
            .any(|c| c.target == Target::CryptoResult(1))
    );
    state.focus = Target::CryptoResult(0);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(
        state.preferences.crypto_coins,
        ["BTC", "ETH", "SOL", "PEPE"]
    );
    // Found coins stay listed after the usual eight, so they can be turned off there.
    let choices = state.crypto_choices();
    assert_eq!(choices.len(), crate::crypto::COINS.len() + 1);
    state.focus = Target::CryptoCoin(choices.len() - 1);
    state.activate();
    assert_eq!(state.preferences.crypto_coins, ["BTC", "ETH", "SOL"]);
}

#[test]
fn crypto_is_a_line_in_today_and_never_takes_the_center() {
    let mut state = priced(&["BTC", "ETH", "SOL", "DOGE"]);
    assert_eq!(state.preferences.center(), None);
    assert!(state.preferences.crypto_in_today());
    assert!(text_at(&scene(&state), "TODAY").is_some_and(|rect| rect.x >= 1160. && rect.y < 400.));
    let coin = text_at(&scene(&state), "BTC").expect("the coins are listed");
    assert!(coin.x > 1160. && coin.y < 400.);
    assert!(text_at(&scene(&state), "Crypto").is_none());
    // Settings no longer asks where it goes: below the switch, straight to the coins.
    state.open(2);
    state.focus = Target::Widget(5);
    state.activate();
    let rows: Vec<_> = controls(&state)
        .into_iter()
        .map(|control| control.target)
        .filter(|target| {
            matches!(
                target,
                Target::CryptoToggle | Target::CryptoSearch | Target::CryptoCoin(0)
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            Target::CryptoToggle,
            Target::CryptoSearch,
            Target::CryptoCoin(0)
        ]
    );
    // Nothing without coins.
    state.preferences.crypto_coins.clear();
    assert!(!state.preferences.crypto_in_today());
}

/// Home with these coins chosen and priced.
fn priced(symbols: &[&str]) -> State {
    let mut state = ready();
    state.preferences.show_crypto = true;
    state.preferences.crypto_coins = symbols.iter().map(|s| (*s).to_owned()).collect();
    state.crypto.quotes = symbols
        .iter()
        .map(|symbol| crate::crypto::Quote {
            symbol: (*symbol).into(),
            price: 77_881.97,
            change: 0.4,
        })
        .collect();
    state
}

/// Home with more coins than the Today card's line holds, its ticker's lap known.
fn ticking() -> State {
    let names = ["BTC", "ETH", "SOL", "BNB", "XRP", "ADA", "DOGE", "TON"];
    let mut state = priced(&names);
    state.ticker_span = scene(&state).ticker;
    state
}

fn text_at(scene: &view::Scene, wanted: &str) -> Option<crate::state::Rect> {
    scene.draw.iter().find_map(|draw| match draw {
        crate::renderer::Draw::Text(label, rect, ..) if label.starts_with(wanted) => Some(*rect),
        _ => None,
    })
}

/// Everywhere `wanted` is drawn, left to right.
fn all_at(scene: &view::Scene, wanted: &str) -> Vec<f32> {
    let mut places: Vec<f32> = scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(label, rect, ..) if label == wanted => Some(rect.x),
            _ => None,
        })
        .collect();
    places.sort_by(f32::total_cmp);
    places
}

#[test]
fn coins_wider_than_todays_line_run_across_it() {
    let mut state = ticking();
    assert!(state.ticker_span > 0.);
    let start = text_at(&scene(&state), "BTC").unwrap();
    state.ticker = 30.;
    let moved = text_at(&scene(&state), "ETH").unwrap();
    state.ticker = 0.;
    let eth = text_at(&scene(&state), "ETH").unwrap();
    assert_eq!(moved.x, eth.x - 30.);
    assert_eq!(moved.y, start.y, "one line");
}

#[test]
fn the_ticker_keeps_its_coins_inside_the_card_and_fades_at_its_edges() {
    let drawn = scene(&ticking());
    let lines: Vec<_> = drawn
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::EdgeFadeX(Some(edges)) if edges[0] >= 1160. => Some(*edges),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 1);
    let [left, full, solid, right] = lines[0];
    assert!(left < full && full < solid && solid < right);
    assert!(right <= 1840. - 24.);
}

#[test]
fn the_tickers_end_joins_its_start_without_a_gap() {
    let mut state = ticking();
    let span = state.ticker_span;
    let origin = scene(&state);
    let first = text_at(&origin, "BTC").unwrap().x;
    state.ticker = span;
    let lap = scene(&state);
    assert_eq!(all_at(&lap, "BTC"), all_at(&origin, "BTC"));
    // Near the end of a lap the first coin is already coming back in.
    state.ticker = span - 10.;
    assert!(all_at(&scene(&state), "BTC").contains(&(first + 10.)));
}

/// Where the UI icon `name` is drawn, left to right.
fn icons_at(scene: &view::Scene, name: &str) -> Vec<crate::state::Rect> {
    let file = format!("ui-icons/{name}.lvec");
    let mut places: Vec<_> = scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::TintedImage(path, rect, ..) if path.ends_with(&file) => {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    places.sort_by(|a, b| a.x.total_cmp(&b.x));
    places
}

#[test]
fn each_coin_on_the_ticker_leads_with_its_own_mark() {
    let drawn = scene(&priced(&["BTC", "ETH"]));
    for (mark, symbol) in [("coin-btc", "BTC"), ("coin-eth", "ETH")] {
        let icon = icons_at(&drawn, mark)[0];
        let name = text_at(&drawn, symbol).unwrap();
        assert!(icon.x + icon.w <= name.x, "{mark} stands before {symbol}");
        assert!(name.x - (icon.x + icon.w) <= 12., "and close to it");
    }
}

#[test]
fn the_prices_line_opens_with_a_general_mark_rather_than_bitcoins() {
    let drawn = scene(&priced(&["ETH"]));
    assert!(icons_at(&drawn, "bitcoin").is_empty());
    let general = icons_at(&drawn, "coins");
    assert_eq!(general.len(), 1);
    assert!(general[0].x + general[0].w <= icons_at(&drawn, "coin-eth")[0].x);
    // A coin without a mark of its own takes the same general one on its way past.
    assert_eq!(icons_at(&scene(&priced(&["PEPE"])), "coins").len(), 2);
}

#[test]
fn coins_that_fit_stand_still() {
    let mut state = priced(&["BTC"]);
    let still = scene(&state);
    assert_eq!(still.ticker, 0.);
    let place = text_at(&still, "BTC").unwrap();
    state.ticker = 50.;
    assert_eq!(text_at(&scene(&state), "BTC").unwrap().x, place.x);
}

#[test]
fn reduced_motion_holds_the_ticker() {
    let mut state = ticking();
    state.preferences.reduced_motion = true;
    let held = scene(&state);
    assert_eq!(held.ticker, 0.);
    let place = text_at(&held, "BTC").unwrap();
    state.ticker = 50.;
    assert_eq!(text_at(&scene(&state), "BTC").unwrap().x, place.x);
}

/// Lets focus and scrolling come to rest.
fn settle(state: &mut State) {
    for _ in 0..120 {
        state.animate(1. / 60.);
    }
    state.dirty = false;
}

#[test]
fn the_ticker_runs_a_pixel_a_frame_and_keeps_home_counted_as_still() {
    let mut state = ticking();
    settle(&mut state);
    let start = state.ticker;
    state.animate(1. / 60.);
    assert_eq!(state.ticker, start + 1., "a pixel at 60 Hz");
    assert!(state.dirty, "the ticker asks for its frame");
    assert!(state.ticker_only, "and only the ticker changed");
    assert!(
        !state.animating(),
        "idle work and wallpapers do not wait for it"
    );
    state.dirty = false;
    state.animate(1. / 30.);
    assert_eq!(state.ticker, start + 3., "the same speed at 30 Hz");
    state.ticker = state.ticker_span - 1.;
    state.animate(1. / 30.);
    assert_eq!(state.ticker, 1., "it wraps within a lap");
}

#[test]
fn only_a_bare_home_runs_the_ticker() {
    let mut state = ticking();
    settle(&mut state);
    assert!(state.ticker_running());
    state.open_menu();
    assert!(!state.ticker_running(), "under an overlay");
    state.overlay = None;
    state.page = Page::Settings;
    assert!(!state.ticker_running(), "on another page");
    state.page = Page::Home;
    state.preferences.reduced_motion = true;
    assert!(!state.ticker_running(), "with reduced motion");
    state.preferences.reduced_motion = false;
    state.previous_background = Some(("old".into(), std::time::Instant::now()));
    assert!(!state.ticker_running(), "while a wallpaper fades");
    state.previous_background = None;
    state.ticker_span = 0.;
    assert!(!state.ticker_running(), "when the coins fit");
    let at = state.ticker;
    state.animate(1.);
    assert_eq!(state.ticker, at);
}

#[test]
fn a_ticker_frame_redraws_only_its_line_for_a_whole_lap() {
    let mut state = ticking();
    let mut tracker = crate::damage::Tracker::default();
    tracker.push(&scene(&state).draw);
    let span = state.ticker_span as usize;
    for offset in (7..span).step_by(7) {
        state.ticker = offset as f32;
        tracker.push(&scene(&state).draw);
        let area = tracker.damage(1).expect("a partial redraw");
        assert!(
            area.y >= 45. && area.h <= 56. && area.x >= 1160. && area.x + area.w <= 1840.,
            "at {offset}: {area:?}"
        );
    }
}

#[test]
fn todays_list_holds_every_coin_up_to_the_limit() {
    let names: Vec<String> = (0..crate::crypto::MAX_COINS)
        .map(|n| format!("COIN{n}"))
        .collect();
    let mut state = priced(&names.iter().map(String::as_str).collect::<Vec<_>>());
    state.focus = Target::TodayCard;
    state.activate();
    state.today_motion = None;
    let last = Target::Coin(crate::crypto::MAX_COINS - 1);
    let mut guard = 0;
    while state.focus != last {
        state.direction(Direction::Down, &controls(&state));
        guard += 1;
        assert!(guard < 60, "stuck at {:?}", state.focus);
    }
    state.reveal_focus(&controls(&state));
    let drawn = scene(&state);
    let row = drawn
        .controls
        .iter()
        .find(|control| control.target == last)
        .expect("the last coin is listed");
    assert!(
        row.rect.y >= 0. && row.rect.y + row.rect.h <= 1080.,
        "{:?}",
        row.rect
    );
}

/// A tint multiplies what the icon already holds, so an icon compiled black stays black.
#[test]
fn every_interface_icon_is_white_underneath_its_tint() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/ui-icons");
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "svg") {
            let svg = std::fs::read_to_string(&path).unwrap();
            assert!(
                !svg.contains("currentColor") || svg.contains(r##"color="#fff""##),
                "{} would draw black whatever its tint",
                path.display()
            );
        }
    }
}

/// Marks in their colours get lost on a light wallpaper's glass: each coin has a dark plate
/// reaching from its mark to the end of its change, under all of them.
#[test]
fn each_coin_on_the_ticker_stands_on_a_dark_plate_around_all_it_says() {
    use crate::renderer::Draw;
    let drawn = scene(&priced(&["BTC", "ETH"]));
    let dark: Vec<_> = drawn
        .draw
        .iter()
        .enumerate()
        .filter_map(|(at, draw)| match draw {
            Draw::Shape(rect, _, _, color) if color[..3] == [0.; 3] && color[3] >= 0.25 => {
                Some((at, *rect))
            }
            _ => None,
        })
        .collect();
    for (mark, symbol) in [("coin-btc", "BTC"), ("coin-eth", "ETH")] {
        let icon = icons_at(&drawn, mark)[0];
        let name = text_at(&drawn, symbol).unwrap();
        let pill = dark
            .iter()
            .map(|(_, rect)| *rect)
            .filter(|rect| rect.x > name.x)
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        let plate = dark.iter().find(|(_, rect)| {
            rect.h < 56.
                && rect.x < icon.x
                && rect.x + rect.w >= pill.x + pill.w
                && rect.y <= icon.y.min(pill.y)
                && rect.y + rect.h >= (icon.y + icon.h).max(pill.y + pill.h)
        });
        let Some((at, _)) = plate else {
            panic!("{symbol} has no plate around it");
        };
        let mark_at = drawn
            .draw
            .iter()
            .position(|draw| matches!(draw, Draw::TintedImage(_, rect, ..) if rect.x == icon.x && rect.y == icon.y))
            .unwrap();
        assert!(*at < mark_at, "{symbol}'s plate is under its mark");
    }
}
