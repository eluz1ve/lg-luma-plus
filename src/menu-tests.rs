use super::*;
use std::time::{Duration, Instant};

fn menu_open(state: &State) -> bool {
    matches!(state.overlay, Some(Overlay::Menu { .. }))
}

fn hdmi(n: u8) -> Target {
    Target::Input(format!("com.webos.app.hdmi{n}"))
}

#[test]
fn home_opens_the_menu_on_the_current_page_and_home_again_closes_it() {
    let mut state = ready();
    state.open(1);
    let focus = state.focus.clone();
    assert!(
        !matches!(focus, Target::Nav(_)),
        "a page opens at its content"
    );
    assert!(state.home_key().is_empty());
    assert!(menu_open(&state));
    assert_eq!(state.page, Page::Apps);
    assert_eq!(state.focus, Target::Nav(1));
    assert!(state.home_key().is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, focus);
}

#[test]
fn home_over_an_open_overlay_closes_it_and_opens_the_menu() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.options();
    assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
    state.home_key();
    assert!(menu_open(&state));
    state.back();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn the_menu_lists_pages_then_visible_inputs_when_unfolded_then_settings() {
    let mut state = ready();
    state.open_menu();
    assert_eq!(
        state.menu_items(),
        [
            Target::Nav(0),
            Target::Nav(1),
            Target::MenuInputs,
            Target::Nav(2),
            Target::TvSettings
        ]
    );
    let shown = controls(&state);
    state.direction(Direction::Down, &shown);
    state.direction(Direction::Down, &shown);
    assert_eq!(state.focus, Target::MenuInputs);
    assert!(state.activate().is_empty());
    assert!(menu_open(&state), "unfolding keeps the menu open");
    // HDMI 3 reports no connection, so Auto hides it as Home does.
    assert_eq!(
        state.menu_items(),
        [
            Target::Nav(0),
            Target::Nav(1),
            Target::MenuInputs,
            hdmi(1),
            hdmi(2),
            Target::Nav(2),
            Target::TvSettings
        ]
    );
    let shown = controls(&state);
    for _ in 0..10 {
        state.direction(Direction::Down, &shown);
    }
    assert_eq!(state.focus, Target::TvSettings, "Down stops at the end");
    for _ in 0..10 {
        state.direction(Direction::Up, &shown);
    }
    assert_eq!(state.focus, Target::Nav(0), "Up stops at the top");
    state.direction(Direction::Left, &shown);
    assert!(menu_open(&state), "Left inside the menu goes nowhere");
}

#[test]
fn ok_on_another_page_opens_it_at_its_first_content() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.open_menu();
    state.focus = Target::Nav(2);
    assert!(state.activate().is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.page, Page::Settings);
    assert_eq!(state.focus, Target::Sidebar(0));
    // The current page's item only closes the menu.
    state.focus = Target::Sidebar(1);
    state.open_menu();
    assert_eq!(state.focus, Target::Nav(2));
    state.activate();
    assert!(state.overlay.is_none());
    assert_eq!(state.page, Page::Settings);
    assert_eq!(state.focus, Target::Sidebar(1));
}

#[test]
fn ok_on_an_input_switches_to_it_and_closes_the_menu() {
    let mut state = ready();
    state.open_menu();
    state.menu_inputs = true;
    state.focus = hdmi(2);
    assert_eq!(
        state.activate(),
        [Effect::Launch("com.webos.app.hdmi2".into())]
    );
    assert!(state.overlay.is_none());
    state.open_menu();
    state.focus = Target::TvSettings;
    assert_eq!(
        state.activate(),
        [Effect::Launch("com.palm.app.settings".into())]
    );
    assert!(state.overlay.is_none());
}

#[test]
fn left_from_the_first_favorite_opens_the_menu() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.direction(Direction::Left, &controls(&state));
    assert!(menu_open(&state));
    assert_eq!(state.focus, Target::Nav(0));
    state.back();
    assert_eq!(state.focus, Target::App("netflix".into()));
    // Further along the row, Left walks the favorites.
    let second = state.apps()[1].id.clone();
    state.focus = Target::App(second);
    state.direction(Direction::Left, &controls(&state));
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn left_from_the_settings_sidebar_opens_the_menu() {
    let mut state = ready();
    state.open(2);
    assert_eq!(state.focus, Target::Sidebar(0));
    state.direction(Direction::Left, &controls(&state));
    assert!(menu_open(&state));
    assert_eq!(state.focus, Target::Nav(2));
}

