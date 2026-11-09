//! Screen geometry and styling mirror the web UI's 1920×1080 / 16px-rem layout.
use crate::{
    i18n::Language,
    model::{App, BACKGROUNDS, CATEGORIES, Input, InputMode, UiSize},
    renderer::{Color, Draw, TextStyle, WHITE},
    state::{
        APPS_COLUMNS, Control, HOME_FADE_LEFT, HOME_FADE_RIGHT, HOME_STRIP_LEFT, HOME_STRIP_RIGHT,
        HOME_TILE_PITCH, HOME_TILE_WIDTH, HOMEBREW_UPDATES_ROW, KEYS, Overlay, Page, Rect,
        SYSTEM_SECTION, State, Subpage, Target, TextField, WIDGETS_SECTION,
    },
    weather::{self, Unit},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

use crate::layout::{
    CONTENT_TOP, GALLERY_COLUMNS, GALLERY_GAP_X, GALLERY_GAP_Y, GALLERY_TILE_HEIGHT,
    GALLERY_TILE_WIDTH, GALLERY_TOP, GUTTER, HEADER_BOTTOM, HOME_BOTTOM, LIST_TOP, LIST_WIDTH,
    LIST_X, PREVIEW_HEIGHT, PREVIEW_WIDTH, PREVIEW_X, PREVIEW_Y, ROW_GROWN, ROW_HEIGHT,
    ROW_SPACING, SAFE_X, SIDEBAR_COLLAPSED, SIDEBAR_EXPANDED, SIDEBAR_ITEM, SIDEBAR_PITCH, WIDTH,
    column as column_width, type_scale,
};

const MAIN_TOP: f32 = 108.8;
const FACE_HEIGHT: f32 = 154.4;
const RECENTS_BACKDROP_OPACITY: f32 = 0.86;

#[path = "cs2-icons.rs"]
mod cs2_icons;
#[path = "cs2-live-view.rs"]
mod cs2_live_view;
#[path = "recents-view.rs"]
mod recents_view;

pub struct Assets {
    pub root: PathBuf,
    pub version: String,
    pub metrics: BTreeMap<String, BTreeMap<char, f32>>,
    pub artwork: crate::app_artwork::Service,
}
impl Assets {
    pub fn load(root: PathBuf, webos: bool) -> Result<Self, String> {
        let load = |file: &str| {
            std::fs::read(root.join(file))
                .map_err(|e| format!("Native assets are missing: {e}. Run pnpm assets."))
        };
        // Generated artwork may outlive a binary rebuild; version the running app, not the assets.
        let version = env!("CARGO_PKG_VERSION").to_owned();
        let metrics =
            serde_json::from_slice(&load("font-metrics.json")?).map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            version,
            metrics,
            artwork: crate::app_artwork::Service::new(webos),
        })
    }
    fn path(&self, path: impl AsRef<Path>) -> String {
        self.root.join(path).to_string_lossy().into_owned()
    }

    /// A gallery card's picture; none for Apple TV until its place's still arrives.
    fn background_thumbnail(&self, state: &State, id: &str) -> Option<String> {
        if id == crate::aerials::ID {
            let place = &state.aerial_place().id;
            return state
                .aerials
                .iter()
                .find(|still| still.id == *place)
                .map(|still| still.thumbnail());
        }
        Some(
            state
                .wallpapers
                .iter()
                .find(|item| item.id == id)
                .map_or_else(
                    || self.path(format!("thumbnails/{id}.jpg")),
                    |item| item.thumbnail(),
                ),
        )
    }

    fn background(&self, state: &State, id: &str) -> String {
        self.background_shaded(state, id, state.preferences.wallpaper_gradient)
    }

    /// The wallpaper `id`, with or without the shade laid under Home's text.
    fn background_shaded(&self, state: &State, id: &str, shaded: bool) -> String {
        // An Apple TV still: `aerials#place`.
        if let Some(place) = id
            .strip_prefix(crate::aerials::ID)
            .and_then(|rest| rest.strip_prefix('#'))
            && let Some(still) = state.aerials.iter().find(|still| still.id == place)
        {
            return if shaded {
                still.image()
            } else {
                still.unshaded_image()
            };
        }
        // A dynamic wallpaper's frame: `id#frame`.
        if let Some((id, frame)) = id.split_once('#')
            && let Some(item) = state.wallpapers.iter().find(|item| item.id == id)
            && let Ok(frame) = frame.parse::<usize>()
            && frame < item.frames.len()
        {
            return item.frame(frame, shaded);
        }
        let id = id.split_once('#').map_or(id, |(id, _)| id);
        if let Some(item) = state.wallpapers.iter().find(|item| item.id == id) {
            return if shaded {
                item.image()
            } else {
                item.unshaded_image()
            };
        }
        let id = if BACKGROUNDS.iter().any(|(known, _)| *known == id) {
            id
        } else {
            "dunes"
        };
        let suffix = if shaded { "" } else { "-plain" };
        self.path(format!("wallpapers/{id}{suffix}.jpg"))
    }

    fn width(&self, value: &str, style: TextStyle) -> f32 {
        let key = format!(
            "{}{}",
            if style.size >= 32. { "display-" } else { "" },
            style.weight
        );
        let advances = self.metrics.get(&key);
        let fallback = self.metrics.get(if style.weight >= 500 {
            "fallback-500"
        } else {
            "fallback-400"
        });
        value
            .chars()
            .map(|c| {
                (advances
                    .and_then(|a| a.get(&c))
                    .or_else(|| fallback.and_then(|a| a.get(&c)))
                    .copied()
                    .unwrap_or(if c == ' ' { 0.28 } else { 0.53 })
                    + style.tracking)
                    * style.size
            })
            .sum()
    }

    fn fit_text(&self, value: &str, mut style: TextStyle, width: f32) -> TextStyle {
        // Leave a pixel for floating-point differences from the renderer's
        // font advances. Re-measure if shrinking crosses the display-font
        // threshold at 32px, where a different optical font is selected.
        let available = width - 1.;
        for _ in 0..2 {
            let measured = self.width(value, style);
            if measured <= available {
                break;
            }
            style.size *= available / measured;
        }
        style
    }

    /// `value` cut to `width` with an ellipsis, at its own size.
    fn ellipsize(&self, value: &str, style: TextStyle, width: f32) -> String {
        let available = width - 1.;
        if self.width(value, style) <= available {
            return value.to_owned();
        }
        let mut kept: Vec<char> = value.chars().collect();
        while !kept.is_empty() {
            kept.pop();
            let cut = format!("{}…", kept.iter().collect::<String>().trim_end());
            if self.width(&cut, style) <= available {
                return cut;
            }
        }
        "…".to_owned()
    }

    fn lines(&self, value: &str, style: TextStyle, width: f32) -> usize {
        let key = format!(
            "{}{}",
            if style.size >= 32. { "display-" } else { "" },
            style.weight
        );
        let advances = self.metrics.get(&key);
        let fallback = self.metrics.get(if style.weight >= 500 {
            "fallback-500"
        } else {
            "fallback-400"
        });
        let advance = |c: char| {
            (advances
                .and_then(|a| a.get(&c))
                .or_else(|| fallback.and_then(|a| a.get(&c)))
                .copied()
                .unwrap_or(if c == ' ' { 0.28 } else { 0.53 })
                + style.tracking)
                * style.size
        };
        let mut lines = 1;
        let mut used = 0.;
        for word in value.split_inclusive(char::is_whitespace) {
            let w: f32 = word.chars().map(advance).sum();
            if used > 0. && used + w > width {
                lines += 1;
                used = 0.;
            }
            for c in word.chars() {
                if c == '\n' {
                    lines += 1;
                    used = 0.;
                    continue;
                }
                let w = advance(c);
                if used > 0. && used + w > width {
                    lines += 1;
                    used = 0.;
                }
                used += w;
            }
        }
        lines
    }
}

pub struct Scene {
    pub draw: Vec<Draw>,
    pub controls: Vec<Control>,
    pub modal: Option<Rect>,
    /// Where Home draws what a setting changes, so Settings' preview can ring it.
    pub regions: Vec<(Region, Rect)>,
    /// Where Settings wants Home drawn small (`home_preview`).
    pub preview: Option<PreviewSlot>,
    /// The wallpapers behind the page and their opacities (two while they cross-fade), which
    /// glass samples.
    pub backdrop: Vec<(String, f32)>,
    /// How many leading commands are the page with its header, under whatever is drawn over
    /// it; while they stay the same, the renderer lays them down as one picture.
    pub ground: usize,
    /// The new wallpaper's opacity while it fades in over the old one.
    pub wallpaper_fade: Option<f32>,
    /// One lap of the Today card's price ticker in pixels; 0 when its coins stand still.
    pub ticker: f32,
}

/// The wallpapers a scene lies on while one fades into the next: both, or either end alone.
/// Everything over them is laid on as `over`, which is linear in what lies below, so the fade
/// is the mix of its two ends: the renderer draws those once and mixes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wallpapers {
    Fading,
    Previous,
    Current,
}

/// A part of Home that settings change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Clock,
    Date,
    Greeting,
    Weather,
    Favorites,
    Inputs,
    Crypto,
    Cs2,
    Shows,
    Apps,
}

/// The place in Settings' draw list for Home's preview, and the part of Home to ring in it.
pub struct PreviewSlot {
    at: usize,
    region: Option<Region>,
}

impl Scene {
    fn new() -> Self {
        Self {
            draw: Vec::new(),
            controls: Vec::new(),
            modal: None,
            regions: Vec::new(),
            preview: None,
            backdrop: Vec::new(),
            ground: 0,
            wallpaper_fade: None,
            ticker: 0.,
        }
    }
    fn shape(&mut self, rect: Rect, radius: f32, fill: Color) {
        self.draw.push(Draw::Shape(rect, radius, 0., fill));
    }
    fn border(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.draw.push(Draw::Shape(rect, radius, width, color));
    }
    fn text(
        &mut self,
        text: impl Into<String>,
        rect: Rect,
        style: TextStyle,
        color: Color,
        center: bool,
    ) {
        let color = if style.shadow { solid(color) } else { color };
        self.draw
            .push(Draw::Text(text.into(), rect, style, color, center));
    }
    fn image(&mut self, path: String, rect: Rect, radius: f32, contain: bool) {
        self.draw.push(Draw::Image(path, rect, radius, contain));
    }
    fn icon(&mut self, assets: &Assets, name: &str, rect: Rect, color: Color) {
        self.draw.push(Draw::TintedImage(
            assets.path(format!("ui-icons/{name}.lvec")),
            rect,
            0.,
            true,
            color,
        ));
    }
    /// An icon straight on the wallpaper, over a soft shadow like its text's.
    fn shadowed_icon(&mut self, assets: &Assets, name: &str, rect: Rect, color: Color) {
        let path = assets.path(format!("ui-icons/{name}.lvec"));
        let (taps, alpha) = crate::renderer::shadow_taps(rect.h * 0.8);
        let shade = [0., 0., 0., alpha * color[3]];
        for [dx, dy] in taps {
            let rect = Rect::new(rect.x + dx, rect.y + dy, rect.w, rect.h);
            self.draw
                .push(Draw::TintedImage(path.clone(), rect, 0., true, shade));
        }
        self.draw
            .push(Draw::TintedImage(path, rect, 0., true, color));
    }
    /// Moves what was drawn and registered since `draws` and `controls` up by `dy`: chrome that
    /// slides away with the page.
    fn slide(&mut self, draws: usize, controls: usize, dy: f32) {
        if dy == 0. {
            return;
        }
        for draw in &mut self.draw[draws..] {
            match draw {
                Draw::Shape(rect, ..)
                | Draw::RoundedGradient(rect, ..)
                | Draw::Shadow(rect, ..)
                | Draw::Text(_, rect, ..)
                | Draw::Image(_, rect, ..)
                | Draw::TintedImage(_, rect, ..)
                | Draw::AppImage(_, rect, ..)
                | Draw::Picture(_, rect, ..)
                | Draw::Cover(_, rect, ..)
                // The wallpaper stays: the panel samples it at its new place.
                | Draw::Glass(_, rect, ..) => rect.y -= dy,
                Draw::Clip(_) | Draw::EdgeFade(_) | Draw::EdgeFadeX(_) => {}
            }
        }
        for control in &mut self.controls[controls..] {
            control.rect.y -= dy;
        }
    }
    fn control(&mut self, target: Target, rect: Rect, scrollable: bool) {
        self.controls.push(Control {
            target,
            rect,
            scrollable,
        });
    }
    fn ring(&mut self, rect: Rect, radius: f32) {
        self.border(rect.inset(-6.9), radius + 6.9, 3.7, c(0xffffff, 0.10));
        self.border(rect.inset(-3.2), radius + 3.2, 3.2, c(0xf0f0f0, 1.));
    }
    fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, y: f32, alpha: f32) {
        self.draw.push(Draw::Shadow(
            Rect::new(rect.x, rect.y + y, rect.w, rect.h),
            radius,
            blur,
            c(0, alpha),
        ));
    }
    fn pill(&mut self, rect: Rect, focused: bool, selected: bool) {
        self.shadow(rect, rect.h / 2., 18., 4., 0.06);
        if focused {
            self.draw.push(Draw::RoundedGradient(
                rect,
                rect.h / 2.,
                c(0xfafafa, 1.),
                c(0xe5e5e5, 1.),
            ));
        } else if selected {
            self.shape(rect, rect.h / 2., c(0x303438, 1.));
            self.border(rect, rect.h / 2., 1., c(0xffffff, 0.07));
        } else {
            self.glass(rect, rect.h / 2.);
        }
    }
    /// Chrome over the wallpaper: its blurred copy, tinted, with a hairline border and a
    /// highlight along the top (spec, «Материалы»).
    fn glass(&mut self, rect: Rect, radius: f32) {
        self.glass_tinted(rect, radius, GLASS_TINT, 0.14);
    }
    /// Glass under a lighter tint or a stronger border, e.g. a focused chip.
    fn glass_tinted(&mut self, rect: Rect, radius: f32, tint: Color, border: f32) {
        self.frost(rect, radius);
        self.shape(rect, radius, tint);
        self.glass_sheen(rect, radius);
        self.glass_edge(rect, radius, border);
    }
    /// Light falling on the glass from above: brighter at the top, gone by the bottom.
    fn glass_sheen(&mut self, rect: Rect, radius: f32) {
        self.draw.push(Draw::RoundedGradient(
            rect,
            radius,
            c(0xffffff, GLASS_SHEEN),
            c(0xffffff, 0.),
        ));
    }
    /// An overlay's panel: glass under a heavier grey, so its text reads on any wallpaper.
    fn modal_glass(&mut self, rect: Rect, radius: f32) {
        self.frost(rect, radius);
        self.draw.push(Draw::RoundedGradient(
            rect,
            radius,
            c(0x2a2e33, 0.78),
            c(0x1c2025, 0.86),
        ));
        self.glass_sheen(rect, radius);
        self.glass_edge(rect, radius, 0.14);
    }
    /// The wallpaper's blurred copy under `rect`, layered as the wallpapers are.
    fn frost(&mut self, rect: Rect, radius: f32) {
        for (path, alpha) in &self.backdrop {
            self.draw.push(Draw::Glass(
                path.clone(),
                rect,
                radius,
                SCREEN,
                [1., 1., 1., *alpha],
            ));
        }
    }
    fn glass_edge(&mut self, rect: Rect, radius: f32, border: f32) {
        self.border(rect, radius, 1., c(0xffffff, border));
        // Along the straight part of the top edge, as light catching the glass's rim.
        let corner = radius.min(rect.w / 2.).min(rect.h / 2.);
        self.shape(
            Rect::new(rect.x + corner, rect.y, rect.w - 2. * corner, 1.),
            0.5,
            c(0xffffff, GLASS_RIM),
        );
    }
    fn fade(&mut self, from: usize, alpha: f32, brightness: f32) {
        for draw in &mut self.draw[from..] {
            let apply = |color: &mut Color| {
                for channel in &mut color[..3] {
                    *channel *= brightness;
                }
                color[3] *= alpha;
            };
            match draw {
                Draw::Shape(_, _, _, color)
                | Draw::Shadow(_, _, _, color)
                | Draw::Text(_, _, _, color, _)
                | Draw::TintedImage(_, _, _, _, color)
                | Draw::AppImage(_, _, _, color)
                | Draw::Picture(_, _, _, color)
                | Draw::Cover(_, _, _, color)
                | Draw::Glass(_, _, _, _, color) => apply(color),
                Draw::RoundedGradient(_, _, a, b) => {
                    apply(a);
                    apply(b);
                }
                Draw::Image(path, rect, radius, contain) => {
                    *draw = Draw::TintedImage(
                        path.clone(),
                        *rect,
                        *radius,
                        *contain,
                        [brightness, brightness, brightness, alpha],
                    );
                }
                Draw::Clip(_) | Draw::EdgeFade(_) | Draw::EdgeFadeX(_) => {}
            }
        }
    }
}

/// The strings a companion dialog shows around its QR code, as message keys.
struct Companion {
    title: &'static str,
    keep_open: &'static str,
    private: &'static str,
    error: &'static str,
    starting: &'static str,
}

fn wallpaper_upload(out: &mut Scene, state: &State, assets: &Assets) {
    companion(
        out,
        state,
        assets,
        state.wallpaper_upload.as_ref(),
        Companion {
            title: "uploadWallpaper",
            keep_open: "wallpaperUploadKeepOpen",
            private: "wallpaperUploadPrivate",
            error: "wallpaperUploadError",
            starting: "wallpaperUploadStarting",
        },
    );
}

fn tile_editor(out: &mut Scene, state: &State, assets: &Assets) {
    companion(
        out,
        state,
        assets,
        state.tile_editor.as_ref(),
        Companion {
            title: "editTilesElsewhere",
            keep_open: "tileEditorKeepOpen",
            private: "tileEditorPrivate",
            error: "tileEditorError",
            starting: "tileEditorStarting",
        },
    );
}

/// A QR code 336 pixels wide at most, centred on the screen from `y`, light on dark to sit in a
/// modal, with Luma's mark in its middle.
fn qr_code(out: &mut Scene, assets: &Assets, size: i32, qr: &[bool], y: f32) {
    let scale = (336. / (size + 8) as f32).floor();
    let side = (size + 8) as f32 * scale;
    let x = (1920. - side) / 2.;
    // Invert the normal QR polarity to blend into the dark modal, while
    // retaining a high-contrast foreground and the full quiet zone.
    let background = c(0x1b2023, 1.);
    let modules = c(0xf5f4ef, 1.);
    out.shape(Rect::new(x, y, side, side), 0., background);
    for row in 0..size {
        // Merge contiguous modules so QR rendering stays a few hundred quads.
        let mut column = 0;
        while column < size {
            let mark_start = (size - 11) / 2;
            let in_mark = (mark_start..mark_start + 11).contains(&row)
                && (mark_start..mark_start + 11).contains(&column);
            if in_mark {
                column += 1;
                continue;
            }
            if !qr[(row * size + column) as usize] {
                column += 1;
                continue;
            }
            let start = column;
            while column < size {
                let in_mark = (mark_start..mark_start + 11).contains(&row)
                    && (mark_start..mark_start + 11).contains(&column);
                if in_mark || !qr[(row * size + column) as usize] {
                    break;
                }
                column += 1;
            }
            out.shape(
                Rect::new(
                    x + (start + 4) as f32 * scale,
                    y + (row + 4) as f32 * scale,
                    (column - start) as f32 * scale,
                    scale,
                ),
                0.,
                modules,
            );
        }
    }
    let mark_start = (size - 11) / 2;
    let mark_plate = Rect::new(
        x + (mark_start + 4) as f32 * scale,
        y + (mark_start + 4) as f32 * scale,
        11. * scale,
        11. * scale,
    );
    out.shape(mark_plate, 1.1 * scale, background);
    let mark = Rect::new(
        mark_plate.x + 1.25 * scale,
        mark_plate.y + 1.25 * scale,
        8.5 * scale,
        8.5 * scale,
    );
    out.icon(assets, "luma-mark", mark, WHITE);
}

/// Signing in to CUB for Lampa's shows: the code to type at cub.red/activate, and a QR code
/// that opens the page with the code filled in. It closes itself once the user confirms.
fn cub_sign_in(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(520., 144., 880., 816.);
    out.modal = Some(panel);
    out.shadow(panel, 24., 40., 12., 0.3);
    out.modal_glass(panel, 24.);
    out.text(
        language.t("lampaSignInTitle"),
        Rect::new(584., 198., 752., 48.),
        text(32., 550, -0.02),
        WHITE,
        true,
    );
    let login = state
        .cub
        .login
        .as_ref()
        .and_then(|login| Some((login, crate::companion::qr(&login.link).ok()?)));
    match login {
        Some((login, (size, modules))) => {
            out.text(
                language.format("lampaSignInHint", &[("uri", &login.uri)]),
                Rect::new(604., 244., 712., 60.),
                text(19., 400, 0.).leading(1.4),
                c(0xffffff, 0.65),
                true,
            );
            qr_code(out, assets, size, &modules, 310.);
            out.text(
                &login.user_code,
                Rect::new(568., 672., 784., 64.),
                text(52., 650, 0.12),
                WHITE,
                true,
            );
            out.text(
                &login.uri,
                Rect::new(568., 748., 784., 32.),
                text(22., 500, 0.),
                c(0xffffff, 0.75),
                true,
            );
            out.text(
                language.t("lampaSignInPrivate"),
                Rect::new(604., 806., 712., 22.),
                text(15., 400, 0.),
                c(0xffffff, 0.5),
                true,
            );
        }
        None => {
            let key = if state.cub.signing_in {
                "lampaSignInStarting"
            } else {
                "lampaSignInError"
            };
            out.text(
                language.t(key),
                Rect::new(604., 470., 712., 90.),
                text(22., 450, 0.).leading(1.5),
                c(0xffffff, 0.75),
                true,
            );
        }
    }
    let close = Rect::new(812., 862., 296., 56.);
    out.pill(close, *state.page_focus() == Target::CloseDialog, false);
    out.text(
        language.t("close"),
        Rect::new(close.x, close.y + 15., close.w, 28.),
        text(20., 550, 0.),
        c(0x171c20, 1.),
        true,
    );
    out.control(Target::CloseDialog, close, false);
}

/// Why Lampa needs Luma's plugin and how to add it: its address, and a QR code that brings the
/// address to a phone, to paste with LG ThinQ's keyboard.
fn lampa_plugin(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(520., 144., 880., 816.);
    out.modal = Some(panel);
    out.shadow(panel, 24., 40., 12., 0.3);
    out.modal_glass(panel, 24.);
    out.text(
        language.t("lampaPlugin"),
        Rect::new(584., 198., 752., 48.),
        text(32., 550, -0.02),
        WHITE,
        true,
    );
    out.text(
        language.t("lampaPluginWhy"),
        Rect::new(584., 254., 752., 84.),
        text(19., 400, 0.).leading(1.4),
        c(0xffffff, 0.65),
        true,
    );
    let url = crate::shows::LAMPA_PLUGIN;
    if let Ok((size, modules)) = crate::companion::qr(url) {
        qr_code(out, assets, size, &modules, 352.);
    }
    out.text(
        url,
        Rect::new(568., 664., 784., 32.),
        assets.fit_text(url, text(22., 600, 0.), 784.),
        WHITE,
        true,
    );
    out.text(
        language.t("lampaPluginSteps"),
        Rect::new(584., 712., 752., 56.),
        text(19., 450, 0.).leading(1.4),
        c(0xffffff, 0.8),
        true,
    );
    out.text(
        language.t("lampaPluginPhone"),
        Rect::new(584., 784., 752., 48.),
        text(15., 400, 0.).leading(1.4),
        c(0xffffff, 0.5),
        true,
    );
    let close = Rect::new(812., 870., 296., 56.);
    out.pill(close, *state.page_focus() == Target::CloseDialog, false);
    out.text(
        language.t("close"),
        Rect::new(close.x, close.y + 15., close.w, 28.),
        text(20., 550, 0.),
        c(0x171c20, 1.),
        true,
    );
    out.control(Target::CloseDialog, close, false);
}

/// A QR code and address for a page on the owner's phone or computer, with a Close button.
fn companion(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    session: Option<&Result<crate::companion::Session, String>>,
    strings: Companion,
) {
    let language = state.preferences.language;
    let panel = Rect::new(520., 144., 880., 816.);
    out.modal = Some(panel);
    out.shadow(panel, 24., 40., 12., 0.3);
    out.modal_glass(panel, 24.);
    out.text(
        language.t(strings.title),
        Rect::new(584., 198., 752., 48.),
        text(32., 550, -0.02),
        WHITE,
        true,
    );
    out.text(
        language.t("wallpaperUploadHint"),
        Rect::new(604., 264., 712., 32.),
        text(19., 400, 0.).leading(1.5),
        c(0xffffff, 0.65),
        true,
    );
    match session {
        Some(Ok(session)) => {
            qr_code(out, assets, session.qr_size, &session.qr, 310.);
            out.text(
                language.t("wallpaperUploadBrowser"),
                Rect::new(604., 670., 712., 32.),
                text(19., 400, 0.).leading(1.5),
                c(0xffffff, 0.65),
                true,
            );
            let display_url = session.url.trim_end_matches('/');
            out.text(
                display_url,
                Rect::new(568., 716., 784., 36.),
                assets.fit_text(display_url, text(26., 600, 0.), 784.),
                WHITE,
                true,
            );
            out.text(
                language.t(strings.keep_open),
                Rect::new(604., 766., 712., 52.),
                text(16., 400, 0.).leading(1.4),
                c(0xffffff, 0.75),
                true,
            );
            out.text(
                language.t(strings.private),
                Rect::new(604., 820., 712., 22.),
                text(15., 400, 0.),
                c(0xffffff, 0.5),
                true,
            );
        }
        result => {
            let key = if result.is_some() {
                strings.error
            } else {
                strings.starting
            };
            out.text(
                language.t(key),
                Rect::new(604., 470., 712., 90.),
                text(22., 450, 0.).leading(1.5),
                c(0xffffff, 0.75),
                true,
            );
        }
    }
    let close = Rect::new(812., 862., 296., 56.);
    out.pill(close, *state.page_focus() == Target::CloseDialog, false);
    out.text(
        language.t("close"),
        Rect::new(close.x, close.y + 15., close.w, 28.),
        text(20., 550, 0.),
        c(0x171c20, 1.),
        true,
    );
    out.control(Target::CloseDialog, close, false);
}

/// Shadowed text is drawn opaque: through a translucent glyph its own shadow would show, greying
/// it. It takes the light grey of its white blended over a mid-tone, so secondary text stays a
/// step below white without turning dark.
fn solid(color: Color) -> Color {
    const BACKDROP: f32 = 120. / 255.;
    let [r, g, b, a] = color;
    let over = |value: f32| value * a + BACKDROP * (1. - a);
    [over(r), over(g), over(b), 1.]
}
fn c(rgb: u32, alpha: f32) -> Color {
    [
        ((rgb >> 16) & 255) as f32 / 255.,
        ((rgb >> 8) & 255) as f32 / 255.,
        (rgb & 255) as f32 / 255.,
        alpha,
    ]
}
/// A gallery card's picture, or Apple TV's night-sky placeholder until its first still arrives.
fn background_card(out: &mut Scene, assets: &Assets, state: &State, id: &str, rect: Rect) {
    match assets.background_thumbnail(state, id) {
        Some(path) => out.image(path, rect, 16., false),
        None => out.draw.push(Draw::RoundedGradient(
            rect,
            16.,
            c(0x23344a, 1.),
            c(0x0b1119, 1.),
        )),
    }
}

/// Apple TV's card says what it is and which place it is on, over a shade at its foot.
fn aerials_caption(out: &mut Scene, assets: &Assets, state: &State, rect: Rect) {
    let shade = Rect::new(rect.x, rect.y + rect.h * 0.42, rect.w, rect.h * 0.58);
    out.draw.push(Draw::RoundedGradient(
        shade,
        16.,
        c(0x000000, 0.),
        c(0x000000, 0.66),
    ));
    // Clear of the check mark a chosen card carries in its corner.
    let width = rect.w - 36. - 34.;
    let place = state.aerial_place().name(state.preferences.language);
    let label = assets.fit_text(crate::aerials::LABEL, text(20., 600, -0.01), width);
    out.text(
        crate::aerials::LABEL,
        Rect::new(rect.x + 18., rect.y + rect.h - 66., width, 26.),
        label,
        WHITE,
        false,
    );
    let name = assets.fit_text(place, text(16., 450, 0.), width);
    out.text(
        place,
        Rect::new(rect.x + 18., rect.y + rect.h - 38., width, 22.),
        name,
        c(0xffffff, 0.78),
        false,
    );
}

/// The whole screen, where the wallpaper is laid out.
const SCREEN: Rect = Rect::new(0., 0., 1920., 1080.);
/// Glass's tint over the wallpaper's blurred copy: `rgba(18,22,30,.46)`.
const GLASS_TINT: Color = [14. / 255., 17. / 255., 22. / 255., 0.24];
/// The sheen's opacity at the glass's top edge, and the rim light along it.
const GLASS_SHEEN: f32 = 0.10;
const GLASS_RIM: f32 = 0.34;

fn text(size: f32, weight: u16, tracking: f32) -> TextStyle {
    TextStyle::new(size).weight(weight).tracking(tracking)
}

/// Fully shown Recents is drawn instead of the page, which is then not part of the scene.
pub fn recents_replaces_page(state: &State) -> bool {
    state.recents_open() && state.recents.visibility == 1.
}

pub fn scene(state: &State, assets: &Assets, now: OffsetDateTime) -> Scene {
    scene_on(state, assets, now, Wallpapers::Fading)
}

/// The wallpaper Home fades in, while one fades.
pub fn fading_wallpaper(state: &State, assets: &Assets) -> Option<String> {
    state
        .previous_background
        .as_ref()
        .map(|_| assets.background(state, &state.background_key()))
}

