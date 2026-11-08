use super::*;
use crate::model::Center;

fn rects(state: &State, of: fn(&Target) -> bool) -> Vec<crate::state::Rect> {
    controls(state)
        .into_iter()
        .filter(|control| of(&control.target))
        .map(|control| control.rect)
        .collect()
}

/// TV shows in the center, the row of posters Lampa fills.
fn with_shows() -> State {
    use crate::shows::Show;
    let mut state = ready();
    state.preferences.show_shows = true;
    state.preferences.shows_today = false;
    state.shows.list = (0..8)
        .map(|id| Show::sample(id, &format!("Show {id}"), time::Date::MIN))
        .collect();
    state
}

#[test]
fn one_widget_takes_the_center_cs2_then_shows() {
    let mut preferences = Preferences::default();
    assert_eq!(preferences.center(), None);
    // Crypto never stands there, whatever an older save says.
    preferences.show_crypto = true;
    assert_eq!(preferences.center(), None);
    assert!(preferences.crypto_in_today());
    // Saved from the days of rows: CS2 and shows both outside Today. CS2 comes first, and the
    // others show in Today.
    preferences.show_cs2 = true;
    preferences.cs2_today = false;
    preferences.show_shows = true;
    preferences.shows_today = false;
    assert_eq!(preferences.center(), Some(Center::Cs2));
    assert!(preferences.in_today(Center::Shows));
    assert!(!preferences.in_today(Center::Cs2));
    preferences.show_cs2 = false;
    assert_eq!(preferences.center(), Some(Center::Shows));
}

#[test]
fn a_save_from_when_crypto_stood_in_the_center_still_loads() {
    let saved = r#"{"show_crypto":true,"crypto_beside":false,"crypto_coins":["BTC"]}"#;
    let preferences: Preferences = serde_json::from_str(saved).unwrap();
    assert!(preferences.crypto_in_today());
    assert_eq!(preferences.center(), None);
}

#[test]
fn placing_a_widget_in_the_center_moves_the_one_there_into_today() {
    let mut state = with_shows();
    state.preferences.show_cs2 = true;
    assert_eq!(state.preferences.center(), Some(Center::Shows));
    state.open(2);
    state.focus = Target::Cs2Layout;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.cs2_today);
    assert!(state.preferences.shows_today, "the shows moved into Today");
    assert_eq!(state.preferences.center(), Some(Center::Cs2));
    // And back: CS2 into Today leaves the center empty.
    state.activate();
    assert!(state.preferences.cs2_today);
    assert_eq!(state.preferences.center(), None);
}

#[test]
fn favorites_sit_at_the_bottom_and_home_has_no_inputs() {
    let state = ready();
    let cards = rects(&state, |t| matches!(t, Target::App(_)));
    assert!(!cards.is_empty());
    assert!(cards.iter().all(|card| card.y == 830. && card.h == 150.));
    assert!(rects(&state, |t| matches!(t, Target::Input(_))).is_empty());
    let heading = state.preferences.language.t("favorites");
    assert!(scene(&state).draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Text(text, rect, ..) if text == heading && rect.y == 780.)));
}

#[test]
fn the_center_widget_sits_between_the_header_and_the_favorites() {
    let state = with_shows();
    let posters = rects(&state, |t| matches!(t, Target::Show(_)));
    assert!(!posters.is_empty());
    let top = posters[0].y - 50.;
    let bottom = posters[0].y + posters[0].h;
    assert!(top >= 300. && bottom <= 780.);
    assert!(
        ((top - 300.) - (780. - bottom)).abs() < 1.,
        "centered: {top}..{bottom}"
    );
    // Nothing in the center without a widget there.
    assert!(rects(&ready(), |t| matches!(t, Target::Show(_))).is_empty());
}

#[test]
fn up_from_the_favorites_reaches_the_center_then_today_and_down_returns() {
    let mut state = with_shows();
    state.focus = Target::App("netflix".into());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Show(0));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::App("netflix".into()));
    // Down from the favorites goes nowhere: nothing lies below.
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn left_from_the_first_card_in_the_center_opens_the_menu() {
    let mut state = with_shows();
    state.focus = Target::Show(1);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Show(0));
    state.direction(Direction::Left, &controls(&state));
    assert!(matches!(state.overlay, Some(Overlay::Menu { .. })));
}

#[test]
fn shows_in_the_center_take_the_room_today_leaves() {
    use crate::shows::Show;
    let mut state = ready();
    state.preferences.show_shows = true;
    state.preferences.shows_today = false;
    state.shows.list = (0..8)
        .map(|id| Show::sample(id, &format!("Show {id}"), time::Date::MIN))
        .collect();
    let posters = rects(&state, |t| matches!(t, Target::Show(_)));
    assert!(!posters.is_empty());
    assert!(posters.iter().all(|p| p.w == 200.), "{posters:?}");
    assert!(posters.windows(2).all(|pair| pair[1].x - pair[0].x == 240.));
    let bottom = posters.iter().map(|p| p.y + p.h).fold(0., f32::max);
    assert!(bottom <= 780., "{bottom}");
    state.focus = Target::Show(0);
    state.direction(Direction::Left, &controls(&state));
    assert!(matches!(state.overlay, Some(Overlay::Menu { .. })));
}
