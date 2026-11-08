//! A logo's colours as a soft fill for the card around it: strongest beside the logo at the
//! card's right, gone by its left, so the text stays on dark glass. Painted on the worker that
//! decodes the logo.
use crate::app_artwork::{hsl, hsv, luminance};
use image::{Rgba, RgbaImage};

/// The card's own proportions; the renderer's filtering smooths it out.
const WIDTH: u32 = 114;
const HEIGHT: u32 = 50;
/// Dark enough for white text on the fill at full strength, whatever the hue.
const MAX_LUMINANCE: f32 = 0.16;
/// A second hue counts when it is at least this far from the first, in 15° steps, and this strong.
const SECOND_APART: usize = 3;
const SECOND_STRENGTH: f32 = 0.2;
/// Neighbouring shade for a logo of one hue, so the fill still has some depth.
const ANALOGOUS: f32 = 28.;

/// The logo's two strongest hues in degrees, strongest first, or `None` when it has no clear
/// colour (black, white and grey logos).
fn hues(logo: &RgbaImage) -> Option<[f32; 2]> {
    let (mut opaque, mut saturated) = (0f32, 0f32);
    let mut bins = [(0f32, 0f32, 0f32); 24];
    for pixel in logo.pixels() {
        let alpha = pixel[3] as f32 / 255.;
        if alpha < 0.25 {
            continue;
        }
        opaque += alpha;
        let (hue, saturation, value) = hsv([pixel[0], pixel[1], pixel[2]]);
        if saturation >= 0.3 && value >= 0.2 {
            let bin = &mut bins[(hue / 15.) as usize % 24];
            let strength = alpha * saturation * value;
            bin.0 += strength;
            bin.1 += strength * hue.to_radians().cos();
            bin.2 += strength * hue.to_radians().sin();
            saturated += alpha;
        }
    }
    if opaque <= 0. || saturated < 0.15 * opaque {
        return None;
    }
    let angle = |bin: (f32, f32, f32)| bin.2.atan2(bin.1).to_degrees().rem_euclid(360.);
    let (first, top) = bins
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.0.total_cmp(&b.1.0))?;
    let second = bins
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, bin)| {
            let apart = index.abs_diff(first);
            apart.min(24 - apart) >= SECOND_APART && bin.0 >= SECOND_STRENGTH * top.0
        })
        .max_by(|a, b| a.1.0.total_cmp(&b.1.0))
        .map_or((angle(top) + ANALOGOUS) % 360., |(_, bin)| angle(bin));
    Some([angle(top), second])
}

/// The hue as a deep colour: as light as white text on it allows.
fn deep(hue: f32, saturation: f32) -> [u8; 3] {
    let mut lightness = 0.52;
    let mut color = hsl(hue, saturation, lightness);
    while luminance(color) > MAX_LUMINANCE && lightness > 0.1 {
        lightness -= 0.02;
        color = hsl(hue, saturation, lightness);
    }
    color
}

fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// The fill for a card with this logo, or `None` when the logo has no colour to take.
pub fn paint(logo: &RgbaImage) -> Option<RgbaImage> {
    let [first, second] = hues(logo)?;
    let (near, far) = (deep(first, 0.74), deep(second, 0.66));
    Some(RgbaImage::from_fn(WIDTH, HEIGHT, |x, y| {
        let u = x as f32 / (WIDTH - 1) as f32;
        let v = y as f32 / (HEIGHT - 1) as f32;
        let mix = smooth(0.3, 0.95, u);
        let channel = |i: usize| (far[i] as f32 + (near[i] as f32 - far[i] as f32) * mix) as u8;
        // A glow around where the logo sits, over a slope across the card.
        let (dx, dy) = ((u - 0.84) / 0.55, (v - 0.42) / 0.9);
        let glow = (-2.2 * (dx * dx + dy * dy)).exp();
        let alpha = (0.05 + 0.52 * smooth(0.02, 1., u) + 0.32 * glow).min(0.84);
        Rgba([channel(0), channel(1), channel(2), (alpha * 255.) as u8])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(color: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(32, 32, Rgba([color[0], color[1], color[2], 255]))
    }

    #[test]
    fn the_fill_takes_the_logos_colour_and_gathers_at_the_right() {
        let wash = paint(&flat([0, 80, 255])).unwrap();
        let right = wash.get_pixel(WIDTH - 1, HEIGHT / 2);
        let left = wash.get_pixel(0, HEIGHT / 2);
        assert!(right[2] > right[0] && right[2] > right[1], "{right:?}");
        assert!(right[3] > left[3] * 4, "{right:?} {left:?}");
    }

    #[test]
    fn a_logo_of_two_colours_uses_both() {
        let mut logo = flat([230, 30, 30]);
        for y in 20..32 {
            for x in 0..32 {
                logo.put_pixel(x, y, Rgba([20, 60, 230, 255]));
            }
        }
        let [first, second] = hues(&logo).unwrap();
        assert!(!(30. ..330.).contains(&first), "{first}");
        assert!((200. ..260.).contains(&second), "{second}");
    }

    #[test]
    fn a_logo_without_colour_gets_no_fill() {
        assert!(paint(&flat([240, 240, 240])).is_none());
        assert!(paint(&flat([12, 12, 12])).is_none());
        assert!(paint(&RgbaImage::new(8, 8)).is_none());
    }

    #[test]
    fn white_text_stays_readable_on_every_hue() {
        for hue in (0..360).step_by(5) {
            let color = deep(hue as f32, 0.74);
            assert!(luminance(color) <= MAX_LUMINANCE, "{hue}: {color:?}");
        }
    }
}
