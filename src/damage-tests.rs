use super::*;
use crate::renderer::TextStyle;

fn shape(x: f32) -> Draw {
    Draw::Shape(Rect::new(x, 300., 100., 100.), 8., 0., [1., 1., 1., 1.])
}
fn wallpaper() -> Draw {
    Draw::Image(
        "wallpapers/dunes.jpg".into(),
        Rect::new(0., 0., 1920., 1080.),
        0.,
        false,
    )
}
fn ring(x: f32) -> Draw {
    Draw::Shape(
        Rect::new(x - 4., 296., 108., 108.),
        12.,
        2.,
        [1., 1., 1., 1.],
    )
}
fn area(damage: Damage) -> Rect {
    damage.expect("partial damage")
}

#[test]
fn the_first_frame_and_unknown_buffer_ages_redraw_everything() {
    let mut tracker = Tracker::default();
    tracker.push(&[wallpaper(), shape(100.)]);
    assert!(tracker.damage(1).is_none());
    tracker.push(&[wallpaper(), shape(100.)]);
    assert!(tracker.damage(0).is_none());
    assert!(
        tracker.damage(3).is_none(),
        "older than the recorded history"
    );
}

#[test]
fn an_unchanged_scene_has_no_damage() {
    let mut tracker = Tracker::default();
    tracker.push(&[wallpaper(), shape(100.)]);
    tracker.push(&[wallpaper(), shape(100.)]);
    let damage = area(tracker.damage(1));
    assert!(damage.w <= 0. || damage.h <= 0.);
}

#[test]
fn a_moved_shape_damages_its_old_and_new_place_only() {
    let mut tracker = Tracker::default();
    tracker.push(&[wallpaper(), shape(100.)]);
    tracker.push(&[wallpaper(), shape(400.)]);
    let damage = area(tracker.damage(1));
    assert_eq!((damage.x, damage.w), (99., 402.));
    assert_eq!((damage.y, damage.h), (299., 102.));
}

#[test]
fn an_inserted_focus_ring_realigns_instead_of_redrawing_everything() {
    let mut tracker = Tracker::default();
    tracker.push(&[wallpaper(), shape(100.), shape(400.), shape(700.)]);
    tracker.push(&[
        wallpaper(),
        shape(100.),
        ring(400.),
        shape(400.),
        shape(700.),
    ]);
    let damage = area(tracker.damage(1));
    assert_eq!((damage.x, damage.w), (395., 110.));
    tracker.push(&[wallpaper(), shape(100.), shape(400.), shape(700.)]);
    let removed = area(tracker.damage(1));
    assert_eq!((removed.x, removed.w), (395., 110.));
}

#[test]
fn older_back_buffers_need_every_change_since_their_frame() {
    let mut tracker = Tracker::default();
    tracker.push(&[wallpaper(), shape(100.)]);
    tracker.push(&[wallpaper(), shape(400.)]);
    tracker.push(&[wallpaper(), shape(400.), shape(1200.)]);
    let latest = area(tracker.damage(1));
    assert_eq!(latest.x, 1199.);
    let two = area(tracker.damage(2));
    assert_eq!((two.x, two.x + two.w), (99., 1301.));
}

#[test]
fn changed_clip_or_fade_state_redraws_only_what_it_covers() {
    let mut tracker = Tracker::default();
    let clip = |y: f32| Draw::Clip(Some(Rect::new(0., y, 1920., 500.)));
    let below = Draw::Shape(Rect::new(100., 900., 100., 100.), 8., 0., [1.; 4]);
    tracker.push(&[
        wallpaper(),
        clip(100.),
        shape(100.),
        Draw::Clip(None),
        below.clone(),
    ]);
    tracker.push(&[
        wallpaper(),
        clip(120.),
        shape(100.),
        Draw::Clip(None),
        below.clone(),
    ]);
    let clipped = area(tracker.damage(1));
    assert_eq!(
        (clipped.x, clipped.y, clipped.w, clipped.h),
        (99., 299., 102., 102.)
    );
    // A fade restyles what it fades, up to the next fade.
    let fade = |edge: f32| Draw::EdgeFadeX(Some([edge, 1., 2., 3.]));
    tracker.push(&[
        wallpaper(),
        fade(0.),
        shape(100.),
        shape(400.),
        Draw::EdgeFadeX(None),
        below.clone(),
    ]);
    tracker.push(&[
        wallpaper(),
        fade(8.),
        shape(100.),
        shape(400.),
        Draw::EdgeFadeX(None),
        below,
    ]);
    let faded = area(tracker.damage(1));
    assert_eq!(
        (faded.x, faded.y, faded.x + faded.w, faded.h),
        (99., 299., 501., 102.)
    );
}

