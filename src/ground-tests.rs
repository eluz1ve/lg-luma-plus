use super::*;
use crate::state::Rect;

/// Frames 60 a second from `start`.
fn frames(start: Instant) -> impl Iterator<Item = Instant> {
    (0u64..).map(move |frame| start + Duration::from_micros(frame * 16_667))
}

fn page(x: f32) -> Vec<Draw> {
    vec![
        Draw::Image(
            "wallpapers/dunes.jpg".into(),
            Rect::new(0., 0., 1920., 1080.),
            0.,
            false,
        ),
        Draw::Shape(Rect::new(x, 300., 100., 100.), 8., 0., [1., 1., 1., 1.]),
    ]
}

#[test]
fn a_page_still_under_a_long_animation_is_drawn_into_its_picture_once() {
    let mut ground = Ground::default();
    let mut frames = frames(Instant::now());
    let start = frames.next().unwrap();
    assert_eq!(ground.plan(&page(100.), start), Plan::Draw);
    for now in frames.by_ref() {
        if now.duration_since(start) >= SETTLED {
            assert_eq!(ground.plan(&page(100.), now), Plan::Fill);
            break;
        }
        assert_eq!(ground.plan(&page(100.), now), Plan::Draw);
    }
    ground.filled(&page(100.));
    assert_eq!(ground.plan(&page(100.), frames.next().unwrap()), Plan::Lay);
    assert_eq!(ground.plan(&page(100.), frames.next().unwrap()), Plan::Lay);
}

#[test]
fn an_overlay_opening_over_a_page_not_kept_draws_it_as_before() {
    // Drawing the picture costs a frame about twice its page; mid-transition that is a stutter.
    let mut ground = Ground::default();
    // The menu and Today open in 0.25 s, 15 frames.
    for now in frames(Instant::now()).take(15) {
        assert_eq!(ground.plan(&page(100.), now), Plan::Draw);
    }
}

#[test]
fn a_page_that_keeps_changing_is_never_drawn_twice() {
    let mut ground = Ground::default();
    for (step, now) in frames(Instant::now()).take(120).enumerate() {
        assert_eq!(ground.plan(&page(step as f32), now), Plan::Draw);
    }
}

#[test]
fn a_changed_page_is_drawn_until_it_is_kept_again() {
    let mut ground = Ground::default();
    let now = Instant::now();
    ground.filled(&page(100.));
    assert_eq!(ground.plan(&page(100.), now), Plan::Lay);
    assert_eq!(ground.plan(&page(140.), now), Plan::Draw);
    ground.filled(&page(140.));
    assert_eq!(ground.plan(&page(140.), now), Plan::Lay);
}

#[test]
fn a_forgotten_picture_is_drawn_from_the_page_again() {
    let mut ground = Ground::default();
    ground.filled(&page(100.));
    ground.forget();
    assert!(!ground.holds(&page(100.)));
    assert_eq!(ground.plan(&page(100.), Instant::now()), Plan::Draw);
}

#[test]
fn a_page_kept_while_still_is_laid_on_the_overlay_first_frame() {
    let mut ground = Ground::default();
    assert!(!ground.holds(&page(100.)));
    ground.filled(&page(100.));
    assert!(ground.holds(&page(100.)));
    assert_eq!(ground.plan(&page(100.), Instant::now()), Plan::Lay);
}
