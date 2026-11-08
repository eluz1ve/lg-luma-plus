use super::*;
use crate::{
    layout::{PREVIEW_HEIGHT, PREVIEW_WIDTH, PREVIEW_X, PREVIEW_Y},
    renderer::Draw,
    state::{Page, Rect},
    view::Region,
};

const SCREEN: (f32, f32, f32, f32) = (0., 0., 1920., 1080.);

fn xywh(rect: &Rect) -> (f32, f32, f32, f32) {
    (rect.x, rect.y, rect.w, rect.h)
}

/// Every glass draw: its wallpaper, panel, source and tint.
fn glass(scene: &view::Scene) -> Vec<(String, Rect, Rect, [f32; 4])> {
    scene
        .draw
        .iter()
        .filter_map(|draw| match draw {
            Draw::Glass(path, rect, _, source, tint) => Some((path.clone(), *rect, *source, *tint)),
            _ => None,
        })
        .collect()
}

fn wallpaper(scene: &view::Scene) -> String {
    match &scene.draw[0] {
        Draw::Image(path, ..) => path.clone(),
        _ => panic!("the wallpaper comes first"),
    }
}

/// Asserts a glass draw lies exactly under `rect`, blurring the screen's wallpaper as drawn.
fn on_glass(scene: &view::Scene, rect: Rect, what: &str) {
    let path = wallpaper(scene);
    assert!(
        glass(scene).iter().any(|(p, r, source, _)| *p == path
            && xywh(r) == xywh(&rect)
            && xywh(source) == SCREEN),
        "{what} at {:?} is on glass",
        xywh(&rect)
    );
}

fn rects(scene: &view::Scene, wanted: impl Fn(&Target) -> bool) -> Vec<Rect> {
    let found: Vec<_> = scene
        .controls
        .iter()
        .filter(|control| wanted(&control.target))
        .map(|control| control.rect)
        .collect();
    assert!(!found.is_empty());
    found
}

fn home() -> State {
    let mut state = ready();
    state.preferences.show_crypto = true;
    state
}

#[test]
fn chrome_sits_on_glass_over_the_wallpaper() {
    let state = home();
    let drawn = scene(&state);
    let mut menu = home();
    menu.open_menu();
    menu.menu_motion = None;
    on_glass(&scene(&menu), view::MENU_PANEL, "the menu");
    // The coins are a line of «Сегодня».
    let card = drawn
        .regions
        .iter()
        .find(|(region, _)| *region == Region::Crypto)
        .map(|(_, rect)| *rect)
        .expect("Today shows the coins");
    on_glass(&drawn, card, "Today");
}

#[test]
fn settings_chrome_sits_on_glass() {
    let mut state = ready();
    state.page = Page::Settings;
    // On the tab bar: no row is focused (white) and the sidebar is folded.
    state.focus = Target::Nav(2);
    let drawn = scene(&state);
    on_glass(
        &drawn,
        Rect::new(80., 150., crate::layout::SIDEBAR_COLLAPSED, 8. + 4. * 68.),
        "the sidebar",
    );
    for rect in rects(&drawn, |t| matches!(t, Target::Setting(_))) {
        on_glass(&drawn, rect, "a row");
    }
    state.section = crate::state::BACKGROUND_SECTION;
    state.focus = Target::Background(0);
    let drawn = scene(&state);
    for rect in rects(&drawn, |t| {
        matches!(t, Target::WallpaperCycle | Target::WallpaperGradient)
    }) {
        on_glass(&drawn, rect, "a gallery switch");
    }
}

#[test]
fn overlays_sit_on_glass() {
    let mut state = ready();
    let id = state.inventory.apps[0].id.clone();
    state.preferences.favorites = Some(vec![id.clone()]);
    state.focus = Target::App(id);
    state.options();
    let drawn = scene(&state);
    let panel = drawn.modal.expect("options are a modal");
    on_glass(&drawn, panel, "the options panel");
}