#[test]
fn a_card_leaving_a_row_does_not_misalign_everything_after_it() {
    let card = |x: f32| -> Vec<Draw> {
        (0..10)
            .map(|layer| {
                Draw::Shape(
                    Rect::new(x, 300., 200., 120.),
                    8. + layer as f32,
                    0.,
                    [1.; 4],
                )
            })
            .collect()
    };
    let rest: Vec<Draw> = (0..20)
        .map(|row| {
            Draw::Shape(
                Rect::new(80., 700. + row as f32 * 10., 400., 8.),
                2.,
                0.,
                [1.; 4],
            )
        })
        .collect();
    let scene = |cards: &[f32]| -> Vec<Draw> {
        let mut scene = vec![wallpaper()];
        for x in cards {
            scene.extend(card(*x));
        }
        scene.extend(rest.clone());
        scene
    };
    let mut tracker = Tracker::default();
    tracker.push(&scene(&[0., 300., 600.]));
    tracker.push(&scene(&[300., 600.]));
    let left = area(tracker.damage(1));
    assert_eq!((left.y, left.h), (299., 122.), "only the row: {left:?}");
    tracker.push(&scene(&[300., 600., 900.]));
    let entered = area(tracker.damage(1));
    assert_eq!(
        (entered.y, entered.h),
        (299., 122.),
        "only the row: {entered:?}"
    );
}

#[test]
fn text_damages_the_full_width_of_its_line() {
    let mut tracker = Tracker::default();
    let style = TextStyle {
        size: 20.,
        weight: 400,
        tracking: 0.,
        line_height: 1.2,
        align_right: false,
        shadow: false,
    };
    let clock = |value: &str| {
        Draw::Text(
            value.into(),
            Rect::new(80., 40., 120., 30.),
            style,
            [1.; 4],
            false,
        )
    };
    tracker.push(&[wallpaper(), clock("18:30")]);
    tracker.push(&[wallpaper(), clock("18:31")]);
    let damage = area(tracker.damage(1));
    assert_eq!((damage.x, damage.w), (0., 1920.));
    assert_eq!((damage.y, damage.h), (20., 70.));
}

#[test]
fn text_under_an_unchanged_clip_damages_its_line_only_inside_the_clip() {
    let mut tracker = Tracker::default();
    let style = TextStyle {
        size: 20.,
        weight: 400,
        tracking: 0.,
        line_height: 1.2,
        align_right: false,
        shadow: false,
    };
    // A label sliding in with a panel, under a clip that stays put.
    let label = |x: f32| {
        [
            Draw::Clip(Some(Rect::new(0., 0., 500., 1080.))),
            Draw::Text(
                "Apps".into(),
                Rect::new(x, 140., 200., 30.),
                style,
                [1.; 4],
                false,
            ),
            Draw::Clip(None),
        ]
    };
    tracker.push(&[[wallpaper()].as_slice(), &label(-100.)].concat());
    tracker.push(&[[wallpaper()].as_slice(), &label(-40.)].concat());
    let damage = area(tracker.damage(1));
    assert_eq!((damage.x, damage.w), (0., 500.));
    assert_eq!((damage.y, damage.h), (120., 70.));
}

#[test]
fn shadows_include_their_blur() {
    let mut tracker = Tracker::default();
    let shadow =
        |a: f32| Draw::Shadow(Rect::new(500., 500., 200., 100.), 12., 18., [0., 0., 0., a]);
    tracker.push(&[wallpaper(), shadow(0.2)]);
    tracker.push(&[wallpaper(), shadow(0.4)]);
    let damage = area(tracker.damage(1));
    assert_eq!((damage.x, damage.w), (481., 238.));
}