pub fn scene_on(
    state: &State,
    assets: &Assets,
    now: OffsetDateTime,
    wallpapers: Wallpapers,
) -> Scene {
    let mut out = Scene::new();
    let key = state.background_key();
    let current = assets.background(state, &key);
    // Glass blurs the wallpaper as drawn, its gradient too, so a pane is no lighter than the
    // ground around it.
    let behind = |key: &str| assets.background(state, key);
    if let Some((previous, start)) = &state.previous_background
        && wallpapers == Wallpapers::Fading
    {
        out.image(assets.background(state, previous), SCREEN, 0., false);
        let alpha = if state.wallpaper_waiting {
            0.
        } else {
            (start.elapsed().as_secs_f32() / state.background_fade).min(1.)
        };
        out.draw.push(Draw::TintedImage(
            current,
            SCREEN,
            0.,
            false,
            [1., 1., 1., alpha],
        ));
        out.backdrop = vec![(behind(previous), 1.), (behind(&key), alpha)];
        out.wallpaper_fade = Some(alpha);
    } else if let Some((previous, _)) = &state.previous_background
        && wallpapers == Wallpapers::Previous
    {
        out.image(assets.background(state, previous), SCREEN, 0., false);
        out.backdrop = vec![(behind(previous), 1.)];
    } else {
        out.image(current, SCREEN, 0., false);
        out.backdrop = vec![(behind(&key), 1.)];
    }
    let content_start = out.draw.len();
    if recents_replaces_page(state) {
        out.shape(
            Rect::new(0., 0., 1920., 1080.),
            0.,
            c(0x080b12, RECENTS_BACKDROP_OPACITY),
        );
        recents_view::render(&mut out, state, assets);
        if let Some((notice, _)) = &state.notice {
            toast(&mut out, state, assets, notice);
        }
        return out;
    }
    let viewport = state.content_viewport();
    let main_bottom = viewport.y + viewport.h;
    if state.page == Page::Home {
        out.draw.push(Draw::Clip(Some(viewport)));
    }
    match state.page {
        Page::Home => home(&mut out, state, assets, now),
        Page::Apps => library(&mut out, state, assets),
        Page::Settings => settings(&mut out, state, assets),
    }
    if let Some(error) = &state.error
        && !state.inventory.apps.is_empty()
        && !out
            .controls
            .iter()
            .any(|control| control.target == Target::Retry)
    {
        let rect = Rect::new(84., main_bottom - 66., 1752., 54.);
        out.shape(rect, 11.2, c(0xad7c3e, 0.07));
        out.border(rect, 11.2, 1., c(0xedc596, 0.19));
        out.icon(
            assets,
            "alert",
            Rect::new(104., rect.y + 18., 18., 18.),
            c(0xf3ddbd, 1.),
        );
        out.text(
            error,
            Rect::new(138., rect.y + 16., 1500., 35.),
            text(14.4, 400, 0.),
            c(0xf3ddbd, 1.),
            false,
        );
        let retry = Rect::new(1720., rect.y + 8., 96., 38.);
        out.text(
            state.preferences.language.t("retry"),
            Rect::new(retry.x, retry.y + 10., retry.w, 20.),
            text(14.4, 500, 0.),
            c(0xf3ddbd, 1.),
            true,
        );
        if *state.page_focus() == Target::Retry {
            out.border(retry, 6.4, 2., WHITE);
        }
        out.control(Target::Retry, retry, false);
    }
    out.draw.push(Draw::EdgeFade(None));
    out.draw.push(Draw::Clip(None));
    let (draws, controls) = (out.draw.len(), out.controls.len());
    header(&mut out, state, assets, now);
    out.slide(draws, controls, state.chrome_offset());
    out.ground = out.draw.len();
    if let Some(
        overlay @ (Overlay::Options { .. }
        | Overlay::Delete { .. }
        | Overlay::BackgroundOptions { .. }
        | Overlay::BackgroundDelete { .. }
        | Overlay::Keyboard
        | Overlay::WallpaperUpload
        | Overlay::TileEditor
        | Overlay::CubSignIn
        | Overlay::LampaPlugin
        | Overlay::Cs2Match { .. }),
    ) = &state.overlay
    {
        let anchor = match overlay {
            Overlay::BackgroundOptions { id, .. } => out.controls.iter()
                .find(|control| matches!(control.target, Target::Background(index) if state.background_id(index) == Some(id.as_str())))
                .map(|control| control.rect),
            Overlay::Options { id, .. } => out
                .controls
                .iter()
                .find(|control| control.target == Target::App(id.clone()))
                .map(|control| {
                    // Hit regions include captions, artwork does not. Reusing the
                    // full hit region here stretches the lifted card vertically.
                    let mut rect = control.rect;
                    // Home names only its focused card, outside the hit region.
                    let names = state.page != Page::Home
                        && state.preferences.app_names == crate::model::AppNames::Always;
                    if names
                        && let Some(app) = state.inventory.apps.iter().find(|app| &app.id == id)
                    {
                        rect.h -= 18.4 + caption_height(state, assets, app, rect.w);
                    }
                    rect
                }),
            _ => None,
        };
        out.controls.clear();
        let start = out.draw.len();
        out.shape(Rect::new(0., 0., 1920., 1080.), 0., c(0x080b12, 0.70));
        if matches!(
            overlay,
            Overlay::Options { .. } | Overlay::BackgroundOptions { .. }
        ) && let Some(opened) = state.panel_started
        {
            // The lifted card stays opaque while the backdrop and popup fade.
            // Fading it over its dimmed parent copy causes a brightness dip.
            out.fade(start, (opened.elapsed().as_secs_f32() / 0.12).min(1.), 1.);
        }
        match overlay {
            Overlay::Options { id, .. } | Overlay::BackgroundOptions { id, .. } => {
                options(&mut out, state, assets, id, anchor)
            }
            Overlay::Delete { id, .. } | Overlay::BackgroundDelete { id, .. } => {
                delete_confirmation(&mut out, state, assets, id)
            }
            Overlay::Keyboard => keyboard(&mut out, state, assets),
            Overlay::WallpaperUpload => wallpaper_upload(&mut out, state, assets),
            Overlay::TileEditor => tile_editor(&mut out, state, assets),
            Overlay::CubSignIn => cub_sign_in(&mut out, state, assets),
            Overlay::LampaPlugin => lampa_plugin(&mut out, state, assets),
            Overlay::Cs2Match { id, .. } => cs2_dialog(&mut out, state, assets, id, now),
            Overlay::SystemKeyboard
            | Overlay::Recents { .. }
            | Overlay::Menu { .. }
            | Overlay::Today { .. } => {}
        }
        if !matches!(
            overlay,
            Overlay::Options { .. } | Overlay::BackgroundOptions { .. }
        ) && let Some(opened) = state.panel_started
        {
            out.fade(start, (opened.elapsed().as_secs_f32() / 0.12).min(1.), 1.);
        }
    }
    if matches!(state.overlay, Some(Overlay::Today { .. })) {
        let card = out
            .controls
            .iter()
            .find(|control| control.target == Target::TodayCard)
            .map(|control| control.rect);
        today_open(&mut out, state, assets, now, card);
    }
    menu(&mut out, state, assets, now);
    if state.recents_open() || state.recents.visibility > 0. {
        // Crossfade the actual parent interface as well as the dimmed stage.
        // Retain outgoing visuals after dismissal, while input immediately
        // belongs to the restored parent. No geometry or hit targets move.
        let parent_controls = std::mem::take(&mut out.controls);
        let parent_modal = out.modal;
        out.fade(content_start, 1. - state.recents.visibility, 1.);
        let start = out.draw.len();
        out.shape(
            Rect::new(0., 0., 1920., 1080.),
            0.,
            c(0x080b12, RECENTS_BACKDROP_OPACITY),
        );
        recents_view::render(&mut out, state, assets);
        out.fade(start, state.recents.visibility, 1.);
        // Dim the wallpaper beneath the outgoing interface rather than over it. While both
        // fade, the backdrop and Recents' full-screen gradient then lie directly on the
        // wallpaper, and the renderer folds them into it instead of two more full-screen
        // passes per frame of the crossfade.
        let fills = out.draw[start..]
            .iter()
            .take_while(|draw| full_screen_fill(draw))
            .count();
        let moved: Vec<_> = out.draw.drain(start..start + fills).collect();
        out.draw.splice(content_start..content_start, moved);
        // They now lie under the page, which still ends with its header.
        out.ground += fills;
        if !state.recents_open() {
            out.controls = parent_controls;
            out.modal = parent_modal;
        }
    }
    if let Some((notice, _)) = &state.notice {
        toast(&mut out, state, assets, notice);
    }
    out
}

fn header(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    let clock = language.clock(
        now,
        state.preferences.hour_24,
        state.preferences.show_seconds,
    );
    let k = state.preferences.ui_scale.scale();
    let style = text(24. * k, 500, -0.03).shadow();
    out.regions.push((
        Region::Clock,
        Rect::new(
            SAFE_X,
            76.8 - 14.4 * k,
            assets.width(&clock, style).min(300. * k),
            28.8 * k,
        ),
    ));
    let width = assets.width(&clock, style);
    out.text(
        clock,
        Rect::new(SAFE_X, 76.8 - 14.4 * k, 300. * k, 28.8 * k),
        style,
        WHITE,
        false,
    );
    // Home's date, on the clock's line.
    if state.page == Page::Home && state.preferences.show_date {
        let date = state.preferences.date_format.format(language, now);
        let x = SAFE_X + width + 18. * k;
        let rect = Rect::new(x, 76.8 - 14.4 * k, 1100. - x, 28.8 * k);
        out.regions.push((
            Region::Date,
            Rect::new(x, rect.y, assets.width(&date, style).min(rect.w), rect.h),
        ));
        out.text(date, rect, style, c(0xf5f5f5, 0.72), false);
    }
}

/// Where the menu stands when open (spec, «Меню»).
pub(crate) const MENU_PANEL: Rect = Rect::new(32., 32., 430., 1016.);
const MENU_ITEM: f32 = 76.;
const MENU_PITCH: f32 = 84.;
const MENU_INPUT: f32 = 64.;
const MENU_INPUT_PITCH: f32 = 68.;
/// The strip the panel and its shadow cross. The menu is drawn clipped to it, so its labels,
/// which would otherwise count as changed across the screen's full width, stay inside it.
const MENU_STRIP: Rect = Rect::new(0., 0., MENU_PANEL.x + MENU_PANEL.w + 48., 1080.);

/// The menu on the left. The page stays where it is, dimmed at once rather than step by step,
/// so while the panel slides only the strip it crosses is drawn again. While it is open only
/// its items take focus and clicks; a click beside it closes it.
fn menu(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let amount = state.menu_amount();
    if amount <= 0. {
        return;
    }
    let open = matches!(state.overlay, Some(Overlay::Menu { .. }));
    if open {
        out.controls.clear();
    }
    out.shape(SCREEN, 0., c(0x05080c, 0.5));
    out.draw.push(Draw::Clip(Some(MENU_STRIP)));
    let panel = Rect::new(
        MENU_PANEL.x - (MENU_PANEL.x + MENU_PANEL.w + 40.) * (1. - amount),
        MENU_PANEL.y,
        MENU_PANEL.w,
        MENU_PANEL.h,
    );
    out.shadow(panel, 36., 40., 12., 0.3 * amount);
    out.glass(panel, 36.);
    if open {
        out.modal = Some(panel);
    }
    let language = state.preferences.language;
    out.text(
        language.clock(now, state.preferences.hour_24, false),
        Rect::new(panel.x + 44., panel.y + 40., 300., 31.2),
        text(26., 500, -0.03),
        c(0xffffff, 0.75),
        false,
    );
    let x = panel.x + 22.;
    let width = panel.w - 44.;
    let mut y = panel.y + 118.;
    let current = state.page_index();
    for (index, icon) in [(0, "home"), (1, "grid")] {
        let label = language.t(["home", "apps"][index]);
        let rect = Rect::new(x, y, width, MENU_ITEM);
        menu_item(
            out,
            state,
            assets,
            Target::Nav(index),
            rect,
            icon,
            label,
            current == index,
        );
        y += MENU_PITCH;
    }
    let rect = Rect::new(x, y, width, MENU_ITEM);
    menu_item(
        out,
        state,
        assets,
        Target::MenuInputs,
        rect,
        "input-hdmi",
        language.t("inputs"),
        false,
    );
    let chevron = if state.menu_inputs { "down" } else { "right" };
    let tone = if open && state.focus == Target::MenuInputs {
        c(0x171c20, 0.7)
    } else {
        c(0xffffff, 0.6)
    };
    out.icon(
        assets,
        chevron,
        Rect::new(
            rect.x + rect.w - 48.,
            rect.y + (MENU_ITEM - 26.) / 2.,
            26.,
            26.,
        ),
        tone,
    );
    y += MENU_PITCH;
    if state.menu_inputs {
        for input in state
            .inventory
            .inputs
            .iter()
            .filter(|input| state.preferences.input_visible(input))
        {
            let target = Target::Input(input.id.clone());
            let row = Rect::new(x + 54., y, width - 54., MENU_INPUT);
            let focused = open && state.focus == target;
            if focused {
                out.shadow(row, 16., 18., 6., 0.25);
                out.draw.push(Draw::RoundedGradient(
                    row,
                    16.,
                    c(0xfafafa, 1.),
                    c(0xe5e5e5, 1.),
                ));
            }
            let (title_tone, status_tone) = if focused {
                (c(0x171c20, 1.), c(0x171c20, 0.62))
            } else {
                (c(0xffffff, 0.92), c(0xe8e8e8, 0.6))
            };
            let inner = row.w - 36.;
            let title = state.preferences.input_title(input);
            let title_style = text(26., 550, -0.01);
            out.text(
                title,
                Rect::new(row.x + 18., row.y + 6., inner, 31.2),
                assets.fit_text(title, title_style, inner),
                title_tone,
                false,
            );
            let name = state.preferences.input_name(input);
            let status = input_status(input, name.is_some(), language);
            let status_style = text(20., 450, 0.);
            out.text(
                &status,
                Rect::new(row.x + 18., row.y + 37., inner, 24.),
                assets.fit_text(&status, status_style, inner),
                status_tone,
                false,
            );
            if open {
                out.control(target, row, false);
            }
            y += MENU_INPUT_PITCH;
        }
    }
    let bottom = panel.y + panel.h - 40. - MENU_ITEM;
    menu_item(
        out,
        state,
        assets,
        Target::Nav(2),
        Rect::new(x, bottom - MENU_PITCH, width, MENU_ITEM),
        "settings",
        language.t("settings"),
        current == 2,
    );
    menu_item(
        out,
        state,
        assets,
        Target::TvSettings,
        Rect::new(x, bottom, width, MENU_ITEM),
        "tv",
        language.t("tvSettings"),
        false,
    );
    out.draw.push(Draw::Clip(None));
}

/// One of the menu's items: white when focused, faintly lit when it is the page in view.
#[allow(clippy::too_many_arguments)]
fn menu_item(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    target: Target,
    rect: Rect,
    icon: &str,
    label: &str,
    current: bool,
) {
    let focused = matches!(state.overlay, Some(Overlay::Menu { .. })) && state.focus == target;
    if focused {
        out.shadow(rect, 20., 18., 6., 0.25);
        out.draw.push(Draw::RoundedGradient(
            rect,
            20.,
            c(0xfafafa, 1.),
            c(0xe5e5e5, 1.),
        ));
    } else if current {
        out.shape(rect, 20., c(0xffffff, 0.1));
    }
    let tone = if focused {
        c(0x171c20, 1.)
    } else {
        c(0xffffff, 0.92)
    };
    out.icon(
        assets,
        icon,
        Rect::new(rect.x + 22., rect.y + (rect.h - 32.) / 2., 32., 32.),
        tone,
    );
    let width = rect.w - 76. - 56.;
    let style = text(31., 550, -0.01);
    out.text(
        label,
        Rect::new(rect.x + 76., rect.y + (rect.h - 37.2) / 2., width, 37.2),
        assets.fit_text(label, style, width),
        tone,
        false,
    );
    if matches!(state.overlay, Some(Overlay::Menu { .. })) {
        out.control(target, rect, false);
    }
}

/// Home's shared focus lift (`motion.rs`): a card grows and rises toward focus, with a soft glow.
/// Returns where to draw the card and how far it has lifted; its hit target stays at `rect`.
fn lift(out: &mut Scene, state: &State, target: &Target, rect: Rect, radius: f32) -> (Rect, f32) {
    let amount = state.motion.focus(target);
    let grow = rect.w.min(rect.h) * 0.07 * amount;
    let rect = Rect::new(
        rect.x - grow / 2.,
        rect.y - grow / 2. - 5. * amount,
        rect.w + grow,
        rect.h + grow,
    );
    out.shadow(
        rect,
        radius,
        4. + 30. * amount,
        1. + 13. * amount,
        0.01 + 0.39 * amount,
    );
    focus_glow(out, state, rect, radius, amount);
    (rect, amount)
}

/// The focused card's glow in the colour chosen in Settings › General: two inexpensive GPU
/// distance-field glows, with no texture blur or CPU readback.
fn focus_glow(out: &mut Scene, state: &State, rect: Rect, radius: f32, amount: f32) {
    let Some(rgb) = state.preferences.focus_glow.rgb().filter(|_| amount > 0.) else {
        return;
    };
    out.draw.push(Draw::Shadow(
        rect.inset(-3.),
        radius + 3.,
        32.,
        c(rgb, 0.2 * amount),
    ));
    out.draw
        .push(Draw::Shadow(rect, radius, 10., c(rgb, 0.24 * amount)));
}

/// The lifted card's edge, fading in with the lift.
fn lift_edge(out: &mut Scene, rect: Rect, radius: f32, amount: f32) {
    if amount > 0. {
        out.border(rect, radius, 1.4, c(0xffffff, 0.5 * amount));
    }
}

/// As on Apple TV, the focused card is named on one line below it, centred, as wide as a
/// column and its gutters; other cards go unnamed.
fn focused_name(out: &mut Scene, state: &State, assets: &Assets, app: &App, rect: Rect) {
    if *state.page_focus() != Target::App(app.id.clone()) {
        return;
    }
    let style = text(type_scale::CAPTION * caption_scale(state), 600, -0.01).shadow();
    let room = rect.w + 2. * GUTTER;
    out.text(
        &app.title,
        Rect::new(
            rect.x + (rect.w - room) / 2.,
            rect.y + rect.h + 14.,
            room,
            style.size * 1.2,
        ),
        assets.fit_text(&app.title, style, room),
        WHITE,
        true,
    );
}

fn tile(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    app: &App,
    rect: Rect,
    names: bool,
    pinned: bool,
) {
    let caption_height = if names {
        caption_height(state, assets, app, rect.w)
    } else {
        0.
    };
    let label_height = if names { 18.4 + caption_height } else { 0. };
    out.control(
        Target::App(app.id.clone()),
        Rect::new(rect.x, rect.y, rect.w, rect.h + label_height),
        true,
    );
    // Home's favorites are drawn where their scroll has eased to; hit targets stay put.
    let rect = if state.page == Page::Home {
        Rect::new(
            rect.x + state.home_scroll
                - state
                    .motion
                    .shown(crate::motion::Lane::Favorites, state.home_scroll),
            rect.y,
            rect.w,
            rect.h,
        )
    } else {
        rect
    };
    let viewport = if state.page == Page::Home {
        state.favorites_viewport()
    } else {
        state.content_viewport()
    };
    let top = if state.page == Page::Apps {
        0.
    } else {
        MAIN_TOP
    };
    if rect.y + rect.h + label_height < top
        || rect.y > 1054.
        || rect.x + rect.w + 40. < viewport.x
        || rect.x - 40. > viewport.x + viewport.w
    {
        return;
    }
    let start = out.draw.len();
    let focused = *state.page_focus() == Target::App(app.id.clone());
    // Home and Apps lift the focused card on the shared motion; previews keep the ring.
    let lifted = matches!(state.page, Page::Home | Page::Apps);
    let amount = if lifted {
        state.motion.focus(&Target::App(app.id.clone()))
    } else if focused {
        1.
    } else {
        0.
    };
    let base_rect = rect;
    // Keep hitboxes and navigation geometry fixed, including when using the Magic Remote.
    let rect = if lifted {
        let scale = 1. + 0.07 * amount;
        Rect::new(
            rect.x - rect.w * (scale - 1.) / 2.,
            rect.y - rect.h * (scale - 1.) / 2. - 5. * amount,
            rect.w * scale,
            rect.h * scale,
        )
    } else {
        rect
    };
    out.shadow(
        rect,
        19.2,
        22. + 12. * amount,
        9. + 5. * amount,
        0.165 + 0.235 * amount,
    );

    if lifted {
        focus_glow(out, state, rect, 19.2, amount);
    }
    let favorite_color = app_face(out, assets, app, rect, state.preferences.language, 19.2);

    if pinned {
        favorite_badge(out, assets, rect, favorite_color);
    }
    if state.moving.as_ref() == Some(&app.id) {
        let indicator = Rect::new(
            rect.x + rect.w / 2. - 37.6,
            rect.y + rect.h - 33.6,
            75.2,
            24.,
        );
        out.shape(indicator, 16., c(0x101318, 0.79));
        out.icon(
            assets,
            "left",
            Rect::new(indicator.x + 8.8, indicator.y + 4., 16., 16.),
            c(0xededed, 1.),
        );
        out.icon(
            assets,
            "arrow",
            Rect::new(indicator.x + 40., indicator.y + 4., 16., 16.),
            c(0xededed, 1.),
        );
    }
    if lifted {
        lift_edge(out, rect, 19.2, amount);
    } else if focused {
        out.ring(rect, 19.2);
    }
    if state.page == Page::Apps && state.filter == 5 {
        out.fade(start, 0.45, 1.);
    }
    if state.pressed.as_ref() == Some(&Target::App(app.id.clone())) {
        out.fade(start, 1., 0.85);
    }
    // Labels remain anchored so toggling focus does not shift text or neighbouring controls.
    let rect = base_rect;
    if names {
        out.text(
            format!(
                "{}{}",
                app.title,
                if state.filter == 5 {
                    format!(" · {}", state.preferences.language.t("hiddenLabel"))
                } else {
                    String::new()
                }
            ),
            Rect::new(
                rect.x + 4.,
                rect.y + rect.h + 18.4,
                rect.w - 8.,
                caption_height,
            ),
            text(
                20.8 * caption_scale(state),
                if focused { 600 } else { 550 },
                -0.02,
            )
            .leading(1.4)
            .shadow(),
            if focused {
                c(0xffffff, 1.)
            } else {
                c(0xececec, 0.66)
            },
            true,
        );
    }
    if state.moving.is_some() && state.moving.as_ref() != Some(&app.id) {
        out.fade(start, 0.48, 1.);
    }
}

fn favorite_badge(out: &mut Scene, assets: &Assets, card: Rect, color: Color) {
    let icon = Rect::new(card.x + card.w - 27.2, card.y + 11.2, 16., 16.);
    out.icon(assets, "heart-filled", icon, color);
}

/// Shared card artwork for Home, Apps and Recents. Curated vectors always take
/// precedence over installed artwork, with the same relative logo proportions.
fn app_face(
    out: &mut Scene,
    assets: &Assets,
    app: &App,
    rect: Rect,
    language: Language,
    radius: f32,
) -> Color {
    let logo = crate::branding::for_key(&app.brand);
    let artwork = logo
        .is_none()
        .then(|| {
            app.icon
                .as_ref()
                .and_then(|source| assets.artwork.get(source))
        })
        .flatten();
    // The heart's centre is 19.2px below the top, outside the inset installed icon.
    // Bundled and generic surfaces are dark, so their badge remains white.
    let favorite_color = if artwork
        .as_ref()
        .is_some_and(|image| image.dark_badge(19.2 / rect.h))
    {
        c(0x000000, 1.)
    } else {
        c(0xffffff, 1.)
    };
    let installed = artwork.is_some();
    if let Some(image) = artwork {
        out.draw.push(Draw::AppImage(image, rect, radius, [1.; 4]));
    } else {
        out.image(
            assets.path(format!("surfaces/{}.png", app.brand)),
            rect,
            radius,
            false,
        );
    }
    out.border(rect, radius, 1., c(0xffffff, 0.063));
    if let Some(brand) = logo {
        let (ratio, height) = brand.artwork_size();
        let height = height * rect.h / FACE_HEIGHT;
        out.image(
            assets.path(format!("logos/{}.lvec", app.brand)),
            Rect::new(
                rect.x + rect.w * (1. - ratio) / 2.,
                rect.y + (rect.h - height) / 2.,
                rect.w * ratio,
                height,
            ),
            0.,
            true,
        );
    } else if !installed {
        brand_mark(out, assets, app, rect, language);
    }
    favorite_color
}

fn caption_height(state: &State, assets: &Assets, app: &App, width: f32) -> f32 {
    let title = format!(
        "{}{}",
        app.title,
        if state.filter == 5 {
            format!(" · {}", state.preferences.language.t("hiddenLabel"))
        } else {
            String::new()
        }
    );
    let style = text(
        20.8 * caption_scale(state),
        if *state.page_focus() == Target::App(app.id.clone()) {
            600
        } else {
            550
        },
        -0.02,
    )
    .leading(1.4);
    (assets.lines(&title, style, width - 8.) as f32 * style.size * style.line_height)
        .max(58.24 * caption_scale(state))
}

fn caption_scale(state: &State) -> f32 {
    state.preferences.ui_scale.scale()
}

fn brand_mark(out: &mut Scene, assets: &Assets, app: &App, rect: Rect, language: Language) {
    let scale = rect.h / FACE_HEIGHT;
    let rect = Rect::new(
        rect.x / scale,
        rect.y / scale,
        rect.w / scale,
        rect.h / scale,
    );
    let start = out.draw.len();
    let (cx, cy) = rect.center();
    match app.brand.as_str() {
        "plex" => {
            out.text(
                "ple",
                Rect::new(cx - 63., cy - 35.2, 130., 70.4),
                text(70.4, 650, -0.08).leading(1.),
                WHITE,
                false,
            );
            out.text(
                "x",
                Rect::new(cx + 20., cy - 35.2, 60., 70.4),
                text(70.4, 800, -0.08).leading(1.),
                c(0xe6aa25, 1.),
                false,
            );
        }
        "spotify" => {
            out.icon(
                assets,
                "spotify",
                Rect::new(cx - 77., cy - 20., 40., 40.),
                c(0x5ae08c, 1.),
            );
            out.text(
                "Spotify",
                Rect::new(cx - 29., cy - 14.4, 120., 36.),
                text(28.8, 650, -0.04).leading(1.),
                c(0x5ae08c, 1.),
                false,
            );
        }
        "kodi" => {
            out.icon(
                assets,
                "kodi",
                Rect::new(cx - 76., cy - 22.5, 45., 45.),
                WHITE,
            );
            out.text(
                "kodi",
                Rect::new(cx - 18., cy - 24.8, 130., 60.),
                text(49.6, 350, -0.04).leading(1.),
                WHITE,
                false,
            );
        }
        brand => {
            let (icon, label) = match brand {
                "live-tv" => ("tv", language.t("liveTV")),
                "homebrew" => ("coffee", language.t("homebrew")),
                "browser" => ("globe", language.t("browser")),
                "moonlight" => ("moonlight", "Moonlight"),
                "gaming" => ("gaming", app.title.as_str()),
                "custom" => (app.mark, app.title.as_str()),
                _ => ("generic", app.title.as_str()),
            };
            let font = if brand == "generic" {
                text(18.4, 500, -0.035).leading(1.35)
            } else {
                text(24., 500, -0.035).leading(1.2)
            };
            let w = (assets.width(label, font) + 54.4).min(rect.w - 32.);
            let label_w = rect.w - 54.4 - 32.;
            let label_h =
                (assets.lines(label, font, label_w) as f32 * font.size * font.line_height)
                    .min(rect.h - 32.);
            out.icon(
                assets,
                icon,
                Rect::new(cx - w / 2., cy - 20.8, 41.6, 41.6),
                WHITE,
            );
            out.text(
                label,
                Rect::new(cx - w / 2. + 54.4, cy - label_h / 2., label_w, label_h),
                font,
                WHITE,
                false,
            );
        }
    }
    if scale != 1. {
        for draw in &mut out.draw[start..] {
            let rect = match draw {
                Draw::Text(_, rect, style, _, _) => {
                    style.size *= scale;
                    rect
                }
                Draw::TintedImage(_, rect, _, _, _) => rect,
                _ => continue,
            };
            *rect = Rect::new(
                rect.x * scale,
                rect.y * scale,
                rect.w * scale,
                rect.h * scale,
            );
        }
    }
}

/// Where the card beside the greeting sits: the header's right columns, from the fifth's gutter.
/// It starts on the clock's line, its heading level with the clock, and grows down.
pub(crate) const TODAY_RECT: Rect = Rect::new(1160., 45., 680., HEADER_BOTTOM - 45.);
/// The greeting's top, under the clock's line.
const GREETING_TOP: f32 = 118.;
/// A row's heading, then its cards this far below the heading's top.
const ROW_HEADING_GAP: f32 = 50.;
/// Room under the favorites for the focused card's name.
const CAPTION_ROOM: f32 = 40.;
const HOME_FACE_HEIGHT: f32 = 150.;

/// A Home row's heading, straight on the wallpaper.
fn row_heading(out: &mut Scene, label: &str, y: f32) -> TextStyle {
    let style = text(type_scale::ROW_TITLE, 600, -0.02).shadow();
    out.text(
        label,
        Rect::new(SAFE_X, y, 1000., type_scale::ROW_TITLE * 1.2),
        style,
        WHITE,
        false,
    );
    style
}

fn home(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    let s = state.page_scroll();
    // The header's left column yields to the card beside it.
    let today = today_lines(state, now);
    let column = if !today.lines.is_empty() {
        TODAY_RECT.x - GUTTER - SAFE_X
    } else {
        WIDTH
    };
    let summary = state
        .preferences
        .show_weather
        .then(|| weather_summary_layout(state, assets, column));
    // The date is on the clock's line.
    let date_height = 0.;
    let greeting_line = (type_scale::GREETING * 1.05).ceil();
    let greeting_height = if state.preferences.show_greeting {
        greeting_line + 16.
    } else {
        0.
    };
    let block = date_height + greeting_height + summary.as_ref().map_or(0., |s| s.height);
    // Just under the clock's line, and never past the header's bottom onto the center.
    let top = GREETING_TOP.min(HEADER_BOTTOM - block) - s;
    if state.preferences.show_greeting {
        let greeting = language.greeting(now.hour(), &state.preferences.greeting_name);
        let style = text(type_scale::GREETING, 600, -0.04)
            .leading(1.05)
            .shadow();
        let fitted = assets.fit_text(&greeting, style, column);
        let height = greeting_line * fitted.size / style.size;
        out.regions.push((
            Region::Greeting,
            Rect::new(
                SAFE_X,
                top + date_height + (greeting_line - height) / 2.,
                assets.width(&greeting, fitted).min(column),
                height,
            ),
        ));
        out.text(
            &greeting,
            Rect::new(
                SAFE_X,
                top + date_height + (greeting_line - height) / 2.,
                column,
                height,
            ),
            fitted,
            WHITE,
            false,
        );
    }
    if let Some(summary) = summary {
        out.regions.push((
            Region::Weather,
            Rect::new(
                SAFE_X,
                top + date_height + greeting_height,
                summary.width,
                summary.height,
            ),
        ));
        weather_summary(
            out,
            state,
            assets,
            summary,
            top + date_height + greeting_height,
        );
    }
    let apps = state.apps();
    let names = state.preferences.show_home_names;
    let placeholders = state.loading && state.inventory.apps.is_empty();
    // The favorites stand at the foot of the screen, with room below for the focused name.
    let cards_y = if apps.is_empty() && !placeholders {
        HOME_BOTTOM - 247.5
    } else {
        HOME_BOTTOM - HOME_FACE_HEIGHT - CAPTION_ROOM
    } - s;
    let heading_y = cards_y - ROW_HEADING_GAP;
    row_heading(out, language.t("favorites"), heading_y);
    if state.moving.is_some() {
        out.text(
            language.t("moveApps"),
            Rect::new(SAFE_X + WIDTH - 600., heading_y + 4., 600., 28.),
            text(type_scale::CAPTION, 450, 0.).right().shadow(),
            c(0xdedede, 1.),
            false,
        );
    }
    if placeholders {
        for index in 0..6 {
            out.shape(
                Rect::new(
                    SAFE_X + index as f32 * HOME_TILE_PITCH,
                    cards_y,
                    HOME_TILE_WIDTH,
                    HOME_FACE_HEIGHT,
                ),
                19.2,
                c(0xffffff, 0.06),
            );
        }
    } else if apps.is_empty() {
        let error = state.error.is_some() && state.inventory.apps.is_empty();
        empty(
            out,
            state,
            assets,
            EmptyState {
                rect: Rect::new(SAFE_X, cards_y - 11.2, WIDTH, 258.7),
                icon: if error { "alert" } else { "heart" },
                title: if error {
                    language.t("aLittleHelp")
                } else {
                    language.t("makeFavorites")
                },
                hint: if error {
                    state.error.as_deref().unwrap_or("")
                } else {
                    language.t("pinHint")
                },
                action: Some(if error { Target::Retry } else { Target::Browse }),
            },
        );
    }
    let shown = apps.len().min(6) as f32;
    if shown > 0. {
        out.regions.push((
            Region::Favorites,
            Rect::new(
                SAFE_X,
                cards_y,
                shown * HOME_TILE_PITCH - (HOME_TILE_PITCH - HOME_TILE_WIDTH),
                HOME_FACE_HEIGHT,
            ),
        ));
    }
    // Like Apple TV's rows, the favourites run to the screen edge: the next tile shows cut off
    // at the right (and a scrolled-away one at the left), so it is clear the row continues.
    out.draw.push(Draw::Clip(Some(state.favorites_viewport())));
    // Soft edges: out to the right from the end of the sixth tile, and in from the left once
    // the row has scrolled.
    let left = if state
        .motion
        .shown(crate::motion::Lane::Favorites, state.home_scroll)
        > 0.
    {
        [HOME_FADE_LEFT, HOME_STRIP_LEFT]
    } else {
        [0., 0.]
    };
    out.draw.push(Draw::EdgeFadeX(Some([
        left[0],
        left[1],
        HOME_STRIP_RIGHT,
        HOME_FADE_RIGHT,
    ])));
    for (index, app) in apps.iter().enumerate() {
        let rect = Rect::new(
            SAFE_X + index as f32 * HOME_TILE_PITCH - state.home_scroll,
            cards_y,
            HOME_TILE_WIDTH,
            HOME_FACE_HEIGHT,
        );
        tile(out, state, assets, app, rect, false, false);
        if names {
            let x = rect.x + state.home_scroll
                - state
                    .motion
                    .shown(crate::motion::Lane::Favorites, state.home_scroll);
            focused_name(
                out,
                state,
                assets,
                app,
                Rect::new(x, rect.y, rect.w, rect.h),
            );
        }
    }
    out.draw.push(Draw::EdgeFadeX(None));
    out.draw.push(Draw::Clip(Some(state.content_viewport())));
    if let Some(card) = today_card(out, state, assets, &today, now) {
        use crate::today::Line;
        for (region, kind) in [
            (Region::Crypto, 0),
            (Region::Cs2, 1),
            (Region::Shows, 2),
            (Region::Apps, 3),
        ] {
            if today.lines.iter().any(|line| {
                matches!(
                    (kind, line),
                    (0, Line::Crypto)
                        | (1, Line::Match { .. })
                        | (2, Line::Episode { .. })
                        | (3, Line::App { .. })
                )
            }) {
                out.regions.push((region, card));
            }
        }
    }
    // One widget in the middle, between the header and the favorites, or none.
    use crate::model::Center;
    let Some(center) = state.preferences.center() else {
        return;
    };
    let (region, height) = match center {
        Center::Cs2 => (Region::Cs2, CS2_CARD_HEIGHT),
        Center::Shows => (Region::Shows, SHOW_POSTER_HEIGHT + SHOW_CAPTION),
        Center::Apps => (Region::Apps, SHOW_POSTER_HEIGHT + SHOW_CAPTION),
    };
    let block = ROW_HEADING_GAP + height;
    let y = HEADER_BOTTOM - s + ((heading_y + s - HEADER_BOTTOM - block) / 2.).max(0.);
    out.regions.push((
        region,
        Rect::new(SAFE_X, y + ROW_HEADING_GAP, WIDTH, height),
    ));
    match center {
        Center::Cs2 => cs2_row(out, state, assets, y, now),
        Center::Shows => shows_row(out, state, assets, y, now.date()),
        Center::Apps => apps_row(out, state, assets, y),
    }
}