#[test]
fn glass_cross_fades_with_the_wallpaper() {
    let mut state = home();
    state.open_menu();
    state.menu_motion = None;
    state.preferences.background = "forest".into();
    state.previous_background = Some((
        "dunes".into(),
        std::time::Instant::now() - std::time::Duration::from_secs_f32(state.background_fade / 2.),
    ));
    let drawn = scene(&state);
    let previous = wallpaper(&drawn);
    let (current, alpha) = drawn
        .draw
        .iter()
        .find_map(|draw| match draw {
            Draw::TintedImage(path, rect, _, _, tint) if xywh(rect) == SCREEN => {
                Some((path.clone(), tint[3]))
            }
            _ => None,
        })
        .expect("the new wallpaper fades in over the old");
    assert!(alpha > 0. && alpha < 1.);
    let bar = view::MENU_PANEL;
    let layers: Vec<_> = glass(&drawn)
        .into_iter()
        .filter(|(_, rect, ..)| xywh(rect) == xywh(&bar))
        .map(|(path, _, _, tint)| (path, tint[3]))
        .collect();
    assert_eq!(layers.len(), 2, "{layers:?}");
    assert_eq!(layers[0], (previous.clone(), 1.));
    assert_eq!(layers[1].0, current);
    assert!((layers[1].1 - alpha).abs() < 0.05, "{layers:?} vs {alpha}");
}

#[test]
fn text_on_glass_has_no_shadow() {
    let mut state = home();
    // The menu's labels, and its inputs unfolded.
    state.open_menu();
    state.menu_motion = None;
    state.menu_inputs = true;
    let drawn = scene(&state);
    let labels: Vec<String> = ["home", "apps", "settings"]
        .map(|key| state.preferences.language.t(key).to_owned())
        .into_iter()
        .chain(
            state
                .inventory
                .inputs
                .iter()
                .map(|input| state.preferences.input_title(input).to_owned()),
        )
        .collect();
    let shadowed = |drawn: &view::Scene, label: &str| {
        drawn.draw.iter().any(
            |draw| matches!(draw, Draw::Text(text, _, style, ..) if text == label && style.shadow),
        )
    };
    for label in &labels {
        assert!(!shadowed(&drawn, label), "{label}");
    }
    state.page = Page::Settings;
    state.focus = Target::Setting(1);
    let drawn = scene(&state);
    let title = state.preferences.language.t("appNames");
    assert!(!shadowed(&drawn, title));
}

#[test]
fn the_preview_samples_its_own_wallpaper() {
    // Coins in the center, on glass.
    let mut state = home();
    state.page = Page::Settings;
    state.focus = Target::Setting(0);
    let mut drawn = scene(&state);
    view::home_preview(
        &mut drawn,
        &mut state,
        &view::Assets {
            root: PathBuf::new(),
            version: "test".into(),
            metrics: BTreeMap::new(),
            artwork: crate::app_artwork::Service::new(false),
        },
        time::OffsetDateTime::UNIX_EPOCH,
    );
    let frame = Rect::new(PREVIEW_X, PREVIEW_Y, PREVIEW_WIDTH, PREVIEW_HEIGHT);
    // Home's wallpaper, as scaled into the frame.
    let wallpaper = Rect::new(
        PREVIEW_X,
        PREVIEW_Y,
        PREVIEW_WIDTH,
        1080. * PREVIEW_WIDTH / 1920.,
    );
    let inside: Vec<_> = glass(&drawn)
        .into_iter()
        .filter(|(_, rect, ..)| {
            rect.x >= frame.x
                && rect.y >= frame.y
                && rect.x + rect.w <= frame.x + frame.w
                && rect.y + rect.h <= frame.y + frame.h
        })
        .collect();
    assert!(
        !inside.is_empty(),
        "Home's chrome in the frame is glass too"
    );
    for (_, rect, source, _) in inside {
        assert_eq!(xywh(&source), xywh(&wallpaper), "{:?}", xywh(&rect));
    }
}

#[test]
fn glass_blurs_the_wallpaper_as_drawn_with_or_without_its_gradient() {
    let mut state = home();
    state.preferences.background = "forest".into();
    for (gradient, file) in [(true, "forest.jpg"), (false, "forest-plain.jpg")] {
        state.preferences.wallpaper_gradient = gradient;
        let drawn = scene(&state);
        let paths: Vec<_> = glass(&drawn).into_iter().map(|(path, ..)| path).collect();
        assert!(!paths.is_empty());
        assert!(
            paths
                .iter()
                .all(|path| path.ends_with(&format!("wallpapers/{file}"))),
            "{paths:?}"
        );
    }
}