#[test]
fn culling_covers_every_pixel_the_damage_clears() {
    use crate::renderer::Renderer;
    let contains = |outer: Rect, inner: Rect| {
        outer.x <= inner.x + 1e-3
            && outer.y <= inner.y + 1e-3
            && outer.x + outer.w >= inner.x + inner.w - 1e-3
            && outer.y + outer.h >= inner.y + inner.h - 1e-3
    };
    let damage = Rect::new(100., 300., 3., 3.);
    for (width, height) in [
        (640, 360),
        (1280, 720),
        (1366, 768),
        (1920, 1080),
        (1920, 1200),
    ] {
        let pixels = Renderer::damage_pixels(damage, width, height);
        assert!(contains(
            Renderer::damage_area(pixels, width, height),
            damage
        ));
    }
    // At 640x360 a drawable pixel spans 3 logical units: the cleared pixels reach x 99..105,
    // so a command at x 104 must still be drawn although it misses the logical damage.
    let pixels = Renderer::damage_pixels(damage, 640, 360);
    let neighbour = bounds(&Draw::Shape(
        Rect::new(104.2, 300., 0.5, 3.),
        0.,
        0.,
        [1., 1., 1., 1.],
    ))
    .expect("shape bounds");
    assert!(!intersects(damage, neighbour));
    assert!(intersects(
        Renderer::damage_area(pixels, 640, 360),
        neighbour
    ));
}

#[test]
fn an_unchanged_frame_names_a_pixel_inside_the_drawable() {
    use crate::renderer::Renderer;
    // EGL_KHR_partial_update is told this rect and the frame is scissored to it. A rect
    // outside the surface leaves the driver nothing to preserve: Mali presented black.
    for (width, height) in [(1920, 1080), (1280, 720), (3840, 2160), (1920, 1200)] {
        let [x, y, w, h] = Renderer::damage_pixels(Rect::default(), width, height);
        assert!(w >= 1 && h >= 1, "{width}x{height}: empty damage {w}x{h}");
        assert!(
            x >= 0 && y >= 0 && x + w <= width as i32 && y + h <= height as i32,
            "{width}x{height}: [{x}, {y}, {w}, {h}] lies outside the drawable"
        );
    }
}

#[test]
fn glass_damages_its_panel() {
    let screen = Rect::new(0., 0., 1920., 1080.);
    let glass = |x: f32| {
        Draw::Glass(
            "wallpapers/dunes.jpg".into(),
            Rect::new(x, 40., 600., 68.),
            34.,
            screen,
            [1.; 4],
        )
    };
    let bounds = bounds(&glass(660.)).unwrap();
    let expected = Rect::new(660., 40., 600., 68.).inset(-1.);
    assert_eq!(
        (bounds.x, bounds.y, bounds.w, bounds.h),
        (expected.x, expected.y, expected.w, expected.h)
    );
    assert!(same(&glass(660.), &glass(660.)));
    assert!(!same(&glass(660.), &glass(670.)));
}

#[test]
fn two_scenes_are_the_same_only_command_for_command() {
    let scene = [wallpaper(), shape(100.), shape(300.)];
    assert!(same_scene(&scene, &scene.clone()));
    assert!(!same_scene(
        &scene,
        &[wallpaper(), shape(100.), shape(301.)]
    ));
    assert!(!same_scene(&scene, &scene[..2]));
}

#[test]
fn the_first_difference_between_scenes_is_found_command_for_command() {
    let shape = |x: f32| Draw::Shape(Rect::new(x, 0., 10., 10.), 0., 0., [1.; 4]);
    assert_eq!(
        first_difference(&[shape(0.), shape(1.)], &[shape(0.), shape(1.)]),
        None
    );
    assert_eq!(
        first_difference(&[shape(0.), shape(1.)], &[shape(0.), shape(2.)]),
        Some(1)
    );
    assert_eq!(
        first_difference(&[shape(0.)], &[shape(0.), shape(2.)]),
        Some(1)
    );
}
