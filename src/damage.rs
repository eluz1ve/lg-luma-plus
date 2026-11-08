//! Partial redraw: which part of the logical 1920×1080 scene changed since an earlier frame.
//! Consecutive scenes are compared command by command; a back buffer that still holds the frame
//! `age` presents ago needs the union of the last `age` changes redrawn.
use crate::{renderer::Draw, state::Rect};
use std::{collections::VecDeque, sync::Arc};

/// EGL swap chains rotate a few buffers; older contents are redrawn in full.
const HISTORY: usize = 4;
/// Commands searched ahead to realign after inserted or removed commands: a focus ring, or a
/// whole card (its shadows, face, logo and border) scrolling out of a row.
const LOOKAHEAD: usize = 48;

/// The redraw region of one frame: `None` is the whole scene, an empty rect is no change.
pub type Damage = Option<Rect>;

#[derive(Default)]
pub struct Tracker {
    previous: Vec<Draw>,
    history: VecDeque<Damage>,
}

impl Tracker {
    /// Records the scene about to be drawn.
    pub fn push(&mut self, commands: &[Draw]) {
        let damage = if self.previous.is_empty() {
            None
        } else {
            diff(&self.previous, commands)
        };
        self.history.push_front(damage);
        self.history.truncate(HISTORY);
        self.previous = commands.to_vec();
    }

    /// What to redraw into a back buffer holding the frame `age` presents ago; 0 means unknown.
    pub fn damage(&self, age: u32) -> Damage {
        let age = age as usize;
        if age == 0 || age > self.history.len() {
            return None;
        }
        self.history
            .iter()
            .take(age)
            .try_fold(Rect::default(), |area, damage| {
                Some(union(area, (*damage)?))
            })
    }

    /// After anything drew into the window outside this tracker, e.g. a resized drawable, or
    /// pixels changed under the same commands, as when a glass copy arrives.
    pub fn reset(&mut self) {
        self.previous.clear();
        self.history.clear();
    }
}

fn diff(old: &[Draw], new: &[Draw]) -> Damage {
    let mut area = Rect::default();
    // Clip, EdgeFade and EdgeFadeX restyle what is drawn after them, up to the next command of
    // their kind; one that changed damages exactly those commands.
    let mut restyled = [false; 3];
    // Under a clip that stayed put, a command paints only inside it: a text line, which
    // otherwise damages the screen's full width, stays inside a sliding panel's strip.
    let (old_clips, new_clips) = (clips(old), clips(new));
    let mut j = 0;
    for (i, command) in new.iter().enumerate() {
        let matched = if j < old.len() && same(&old[j], command) {
            j += 1;
            true
        } else if let Some(k) = (j..old.len().min(j + LOOKAHEAD)).find(|&k| same(&old[k], command))
        {
            // Commands left out since the old scene; a left-out state restyles what follows.
            for (removed, clip) in old[j..k].iter().zip(&old_clips[j..k]) {
                match state(removed) {
                    Some(kind) => restyled[kind] = true,
                    None => area = union(area, within(bounds(removed)?, *clip)),
                }
            }
            j = k + 1;
            true
        } else {
            false
        };
        match state(command) {
            Some(kind) => restyled[kind] = !matched,
            None if !matched || restyled.contains(&true) => {
                let clip = if restyled[0] { None } else { new_clips[i] };
                area = union(area, within(bounds(command)?, clip));
            }
            None => {}
        }
    }
    let start = j.min(old.len());
    for (removed, clip) in old[start..].iter().zip(&old_clips[start..]) {
        if state(removed).is_none() {
            area = union(area, within(bounds(removed)?, *clip));
        }
    }
    Some(area)
}

/// The clip each command is drawn under.
fn clips(commands: &[Draw]) -> Vec<Option<Rect>> {
    let mut clip = None;
    commands
        .iter()
        .map(|command| {
            if let Draw::Clip(rect) = command {
                clip = *rect;
            }
            clip
        })
        .collect()
}

/// `area` cut to `clip`; empty when they miss each other.
fn within(area: Rect, clip: Option<Rect>) -> Rect {
    let Some(clip) = clip else {
        return area;
    };
    let x = area.x.max(clip.x);
    let y = area.y.max(clip.y);
    let w = (area.x + area.w).min(clip.x + clip.w) - x;
    let h = (area.y + area.h).min(clip.y + clip.h) - y;
    if w <= 0. || h <= 0. {
        Rect::default()
    } else {
        Rect::new(x, y, w, h)
    }
}

/// Which state a command sets, if it is one.
fn state(command: &Draw) -> Option<usize> {
    match command {
        Draw::Clip(_) => Some(0),
        Draw::EdgeFade(_) => Some(1),
        Draw::EdgeFadeX(_) => Some(2),
        _ => None,
    }
}

