//! The page under an overlay, kept as a picture. On the TV every full-screen layer costs the GPU
//! 2–3 ms whatever it draws, and Home is several of them; while the page stays the same, an
//! overlay's frames lay its picture down in one layer and draw only themselves on top.
use crate::renderer::Draw;

/// What a frame does with the page under its overlay.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    /// The page changed since the last frame: drawn with the overlay, as without a picture.
    Draw,
    /// The page stayed the same for a frame: drawn into the picture, which is then laid down.
    Fill,
    /// The picture holds the page: laid down.
    Lay,
}

#[derive(Default)]
pub struct Ground {
    /// The page the picture holds.
    held: Option<Vec<Draw>>,
    /// The page under the last frame that drew one.
    seen: Vec<Draw>,
}

impl Ground {
    /// A page that keeps changing is drawn once a frame, never into the picture as well.
    pub fn plan(&mut self, page: &[Draw]) -> Plan {
        if self.holds(page) {
            return Plan::Lay;
        }
        if !self.seen.is_empty() && crate::damage::same_scene(&self.seen, page) {
            return Plan::Fill;
        }
        self.seen = page.to_vec();
        Plan::Draw
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
    }
}

#[cfg(test)]
#[path = "ground-tests.rs"]
mod tests;
