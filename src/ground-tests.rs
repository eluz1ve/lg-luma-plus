use super::*;
use crate::state::Rect;

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
fn a_page_is_kept_once_it_stays_the_same_for_a_frame() {
    let mut ground = Ground::default();
    assert_eq!(ground.plan(&page(100.)), Plan::Draw);
    assert_eq!(ground.plan(&page(100.)), Plan::Fill);
    ground.filled(&page(100.));
    assert_eq!(ground.plan(&page(100.)), Plan::Lay);
    assert_eq!(ground.plan(&page(100.)), Plan::Lay);
}

#[test]
fn a_page_that_keeps_changing_is_never_drawn_twice() {
    let mut ground = Ground::default();
    for x in [100., 110., 120., 130.] {
        assert_eq!(ground.plan(&page(x)), Plan::Draw);
    }
}

#[test]
fn a_changed_page_is_drawn_until_it_settles_again() {
    let mut ground = Ground::default();
    ground.filled(&page(100.));
    assert_eq!(ground.plan(&page(100.)), Plan::Lay);
    assert_eq!(ground.plan(&page(140.)), Plan::Draw);
    assert_eq!(ground.plan(&page(140.)), Plan::Fill);
    ground.filled(&page(140.));
    assert_eq!(ground.plan(&page(140.)), Plan::Lay);
}

#[test]
fn a_forgotten_picture_is_drawn_again_from_the_same_page() {
    let mut ground = Ground::default();
    ground.plan(&page(100.));
    ground.plan(&page(100.));
    ground.filled(&page(100.));
    ground.forget();
    assert!(!ground.holds(&page(100.)));
    assert_eq!(ground.plan(&page(100.)), Plan::Fill);
}

#[test]
fn a_page_kept_while_still_is_laid_on_the_overlay_first_frame() {
    let mut ground = Ground::default();
    assert!(!ground.holds(&page(100.)));
    ground.filled(&page(100.));
    assert!(ground.holds(&page(100.)));
    assert_eq!(ground.plan(&page(100.)), Plan::Lay);
}