/// The area a command can paint. `None` for state that affects everything drawn after it.
pub fn bounds(command: &Draw) -> Option<Rect> {
    match command {
        Draw::Shape(rect, ..)
        | Draw::RoundedGradient(rect, ..)
        | Draw::Image(_, rect, ..)
        | Draw::TintedImage(_, rect, ..)
        | Draw::AppImage(_, rect, ..)
        | Draw::Picture(_, rect, ..)
        | Draw::Cover(_, rect, ..)
        | Draw::Glass(_, rect, ..) => Some(rect.inset(-1.)),
        Draw::Shadow(rect, _, blur, _) => Some(rect.inset(-blur - 1.)),
        // Glyphs may extend past their layout box: redraw the full width of the line.
        Draw::Text(_, rect, style, ..) => Some(Rect::new(
            0.,
            rect.y - style.size,
            1920.,
            rect.h + 2. * style.size,
        )),
        Draw::Clip(_) | Draw::EdgeFade(_) | Draw::EdgeFadeX(_) => None,
    }
}

/// Whether two scenes draw the same thing, command for command.
pub fn same_scene(a: &[Draw], b: &[Draw]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
}

/// The first command where two scenes differ, for telling what changed.
pub fn first_difference(a: &[Draw], b: &[Draw]) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(a, b)| !same(a, b))
        .or_else(|| (a.len() != b.len()).then(|| a.len().min(b.len())))
}

fn same(a: &Draw, b: &Draw) -> bool {
    let rect = |a: &Rect, b: &Rect| a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h;
    match (a, b) {
        (Draw::Shape(r1, a1, b1, c1), Draw::Shape(r2, a2, b2, c2)) => {
            rect(r1, r2) && a1 == a2 && b1 == b2 && c1 == c2
        }
        (Draw::RoundedGradient(r1, a1, t1, b1), Draw::RoundedGradient(r2, a2, t2, b2)) => {
            rect(r1, r2) && a1 == a2 && t1 == t2 && b1 == b2
        }
        (Draw::Shadow(r1, a1, b1, c1), Draw::Shadow(r2, a2, b2, c2)) => {
            rect(r1, r2) && a1 == a2 && b1 == b2 && c1 == c2
        }
        (Draw::Text(t1, r1, s1, c1, x1), Draw::Text(t2, r2, s2, c2, x2)) => {
            t1 == t2
                && rect(r1, r2)
                && s1.size == s2.size
                && s1.weight == s2.weight
                && s1.tracking == s2.tracking
                && s1.line_height == s2.line_height
                && s1.align_right == s2.align_right
                && s1.shadow == s2.shadow
                && c1 == c2
                && x1 == x2
        }
        (Draw::Image(p1, r1, a1, c1), Draw::Image(p2, r2, a2, c2)) => {
            p1 == p2 && rect(r1, r2) && a1 == a2 && c1 == c2
        }
        (Draw::TintedImage(p1, r1, a1, c1, t1), Draw::TintedImage(p2, r2, a2, c2, t2)) => {
            p1 == p2 && rect(r1, r2) && a1 == a2 && c1 == c2 && t1 == t2
        }
        (Draw::AppImage(i1, r1, a1, t1), Draw::AppImage(i2, r2, a2, t2))
        | (Draw::Picture(i1, r1, a1, t1), Draw::Picture(i2, r2, a2, t2))
        | (Draw::Cover(i1, r1, a1, t1), Draw::Cover(i2, r2, a2, t2)) => {
            Arc::ptr_eq(i1, i2) && rect(r1, r2) && a1 == a2 && t1 == t2
        }
        (Draw::Glass(p1, r1, a1, s1, t1), Draw::Glass(p2, r2, a2, s2, t2)) => {
            p1 == p2 && rect(r1, r2) && a1 == a2 && rect(s1, s2) && t1 == t2
        }
        (Draw::Clip(c1), Draw::Clip(c2)) => match (c1, c2) {
            (Some(r1), Some(r2)) => rect(r1, r2),
            (None, None) => true,
            _ => false,
        },
        (Draw::EdgeFade(f1), Draw::EdgeFade(f2)) | (Draw::EdgeFadeX(f1), Draw::EdgeFadeX(f2)) => {
            f1 == f2
        }
        _ => false,
    }
}

pub fn intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn union(a: Rect, b: Rect) -> Rect {
    if a.w <= 0. || a.h <= 0. {
        return b;
    }
    if b.w <= 0. || b.h <= 0. {
        return a;
    }
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        (a.x + a.w).max(b.x + b.w) - x,
        (a.y + a.h).max(b.y + b.h) - y,
    )
}

#[cfg(test)]
#[path = "damage-tests.rs"]
mod tests;