/// Show cards in the middle of Home: posters with a 2:3 face, smaller than the favorites so
/// the row fits between the header and them.
const SHOW_WIDTH: f32 = 200.;
const SHOW_POSTER_HEIGHT: f32 = SHOW_WIDTH * 1.5;
const SHOW_PITCH: f32 = 240.;
/// The title and details below a poster.
const SHOW_CAPTION: f32 = 60.;

/// Home's Lampa row: a heading naming what it lists and crediting Lampa, then posters in a row
/// that slides six at a time.
fn shows_row(out: &mut Scene, state: &State, assets: &Assets, y: f32, date: time::Date) {
    use crate::model::LampaFeed;
    let language = state.preferences.language;
    let feed = state.preferences.lampa_feed;
    let heading = language.t(feed.key());
    let style = row_heading(out, heading, y);
    // Outside Russian the posters are TVmaze's, which asks to be credited.
    let credit = if language == crate::i18n::Language::Ru {
        "Lampa"
    } else {
        "Lampa · TVmaze"
    };
    let credit_x = SAFE_X + assets.width(heading, style) + 20.;
    let caption = text(type_scale::CAPTION, 400, 0.).shadow();
    out.text(
        credit,
        Rect::new(credit_x, y + 5., 600., 27.),
        caption,
        c(0xf5f5f5, 0.5),
        false,
    );
    // The red key's hint: the remote's one-dot red button and the collection it turns to.
    let hint = language.format(
        "lampaNextFeed",
        &[("feed", language.t(state.lampa_feed_after().key()))],
    );
    let dot_x = credit_x + assets.width(credit, caption) + 28.;
    let dot = 12.;
    out.shape(
        Rect::new(dot_x, y + 5. + (27. - dot) / 2., dot, dot),
        dot / 2.,
        c(0xe5484d, 0.95),
    );
    out.text(
        &hint,
        Rect::new(dot_x + dot + 8., y + 5., 700., 27.),
        caption,
        c(0xf5f5f5, 0.5),
        false,
    );
    let cards = lampa_row(state, date);
    if cards.is_empty() {
        let cub = &state.cub;
        let notice = match feed {
            LampaFeed::Watching if state.shows.failed => "showsUnavailable",
            LampaFeed::Watching => "showsLoading",
            LampaFeed::Continue if !cub.signed_in() => "lampaNeedsAccount",
            LampaFeed::Continue if cub.failed => "lampaUnavailable",
            LampaFeed::Continue if cub.fetched.is_none() => "showsLoading",
            LampaFeed::Continue => "lampaNoHistory",
            _ if state.shows.failed => "lampaUnavailable",
            _ => "showsLoading",
        };
        out.text(
            language.t(notice),
            Rect::new(SAFE_X, y + ROW_HEADING_GAP, 1200., 29.),
            text(type_scale::SECONDARY, 400, 0.).shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
        return;
    }
    let visible = crate::shows::VISIBLE;
    let first = state.shows_first.min(cards.len().saturating_sub(visible));
    if cards.len() > visible {
        out.text(
            format!(
                "{}–{} / {}",
                first + 1,
                (first + visible).min(cards.len()),
                cards.len()
            ),
            Rect::new(SAFE_X + WIDTH - 400., y + 5., 400., 27.),
            text(type_scale::CAPTION, 500, 0.02).right().shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
    }
    carousel(
        out,
        state,
        Carousel {
            lane: crate::motion::Lane::Shows,
            first: state.shows_first,
            count: cards.len(),
            visible,
            pitch: SHOW_PITCH,
            card: Rect::new(SAFE_X, y + ROW_HEADING_GAP, SHOW_WIDTH, SHOW_POSTER_HEIGHT),
            below: SHOW_CAPTION,
        },
        |out, index, rect, hit| show_card(out, assets, state, &cards[index], index, rect, hit),
    );
}

/// What a card of the Lampa row shows: its poster, title, and a line of details, accented when
/// it tells of a new episode.
struct LampaCard {
    poster: Option<std::sync::Arc<crate::app_artwork::Artwork>>,
    title: String,
    meta: String,
    new: bool,
    /// For a card an app published (`app_cards.rs`): the app, whose tile marks the card, and
    /// whose 16:9 picture `poster` then is.
    app: Option<crate::model::App>,
}

/// The Lampa row's cards, in the order of `State::lampa_cards`; `date` is the TV's.
fn lampa_row(state: &State, date: time::Date) -> Vec<LampaCard> {
    let language = state.preferences.language;
    match state.preferences.lampa_feed {
        crate::model::LampaFeed::Continue if state.cub.signed_in() => state
            .cub
            .watched
            .iter()
            .take(crate::shows::ROW_CARDS)
            .map(|watched| LampaCard {
                poster: watched
                    .poster_key(language)
                    .and_then(|key| state.cub.poster(&key)),
                title: watched.title.clone(),
                meta: watched
                    .episode
                    .map(|(season, episode)| format!("S{season} E{episode}"))
                    .unwrap_or_else(|| language.t("lampaMovie").to_owned()),
                new: false,
                app: None,
            })
            .collect(),
        crate::model::LampaFeed::Continue => Vec::new(),
        feed => state
            .shows
            .list
            .iter()
            .take(crate::shows::ROW_CARDS)
            .map(|show| {
                // The season, then today's news or the rating; a film's year and rating.
                let new =
                    feed == crate::model::LampaFeed::Watching && !show.movie && show.new_on(date);
                let news = if new {
                    Some(language.t("lampaNew").to_owned())
                } else {
                    show.rating.map(|rating| format!("★ {rating:.1}"))
                };
                LampaCard {
                    poster: state.shows.poster(show, state.preferences.language),
                    title: show.title(language).to_owned(),
                    meta: [
                        show.season.map(|season| format!("S{season}")),
                        show.movie.then(|| show.aired.year().to_string()),
                        news,
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · "),
                    new,
                    app: None,
                }
            })
            .collect(),
    }
}

/// Home's Apps row: the cards the TV's apps publish, as posters, sliding six at a time.
fn apps_row(out: &mut Scene, state: &State, assets: &Assets, y: f32) {
    let language = state.preferences.language;
    row_heading(out, language.t("appsWidget"), y);
    let cards = apps_cards(state);
    if cards.is_empty() {
        out.text(
            language.t("appsNone"),
            Rect::new(SAFE_X, y + ROW_HEADING_GAP, 1400., 29.),
            text(type_scale::SECONDARY, 400, 0.).shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
        return;
    }
    let visible = crate::shows::VISIBLE;
    let first = state.shows_first.min(cards.len().saturating_sub(visible));
    if cards.len() > visible {
        out.text(
            format!(
                "{}–{} / {}",
                first + 1,
                (first + visible).min(cards.len()),
                cards.len()
            ),
            Rect::new(SAFE_X + WIDTH - 400., y + 5., 400., 27.),
            text(type_scale::CAPTION, 500, 0.02).right().shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
    }
    carousel(
        out,
        state,
        Carousel {
            lane: crate::motion::Lane::Shows,
            first: state.shows_first,
            count: cards.len(),
            visible,
            pitch: SHOW_PITCH,
            card: Rect::new(SAFE_X, y + ROW_HEADING_GAP, SHOW_WIDTH, SHOW_POSTER_HEIGHT),
            below: SHOW_CAPTION,
        },
        |out, index, rect, hit| show_card(out, assets, state, &cards[index], index, rect, hit),
    );
}

/// Widgets › Apps: whether it shows and where, then a switch for each app that publishes cards.
fn apps_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    let apps: Vec<(String, bool)> = state
        .app_cards
        .apps()
        .into_iter()
        .map(|id| {
            let on = !state.preferences.apps_off.contains(&id);
            (
                language.format("appsCards", &[("app", &app_of(state, &id).title)]),
                on,
            )
        })
        .collect();
    let mut rows: Vec<(Target, &str, &str, Option<&str>)> = vec![
        (
            Target::AppsToggle,
            language.t("showWidget"),
            "eye",
            Some(language.t(if state.preferences.show_apps {
                "stateOn"
            } else {
                "stateOff"
            })),
        ),
        (
            Target::AppsLayout,
            language.t("cryptoLayout"),
            "grid",
            Some(place(language, state, crate::model::Center::Apps)),
        ),
    ];
    rows.extend(apps.iter().enumerate().map(|(index, (title, on))| {
        (
            Target::AppsApp(index),
            title.as_str(),
            "tv",
            Some(language.t(if *on { "stateOn" } else { "stateOff" })),
        )
    }));
    let count = rows.len() as f32;
    for (index, (target, title, icon, value)) in rows.into_iter().enumerate() {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - s,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title,
                icon: Some(icon),
                value,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
    out.text(
        language.t("appsSource"),
        Rect::new(
            LIST_X + 24.,
            LIST_TOP + count * LIST_PITCH + 6. - s,
            LIST_WIDTH - 48.,
            120.,
        ),
        text(type_scale::CAPTION, 450, 0.).leading(1.4).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
}

/// An app card's picture on a Today line, cropped to the square; the app's tile until it loads.
fn app_line_picture(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    card: &crate::app_cards::AppCard,
    rect: Rect,
    radius: f32,
) {
    match card
        .image
        .as_deref()
        .and_then(|url| state.app_cards.image(url))
    {
        Some(picture) => out.draw.push(Draw::Cover(picture, rect, radius, [1.; 4])),
        None => {
            let app = app_of(state, &card.app);
            let tile = Rect::new(rect.x, rect.y + rect.h * 0.21875, rect.w, rect.w * 9. / 16.);
            app_face(
                out,
                assets,
                &app,
                tile,
                state.preferences.language,
                radius * 0.6,
            );
        }
    }
}

/// The Apps widget's cards: what each app published, as posters, each app's in turn.
fn apps_cards(state: &State) -> Vec<LampaCard> {
    state
        .shown_app_cards()
        .into_iter()
        .map(|card| {
            let app = app_of(state, &card.app);
            LampaCard {
                poster: card
                    .image
                    .as_deref()
                    .and_then(|url| state.app_cards.image(url)),
                title: card.title.clone(),
                meta: app.title.clone(),
                new: false,
                app: Some(app),
            }
        })
        .collect()
}

/// The installed app `id`, or a stand-in named after its ID when the inventory lacks it.
fn app_of(state: &State, id: &str) -> crate::model::App {
    state
        .inventory
        .apps
        .iter()
        .find(|app| app.id == id)
        .cloned()
        .unwrap_or_else(|| crate::model::App::new(id, &app_name(id)))
}

/// A readable name from an app ID: "youtube.leanback.v4" gives "Youtube".
fn app_name(id: &str) -> String {
    let word = id
        .split('.')
        .find(|part| !matches!(*part, "com" | "tv" | "app" | "webos" | "lge"))
        .unwrap_or(id);
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The face of a card an app published: its picture over the whole face, as Lampa's posters
/// are; until it loads, the app's tile in the middle.
fn app_card_face(
    out: &mut Scene,
    assets: &Assets,
    state: &State,
    picture: Option<std::sync::Arc<crate::app_artwork::Artwork>>,
    app: &crate::model::App,
    face: Rect,
) {
    if let Some(picture) = picture {
        out.draw.push(Draw::Cover(picture, face, 18.4, [1.; 4]));
        return;
    }
    let scale = face.w / SHOW_WIDTH;
    out.draw.push(Draw::RoundedGradient(
        face,
        18.4,
        c(0x262c33, 0.8),
        c(0x171c22, 0.8),
    ));
    let tile_w = 120. * scale;
    let tile = Rect::new(
        face.x + (face.w - tile_w) / 2.,
        face.y + (face.h - tile_w * 9. / 16.) / 2.,
        tile_w,
        tile_w * 9. / 16.,
    );
    app_face(
        out,
        assets,
        app,
        tile,
        state.preferences.language,
        10. * scale,
    );
}

/// A card of the Lampa row: its poster, then the title and its details below.
fn show_card(
    out: &mut Scene,
    assets: &Assets,
    state: &State,
    card: &LampaCard,
    index: usize,
    rect: Rect,
    hit: Option<Rect>,
) {
    let target = Target::Show(index);
    let (face, lift_amount) = lift(out, state, &target, rect, 18.4);
    if let Some(app) = &card.app {
        app_card_face(out, assets, state, card.poster.clone(), app, face);
    } else {
        match &card.poster {
            Some(poster) => out
                .draw
                .push(Draw::Cover(poster.clone(), face, 18.4, [1.; 4])),
            None => {
                out.draw.push(Draw::RoundedGradient(
                    face,
                    18.4,
                    c(0x262c33, 0.8),
                    c(0x171c22, 0.8),
                ));
                out.icon(
                    assets,
                    "tv",
                    Rect::new(
                        face.x + face.w / 2. - 22.,
                        face.y + face.h / 2. - 22.,
                        44.,
                        44.,
                    ),
                    c(0xf5f5f5, 0.35),
                );
            }
        }
    }
    lift_edge(out, face, 18.4, lift_amount);
    let style = text(19.2, 550, -0.01);
    let title = assets.ellipsize(&card.title, style, rect.w);
    let below = rect.y + rect.h + 12.;
    out.text(
        &title,
        Rect::new(rect.x, below, rect.w, 26.),
        style,
        WHITE,
        false,
    );
    out.text(
        &card.meta,
        Rect::new(rect.x, below + 29., rect.w, 20.),
        assets.fit_text(&card.meta, text(14.4, 400, 0.), rect.w),
        if card.new {
            c(0x9fd0ff, 0.95)
        } else {
            c(0xf5f5f5, 0.6)
        },
        false,
    );
    if let Some(hit) = hit {
        out.control(target, hit, true);
    }
}

/// Widgets › Lampa: whether it shows and where, what it lists, and the CUB account its
/// history comes from; below, where that comes from and how fresh it is.
fn shows_settings(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::model::LampaFeed;
    let language = state.preferences.language;
    let s = state.scroll;
    let cub = &state.cub;
    let feed = state.preferences.lampa_feed;
    let name = cub.profile.as_ref().map(|profile| profile.name.as_str());
    let rows = [
        (
            Target::ShowsToggle,
            language.t("showWidget"),
            "eye",
            Some(language.t(if state.preferences.show_shows {
                "stateOn"
            } else {
                "stateOff"
            })),
            false,
        ),
        (
            Target::ShowsLayout,
            language.t("cryptoLayout"),
            "grid",
            Some(place(language, state, crate::model::Center::Shows)),
            false,
        ),
        (
            Target::LampaFeed,
            language.t("lampaFeed"),
            "tv",
            Some(language.t(feed.key())),
            true,
        ),
        (
            Target::LampaAccount,
            language.t(if cub.signed_in() {
                "lampaSignOut"
            } else {
                "lampaSignIn"
            }),
            "power",
            name.filter(|_| cub.signed_in()),
            !cub.signed_in(),
        ),
        (
            Target::LampaPlugin,
            language.t("lampaPlugin"),
            "upload",
            Some(language.t("lampaPluginHow")),
            true,
        ),
    ];
    let count = rows.len() as f32;
    for (index, (target, title, icon, value, chevron)) in rows.into_iter().enumerate() {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - s,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title,
                icon: Some(icon),
                value,
                description: None,
                selected: false,
                chevron,
            },
        );
    }
    let status = language.t(match feed {
        LampaFeed::Watching => "showsSource",
        LampaFeed::Continue if cub.signed_in() && cub.failed => "lampaUnavailable",
        LampaFeed::Continue => "lampaSource",
        _ => "lampaCatalogueSource",
    });
    out.text(
        status,
        Rect::new(
            LIST_X + 24.,
            LIST_TOP + count * LIST_PITCH + 6. - s,
            LIST_WIDTH - 48.,
            120.,
        ),
        text(type_scale::CAPTION, 450, 0.).leading(1.4).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
}

/// Where a widget sits on Home: in the Today card, or in the middle.
fn place(language: Language, state: &State, widget: crate::model::Center) -> &'static str {
    let center = state.preferences.center() == Some(widget);
    language.t(if center { "placeCenter" } else { "placeToday" })
}

/// The Today card's lines for this moment, on the TV's local day.
fn today_lines(state: &State, now: OffsetDateTime) -> crate::today::Today {
    crate::today::today(state, now.unix_timestamp(), now.date())
}

/// A Lampa line's poster (by key), or the TV icon until it loads.
fn lampa_poster(out: &mut Scene, state: &State, assets: &Assets, key: Option<String>, icon: Rect) {
    match key.and_then(|key| state.cub.poster(&key)) {
        Some(poster) => out.draw.push(Draw::Cover(poster, icon, 8., [1.; 4])),
        None => out.icon(assets, "tv", icon.inset(6.), c(0xf5f5f5, 0.7)),
    }
}

const TODAY_PAD: f32 = 20.;
const TODAY_EYEBROW: f32 = 28.;
const TODAY_LINE: f32 = 56.;

/// The gap between two coins of the ticker, and the width over which it fades at its ends.
const TICKER_GAP: f32 = 12.;
/// A coin's dark plate, round its mark on the left and its change's pill on the right.
const TICKER_PLATE_HEIGHT: f32 = 44.;
const TICKER_PLATE_LEAD: f32 = 9.;
const TICKER_PLATE_END: f32 = 4.;
const TICKER_FADE: f32 = 32.;
/// A coin's mark before its symbol on the ticker.
const TICKER_MARK: f32 = 26.;

/// A coin on the ticker: its mark and the mark's colour, its coloured labels with their widths,
/// the change for its pill, and its whole width.
struct TickerCoin {
    mark: (&'static str, u32),
    parts: Vec<(String, Color, f32)>,
    change: Option<(String, f64)>,
    width: f32,
}

/// The Today card's prices on one `line`: the coin's mark, the symbol white, the price quieter, the change in
/// its tinted pill. Coins wider than the line run across it without a seam.
/// Returns the length of one lap in whole pixels, or 0 when the coins stand still.
fn price_ticker(out: &mut Scene, state: &State, assets: &Assets, line: Rect) -> f32 {
    let style = text(type_scale::CAPTION, 600, 0.);
    let pill_style = pill_style();
    let inner_gap = 8.;
    let coins: Vec<TickerCoin> = state
        .preferences
        .crypto_coins
        .iter()
        .map(|symbol| {
            let quote = state.crypto.quotes.iter().find(|q| &q.symbol == symbol);
            let mut parts = vec![(symbol.clone(), WHITE)];
            if let Some(quote) = quote {
                parts.push((
                    crate::crypto::format_compact(quote.price),
                    c(0xe8e8e8, 0.72),
                ));
            }
            let parts: Vec<_> = parts
                .into_iter()
                .map(|(label, colour)| {
                    let width = assets.width(&label, style);
                    (label, colour, width)
                })
                .collect();
            let change = quote.map(|quote| (change_label(quote.change, 1), quote.change));
            let width = TICKER_PLATE_LEAD
                + TICKER_MARK
                + inner_gap
                + parts.iter().map(|part| part.2 + inner_gap).sum::<f32>()
                + change
                    .as_ref()
                    .map_or(TICKER_PLATE_LEAD - inner_gap, |(label, _)| {
                        pill_width(assets, label, pill_style) + TICKER_PLATE_END
                    });
            TickerCoin {
                mark: crate::crypto::mark(symbol),
                parts,
                change,
                width,
            }
        })
        .collect();
    let widths = coins
        .iter()
        .map(|coin| coin.width + TICKER_GAP)
        .sum::<f32>();
    let overflows = widths - TICKER_GAP > line.w;
    let span = if overflows && !state.preferences.reduced_motion {
        widths.ceil()
    } else {
        0.
    };
    let offset = if span > 0. {
        state.ticker.rem_euclid(span)
    } else {
        0.
    };
    let right = line.x + line.w;
    out.draw.push(Draw::Clip(Some(line)));
    if overflows {
        let left = if span > 0. {
            [line.x, line.x + TICKER_FADE]
        } else {
            [0., 0.]
        };
        out.draw.push(Draw::EdgeFadeX(Some([
            left[0],
            left[1],
            right - TICKER_FADE,
            right,
        ])));
    }
    let laps = if span > 0. { 2 } else { 1 };
    for lap in 0..laps {
        let mut left = line.x - offset + lap as f32 * span;
        for coin in &coins {
            if left < right && left + coin.width > line.x {
                out.shape(
                    Rect::new(
                        left,
                        line.y + (line.h - TICKER_PLATE_HEIGHT) / 2. - 1.,
                        coin.width,
                        TICKER_PLATE_HEIGHT,
                    ),
                    TICKER_PLATE_HEIGHT / 2.,
                    c(0x000000, 0.36),
                );
                let (mark, tint) = coin.mark;
                let mark_x = left + TICKER_PLATE_LEAD;
                out.icon(
                    assets,
                    mark,
                    Rect::new(mark_x, line.y + 14., TICKER_MARK, TICKER_MARK),
                    c(tint, 0.95),
                );
                let mut pen = mark_x + TICKER_MARK + inner_gap;
                for (label, colour, part) in &coin.parts {
                    out.text(
                        label,
                        Rect::new(pen, line.y + 15., part + 1., 28.),
                        style,
                        *colour,
                        false,
                    );
                    pen += part + inner_gap;
                }
                if let Some((label, change)) = &coin.change {
                    tinted_pill(
                        out,
                        assets,
                        label,
                        market(*change, false),
                        pen,
                        line.y + 15.,
                        pill_style,
                        false,
                    );
                }
            }
            left += coin.width + TICKER_GAP;
        }
    }
    if overflows {
        out.draw.push(Draw::EdgeFadeX(None));
    }
    out.draw.push(Draw::Clip(Some(state.content_viewport())));
    span
}

/// Home's Today card beside the greeting: live matches, today's episodes and the prices, each
/// line with its picture, what it is, and when. None without lines; the header keeps its height.
fn today_card(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    today: &crate::today::Today,
    now: OffsetDateTime,
) -> Option<Rect> {
    use crate::today::Line;
    if today.lines.is_empty() {
        return None;
    }
    let language = state.preferences.language;
    let height = TODAY_PAD * 2. + TODAY_EYEBROW + TODAY_LINE * today.lines.len() as f32;
    let rect = Rect::new(
        TODAY_RECT.x,
        TODAY_RECT.y - state.page_scroll(),
        TODAY_RECT.w,
        height,
    );
    // Whole, it takes the focus as the other cards do; OK opens it in the center.
    let amount = state.motion.focus(&Target::TodayCard);
    out.shadow(
        rect,
        24.,
        4. + 26. * amount,
        1. + 11. * amount,
        0.01 + 0.3 * amount,
    );
    focus_glow(out, state, rect, 24., amount);
    out.glass(rect, 24.);
    // Too large to lift, it lights up instead: a brighter pane and a clear edge.
    if amount > 0. {
        out.shape(rect, 24., c(0xffffff, 0.16 * amount));
        out.border(rect, 24., 2., c(0xffffff, 0.7 * amount));
    }
    out.control(Target::TodayCard, rect, false);
    let x = rect.x + 24.;
    let inner = rect.w - 48.;
    let mut eyebrow = language.t("today").to_owned();
    if today.more > 0 {
        eyebrow = format!(
            "{eyebrow} · {}",
            language.format("todayMore", &[("count", &today.more.to_string())])
        );
    }
    out.text(
        eyebrow.to_uppercase(),
        Rect::new(x, rect.y + TODAY_PAD, inner, 24.),
        text(20., 600, 0.12),
        WHITE,
        false,
    );
    let main = text(type_scale::BODY, 550, -0.01);
    let side = text(type_scale::CAPTION, 450, 0.);
    for (index, line) in today.lines.iter().enumerate() {
        let row = Rect::new(
            rect.x + 8.,
            rect.y + TODAY_PAD + TODAY_EYEBROW + index as f32 * TODAY_LINE,
            rect.w - 16.,
            TODAY_LINE,
        );
        let icon = Rect::new(x, row.y + 8., 40., 40.);
        let text_x = icon.x + icon.w + 16.;
        // A match ends its line with the second team's logo.
        let mut second = None;
        // A live match says so on a pill.
        let mut on_pill = false;
        // (what, when, when's colour)
        let (title, when, tone) = match line {
            Line::Match { id } => {
                let Some(item) = state.cs2.find(id) else {
                    continue;
                };
                cs2_logo(out, state, item.teams[0].as_ref(), icon.inset(2.));
                second = Some(item.teams[1].as_ref());
                let side = |index: usize| {
                    item.teams[index]
                        .as_ref()
                        .map_or("TBD", |team| team.short.as_str())
                        .to_owned()
                };
                let live = item.state == crate::cs2::State::Live;
                let round = cs2_round_score(state, item, now.unix_timestamp());
                let mut title = match item.score.filter(|_| live) {
                    Some([a, b]) => format!("{} {a}–{b} {}", side(0), side(1)),
                    None => format!("{} – {}", side(0), side(1)),
                };
                if let Some(RoundNow { score: [a, b], .. }) = round {
                    title = format!("{title} · {a}:{b}");
                }
                if live {
                    let map = cs2_live_map(item, round);
                    let when = [Some(language.t("cs2Live").to_owned()), map]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · ");
                    on_pill = true;
                    (title, when, red(false))
                } else {
                    (title, cs2_start(state, item.begins, now), c(0xe8e8e8, 0.62))
                }
            }
            Line::App { app, id } => {
                let Some(card) = state
                    .app_cards
                    .list
                    .iter()
                    .find(|card| &card.app == app && &card.content_id == id)
                else {
                    continue;
                };
                app_line_picture(out, state, assets, card, icon, 8.);
                let name = app_of(state, app).title;
                (card.title.clone(), name, c(0xe8e8e8, 0.62))
            }
            Line::Episode { show } => {
                let Some(show) = state.shows.list.iter().find(|s| !s.movie && s.id == *show) else {
                    continue;
                };
                match state.shows.poster(show, state.preferences.language) {
                    Some(poster) => out.draw.push(Draw::Cover(poster, icon, 8., [1.; 4])),
                    None => out.icon(assets, "tv", icon.inset(6.), c(0xf5f5f5, 0.7)),
                }
                let when = match show.season {
                    Some(season) => format!("S{season} · {}", language.t("lampaNew")),
                    None => language.t("lampaNew").to_owned(),
                };
                (show.title(language).to_owned(), when, c(0x9fd0ff, 0.95))
            }
            Line::Release { card, date } => {
                let Some(release) = state
                    .cub
                    .releases
                    .iter()
                    .find(|r| &r.card == card && r.date == *date)
                else {
                    continue;
                };
                lampa_poster(
                    out,
                    state,
                    assets,
                    release.poster_key(state.preferences.language),
                    icon,
                );
                use crate::cub::Badge;
                let label = language.t(match release.badge {
                    Badge::Finale => "lampaFinale",
                    Badge::SeasonPremiere => "lampaSeasonPremiere",
                    Badge::Premiere => "lampaPremiere",
                    Badge::Movie => "lampaMovie",
                    _ if release.date == now.date() => "lampaToday",
                    _ => "lampaNew",
                });
                let episodes = match release.episodes.as_slice() {
                    _ if release.season == 0 => None,
                    [first] => Some(format!("S{} E{first}", release.season)),
                    [first, .., last] => Some(format!("S{} E{first}–{last}", release.season)),
                    [] => None,
                };
                let when = [episodes.as_deref(), Some(label)]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                // Out today: the line's news, in the accent; earlier ones wait in grey.
                let tone = if release.date == now.date() {
                    c(0x9fd0ff, 0.95)
                } else {
                    c(0xe8e8e8, 0.62)
                };
                (release.title.clone(), when, tone)
            }
            Line::Continue { card } => {
                let Some(watched) = state.cub.watched.iter().find(|w| &w.card == card) else {
                    continue;
                };
                lampa_poster(
                    out,
                    state,
                    assets,
                    watched.poster_key(state.preferences.language),
                    icon,
                );
                let when = match watched.episode {
                    Some((season, episode)) => {
                        format!("{} · S{season} E{episode}", language.t("lampaContinue"))
                    }
                    None => language.t("lampaContinue").to_owned(),
                };
                (watched.title.clone(), when, c(0xe8e8e8, 0.62))
            }
            Line::Crypto => {
                out.icon(assets, "coins", icon.inset(4.), c(0xf5f5f5, 0.9));
                if state.crypto.quotes.is_empty() {
                    let status = language.t(if state.crypto.failed {
                        "cryptoUnavailable"
                    } else {
                        "cryptoLoading"
                    });
                    out.text(
                        status,
                        Rect::new(text_x, row.y + 16., inner - text_x + x, 28.),
                        side,
                        c(0xe8e8e8, 0.62),
                        false,
                    );
                    continue;
                }
                let right = rect.x + rect.w - 24.;
                out.ticker = price_ticker(
                    out,
                    state,
                    assets,
                    Rect::new(text_x, row.y, right - text_x, row.h),
                );
                continue;
            }
        };
        let right = rect.x + rect.w - 24.;
        let when_width = if on_pill {
            let style = assets.fit_text(&when, pill_style(), inner * 0.45 - 2. * PILL_PAD);
            let width = pill_width(assets, &when, style);
            let x = right - width;
            tinted_pill(out, assets, &when, tone, x, row.y + 15., style, false);
            width
        } else {
            let width = assets.width(&when, side).min(inner * 0.45);
            out.text(
                &when,
                Rect::new(right - width, row.y + 16., width, 28.),
                assets.fit_text(&when, side.right(), width),
                tone,
                false,
            );
            width
        };
        let logo = 32.;
        let room = right - when_width - 16. - text_x - second.map_or(0., |_| logo + 12.);
        let style = assets.fit_text(&title, main, room);
        out.text(
            &title,
            Rect::new(text_x, row.y + 13., room, 32.),
            style,
            WHITE,
            false,
        );
        if let Some(team) = second {
            let after = text_x + assets.width(&title, style).min(room) + 12.;
            cs2_logo(out, state, team, Rect::new(after, row.y + 12., logo, logo));
        }
    }
    Some(rect)
}

/// The red of a live match and of a falling coin. `light` is for a focused line's white.
fn red(light: bool) -> Color {
    if light {
        c(0xc9252d, 1.)
    } else {
        c(0xff6b62, 1.)
    }
}

/// A price change's colour: green up, red down. `light` is for a focused line's white.
fn market(change: f64, light: bool) -> Color {
    match (change < -0.005, light) {
        (true, _) => red(light),
        (false, false) => c(0x5fd36e, 1.),
        (false, true) => c(0x15803d, 1.),
    }
}

/// A change with its arrow: "▲ 0.38%" / "▼ 1.02%", to `decimals` places.
fn change_label(change: f64, decimals: usize) -> String {
    let scale = 10f64.powi(decimals as i32);
    let rounded = (change * scale).round() / scale + 0.;
    // The same rule as its colour, so a tiny fall rounds to "▼ 0.0%" in red.
    let arrow = if change < -0.005 { "▼" } else { "▲" };
    format!("{arrow} {:.*}%", decimals, rounded.abs())
}

/// Space either side of a pill's label.
const PILL_PAD: f32 = 12.;

/// The text on a tinted pill.
fn pill_style() -> TextStyle {
    text(type_scale::CAPTION, 650, 0.)
}

/// A pill's width for its label.
fn pill_width(assets: &Assets, label: &str, style: TextStyle) -> f32 {
    assets.width(label, style) + 2. * PILL_PAD
}

/// A short status readable on any wallpaper: a dark capsule tinted `tint`, the label in that
/// colour, or a light capsule on a focused line's white. The label's top is at `y`, like the
/// text beside it.
#[allow(clippy::too_many_arguments)]
fn tinted_pill(
    out: &mut Scene,
    assets: &Assets,
    label: &str,
    tint: Color,
    x: f32,
    y: f32,
    style: TextStyle,
    light: bool,
) {
    let width = pill_width(assets, label, style);
    let height = style.size + 14.;
    let pill = Rect::new(x, y - 6., width, height);
    if !light {
        out.shape(pill, height / 2., c(0x000000, 0.4));
    }
    out.shape(
        pill,
        height / 2.,
        [tint[0], tint[1], tint[2], if light { 0.14 } else { 0.2 }],
    );
    out.text(
        label,
        Rect::new(x + PILL_PAD, y, width, 28.),
        style,
        tint,
        false,
    );
}

/// The opened Today card's section headings and lines.
const TODAY_SECTION: f32 = 48.;
const TODAY_ROW: f32 = 84.;
const TODAY_ROW_PITCH: f32 = 88.;

/// The Today card opened in the center (spec «Сегодня» раскрытая): a tab per widget in Today,
/// each with all it lists by section, one line focused at a time, scrolling under the heading.
/// It grows out of the card, which stays on Home beneath it.
fn today_open(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    now: OffsetDateTime,
    card: Option<Rect>,
) {
    use crate::state::{TODAY_LIST, TODAY_PANEL};
    use crate::today::{Line, Section};
    let language = state.preferences.language;
    let tabs = crate::today::tabs(state, now.unix_timestamp(), now.date());
    let current = state
        .today_tab
        .and_then(|tab| tabs.iter().position(|(t, _)| *t == tab))
        .unwrap_or(0);
    let sections = tabs
        .get(current)
        .map(|(_, sections)| sections.clone())
        .unwrap_or_default();
    out.controls.clear();
    let coins = &state.preferences.crypto_coins;
    let rows = |section: &Section, lines: &Vec<Line>| match section {
        Section::Prices => coins.len(),
        _ => lines.len(),
    };
    let count: usize = sections.iter().map(|(s, lines)| rows(s, lines)).sum();
    let height = |sections: &[(Section, Vec<Line>)]| {
        8. + sections
            .iter()
            .map(|(s, lines)| TODAY_SECTION + rows(s, lines) as f32 * TODAY_ROW_PITCH + 12.)
            .sum::<f32>()
    };
    // As tall as the longest tab, up to the panel, so switching tabs keeps its size; the list
    // scrolls past that.
    let content = height(&sections);
    let tallest = tabs
        .iter()
        .map(|(_, sections)| height(sections))
        .fold(content, f32::max);
    let list = Rect::new(
        TODAY_LIST.x,
        TODAY_LIST.y,
        TODAY_LIST.w,
        tallest.min(TODAY_LIST.h),
    );
    let foot = TODAY_PANEL.y + TODAY_PANEL.h - (TODAY_LIST.y + TODAY_LIST.h);
    let whole = Rect::new(
        TODAY_PANEL.x,
        TODAY_PANEL.y,
        TODAY_PANEL.w,
        list.y + list.h + foot - TODAY_PANEL.y,
    );
    let amount = state.today_amount();
    out.shape(SCREEN, 0., c(0x05080c, 0.55 * amount));
    let from = card.unwrap_or(whole);
    let lerp = |a: f32, b: f32| a + (b - a) * amount;
    let panel = Rect::new(
        lerp(from.x, whole.x),
        lerp(from.y, whole.y),
        lerp(from.w, whole.w),
        lerp(from.h, whole.h),
    );
    out.modal = Some(whole);
    out.shadow(panel, 34., 100., 24., 0.47 * amount);
    out.modal_glass(panel, 34.);
    // Drawn where it ends up, revealed as the panel grows.
    out.draw.push(Draw::Clip(Some(panel)));
    let x = TODAY_PANEL.x + 48.;
    out.text(
        language.t("today"),
        Rect::new(x, TODAY_PANEL.y + 40., 700., 52.8),
        text(type_scale::PAGE_TITLE, 600, -0.02),
        WHITE,
        false,
    );
    let subtitle = format!(
        "{} · {}",
        state.preferences.date_format.format(language, now),
        language.format("todayCount", &[("count", &count.to_string())])
    );
    out.text(
        subtitle,
        Rect::new(x, TODAY_PANEL.y + 100., 900., 28.8),
        text(type_scale::SECONDARY, 450, 0.),
        c(0xf5f5f5, 0.6),
        false,
    );
    // The widgets' tabs on the title's line, to the right: Left and Right switch them.
    let tab_style = text(type_scale::CAPTION, 600, 0.01);
    let widths: Vec<f32> = tabs
        .iter()
        .map(|(tab, _)| assets.width(language.t(tab.key()), tab_style) + 44.)
        .collect();
    let mut left = TODAY_PANEL.x + TODAY_PANEL.w
        - 48.
        - widths.iter().sum::<f32>()
        - 10. * widths.len().saturating_sub(1) as f32;
    for (index, ((tab, _), width)) in tabs.iter().zip(widths).enumerate() {
        let pill = Rect::new(left, TODAY_PANEL.y + 44., width, 48.);
        left += width + 10.;
        let selected = index == current;
        if selected {
            out.shadow(pill, 24., 14., 4., 0.22);
            out.draw.push(Draw::RoundedGradient(
                pill,
                24.,
                c(0xfafafa, 1.),
                c(0xe5e5e5, 1.),
            ));
        } else {
            out.shape(pill, 24., c(0xffffff, 0.08));
        }
        out.text(
            language.t(tab.key()),
            Rect::new(pill.x, pill.y + 11., pill.w, 26.),
            tab_style,
            if selected {
                c(0x171c20, 1.)
            } else {
                c(0xf5f5f5, 0.72)
            },
            true,
        );
        out.control(Target::TodayTab(*tab), pill, false);
    }
    let visible = Rect::new(
        list.x.max(panel.x),
        list.y.max(panel.y),
        (list.x + list.w).min(panel.x + panel.w) - list.x.max(panel.x),
        (list.y + list.h).min(panel.y + panel.h) - list.y.max(panel.y),
    );
    out.draw.push(Draw::Clip(Some(visible)));
    let bottom = list.y + list.h;
    let fade = if content > list.h { 48. } else { 0. };
    out.draw.push(Draw::EdgeFade(Some([
        list.y,
        list.y + 16.,
        bottom - fade,
        bottom,
    ])));
    let shown = state
        .motion
        .shown(crate::motion::Lane::Today, state.today_scroll);
    let top = list.y + 8. - shown;
    let mut y = top;
    let row = |y: f32| Rect::new(TODAY_PANEL.x + 24., y, TODAY_PANEL.w - 48. - 16., TODAY_ROW);
    for (section, lines) in &sections {
        out.text(
            language.t(section.key()).to_uppercase(),
            Rect::new(x, y + 14., 700., 24.),
            text(20., 600, 0.12),
            c(0xf5f5f5, 0.6),
            false,
        );
        y += TODAY_SECTION;
        for line in lines {
            if let Line::Crypto = line {
                for index in 0..coins.len() {
                    today_coin(out, state, assets, index, row(y));
                    y += TODAY_ROW_PITCH;
                }
            } else if today_line(out, state, assets, line, row(y), now) {
                y += TODAY_ROW_PITCH;
            }
        }
        y += 12.;
    }
    out.draw.push(Draw::EdgeFade(None));
    out.draw.push(Draw::Clip(Some(panel)));
    // A scroll bar once the day is longer than the list.
    if content > list.h {
        let track = Rect::new(TODAY_PANEL.x + TODAY_PANEL.w - 28., list.y, 4., list.h);
        out.shape(track, 2., c(0xffffff, 0.1));
        let thumb = (list.h * list.h / content).max(40.);
        let travel = (shown / (content - list.h)).clamp(0., 1.);
        out.shape(
            Rect::new(track.x, track.y + travel * (list.h - thumb), 4., thumb),
            2.,
            c(0xffffff, 0.5),
        );
    }
    let hint = language.t("todayHint");
    let style = text(type_scale::CAPTION, 450, 0.);
    let width = assets.width(hint, style);
    out.text(
        hint,
        Rect::new(
            whole.x + (whole.w - width) / 2.,
            whole.y + whole.h - 58.,
            width,
            26.4,
        ),
        style,
        c(0xf5f5f5, 0.5),
        false,
    );
    out.draw.push(Draw::Clip(None));
}

/// A line of the opened Today card: white when focused, with its picture, what it is, a second
/// line, and when. False for a line whose match or show is gone.
fn today_line(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    line: &crate::today::Line,
    row: Rect,
    now: OffsetDateTime,
) -> bool {
    use crate::today::Line;
    let language = state.preferences.language;
    let picture = Rect::new(row.x + 16., row.y + 14., 56., 56.);
    let mut parts = |target: Target| {
        let focused = state.focus == target;
        out.control(target, row, true);
        if focused {
            out.shadow(row, 20., 18., 6., 0.25);
            out.draw.push(Draw::RoundedGradient(
                row,
                20.,
                c(0xfafafa, 1.),
                c(0xe5e5e5, 1.),
            ));
        }
        focused
    };
    // (target, title, second line, when, when's colour)
    // A live match says so on a pill, as the prices do.
    let mut on_pill = false;
    let (focused, title, detail, mut when, tone) = match line {
        Line::Match { id } => {
            let Some(item) = state.cs2.find(id) else {
                return false;
            };
            let focused = parts(Target::TodayMatch(id.clone()));
            match item.icon.as_ref().and_then(|url| state.cs2.image(url)) {
                Some(image) => out.draw.push(Draw::Picture(image, picture, 0., [1.; 4])),
                None => out.icon(
                    assets,
                    "gaming",
                    picture.inset(10.),
                    today_ink(focused, 0.7),
                ),
            }
            let side = |index: usize| {
                item.teams[index]
                    .as_ref()
                    .map_or("TBD", |team| team.name.as_str())
                    .to_owned()
            };
            let live = item.state == crate::cs2::State::Live;
            let finished = item.state == crate::cs2::State::Finished;
            let round = cs2_round_score(state, item, now.unix_timestamp());
            let mut title = match item.score.filter(|_| live || finished) {
                Some([a, b]) => format!("{} {a}–{b} {}", side(0), side(1)),
                None => format!("{} – {}", side(0), side(1)),
            };
            if let Some(RoundNow { score: [a, b], .. }) = round {
                title = format!("{title} · {a}:{b}");
            }
            let detail = [
                Some(item.tournament.clone()),
                (!item.stage.is_empty()).then(|| item.stage.clone()),
                item.best_of.map(|n| format!("BO{n}")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            if live {
                let map = cs2_live_map(item, round);
                let when = [Some(language.t("cs2Live").to_owned()), map]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                on_pill = true;
                (focused, title, detail, when, red(focused))
            } else if finished {
                let when = language.t("cs2Final").to_owned();
                (focused, title, detail, when, c(0xe8e8e8, 0.62))
            } else {
                let when = cs2_start(state, item.begins, now);
                (focused, title, detail, when, c(0xe8e8e8, 0.62))
            }
        }
        Line::App { app, id } => {
            let Some(card) = state
                .app_cards
                .list
                .iter()
                .find(|card| &card.app == app && &card.content_id == id)
            else {
                return false;
            };
            let focused = parts(Target::TodayApp(app.clone(), id.clone()));
            app_line_picture(out, state, assets, card, picture, 10.);
            let name = app_of(state, app).title;
            (
                focused,
                card.title.clone(),
                card.category.clone(),
                name,
                c(0xe8e8e8, 0.62),
            )
        }
        Line::Episode { show } => {
            let Some(show) = state.shows.list.iter().find(|s| !s.movie && s.id == *show) else {
                return false;
            };
            let focused = parts(Target::TodayShow(show.id));
            match state.shows.poster(show, state.preferences.language) {
                Some(poster) => out.draw.push(Draw::Cover(poster, picture, 10., [1.; 4])),
                None => out.icon(assets, "tv", picture.inset(10.), today_ink(focused, 0.7)),
            }
            let detail = [
                show.season.map(|season| format!("S{season}")),
                show.rating.map(|rating| format!("★ {rating:.1}")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            let when = language.t("lampaNew").to_owned();
            let title = show.title(language).to_owned();
            (focused, title, detail, when, c(0x9fd0ff, 0.95))
        }
        Line::Release { card, date } => {
            let Some(release) = state
                .cub
                .releases
                .iter()
                .find(|r| &r.card == card && r.date == *date)
            else {
                return false;
            };
            let focused = parts(Target::TodayLampa(card.clone()));
            lampa_poster(
                out,
                state,
                assets,
                release.poster_key(state.preferences.language),
                picture,
            );
            use crate::cub::Badge;
            let label = language.t(match release.badge {
                Badge::Finale => "lampaFinale",
                Badge::SeasonPremiere => "lampaSeasonPremiere",
                Badge::Premiere => "lampaPremiere",
                Badge::Movie => "lampaMovie",
                _ if release.date == now.date() => "lampaToday",
                _ => "lampaNew",
            });
            let detail = match release.episodes.as_slice() {
                _ if release.season == 0 => String::new(),
                [first] => format!("S{} E{first}", release.season),
                [first, .., last] => format!("S{} E{first}–{last}", release.season),
                [] => String::new(),
            };
            let tone = if release.date == now.date() {
                c(0x9fd0ff, 0.95)
            } else {
                c(0xe8e8e8, 0.62)
            };
            (
                focused,
                release.title.clone(),
                detail,
                label.to_owned(),
                tone,
            )
        }
        Line::Continue { card } => {
            let Some(watched) = state.cub.watched.iter().find(|w| &w.card == card) else {
                return false;
            };
            let focused = parts(Target::TodayLampa(card.clone()));
            lampa_poster(
                out,
                state,
                assets,
                watched.poster_key(state.preferences.language),
                picture,
            );
            let detail = watched
                .episode
                .map(|(season, episode)| format!("S{season} E{episode}"))
                .unwrap_or_default();
            (
                focused,
                watched.title.clone(),
                detail,
                language.t("lampaContinue").to_owned(),
                c(0xe8e8e8, 0.62),
            )
        }
        Line::Crypto => return false,
    };
    let tone = if focused && tone[0] < 0.95 {
        today_ink(true, 0.62)
    } else if focused {
        // The accents, darker on white.
        [tone[0] * 0.75, tone[1] * 0.55, tone[2] * 0.55, 1.]
    } else {
        tone
    };
    let mut row = row;
    if on_pill {
        let right = row.x + row.w - 24.;
        let room = (right - picture.x - picture.w - 20.) * 0.4;
        let style = assets.fit_text(&when, pill_style(), room - 2. * PILL_PAD);
        let width = pill_width(assets, &when, style);
        tinted_pill(
            out,
            assets,
            &when,
            red(focused),
            right - width,
            row.y + 28.,
            style,
            focused,
        );
        row.w -= width + 16.;
        when.clear();
    }
    today_texts(
        out, assets, row, picture, focused, &title, &detail, &when, tone,
    );
    true
}

/// A coin's line among the opened Today card's prices: its symbol and name, price and change.
fn today_coin(out: &mut Scene, state: &State, assets: &Assets, index: usize, row: Rect) {
    let target = Target::Coin(index);
    let focused = state.focus == target;
    out.control(target, row, true);
    if focused {
        out.shadow(row, 20., 18., 6., 0.25);
        out.draw.push(Draw::RoundedGradient(
            row,
            20.,
            c(0xfafafa, 1.),
            c(0xe5e5e5, 1.),
        ));
    }
    let picture = Rect::new(row.x + 16., row.y + 14., 56., 56.);
    let symbol = &state.preferences.crypto_coins[index];
    let (mark, tint) = crate::crypto::mark(symbol);
    // Light marks would vanish on a focused line's white.
    let tint = if focused && tint == 0xe8e8e8 {
        today_ink(true, 0.8)
    } else {
        c(tint, 0.95)
    };
    out.icon(assets, mark, picture.inset(10.), tint);
    let name = match crate::crypto::name(symbol) {
        name if name == symbol => "",
        name => name,
    };
    let quote = state.crypto.quotes.iter().find(|q| &q.symbol == symbol);
    let mut row = row;
    let (when, tone) = match quote {
        Some(quote) => {
            // The change in its pill at the line's end; the price beside it, in the ink.
            let label = change_label(quote.change, 2);
            let style = pill_style();
            let width = pill_width(assets, &label, style);
            let right = row.x + row.w - 24.;
            tinted_pill(
                out,
                assets,
                &label,
                market(quote.change, focused),
                right - width,
                row.y + 28.,
                style,
                focused,
            );
            row.w -= width + 16.;
            (
                crate::crypto::format_price(quote.price),
                today_ink(focused, 0.8),
            )
        }
        None => (
            state
                .preferences
                .language
                .t(if state.crypto.failed {
                    "cryptoUnavailable"
                } else {
                    "cryptoLoading"
                })
                .to_owned(),
            c(0xe8e8e8, 0.62),
        ),
    };
    today_texts(
        out, assets, row, picture, focused, symbol, name, &when, tone,
    );
}

/// Text on the opened card's lines: dark on a focused line's white, light on the glass.
fn today_ink(focused: bool, alpha: f32) -> Color {
    if focused {
        c(0x171c20, alpha)
    } else {
        c(0xf5f5f5, alpha)
    }
}

/// A line's title and second line beside its picture, and when on the right.
#[allow(clippy::too_many_arguments)]
fn today_texts(
    out: &mut Scene,
    assets: &Assets,
    row: Rect,
    picture: Rect,
    focused: bool,
    title: &str,
    detail: &str,
    when: &str,
    tone: Color,
) {
    let text_x = picture.x + picture.w + 20.;
    let right = row.x + row.w - 24.;
    let side = text(type_scale::CAPTION, 500, 0.);
    let when_width = assets.width(when, side).min((right - text_x) * 0.4);
    if !when.is_empty() {
        out.text(
            when,
            Rect::new(right - when_width, row.y + 28., when_width, 28.),
            assets.fit_text(when, side.right(), when_width),
            tone,
            false,
        );
    }
    let room = right - when_width - 24. - text_x;
    let main = text(type_scale::BODY, 550, -0.01);
    out.text(
        title,
        Rect::new(text_x, row.y + 13., room, 31.2),
        assets.fit_text(title, main, room),
        if focused { c(0x171c20, 1.) } else { WHITE },
        false,
    );
    let second = text(type_scale::CAPTION, 450, 0.);
    out.text(
        detail,
        Rect::new(text_x, row.y + 46., room, 26.4),
        assets.fit_text(detail, second, room),
        today_ink(focused, 0.62),
        false,
    );
}

const CS2_CARD_HEIGHT: f32 = 252.;
const CS2_CARD_PITCH: f32 = 600.;
const CS2_CARD_RADIUS: f32 = 18.4;
/// Players named under a team on its card.
const CS2_CARD_ROSTER: usize = 5;
/// Maps a card has room for.
const CS2_CARD_MAPS: usize = 5;
/// The sides' colours, as the game paints them.
const CS2_CT: u32 = 0x7fb2ea;
const CS2_T: u32 = 0xeab05a;

/// The map a live match is playing, from the feeds server.
#[derive(Clone, Copy)]
struct RoundNow<'a> {
    map: &'a str,
    round: u32,
    /// In the feed's team order.
    score: [u32; 2],
    /// Each team's side, while its card has the full file.
    sides: Option<[crate::cs2_live::Side; 2]>,
}

fn cs2_round_score<'a>(
    state: &'a State,
    item: &crate::cs2::Match,
    now: i64,
) -> Option<RoundNow<'a>> {
    if item.state != crate::cs2::State::Live {
        return None;
    }
    if let Some(live) = state.cs2_live.detail(&item.id, now) {
        return Some(RoundNow {
            map: &live.map,
            round: live.round,
            score: live.score,
            sides: Some(live.sides),
        });
    }
    let round = state.cs2_live.round(&item.id, now)?;
    Some(RoundNow {
        map: &round.map,
        round: round.round,
        score: round.score,
        sides: None,
    })
}

/// Which of `maps` is being played and its round score. Liquipedia can lag a map behind: the
/// live file's map is found by name, and when Liquipedia has no tile for it, the tile it thinks
/// is being played takes the live map's name (the third value says so).
fn cs2_playing<'a>(
    state: &'a State,
    item: &crate::cs2::Match,
    maps: &[crate::cs2::Map],
    now: i64,
) -> (Option<usize>, Option<RoundNow<'a>>, bool) {
    let round = cs2_round_score(state, item, now);
    let named = round
        .filter(|round| !round.map.is_empty())
        .and_then(|round| {
            maps.iter()
                .position(|map| map.winner.is_none() && map.name.eq_ignore_ascii_case(round.map))
        });
    let playing = named.or_else(|| {
        item.current_map()
            .filter(|(number, _)| item.state == crate::cs2::State::Live && *number <= maps.len())
            .map(|(number, _)| number - 1)
    });
    let renamed = named.is_none() && round.is_some_and(|round| !round.map.is_empty());
    (playing, round, renamed)
}

/// The map a live match is playing: the live file's when there is one, else Liquipedia's.
fn cs2_live_map(item: &crate::cs2::Match, round: Option<RoundNow>) -> Option<String> {
    round
        .map(|round| round.map.to_owned())
        .filter(|map| !map.is_empty())
        .or_else(|| item.current_map().map(|(_, map)| map.name.clone()))
}

fn cs2_side_color(side: crate::cs2_live::Side, alpha: f32) -> Color {
    match side {
        crate::cs2_live::Side::Ct => c(CS2_CT, alpha),
        crate::cs2_live::Side::T => c(CS2_T, alpha),
    }
}

/// Home's CS2 Scores: a heading crediting Liquipedia, then up to three match cards in the inputs'
/// style, live ones first.
fn cs2_row(out: &mut Scene, state: &State, assets: &Assets, y: f32, now: OffsetDateTime) {
    let language = state.preferences.language;
    let heading = language.t("cs2Score");
    let style = row_heading(out, heading, y);
    let credit = language.t("cs2Credit");
    let credit_x = SAFE_X + assets.width(heading, style) + 20.;
    let credit_style = text(type_scale::CAPTION, 400, 0.).shadow();
    out.text(
        credit,
        Rect::new(credit_x, y + 5., 600., 27.),
        credit_style,
        c(0xf5f5f5, 0.5),
        false,
    );
    let preferences = &state.preferences;
    let unix = now.unix_timestamp();
    // Scores from an old feed say how old they are.
    if state.cs2.stale(unix)
        && let Some(updated) = cs2_updated(state, now)
    {
        out.text(
            format!("·  {updated}"),
            Rect::new(
                credit_x + assets.width(credit, credit_style) + 10.,
                y + 5.,
                600.,
                27.,
            ),
            text(type_scale::CAPTION, 550, 0.).shadow(),
            c(0xf2c27a, 0.9),
            false,
        );
    }
    let Some(feed) = state.cs2.current(unix) else {
        out.text(
            language.t(
                if state.cs2.loading || (state.cs2.feed.is_none() && !state.cs2.failed) {
                    "cs2Loading"
                } else {
                    "cs2Unavailable"
                },
            ),
            Rect::new(SAFE_X, y + ROW_HEADING_GAP, 1200., 29.),
            text(type_scale::SECONDARY, 400, 0.).shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
        return;
    };
    let matches = crate::cs2::shown(feed, preferences.cs2_major_only, preferences.cs2_results);
    if matches.is_empty() {
        out.text(
            language.t(if preferences.cs2_major_only {
                "cs2NoMajor"
            } else {
                "cs2None"
            }),
            Rect::new(SAFE_X, y + ROW_HEADING_GAP, 1200., 29.),
            text(type_scale::SECONDARY, 400, 0.).shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
        return;
    }
    // Three cards at a time from `cs2_first`; Left and Right scroll the rest into view.
    let visible = crate::cs2::VISIBLE;
    let first = state.cs2_first.min(matches.len().saturating_sub(visible));
    if matches.len() > visible {
        let last = (first + visible).min(matches.len());
        out.text(
            format!("{}–{} / {}", first + 1, last, matches.len()),
            Rect::new(SAFE_X + WIDTH - 400., y + 5., 400., 27.),
            text(type_scale::CAPTION, 500, 0.02).right().shadow(),
            c(0xf5f5f5, 0.6),
            false,
        );
    }
    carousel(
        out,
        state,
        Carousel {
            lane: crate::motion::Lane::Cs2,
            first: state.cs2_first,
            count: matches.len(),
            visible,
            pitch: CS2_CARD_PITCH,
            card: Rect::new(
                SAFE_X,
                y + ROW_HEADING_GAP,
                column_width(3),
                CS2_CARD_HEIGHT,
            ),
            below: 0.,
        },
        |out, index, rect, hit| cs2_card(out, state, assets, matches[index], rect, hit, index, now),
    );
}

/// A row of cards on Home that slides as one (`motion.rs`). `card` is where the first card in
/// view sits; `below` is extra room under each card, for labels.
struct Carousel {
    lane: crate::motion::Lane,
    first: usize,
    count: usize,
    visible: usize,
    pitch: f32,
    card: Rect,
    below: f32,
}

/// The carousel every row of cards on Home uses, so all of them move alike: the cards slide on
/// the row's lane toward `first`, those off screen are skipped, and only the `visible` ones take
/// focus and clicks, at their settled places. `draw` draws card `index` at `rect` and registers
/// `hit` when given.
fn carousel(
    out: &mut Scene,
    state: &State,
    row: Carousel,
    mut draw: impl FnMut(&mut Scene, usize, Rect, Option<Rect>),
) {
    let last_first = row.count.saturating_sub(row.visible);
    let first = row.first.min(last_first);
    let shown = state
        .motion
        .shown(row.lane, row.first as f32)
        .clamp(0., last_first as f32);
    let content = state.content_viewport();
    let top = (row.card.y - 38.).max(content.y);
    out.draw.push(Draw::Clip(Some(Rect::new(
        content.x,
        top,
        content.w,
        (row.card.y + row.card.h + row.below + 40. - top).max(0.),
    ))));
    for index in 0..row.count {
        let x = |at: f32| row.card.x + (index as f32 - at) * row.pitch;
        let rect = Rect::new(x(shown), row.card.y, row.card.w, row.card.h);
        if rect.x + rect.w < content.x - 40. || rect.x > content.x + content.w + 40. {
            continue;
        }
        let hit = (first..first + row.visible)
            .contains(&index)
            .then(|| Rect::new(x(first as f32), rect.y, rect.w, rect.h + row.below));
        draw(out, index, rect, hit);
    }
    out.draw.push(Draw::Clip(Some(content)));
}

#[allow(clippy::too_many_arguments)]
fn cs2_card(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    item: &crate::cs2::Match,
    rect: Rect,
    hit: Option<Rect>,
    index: usize,
    now: OffsetDateTime,
) {
    use crate::cs2::State as Phase;
    let language = state.preferences.language;
    let focused = *state.page_focus() == Target::Cs2Match(index) && !state.pointer;
    let live = item.state == Phase::Live;
    let (rect, lift_amount) = lift(out, state, &Target::Cs2Match(index), rect, CS2_CARD_RADIUS);
    let icon = item.icon.as_ref().and_then(|url| state.cs2.image(url));
    cs2_card_face(out, rect, focused, live, icon.as_deref());
    if let Some(icon) = icon {
        // A logo without colours is usually white, and would stand out behind the score.
        let strength = match (icon.wash.is_some(), focused) {
            (true, true) => 0.26,
            (true, false) => 0.18,
            (false, true) => 0.08,
            (false, false) => 0.06,
        };
        out.draw.push(Draw::Picture(
            icon,
            Rect::new(rect.x + rect.w - 200., rect.y + 26., 180., rect.h - 52.),
            0.,
            [1., 1., 1., strength],
        ));
    }
    // Top line: LIVE, the start or "Final", the tournament and the format.
    let x = rect.x + 22.;
    let mut caption_x = x;
    let top = rect.y + 18.;
    if live {
        let label = language.t("cs2Live");
        let style = text(13.6, 650, 0.06);
        let width = assets.width(label, style) + 18.;
        let badge = Rect::new(x, top, width, 24.);
        out.shape(badge, 7., c(0xe5484d, 0.95));
        out.text(
            label,
            Rect::new(x, top + 4.4, width, 17.),
            style,
            WHITE,
            true,
        );
        caption_x += width + 12.;
    } else {
        let label = if item.state == Phase::Finished {
            language.t("cs2Final").to_owned()
        } else {
            cs2_start(state, item.begins, now)
        };
        let style = text(16., 600, 0.);
        out.text(
            &label,
            Rect::new(x, top + 2., 260., 21.),
            style,
            WHITE,
            false,
        );
        caption_x += assets.width(&label, style) + 12.;
    }
    let mut caption_end = rect.x + rect.w - 22.;
    if let Some(best_of) = item.best_of.map(|n| format!("BO{n}")) {
        let style = text(15.2, 600, 0.04).right();
        let width = assets.width(&best_of, style);
        out.text(
            &best_of,
            Rect::new(caption_end - width, top + 3., width, 20.),
            style,
            c(0xf5f5f5, 0.72),
            false,
        );
        caption_end -= width + 14.;
    }
    let caption = text(15.2, 400, 0.);
    let room = caption_end - caption_x;
    out.text(
        &item.tournament,
        Rect::new(caption_x, top + 3., room, 20.),
        assets.fit_text(&item.tournament, caption, room),
        c(0xf5f5f5, 0.62),
        false,
    );
    // The two sides: logo, name, the players under it, and the maps they won.
    let scored = item.state != Phase::Upcoming && item.score.is_some();
    let room = rect.w - 22. - 56. - 22. - 84.;
    for side in 0..2 {
        let line = rect.y + 58. + side as f32 * 60.;
        let name = item.teams[side]
            .as_ref()
            .map_or("TBD", |team| team.name.as_str());
        let lead = item
            .score
            .is_none_or(|score| score[side] >= score[1 - side]);
        cs2_logo(
            out,
            state,
            item.teams[side].as_ref(),
            Rect::new(x, line + 2., 44., 44.),
        );
        let roster = item.lineups[side]
            .iter()
            .take(CS2_CARD_ROSTER)
            .map(|player| player.name.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        out.text(
            name,
            Rect::new(
                x + 56.,
                if roster.is_empty() { line + 10. } else { line },
                room,
                28.,
            ),
            assets.fit_text(name, text(22.4, 600, -0.02), room),
            if item.teams[side].is_some() {
                WHITE
            } else {
                c(0xf5f5f5, 0.55)
            },
            false,
        );
        if !roster.is_empty() {
            out.text(
                &roster,
                Rect::new(x + 56., line + 29., room, 20.),
                assets.fit_text(&roster, text(15.2, 400, 0.), room),
                c(0xf5f5f5, 0.66),
                false,
            );
        }
        if scored && let Some(score) = item.score {
            out.text(
                score[side].to_string(),
                Rect::new(rect.x + rect.w - 92., line + 4., 70., 40.),
                text(34., 650, -0.02).right(),
                c(0xffffff, if lead { 1. } else { 0.55 }),
                false,
            );
        }
    }
    // Bottom: each map with its score, the one being played marked; before a match its round.
    let strip = Rect::new(x, rect.y + rect.h - 66., rect.w - 44., 48.);
    if item.state != Phase::Upcoming && !item.maps.is_empty() {
        cs2_card_maps(out, state, assets, item, strip, now.unix_timestamp());
    } else if let Some(round) =
        cs2_round(language, item).or_else(|| (!item.stage.is_empty()).then(|| item.stage.clone()))
    {
        out.text(
            round,
            Rect::new(strip.x, strip.y + 18., strip.w, 22.),
            text(16., 500, 0.),
            c(0xf5f5f5, 0.62),
            false,
        );
    }
    lift_edge(out, rect, CS2_CARD_RADIUS, lift_amount);
    if let Some(hit) = hit {
        out.control(Target::Cs2Match(index), hit, true);
    }
}

/// A match card's glass: the wallpaper's blur under a dark veil, so text reads on any wallpaper,
/// washed with the colours of the tournament's logo.
fn cs2_card_face(
    out: &mut Scene,
    rect: Rect,
    focused: bool,
    live: bool,
    icon: Option<&crate::app_artwork::Artwork>,
) {
    out.frost(rect, CS2_CARD_RADIUS);
    out.shape(
        rect,
        CS2_CARD_RADIUS,
        if focused {
            c(0x1c2127, 0.58)
        } else {
            c(0x10141a, 0.66)
        },
    );
    if let Some(wash) = icon.and_then(|icon| icon.wash.clone()) {
        out.draw.push(Draw::Cover(
            wash,
            rect,
            CS2_CARD_RADIUS,
            [1., 1., 1., if focused { 1. } else { 0.86 }],
        ));
    }
    out.glass_sheen(rect, CS2_CARD_RADIUS);
    out.glass_edge(rect, CS2_CARD_RADIUS, if focused { 0.26 } else { 0.11 });
    if live {
        out.border(rect, CS2_CARD_RADIUS, 1., c(0xff5a52, 0.38));
    }
}

/// A row of a match's maps: name above, score below. The map being played stands out with its
/// round score from the feeds server, or says LIVE while there is none. Liquipedia can lag a
/// map behind: the live file's map then takes the tile Liquipedia thinks is being played.
fn cs2_card_maps(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    item: &crate::cs2::Match,
    strip: Rect,
    now: i64,
) {
    let language = state.preferences.language;
    let maps = &item.maps[..item.maps.len().min(CS2_CARD_MAPS)];
    let (playing, round, renamed) = cs2_playing(state, item, maps, now);
    let gap = 8.;
    let count = maps.len() as f32;
    let width = ((strip.w - gap * (count - 1.)) / count).min(172.);
    for (index, map) in maps.iter().enumerate() {
        let chip = Rect::new(
            strip.x + index as f32 * (width + gap),
            strip.y,
            width,
            strip.h,
        );
        let current = playing == Some(index);
        out.shape(
            chip,
            10.,
            if current {
                c(0xe5484d, 0.26)
            } else {
                c(0xffffff, 0.08)
            },
        );
        if current {
            out.border(chip, 10., 1., c(0xff5a52, 0.55));
        }
        let room = width - 20.;
        let name = match round {
            Some(round) if current && renamed => round.map,
            _ => map.name.as_str(),
        };
        out.text(
            name,
            Rect::new(chip.x + 10., chip.y + 6., room, 17.),
            assets.fit_text(name, text(13.6, 500, 0.), room),
            c(
                0xf5f5f5,
                if map.score.is_some() || current {
                    0.76
                } else {
                    0.42
                },
            ),
            false,
        );
        let place = Rect::new(chip.x + 10., chip.y + 23., room, 22.);
        if current
            && let Some(RoundNow {
                score: [a, b],
                sides,
                ..
            }) = round
        {
            let style = text(18.4, 650, 0.);
            match sides {
                // Each team's rounds in the colour of the side it plays.
                Some(sides) => {
                    let mut x = place.x;
                    for (part, tone) in [
                        (a.to_string(), cs2_side_color(sides[0], 1.)),
                        (":".to_owned(), c(0xffffff, 0.7)),
                        (b.to_string(), cs2_side_color(sides[1], 1.)),
                    ] {
                        let width = assets.width(&part, style);
                        out.text(
                            &part,
                            Rect::new(x, place.y, width + 2., place.h),
                            style,
                            tone,
                            false,
                        );
                        x += width;
                    }
                }
                None => out.text(format!("{a}:{b}"), place, style, c(0xffffff, 0.96), false),
            }
            continue;
        }
        let (value, tone) = match map.score {
            Some([a, b]) => (format!("{a}:{b}"), c(0xffffff, 0.96)),
            None if current => (language.t("cs2Live").to_owned(), c(0xffb4ae, 0.96)),
            None => ("—".to_owned(), c(0xf5f5f5, 0.36)),
        };
        out.text(value, place, text(18.4, 650, 0.), tone, false);
    }
}

/// A match's round in the TV's language: "Upper bracket · Semifinals", "Grand final"…
fn cs2_round(language: Language, item: &crate::cs2::Match) -> Option<String> {
    use crate::cs2::{Bracket, Round};
    let (round, bracket) = item.round.as_ref()?;
    let name = match round {
        Round::GrandFinal => language.t("cs2RoundGrandFinal").to_owned(),
        Round::Final => language.t("cs2RoundFinal").to_owned(),
        Round::Semifinals => language.t("cs2RoundSemifinals").to_owned(),
        Round::Quarterfinals => language.t("cs2RoundQuarterfinals").to_owned(),
        Round::Top(number) => language.format("cs2RoundOf", &[("number", &number.to_string())]),
        Round::ThirdPlace => language.t("cs2RoundThirdPlace").to_owned(),
        Round::Named(name) => name.clone(),
    };
    Some(match bracket {
        Some(Bracket::Upper) => format!("{} · {name}", language.t("cs2UpperBracket")),
        Some(Bracket::Lower) => format!("{} · {name}", language.t("cs2LowerBracket")),
        None => name,
    })
}

/// A team's logo, or its initials on a tile while there is none.
fn cs2_logo(out: &mut Scene, state: &State, team: Option<&crate::cs2::Team>, rect: Rect) {
    let radius = rect.w * 0.24;
    if let Some(image) = team.and_then(|team| state.cs2.logo(team)) {
        out.draw.push(Draw::AppImage(image, rect, radius, [1.; 4]));
        return;
    }
    out.shape(rect, radius, c(0xffffff, 0.09));
    let initials: String = team
        .map_or("?", |team| team.short.as_str())
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let size = rect.h * 0.38;
    out.text(
        initials,
        Rect::new(
            rect.x,
            rect.y + (rect.h - size * 1.2) / 2.,
            rect.w,
            size * 1.2,
        ),
        text(size, 650, 0.02),
        c(0xf5f5f5, 0.8),
        true,
    );
}

/// A CS2 Scores match in detail: the series, its maps and both lineups, live while it is open.
/// "Watch" and a chip per stream, most watched first, as many as fit; false without any.
fn cs2_streams_row(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    x: f32,
    y: f32,
    width: f32,
) -> bool {
    use crate::cs2_live::Platform;
    let streams = state.cs2_streams();
    if streams.is_empty() {
        return false;
    }
    let label = state.preferences.language.t("cs2Watch");
    let label_style = text(17.6, 550, 0.);
    let mut left = x;
    out.text(
        label,
        Rect::new(left, y + 10., 200., 24.),
        label_style,
        c(0xf5f5f5, 0.56),
        false,
    );
    left += assets.width(label, label_style) + 16.;
    let name_style = text(17.6, 550, 0.);
    let tag_style = text(13.6, 600, 0.04);
    for (index, (stream, ..)) in streams.iter().enumerate() {
        let (platform, icon, tint) = match stream.platform {
            Platform::Twitch => ("Twitch", "stream-twitch", 0xa970ff),
            Platform::Youtube => ("YouTube", "stream-youtube", 0xff3b3b),
            // Kick streams never get here: its TV app cannot play them (`launch_params`).
            Platform::Kick => continue,
        };
        let name = format!("{platform} · {}", stream.channel);
        let tag = stream.language.as_deref().map(str::to_uppercase);
        let tag_width = tag
            .as_ref()
            .map_or(0., |tag| assets.width(tag, tag_style) + 12.);
        let chip_width = 16.
            + 20.
            + 10.
            + assets.width(&name, name_style).min(320.)
            + tag.as_ref().map_or(0., |_| 10. + tag_width)
            + 16.;
        if left + chip_width > x + width {
            break;
        }
        let chip = Rect::new(left, y, chip_width, 44.);
        let focused = *state.page_focus() == Target::Cs2Stream(index) && !state.pointer;
        out.shape(chip, 14., c(0xffffff, if focused { 0.96 } else { 0.1 }));
        out.icon(
            assets,
            icon,
            Rect::new(chip.x + 16., chip.y + 12., 20., 20.),
            c(tint, 1.),
        );
        let ink = if focused {
            c(0x1c1c1e, 1.)
        } else {
            c(0xf5f5f5, 0.92)
        };
        let name_x = chip.x + 46.;
        let shown = assets.ellipsize(&name, name_style, 320.);
        out.text(
            &shown,
            Rect::new(name_x, chip.y + 10., 320., 24.),
            name_style,
            ink,
            false,
        );
        if let Some(tag) = tag {
            let pill = Rect::new(
                name_x + assets.width(&shown, name_style) + 10.,
                chip.y + 12.,
                tag_width,
                20.,
            );
            out.shape(
                pill,
                5.,
                if focused {
                    c(0x1c1c1e, 0.12)
                } else {
                    c(0xffffff, 0.12)
                },
            );
            out.text(
                &tag,
                Rect::new(pill.x + 6., pill.y + 2., tag_width, 17.),
                tag_style,
                ink,
                false,
            );
        }
        out.control(Target::Cs2Stream(index), chip, true);
        left += chip_width + 10.;
    }
    true
}

/// The match with HLTV's maps and picks when its live file has them: Liquipedia's list can lag
/// the veto by a whole map. Screenshots come from the feed's maps of the same name.
fn cs2_with_live_maps<'a>(
    state: &State,
    item: &'a crate::cs2::Match,
) -> std::borrow::Cow<'a, crate::cs2::Match> {
    let live = state.cs2_live.maps(&item.id);
    if live.is_empty() {
        return std::borrow::Cow::Borrowed(item);
    }
    let image = |name: &str| crate::cs2::map_image(state.cs2.feed.as_ref(), item, name);
    let mut merged = item.clone();
    merged.maps = live
        .iter()
        .map(|map| crate::cs2::Map {
            name: map.name.clone(),
            score: map.score,
            winner: map.winner,
            image: image(&map.name),
            picked: map.picked,
        })
        .collect();
    std::borrow::Cow::Owned(merged)
}

/// "Pick: FaZe" or "Decider" under a map's name, once the veto is known.
fn cs2_pick(state: &State, item: &crate::cs2::Match, map: &crate::cs2::Map) -> Option<String> {
    if !state
        .cs2_live
        .maps(&item.id)
        .iter()
        .any(|live| live.name == map.name)
    {
        return None;
    }
    let language = state.preferences.language;
    Some(match map.picked {
        Some(team) => {
            let team = item.teams[team].as_ref()?;
            let name = if team.short.is_empty() {
                &team.name
            } else {
                &team.short
            };
            language.format("cs2Pick", &[("team", name)])
        }
        None => language.t("cs2Decider").to_owned(),
    })
}

fn cs2_dialog(out: &mut Scene, state: &State, assets: &Assets, id: &str, now: OffsetDateTime) {
    use crate::cs2::State as Phase;
    let language = state.preferences.language;
    let panel = Rect::new(296., 60., 1328., 960.);
    out.modal = Some(panel);
    out.shadow(panel, 28., 100., 24., 0.47);
    out.modal_glass(panel, 28.);
    let close = Rect::new(panel.x + panel.w - 92., panel.y + 36., 56., 56.);
    let close_focused = *state.page_focus() == Target::Cs2Close && !state.pointer;
    out.shape(
        close,
        28.,
        c(0xffffff, if close_focused { 0.96 } else { 0.08 }),
    );
    out.icon(
        assets,
        "close",
        close.inset(17.),
        if close_focused {
            c(0x2c2c2c, 1.)
        } else {
            c(0xeeeeee, 0.85)
        },
    );
    out.control(Target::Cs2Close, close, true);
    let Some(item) = state.cs2.find(id) else {
        out.text(
            language.t("cs2Unavailable"),
            Rect::new(panel.x + 48., panel.y + 48., 900., 30.),
            text(22.4, 550, 0.),
            c(0xf5f5f5, 0.7),
            false,
        );
        return;
    };
    let item = &*cs2_with_live_maps(state, item);
    let x = panel.x + 48.;
    let inner = panel.w - 96.;
    let live = item.state == Phase::Live;

    // Header: the tournament with its logo, and everything about the match on one line.
    let icon = item.icon.as_ref().and_then(|url| state.cs2.image(url));
    let title_x = if let Some(icon) = icon {
        out.draw.push(Draw::Picture(
            icon,
            Rect::new(x, panel.y + 40., 60., 60.),
            0.,
            [1.; 4],
        ));
        x + 78.
    } else {
        x
    };
    let title_room = inner - 120. - (title_x - x);
    out.text(
        &item.tournament,
        Rect::new(title_x, panel.y + 40., title_room, 36.),
        assets.fit_text(&item.tournament, text(28., 650, -0.02), title_room),
        WHITE,
        false,
    );
    let subtitle = [
        Some("Counter-Strike 2".to_owned()),
        Some(item.stage.clone()),
        cs2_round(language, item),
        item.best_of.map(|n| format!("BO{n}")),
        item.tier
            .map(|tier| language.format("cs2Tier", &[("number", &tier.to_string())])),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");
    out.text(
        &subtitle,
        Rect::new(title_x, panel.y + 82., title_room, 24.),
        assets.fit_text(&subtitle, text(18.4, 400, 0.), title_room),
        c(0xf5f5f5, 0.62),
        false,
    );

    // The match's streams that an app on the TV can play, then the hero under them.
    let streams = cs2_streams_row(out, state, assets, x, panel.y + 120., inner);
    let shift = if streams { 60. } else { 0. };

    // Hero: both teams around the series score, the maps under them in the same block.
    let hero = Rect::new(x, panel.y + 124. + shift, inner, 236.);
    // The map being played (or the last one) behind the teams, darkened for the text.
    match item
        .featured_map()
        .and_then(|map| map.image.as_ref())
        .and_then(|url| state.cs2.image(url))
    {
        Some(image) => {
            out.draw.push(Draw::Cover(image, hero, 22., [1.; 4]));
            out.draw.push(Draw::RoundedGradient(
                hero,
                22.,
                c(0x0b0e12, 0.62),
                c(0x0b0e12, 0.84),
            ));
        }
        None => out.shape(hero, 22., c(0xffffff, 0.04)),
    }
    out.border(hero, 22., 1., c(0xffffff, 0.06));
    let middle = hero.x + hero.w / 2.;
    let name_room = hero.w / 2. - 140. - 24. - 72. - 18.;
    for side in 0..2 {
        let team = item.teams[side].as_ref();
        let logo = if side == 0 {
            Rect::new(hero.x + 24., hero.y + 24., 72., 72.)
        } else {
            Rect::new(hero.x + hero.w - 24. - 72., hero.y + 24., 72., 72.)
        };
        cs2_logo(out, state, team, logo);
        let name = team.map_or("TBD", |team| team.name.as_str());
        let style = assets.fit_text(name, text(28., 650, -0.02), name_room);
        let (place, style) = if side == 0 {
            (
                Rect::new(logo.x + logo.w + 18., hero.y + 41., name_room, 36.),
                style,
            )
        } else {
            (
                Rect::new(logo.x - 18. - name_room, hero.y + 41., name_room, 36.),
                style.right(),
            )
        };
        out.text(
            name,
            place,
            style,
            if team.is_some() {
                WHITE
            } else {
                c(0xf5f5f5, 0.55)
            },
            false,
        );
    }
    match item.score.filter(|_| item.state != Phase::Upcoming) {
        Some(score) => {
            let style = text(64., 650, -0.02);
            for side in 0..2 {
                let lead = score[side] >= score[1 - side];
                let offset = if side == 0 { -132. } else { 24. };
                out.text(
                    score[side].to_string(),
                    Rect::new(middle + offset, hero.y + 12., 108., 76.),
                    if side == 0 { style.right() } else { style },
                    c(0xffffff, if lead { 1. } else { 0.42 }),
                    false,
                );
            }
            out.text(
                ":",
                Rect::new(middle - 24., hero.y + 8., 48., 76.),
                text(56., 650, 0.),
                c(0xffffff, 0.5),
                true,
            );
        }
        None => out.text(
            "VS",
            Rect::new(middle - 100., hero.y + 26., 200., 56.),
            text(44., 650, 0.04),
            c(0xffffff, 0.45),
            true,
        ),
    }
    if live {
        let label = language.t("cs2Live");
        let style = text(13.6, 650, 0.06);
        let width = assets.width(label, style) + 18.;
        let badge = Rect::new(middle - width / 2., hero.y + 92., width, 24.);
        out.shape(badge, 7., c(0xe5484d, 0.95));
        out.text(
            label,
            Rect::new(badge.x, badge.y + 4.4, width, 17.),
            style,
            WHITE,
            true,
        );
    } else {
        let line = if item.state == Phase::Finished {
            language.t("cs2Final").to_owned()
        } else {
            cs2_start(state, item.begins, now)
        };
        out.text(
            line,
            Rect::new(middle - 200., hero.y + 90., 400., 26.),
            text(19.2, 650, 0.),
            WHITE,
            true,
        );
    }
    cs2_dialog_maps(
        out,
        state,
        assets,
        item,
        Rect::new(hero.x + 22., hero.y + 132., hero.w - 44., 84.),
        now.unix_timestamp(),
    );

    // While the feeds server has the match: its scoreboard; else the lineups.
    if let Some(live) = state.cs2_live.detail(&item.id, now.unix_timestamp()) {
        let top = hero.y + hero.h + 20.;
        let height = cs2_live_view::height().min(panel.y + 900. - top);
        cs2_live_view::scoreboard(
            out,
            state,
            assets,
            live,
            item,
            Rect::new(x, top, 760., height),
        );
        cs2_live_view::game_log(
            out,
            state,
            assets,
            live,
            Rect::new(x + 774., top, inner - 774., height),
        );
        cs2_footer(out, state, x, inner, panel, now);
        return;
    }
    // Lineups, down to the footer.
    let lists_y = hero.y + hero.h + 24.;
    let column_w = (inner - 24.) / 2.;
    for side in 0..2 {
        let column = Rect::new(
            x + side as f32 * (column_w + 24.),
            lists_y,
            column_w,
            // Five players at most 62 apart under the heading, short of the footer.
            (panel.y + panel.h - 64. - lists_y).min(74. + 62. * 5.),
        );
        out.shape(column, 20., c(0xffffff, 0.04));
        let team = item.teams[side].as_ref();
        cs2_logo(
            out,
            state,
            team,
            Rect::new(column.x + 20., column.y + 16., 32., 32.),
        );
        let heading = team.map_or("TBD", |team| team.short.as_str());
        let heading_style = text(22.4, 600, 0.);
        let heading_w = assets.width(heading, heading_style);
        let heading_x = column.x + 64.;
        out.text(
            heading,
            Rect::new(heading_x, column.y + 18., heading_w, 28.),
            heading_style,
            WHITE,
            false,
        );
        let players = &item.lineups[side];
        if players.is_empty() {
            out.text(
                language.t("cs2NoLineup"),
                Rect::new(column.x + 20., column.y + 68., column.w - 40., 28.),
                text(20.8, 400, 0.),
                c(0xf5f5f5, 0.5),
                false,
            );
        }
        let coaches = team.map_or(&[][..], |team| {
            state
                .cs2
                .feed
                .as_ref()
                .map_or(&[][..], |feed| feed.coaches(team))
        });
        // Coaches share the heading's line, as many as fit beside it, so the players get the height.
        let label = language.t("cs2Coach").to_uppercase();
        let label_style = text(15.2, 650, 0.1);
        let label_w = assets.width(&label, label_style);
        let names_style = text(20.8, 600, 0.);
        let right = column.x + column.w - 20.;
        let room = right - (heading_x + heading_w) - 24. - label_w - 12.;
        let names = (1..=coaches.len())
            .rev()
            .map(|count| {
                coaches[..count]
                    .iter()
                    .map(|coach| coach.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .find(|names| assets.width(names, names_style) <= room);
        if let Some(names) = names {
            let names_w = assets.width(&names, names_style);
            out.text(
                &label,
                Rect::new(
                    right - names_w - 12. - label_w,
                    column.y + 24.,
                    label_w,
                    20.,
                ),
                label_style,
                c(0xf5f5f5, 0.5),
                false,
            );
            out.text(
                names,
                Rect::new(right - names_w, column.y + 20., names_w, 28.),
                names_style,
                WHITE,
                false,
            );
        }
        // Five players sit roomy; a stand-in as the sixth tightens every row.
        let top = column.y + 62.;
        let pitch = ((column.h - 62. - 12.) / players.len().clamp(5, 6) as f32).min(62.);
        for (row, player) in players.iter().take(6).enumerate() {
            let middle = top + (row as f32 + 0.5) * pitch;
            let flag = Rect::new(column.x + 20., middle - 11., 30., 22.);
            match crate::cs2_flags::code(&player.flag) {
                Some(code) => out.image(assets.path(format!("flags/{code}.png")), flag, 4., false),
                None => out.shape(flag, 4., c(0xffffff, 0.06)),
            }
            out.text(
                &player.name,
                Rect::new(heading_x, middle - 16., column.w - 84., 32.),
                text(25.6, 600, -0.01),
                WHITE,
                false,
            );
        }
    }

    cs2_footer(out, state, x, inner, panel, now);
}

/// Where a match card's data comes from, and when the feed was last rebuilt.
fn cs2_footer(
    out: &mut Scene,
    state: &State,
    x: f32,
    inner: f32,
    panel: Rect,
    now: OffsetDateTime,
) {
    let language = state.preferences.language;
    out.text(
        format!("{} · CC BY-SA 3.0", language.t("cs2Credit")),
        Rect::new(x, panel.y + panel.h - 44., inner, 20.),
        text(14.4, 500, 0.).right(),
        c(0xf5f5f5, 0.5),
        false,
    );
    // When the feed was last rebuilt, so a stalled feed shows.
    if let Some(updated) = cs2_updated(state, now) {
        out.text(
            updated,
            Rect::new(x, panel.y + panel.h - 44., inner - 340., 20.),
            text(14.4, 500, 0.),
            c(0xf5f5f5, 0.6),
            false,
        );
    }
}

/// The map tiles of a match in detail: each map with its score, the one being played marked,
/// with its rounds in the sides' colours and the round number while the feeds server has them.
fn cs2_dialog_maps(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    item: &crate::cs2::Match,
    strip: Rect,
    now: i64,
) {
    let language = state.preferences.language;
    let slots = item
        .best_of
        .unwrap_or(1)
        .max(item.maps.len() as u32)
        .clamp(1, 7) as usize;
    let (playing, round, renamed) = cs2_playing(state, item, &item.maps, now);
    let gap = 12.;
    let tile_w = (strip.w - gap * (slots - 1) as f32) / slots as f32;
    for slot in 0..slots {
        let tile = Rect::new(
            strip.x + slot as f32 * (tile_w + gap),
            strip.y,
            tile_w,
            strip.h,
        );
        let map = item.maps.get(slot);
        let current = playing == Some(slot);
        let pick = map.and_then(|map| cs2_pick(state, item, map));
        out.shape(
            tile,
            14.,
            if current {
                c(0x781e26, 0.45)
            } else {
                c(0x000000, 0.25)
            },
        );
        out.border(
            tile,
            14.,
            if current { 2. } else { 1. },
            if current {
                c(0xd9484f, 1.)
            } else {
                c(0xffffff, 0.08)
            },
        );
        let title = match (map, round) {
            (_, Some(round)) if current && renamed => round.map.to_owned(),
            (Some(map), _) => map.name.clone(),
            (None, _) => language.format("cs2Map", &[("number", &(slot + 1).to_string())]),
        };
        let score_room = 120.;
        let room = tile.w - 32. - score_room;
        out.text(
            &title,
            Rect::new(
                tile.x + 16.,
                tile.y + if current || pick.is_some() { 13. } else { 28. },
                room,
                28.,
            ),
            assets.fit_text(&title, text(20., 600, -0.01), room),
            c(0xffffff, if map.is_some() || current { 1. } else { 0.45 }),
            false,
        );
        let right = tile.x + tile.w - 16.;
        if current {
            // LIVE beside the name, the round under it.
            let title_style = assets.fit_text(&title, text(20., 600, -0.01), room);
            let label = language.t("cs2Live");
            let label_style = text(13.6, 650, 0.06);
            let label_x = tile.x + 16. + assets.width(&title, title_style).min(room) + 10.;
            if label_x + assets.width(label, label_style) <= tile.x + 16. + room {
                out.text(
                    label,
                    Rect::new(label_x, tile.y + 18., 80., 18.),
                    label_style,
                    c(0xff6b62, 1.),
                    false,
                );
            }
            let below = match round {
                Some(round) if round.round > 0 => {
                    language.format("cs2RoundNumber", &[("number", &round.round.to_string())])
                }
                Some(_) => String::new(),
                None if map.is_none_or(|map| map.score.is_none()) => {
                    language.t("cs2NoRounds").to_owned()
                }
                None => String::new(),
            };
            let below = match (below.is_empty(), &pick) {
                (_, None) => below,
                (true, Some(pick)) => pick.clone(),
                (false, Some(pick)) => format!("{below} · {pick}"),
            };
            let below_room = tile.w - 32. - if round.is_some() { score_room } else { 0. };
            out.text(
                &below,
                Rect::new(tile.x + 16., tile.y + 47., below_room, 20.),
                assets.fit_text(&below, text(14.4, 600, 0.04), below_room),
                c(0xf0c6c8, 1.),
                false,
            );
        }
        if let Some(pick) = pick.as_ref().filter(|_| !current) {
            out.text(
                pick,
                Rect::new(tile.x + 16., tile.y + 47., room, 20.),
                assets.fit_text(pick, text(14.4, 600, 0.04), room),
                c(0xf5f5f5, 0.6),
                false,
            );
        }
        match (current, round, map.and_then(|map| map.score)) {
            (true, Some(round), _) => {
                let style = text(28., 650, 0.);
                let [a, b] = round.score;
                let parts = [
                    (
                        a.to_string(),
                        round.sides.map(|sides| cs2_side_color(sides[0], 1.)),
                    ),
                    (":".to_owned(), Some(c(0xffffff, 0.6))),
                    (
                        b.to_string(),
                        round.sides.map(|sides| cs2_side_color(sides[1], 1.)),
                    ),
                ];
                let spacing = 6.;
                let total: f32 = parts
                    .iter()
                    .map(|(part, _)| assets.width(part, style))
                    .sum::<f32>()
                    + spacing * 2.;
                let mut x = right - total;
                for (part, tone) in parts {
                    let width = assets.width(&part, style);
                    out.text(
                        &part,
                        Rect::new(x, tile.y + 24., width + 2., 36.),
                        style,
                        tone.unwrap_or(WHITE),
                        false,
                    );
                    x += width + spacing;
                }
            }
            (_, _, Some([a, b])) => out.text(
                format!("{a}:{b}"),
                Rect::new(right - score_room, tile.y + 27., score_room, 30.),
                text(22.4, 650, 0.).right(),
                WHITE,
                false,
            ),
            _ if !current => out.text(
                "—",
                Rect::new(right - score_room, tile.y + 27., score_room, 30.),
                text(22.4, 600, 0.).right(),
                c(0xf5f5f5, 0.36),
                false,
            ),
            _ => {}
        }
    }
}

/// "Last updated 21:05" for the feed, in the TV's time; the date too when not today.
fn cs2_updated(state: &State, now: OffsetDateTime) -> Option<String> {
    let language = state.preferences.language;
    let feed = state.cs2.feed.as_ref()?;
    let time = OffsetDateTime::from_unix_timestamp(feed.updated)
        .ok()?
        .to_offset(now.offset());
    let mut clock = language.clock(time, state.preferences.hour_24, false);
    if time.date() != now.date() {
        clock = format!("{:02}.{:02} {clock}", time.day(), time.month() as u8);
    }
    Some(language.format("cs2Updated", &[("time", &clock)]))
}

/// A match's start in the TV's time: "21:00" today, "Tomorrow 21:00", else its date.
fn cs2_start(state: &State, begins: i64, now: OffsetDateTime) -> String {
    let Ok(start) = OffsetDateTime::from_unix_timestamp(begins).map(|t| t.to_offset(now.offset()))
    else {
        return String::new();
    };
    let preferences = &state.preferences;
    let clock = preferences
        .language
        .clock(start, preferences.hour_24, false);
    let days = (start.date() - now.date()).whole_days();
    match days {
        0 => clock,
        1 => preferences
            .language
            .format("cs2Tomorrow", &[("time", &clock)]),
        _ => {
            let (day, month) = (start.day(), start.month() as u8);
            let month_first = preferences.date_format == crate::model::DateFormat::MonthDayYear
                || (preferences.date_format == crate::model::DateFormat::Localized
                    && preferences.language == Language::En);
            if month_first {
                format!("{month:02}/{day:02} {clock}")
            } else {
                format!("{day:02}.{month:02} {clock}")
            }
        }
    }
}

const WEATHER_TEXT: f32 = 28.;
const WEATHER_ICON: f32 = 30.;
/// The weather's text starts past its icon.
const WEATHER_INDENT: f32 = WEATHER_ICON + 12.;
struct WeatherSummary {
    height: f32,
    width: f32,
    icon: String,
    parts: Vec<(String, TextStyle, Color, Rect)>,
}
type WeatherPart = (String, TextStyle, Color, f32, f32);
/// The header's weather line, wrapping within `column` (the left column's width).
fn weather_summary_layout(state: &State, assets: &Assets, column: f32) -> WeatherSummary {
    let language = state.preferences.language;
    let base = text(WEATHER_TEXT, 400, -0.02).leading(1.5);
    let mut parts = Vec::new();
    let mut icon = "location".to_owned();
    if let Some(place) = &state.preferences.weather_location {
        parts.push((place.name.clone(), base.weight(450), c(0xf5f5f5, 0.72)));
        parts.push(("·".into(), base, c(0xf5f5f5, 0.324)));
        if let Some(reading) = state.weather.reading(place) {
            let condition = weather::condition(reading);
            icon = format!("weather-{}", condition.icon);
            parts.push((
                weather::format_temperature(
                    reading.temperature,
                    state.preferences.temperature_unit,
                ),
                base.weight(550),
                c(0xf5f5f5, 0.91),
            ));
            parts.push((language.t(condition.key).into(), base, c(0xf5f5f5, 0.58)));
            if state.weather.failed {
                parts.push((
                    language.t("lastReading").into(),
                    text(type_scale::CAPTION, 400, 0.).leading(1.5),
                    c(0xf2d5a5, 0.65),
                ));
            }
        } else {
            parts.push((
                if state.weather.loading {
                    language.t("loadingWeather")
                } else {
                    language.t("weatherUnavailable")
                }
                .into(),
                base,
                c(0xf5f5f5, 0.58),
            ));
        }
    } else {
        parts.push((
            language.t("setWeatherLocation").into(),
            base,
            c(0xf5f5f5, 0.72),
        ));
    }
    let mut rows: Vec<Vec<WeatherPart>> = vec![Vec::new()];
    let mut used = 0.;
    for (label, style, color) in parts {
        // A part wider than the column (a very long place name) shrinks onto one line rather than
        // wrapping the header down into the favorites.
        let style = assets.fit_text(&label, style, column - WEATHER_INDENT);
        let measured_width = assets.width(&label, style);
        let width = measured_width.min(column - WEATHER_INDENT);
        // Use the same extra pixel as the rendered text rectangle. Counting
        // lines at the exact advance width can round into a phantom second
        // line even though the renderer displays the label on one line.
        let text_width = width + 1.;
        let lines = if measured_width <= text_width && !label.contains('\n') {
            1
        } else {
            assets.lines(&label, style, text_width)
        };
        let height = lines as f32 * style.size * style.line_height;
        if used > 0. && used + 11.2 + width > column - WEATHER_INDENT {
            rows.push(Vec::new());
            used = 0.;
        }
        rows.last_mut()
            .unwrap()
            .push((label, style, color, width, height));
        used += width + if used > 0. { 11.2 } else { 0. };
    }
    let mut positioned = Vec::new();
    let mut top = 1.6;
    let mut right: f32 = 0.;
    for row in rows {
        let height = row
            .iter()
            .map(|part| part.4)
            .fold(WEATHER_TEXT * 1.5, f32::max);
        let mut x = SAFE_X + WEATHER_INDENT;
        for (label, style, color, width, item_height) in row {
            positioned.push((
                label,
                style,
                color,
                Rect::new(
                    x,
                    top + (height - item_height) / 2.,
                    width + 1.,
                    item_height,
                ),
            ));
            x += width + 11.2;
        }
        right = right.max(x - 11.2);
        top += height + 11.2;
    }
    WeatherSummary {
        height: top - 11.2 + 1.6,
        width: right - SAFE_X,
        icon,
        parts: positioned,
    }
}
fn weather_summary(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    summary: WeatherSummary,
    y: f32,
) {
    let label_focus = summary.parts.first().map(|(_, style, _, rect)| {
        // The label's layout line box includes leading above and below the
        // text. Keep the focus ring snug to the label rather than the whole
        // weather row, which also contains the location icon.
        let leading = style.size * (style.line_height - 1.).max(0.);
        Rect::new(
            rect.x,
            y + rect.y + leading / 2.,
            rect.w,
            (rect.h - leading).max(style.size),
        )
    });
    out.shadowed_icon(
        assets,
        &summary.icon,
        Rect::new(
            SAFE_X,
            y + (WEATHER_TEXT * 1.5 - WEATHER_ICON) / 2. + 1.6,
            WEATHER_ICON,
            WEATHER_ICON,
        ),
        c(0xdedede, 0.83),
    );
    for (label, style, color, rect) in summary.parts {
        out.text(
            label,
            Rect::new(rect.x, rect.y + y, rect.w, rect.h),
            style.shadow(),
            color,
            false,
        );
    }
    if state.preferences.weather_location.is_none() {
        let rect = Rect::new(SAFE_X, y, summary.width, summary.height);
        if *state.page_focus() == Target::WeatherConfigure
            && !state.pointer
            && let Some(label) = label_focus
        {
            out.border(label.inset(-5.), 8.6, 2., c(0xf0f0f0, 0.95));
        }
        out.control(Target::WeatherConfigure, rect, true);
    }
}

fn library(out: &mut Scene, state: &State, assets: &Assets) {
    library_grid(out, state, assets);
    // The filters go up with the grid, like the tab bar.
    let (draws, controls) = (out.draw.len(), out.controls.len());
    library_toolbar(out, state, assets);
    out.slide(draws, controls, state.chrome_offset());
}

/// The toolbar stays centred on the line it sits on at normal scale, clear of the grid.
/// The Apps page: one row of filters and search from y 150, then six 260 px columns of cards,
/// rows 206 apart, so four rows fit the screen.
const APPS_TOOLBAR_TOP: f32 = 150.;
const TOOLBAR_HEIGHT: f32 = 60.;
const TOOLBAR_CENTER: f32 = APPS_TOOLBAR_TOP + TOOLBAR_HEIGHT / 2.;
const APPS_GRID_TOP: f32 = APPS_TOOLBAR_TOP + TOOLBAR_HEIGHT + GUTTER;
const APPS_FACE_HEIGHT: f32 = 150.;
const APPS_ROW_PITCH: f32 = 206.;

fn library_toolbar(out: &mut Scene, state: &State, assets: &Assets) {
    let k = state.preferences.ui_scale.scale();
    let x = category_chips(out, state, assets, SAFE_X, TOOLBAR_CENTER);
    if state.filter != 0 {
        let h = TOOLBAR_HEIGHT * k;
        let rect = Rect::new(x, TOOLBAR_CENTER - h / 2., h, h);
        let focused = *state.page_focus() == Target::ClearFilter;
        out.pill(rect, focused, false);
        out.icon(
            assets,
            "close",
            rect.inset(16. * k),
            if focused {
                c(0x2c2c2c, 1.)
            } else {
                c(0xeeeeee, 0.73)
            },
        );
        out.control(Target::ClearFilter, rect, false);
    }
    search_field(out, state, assets, SAFE_X + WIDTH, TOOLBAR_CENTER, true);
}

/// The category filter chips from `x`, centred on `y`; returns where the row ends.
fn category_chips(out: &mut Scene, state: &State, assets: &Assets, mut x: f32, y: f32) -> f32 {
    let language = state.preferences.language;
    let k = state.preferences.ui_scale.scale();
    let style = text(24. * k, 550, -0.02);
    let h = TOOLBAR_HEIGHT * k;
    // All, with how many apps there are.
    let count = state
        .inventory
        .apps
        .iter()
        .filter(|app| !state.preferences.hidden.contains(&app.id))
        .count();
    let all = language.format("allApps", &[("count", &count.to_string())]);
    let width = assets.width(&all, style) + 40. * k;
    filter(
        out,
        state,
        Target::Filter(0),
        Rect::new(x, y - h / 2., width, h),
        &all,
        state.filter == 0,
    );
    x += width + 7.2 * k;
    for (index, label) in CATEGORIES.into_iter().enumerate() {
        let label = language.category(label);
        let width = assets.width(label, style) + 40. * k;
        filter(
            out,
            state,
            Target::Filter(index + 1),
            Rect::new(x, y - h / 2., width, h),
            label,
            state.filter == index + 1,
        );
        x += width + 7.2 * k;
    }
    if state.hidden_count() > 0 {
        let label = language.format("hidden", &[("count", &state.hidden_count().to_string())]);
        let rect = Rect::new(x, y - h / 2., assets.width(&label, style) + 40. * k, h);
        filter(
            out,
            state,
            Target::Filter(5),
            rect,
            &label,
            state.filter == 5,
        );
        x += rect.w + 7.2 * k;
    }
    x
}

/// The app search field, right-aligned at `right` and centred on `y`.
fn search_field(out: &mut Scene, state: &State, assets: &Assets, right: f32, y: f32, live: bool) {
    let language = state.preferences.language;
    let k = state.preferences.ui_scale.scale();
    let h = TOOLBAR_HEIGHT * k;
    let search = Rect::new(right - 320. * k, y - h / 2., 320. * k, h);
    let focused = live && matches!(state.page_focus(), Target::Search | Target::ClearSearch);
    let query = if live { state.query.as_str() } else { "" };
    if !focused {
        out.glass(search, 48.);
    } else {
        out.shape(search, 48., c(0x303438, 1.));
        out.border(search, 48., 1., c(0xffffff, 0.70));
        out.border(search.inset(-3.), 51., 3., c(0xffffff, 0.063));
    }
    out.icon(
        assets,
        "search",
        Rect::new(
            search.x + 19.4 * k,
            search.y + (search.h - 19.2 * k) / 2.,
            19.2 * k,
            19.2 * k,
        ),
        c(0xeeeeee, 0.73),
    );
    out.text(
        if query.is_empty() {
            language.t("findApp")
        } else {
            query
        },
        Rect::new(
            search.x + 49.8 * k,
            search.y + (search.h - 33.6 * k) / 2.,
            if query.is_empty() { 250. * k } else { 218. * k },
            33.6 * k,
        ),
        text(24. * k, 550, -0.02).leading(1.4),
        c(0xeeeeee, if query.is_empty() { 0.73 } else { 1. }),
        false,
    );
    out.control(Target::Search, search, false);
    if !query.is_empty() {
        let clear = Rect::new(
            search.x + search.w - 45.8 * k,
            search.y + 14.8 * k,
            26.4 * k,
            26.4 * k,
        );
        out.shape(clear, 13.2 * k, c(0xffffff, 0.055));
        out.icon(assets, "close", clear.inset(5.2 * k), WHITE);
        if *state.page_focus() == Target::ClearSearch {
            out.border(clear.inset(-2.), 13.2 * k + 2., 2., WHITE);
        }
        out.control(Target::ClearSearch, clear, false);
    }
}

fn library_grid(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    let favorites = state.preferences.favorites(&state.inventory.apps);
    let apps = state.apps();
    if apps.is_empty() {
        let title = if state.loading {
            language.t("findingApps")
        } else if !state.query.is_empty() {
            language.t("noMatches")
        } else if state.filter == 5 {
            language.t("noHidden")
        } else {
            language.t("roomForNew")
        };
        let hint = state.error.clone().unwrap_or_else(|| {
            if !state.query.is_empty() {
                language.format("noMatchesHint", &[("query", &state.query)])
            } else if state.filter == 5 {
                language.t("hiddenHint").into()
            } else {
                language.t("noCategoryApps").into()
            }
        });
        empty(
            out,
            state,
            assets,
            EmptyState {
                rect: Rect::new(SAFE_X, APPS_GRID_TOP - s, WIDTH, 304.),
                icon: "grid",
                title,
                hint: &hint,
                action: state.error.as_ref().map(|_| Target::Retry),
            },
        );
    }
    let mut y = APPS_GRID_TOP;
    let names = state.preferences.app_names == crate::model::AppNames::Always;
    let on_focus = state.preferences.app_names == crate::model::AppNames::Focus;
    let width = crate::layout::column(APPS_COLUMNS);
    for row in apps.chunks(APPS_COLUMNS) {
        // Names under every card need their room; the focused card's name fits the gap.
        let row_height = if names {
            APPS_FACE_HEIGHT
                + 18.4
                + row
                    .iter()
                    .map(|app| caption_height(state, assets, app, width))
                    .fold(58.24 * caption_scale(state), f32::max)
                + 38.4
        } else {
            APPS_ROW_PITCH
        };
        for (column, app) in row.iter().enumerate() {
            let rect = Rect::new(
                SAFE_X + column as f32 * (width + GUTTER),
                y - s,
                width,
                APPS_FACE_HEIGHT,
            );
            tile(
                out,
                state,
                assets,
                app,
                rect,
                names,
                favorites.contains(&app.id),
            );
            if on_focus {
                focused_name(out, state, assets, app, rect);
            }
        }
        y += row_height;
    }
}

fn filter(out: &mut Scene, state: &State, target: Target, rect: Rect, label: &str, selected: bool) {
    let focused = *state.page_focus() == target;
    let k = rect.h / TOOLBAR_HEIGHT;
    out.pill(rect, focused, selected);
    if selected && !focused {
        out.border(rect, rect.h / 2., 1.5, c(0xffffff, 0.35));
    }
    out.text(
        label,
        Rect::new(rect.x, rect.y + (rect.h - 33.6 * k) / 2., rect.w, 33.6 * k),
        text(24. * k, 550, -0.02).leading(1.4),
        if focused {
            c(0x2c2c2c, 1.)
        } else {
            c(0xeeeeee, if selected { 1. } else { 0.73 })
        },
        true,
    );
    out.control(target, rect, false);
}

/// The existing System status card owns the only update action.
fn update_value(state: &State) -> String {
    use crate::update::{InstallStatus, Status};
    let language = state.preferences.language;
    match &state.update {
        InstallStatus::Idle => {
            if let Some(version) = &state.available_update {
                language.format(
                    "installUpdate",
                    &[("version", &crate::update::label(version))],
                )
            } else {
                language
                    .t(if state.update_status == Status::Checking {
                        "checkingUpdates"
                    } else {
                        "checkAgain"
                    })
                    .to_owned()
            }
        }
        InstallStatus::Downloading(percent) => {
            language.format("updateDownloading", &[("percent", &percent.to_string())])
        }
        InstallStatus::Installing(_) => language.t("updateInstalling").to_owned(),
        InstallStatus::Failed(_) => language.t("updateFailed").to_owned(),
    }
}

struct Row<'a> {
    target: Target,
    rect: Rect,
    title: &'a str,
    icon: Option<&'a str>,
    value: Option<&'a str>,
    description: Option<&'a str>,
    selected: bool,
    chevron: bool,
}

/// Settings rows one under another, from `top`: the focused one grows to show its description.
fn list_rects(state: &State, top: f32, rows: &[(Target, bool)]) -> Vec<Rect> {
    let mut y = top - state.scroll;
    rows.iter()
        .map(|(target, described)| {
            let grown = if *described {
                state.motion.focus(target)
            } else {
                0.
            };
            let rect = Rect::new(
                LIST_X,
                y,
                LIST_WIDTH,
                ROW_HEIGHT + (ROW_GROWN - ROW_HEIGHT) * grown,
            );
            y += rect.h + ROW_SPACING;
            rect
        })
        .collect()
}

/// Between rows of a settings list.
const LIST_PITCH: f32 = ROW_HEIGHT + ROW_SPACING;
/// Between rows whose second line always shows (a name, a status, a place's region).
const TWO_LINE_PITCH: f32 = ROW_GROWN + ROW_SPACING;

/// A settings row: icon and title on the left, the value right beside the list's edge. The
/// description shows once the row is taller than one line: always for rows sized for two, on
/// focus for rows `list_rects` grows.
fn row(out: &mut Scene, state: &State, assets: &Assets, row: Row<'_>) {
    const RADIUS: f32 = 16.;
    let rect = row.rect;
    let focused = *state.page_focus() == row.target;
    if focused {
        out.draw.push(Draw::RoundedGradient(
            rect,
            RADIUS,
            c(0xfafafa, 1.),
            c(0xe5e5e5, 1.),
        ));
    } else {
        settings_surface(out, rect, RADIUS);
    }
    let ink = if focused {
        c(0x2c2c2c, 1.)
    } else {
        c(0xf4f4f4, 0.92)
    };
    let quiet = if focused {
        c(0x2c2c2c, 0.62)
    } else {
        c(0xffffff, 0.62)
    };
    let mut x = rect.x + 24.;
    if let Some(icon) = row.icon {
        out.icon(
            assets,
            icon,
            Rect::new(x, rect.y + 21., 26., 26.),
            if focused {
                c(0x444444, 1.)
            } else {
                c(0xe4e4e4, 0.7)
            },
        );
        x += 44.;
    }
    let right = rect.x + rect.w
        - if row.chevron || row.selected {
            56.
        } else {
            24.
        };
    let value_style = text(type_scale::SECONDARY, 450, 0.).right();
    let value_width = row.value.map_or(0., |value| {
        // `fit_text` keeps a pixel spare.
        (assets.width(value, value_style) + 1.)
            .ceil()
            .min(rect.w * 0.45)
    });
    if let Some(value) = row.value {
        out.text(
            value,
            Rect::new(right - value_width, rect.y + 20., value_width, 29.),
            assets.fit_text(value, value_style, value_width),
            quiet,
            false,
        );
    }
    let title_width = if row.value.is_some() {
        right - value_width - 16.
    } else {
        right
    } - x;
    out.text(
        row.title,
        Rect::new(x, rect.y + 18.5, title_width, 31.),
        assets.fit_text(row.title, text(type_scale::BODY, 550, -0.02), title_width),
        ink,
        false,
    );
    if let Some(description) = row.description {
        let shown = ((rect.h - ROW_HEIGHT) / (ROW_GROWN - ROW_HEIGHT)).clamp(0., 1.);
        if shown > 0. {
            let mut color = if focused {
                c(0x555555, 1.)
            } else {
                c(0xf5f5f5, 0.62)
            };
            color[3] *= shown;
            out.text(
                description,
                Rect::new(
                    x,
                    rect.y + 56.,
                    rect.x + rect.w - 24. - x,
                    (rect.h - 70.).max(27.),
                ),
                text(type_scale::CAPTION, 450, 0.),
                color,
                false,
            );
        }
    }
    if row.chevron || row.selected {
        out.icon(
            assets,
            if row.selected { "check" } else { "right" },
            Rect::new(rect.x + rect.w - 44., rect.y + 24., 20., 20.),
            if focused {
                c(0x444444, 1.)
            } else {
                c(0xe4e4e4, 0.61)
            },
        );
    }
    out.control(row.target, rect, true);
}

fn weather_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherToggle,
            rect: Rect::new(LIST_X, LIST_TOP - s, LIST_WIDTH, ROW_HEIGHT),
            title: language.t("showWidget"),
            icon: Some("eye"),
            value: Some(if state.preferences.show_weather {
                language.t("stateOn")
            } else {
                language.t("stateOff")
            }),
            description: None,
            selected: false,
            chevron: false,
        },
    );
    let city = state
        .preferences
        .weather_location
        .as_ref()
        .map(|place| place.name.as_str());
    let style = text(type_scale::SECONDARY, 450, 0.).leading(1.2);
    let line = type_scale::SECONDARY * 1.2;
    let city_lines = city.map_or(1, |city| assets.lines(city, style, 400.));
    let height = (40. + city_lines as f32 * line).max(ROW_HEIGHT);
    let rect = Rect::new(LIST_X, LIST_TOP + LIST_PITCH - s, LIST_WIDTH, height);
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherLocation,
            rect,
            title: language.t("location"),
            icon: Some("location"),
            value: None,
            description: None,
            selected: false,
            chevron: true,
        },
    );
    if let Some(city) = city {
        out.text(
            city,
            Rect::new(
                rect.x + rect.w - 56. - 400.,
                rect.y + (height - city_lines as f32 * line) / 2.,
                400.,
                city_lines as f32 * line,
            ),
            style.right(),
            if *state.page_focus() == Target::WeatherLocation {
                c(0x2c2c2c, 0.62)
            } else {
                c(0xffffff, 0.62)
            },
            false,
        );
    }
    let y = rect.y + height + ROW_SPACING;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherTemperature,
            rect: Rect::new(LIST_X, y, LIST_WIDTH, ROW_HEIGHT),
            title: language.t("temperature"),
            icon: Some("thermometer"),
            value: Some(state.preferences.temperature_unit.symbol()),
            description: None,
            selected: false,
            chevron: true,
        },
    );
}

fn crypto_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    let on_off = |on: bool| {
        Some(if on {
            language.t("stateOn")
        } else {
            language.t("stateOff")
        })
    };
    let chosen = |symbol: &str| state.preferences.crypto_coins.iter().any(|c| c == symbol);
    // BNB and XRP are their own names.
    let title = |symbol: &str| match crate::crypto::name(symbol) {
        name if name == symbol => symbol.to_owned(),
        name => format!("{name} · {symbol}"),
    };
    let simple = |out: &mut Scene, target, y: f32, title: &str, icon, value| {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(LIST_X, y - s, LIST_WIDTH, ROW_HEIGHT),
                title,
                icon,
                value,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    };
    simple(
        out,
        Target::CryptoToggle,
        LIST_TOP,
        language.t("showWidget"),
        Some("eye"),
        on_off(state.preferences.show_crypto),
    );
    let mut y = LIST_TOP + LIST_PITCH + 16.;
    out.text(
        language.format(
            "cryptoCoins",
            &[("count", &crate::crypto::MAX_COINS.to_string())],
        ),
        Rect::new(LIST_X + 24., y - s, LIST_WIDTH - 48., 28.),
        text(type_scale::CAPTION, 450, 0.).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
    y += 40.;
    let search = &state.crypto.search;
    simple(
        out,
        Target::CryptoSearch,
        y,
        language.t("cryptoFind"),
        Some("search"),
        (!search.query.is_empty()).then_some(search.query.as_str()),
    );
    y += LIST_PITCH;
    if !search.query.is_empty() {
        let matches = search.matches();
        let status = if search.loading {
            Some("cryptoSearching")
        } else if search.failed && search.catalog.is_empty() {
            Some("cryptoCatalogError")
        } else if matches.is_empty() {
            Some("cryptoNoMatch")
        } else {
            None
        };
        if let Some(key) = status {
            out.text(
                language.t(key),
                Rect::new(LIST_X + 24., y + 4. - s, LIST_WIDTH - 48., 28.),
                text(type_scale::CAPTION, 450, 0.).shadow(),
                c(0xf5f5f5, 0.72),
                false,
            );
            y += 52.;
        }
        for (index, symbol) in matches.into_iter().enumerate() {
            simple(
                out,
                Target::CryptoResult(index),
                y,
                &title(symbol),
                None,
                on_off(chosen(symbol)),
            );
            y += LIST_PITCH;
        }
        y += 20.;
    }
    for (index, symbol) in state.crypto_choices().iter().enumerate() {
        simple(
            out,
            Target::CryptoCoin(index),
            y,
            &title(symbol),
            None,
            on_off(chosen(symbol)),
        );
        y += LIST_PITCH;
    }
}

fn cs2_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    let preferences = &state.preferences;
    for (index, (target, key, icon, on)) in [
        (Target::Cs2Toggle, "showWidget", "eye", preferences.show_cs2),
        (
            Target::Cs2Layout,
            "cryptoLayout",
            "grid",
            preferences.center() == Some(crate::model::Center::Cs2),
        ),
        (
            Target::Cs2Major,
            "cs2MajorOnly",
            "sparkles",
            preferences.cs2_major_only,
        ),
        (
            Target::Cs2Results,
            "cs2Results",
            "check",
            preferences.cs2_results,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let layout = target == Target::Cs2Layout;
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - s,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title: language.t(key),
                icon: Some(icon),
                value: Some(if layout {
                    place(language, state, crate::model::Center::Cs2)
                } else {
                    language.t(if on { "stateOn" } else { "stateOff" })
                }),
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
    out.text(
        language.t("cs2Source"),
        Rect::new(
            LIST_X + 24.,
            LIST_TOP + 4. * LIST_PITCH + 6. - s,
            LIST_WIDTH - 48.,
            56.,
        ),
        text(type_scale::CAPTION, 450, 0.).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
}

fn location_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let mut y = LIST_TOP - state.scroll;
    if let Some(place) = &state.preferences.weather_location {
        let rect = Rect::new(LIST_X, y, LIST_WIDTH, 88.);
        settings_surface(out, rect, 16.);
        out.icon(
            assets,
            "location",
            Rect::new(rect.x + 24., y + 31., 26., 26.),
            c(0xe4e4e4, 0.7),
        );
        out.text(
            &place.name,
            Rect::new(rect.x + 68., y + 14., rect.w - 160., 31.),
            assets.fit_text(
                &place.name,
                text(type_scale::BODY, 550, -0.02),
                rect.w - 160.,
            ),
            WHITE,
            false,
        );
        out.text(
            place.detail(),
            Rect::new(rect.x + 68., y + 47., rect.w - 160., 27.),
            text(type_scale::CAPTION, 450, 0.),
            c(0xf5f5f5, 0.62),
            false,
        );
        let clear = Rect::new(rect.x + rect.w - 68., y + 20., 48., 48.);
        out.shape(clear, 24., c(0x23282c, 0.5));
        out.border(clear, 24., 1., c(0xffffff, 0.07));
        out.icon(assets, "close", clear.inset(14.), WHITE);
        if *state.page_focus() == Target::WeatherClear {
            out.border(clear.inset(-2.), 26., 2., c(0xf0f0f0, 0.95));
        }
        out.control(Target::WeatherClear, clear, true);
        y += 88. + 20.;
    }
    let query = Rect::new(LIST_X, y, LIST_WIDTH - 84., ROW_HEIGHT);
    let focused = *state.page_focus() == Target::WeatherQuery;
    if !focused {
        out.glass(query, query.h / 2.);
    } else {
        out.shape(query, query.h / 2., c(0x303438, 1.));
        out.border(query, query.h / 2., 1., c(0xffffff, 0.70));
        out.border(query.inset(-3.), query.h / 2. + 3., 3., c(0xffffff, 0.063));
    }
    out.icon(
        assets,
        "search",
        Rect::new(query.x + 24., y + 21., 26., 26.),
        c(0xeeeeee, 0.73),
    );
    out.text(
        if state.weather.query.is_empty() {
            language.t("cityPostcode")
        } else {
            &state.weather.query
        },
        Rect::new(query.x + 68., y + 18.5, query.w - 92., 31.),
        text(type_scale::BODY, 450, -0.02),
        c(0xeeeeee, 0.73),
        false,
    );
    out.control(Target::WeatherQuery, query, true);
    let search = Rect::new(LIST_X + LIST_WIDTH - ROW_HEIGHT, y, ROW_HEIGHT, ROW_HEIGHT);
    out.glass(search, search.h / 2.);
    out.border(search, search.h / 2., 1., c(0xffffff, 0.07));
    out.icon(
        assets,
        if state.weather.searching {
            "refresh"
        } else {
            "search"
        },
        search.inset(21.),
        WHITE,
    );
    if *state.page_focus() == Target::WeatherSearch {
        out.border(search.inset(-7.), search.h / 2. + 7., 2., c(0xf0f0f0, 0.95));
    }
    out.control(Target::WeatherSearch, search, true);
    y += LIST_PITCH;
    if state.weather.searching || state.weather.message.is_some() {
        out.text(
            if state.weather.searching {
                language.t("searching")
            } else {
                language.t(state.weather.message.as_deref().unwrap_or(""))
            },
            Rect::new(LIST_X + 24., y, LIST_WIDTH - 48., 58.),
            text(type_scale::CAPTION, 450, 0.).shadow(),
            c(0xf0f0f0, 0.8),
            false,
        );
    }
    for (index, place) in state.weather.results.iter().enumerate() {
        let detail = place.detail();
        let room = LIST_WIDTH - 68. - 56.;
        let height = (18.5
            + assets.lines(&place.name, text(type_scale::BODY, 550, -0.02), room) as f32 * 31.
            + 6.
            + assets.lines(&detail, text(type_scale::CAPTION, 450, 0.), room) as f32 * 27.
            + 20.)
            .max(ROW_GROWN);
        let rect = Rect::new(LIST_X, y, LIST_WIDTH, height);
        let chosen = state
            .preferences
            .weather_location
            .as_ref()
            .is_some_and(|saved| saved.key() == place.key());
        row(
            out,
            state,
            assets,
            Row {
                target: Target::WeatherResult(index),
                rect,
                title: &place.name,
                icon: Some("location"),
                value: None,
                description: Some(&detail),
                selected: chosen,
                chevron: false,
            },
        );
        y += height + ROW_SPACING;
    }
}

fn settings(out: &mut Scene, state: &State, assets: &Assets) {
    let start = out.draw.len();
    // Lists scroll under the title through a soft edge; the gallery masks itself.
    let list = !(state.section == crate::state::BACKGROUND_SECTION && state.subpage.is_none());
    if list {
        let viewport = state.list_viewport();
        out.draw.push(Draw::Clip(Some(viewport)));
        out.draw.push(Draw::EdgeFade(Some([
            viewport.y,
            LIST_TOP,
            viewport.y + viewport.h - 40.,
            viewport.y + viewport.h,
        ])));
    }
    settings_content(out, state, assets);
    if list {
        out.draw.push(Draw::EdgeFade(None));
        out.draw.push(Draw::Clip(None));
    }
    let title_width = if list {
        LIST_WIDTH
    } else {
        gallery_actions(state, assets)[1].1.x - 24. - LIST_X
    };
    settings_title(out, state, assets, title_width);
    match state.section {
        0 | WIDGETS_SECTION => preview_frame(out, state),
        SYSTEM_SECTION if state.subpage.is_none() => system_summary(out, state, assets),
        _ => {}
    }
    // The open sidebar lies over the page, which steps back behind it.
    let dim = sidebar_dim(state);
    if dim < 1. {
        out.fade(start, 1., dim);
    }
    settings_sidebar(out, state, assets);
}

/// A gallery tile's place: four columns from the list's edge to the safe edge.
fn gallery_rect(state: &State, slot: usize) -> Rect {
    Rect::new(
        LIST_X + (slot % GALLERY_COLUMNS) as f32 * (GALLERY_TILE_WIDTH + GALLERY_GAP_X),
        GALLERY_TOP + (slot / GALLERY_COLUMNS) as f32 * (GALLERY_TILE_HEIGHT + GALLERY_GAP_Y)
            - state.scroll,
        GALLERY_TILE_WIDTH,
        GALLERY_TILE_HEIGHT,
    )
}

/// The Background page: every wallpaper as a tile under the title, upload last, with the two
/// switches beside the title.
fn gallery(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let viewport = state.backgrounds_viewport();
    out.draw.push(Draw::Clip(Some(viewport)));
    out.draw.push(Draw::EdgeFade(Some([
        viewport.y,
        GALLERY_TOP - 8.,
        viewport.y + viewport.h - 40.,
        viewport.y + viewport.h,
    ])));
    for index in 0..state.background_count() {
        let id = state.background_id(index).unwrap();
        let rect = gallery_rect(state, index);
        out.shadow(rect, 16., 15., 3., 0.13);
        background_card(out, assets, state, id, rect);
        if id == crate::aerials::ID {
            aerials_caption(out, assets, state, rect);
        }
        out.border(rect, 16., 1., c(0xffffff, 0.086));
        if state.preferences.background == id {
            out.border(rect.inset(-2.), 18., 2., c(0xdedede, 1.));
            out.border(rect.inset(-7.), 23., 3., c(0xdedede, 0.10));
            let check = Rect::new(rect.x + rect.w - 28., rect.y + rect.h - 28., 21.6, 21.6);
            out.shape(check, 10.8, c(0xf3f3f3, 1.));
            out.icon(assets, "check", check.inset(3.6), c(0x333333, 1.));
        }
        if *state.page_focus() == Target::Background(index) {
            out.ring(rect, 16.);
        }
        out.control(Target::Background(index), rect, true);
    }
    let rect = gallery_rect(state, state.background_count());
    let focused = *state.page_focus() == Target::UploadWallpaper;
    if focused {
        out.shape(rect, 16., c(0xffffff, 0.10));
    }
    dashed_border(out, rect, 16., c(0xffffff, 0.45));
    let label = language.t("uploadWallpaper");
    let style = text(type_scale::SECONDARY, 550, -0.01);
    let width = rect.w - 48.;
    out.icon(
        assets,
        "upload",
        Rect::new(
            rect.x + (rect.w - 32.) / 2.,
            rect.y + rect.h / 2. - 44.,
            32.,
            32.,
        ),
        c(0xffffff, 0.86),
    );
    out.text(
        label,
        Rect::new(rect.x + 24., rect.y + rect.h / 2. + 4., width, 30.),
        assets.fit_text(label, style, width),
        c(0xffffff, 0.86),
        true,
    );
    if focused {
        out.ring(rect, 16.);
    }
    out.control(Target::UploadWallpaper, rect, true);
    out.draw.push(Draw::EdgeFade(None));
    out.draw.push(Draw::Clip(None));
    for (target, rect, icon, title, value) in gallery_actions(state, assets) {
        gallery_action(out, state, assets, target, rect, icon, title, value);
    }
}

/// The gallery's switches, laid right to left from the safe edge: rotation, then the shade.
fn gallery_actions(
    state: &State,
    assets: &Assets,
) -> [(Target, Rect, &'static str, &'static str, &'static str); 2] {
    let language = state.preferences.language;
    let on_off = |on: bool| language.t(if on { "stateOn" } else { "stateOff" });
    // Sized for the longer state, so a switch keeps its width when it flips; measured as
    // `gallery_action` places it.
    let value_width = ["stateOn", "stateOff"]
        .map(|key| {
            (assets.width(language.t(key), text(type_scale::SECONDARY, 450, 0.)) + 1.).ceil()
        })
        .into_iter()
        .fold(0., f32::max);
    let cycle = state.preferences.wallpaper_cycle;
    let gradient = state.preferences.wallpaper_gradient;
    let mut right = SAFE_X + WIDTH;
    [
        (
            Target::WallpaperCycle,
            if cycle { "refresh" } else { "refresh-off" },
            language.t("wallpaperCycle"),
            on_off(cycle),
        ),
        (
            Target::WallpaperGradient,
            if gradient { "eye" } else { "eye-off" },
            language.t("wallpaperGradient"),
            on_off(gradient),
        ),
    ]
    .map(|(target, icon, title, value)| {
        let title_width = assets.width(title, text(type_scale::SECONDARY, 550, -0.01));
        let width = (20. + 24. + 12. + title_width + 12. + value_width + 24.)
            .ceil()
            .min(560.);
        let rect = Rect::new(right - width, 148., width, 56.);
        right = rect.x - 16.;
        (target, rect, icon, title, value)
    })
}

#[allow(clippy::too_many_arguments)]
fn gallery_action(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    target: Target,
    rect: Rect,
    icon: &str,
    title: &str,
    value: &str,
) {
    let focused = *state.page_focus() == target;
    if focused {
        out.draw.push(Draw::RoundedGradient(
            rect,
            28.,
            c(0xfafafa, 1.),
            c(0xe5e5e5, 1.),
        ));
    } else {
        settings_surface(out, rect, 28.);
    }
    let (ink, quiet, mark) = if focused {
        (c(0x2c2c2c, 1.), c(0x2c2c2c, 0.62), c(0x444444, 1.))
    } else {
        (c(0xf4f4f4, 0.92), c(0xffffff, 0.62), c(0xe4e4e4, 0.7))
    };
    out.icon(
        assets,
        icon,
        Rect::new(rect.x + 20., rect.y + 16., 24., 24.),
        mark,
    );
    let value_style = text(type_scale::SECONDARY, 450, 0.).right();
    let value_width = (assets.width(value, value_style) + 1.).ceil();
    let right = rect.x + rect.w - 24.;
    out.text(
        value,
        Rect::new(right - value_width, rect.y + 13.5, value_width, 29.),
        value_style,
        quiet,
        false,
    );
    let x = rect.x + 56.;
    let width = right - value_width - 12. - x;
    out.text(
        title,
        Rect::new(x, rect.y + 13.5, width, 29.),
        assets.fit_text(title, text(type_scale::SECONDARY, 550, -0.01), width),
        ink,
        false,
    );
    out.control(target, rect, false);
}

/// A dashed outline along a rounded rect's straight edges: 2 px dashes, 14 long and 10 apart,
/// each edge's run centred between its corners.
fn dashed_border(out: &mut Scene, rect: Rect, radius: f32, color: Color) {
    const DASH: f32 = 14.;
    const GAP: f32 = 10.;
    const WIDTH: f32 = 2.;
    let run = |length: f32| {
        let room = length - 2. * radius;
        let count = ((room + GAP) / (DASH + GAP)).floor().max(0.) as usize;
        let used = count as f32 * (DASH + GAP) - GAP;
        (count, radius + (room - used) / 2.)
    };
    let (across, from) = run(rect.w);
    for i in 0..across {
        let x = rect.x + from + i as f32 * (DASH + GAP);
        out.shape(Rect::new(x, rect.y, DASH, WIDTH), 1., color);
        out.shape(
            Rect::new(x, rect.y + rect.h - WIDTH, DASH, WIDTH),
            1.,
            color,
        );
    }
    let (down, from) = run(rect.h);
    for i in 0..down {
        let y = rect.y + from + i as f32 * (DASH + GAP);
        out.shape(Rect::new(rect.x, y, WIDTH, DASH), 1., color);
        out.shape(
            Rect::new(rect.x + rect.w - WIDTH, y, WIDTH, DASH),
            1.,
            color,
        );
    }
}

const PREVIEW: Rect = Rect::new(PREVIEW_X, PREVIEW_Y, PREVIEW_WIDTH, PREVIEW_HEIGHT);

/// Beside General and Widgets: what Home will look like, with a line on what it shows. Home
/// itself is drawn into the slot later, by `home_preview`.
fn preview_frame(out: &mut Scene, state: &State) {
    let language = state.preferences.language;
    out.text(
        language.t("previewTitle").to_uppercase(),
        Rect::new(
            PREVIEW.x,
            PREVIEW.y - 36.,
            PREVIEW.w,
            type_scale::EYEBROW * 1.2,
        ),
        text(type_scale::EYEBROW, 600, 0.12).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
    out.shadow(PREVIEW, 18., 30., 8., 0.35);
    out.preview = Some(PreviewSlot {
        at: out.draw.len(),
        region: preview_region(state),
    });
    out.border(PREVIEW, 18., 1., c(0xffffff, 0.14));
    out.text(
        language.t("previewHint"),
        Rect::new(PREVIEW.x, PREVIEW.y + PREVIEW.h + 24., PREVIEW.w, 60.),
        text(type_scale::CAPTION, 450, 0.).leading(1.35).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
}

/// The part of Home the focused setting, or the open subpage, changes.
fn preview_region(state: &State) -> Option<Region> {
    if let Some(subpage) = &state.subpage {
        return match subpage {
            Subpage::AppNames | Subpage::Shortcuts => Some(Region::Favorites),
            Subpage::Clock | Subpage::ClockFormat | Subpage::TimeFormat => Some(Region::Clock),
            Subpage::Date | Subpage::DateFormat => Some(Region::Date),
            Subpage::Greeting => Some(Region::Greeting),
            Subpage::Weather | Subpage::WeatherLocation | Subpage::Temperature => {
                Some(Region::Weather)
            }
            Subpage::Inputs | Subpage::Input(_) => Some(Region::Inputs),
            Subpage::Crypto => Some(Region::Crypto),
            Subpage::Cs2 => Some(Region::Cs2),
            Subpage::Shows | Subpage::LampaFeed => Some(Region::Shows),
            Subpage::Apps => Some(Region::Apps),
            _ => None,
        };
    }
    match state.page_focus() {
        Target::Setting(0) | Target::Widget(2) => Some(Region::Favorites),
        Target::Setting(index) if *index == crate::state::FOCUS_GLOW_ROW => Some(Region::Favorites),
        Target::Setting(1) => Some(Region::Clock),
        Target::Widget(4) => Some(Region::Date),
        Target::Widget(0) => Some(Region::Greeting),
        Target::Widget(1) => Some(Region::Weather),
        Target::Widget(3) => Some(Region::Inputs),
        Target::Widget(5) => Some(Region::Crypto),
        Target::Widget(6) => Some(Region::Cs2),
        Target::Widget(7) => Some(Region::Shows),
        Target::Widget(8) => Some(Region::Apps),
        _ => None,
    }
}

/// Draws Home, small, where Settings left room for it: Home's own scene, built as Home would
/// draw it at rest, scaled into the frame. It takes no focus, and Settings' state is restored.
pub fn home_preview(scene: &mut Scene, state: &mut State, assets: &Assets, now: OffsetDateTime) {
    let Some(slot) = scene.preview.take() else {
        return;
    };
    let dim = sidebar_dim(state);
    let saved = (
        std::mem::replace(&mut state.page, Page::Home),
        std::mem::replace(&mut state.focus, Target::Nav(0)),
        state.menu_motion.take(),
        std::mem::replace(&mut state.scroll, 0.),
        std::mem::replace(&mut state.home_scroll, 0.),
        // No focus ring anywhere.
        std::mem::replace(&mut state.pointer, true),
        state.overlay.take(),
        state.notice.take(),
        state.error.take(),
        state.pressed.take(),
        state.moving.take(),
    );
    let home = self::scene(state, assets, now);
    (
        state.page,
        state.focus,
        state.menu_motion,
        state.scroll,
        state.home_scroll,
        state.pointer,
        state.overlay,
        state.notice,
        state.error,
        state.pressed,
        state.moving,
    ) = saved;
    let k = PREVIEW.w / 1920.;
    let place = |rect: Rect| {
        Rect::new(
            PREVIEW.x + rect.x * k,
            PREVIEW.y + rect.y * k,
            rect.w * k,
            rect.h * k,
        )
    };
    let within = |rect: Option<Rect>| {
        let Some(rect) = rect.map(place) else {
            return PREVIEW;
        };
        let x = rect.x.max(PREVIEW.x);
        let y = rect.y.max(PREVIEW.y);
        let right = (rect.x + rect.w).min(PREVIEW.x + PREVIEW.w);
        let bottom = (rect.y + rect.h).min(PREVIEW.y + PREVIEW.h);
        Rect::new(x, y, (right - x).max(0.), (bottom - y).max(0.))
    };
    // The wallpaper fills the frame, whose corners are round.
    let corner = |rect: Rect, radius: f32| {
        if rect.w >= 1920. && rect.h >= 1080. {
            18.
        } else {
            radius * k
        }
    };
    let mut small = Scene::new();
    small.draw.push(Draw::Clip(Some(PREVIEW)));
    for draw in home.draw {
        small.draw.push(match draw {
            Draw::Shape(rect, radius, width, color) => Draw::Shape(
                place(rect),
                radius * k,
                if width > 0. { (width * k).max(1.) } else { 0. },
                color,
            ),
            Draw::RoundedGradient(rect, radius, a, b) => {
                Draw::RoundedGradient(place(rect), radius * k, a, b)
            }
            Draw::Shadow(rect, radius, blur, color) => {
                Draw::Shadow(place(rect), radius * k, blur * k, color)
            }
            Draw::Text(label, rect, mut style, color, center) => {
                style.size *= k;
                Draw::Text(label, place(rect), style, color, center)
            }
            Draw::Image(path, rect, radius, contain) => {
                Draw::Image(path, place(rect), corner(rect, radius), contain)
            }
            Draw::TintedImage(path, rect, radius, contain, color) => {
                Draw::TintedImage(path, place(rect), corner(rect, radius), contain, color)
            }
            Draw::AppImage(art, rect, radius, color) => {
                Draw::AppImage(art, place(rect), radius * k, color)
            }
            Draw::Picture(art, rect, radius, color) => {
                Draw::Picture(art, place(rect), radius * k, color)
            }
            Draw::Cover(art, rect, radius, color) => {
                Draw::Cover(art, place(rect), radius * k, color)
            }
            // Home's glass samples the wallpaper drawn in the frame.
            Draw::Glass(path, rect, radius, source, tint) => {
                Draw::Glass(path, place(rect), radius * k, place(source), tint)
            }
            Draw::Clip(rect) => Draw::Clip(Some(within(rect))),
            Draw::EdgeFade(fade) => {
                Draw::EdgeFade(fade.map(|edges| edges.map(|y| PREVIEW.y + y * k)))
            }
            Draw::EdgeFadeX(fade) => {
                Draw::EdgeFadeX(fade.map(|edges| edges.map(|x| PREVIEW.x + x * k)))
            }
        });
    }
    small.draw.push(Draw::EdgeFade(None));
    small.draw.push(Draw::EdgeFadeX(None));
    small.draw.push(Draw::Clip(None));
    if let Some(rect) = slot.region.and_then(|region| {
        home.regions
            .iter()
            .find(|(drawn, _)| *drawn == region)
            .map(|(_, rect)| place(*rect).inset(-6.))
    }) {
        small.border(rect.inset(-3.), 13., 3., c(0xffffff, 0.18));
        small.border(rect, 10., 2., c(0xffffff, 0.95));
    }
    if dim < 1. {
        small.fade(0, 1., dim);
    }
    scene.draw.splice(slot.at..slot.at, small.draw);
}

/// How bright the page under the sidebar is: full beside the icons, dimmed once it is open.
fn sidebar_dim(state: &State) -> f32 {
    let open = (state.sidebar_width() - SIDEBAR_COLLAPSED) / (SIDEBAR_EXPANDED - SIDEBAR_COLLAPSED);
    1. - 0.45 * open.clamp(0., 1.)
}

const SECTIONS: [(&str, &str); 4] = [
    ("interface", "settings"),
    ("background", "photo"),
    ("widgets", "sparkles"),
    ("system", "tv"),
];

/// The page's title: the section, then the open subpage's path ("Widgets › Weather").
fn breadcrumb(state: &State) -> Vec<&str> {
    let language = state.preferences.language;
    let mut path = vec![language.t(SECTIONS[state.section.min(SECTIONS.len() - 1)].0)];
    match &state.subpage {
        Some(Subpage::DateFormat) => path.extend([language.t("date"), language.t("dateFormat")]),
        Some(Subpage::LampaFeed) => path.extend([language.t("lampa"), language.t("lampaFeed")]),
        Some(Subpage::ClockFormat) => path.extend([language.t("clock"), language.t("clockFormat")]),
        Some(Subpage::TimeFormat) => path.extend([language.t("clock"), language.t("timeFormat")]),
        Some(Subpage::WeatherLocation) => {
            path.extend([language.t("weather"), language.t("location")])
        }
        Some(Subpage::Temperature) => {
            path.extend([language.t("weather"), language.t("temperature")])
        }
        Some(Subpage::Input(id)) => path.extend([
            language.t("inputs"),
            state
                .inventory
                .inputs
                .iter()
                .find(|input| &input.id == id)
                .map_or(language.t("inputs"), |input| {
                    state.preferences.input_title(input)
                }),
        ]),
        Some(subpage) => path.push(match subpage {
            Subpage::AppNames => language.t("appNames"),
            Subpage::Greeting => language.t("greeting"),
            Subpage::Date => language.t("date"),
            Subpage::Weather => language.t("weather"),
            Subpage::Shortcuts => language.t("shortcuts"),
            Subpage::Crypto => language.t("crypto"),
            Subpage::Inputs => language.t("inputs"),
            Subpage::Clock => language.t("clock"),
            Subpage::Cs2 => language.t("cs2Score"),
            Subpage::Shows => language.t("lampa"),
            Subpage::Apps => language.t("appsWidget"),
            Subpage::Language => language.t("language"),
            Subpage::Appearance => language.t("uiScale"),
            Subpage::TextRendering => language.t("textRendering"),
            Subpage::HomeTakeover => language.t("homeTakeover"),
            _ => "",
        }),
        None => {}
    }
    path.retain(|part| !part.is_empty());
    path
}

/// The section's name, or the subpage's path, large above the list; it shrinks to `width`.
fn settings_title(out: &mut Scene, state: &State, assets: &Assets, width: f32) {
    const SEPARATOR: f32 = 26.;
    const GAP: f32 = 10.;
    let path = breadcrumb(state);
    let separators = path.len().saturating_sub(1) as f32 * (SEPARATOR + GAP * 2.);
    let mut style = text(type_scale::PAGE_TITLE, 600, -0.02).shadow();
    let words: f32 = path.iter().map(|part| assets.width(part, style)).sum();
    if words > 0. {
        style.size *= ((width - separators) / words).min(1.);
    }
    let height = (type_scale::PAGE_TITLE * 1.2).ceil();
    let muted = c(0xffffff, 0.62);
    let mut x = LIST_X;
    for (position, part) in path.iter().enumerate() {
        if position > 0 {
            out.shadowed_icon(
                assets,
                "right",
                Rect::new(
                    x + GAP,
                    CONTENT_TOP + (height - SEPARATOR) / 2.,
                    SEPARATOR,
                    SEPARATOR,
                ),
                muted,
            );
            x += SEPARATOR + GAP * 2.;
        }
        let part_width = assets.width(part, style);
        out.text(
            *part,
            Rect::new(x, CONTENT_TOP, part_width + 2., height),
            style,
            if position + 1 == path.len() {
                WHITE
            } else {
                muted
            },
            false,
        );
        x += part_width;
    }
}

/// The sections: icons in a column while the page has focus, opening over the page, with their
/// names, while the sidebar has it.
fn settings_sidebar(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let width = state.sidebar_width();
    let open = ((width - SIDEBAR_COLLAPSED) / (SIDEBAR_EXPANDED - SIDEBAR_COLLAPSED)).clamp(0., 1.);
    let panel = Rect::new(
        SAFE_X,
        CONTENT_TOP,
        width,
        8. + SECTIONS.len() as f32 * SIDEBAR_PITCH,
    );
    let radius = SIDEBAR_COLLAPSED / 2.;
    out.shadow(panel, radius, 24., 6., 0.10 + 0.14 * open);
    out.glass(panel, radius);
    // Names are laid out at the open width; while it opens they are cut at its edge.
    let opening = open > 0. && open < 1.;
    if opening {
        out.draw.push(Draw::Clip(Some(panel)));
    }
    for (index, (key, icon)) in SECTIONS.into_iter().enumerate() {
        let rect = Rect::new(
            SAFE_X + 8.,
            CONTENT_TOP + 8. + index as f32 * SIDEBAR_PITCH,
            width - 16.,
            SIDEBAR_ITEM,
        );
        let focused = *state.page_focus() == Target::Sidebar(index);
        let selected = state.section == index;
        if focused {
            out.draw.push(Draw::RoundedGradient(
                rect,
                SIDEBAR_ITEM / 2.,
                c(0xfafafa, 1.),
                c(0xe5e5e5, 1.),
            ));
        } else if selected {
            out.shape(rect, SIDEBAR_ITEM / 2., c(0xffffff, 0.16));
        }
        let ink = if focused {
            c(0x2c2c2c, 1.)
        } else if selected {
            WHITE
        } else {
            c(0xeeeeee, 0.73)
        };
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 16., rect.y + 16., 28., 28.),
            ink,
        );
        if open > 0. {
            let label = language.t(key);
            // Measured at the open width, so the names do not shrink while the sidebar opens.
            let room = SIDEBAR_EXPANDED - 16. - 76.;
            let mut color = ink;
            color[3] *= open;
            out.text(
                label,
                Rect::new(rect.x + 60., rect.y + 14.5, room, 31.),
                assets.fit_text(label, text(type_scale::BODY, 550, -0.02), room),
                color,
                false,
            );
        }
        out.control(Target::Sidebar(index), rect, false);
    }
    if opening {
        out.draw.push(Draw::Clip(None));
    }
}

fn settings_content(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    if let Some(subpage) = &state.subpage {
        let start = out.draw.len();
        if matches!(
            subpage,
            Subpage::WeatherLocation
                | Subpage::Cs2
                | Subpage::Shows
                | Subpage::Apps
                | Subpage::Weather
                | Subpage::Shortcuts
                | Subpage::Inputs
                | Subpage::Crypto
                | Subpage::Greeting
                | Subpage::Date
                | Subpage::AppNames
                | Subpage::Clock
        ) {
            match subpage {
                Subpage::WeatherLocation => location_settings(out, state, assets),
                Subpage::Cs2 => cs2_settings(out, state, assets),
                Subpage::Shows => shows_settings(out, state, assets),
                Subpage::Apps => apps_settings(out, state, assets),
                Subpage::Weather => weather_settings(out, state, assets),
                Subpage::Shortcuts => shortcut_settings(out, state, assets),
                Subpage::Inputs => inputs_settings(out, state, assets),
                Subpage::Crypto => crypto_settings(out, state, assets),
                Subpage::Greeting => greeting_settings(out, state, assets),
                Subpage::Date => date_settings(out, state, assets),
                Subpage::AppNames => app_name_settings(out, state, assets),
                Subpage::Clock => clock_settings(out, state, assets),
                _ => unreachable!(),
            }
            if let Some(opened) = state.panel_started {
                out.fade(start, (opened.elapsed().as_secs_f32() / 0.15).min(1.), 1.);
            }
            return;
        }
        let options: Vec<(&str, Option<&str>, bool)> = match subpage {
            Subpage::LampaFeed => crate::model::LampaFeed::ALL
                .iter()
                .map(|feed| {
                    (
                        language.t(feed.key()),
                        None,
                        *feed == state.preferences.lampa_feed,
                    )
                })
                .collect(),
            Subpage::DateFormat => crate::model::DateFormat::ALL
                .iter()
                .map(|format| {
                    (
                        format.label(language),
                        None,
                        *format == state.preferences.date_format,
                    )
                })
                .collect(),
            Subpage::ClockFormat => vec![
                (language.t("hour12"), None, !state.preferences.hour_24),
                (language.t("hour24"), None, state.preferences.hour_24),
            ],
            Subpage::TimeFormat => vec![
                ("HH:MM", None, !state.preferences.show_seconds),
                ("HH:MM:SS", None, state.preferences.show_seconds),
            ],
            Subpage::Input(id) => {
                let current = state
                    .preferences
                    .input_modes
                    .get(id)
                    .copied()
                    .unwrap_or_default();
                vec![
                    (
                        language.t("stateOn"),
                        Some(language.t("inputAlways")),
                        current == InputMode::On,
                    ),
                    (
                        language.t("stateOff"),
                        Some(language.t("inputNever")),
                        current == InputMode::Off,
                    ),
                    (
                        language.t("stateAuto"),
                        Some(language.t(if id == "airplay" {
                            "airplayInputAuto"
                        } else {
                            "inputWhenConnected"
                        })),
                        current == InputMode::Auto,
                    ),
                ]
            }
            Subpage::Temperature => vec![
                (
                    language.t("celsius"),
                    None,
                    state.preferences.temperature_unit == Unit::Celsius,
                ),
                (
                    language.t("fahrenheit"),
                    None,
                    state.preferences.temperature_unit == Unit::Fahrenheit,
                ),
            ],
            Subpage::WeatherLocation
            | Subpage::Cs2
            | Subpage::Shows
            | Subpage::Apps
            | Subpage::Weather
            | Subpage::Shortcuts
            | Subpage::Inputs
            | Subpage::Crypto
            | Subpage::Greeting
            | Subpage::Date
            | Subpage::AppNames
            | Subpage::Clock => {
                unreachable!()
            }
            Subpage::Language => Language::ALL
                .into_iter()
                .map(|option| (option.name(), None, option == language))
                .collect(),
            Subpage::Appearance => UiSize::ALL
                .into_iter()
                .map(|size| {
                    let (name, hint) = size.keys();
                    (
                        language.t(name),
                        Some(language.t(hint)),
                        size == state.preferences.ui_scale,
                    )
                })
                .collect(),
            Subpage::TextRendering => crate::text_rendering::TextRendering::ALL
                .into_iter()
                .map(|mode| {
                    (
                        language.t(mode.label_key()),
                        Some(language.t("textRenderingRestart")),
                        mode == state.preferences.text_rendering,
                    )
                })
                .collect(),
            Subpage::HomeTakeover => vec![
                (
                    language.t("stateOn"),
                    Some(language.t("homeTakeoverOnHint")),
                    state.home_takeover,
                ),
                (
                    language.t("stateOff"),
                    Some(language.t("homeTakeoverOffHint")),
                    !state.home_takeover,
                ),
            ],
        };
        let rects = list_rects(
            state,
            LIST_TOP,
            &options
                .iter()
                .enumerate()
                .map(|(index, option)| (Target::Choice(index), option.1.is_some()))
                .collect::<Vec<_>>(),
        );
        let bottom = rects.last().map_or(LIST_TOP - s, |rect| rect.y + rect.h);
        for ((index, (title, description, selected)), rect) in
            options.into_iter().enumerate().zip(rects)
        {
            row(
                out,
                state,
                assets,
                Row {
                    target: Target::Choice(index),
                    rect,
                    title,
                    icon: None,
                    value: None,
                    description,
                    selected,
                    chevron: false,
                },
            );
        }
        if let Subpage::Input(id) = subpage {
            input_name_rows(out, state, assets, id, bottom + 16.);
        }
        if let Some(opened) = state.panel_started {
            out.fade(start, (opened.elapsed().as_secs_f32() / 0.15).min(1.), 1.);
        }
        return;
    }
    match state.section {
        0 => {
            let rows = [
                (0, "appNames", "appNamesAbout", "type", None, true),
                (1, "clock", "clockAbout", "clock", None, true),
                (
                    4,
                    "language",
                    "languageAbout",
                    "language",
                    Some(language.name()),
                    true,
                ),
                (
                    2,
                    "reducedMotion",
                    "reducedMotionAbout",
                    "motion",
                    Some(if state.preferences.reduced_motion {
                        language.t("stateOn")
                    } else {
                        language.t("stateOff")
                    }),
                    false,
                ),
                (
                    crate::state::FOCUS_GLOW_ROW,
                    "focusGlow",
                    "focusGlowAbout",
                    "sparkles",
                    Some(language.t(state.preferences.focus_glow.key())),
                    false,
                ),
                (
                    crate::state::FAVORITES_RECENT_ROW,
                    "favoritesRecent",
                    "favoritesRecentAbout",
                    "clock",
                    Some(language.t(if state.preferences.favorites_recent {
                        "stateOn"
                    } else {
                        "stateOff"
                    })),
                    false,
                ),
                (
                    crate::state::INTERFACE_SCALE_ROW,
                    "uiScale",
                    "uiScaleAbout",
                    "grid",
                    Some(language.t(state.preferences.ui_scale.keys().0)),
                    true,
                ),
            ];
            let rects = list_rects(
                state,
                LIST_TOP,
                &rows.map(|row| (Target::Setting(row.0), true)),
            );
            for ((index, key, about, icon, value, chevron), rect) in rows.into_iter().zip(rects) {
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::Setting(index),
                        rect,
                        title: language.t(key),
                        icon: Some(icon),
                        value,
                        description: Some(language.t(about)),
                        selected: false,
                        chevron,
                    },
                );
            }
        }
        1 => gallery(out, state, assets),
        WIDGETS_SECTION => {
            let rows = [
                (4, "date", "dateAbout", "clock"),
                (0, "greeting", "greetingAbout", "type"),
                (1, "weather", "weatherAbout", "weather-sun"),
                (2, "shortcuts", "shortcutsAbout", "sparkles"),
                (3, "inputs", "inputsAbout", "input-hdmi"),
                (5, "crypto", "cryptoAbout", "globe"),
                // Last, so the rows above keep their places.
                (6, "cs2Score", "cs2ScoreAbout", "gaming"),
                (7, "lampa", "lampaWidgetAbout", "tv"),
                (8, "appsWidget", "appsAbout", "grid"),
            ];
            let rects = list_rects(
                state,
                LIST_TOP,
                &rows.map(|row| (Target::Widget(row.0), true)),
            );
            for ((widget, key, about, icon), rect) in rows.into_iter().zip(rects) {
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::Widget(widget),
                        rect,
                        title: language.t(key),
                        icon: Some(icon),
                        value: None,
                        description: Some(language.t(about)),
                        selected: false,
                        chevron: true,
                    },
                );
            }
        }
        SYSTEM_SECTION => {
            let rows = [
                (
                    3,
                    language.t(if state.loading {
                        "refreshNotice"
                    } else {
                        "refreshApps"
                    }),
                    "refreshAppsAbout",
                    "refresh",
                    None,
                    false,
                ),
                (
                    7,
                    language.t("textRendering"),
                    "textRenderingAbout",
                    "type",
                    Some(language.t(state.preferences.text_rendering.label_key())),
                    true,
                ),
                (
                    HOMEBREW_UPDATES_ROW,
                    language.t("homebrewUpdates"),
                    "homebrewUpdatesAbout",
                    "refresh",
                    Some(language.t(if state.preferences.homebrew_updates {
                        "stateOn"
                    } else {
                        "stateOff"
                    })),
                    false,
                ),
                (
                    crate::state::HOME_TAKEOVER_ROW,
                    language.t("homeTakeover"),
                    "homeTakeoverAbout",
                    "tv",
                    Some(language.t(if state.home_takeover {
                        "stateOn"
                    } else {
                        "stateOff"
                    })),
                    true,
                ),
                (
                    5,
                    language.t("quitApp"),
                    "quitAppAbout",
                    "power",
                    None,
                    false,
                ),
            ];
            let update = if state.available_update.is_some() {
                Target::InstallUpdate
            } else {
                Target::CheckUpdates
            };
            let mut rects = list_rects(
                state,
                LIST_TOP,
                &std::iter::once((update.clone(), false))
                    .chain(rows.iter().map(|row| (Target::Setting(row.0), true)))
                    .collect::<Vec<_>>(),
            )
            .into_iter();
            row(
                out,
                state,
                assets,
                Row {
                    target: update,
                    rect: rects.next().unwrap(),
                    title: &update_value(state),
                    icon: Some(if state.available_update.is_some() {
                        "arrow"
                    } else {
                        "refresh"
                    }),
                    value: None,
                    description: None,
                    selected: false,
                    chevron: false,
                },
            );
            for ((index, title, about, icon, value, chevron), rect) in rows.into_iter().zip(rects) {
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::Setting(index),
                        rect,
                        title,
                        icon: Some(icon),
                        value,
                        description: Some(language.t(about)),
                        selected: false,
                        chevron,
                    },
                );
            }
        }
        _ => {}
    }
}

