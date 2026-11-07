use super::*;
use image::{Rgba, RgbaImage};

fn luminance(image: &RgbaImage, x: u32, y: u32) -> f32 {
    let Rgba([r, g, b, _]) = *image.get_pixel(x, y);
    (r as f32 + g as f32 + b as f32) / 3.
}

#[test]
fn frost_makes_one_small_soft_copy() {
    let wallpaper = RgbaImage::from_fn(1920, 1080, |x, _| {
        if x < 960 {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([255, 255, 255, 255])
        }
    });
    let copy = frost(&wallpaper);
    assert_eq!(copy.dimensions(), (480, 270));
    assert!(luminance(&copy, 10, 135) < 8.);
    assert!(luminance(&copy, 470, 135) > 247.);
    assert!((luminance(&copy, 240, 135) - 128.).abs() <= 24.);
    let row: Vec<f32> = (200..=280).map(|x| luminance(&copy, x, 135)).collect();
    assert!(row.windows(2).all(|pair| pair[1] >= pair[0]), "{row:?}");
    let band = row.iter().filter(|&&v| v > 25.5 && v < 229.5).count();
    assert!(band > 12, "the edge is soft over {band} px");
}

#[test]
fn frost_takes_any_wallpaper_size() {
    for (width, height) in [(3840, 2160), (1001, 563)] {
        let wallpaper = RgbaImage::from_pixel(width, height, Rgba([40, 90, 160, 255]));
        let copy = frost(&wallpaper);
        assert_eq!(copy.dimensions(), (480, 270));
        for pixel in copy.pixels() {
            for (channel, expected) in pixel.0.iter().zip([40u8, 90, 160, 255]) {
                assert!(channel.abs_diff(expected) <= 1, "{pixel:?}");
            }
        }
    }
}

fn wait_ready(worker: &mut Worker) -> Vec<Reply> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let ready = worker.ready();
        if !ready.is_empty() || std::time::Instant::now() > deadline {
            return ready;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn the_worker_makes_each_copy_once_off_the_frame() {
    let directory = std::env::temp_dir().join(format!("luma-glass-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("wall.png");
    RgbaImage::from_pixel(64, 36, Rgba([40, 90, 160, 255]))
        .save(&path)
        .unwrap();
    let path = path.to_string_lossy().into_owned();
    let mut worker = Worker::default();
    assert!(!worker.pending());
    worker.request(&path);
    worker.request(&path);
    assert!(worker.pending());
    let ready = wait_ready(&mut worker);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].path, path);
    assert_eq!(
        ready[0].copy.as_ref().map(RgbaImage::dimensions),
        Some((480, 270))
    );
    assert!(ready[0].whole.is_none());
    assert!(!worker.pending());
    // A wallpaper that cannot be read gives no copy, once.
    let missing = directory.join("missing.jpg").to_string_lossy().into_owned();
    worker.request(&missing);
    let ready = wait_ready(&mut worker);
    assert_eq!(ready.len(), 1);
    assert!(ready[0].copy.is_none());
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_wallpaper_asked_for_whole_comes_back_decoded_with_its_copy() {
    let directory = std::env::temp_dir().join(format!("luma-whole-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("wall.png");
    RgbaImage::from_pixel(64, 36, Rgba([40, 90, 160, 255]))
        .save(&path)
        .unwrap();
    let path = path.to_string_lossy().into_owned();
    let mut worker = Worker::default();
    // Its copy asked for first, by a pane over it; then the whole of it, ahead of a fade.
    worker.request(&path);
    worker.request_whole(&path);
    worker.request_whole(&path);
    let mut replies = Vec::new();
    while worker.pending() {
        replies.extend(wait_ready(&mut worker));
    }
    let whole: Vec<_> = replies
        .iter()
        .filter(|reply| reply.whole.is_some())
        .collect();
    assert_eq!(whole.len(), 1);
    assert_eq!(
        whole[0].whole.as_ref().map(RgbaImage::dimensions),
        Some((64, 36))
    );
    assert_eq!(
        whole[0].copy.as_ref().map(RgbaImage::dimensions),
        Some((480, 270))
    );
    std::fs::remove_dir_all(&directory).unwrap();
}
