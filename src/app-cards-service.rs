//! The app cards' worker: reads the TV's list off the SDL thread, and fetches the cards'
//! pictures one at a time on a second thread so a slow server never delays the list.
use crate::app_artwork::Artwork;
use crate::app_cards::{AppCard, image_allowed};
use std::{io::Read, sync::Arc, sync::mpsc, thread, time::Duration};

const IMAGE_LIMIT: usize = 2 * 1024 * 1024;
/// The poster made from a picture: 2:3, as Lampa's, 300 pixels tall at 1080p and 600 at 4K.
const POSTER_WIDTH: u32 = 360;
const POSTER_HEIGHT: u32 = 540;
/// How much wider than the poster a wide picture is laid in: about two thirds of it show.
const WIDE_SCALE: f32 = 1.5;

pub enum Response {
    Cards(Result<Vec<AppCard>, String>),
    Image {
        url: String,
        image: Option<Arc<Artwork>>,
    },
}

pub struct Service {
    reads: mpsc::Sender<()>,
    images: mpsc::Sender<String>,
    pub receiver: mpsc::Receiver<Response>,
}

impl Service {
    pub fn new(webos: bool) -> Self {
        let (reads, wanted_reads) = mpsc::channel::<()>();
        let (images, wanted_images) = mpsc::channel::<String>();
        let (responses, receiver) = mpsc::channel();
        let image_responses = responses.clone();
        thread::spawn(move || {
            while wanted_reads.recv().is_ok() {
                while wanted_reads.try_recv().is_ok() {}
                let result = if webos {
                    read()
                } else {
                    Ok(crate::app_cards::demo())
                };
                if responses.send(Response::Cards(result)).is_err() {
                    break;
                }
            }
        });
        thread::spawn(move || {
            let agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(8)))
                .max_redirects(2)
                .build()
                .new_agent();
            for url in wanted_images {
                let image = fetch(&agent, &url);
                if image_responses
                    .send(Response::Image { url, image })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            reads,
            images,
            receiver,
        }
    }

    pub fn read(&self) {
        let _ = self.reads.send(());
    }

    pub fn image(&self, url: String) {
        let _ = self.images.send(url);
    }
}

#[cfg(all(feature = "webos", target_os = "linux", target_arch = "arm"))]
fn read() -> Result<Vec<AppCard>, String> {
    let reply = crate::luna::call_with_timeout(
        crate::app_cards::URI,
        &serde_json::json!({}),
        Duration::from_secs(10),
    )?;
    crate::app_cards::parse(&reply)
}

#[cfg(not(all(feature = "webos", target_os = "linux", target_arch = "arm")))]
fn read() -> Result<Vec<AppCard>, String> {
    Err("App cards are only available in an ARM webOS build".into())
}

fn fetch(agent: &ureq::Agent, url: &str) -> Option<Arc<Artwork>> {
    if !image_allowed(url) {
        return None;
    }
    let mut response = match agent.get(url).header("User-Agent", "LumaNative").call() {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Luma app card picture: {error} from {}", host(url));
            return None;
        }
    };
    if !response.status().is_success() {
        eprintln!(
            "Luma app card picture: HTTP {} from {}",
            response.status(),
            host(url)
        );
        return None;
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(IMAGE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > IMAGE_LIMIT {
        return None;
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = match reader.decode() {
        Ok(image) => image,
        Err(error) => {
            eprintln!("Luma app card picture: {error} from {}", host(url));
            return None;
        }
    };
    let pixels = poster(image);
    Some(Arc::new(Artwork {
        key: format!("app-card:{url}"),
        pixels,
        backdrop: [[0; 3]; 2],
        wash: None,
    }))
}

/// The server of a picture's address, for the log.
fn host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("?")
}

/// A picture as a 2:3 poster. One already about that shape is only scaled; a wide one is laid
/// across the middle at `WIDE_SCALE` the poster's width, over itself blurred and dimmed.
pub fn poster(image: image::DynamicImage) -> image::RgbaImage {
    use image::imageops::FilterType;
    let image = image.into_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return image;
    }
    if (width as f32) / (height as f32) < 0.9 {
        return if height > POSTER_HEIGHT {
            image::imageops::resize(
                &image,
                (width * POSTER_HEIGHT / height).max(1),
                POSTER_HEIGHT,
                FilterType::Triangle,
            )
        } else {
            image
        };
    }
    // The backdrop: the picture covering the poster, small, blurred, then dimmed.
    let small_h = POSTER_HEIGHT / 6;
    let small_w = (width * small_h / height).max(POSTER_WIDTH / 6);
    let small = image::imageops::resize(&image, small_w, small_h, FilterType::Triangle);
    let crop_x = (small_w - POSTER_WIDTH / 6) / 2;
    let small = image::imageops::crop_imm(&small, crop_x, 0, POSTER_WIDTH / 6, small_h).to_image();
    let small = image::imageops::blur(&small, 2.5);
    let mut out =
        image::imageops::resize(&small, POSTER_WIDTH, POSTER_HEIGHT, FilterType::Triangle);
    for pixel in out.pixels_mut() {
        for channel in &mut pixel.0[..3] {
            *channel = (*channel as f32 * 0.45) as u8;
        }
        pixel.0[3] = 255;
    }
    // The picture itself, centred, its sides cut by the poster.
    let wide_w = (POSTER_WIDTH as f32 * WIDE_SCALE) as u32;
    let wide_h = (wide_w * height / width).clamp(1, POSTER_HEIGHT);
    let wide = image::imageops::resize(&image, wide_w, wide_h, FilterType::Triangle);
    let from_x = (wide_w - POSTER_WIDTH) / 2;
    let shown = image::imageops::crop_imm(&wide, from_x, 0, POSTER_WIDTH, wide_h).to_image();
    let top = (POSTER_HEIGHT - wide_h) / 2;
    // Its top and bottom edges melt into the backdrop over a few rows.
    let feather = (wide_h / 8).max(1);
    for (x, y, pixel) in shown.enumerate_pixels() {
        let edge = y.min(wide_h - 1 - y);
        let alpha = (edge as f32 / feather as f32).min(1.);
        let alpha = alpha * alpha * (3. - 2. * alpha);
        let under = out.get_pixel_mut(x, top + y);
        for channel in 0..3 {
            under.0[channel] =
                (pixel.0[channel] as f32 * alpha + under.0[channel] as f32 * (1. - alpha)) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_pictures_become_two_by_three_posters_and_tall_ones_keep_their_shape() {
        let wide = image::DynamicImage::new_rgba8(960, 540);
        assert_eq!(poster(wide).dimensions(), (POSTER_WIDTH, POSTER_HEIGHT));
        let tall = image::DynamicImage::new_rgba8(382, 540);
        assert_eq!(poster(tall).dimensions(), (382, 540));
        let big = image::DynamicImage::new_rgba8(764, 1080);
        assert_eq!(poster(big).dimensions(), (382, 540));
    }
}