fn inputs_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    for (index, input) in state.inventory.inputs.iter().enumerate() {
        let mode = state
            .preferences
            .input_modes
            .get(&input.id)
            .copied()
            .unwrap_or_default();
        row(
            out,
            state,
            assets,
            Row {
                target: Target::InputMode(input.id.clone()),
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * TWO_LINE_PITCH - s,
                    LIST_WIDTH,
                    ROW_GROWN,
                ),
                title: state.preferences.input_title(input),
                icon: Some(&format!(
                    "input-{}",
                    input_icon(input, state.preferences.input_name(input))
                )),
                value: Some(language.t(match mode {
                    InputMode::On => "stateOn",
                    InputMode::Off => "stateOff",
                    InputMode::Auto => "stateAuto",
                })),
                description: Some(&input_status(
                    input,
                    state.preferences.input_name(input).is_some(),
                    language,
                )),
                selected: false,
                chevron: true,
            },
        );
    }
    let mut y = LIST_TOP + state.inventory.inputs.len() as f32 * TWO_LINE_PITCH + 16. - s;
    if state.inventory.inputs.is_empty() {
        out.text(
            language.t("noInputs"),
            Rect::new(664., 165. - s, 1000., 26.),
            text(16.8, 400, 0.).leading(1.5),
            c(0xf5f5f5, 0.72),
            false,
        );
        y += 36.;
    }
    row(
        out,
        state,
        assets,
        Row {
            target: Target::Setting(3),
            rect: Rect::new(LIST_X, y, LIST_WIDTH, ROW_HEIGHT),
            title: if state.loading {
                language.t("refreshNotice")
            } else {
                language.t("refreshInputs")
            },
            icon: Some("refresh"),
            value: None,
            description: None,
            selected: false,
            chevron: false,
        },
    );
}

