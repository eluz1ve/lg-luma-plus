//! The page under an overlay, kept as a picture. On the TV every full-screen layer costs the GPU
//! 2–3 ms whatever it draws, and Home is several of them; while the page stays the same, an
//! overlay's frames lay its picture down in one layer and draw only themselves on top.
//!
//! Drawing the picture costs a frame about twice its page, so it is drawn while the screen is
//! still (`Renderer::prime_ground`) and, mid-animation, only once the page has stayed the same
//! for longer than a transition lasts.
use crate::renderer::Draw;
use std::time::{Duration, Instant};

/// How long a page stays the same under a running animation before it is drawn into its
/// picture: longer than the menu's and Today's opening (0.25 s).
pub const SETTLED: Duration = Duration::from_millis(600);

/// What a frame does with the page under its overlay.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    /// Drawn with the overlay, as without a picture.
    Draw,
    /// Drawn into the picture, which is then laid down.
    Fill,
    /// The picture holds the page: laid down.
    Lay,
}

#[derive(Default)]
pub struct Ground {
    /// The page the picture holds.
    held: Option<Vec<Draw>>,
    /// The page under the last frame that drew one, and since when it has stayed so.
    seen: Vec<Draw>,
    since: Option<Instant>,
}

impl Ground {
    /// For a frame drawn at `now`.
    pub fn plan(&mut self, page: &[Draw], now: Instant) -> Plan {
        if self.holds(page) {
            return Plan::Lay;
        }
        if self.seen.is_empty() || !crate::damage::same_scene(&self.seen, page) {
            self.seen = page.to_vec();
            self.since = Some(now);
        }
        if self
            .since
            .is_some_and(|since| now.duration_since(since) >= SETTLED)
        {
            Plan::Fill
        } else {
            Plan::Draw
        }
    }

    pub fn holds(&self, page: &[Draw]) -> bool {
        self.held
            .as_deref()
            .is_some_and(|held| crate::damage::same_scene(held, page))
    }

    /// The picture now shows `page`.
    pub fn filled(&mut self, page: &[Draw]) {
        self.held = Some(page.to_vec());
    }

    /// The picture is out of date in a way the page's commands do not show (a glass copy
    /// arrived, the drawable was resized).
    pub fn forget(&mut self) {
        self.held = None;
        self.seen.clear();
    }
}

#[cfg(test)]
#[path = "ground-tests.rs"]
mod tests;
