use super::*;

/// Points spread over the unit square, none on a part's edge.
fn samples() -> impl Iterator<Item = (f32, f32)> {
    (0..97).flat_map(|i| (0..89).map(move |j| ((i as f32 + 0.37) / 97., (j as f32 + 0.61) / 89.)))
}

fn holds(part: &Part, (x, y): (f32, f32)) -> bool {
    part[0] <= x && x < part[2] && part[1] <= y && y < part[3]
}

#[test]
fn the_parts_around_a_hole_cover_the_rest_of_the_quad_once() {
    let hole = [0.2, 0.3, 0.7, 0.9];
    let parts = around(hole);
    for point in samples() {
        let covering = parts.iter().filter(|part| holds(part, point)).count();
        assert_eq!(covering, usize::from(!holds(&hole, point)), "{point:?}");
    }
}

#[test]
fn a_border_leaves_its_inside_clear_past_the_stroke_and_the_corners() {
    let rect = Rect::new(100., 100., 400., 300.);
    let inside = inside_border(rect, 34., 1.);
    assert_eq!(
        [inside.x, inside.y, inside.w, inside.h],
        [136., 136., 328., 228.]
    );
    // A stroke wider than the corners.
    let inside = inside_border(rect, 4., 10.);
    assert_eq!([inside.x, inside.w], [112., 376.]);
}

#[test]
fn a_hole_is_counted_in_fractions_of_its_quad_and_only_when_it_has_room() {
    let rect = Rect::new(100., 200., 400., 200.);
    assert_eq!(
        hole_in(rect, Rect::new(200., 250., 100., 100.)),
        Some([0.25, 0.25, 0.5, 0.75])
    );
    // Cut to the quad.
    assert_eq!(
        hole_in(rect, Rect::new(0., 300., 300., 300.)),
        Some([0., 0.5, 0.5, 1.])
    );
    assert_eq!(hole_in(rect, Rect::new(200., 250., 0., 100.)), None);
    assert_eq!(hole_in(rect, Rect::new(600., 250., 50., 50.)), None);
}

fn glass(rect: Rect, alpha: f32) -> Draw {
    Draw::Glass(
        "wallpapers/dunes.jpg".into(),
        rect,
        34.,
        Rect::new(0., 0., 1920., 1080.),
        [1., 1., 1., alpha],
    )
}

#[test]
fn an_opaque_glass_hides_what_lies_under_it_inside_its_corners() {
    let panel = Rect::new(410., 110., 1100., 860.);
    let opaque = glass(panel, 1.);
    let (path, hidden) = under_glass(&opaque).unwrap();
    assert_eq!(path, "wallpapers/dunes.jpg");
    assert_eq!(
        [hidden.x, hidden.y, hidden.w, hidden.h],
        [446., 146., 1028., 788.]
    );
    // Fading in over the other wallpaper's glass, it lets that through.
    assert!(under_glass(&glass(panel, 0.5)).is_none());
    assert!(under_glass(&Draw::Shape(panel, 34., 0., [1.; 4])).is_none());
}

#[test]
fn the_fills_over_exactly_a_glass_fold_into_it_up_to_its_rim() {
    let panel = Rect::new(410., 110., 1100., 860.);
    let rest = [
        Draw::RoundedGradient(panel, 34., [0.2, 0.2, 0.2, 0.78], [0.1, 0.1, 0.1, 0.86]),
        Draw::RoundedGradient(panel, 34., [1., 1., 1., 0.1], [1., 1., 1., 0.]),
        Draw::Shape(panel, 34., 0., [0., 0., 0., 0.1]),
        // The rim: a border, drawn as it is.
        Draw::Shape(panel, 34., 1., [1., 1., 1., 0.14]),
        Draw::Shape(panel, 34., 0., [0., 0., 0., 0.1]),
    ];
    assert_eq!(fills_over_glass(&glass(panel, 1.), &rest), 3);
    assert_eq!(fills_over_glass(&glass(panel, 0.5), &rest), 0);
    let elsewhere = [Draw::RoundedGradient(
        panel.inset(8.),
        34.,
        [0.; 4],
        [0.; 4],
    )];
    assert_eq!(fills_over_glass(&glass(panel, 1.), &elsewhere), 0);
    let rounder = [Draw::Shape(panel, 40., 0., [0.; 4])];
    assert_eq!(fills_over_glass(&glass(panel, 1.), &rounder), 0);
}

#[test]
fn a_rounded_rect_parts_into_rows_clear_of_its_corners_and_the_corners() {
    let (rows, corners) = rounded_apart(1100., 860., 34., 16).unwrap();
    let (rx, ry) = (35. / 1100., 35. / 860.);
    for point in samples() {
        let covering = rows
            .iter()
            .chain(&corners)
            .filter(|part| holds(part, point))
            .count();
        assert_eq!(covering, 1, "{point:?}");
    }
    for corner in &corners {
        assert!((corner[2] - corner[0] - rx).abs() < 1e-6);
        assert!((corner[3] - corner[1] - ry).abs() < 1e-6);
    }
    // At least 16 rows, so a gradient folds in as closely as over a wallpaper.
    assert!(rows.iter().filter(|row| row[0] == 0.).count() >= 14);
    // Too round to leave an interior.
    assert!(rounded_apart(60., 60., 34., 16).is_none());
}

#[test]
fn a_layer_with_no_opacity_changes_no_pixel() {
    let screen = Rect::new(0., 0., 1920., 1080.);
    let path = String::from("wallpapers/forest.jpg");
    assert!(invisible(&Draw::TintedImage(
        path.clone(),
        screen,
        0.,
        false,
        [1., 1., 1., 0.]
    )));
    assert!(!invisible(&Draw::TintedImage(
        path.clone(),
        screen,
        0.,
        false,
        [1., 1., 1., 0.01]
    )));
    assert!(invisible(&Draw::Glass(
        path.clone(),
        screen,
        34.,
        screen,
        [1., 1., 1., 0.]
    )));
    assert!(invisible(&Draw::Shape(screen, 0., 0., [0., 0., 0., 0.])));
    assert!(invisible(&Draw::Shadow(
        screen,
        34.,
        100.,
        [0., 0., 0., 0.]
    )));
    assert!(invisible(&Draw::RoundedGradient(
        screen,
        0.,
        [0.; 4],
        [1., 1., 1., 0.]
    )));
    assert!(!invisible(&Draw::RoundedGradient(
        screen,
        0.,
        [0.; 4],
        [1., 1., 1., 0.1]
    )));
    assert!(!invisible(&Draw::Image(path, screen, 0., false)));
}
