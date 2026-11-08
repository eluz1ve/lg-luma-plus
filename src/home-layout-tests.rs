use super::*;
use crate::{
    layout::column,
    renderer::{Draw, TextStyle},
    state::Rect,
};

fn favorites(state: &mut State, count: usize) {
    let ids: Vec<_> = state
        .inventory
        .apps
        .iter()
        .take(count)
        .map(|app| app.id.clone())
        .collect();
    assert_eq!(ids.len(), count, "the demo has enough apps");
    state.preferences.favorites = Some(ids);
}

fn texts(state: &State) -> Vec<(String, Rect, TextStyle)> {
    scene(state)
        .draw
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text(text, rect, style, ..) => Some((text, rect, style)),
            _ => None,
        })
        .collect()
}

/// The greeting at the scenes' time, midnight UTC.
fn greeting(state: &State) -> (String, Rect, TextStyle) {
    let expected = crate::i18n::Language::En.greeting(0, &state.preferences.greeting_name);
    texts(state)
        .into_iter()
        .find(|(text, ..)| *text == expected)
        .expect("the greeting is drawn")
}

/// The width tests measure text at: the renderer's fallback advances, without font metrics.
fn measured(text: &str, style: TextStyle) -> f32 {
    text.chars()
        .map(|c| (if c == ' ' { 0.28 } else { 0.53 }) + style.tracking)
        .sum::<f32>()
        * style.size
}

#[test]
fn layout_columns_follow_the_tvos_grid() {
    assert_eq!(column(6), 260.);
    assert_eq!(column(5), 320.);
    assert_eq!(column(4), 410.);
    assert_eq!(column(3), 560.);
    assert_eq!(column(2), 860.);
}

#[test]
fn home_fits_one_screen_with_weather_six_favorites_and_four_coins() {
    let mut state = ready();
    favorites(&mut state, 6);
    state.preferences.show_weather = true;
    state.preferences.show_crypto = true;
    state.preferences.crypto_coins = ["BTC", "ETH", "SOL", "BNB"].map(String::from).to_vec();
    let controls = controls(&state);
    let tiles: Vec<_> = controls
        .iter()
        .filter(|c| matches!(c.target, Target::App(_)))
        .map(|c| c.rect)
        .collect();
    assert_eq!(tiles.len(), 6);
    assert_eq!(
        (tiles[0].x, tiles[0].y, tiles[0].w, tiles[0].h),
        (80., 830., 260., 150.)
    );
    assert_eq!(tiles[1].x, 380.);
    assert_eq!(tiles[5].x + tiles[5].w, 1840.);
    // The inputs are in the menu; the coins stand in the center.
    assert!(
        !controls
            .iter()
            .any(|c| matches!(c.target, Target::Input(_)))
    );
    let coins: Vec<_> = controls
        .iter()
        .filter(|c| matches!(c.target, Target::Coin(_)))
        .map(|c| c.rect)
        .collect();
    assert_eq!(coins.len(), 4);
    assert!(
        coins
            .iter()
            .all(|rect| rect.y > 400. && rect.y + rect.h < 780.)
    );
    let bottom = controls
        .iter()
        .map(|c| c.rect.y + c.rect.h)
        .fold(0., f32::max);
    assert!(bottom <= 1020., "the last row ends at {bottom}");
    // A lifted first tile grows 7% past x = 80 and is not clipped.
    assert!(state.favorites_viewport().x <= 80. - 260. * 0.035);
    let (_, _, style) = greeting(&state);
    assert_eq!(style.size, 76.);
    assert_eq!(style.weight, 600);
}

#[test]
fn focused_favorite_alone_shows_its_name() {
    let mut state = ready();
    favorites(&mut state, 6);
    state.preferences.show_home_names = true;
    let first = state.inventory.apps[0].title.clone();
    let second = state.inventory.apps[1].title.clone();
    state.focus = Target::App(state.inventory.apps[1].id.clone());
    let drawn = texts(&state);
    assert!(drawn.iter().any(|(text, ..)| *text == second));
    assert!(!drawn.iter().any(|(text, ..)| *text == first));
    let (_, name, _) = drawn.iter().find(|(text, ..)| *text == second).unwrap();
    assert!(
        name.y >= 830. + 150. && name.y + name.h <= 1080.,
        "{name:?}"
    );
    let label = state.preferences.language.t("favorites");
    let (_, heading, _) = drawn
        .iter()
        .find(|(text, ..)| text == label)
        .expect("the favorites' heading");
    // The name's room is kept whichever card is focused, so rows never jump.
    state.focus = Target::Nav(0);
    let unfocused = texts(&state);
    assert!(
        !unfocused
            .iter()
            .any(|(text, ..)| *text == first || *text == second)
    );
    assert_eq!(
        unfocused
            .iter()
            .find(|(text, ..)| text == label)
            .map(|t| t.1.y),
        Some(heading.y)
    );
}