fn greeting_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let description = if state.text_input_active() && state.keyboard_field == TextField::Name {
        format!("{}|", state.greeting_name_draft)
    } else if state.preferences.greeting_name.is_empty() {
        language.t("stateOff").into()
    } else {
        state.preferences.greeting_name.clone()
    };
    row(
        out,
        state,
        assets,
        Row {
            target: Target::GreetingName,
            rect: Rect::new(
                LIST_X,
                LIST_TOP + LIST_PITCH - state.scroll,
                LIST_WIDTH,
                ROW_GROWN,
            ),
            title: language.t("name"),
            icon: Some("type"),
            value: None,
            description: Some(&description),
            selected: false,
            chevron: true,
        },
    );
    // Always present so completing name entry cannot move a clicked control.
    row(
        out,
        state,
        assets,
        Row {
            target: Target::GreetingNameClear,
            rect: Rect::new(
                LIST_X,
                LIST_TOP + LIST_PITCH + TWO_LINE_PITCH - state.scroll,
                LIST_WIDTH,
                ROW_HEIGHT,
            ),
            title: language.t("clearName"),
            icon: Some("refresh"),
            value: None,
            description: None,
            selected: false,
            chevron: false,
        },
    );
    row(
        out,
        state,
        assets,
        Row {
            target: Target::GreetingToggle,
            rect: Rect::new(LIST_X, LIST_TOP - state.scroll, LIST_WIDTH, ROW_HEIGHT),
            title: language.t("showWidget"),
            icon: None,
            value: Some(language.t(if state.preferences.show_greeting {
                "stateOn"
            } else {
                "stateOff"
            })),
            description: None,
            selected: false,
            chevron: false,
        },
    );
}