#[test]
fn left_from_the_first_column_of_apps_opens_the_menu() {
    let mut state = ready();
    state.open(1);
    state.direction(Direction::Left, &controls(&state));
    assert!(menu_open(&state));
    assert_eq!(state.focus, Target::Nav(1));
}

#[test]
fn back_and_right_close_the_menu_and_restore_the_focus() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.open_menu();
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Nav(1));
    assert!(state.back().is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("netflix".into()));
    state.open_menu();
    state.direction(Direction::Right, &controls(&state));
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn a_relaunch_returns_home_without_the_menu() {
    let mut state = ready();
    state.open(2);
    assert!(state.relaunch_home().is_empty());
    assert_eq!(state.page, Page::Home);
    assert!(!menu_open(&state));
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn up_from_the_favorites_without_today_stays() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::App("netflix".into()));
    assert!(state.overlay.is_none());
}

#[test]
fn back_from_the_top_of_a_page_goes_home() {
    let mut state = ready();
    state.open(2);
    assert_eq!(state.focus, Target::Sidebar(0));
    state.back();
    assert_eq!(state.page, Page::Home);
    assert!(state.overlay.is_none());
}

#[test]
fn the_menu_slides_unless_motion_is_reduced() {
    let mut state = ready();
    state.preferences.reduced_motion = false;
    assert_eq!(state.menu_amount(), 0.);
    state.open_menu();
    state.menu_motion = Some((Instant::now() - Duration::from_millis(125), true));
    let half = state.menu_amount();
    assert!(
        half > 0.5 && half < 1.,
        "ease-out is past half at half time: {half}"
    );
    state.menu_motion = Some((Instant::now() - Duration::from_millis(300), true));
    state.tick();
    assert_eq!(state.menu_amount(), 1.);
    assert!(state.menu_motion.is_none());
    state.close_menu();
    assert!(state.menu_amount() > 0.9, "closing starts from open");
    state.menu_motion = Some((Instant::now() - Duration::from_millis(250), false));
    state.tick();
    assert_eq!(state.menu_amount(), 0.);
    state.preferences.reduced_motion = true;
    state.open_menu();
    assert_eq!(state.menu_amount(), 1.);
    state.close_menu();
    assert_eq!(state.menu_amount(), 0.);
}

fn texts(scene: &view::Scene) -> Vec<String> {
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
fn no_tab_bar_is_drawn_and_the_clock_stays() {
    let state = ready();
    let drawn = scene(&state);
    assert!(!drawn.controls.iter().any(|control| matches!(
        control.target,
        Target::Nav(_) | Target::TvSettings | Target::MenuInputs
    )));
    let language = state.preferences.language;
    for key in ["apps", "settings", "tvSettings"] {
        assert!(
            !texts(&drawn).contains(&language.t(key).to_owned()),
            "{key}"
        );
    }
    // The clock still starts at the safe area.
    assert!(drawn.draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Text(text, rect, ..) if text.contains(':') && rect.x == 80. && rect.y < 100.)));
}