#[test]
fn long_greeting_shrinks_to_the_left_column_beside_today() {
    let mut state = ready();
    state.preferences.greeting_name = "Maximilian-Alexander von Hohenzollern".into();
    state.preferences.show_crypto = true;
    state.preferences.crypto_beside = true;
    let (text, rect, style) = greeting(&state);
    assert_eq!(rect.x, 80.);
    assert!(
        measured(&text, style) <= 1040.,
        "{}",
        measured(&text, style)
    );
    // Without the card beside it the greeting keeps its full size.
    state.preferences.crypto_beside = false;
    state.preferences.greeting_name = "Ada".into();
    let (_, _, style) = greeting(&state);
    assert_eq!(style.size, 76.);
}

#[test]
fn cs2_and_shows_in_today_leave_the_center_empty() {
    let mut state = ready();
    state.preferences.show_cs2 = true;
    state.preferences.show_shows = true;
    let headings = |state: &State| -> Vec<String> {
        texts(state)
            .into_iter()
            .map(|(text, ..)| text)
            .filter(|text| text == "CS2 Scores" || text == "Watching now")
            .collect()
    };
    assert!(headings(&state).is_empty(), "{:?}", headings(&state));
    // Saved from the days of rows, both outside Today: CS2 takes the center, Shows stays in Today.
    state.preferences.cs2_today = false;
    state.preferences.shows_today = false;
    assert_eq!(headings(&state), ["CS2 Scores"]);
}

#[test]
fn the_date_shares_the_clock_line_and_the_greeting_rises() {
    let state = ready();
    let language = state.preferences.language;
    let now = time::OffsetDateTime::UNIX_EPOCH;
    let drawn = texts(&state);
    let clock = language.clock(
        now,
        state.preferences.hour_24,
        state.preferences.show_seconds,
    );
    let (_, clock, _) = drawn.iter().find(|(text, ..)| *text == clock).unwrap();
    let (_, date, _) = drawn
        .iter()
        .find(|(text, ..)| text.to_lowercase() == language.date(now).to_lowercase())
        .expect("the date is drawn");
    let middle = |rect: &Rect| rect.y + rect.h / 2.;
    assert!(
        (middle(date) - middle(clock)).abs() < 1.,
        "{date:?} {clock:?}"
    );
    assert!(date.x > clock.x + 40.);
    let (_, greeting, _) = greeting(&state);
    assert!(greeting.y < 150., "{greeting:?}");
}

#[test]
fn today_grows_up_to_three_lines_from_the_clock_line() {
    use crate::shows::Show;
    let mut state = ready();
    state.preferences.show_shows = true;
    // New on the scene's day, the Unix epoch's.
    let epoch = time::OffsetDateTime::UNIX_EPOCH.date();
    state.shows.list = (0..7)
        .map(|id| Show::sample(id, &format!("Show {id}"), epoch))
        .collect();
    let drawn = scene(&state);
    // The card, and the clip it is drawn under: nothing of it is cut off.
    let mut clip = None;
    let (card, clip) = drawn
        .draw
        .iter()
        .find_map(|draw| match draw {
            Draw::Clip(rect) => {
                clip = *rect;
                None
            }
            Draw::Glass(_, rect, ..) if rect.x == view::TODAY_RECT.x => Some((*rect, clip)),
            _ => None,
        })
        .expect("the Today card");
    assert!(
        clip.is_none_or(|clip| clip.y <= card.y),
        "{clip:?} cuts {card:?}"
    );
    let lines = texts(&state)
        .iter()
        .filter(|(text, rect, _)| text.starts_with("Show ") && rect.x >= view::TODAY_RECT.x)
        .count();
    assert_eq!(lines, 3);
    assert!(
        card.y < 60. && card.y + card.h <= crate::layout::HEADER_BOTTOM,
        "{card:?}"
    );
    assert!(
        texts(&state)
            .iter()
            .any(|(text, ..)| text.ends_with("4 MORE")),
        "the two left out are counted"
    );
}