fn date_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    for (index, (target, title, value, chevron)) in [
        (
            Target::DateToggle,
            "showWidget",
            language.t(if state.preferences.show_date {
                "stateOn"
            } else {
                "stateOff"
            }),
            false,
        ),
        (
            Target::DateFormat,
            "dateFormat",
            state.preferences.date_format.label(language),
            true,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - state.scroll,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title: language.t(title),
                icon: None,
                value: Some(value),
                description: None,
                selected: false,
                chevron,
            },
        );
    }
}

fn clock_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    for (index, (target, key, value)) in [
        (
            Target::ClockFormat,
            "clockFormat",
            language.t(if state.preferences.hour_24 {
                "hour24"
            } else {
                "hour12"
            }),
        ),
        (
            Target::ClockTimeFormat,
            "timeFormat",
            if state.preferences.show_seconds {
                "HH:MM:SS"
            } else {
                "HH:MM"
            },
        ),
    ]
    .into_iter()
    .enumerate()
    {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - state.scroll,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title: language.t(key),
                icon: None,
                value: Some(value),
                description: None,
                selected: false,
                chevron: true,
            },
        );
    }
}

fn app_name_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    for (index, (target, title, enabled)) in [
        (
            Target::AppNamesHome,
            "showOnHome",
            state.preferences.show_home_names,
        ),
        (
            Target::AppNamesApps,
            "showOnApps",
            state.preferences.app_names != crate::model::AppNames::Off,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let value = if target == Target::AppNamesApps {
            language.t(state.preferences.app_names.key())
        } else {
            language.t(if enabled { "stateOn" } else { "stateOff" })
        };
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    LIST_X,
                    LIST_TOP + index as f32 * LIST_PITCH - state.scroll,
                    LIST_WIDTH,
                    ROW_HEIGHT,
                ),
                title: language.t(title),
                icon: None,
                value: Some(value),
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
}