#[test]
fn the_menu_draws_its_items_on_glass_over_the_dimmed_page() {
    let mut state = ready();
    state.open_menu();
    state.menu_motion = None;
    state.menu_inputs = true;
    let drawn = scene(&state);
    let targets: Vec<_> = drawn.controls.iter().map(|c| c.target.clone()).collect();
    assert_eq!(targets, state.menu_items(), "only the menu takes focus");
    let modal = drawn.modal.expect("the menu is modal");
    let p = view::MENU_PANEL;
    assert_eq!((modal.x, modal.y, modal.w, modal.h), (p.x, p.y, p.w, p.h));
    let language = state.preferences.language;
    let shown = texts(&drawn);
    for key in ["home", "apps", "inputs", "settings", "tvSettings"] {
        assert!(shown.contains(&language.t(key).to_owned()), "{key}");
    }
    assert!(shown.contains(&"HDMI 1".to_owned()));
    // Every item lies inside the panel, the bottom two at its foot.
    for control in &drawn.controls {
        let r = control.rect;
        let p = view::MENU_PANEL;
        assert!(r.x >= p.x && r.x + r.w <= p.x + p.w && r.y >= p.y && r.y + r.h <= p.y + p.h);
    }
    let settings = drawn
        .controls
        .iter()
        .find(|c| c.target == Target::Nav(2))
        .unwrap();
    let inputs = drawn
        .controls
        .iter()
        .filter(|c| matches!(c.target, Target::Input(_)));
    assert!(
        inputs
            .into_iter()
            .all(|c| c.rect.y + c.rect.h < settings.rect.y)
    );
    // The page behind stays put: only the panel's strip is drawn again as it slides.
    let closed = scene(&ready());
    let greeting = |scene: &view::Scene| {
        scene.draw.iter().find_map(|draw| match draw {
            crate::renderer::Draw::Text(text, rect, ..) if text.contains(':') => Some(rect.x),
            _ => None,
        })
    };
    assert_eq!(greeting(&drawn).unwrap(), greeting(&closed).unwrap());
}

#[test]
fn menu_items_are_clickable() {
    let mut state = ready();
    state.open_menu();
    state.menu_motion = None;
    let drawn = scene(&state);
    let apps = drawn
        .controls
        .iter()
        .find(|c| c.target == Target::Nav(1))
        .unwrap();
    let (x, y) = apps.rect.center();
    let target = state.hit_test(&drawn.controls, x, y).unwrap();
    state.pointer_focus(target);
    state.activate();
    assert_eq!(state.page, Page::Apps);
    assert!(state.overlay.is_none());
}

#[test]
fn a_closing_menu_slides_out_and_leaves_the_page_its_controls() {
    let mut state = ready();
    state.preferences.reduced_motion = false;
    state.focus = Target::App("netflix".into());
    state.open_menu();
    state.menu_motion = None;
    state.close_menu();
    let drawn = scene(&state);
    assert!(drawn.modal.is_none());
    assert!(
        drawn
            .controls
            .iter()
            .any(|c| c.target == Target::App("netflix".into()))
    );
    assert!(!drawn.controls.iter().any(|c| c.target == Target::Nav(0)));
}

#[test]
fn while_the_menu_slides_only_its_strip_is_drawn_again() {
    for page in [0, 1, 2] {
        let mut state = ready();
        state.preferences.reduced_motion = false;
        state.open(page);
        if page == 0 {
            state.focus = Target::App("netflix".into());
        }
        for _ in 0..120 {
            state.animate(1. / 60.);
        }
        state.open_menu();
        let mut tracker = crate::damage::Tracker::default();
        for ms in [60, 90, 120] {
            state.menu_motion = Some((Instant::now() - Duration::from_millis(ms), true));
            state.animate(1. / 60.);
            tracker.push(&scene(&state).draw);
        }
        let area = tracker.damage(1).expect("a partial redraw");
        let p = view::MENU_PANEL;
        assert!(
            area.x + area.w <= p.x + p.w + 80.,
            "page {page}: {area:?} reaches past the panel"
        );
    }
}

#[test]
fn a_perf_script_opens_the_side_menu_and_skips_a_missing_today_card() {
    let mut state = State::new(crate::model::Preferences::default());
    state.set_inventory(crate::model::demo_inventory(), None);
    let controls = crate::view::scene(
        &state,
        &crate::view::Assets {
            root: std::path::PathBuf::new(),
            version: "test".into(),
            metrics: Default::default(),
            artwork: crate::app_artwork::Service::new(false),
        },
        time::OffsetDateTime::UNIX_EPOCH,
    )
    .controls;
    if !controls
        .iter()
        .any(|control| control.target == Target::TodayCard)
    {
        let focus = state.focus.clone();
        assert_eq!(crate::script("today", &mut state, &controls), Ok(vec![]));
        assert!(state.overlay.is_none());
        assert_eq!(state.focus, focus);
    }
    assert_eq!(
        crate::script("side-menu", &mut state, &controls),
        Ok(vec![])
    );
    assert!(menu_open(&state));
}
