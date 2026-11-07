//! Fewer pixels for the same picture. The TV's GPU is fill-bound: every layer costs it each
//! pixel it covers, whatever it draws there. These find the pixels a layer leaves unchanged or
//! a later one hides, and layers that can be drawn as one.
use crate::{renderer::Draw, state::Rect};

/// A part of a quad, in fractions of it: left, top, right, bottom.
pub type Part = [f32; 4];

/// The parts of a quad around `hole`, which they leave out.
pub fn around([left, top, right, bottom]: Part) -> Vec<Part> {
    [
        [0., 0., 1., top],
        [0., bottom, 1., 1.],
        [0., top, left, bottom],
        [right, top, 1., bottom],
    ]
    .into_iter()
    .filter(|part| part[2] > part[0] && part[3] > part[1])
    .collect()
}

/// `hole` over `rect`, in fractions of it and cut to it; none when that leaves nothing.
pub fn hole_in(rect: Rect, hole: Rect) -> Option<Part> {
    if rect.w <= 0. || rect.h <= 0. {
        return None;
    }
    let x = |x: f32| ((x - rect.x) / rect.w).clamp(0., 1.);
    let y = |y: f32| ((y - rect.y) / rect.h).clamp(0., 1.);
    let part = [x(hole.x), y(hole.y), x(hole.x + hole.w), y(hole.y + hole.h)];
    (part[2] > part[0] && part[3] > part[1]).then_some(part)
}

/// Where a border's stroke never reaches: inside the stroke and its corners, with a pixel's
/// edge smoothing to spare.
pub fn inside_border(rect: Rect, radius: f32, border: f32) -> Rect {
    rect.inset(radius.max(border) + 2.)
}

/// An opaque glass's copy and where it hides what was drawn under it: inside its corners.
pub fn under_glass(command: &Draw) -> Option<(&str, Rect)> {
    match command {
        Draw::Glass(path, rect, radius, _, tint) if *tint == [1.; 4] => {
            Some((path, rect.inset(radius.max(0.) + 2.)))
        }
        _ => None,
    }
}

/// How many of the fills after an opaque glass lie over exactly its shape, so they fold into
/// it as into a wallpaper.
pub fn fills_over_glass(glass: &Draw, rest: &[Draw]) -> usize {
    let Draw::Glass(_, rect, radius, _, [1., 1., 1., 1.]) = glass else {
        return 0;
    };
    let over = |other: &Rect, other_radius: f32| {
        [other.x, other.y, other.w, other.h] == [rect.x, rect.y, rect.w, rect.h]
            && other_radius == *radius
    };
    rest.iter()
        .take_while(|command| match command {
            Draw::RoundedGradient(other, other_radius, ..) => over(other, *other_radius),
            Draw::Shape(other, other_radius, border, _) => {
                *border <= 0. && over(other, *other_radius)
            }
            _ => false,
        })
        .count()
}

/// A `width` × `height` rect rounded by `radius`, apart: rows of its interior clear of the
/// corners, at least `rows` of them, then its four corners. None when the corners leave no
/// interior.
pub fn rounded_apart(
    width: f32,
    height: f32,
    radius: f32,
    rows: usize,
) -> Option<(Vec<Part>, [Part; 4])> {
    // The shader rounds by at most half the shorter side; a pixel more covers its smoothing.
    let corner = radius.max(0.).min(width / 2.).min(height / 2.) + 1.;
    if 2. * corner >= width || 2. * corner >= height || rows == 0 {
        return None;
    }
    let (x, y) = (corner / width, corner / height);
    let mut edges: Vec<f32> = (0..=rows).map(|row| row as f32 / rows as f32).collect();
    edges.extend([y, 1. - y]);
    edges.sort_by(f32::total_cmp);
    edges.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let parts = edges
        .windows(2)
        .map(|pair| {
            let (top, bottom) = (pair[0], pair[1]);
            if top >= y - 1e-6 && bottom <= 1. - y + 1e-6 {
                [0., top, 1., bottom]
            } else {
                [x, top, 1. - x, bottom]
            }
        })
        .collect();
    let corners = [
        [0., 0., x, y],
        [1. - x, 0., 1., y],
        [0., 1. - y, x, 1.],
        [1. - x, 1. - y, 1., 1.],
    ];
    Some((parts, corners))
}

#[cfg(test)]
#[path = "layers-tests.rs"]
mod tests;