fn input_name_rows(out: &mut Scene, state: &State, assets: &Assets, id: &str, y: f32) {
    let language = state.preferences.language;
    let custom = state.preferences.input_names.get(id);
    let editing = state.text_input_active() && state.keyboard_field == TextField::InputName;
    let description = if editing {
        format!("{}|", state.input_name)
    } else {
        custom
            .cloned()
            .unwrap_or_else(|| language.t("inputNameHint").into())
    };
    row(
        out,
        state,
        assets,
        Row {
            target: Target::InputRename,
            rect: Rect::new(LIST_X, y, LIST_WIDTH, ROW_GROWN),
            title: language.t("renameInput"),
            icon: Some("type"),
            value: None,
            description: Some(&description),
            selected: false,
            chevron: false,
        },
    );
    if custom.is_some() {
        row(
            out,
            state,
            assets,
            Row {
                target: Target::InputNameReset,
                rect: Rect::new(LIST_X, y + TWO_LINE_PITCH, LIST_WIDTH, ROW_HEIGHT),
                title: language.t("clearName"),
                icon: Some("refresh"),
                value: None,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
}

/// How far down System's summary may grow.
const SYSTEM_SUMMARY_BOTTOM: f32 = 1020.;
/// A listed change's text, past its bullet.
const CHANGE_INDENT: f32 = 26.;
const CHANGE_GAP: f32 = 12.;

/// System, on the right where other sections show Home: Luma's version, its updates and what
/// an update on offer changes since the installed build.
fn system_summary(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let title = if matches!(state.update, crate::update::InstallStatus::Failed(_)) {
        "updateError"
    } else if state.available_update.is_some() {
        "updateAvailable"
    } else {
        match state.update_status {
            crate::update::Status::Unchecked => "checkForUpdates",
            crate::update::Status::Checking => "checkingUpdates",
            crate::update::Status::Checked => "latestVersionInstalled",
            crate::update::Status::Failed => "updateCheckFailed",
        }
    };
    let version = if let Some(latest) = &state.available_update {
        language.format(
            "updateVersions",
            &[("current", assets.version.trim()), ("latest", latest)],
        )
    } else {
        language.format("currentVersion", &[("current", assets.version.trim())])
    };
    let inner = PREVIEW.w - 64.;
    let title_style = text(34., 600, -0.02).leading(1.25);
    let title_height = assets.lines(language.t(title), title_style, inner) as f32 * 34. * 1.25;
    let version_style = text(type_scale::SECONDARY, 450, 0.).leading(1.35);
    let version_height =
        assets.lines(&version, version_style, inner) as f32 * type_scale::SECONDARY * 1.35;
    let changes: &[String] = if state.available_update.is_some() {
        &state.update_changes
    } else {
        &[]
    };
    let change_style = text(type_scale::SECONDARY, 450, 0.).leading(1.35);
    let change_line = type_scale::SECONDARY * 1.35;
    let change_width = inner - CHANGE_INDENT;
    let head = PREVIEW.y + 28. + title_height + 10. + version_height;
    let list_top = head + 24. + 24. + type_scale::EYEBROW * 1.2 + 14.;
    // As many changes as fit above the TV's safe margin; the rest are counted.
    let mut fitting = Vec::new();
    let mut end = list_top;
    for (index, change) in changes.iter().enumerate() {
        let height = assets.lines(change, change_style, change_width) as f32 * change_line;
        let counted = if index + 1 < changes.len() {
            CHANGE_GAP + change_line
        } else {
            0.
        };
        if end + height + counted > SYSTEM_SUMMARY_BOTTOM - 30. {
            break;
        }
        fitting.push((change, height));
        end += height + CHANGE_GAP;
    }
    let rest = changes.len() - fitting.len();
    let bottom = if changes.is_empty() {
        head
    } else if rest > 0 {
        end + change_line
    } else {
        end - CHANGE_GAP
    };
    let panel = Rect::new(PREVIEW.x, PREVIEW.y, PREVIEW.w, bottom + 30. - PREVIEW.y);
    out.text(
        "LUMA",
        Rect::new(
            PREVIEW.x,
            PREVIEW.y - 36.,
            PREVIEW.w,
            type_scale::EYEBROW * 1.2,
        ),
        text(type_scale::EYEBROW, 600, 0.12).shadow(),
        c(0xf5f5f5, 0.72),
        false,
    );
    out.glass(panel, 22.);
    let mut y = panel.y + 28.;
    out.text(
        language.t(title),
        Rect::new(panel.x + 32., y, inner, title_height),
        title_style,
        WHITE,
        false,
    );
    y += title_height + 10.;
    out.text(
        version,
        Rect::new(panel.x + 32., y, inner, version_height),
        version_style,
        c(0xffffff, 0.62),
        false,
    );
    if changes.is_empty() {
        return;
    }
    y = head + 24.;
    out.shape(
        Rect::new(panel.x + 32., y, inner, 1.),
        0.,
        c(0xffffff, 0.12),
    );
    out.text(
        language.t("updateChanges").to_uppercase(),
        Rect::new(panel.x + 32., y + 24., inner, type_scale::EYEBROW * 1.2),
        text(type_scale::EYEBROW, 600, 0.12),
        c(0xf5f5f5, 0.6),
        false,
    );
    y = list_top;
    let text_x = panel.x + 32. + CHANGE_INDENT;
    for (change, height) in fitting {
        out.shape(
            Rect::new(panel.x + 34., y + change_line / 2. - 3.5, 7., 7.),
            3.5,
            c(0xffffff, 0.42),
        );
        out.text(
            change,
            Rect::new(text_x, y, change_width, height),
            change_style,
            c(0xffffff, 0.88),
            false,
        );
        y += height + CHANGE_GAP;
    }
    if rest > 0 {
        out.text(
            language.format("updateMoreChanges", &[("count", &rest.to_string())]),
            Rect::new(text_x, y, change_width, change_line),
            text(type_scale::SECONDARY, 450, 0.),
            c(0xffffff, 0.5),
            false,
        );
    }
}

fn settings_surface(out: &mut Scene, rect: Rect, radius: f32) {
    out.glass(rect, radius);
}

fn shortcut_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let tiles = &state.preferences.custom_tiles;
    for (index, tile) in tiles.iter().enumerate() {
        let description = format!("{} · {}", tile.method.label(), shorten(&tile.url));
        let y = LIST_TOP + LIST_PITCH + index as f32 * TWO_LINE_PITCH - state.scroll;
        row(
            out,
            state,
            assets,
            Row {
                target: Target::Tile(tile.id.clone()),
                rect: Rect::new(LIST_X, y, LIST_WIDTH, ROW_GROWN),
                title: &tile.title,
                icon: Some(tile.icon()),
                value: None,
                description: Some(&description),
                selected: false,
                chevron: false,
            },
        );
    }
    if tiles.is_empty() {
        let rect = Rect::new(
            LIST_X,
            LIST_TOP + LIST_PITCH - state.scroll,
            LIST_WIDTH,
            220.,
        );
        settings_surface(out, rect, 20.);
        out.icon(
            assets,
            "sparkles",
            Rect::new(rect.x + (rect.w - 32.) / 2., rect.y + 28., 32., 32.),
            c(0xeeeeee, 0.73),
        );
        out.text(
            language.t("noTiles"),
            Rect::new(rect.x + 32., rect.y + 80., rect.w - 64., 34.),
            text(24., 550, -0.02).leading(1.4),
            WHITE,
            true,
        );
        out.text(
            language.t("addFirstShortcutHint"),
            Rect::new(rect.x + 80., rect.y + 130., rect.w - 160., 56.),
            text(18.4, 400, 0.).leading(1.5),
            c(0xf5f5f5, 0.73),
            true,
        );
    }
    row(
        out,
        state,
        assets,
        Row {
            target: Target::TileEditor,
            rect: Rect::new(LIST_X, LIST_TOP - state.scroll, LIST_WIDTH, ROW_HEIGHT),
            title: language.t("configure"),
            icon: Some("settings"),
            value: None,
            description: None,
            selected: false,
            chevron: false,
        },
    );
}

/// URLs have no break opportunities, so long ones are cut to a single line.
fn shorten(text: &str) -> String {
    const MAX: usize = 96;
    let count = text.chars().count();
    if count <= MAX {
        text.into()
    } else {
        format!("{}…", text.chars().take(MAX - 1).collect::<String>())
    }
}

/// A custom name such as "PS5" can choose the icon; otherwise the TV's label.
fn input_icon(input: &Input, custom: Option<&str>) -> &'static str {
    if input.id == "airplay" {
        return "airplay";
    }
    custom
        .map(device_icon)
        .filter(|icon| *icon != "hdmi")
        .unwrap_or_else(|| device_icon(&input.title))
}

fn device_icon(name: &str) -> &'static str {
    let name = name.to_lowercase().replace(' ', "");
    if name.contains("playstation") || ["ps1", "ps2", "ps3", "ps4", "ps5"].contains(&name.as_str())
    {
        "playstation"
    } else if name.contains("appletv") {
        "apple-tv"
    } else if name.contains("usb") {
        "usb"
    } else if name.contains("switch2") {
        "switch2"
    } else if name.contains("switch") {
        "switch"
    } else if name.contains("shield") {
        "shield"
    } else if name.contains("xbox") {
        "xbox"
    } else if name.contains("console") {
        "console"
    } else if ["pc", "computer", "laptop"]
        .iter()
        .any(|part| name.contains(part))
    {
        "computer"
    } else if ["stream", "fire", "roku"]
        .iter()
        .any(|part| name.contains(part))
    {
        "streamer"
    } else {
        "hdmi"
    }
}
/// Detail under an input title: the TV's own label (when connected and not just
/// the port), the port when a custom name replaces it, then the status.
pub fn input_status(input: &Input, renamed: bool, language: Language) -> String {
    if input.id == "airplay" {
        return language.t("airplayInputDetail").into();
    }
    let mut parts = vec![];
    if input.connected == Some(true) && input.title != input.port {
        parts.push(input.title.as_str());
    }
    if renamed {
        parts.push(input.port.as_str());
    }
    parts.push(match input.connected {
        Some(false) => language.t("noDevice"),
        None => language.t("externalInput"),
        Some(true) if input.signal == Some(false) => language.t("noSignal"),
        Some(true) => language.t("connected"),
    });
    parts.join(" · ")
}

