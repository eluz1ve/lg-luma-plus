//! The launcher's shared motion: critically damped springs for the focused card's lift and for
//! scrolling. Retargeting keeps position and velocity, the motion is the same at any frame rate,
//! and reduced motion snaps everything to its target. Any card or row a widget adds takes part
//! by its `Target` or a `Lane`, without motion code of its own.
use crate::state::Target;
use std::collections::BTreeMap;

/// The focused card lifts in about a fifth of a second.
const FOCUS_OMEGA: f32 = 24.;
/// Scrolling covers 90% of the way in about a fifth of a second, starting and stopping softly.
const SCROLL_OMEGA: f32 = 19.5;

#[derive(Default, Clone, Copy)]
struct Spring {
    value: f32,
    velocity: f32,
    target: f32,
}

impl Spring {
    fn at(value: f32) -> Self {
        Self {
            value,
            velocity: 0.,
            target: value,
        }
    }

    fn moving(&self) -> bool {
        self.value != self.target || self.velocity != 0.
    }

    /// `epsilon` is how close counts as there: a fraction for focus, about a pixel for scrolling.
    fn advance(&mut self, dt: f32, omega: f32, reduced: bool, epsilon: f32) {
        if reduced {
            self.value = self.target;
            self.velocity = 0.;
            return;
        }
        // Analytic solution: consistent motion at 30, 60 and 120 Hz, without Euler instability.
        let displacement = self.value - self.target;
        let coefficient = self.velocity + omega * displacement;
        let decay = (-omega * dt).exp();
        self.value = self.target + (displacement + coefficient * dt) * decay;
        self.velocity = (self.velocity - omega * coefficient * dt) * decay;
        if (self.value - self.target).abs() < epsilon && self.velocity.abs() < epsilon * 20. {
            self.value = self.target;
            self.velocity = 0.;
        }
    }
}

/// Something that scrolls: Home's page, or a row on it. A widget with a row of its own adds
/// its lane here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lane {
    /// Home, up and down between the widgets.
    Page,
    Favorites,
    Cs2,
    Shows,
    /// The tab bar (and Apps' filters) sliding away once the page has scrolled.
    Chrome,
    /// Settings' sidebar, in pixels: a column of icons, or open with the sections' names.
    Sidebar,
    /// The opened Today card's list.
    Today,
}

impl Lane {
    /// How close counts as there, in the lane's own unit: about a pixel.
    fn settled_within(self) -> f32 {
        match self {
            // These two count whole cards rather than pixels.
            Lane::Cs2 | Lane::Shows => 0.002,
            _ => 0.5,
        }
    }
}

#[derive(Default)]
pub struct Motion {
    focus: BTreeMap<String, Spring>,
    lanes: BTreeMap<Lane, Spring>,
}

impl Motion {
    fn key(target: &Target) -> String {
        format!("{target:?}")
    }

    /// How far a card has lifted toward focus, 0–1.
    pub fn focus(&self, target: &Target) -> f32 {
        self.focus
            .get(&Self::key(target))
            .map_or(0., |spring| spring.value.clamp(0., 1.))
    }

    /// Where a lane is drawn: its spring's position, or `target` when it is not moving.
    pub fn shown(&self, lane: Lane, target: f32) -> f32 {
        self.lanes.get(&lane).map_or(target, |spring| spring.value)
    }

    pub fn animating(&self) -> bool {
        self.focus.values().any(Spring::moving) || self.lanes.values().any(Spring::moving)
    }