struct EmptyState<'a> {
    rect: Rect,
    icon: &'a str,
    title: &'a str,
    hint: &'a str,
    action: Option<Target>,
}
fn empty(out: &mut Scene, state: &State, assets: &Assets, content: EmptyState<'_>) {
    let EmptyState {
        rect,
        icon,
        title,
        hint,
        action,
    } = content;
    out.shape(rect, 17.6, c(0xffffff, 0.02));
    out.border(rect, 17.6, 1., c(0xffffff, 0.05));
    let center = rect.x + rect.w / 2.;
    let top = rect.y
        + if action.is_some() {
            33.
        } else {
            (rect.h - 123.2) / 2.
        };
    out.icon(
        assets,
        icon,
        Rect::new(center - 17.6, top, 35.2, 35.2),
        c(0xd8d8d8, 0.60),
    );
    out.text(
        title,
        Rect::new(rect.x + 32., top + 52., rect.w - 64., 31.),
        text(24., 550, -0.035),
        WHITE,
        true,
    );
    out.text(
        hint,
        Rect::new(center - 352., top + 97.6, 704., 64.),
        text(16., 400, 0.).leading(1.6),
        c(0xf5f5f5, 0.72),
        true,
    );
    if let Some(target) = action {
        let label = state.preferences.language.t(if target == Target::Retry {
            "retry"
        } else {
            "exploreApps"
        });
        let width = assets.width(label, text(15.2, 600, 0.)) + 72.;
        let button = Rect::new(center - width / 2., top + 144.8, width, 46.8);
        out.shape(button, 11.2, c(0xededed, 1.));
        if *state.page_focus() == target {
            out.border(button.inset(-7.), 18.2, 2., WHITE);
        }
        out.text(
            label,
            Rect::new(button.x + 20.8, button.y + 14.4, button.w - 72., 18.4),
            text(15.2, 600, 0.),
            c(0x242424, 1.),
            false,
        );
        out.icon(
            assets,
            "arrow",
            Rect::new(button.x + button.w - 38.4, button.y + 14.6, 17.6, 17.6),
            c(0x242424, 1.),
        );
        out.control(target, button, true);
    }
}

fn options(out: &mut Scene, state: &State, assets: &Assets, id: &str, anchor: Option<Rect>) {
    let language = state.preferences.language;
    let shortcut = matches!(
        &state.overlay,
        Some(Overlay::Options {
            restore: Target::Tile(_),
            ..
        })
    );
    let background = matches!(state.overlay, Some(Overlay::BackgroundOptions { .. }));
    let app = state.inventory.apps.iter().find(|app| app.id == id);
    let favorite = state
        .preferences
        .favorites(&state.inventory.apps)
        .iter()
        .any(|item| item == id);
    let deletable = if background {
        state.can_delete_background(id) && state.deleting_wallpaper.is_none()
    } else {
        crate::platform::can_uninstall(id)
    };
    let skippable = background
        && id == crate::aerials::ID
        && state.preferences.background == crate::aerials::ID;
    let hidden = state.preferences.hidden.iter().any(|item| item == id);
    // Only Home's favorites row is reordered.
    let movable = state.page == Page::Home && favorite && !hidden;
    let actions = if shortcut {
        vec![
            (Target::TileTest(id.into()), language.t("testShortcut")),
            (Target::TileRemove(id.into()), language.t("removeTile")),
        ]
    } else if background {
        let mut actions = vec![(Target::UseBackground, language.t("useBackground"))];
        if deletable {
            actions.push((Target::DeleteBackground, language.t("deleteBackground")));
        }
        if skippable {
            actions.push((Target::NextAerial, language.t("nextAerial")));
        }
        actions
    } else {
        let mut actions = vec![(Target::OpenApp, language.t("open"))];
        if movable {
            actions.push((Target::MoveApp, language.t("move")));
        }
        actions.push((
            Target::Option(0),
            if favorite {
                language.t("removeFavorite")
            } else {
                language.t("addFavorite")
            },
        ));
        actions.push((
            Target::Option(1),
            language.t(if hidden { "showApp" } else { "hideApp" }),
        ));
        if deletable {
            actions.push((Target::Option(2), language.t("deleteApp")));
        }
        actions
    };
    let height = 22.4 + 76.8 * actions.len() as f32;
    let card = anchor.unwrap_or(Rect::new(460., 380., 256., 153.6));
    let x = if card.x + card.w + 32. + 520. <= 1836. {
        card.x + card.w + 32.
    } else {
        (card.x - 552.).max(84.)
    };
    let panel = Rect::new(x, card.y.clamp(164., 974. - height), 520., height);
    // Lift the selected card out of the dimmed grid without moving its hit region.
    if background {
        background_card(out, assets, state, id, card);
        out.border(card, 16., 1., c(0xffffff, 0.086));
    } else if let Some(app) = app.filter(|_| !shortcut) {
        let favorite_color = app_face(out, assets, app, card, language, 19.2);
        if favorite {
            favorite_badge(out, assets, card, favorite_color);
        }
    }
    let panel_start = out.draw.len();
    out.modal = Some(panel);
    out.shadow(panel, 27.2, 120., 25., 0.47);
    out.modal_glass(panel, 27.2);
    for (row, (target, label)) in actions.into_iter().enumerate() {
        let rect = Rect::new(
            panel.x + 16.,
            panel.y + 16. + row as f32 * 76.8,
            panel.w - 32.,
            67.2,
        );
        let focused = *state.page_focus() == target;
        let destructive = matches!(
            target,
            Target::Option(2) | Target::DeleteBackground | Target::TileRemove(_)
        );
        let label_style = text(20.8, 600, -0.02);
        let label_height = label_style.size * label_style.line_height;
        let label_color = if destructive || !focused {
            WHITE
        } else {
            c(0x242424, 1.)
        };
        out.shape(
            rect,
            12.,
            if destructive {
                c(0xd63b35, if focused { 1. } else { 0.25 })
            } else {
                c(0xffffff, if focused { 0.96 } else { 0.10 })
            },
        );
        out.text(
            label,
            Rect::new(
                rect.x + 24.,
                rect.y + (rect.h - label_height) / 2.,
                rect.w - 88.,
                label_height,
            ),
            label_style,
            label_color,
            false,
        );
        out.icon(
            assets,
            match &target {
                Target::OpenApp | Target::TileTest(_) => "arrow",
                Target::MoveApp => "move",
                Target::UseBackground => "check",
                Target::NextAerial => "right",
                Target::Option(0) => {
                    if favorite {
                        "heart-filled"
                    } else {
                        "heart"
                    }
                }
                Target::Option(1) => {
                    if hidden {
                        "eye"
                    } else {
                        "eye-off"
                    }
                }
                _ => "circle-minus",
            },
            Rect::new(rect.x + rect.w - 45.6, rect.y + 22.8, 21.6, 21.6),
            label_color,
        );
        out.control(target, rect, false);
    }
    if let Some(opened) = state.panel_started {
        out.fade(
            panel_start,
            (opened.elapsed().as_secs_f32() / 0.12).min(1.),
            1.,
        );
    }
}

fn delete_confirmation(out: &mut Scene, state: &State, assets: &Assets, id: &str) {
    let language = state.preferences.language;
    let background = matches!(state.overlay, Some(Overlay::BackgroundDelete { .. }));
    let deleting = if background {
        state.deleting_wallpaper.as_deref() == Some(id)
    } else {
        state.uninstalling.as_deref() == Some(id)
    };
    let warning_key = if background {
        "deleteBackgroundWarning"
    } else {
        "deleteAppWarning"
    };
    let deleting_key = if background {
        "deletingBackground"
    } else {
        "deletingApp"
    };
    let delete_key = if background {
        "deleteBackground"
    } else {
        "deleteApp"
    };
    let title = state
        .inventory
        .apps
        .iter()
        .find(|app| app.id == id)
        .map_or(id, |app| app.title.as_str());
    let body_style = text(20.8, 600, -0.02).leading(1.5);
    let body = language.format(
        if deleting { deleting_key } else { warning_key },
        &[("app", title)],
    );
    // Status changes must not move the confirmation buttons under the pointer.
    let warning = language.format(warning_key, &[("app", title)]);
    let body_height = assets
        .lines(&warning, body_style, 568.)
        .max(assets.lines(language.t(deleting_key), body_style, 568.))
        .max(1) as f32
        * 31.2;
    let height = 244. + body_height;
    let panel = Rect::new(640., (1080. - height) / 2., 640., height);
    out.modal = Some(panel);
    out.shadow(panel, 28., 100., 24., 0.47);
    out.modal_glass(panel, 28.);
    out.text(
        body,
        Rect::new(panel.x + 36., panel.y + 36., 568., body_height),
        body_style,
        c(0xf5f5f5, 0.75),
        true,
    );
    for (index, (target, label)) in [
        (Target::ConfirmDelete, delete_key),
        (Target::CancelDelete, "cancel"),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::new(
            panel.x + 28.,
            panel.y + panel.h - 184. + index as f32 * 80.,
            panel.w - 56.,
            68.,
        );
        let focused = *state.page_focus() == target;
        let disabled = target == Target::ConfirmDelete && deleting;
        let label_style = text(20.8, 600, -0.02);
        let label_height = label_style.size * label_style.line_height;
        out.shape(
            rect,
            14.,
            if index == 0 {
                c(
                    0xd63b35,
                    if disabled {
                        0.15
                    } else if focused {
                        1.
                    } else {
                        0.25
                    },
                )
            } else {
                c(0xffffff, if focused { 0.96 } else { 0.10 })
            },
        );
        out.text(
            language.t(label),
            Rect::new(
                rect.x + 20.,
                rect.y + (rect.h - label_height) / 2.,
                rect.w - 40.,
                label_height,
            ),
            label_style,
            if index == 1 && focused {
                c(0x242424, 1.)
            } else {
                WHITE
            },
            true,
        );
        if !disabled {
            out.control(target, rect, false);
        }
    }
}

fn keyboard(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(350., 290., 1220., 530.);
    out.modal = Some(panel);
    out.shadow(panel, 27.2, 90., 20., 0.45);
    out.modal_glass(panel, 27.2);
    out.text(
        match state.keyboard_field {
            TextField::Location => language.t("cityPostcode"),
            TextField::Name => language.t("greetingName"),
            TextField::InputName => language.t("inputName"),
            TextField::Apps => language.t("findApp"),
            TextField::Coin => language.t("cryptoFind"),
        },
        Rect::new(389., 324., 1000., 44.),
        text(37.6, 500, -0.055).leading(1.1),
        WHITE,
        false,
    );
    let close = Rect::new(1497., 328., 33.6, 33.6);
    out.icon(assets, "close", close.inset(7.6), WHITE);
    out.control(Target::CloseDialog, close, false);
    if *state.page_focus() == Target::CloseDialog {
        out.border(close, 16.8, 2., WHITE);
    }
    out.glass(Rect::new(389., 390., 1142., 56.), 28.);
    out.icon(
        assets,
        "search",
        Rect::new(409., 408., 20., 20.),
        c(0xeeeeee, 0.73),
    );
    out.text(
        format!("{}|", state.editing_text()),
        Rect::new(446., 406., 1058., 25.),
        text(20.8, 400, -0.02),
        WHITE,
        false,
    );
    for (index, key) in KEYS.iter().enumerate() {
        let label = match *key {
            "Space" => language.t("space"),
            "Delete" => language.t("delete"),
            "Clear" => language.t("clear"),
            "Done" => language.t("done"),
            _ => key,
        };
        let rect = Rect::new(
            389. + (index % 10) as f32 * 115.6,
            474. + (index / 10) as f32 * 74.,
            101.6,
            60.,
        );
        let focused = *state.page_focus() == Target::Key(index);
        out.pill(rect, focused, false);
        let mut style = text(18.4, 550, -0.02);
        let width = assets.width(label, style);
        if width > rect.w - 12. {
            style.size *= (rect.w - 12.) / width;
        }
        out.text(
            label,
            Rect::new(rect.x, rect.y + 18., rect.w, 25.),
            style,
            if focused { c(0x2c2c2c, 1.) } else { WHITE },
            true,
        );
        out.control(Target::Key(index), rect, false);
    }
}

fn toast(out: &mut Scene, state: &State, assets: &Assets, notice: &str) {
    let start = out.draw.len();
    let notice = notice.trim_end_matches(['.', '。']);
    let style = text(20.8, 550, -0.02);
    let width = (assets.width(notice, style).ceil() + 42.).clamp(42., 1856.);
    let text_height =
        assets.lines(notice, style, width - 40.) as f32 * style.size * style.line_height;
    let height = 20. + text_height;
    let rect = Rect::new(960. - width / 2., 1080. - 51.2 - height, width, height);
    out.shadow(rect, 16., 18., 4., 0.06);
    out.glass(rect, 16.);
    out.text(
        notice,
        Rect::new(rect.x + 20., rect.y + 10., width - 40., text_height),
        style,
        WHITE,
        true,
    );
    if *state.page_focus() == Target::DismissNotice {
        out.border(rect, 16., 2., WHITE);
    }
    if state.overlay.is_none() {
        out.control(Target::DismissNotice, rect, false);
    }
    if !state.preferences.reduced_motion
        && let Some((_, opened)) = &state.notice
    {
        out.fade(start, (opened.elapsed().as_secs_f32() / 0.12).min(1.), 1.);
    }
}

/// A corner-less, borderless fill of the whole logical screen.
fn full_screen_fill(draw: &Draw) -> bool {
    let whole = |r: &Rect| r.x <= 0. && r.y <= 0. && r.x + r.w >= 1920. && r.y + r.h >= 1080.;
    match draw {
        Draw::Shape(rect, radius, border, _) => whole(rect) && *radius <= 0. && *border <= 0.,
        Draw::RoundedGradient(rect, radius, ..) => whole(rect) && *radius <= 0.,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Preferences, demo_inventory};

    fn assets() -> Assets {
        Assets {
            root: PathBuf::new(),
            version: "test".into(),
            metrics: BTreeMap::from([
                ("display-500".into(), BTreeMap::from([('W', 1.)])),
                ("500".into(), BTreeMap::from([('W', 1.2)])),
                (
                    "fallback-500".into(),
                    BTreeMap::from([('界', 1.), ('한', 1.)]),
                ),
            ]),
            artwork: crate::app_artwork::Service::new(false),
        }
    }

    #[test]
    fn glass_slides_and_fades_with_the_page() {
        let screen = Rect::new(0., 0., 1920., 1080.);
        let mut scene = Scene::new();
        scene.draw.push(Draw::Glass(
            "wallpapers/dunes.jpg".into(),
            Rect::new(80., 400., 410., 96.),
            18.,
            screen,
            [1.; 4],
        ));
        scene.slide(0, 0, 100.);
        scene.fade(0, 0.5, 0.8);
        let Draw::Glass(_, rect, _, source, tint) = &scene.draw[0] else {
            panic!("still glass");
        };
        assert_eq!((rect.x, rect.y), (80., 300.));
        assert_eq!(
            (source.x, source.y, source.w, source.h),
            (0., 0., 1920., 1080.)
        );
        assert_eq!(*tint, [0.8, 0.8, 0.8, 0.5]);
    }

    #[test]
    fn settings_breadcrumbs_use_equal_gaps_and_only_highlight_the_leaf() {
        let assets = assets();
        let mut state = State::new(Preferences::default());
        state.open(2);
        state.section = 2;
        state.subpage = Some(Subpage::DateFormat);
        state.focus = Target::Choice(0);
        for language in Language::ALL {
            state.preferences.language = language;
            let mut scene = Scene::new();
            settings_title(&mut scene, &state, &assets, LIST_WIDTH);
            let parts: Vec<_> = scene
                .draw
                .iter()
                .filter_map(|draw| match draw {
                    Draw::Text(label, rect, style, color, _) => Some((label, rect, style, color)),
                    _ => None,
                })
                .collect();
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0].0, language.t("widgets"));
            assert_eq!(parts[1].0, language.t("date"));
            assert_eq!(parts[2].0, language.t("dateFormat"));
            assert_eq!(parts[0].3, parts[1].3);
            assert!(parts[2].3[0] > parts[1].3[0], "the leaf is brighter");
            let chevrons: Vec<_> = scene
                .draw
                .iter()
                .filter_map(|draw| match draw {
                    // The chevrons themselves, not their shadow.
                    Draw::TintedImage(_, rect, _, _, color) if color[0] > 0.5 => Some(rect),
                    _ => None,
                })
                .collect();
            assert_eq!(chevrons.len(), 2);
            for (index, chevron) in chevrons.iter().enumerate() {
                let (label, rect, style, _) = parts[index];
                let left_gap = chevron.x - rect.x - assets.width(label, *style);
                let right_gap = parts[index + 1].1.x - chevron.x - chevron.w;
                assert!((left_gap - 10.).abs() < 0.001);
                assert!((right_gap - 10.).abs() < 0.001);
            }
        }
    }

    #[test]
    fn gallery_switches_keep_their_width_when_they_flip_in_every_language() {
        let assets = assets();
        let mut state = State::new(Preferences::default());
        state.open(2);
        state.section = 1;
        for language in Language::ALL {
            state.preferences.language = language;
            let mut widths = Vec::new();
            for on in [false, true] {
                state.preferences.wallpaper_cycle = on;
                state.preferences.wallpaper_gradient = on;
                let mut scene = Scene::new();
                settings(&mut scene, &state, &assets);
                widths.push(
                    [Target::WallpaperCycle, Target::WallpaperGradient].map(|target| {
                        scene
                            .controls
                            .iter()
                            .find(|control| control.target == target)
                            .unwrap()
                            .rect
                    }),
                );
            }
            assert_eq!(
                widths[0].map(|rect| (rect.x, rect.w)),
                widths[1].map(|rect| (rect.x, rect.w)),
                "{}",
                language.code()
            );
        }
    }

    #[test]
    fn auto_rotation_keeps_a_compact_gap_and_right_aligned_state() {
        let assets = assets();
        let mut state = State::new(Preferences::default());
        state.open(2);
        state.section = 1;
        let label = state.preferences.language.t("wallpaperCycle");
        let label_width = assets.width(label, text(type_scale::SECONDARY, 550, -0.01));
        for enabled in [false, true] {
            state.preferences.wallpaper_cycle = enabled;
            let mut scene = Scene::new();
            settings(&mut scene, &state, &assets);
            let button = scene
                .controls
                .iter()
                .find(|control| control.target == Target::WallpaperCycle)
                .unwrap()
                .rect;
            let title = scene
                .draw
                .iter()
                .find_map(|draw| match draw {
                    Draw::Text(value, rect, ..) if value == label => Some(rect),
                    _ => None,
                })
                .unwrap();
            let state_label =
                state
                    .preferences
                    .language
                    .t(if enabled { "stateOn" } else { "stateOff" });
            let (value, style) = scene
                .draw
                .iter()
                .find_map(|draw| match draw {
                    Draw::Text(label, rect, style, ..) if label == state_label => {
                        Some((rect, style))
                    }
                    _ => None,
                })
                .unwrap();
            assert!(style.align_right);
            assert!((button.x + button.w - value.x - value.w - 24.).abs() < 0.001);
            assert!(value.x - title.x - label_width >= 12.);
            assert!(button.w < 400.);
        }
    }

    #[test]
    fn short_greetings_keep_their_original_style_and_geometry() {
        let assets = assets();
        for name in ["", "Ana"] {
            let state = State::new(Preferences {
                greeting_name: name.into(),
                show_weather: false,
                ..Preferences::default()
            });
            let mut scene = Scene::new();
            home(&mut scene, &state, &assets, OffsetDateTime::UNIX_EPOCH);
            let greeting = state.preferences.language.greeting(0, name);
            let (rect, style) = scene
                .draw
                .iter()
                .find_map(|draw| match draw {
                    Draw::Text(value, rect, style, ..) if value == &greeting => Some((rect, style)),
                    _ => None,
                })
                .unwrap();
            // Just under the clock's line, which holds the date.
            assert_eq!((rect.x, rect.w, rect.h), (80., 1760., 80.));
            assert!((rect.y - GREETING_TOP).abs() < 0.001, "{rect:?}");
            assert_eq!(style.size, 76.);
            assert_eq!(style.weight, 600);
            assert_eq!(style.tracking, -0.04);
        }
    }

    #[test]
    fn long_localized_greetings_fit_on_one_line_without_changing_the_name() {
        let assets = assets();
        let base = text(90.4, 500, -0.065).leading(1.05);
        for language in Language::ALL {
            for hour in [2, 9, 15, 20] {
                for name in [
                    "W".repeat(40),
                    "界".repeat(40),
                    "한".repeat(40),
                    "Alexandra Catherine Elizabeth Montgomery".into(),
                ] {
                    let greeting = language.greeting(hour, &name);
                    let fitted = assets.fit_text(&greeting, base, 1500.);
                    assert!(fitted.size > 0. && fitted.size < base.size);
                    assert!(assets.width(&greeting, fitted) < 1500.);
                    assert_eq!(assets.lines(&greeting, fitted, 1500.), 1);
                    assert!(greeting.contains(&name));
                }
            }
        }
    }

    #[test]
    fn fitting_remeasures_when_shrinking_changes_the_optical_font() {
        let assets = assets();
        let greeting = "W".repeat(100);
        let fitted = assets.fit_text(&greeting, text(90.4, 500, -0.065), 1500.);
        assert!(fitted.size < 32.);
        assert!(assets.width(&greeting, fitted) < 1500.);
        assert_eq!(assets.lines(&greeting, fitted, 1500.), 1);
    }

    #[test]
    fn shrinking_a_greeting_preserves_weather_favorites_and_control_positions() {
        let assets = assets();
        for show_weather in [false, true] {
            let mut state = State::new(Preferences {
                greeting_name: "Ana".into(),
                show_weather,
                ..Preferences::default()
            });
            state.set_inventory(demo_inventory(), None);
            let mut original = Scene::new();
            home(&mut original, &state, &assets, OffsetDateTime::UNIX_EPOCH);
            state.preferences.greeting_name = "W".repeat(40);
            let mut fitted = Scene::new();
            home(&mut fitted, &state, &assets, OffsetDateTime::UNIX_EPOCH);
            for (before, after) in original.draw.iter().zip(&fitted.draw) {
                if let (
                    Draw::Text(old, rect, old_style, ..),
                    Draw::Text(new, fitted_rect, style, ..),
                ) = (before, after)
                {
                    if old == new {
                        assert_eq!(
                            (rect.x, rect.y, rect.w, rect.h),
                            (fitted_rect.x, fitted_rect.y, fitted_rect.w, fitted_rect.h)
                        );
                        assert_eq!(old_style.size, style.size);
                    } else {
                        assert_eq!(
                            new,
                            &state
                                .preferences
                                .language
                                .greeting(0, &state.preferences.greeting_name)
                        );
                        assert!(
                            (rect.y + rect.h / 2. - fitted_rect.y - fitted_rect.h / 2.).abs()
                                < 0.001
                        );
                        assert!(style.size * style.line_height <= fitted_rect.h);
                    }
                }
            }
            assert_eq!(original.controls.len(), fitted.controls.len());
            for (before, after) in original.controls.iter().zip(&fitted.controls) {
                assert_eq!(before.target, after.target);
                assert_eq!(
                    (before.rect.x, before.rect.y, before.rect.w, before.rect.h),
                    (after.rect.x, after.rect.y, after.rect.w, after.rect.h)
                );
            }
        }
    }

    #[test]
    fn hidden_home_widgets_collapse_upward_without_moving_favorites() {
        let assets = assets();
        let mut state = State::new(Preferences::default());
        state.set_inventory(demo_inventory(), None);
        let mut original = Scene::new();
        home(&mut original, &state, &assets, OffsetDateTime::UNIX_EPOCH);
        let weather_y = |scene: &Scene| {
            scene
                .controls
                .iter()
                .find(|control| control.target == Target::WeatherConfigure)
                .unwrap()
                .rect
                .y
        };
        let original_y = weather_y(&original);
        for show_date in [false, true] {
            for show_greeting in [false, true] {
                state.preferences.show_date = show_date;
                state.preferences.show_greeting = show_greeting;
                let mut rendered = Scene::new();
                home(&mut rendered, &state, &assets, OffsetDateTime::UNIX_EPOCH);
                // The date is on the clock's line; without the greeting the weather rises.
                let removed = if show_greeting { 0. } else { 80. + 16. };
                assert!((weather_y(&rendered) - (original_y - removed)).abs() < 0.001);
                for control in original
                    .controls
                    .iter()
                    .filter(|control| matches!(control.target, Target::App(_) | Target::Input(_)))
                {
                    let other = rendered
                        .controls
                        .iter()
                        .find(|other| other.target == control.target)
                        .unwrap();
                    assert_eq!(
                        (control.rect.x, control.rect.y),
                        (other.rect.x, other.rect.y)
                    );
                }
                if !show_greeting {
                    // Weather's control includes padding around the summary itself.
                    assert!((weather_y(&rendered) - GREETING_TOP).abs() < 60.);
                }
            }
        }
    }
}