    /// Lifts `selected` and lets the other cards settle; eases each lane toward its target.
    /// A lane seen for the first time starts at its target, and lanes left out are forgotten,
    /// so returning to a page never slides in from where it was. Returns whether anything moved.
    pub fn advance(
        &mut self,
        selected: Option<&Target>,
        lanes: &[(Lane, f32)],
        dt: f32,
        reduced: bool,
    ) -> bool {
        let dt = dt.max(0.);
        let selected = selected.map(Self::key);
        if let Some(key) = &selected {
            self.focus.entry(key.clone()).or_default();
        }
        let mut changed = false;
        for (key, spring) in &mut self.focus {
            spring.target = if selected.as_ref() == Some(key) {
                1.
            } else {
                0.
            };
            changed |= spring.moving();
            spring.advance(dt, FOCUS_OMEGA, reduced, 0.001);
        }
        self.focus
            .retain(|_, spring| spring.target != 0. || spring.moving());

        self.lanes
            .retain(|lane, _| lanes.iter().any(|(kept, _)| kept == lane));
        for &(lane, target) in lanes {
            let spring = self.lanes.entry(lane).or_insert_with(|| Spring::at(target));
            spring.target = target;
            changed |= spring.moving();
            spring.advance(dt, SCROLL_OMEGA, reduced, lane.settled_within());
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> Target {
        Target::App(id.into())
    }

    #[test]
    fn retarget_preserves_motion_and_eventually_stops() {
        let mut motion = Motion::default();
        motion.advance(Some(&app("youtube")), &[], 0.08, false);
        let before = motion.focus(&app("youtube"));
        assert!(before > 0. && before < 1.);
        motion.advance(Some(&app("netflix")), &[], 0., false);
        assert_eq!(motion.focus(&app("youtube")), before);
        assert_eq!(motion.focus(&app("netflix")), 0.);
        for _ in 0..120 {
            motion.advance(Some(&app("netflix")), &[], 1. / 60., false);
        }
        assert_eq!(motion.focus(&app("netflix")), 1.);
        assert_eq!(motion.focus.len(), 1);
        assert!(!motion.animating());
        motion.advance(None, &[], 2., false);
        assert!(motion.focus.is_empty());
    }

    #[test]
    fn motion_is_frame_rate_independent() {
        let mut slow = Motion::default();
        let mut fast = Motion::default();
        slow.advance(None, &[(Lane::Page, 0.)], 0., false);
        fast.advance(None, &[(Lane::Page, 0.)], 0., false);
        for _ in 0..6 {
            slow.advance(Some(&app("a")), &[(Lane::Page, 400.)], 1. / 30., false);
        }
        for _ in 0..24 {
            fast.advance(Some(&app("a")), &[(Lane::Page, 400.)], 1. / 120., false);
        }
        assert!((slow.focus(&app("a")) - fast.focus(&app("a"))).abs() < 0.00001);
        assert!((slow.shown(Lane::Page, 400.) - fast.shown(Lane::Page, 400.)).abs() < 0.01);
    }

    #[test]
    fn lanes_ease_softly_start_where_they_are_and_are_forgotten_when_left() {
        let mut motion = Motion::default();
        motion.advance(None, &[(Lane::Favorites, 0.)], 1. / 60., false);
        assert_eq!(motion.shown(Lane::Favorites, 0.), 0.);
        motion.advance(None, &[(Lane::Favorites, 300.)], 1. / 60., false);
        let first = motion.shown(Lane::Favorites, 300.);
        // A spring starts slowly rather than jumping a fixed share of the way.
        assert!(first > 0. && first < 30.);
        for _ in 0..60 {
            motion.advance(None, &[(Lane::Favorites, 300.)], 1. / 60., false);
        }
        assert_eq!(motion.shown(Lane::Favorites, 300.), 300.);
        assert!(!motion.animating());
        motion.advance(None, &[], 1. / 60., false);
        assert_eq!(motion.shown(Lane::Favorites, 50.), 50.);
    }

    #[test]
    fn a_row_counted_in_cards_moves_like_one_counted_in_pixels() {
        const PITCH: f32 = 240.;
        let mut motion = Motion::default();
        let start = [(Lane::Shows, 0.), (Lane::Favorites, 0.)];
        motion.advance(None, &start, 1. / 60., false);
        let step = [(Lane::Shows, 1.), (Lane::Favorites, PITCH)];
        for frame in 0..60 {
            motion.advance(None, &step, 1. / 60., false);
            let cards = motion.shown(Lane::Shows, 1.) * PITCH;
            let pixels = motion.shown(Lane::Favorites, PITCH);
            assert!(
                (cards - pixels).abs() <= 1.,
                "frame {frame}: the cards are at {cards}px, the favorites at {pixels}px"
            );
        }
    }

    #[test]
    fn reduced_motion_snaps_and_does_not_keep_rendering() {
        let mut motion = Motion::default();
        motion.advance(Some(&app("a")), &[(Lane::Page, 0.)], 0.02, false);
        motion.advance(Some(&app("b")), &[(Lane::Page, 500.)], 0., true);
        assert_eq!(motion.focus(&app("a")), 0.);
        assert_eq!(motion.focus(&app("b")), 1.);
        assert_eq!(motion.shown(Lane::Page, 500.), 500.);
        assert!(!motion.animating());
        assert!(!motion.advance(Some(&app("b")), &[(Lane::Page, 500.)], 1., true));
    }
}
