use crate::i18n::Language;
use crate::model::{
    App, BACKGROUNDS, CATEGORIES, CUSTOM_TILE_PREFIX, CustomTile, INPUT_NAME_LIMIT, InputMode,
    Inventory, Preferences, UiSize, clean_input_name,
};
use crate::text_rendering::TextRendering;
use crate::weather::{self, Unit};
use serde::Serialize;
use std::time::{Duration, Instant};

pub const BACKGROUND_SECTION: usize = 1;
pub const INTERFACE_SCALE_ROW: usize = 8;
pub const WIDGETS_SECTION: usize = 2;
pub const SYSTEM_SECTION: usize = 3;
/// System › Luma as Home screen.
pub const HOME_TAKEOVER_ROW: usize = 9;
pub const SECTION_COUNT: usize = 4;
/// System row turning automatic Homebrew app updates on or off (homebrew_updates.rs).
pub const HOMEBREW_UPDATES_ROW: usize = 13;
/// How far the tab bar and Apps' filters (down to y 210) slide up to be gone.
pub const CHROME_TRAVEL: f32 = 220.;
/// The tvOS safe area's top and bottom margin.
const SAFE_TOP: f32 = 60.;
/// Settings › General's focus glow colour row.
pub const FOCUS_GLOW_ROW: usize = 14;
pub const SETTINGS_BOTTOM_PADDING: f32 = 64.;
pub const SETTINGS_LIST_BOTTOM_PADDING: f32 = 48.;
pub const APPS_COLUMNS: usize = 6;
/// Automatic inventory retries before the error shows: with the 6.5 s Luna timeout about 50 s.
pub const INVENTORY_RETRIES: u32 = 5;
pub const INVENTORY_RETRY_DELAY: Duration = Duration::from_secs(2);
pub const HOME_STRIP_LEFT: f32 = crate::layout::SAFE_X;
// Six favourites in full and the seventh cut off and fading out at the right (Apple TV rows):
// the strip ends after the sixth tile, so a focused tile never sits in the fade.
pub const HOME_STRIP_RIGHT: f32 = crate::layout::SAFE_X + crate::layout::WIDTH;
pub const HOME_TILE_WIDTH: f32 = 260.;
pub const HOME_TILE_PITCH: f32 = HOME_TILE_WIDTH + crate::layout::GUTTER;
pub const HOME_FADE_LEFT: f32 = 8.;
pub const HOME_FADE_RIGHT: f32 = 1910.;
pub const BACKGROUND_FADE: f32 = 0.4;
pub const CYCLE_FADE: f32 = 1.6;

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Page {
    Home,
    Apps,
    Settings,
}

/// The menu slides in over this many seconds, and out over the second.
const MENU_OPEN: f32 = 0.25;
const MENU_CLOSE: f32 = 0.2;
/// Spatial focus prefers a control in line; one only in another row scores this much more.
const ASIDE: f32 = 10000.;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Target {
    /// A page in the menu: Home, Apps, Settings.
    Nav(usize),
    /// The menu's Inputs: folds the list of inputs open and closed.
    MenuInputs,
    TvSettings,
    InstallUpdate,
    CheckUpdates,
    Recent(String),
    RecentEmpty,
    RecentRetry,
    RecentCloseAll,
    App(String),
    Input(String),
    /// A crypto price card on Home, by position.
    Coin(usize),
    Browse,
    Filter(usize),
    ClearFilter,
    Search,
    ClearSearch,
    Sidebar(usize),
    Setting(usize),
    Widget(usize),
    GreetingName,
    GreetingNameClear,
    GreetingToggle,
    DateToggle,
    DateFormat,
    AppNamesHome,
    AppNamesApps,
    ClockFormat,
    ClockTimeFormat,
    Background(usize),
    UploadWallpaper,
    WallpaperCycle,
    WallpaperGradient,
    UseBackground,
    DeleteBackground,
    NextAerial,
    InputMode(String),
    InputRename,
    InputNameReset,
    Choice(usize),
    CloseDialog,
    Retry,
    DismissNotice,
    Option(usize),
    OpenApp,
    /// Options › Move: reorder a Home favorite with Left and Right.
    MoveApp,
    ConfirmDelete,
    CancelDelete,
    Key(usize),
    WeatherConfigure,
    WeatherToggle,
    WeatherLocation,
    WeatherTemperature,
    WeatherQuery,
    WeatherSearch,
    /// Widgets › CS2 Scores: show it, Tier 1–2 only, recent results.
    Cs2Toggle,
    Cs2Major,
    Cs2Results,
    /// A match card in Home's CS2 Scores row, by position.
    Cs2Match(usize),
    /// The close button of a CS2 Scores match's dialog.
    Cs2Close,
    /// Widgets › TV Shows: show the row.
    ShowsToggle,
    /// A show card in Home's TV Shows row, by position.
    Show(usize),
    /// Home's Today card, whole: OK opens it in the center.
    TodayCard,
    /// A tab of the opened Today card: a widget whose lines it shows.
    TodayTab(crate::today::Tab),
    /// A match line of the opened Today card, by the match's id.
    TodayMatch(String),
    /// A TV Shows line of the opened Today card, by the series' TMDB id: opens Lampa.
    TodayShow(i64),
    /// A Lampa line of the opened Today card, by its CUB card ("tv_2190"): opens Lampa.
    TodayLampa(String),
    /// Widgets › Lampa: sign in to CUB, or out.
    LampaAccount,
    /// Widgets › Lampa: its lines in the Today card.
    LampaFeed,
    LampaPlugin,
    WeatherClear,
    WeatherResult(usize),
    CryptoToggle,
    /// Widgets › Crypto: a row below the inputs or a line in Today.
    CryptoLayout,
    /// Widgets › CS2 Scores: matches in Today or in their own row.
    Cs2Layout,
    /// Widgets › TV Shows: episodes in Today or a row of posters.
    ShowsLayout,
    /// Widgets › Crypto: the coin search's field.
    CryptoSearch,
    /// Widgets › Crypto: one of the search's matches.
    CryptoResult(usize),
    /// Widgets › Crypto: one of `State::crypto_choices`.
    CryptoCoin(usize),
    Tile(String),
    TileTest(String),
    TileRemove(String),
    TileEditor,
}

impl Target {
    /// Home's Today card.
    pub fn in_today(&self) -> bool {
        *self == Target::TodayCard
    }

    /// A line of the opened Today card that opens something of its own.
    pub fn today_line(&self) -> bool {
        matches!(
            self,
            Target::TodayMatch(_) | Target::TodayShow(_) | Target::TodayLampa(_)
        )
    }

    /// A card of the widget in the center of Home.
    pub fn in_center(&self) -> bool {
        matches!(
            self,
            Target::Coin(_) | Target::Cs2Match(_) | Target::Show(_)
        )
    }
}

/// The card among `of` nearest across from `from`: where Up or Down between rows lands.
fn across(controls: &[Control], from: &Target, of: fn(&Target) -> bool) -> Option<Target> {
    let x = controls
        .iter()
        .find(|control| control.target == *from)
        .map_or(960., |control| control.rect.center().0);
    controls
        .iter()
        .filter(|control| of(&control.target))
        .min_by(|a, b| {
            (a.rect.center().0 - x)
                .abs()
                .total_cmp(&(b.rect.center().0 - x).abs())
        })
        .map(|control| control.target.clone())
}

/// The Today card's last line, the one nearest the rows below it.
fn today_last(controls: &[Control]) -> Option<Target> {
    controls
        .iter()
        .filter(|control| control.target.in_today())
        .max_by(|a, b| a.rect.y.total_cmp(&b.rect.y))
        .map(|control| control.target.clone())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    pub fn center(self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(self.x + n, self.y + n, self.w - 2. * n, self.h - 2. * n)
    }
}

#[derive(Clone, Debug)]
pub struct Control {
    pub target: Target,
    pub rect: Rect,
    pub scrollable: bool,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    Options {
        id: String,
        restore: Target,
    },
    Delete {
        id: String,
        restore: Target,
    },
    Recents {
        restore: Target,
    },
    Keyboard,
    SystemKeyboard,
    WallpaperUpload,
    TileEditor,
    /// Signing in to CUB: its code and QR code while the user confirms on their phone.
    CubSignIn,
    /// Why Lampa needs Luma's plugin, and how to add it.
    LampaPlugin,
    BackgroundOptions {
        id: String,
        restore: Target,
    },
    BackgroundDelete {
        id: String,
        restore: Target,
    },
    /// A CS2 Scores match in detail, by its id, so it stays open across refreshes.
    Cs2Match {
        id: String,
        restore: Target,
    },
    /// The menu on the left: the pages, the inputs and the TV's settings.
    Menu {
        restore: Target,
    },
    /// The Today card opened in the center: the whole day by section.
    Today {
        restore: Target,
    },
}

/// The opened Today card (spec «Сегодня» раскрытая), and its list under the heading.
pub const TODAY_PANEL: Rect = Rect::new(410., 110., 1100., 860.);
pub const TODAY_LIST: Rect = Rect::new(410., 260., 1100., 634.);
const TODAY_OPEN: f32 = 0.25;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Subpage {
    AppNames,
    Greeting,
    Date,
    DateFormat,
    Weather,
    Shortcuts,
    Inputs,
    /// Widgets › Crypto: show the prices and choose their coins.
    Crypto,
    Clock,
    ClockFormat,
    TimeFormat,
    /// Widgets › CS2 Scores.
    Cs2,
    /// Widgets › Lampa: what it lists, and the CUB account its history comes from.
    Shows,
    Input(String),
    WeatherLocation,
    Temperature,
    Language,
    /// General › Interface scale: Normal, Large or Extra large, with a live preview.
    Appearance,
    TextRendering,
    /// System › Luma as Home: on (replace LG Home) or off.
    HomeTakeover,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum TextField {
    Apps,
    Location,
    Name,
    InputName,
    Coin,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Save,
    /// Read the app inventory again, keeping decoded icons of unchanged apps.
    Refresh,
    /// Refresh apps: read the inventory and every app icon again.
    Reload,
    Launch(String),
    /// Opens Lampa (`app`) on a card ("tv_2190").
    OpenCard {
        app: String,
        card: String,
    },
    Uninstall(String),
    DeleteWallpaper(String),
    Recent {
        generation: u64,
        command: crate::recents::Command,
    },
    /// Fire-and-forget HTTP request for a custom tile, off the SDL thread.
    FireWebhook(CustomTile),
    Quit,
    Weather(weather::Command),
    UpdateDownload(String),
    /// Replace LG Home with Luma (true) or restore it (false), through Homebrew's root service.
    HomeTakeover(bool),
}

pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];

pub struct State {
    /// The focused card's lift and the scrolling of Home and its rows (`motion.rs`).
    pub motion: crate::motion::Motion,
    pub page: Page,
    pub focus: Target,
    pub inventory: Inventory,
    pub preferences: Preferences,
    pub query: String,
    pub filter: usize,
    pub section: usize,
    pub subpage: Option<Subpage>,
    pub scroll: f32,
    pub home_scroll: f32,
    pub overlay: Option<Overlay>,
    pub moving: Option<String>,
    pub loading: bool,
    /// Set by Refresh apps: the next inventory also reloads every app icon.
    pub reload_artwork: bool,
    pub launching: bool,
    pub uninstalling: Option<String>,
    pub notice: Option<(String, Instant)>,
    pub error: Option<String>,
    pub available_update: Option<String>,
    pub update_status: crate::update::Status,
    pub update_check_requested: bool,
    /// Inventory loads that failed in a row while no apps were shown yet.
    pub inventory_failures: u32,
    pub dirty: bool,
    pub pointer: bool,
    pub pressed: Option<Target>,
    pub previous_background: Option<(String, Instant)>,
    /// The renderer holds each wallpaper fade until it is ready for it (`wallpaper_waiting`).
    pub hold_fades: bool,
    /// A fade held on the old wallpaper, its clock stopped, until the renderer releases it.
    pub wallpaper_waiting: bool,
    /// The frame a dynamic wallpaper shows, for the wallpaper it was chosen for, and when.
    pub dynamic_frame: Option<(String, usize, i64)>,
    pub background_fade: f32,
    pub wallpapers: Vec<crate::wallpapers::Wallpaper>,
    /// Apple TV stills on the TV (aerials.rs).
    pub aerials: Vec<crate::wallpapers::Wallpaper>,
    /// Set when the Apple TV worker should be told which stills to keep (`aerial_ids`).
    pub aerials_request: bool,
    /// The place shown before the last change, kept while it fades out.
    aerials_previous: Option<String>,
    pub deleting_wallpaper: Option<String>,
    pub wallpaper_upload: Option<Result<crate::wallpapers::Session, String>>,
    pub tile_editor: Option<Result<crate::companion::Session, String>>,
    /// Luma update (Settings → System), driven by the update worker.
    pub update: crate::update::InstallStatus,
    pub panel_started: Option<Instant>,
    /// Whether the menu lists the inputs under its Inputs item.
    pub menu_inputs: bool,
    /// The menu's slide: when it began, and whether it opens.
    pub menu_motion: Option<(Instant, bool)>,
    /// How far the opened Today card's list has scrolled.
    pub today_scroll: f32,
    /// The opened Today card's tab; None for its first.
    pub today_tab: Option<crate::today::Tab>,
    /// When the Today card began to open.
    pub today_motion: Option<Instant>,
    pub weather: weather::Weather,
    pub crypto: crate::crypto::Crypto,
    pub cs2: crate::cs2::Scores,
    /// The first of the CS2 Scores cards in view; Left and Right scroll through the rest.
    pub cs2_first: usize,
    pub shows: crate::shows::Shows,
    /// Lampa's CUB account, for the Today card.
    pub cub: crate::cub::Cub,
    /// The user signed out of CUB: its kept sign-in is to be removed.
    pub cub_forget: bool,
    /// The first of the TV Shows cards in view; Left and Right scroll through the rest.
    pub shows_first: usize,
    pub keyboard_field: TextField,
    pub greeting_name_draft: String,
    pub input_name: String,
    pub recents: crate::recents::Recents,
    /// Whether this instance runs as the TV's Home (the takeover is enabled).
    pub home_takeover: bool,
}

impl State {
    pub fn new(preferences: Preferences) -> Self {
        Self {
            motion: crate::motion::Motion::default(),
            page: Page::Home,
            focus: Target::Nav(0),
            inventory: Inventory::default(),
            preferences,
            query: String::new(),
            filter: 0,
            section: 0,
            subpage: None,
            scroll: 0.,
            home_scroll: 0.,
            overlay: None,
            moving: None,
            loading: false,
            reload_artwork: false,
            launching: false,
            uninstalling: None,
            notice: None,
            error: None,
            available_update: None,
            update_status: crate::update::Status::Unchecked,
            update_check_requested: false,
            inventory_failures: 0,
            dirty: true,
            pointer: false,
            pressed: None,
            previous_background: None,
            hold_fades: false,
            wallpaper_waiting: false,
            dynamic_frame: None,
            background_fade: BACKGROUND_FADE,
            wallpapers: vec![],
            aerials: vec![],
            aerials_request: false,
            aerials_previous: None,
            deleting_wallpaper: None,
            wallpaper_upload: None,
            tile_editor: None,
            update: crate::update::InstallStatus::default(),
            panel_started: None,
            menu_inputs: false,
            menu_motion: None,
            today_scroll: 0.,
            today_tab: None,
            today_motion: None,
            weather: weather::Weather::default(),
            crypto: crate::crypto::Crypto::default(),
            cs2: crate::cs2::Scores::default(),
            cs2_first: 0,
            shows: crate::shows::Shows::default(),
            cub: crate::cub::Cub::default(),
            cub_forget: false,
            shows_first: 0,
            keyboard_field: TextField::Apps,
            greeting_name_draft: String::new(),
            input_name: String::new(),
            recents: crate::recents::Recents::default(),
            home_takeover: false,
        }
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty = true;
    }

    pub fn update_response(&mut self, result: Result<Option<String>, String>) {
        match result {
            Ok(latest) => {
                let announce = latest.is_some() && latest != self.available_update;
                self.available_update = latest;
                self.update_status = crate::update::Status::Checked;
                if announce {
                    self.notify(self.preferences.language.t("updateAvailable"));
                }
            }
            Err(error) => {
                eprintln!("Luma update check: {error}");
                self.update_status = crate::update::Status::Failed;
            }
        }
        if matches!(self.focus, Target::CheckUpdates | Target::InstallUpdate) {
            let target = if self.available_update.is_some() {
                Target::InstallUpdate
            } else {
                Target::CheckUpdates
            };
            if self.focus != target {
                self.focus = target;
                self.pressed = None;
            }
        }
        self.dirty = true;
    }

    pub fn update_busy(&self) -> bool {
        self.update_status == crate::update::Status::Checking
            || matches!(
                self.update,
                crate::update::InstallStatus::Downloading(_)
                    | crate::update::InstallStatus::Installing(_)
            )
    }

    pub fn install_response(&mut self, response: crate::update::Response) {
        use crate::update::{InstallStatus, Response};
        match response {
            Response::Progress(percent) => self.update = InstallStatus::Downloading(percent),
            Response::Installing(version) => {
                self.update = InstallStatus::Installing(version);
                self.notify(self.preferences.language.t("updateRestart"));
            }
            Response::Finished(result) => {
                self.available_update = None;
                match result {
                    Ok(()) => {
                        self.update = InstallStatus::Idle;
                        // This process may still run the old executable. Recheck rather
                        // than claim that its in-memory version was replaced.
                        self.update_status = crate::update::Status::Unchecked;
                    }
                    Err(error) => {
                        eprintln!("Luma update: {error}");
                        self.update = InstallStatus::Failed(error);
                        self.update_status = crate::update::Status::Failed;
                        self.notify(self.preferences.language.t("updateError"));
                    }
                }
                if self.focus == Target::InstallUpdate {
                    self.focus = Target::CheckUpdates;
                    self.pressed = None;
                }
            }
        }
        self.dirty = true;
    }

    pub fn tick(&mut self) {
        let recents_open = self.recents_open();
        if recents_open || self.recents.visibility > 0. {
            self.dirty |= self.recents.animate(
                recents_open,
                self.preferences.reduced_motion,
                self.pointer,
                Instant::now(),
            );
        }
        if let Some((start, opening)) = self.menu_motion {
            self.dirty = true;
            let duration = if opening { MENU_OPEN } else { MENU_CLOSE };
            if self.preferences.reduced_motion || start.elapsed().as_secs_f32() >= duration {
                self.menu_motion = None;
            }
        }
        if let Some(start) = self.today_motion {
            self.dirty = true;
            if self.preferences.reduced_motion || start.elapsed().as_secs_f32() >= TODAY_OPEN {
                self.today_motion = None;
            }
        }
        if let Some(start) = self.panel_started {
            self.dirty = true;
            let duration = if self.overlay.is_some() { 120 } else { 150 };
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(duration)
            {
                self.panel_started = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120))
            && !self.preferences.reduced_motion
        {
            self.dirty = true;
        }
        if let Some((_, start)) = &mut self.previous_background {
            self.dirty = true;
            if self.wallpaper_waiting {
                *start = Instant::now();
            } else if self.preferences.reduced_motion
                || start.elapsed().as_secs_f32() >= self.background_fade
            {
                self.previous_background = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() > Duration::from_secs(5))
        {
            self.notice = None;
            self.dirty = true;
        }
    }

    pub fn page_index(&self) -> usize {
        match self.page {
            Page::Home => 0,
            Page::Apps => 1,
            Page::Settings => 2,
        }
    }

    /// Advances the shared motion: Home's focused card lifts, and Home and its rows scroll to
    /// where navigation put them. Hit targets and navigation use the targets at once.
    pub fn animate(&mut self, dt: f32) {
        use crate::motion::Lane;
        let home = self.page == Page::Home;
        let apps = self.page == Page::Apps;
        let selected = (self.overlay.is_none()
            || matches!(
                self.overlay,
                Some(Overlay::Menu { .. } | Overlay::Today { .. })
            ))
        .then_some(self.page_focus())
        .filter(|target| match self.page {
            Page::Home | Page::Apps => matches!(
                target,
                Target::App(_)
                    | Target::Input(_)
                    | Target::Coin(_)
                    | Target::Cs2Match(_)
                    | Target::Show(_)
                    | Target::TodayCard
            ),
            // A settings row grows to show its description.
            Page::Settings => matches!(
                target,
                Target::Setting(_) | Target::Widget(_) | Target::Choice(_)
            ),
        })
        .cloned();
        let lanes: Vec<(Lane, f32)> = if home {
            let mut lanes = vec![
                (Lane::Page, self.scroll),
                (Lane::Favorites, self.home_scroll),
                (Lane::Cs2, self.cs2_first as f32),
                (Lane::Shows, self.shows_first as f32),
                (Lane::Chrome, self.chrome_target()),
            ];
            if matches!(self.overlay, Some(Overlay::Today { .. })) {
                lanes.push((Lane::Today, self.today_scroll));
            }
            lanes
        } else if apps {
            vec![
                (Lane::Page, self.scroll),
                (Lane::Chrome, self.chrome_target()),
            ]
        } else {
            vec![(Lane::Sidebar, self.sidebar_target())]
        };
        let reduced = self.preferences.reduced_motion;
        self.dirty |= self.motion.advance(selected.as_ref(), &lanes, dt, reduced);
    }

    /// Where the page is drawn: Home eases toward `scroll`, other pages are drawn at it.
    pub fn page_scroll(&self) -> f32 {
        if matches!(self.page, Page::Home | Page::Apps) {
            self.motion.shown(crate::motion::Lane::Page, self.scroll)
        } else {
            self.scroll
        }
    }

    pub fn animating(&self) -> bool {
        self.animating_besides_wallpaper() || self.previous_background.is_some()
    }

    /// Whether anything but a wallpaper fade moves: a fade waits for it, as it changes both of
    /// the fade's ends.
    pub fn animating_besides_wallpaper(&self) -> bool {
        ((self.recents_open() || self.recents.visibility > 0.)
            && (self.recents.visibility != if self.recents_open() { 1. } else { 0. }
                || self.recents.cards_animating(self.pointer)))
            || self.motion.animating()
            || self.panel_started.is_some()
            || self.menu_motion.is_some()
            || self.today_motion.is_some()
            || (!self.preferences.reduced_motion
                && self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120)))
    }

    /// Home's lifecycle relaunch returns directly to Home, even from an overlay.
    pub fn return_home(&mut self) -> Vec<Effect> {
        // Commit keyboard drafts before open() clears their settings subpage.
        let pending = self.finish_text_input();
        self.recents.pending = None;
        self.recents.visibility = 0.;
        self.recents.closing_focus = None;
        self.wallpaper_upload = None;
        self.pointer = false;
        self.pressed = None;
        self.open(0);
        self.home_scroll = 0.;
        self.focus = self.first_content();
        pending
    }

    /// Home with Luma in front: the menu opens over the page, and Home again closes it.
    pub fn home_key(&mut self) -> Vec<Effect> {
        if matches!(self.overlay, Some(Overlay::Menu { .. })) {
            self.close_menu();
            return vec![];
        }
        let pending = self.clear_for_menu();
        self.open_menu();
        pending
    }

    /// Home from another app: Luma comes back on Home, with the menu open.
    pub fn relaunch_home(&mut self) -> Vec<Effect> {
        let pending = self.return_home();
        self.open_menu();
        pending
    }

    /// Before the menu opens over the page: drafts are kept, and a dialog or Recents closes back
    /// to where it was opened from.
    fn clear_for_menu(&mut self) -> Vec<Effect> {
        let pending = self.finish_text_input();
        self.recents.pending = None;
        self.recents.visibility = 0.;
        self.recents.closing_focus = None;
        self.wallpaper_upload = None;
        self.pointer = false;
        self.pressed = None;
        self.moving = None;
        self.panel_started = None;
        if let Some(overlay) = self.overlay.take() {
            self.focus = match overlay {
                Overlay::Options { restore, .. }
                | Overlay::Delete { restore, .. }
                | Overlay::Recents { restore }
                | Overlay::BackgroundOptions { restore, .. }
                | Overlay::BackgroundDelete { restore, .. }
                | Overlay::Menu { restore }
                | Overlay::Today { restore } => restore,
                // From the opened Today card: back to the card, closed.
                Overlay::Cs2Match { restore, .. } if restore.today_line() => Target::TodayCard,
                Overlay::Cs2Match { restore, .. } => restore,
                Overlay::Keyboard | Overlay::SystemKeyboard => self.keyboard_return(),
                Overlay::WallpaperUpload => Target::UploadWallpaper,
                Overlay::TileEditor => {
                    self.tile_editor = None;
                    Target::TileEditor
                }
                Overlay::LampaPlugin => Target::LampaPlugin,
                Overlay::CubSignIn => {
                    self.cub.cancel();
                    Target::LampaAccount
                }
            };
        }
        pending
    }

    /// The menu over the page, on the page's own item; closing it returns to the focus it had.
    pub fn open_menu(&mut self) {
        let amount = self.menu_amount();
        let restore = if matches!(self.focus, Target::Nav(_) | Target::MenuInputs) {
            self.first_content()
        } else {
            self.focus.clone()
        };
        self.overlay = Some(Overlay::Menu { restore });
        self.focus = Target::Nav(self.page_index());
        self.pressed = None;
        // Reversing midway starts from where the panel is.
        self.menu_motion = (!self.preferences.reduced_motion).then(|| {
            let done = 1. - (1. - amount).cbrt();
            (
                Instant::now() - Duration::from_secs_f32(done * MENU_OPEN),
                true,
            )
        });
        self.dirty = true;
    }

    pub fn close_menu(&mut self) {
        if !matches!(self.overlay, Some(Overlay::Menu { .. })) {
            return;
        }
        let amount = self.menu_amount();
        if let Some(Overlay::Menu { restore }) = self.overlay.take() {
            self.focus = restore;
        }
        self.pressed = None;
        self.menu_motion = (!self.preferences.reduced_motion).then(|| {
            let done = 1. - amount.cbrt();
            (
                Instant::now() - Duration::from_secs_f32(done * MENU_CLOSE),
                false,
            )
        });
        self.dirty = true;
    }

    /// OK on the Today card: it opens in the center. Its first line takes the focus once drawn
    /// (`reveal_focus`).
    fn open_today(&mut self) {
        self.overlay = Some(Overlay::Today {
            restore: Target::TodayCard,
        });
        self.today_scroll = 0.;
        self.today_tab = None;
        self.today_motion = (!self.preferences.reduced_motion).then(Instant::now);
        self.pressed = None;
    }

    /// The opened Today card shows `tab`'s lines from the top; its first takes the focus once
    /// drawn (`reveal_focus`).
    fn show_today_tab(&mut self, tab: crate::today::Tab) {
        self.today_tab = Some(tab);
        self.today_scroll = 0.;
        self.focus = Target::TodayTab(tab);
        self.pressed = None;
        self.dirty = true;
    }

    /// OK on a line of the opened Today card: a match opens over it, an app it plays closes it,
    /// and a line with nothing to open (no app for it) leaves it open with a notice.
    fn today_activate(&mut self) -> Vec<Effect> {
        if let Target::TodayTab(tab) = self.focus {
            self.show_today_tab(tab);
            return vec![];
        }
        let Some(Overlay::Today { restore }) = self.overlay.take() else {
            return vec![];
        };
        let line = self.focus.clone();
        let effects = self.activate();
        if self.overlay.is_none() && self.page == Page::Home {
            if effects.is_empty() && self.focus == line {
                self.overlay = Some(Overlay::Today { restore });
            } else {
                self.focus = restore;
            }
        }
        effects
    }

    /// How far the Today card has opened: 0 at the card's own place, 1 in the center.
    pub fn today_amount(&self) -> f32 {
        match self.today_motion {
            Some(start) if !self.preferences.reduced_motion => {
                let t = (start.elapsed().as_secs_f32() / TODAY_OPEN).min(1.);
                1. - (1. - t).powi(3)
            }
            _ => 1.,
        }
    }

    /// The focus the page shows: under the menu, the one it closes back to, so the page looks
    /// exactly as before and only the panel's strip is drawn again.
    pub fn page_focus(&self) -> &Target {
        match &self.overlay {
            Some(Overlay::Menu { restore } | Overlay::Today { restore }) => restore,
            _ => &self.focus,
        }
    }

    /// How far the menu is in: 0 closed, 1 open, easing out both ways.
    pub fn menu_amount(&self) -> f32 {
        match self.menu_motion {
            Some((start, true)) => {
                let done = (start.elapsed().as_secs_f32() / MENU_OPEN).min(1.);
                1. - (1. - done).powi(3)
            }
            Some((start, false)) => {
                let done = (start.elapsed().as_secs_f32() / MENU_CLOSE).min(1.);
                (1. - done).powi(3)
            }
            None if matches!(self.overlay, Some(Overlay::Menu { .. })) => 1.,
            None => 0.,
        }
    }

    /// The menu's items from the top: the pages, the inputs while unfolded, then Settings.
    pub fn menu_items(&self) -> Vec<Target> {
        let mut items = vec![Target::Nav(0), Target::Nav(1), Target::MenuInputs];
        if self.menu_inputs {
            items.extend(
                self.inventory
                    .inputs
                    .iter()
                    .filter(|input| self.preferences.input_visible(input))
                    .map(|input| Target::Input(input.id.clone())),
            );
        }
        items.extend([Target::Nav(2), Target::TvSettings]);
        items
    }

    fn menu_direction(&mut self, direction: Direction) {
        let items = self.menu_items();
        let index = items
            .iter()
            .position(|item| *item == self.focus)
            .unwrap_or(0);
        match direction {
            Direction::Up => self.focus = items[index.saturating_sub(1)].clone(),
            Direction::Down => self.focus = items[(index + 1).min(items.len() - 1)].clone(),
            Direction::Right => self.close_menu(),
            Direction::Left => {}
        }
    }

    fn menu_activate(&mut self) -> Vec<Effect> {
        match self.focus.clone() {
            Target::MenuInputs => {
                self.menu_inputs = !self.menu_inputs;
                self.dirty = true;
            }
            Target::Nav(index) => {
                self.close_menu();
                if index != self.page_index() {
                    self.open(index);
                }
            }
            Target::Input(id) => {
                self.close_menu();
                if !self.launching {
                    return vec![Effect::Launch(id)];
                }
            }
            Target::TvSettings => {
                self.close_menu();
                return vec![Effect::Launch("com.palm.app.settings".into())];
            }
            _ => {}
        }
        vec![]
    }

    pub fn open(&mut self, index: usize) {
        self.weather.cancel_search();
        self.page = match index {
            1 => Page::Apps,
            2 => Page::Settings,
            _ => Page::Home,
        };
        self.scroll = 0.;
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        self.query.clear();
        self.filter = 0;
        self.overlay = None;
        self.moving = None;
        self.subpage = None;
        self.section = 0;
        self.panel_started = None;
        self.focus = self.first_content();
        self.dirty = true;
    }

    pub fn apps(&self) -> Vec<&App> {
        let ids = self.preferences.favorites(&self.inventory.apps);
        if self.page == Page::Home {
            return ids
                .iter()
                .filter(|id| !self.preferences.hidden.contains(id))
                .filter_map(|id| self.inventory.apps.iter().find(|app| &app.id == id))
                .collect();
        }
        let query = self.query.to_lowercase();
        let mut apps: Vec<_> = self
            .inventory
            .apps
            .iter()
            .filter(|app| {
                self.preferences.hidden.contains(&app.id) == (self.filter == 5)
                    && (self.filter == 0
                        || self.filter == 5
                        || app.category == CATEGORIES[self.filter - 1])
                    && app.title.to_lowercase().contains(&query)
            })
            .collect();
        apps.sort_by(|a, b| {
            let rank = |app: &App| {
                ids.iter()
                    .position(|id| id == &app.id)
                    .unwrap_or(usize::MAX)
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        apps
    }

    pub fn content_viewport(&self) -> Rect {
        // Home has the screen's top: its header starts on the clock's line.
        if self.page == Page::Home {
            return Rect::new(64., 0., 1792., 1080.);
        }
        // Apps' filters slide away with its content, giving it the screen's top.
        let top = (108.8 - self.chrome_offset()).max(0.);
        Rect::new(64., top, 1792., 1080. - top)
    }

    /// How far the tab bar (and Apps' filters) has slid up. As in tvOS it is either shown or
    /// gone: it slides away whole once the page scrolls, never left half on screen, and comes
    /// back at the top. Settings keep theirs.
    pub fn chrome_offset(&self) -> f32 {
        match self.page {
            Page::Home | Page::Apps => self
                .motion
                .shown(crate::motion::Lane::Chrome, self.chrome_target()),
            Page::Settings => 0.,
        }
    }

    /// How wide Settings' sidebar is drawn: open while it has focus, a column of icons otherwise.
    pub fn sidebar_width(&self) -> f32 {
        let target = self.sidebar_target();
        if self.page == Page::Settings {
            self.motion.shown(crate::motion::Lane::Sidebar, target)
        } else {
            target
        }
    }

    fn sidebar_target(&self) -> f32 {
        if matches!(self.page_focus(), Target::Sidebar(_)) {
            crate::layout::SIDEBAR_EXPANDED
        } else {
            crate::layout::SIDEBAR_COLLAPSED
        }
    }

    fn chrome_target(&self) -> f32 {
        if self.scroll > 0. { CHROME_TRAVEL } else { 0. }
    }

    fn scroll_bottom(&self) -> f32 {
        let viewport = self.content_viewport();
        viewport.y + viewport.h
            - if self.page == Page::Settings {
                if self.subpage == Some(Subpage::Language)
                    || (self.section == 1 && self.subpage.is_none())
                {
                    SETTINGS_LIST_BOTTOM_PADDING
                } else {
                    SETTINGS_BOTTOM_PADDING
                }
            } else {
                12.
            }
    }

    pub fn favorites_viewport(&self) -> Rect {
        let content = self.content_viewport();
        let left = if self.home_scroll > 0. {
            HOME_FADE_LEFT
        } else {
            0.
        };
        Rect::new(left, content.y, HOME_FADE_RIGHT - left, content.h)
    }

    pub fn apps_viewport(&self) -> Rect {
        let content = self.content_viewport();
        // Below the filters, with room for the focused card's lift; up to the top once they
        // have slid away.
        let top = (236. - self.chrome_offset()).max(0.);
        Rect::new(content.x, top, content.w, content.y + content.h - top)
    }

    /// Settings' list, below the page's title: rows scrolled above it are masked and cannot be
    /// clicked.
    pub fn list_viewport(&self) -> Rect {
        let content = self.content_viewport();
        let top = crate::layout::LIST_TOP - 10.;
        Rect::new(0., top, 1920., content.y + content.h - top)
    }

    /// The background gallery, below its title and actions: tiles scrolled above it are masked
    /// and cannot be clicked. It starts where the list's does, so a first-row ring is not faded.
    pub fn backgrounds_viewport(&self) -> Rect {
        let content = self.content_viewport();
        let top = crate::layout::LIST_TOP - 10.;
        Rect::new(176., top, 1684., content.y + content.h - top)
    }

    /// The gallery's tiles: every background, then the upload tile.
    pub fn gallery_slots(&self) -> usize {
        self.background_count() + 1
    }

    pub fn gallery_target(&self, slot: usize) -> Target {
        if slot < self.background_count() {
            Target::Background(slot)
        } else {
            Target::UploadWallpaper
        }
    }

    pub fn gallery_slot(&self, target: &Target) -> Option<usize> {
        match target {
            Target::Background(index) => Some(*index),
            Target::UploadWallpaper => Some(self.background_count()),
            _ => None,
        }
    }

    pub fn hit_test(&self, controls: &[Control], x: f32, y: f32) -> Option<Target> {
        controls
            .iter()
            .rev()
            .find(|control| {
                let viewport = if matches!(self.overlay, Some(Overlay::Today { .. })) {
                    TODAY_LIST
                } else if self.page == Page::Home && matches!(control.target, Target::App(_)) {
                    self.favorites_viewport()
                } else if matches!(
                    control.target,
                    Target::Background(_) | Target::UploadWallpaper
                ) {
                    self.backgrounds_viewport()
                } else if self.page == Page::Apps {
                    self.apps_viewport()
                } else if self.page == Page::Settings {
                    self.list_viewport()
                } else {
                    self.content_viewport()
                };
                control.rect.contains(x, y) && (!control.scrollable || viewport.contains(x, y))
            })
            .map(|control| control.target.clone())
    }

    /// Returns whether this wheel event scrolled favorites, so a pending card press is cancelled.
    pub fn wheel_scroll(
        &mut self,
        controls: &[Control],
        position: Option<(f32, f32)>,
        x: i32,
        y: i32,
    ) -> bool {
        let backgrounds =
            self.page == Page::Settings && self.section == 1 && self.subpage.is_none();
        if backgrounds && position.is_some_and(|(x, y)| !self.backgrounds_viewport().contains(x, y))
        {
            return false;
        }
        let over_favorites = self.page == Page::Home
            && position.is_some_and(|(x, y)| {
                self.favorites_viewport().contains(x, y)
                    && controls.iter().any(|control| {
                        matches!(control.target, Target::App(_)) && control.rect.contains(x, y)
                    })
            });
        let favorites = self.page == Page::Home && (x != 0 || over_favorites);
        if favorites {
            self.scroll_favorites((if x != 0 { x } else { -y }) as f32 * HOME_TILE_PITCH);
        } else {
            let bottom = self.scroll_bottom();
            let max = controls
                .iter()
                .filter(|c| {
                    c.scrollable
                        && (!backgrounds
                            || matches!(c.target, Target::Background(_) | Target::UploadWallpaper))
                })
                .map(|c| c.rect.y + c.rect.h + self.scroll - bottom)
                .fold(0., f32::max);
            self.scroll = (self.scroll - y as f32 * 90.).clamp(0., max.max(0.));
        }
        self.pointer = true;
        self.dirty = true;
        favorites
    }

    pub fn scroll_favorites(&mut self, delta: f32) {
        let count = self
            .preferences
            .favorites(&self.inventory.apps)
            .iter()
            .filter(|id| {
                !self.preferences.hidden.contains(id)
                    && self.inventory.apps.iter().any(|app| &app.id == *id)
            })
            .count();
        let content = if count == 0 {
            0.
        } else {
            (count - 1) as f32 * HOME_TILE_PITCH + HOME_TILE_WIDTH
        };
        let max = (content - (HOME_STRIP_RIGHT - HOME_STRIP_LEFT)).max(0.);
        let next = (self.home_scroll + delta).clamp(0., max);
        if next != self.home_scroll {
            self.home_scroll = next;
            self.dirty = true;
        }
    }

    pub fn set_inventory(&mut self, next: Inventory, warning: Option<String>) {
        self.inventory.apps = next.apps;
        self.sync_custom_tiles();
        if let Some(warning) = warning {
            self.notify(warning);
        } else {
            self.inventory.inputs = next.inputs;
        }
        self.loading = false;
        self.error = None;
        self.inventory_failures = 0;
        self.dirty = true;
        self.reconcile_focus();
    }

    /// Replace only input status; leave installed apps, artwork and saved preferences alone.
    pub fn set_inputs(&mut self, inputs: Vec<crate::model::Input>) {
        if self.inventory.inputs == inputs {
            return;
        }
        self.inventory.inputs = inputs;
        self.dirty = true;
        self.reconcile_focus();
        let missing = |id: &str| !self.inventory.inputs.iter().any(|input| input.id == id);
        if self.page == Page::Settings
            && (matches!(&self.subpage, Some(Subpage::Input(id)) if missing(id))
                || matches!(&self.focus, Target::InputMode(id) if missing(id)))
        {
            self.subpage = Some(Subpage::Inputs);
            self.scroll = 0.;
            self.focus = self
                .inventory
                .inputs
                .first()
                .map_or(Target::Sidebar(WIDGETS_SECTION), |input| {
                    Target::InputMode(input.id.clone())
                });
        }
    }

    /// A failed inventory load. Right after a cold boot, or when the TV is short of memory, the
    /// Homebrew service that reads the inventory can take longer than one request allows. While
    /// no apps are shown yet, the screen keeps loading and the caller retries after the returned
    /// delay; the error and its Retry button only appear once those retries are used up.
    /// Custom tiles come from preferences, not the TV, so they don't count as loaded apps.
    pub fn inventory_failed(&mut self, error: String) -> Option<Duration> {
        self.dirty = true;
        let no_tv_apps = self
            .inventory
            .apps
            .iter()
            .all(|app| app.id.starts_with(CUSTOM_TILE_PREFIX));
        if no_tv_apps && self.inventory_failures < INVENTORY_RETRIES {
            self.inventory_failures += 1;
            return Some(INVENTORY_RETRY_DELAY);
        }
        self.inventory_failures = 0;
        self.loading = false;
        self.error = Some(error);
        None
    }

    /// Custom webhook tiles are never read from the TV. They are merged into
    /// the inventory on every refresh and edit, so favoriting, hiding and
    /// search treat them like any other app.
    pub fn sync_custom_tiles(&mut self) {
        self.inventory
            .apps
            .retain(|app| !app.id.starts_with(CUSTOM_TILE_PREFIX));
        self.inventory.apps.extend(self.preferences.custom_apps());
        self.dirty = true;
    }

    pub fn reconcile_focus(&mut self) {
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if let Target::App(id) = &self.focus
            && !self.apps().iter().any(|app| &app.id == id)
        {
            self.focus = self.first_content();
        }
        if let Target::Input(id) = &self.focus
            && !self
                .inventory
                .inputs
                .iter()
                .any(|input| &input.id == id && self.preferences.input_visible(input))
        {
            // In the menu, a vanished input hands the focus to the list's own item.
            self.focus = if matches!(self.overlay, Some(Overlay::Menu { .. })) {
                Target::MenuInputs
            } else {
                self.first_content()
            };
        }
    }

    pub fn first_content(&self) -> Target {
        match self.page {
            Page::Home => self
                .apps()
                .get(((self.home_scroll - 0.01).max(0.) / HOME_TILE_PITCH).ceil() as usize)
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(if self.error.is_some() {
                    Target::Retry
                } else {
                    Target::Browse
                }),
            Page::Apps => self
                .apps()
                .first()
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(Target::Filter(0)),
            Page::Settings => Target::Sidebar(0),
        }
    }

    pub fn focus(&mut self, target: Target, controls: &[Control]) {
        self.pointer = false;
        self.focus = target;
        self.reveal_focus(controls);
        if let Target::Sidebar(section) = self.focus {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.dirty = true;
    }

    /// Remote focus is revealed after layout changes, including hide/reorder.
    /// Pointer paths deliberately do not call this method.
    pub fn reveal_focus(&mut self, controls: &[Control]) -> bool {
        if matches!(self.overlay, Some(Overlay::Today { .. })) {
            return self.reveal_today_line(controls);
        }
        let previous = self.scroll;
        let previous_home = self.home_scroll;
        // A Today line can leave the card (a match ends, the day turns): focus the favorites.
        if self.page == Page::Home
            && self.overlay.is_none()
            && self.focus.in_today()
            && !controls.iter().any(|c| c.target == self.focus)
        {
            self.focus = self.first_content();
            self.dirty = true;
        }
        if let Some(control) = controls.iter().find(|c| c.target == self.focus) {
            if self.page == Page::Home && matches!(self.focus, Target::App(_)) {
                if control.rect.x < HOME_STRIP_LEFT {
                    self.home_scroll += control.rect.x - HOME_STRIP_LEFT;
                } else if control.rect.x + control.rect.w > HOME_STRIP_RIGHT {
                    self.home_scroll += control.rect.x + control.rect.w - HOME_STRIP_RIGHT;
                }
            }
            if matches!(
                self.focus,
                Target::Nav(_)
                    | Target::Filter(_)
                    | Target::ClearFilter
                    | Target::Search
                    | Target::ClearSearch
                    | Target::Sidebar(_)
            ) {
                self.scroll = 0.;
            } else if control.scrollable {
                let apps = self.page == Page::Apps;
                let first_row = apps
                    && matches!(&self.focus, Target::App(id)
                        if self.apps().iter().position(|app| &app.id == id)
                            .is_some_and(|index| index < APPS_COLUMNS));
                let top = if matches!(self.focus, Target::Background(_) | Target::UploadWallpaper) {
                    crate::layout::GALLERY_TOP
                } else if apps {
                    // The bar has slid away: cards may rise to the safe area.
                    SAFE_TOP
                } else if self.page == Page::Settings {
                    crate::layout::LIST_TOP
                } else {
                    120.8
                };
                // The page may still be easing: measure where the card is once it settles.
                let y = control.rect.y + self.page_scroll() - self.scroll;
                if first_row {
                    // The first row brings the page, and its bar, back to the top.
                    self.scroll = 0.;
                } else {
                    if y < top {
                        self.scroll = (self.scroll + y - top).max(0.);
                    }
                    let bottom = if apps {
                        1080. - SAFE_TOP
                    } else {
                        self.scroll_bottom()
                    };
                    if y + control.rect.h > bottom {
                        self.scroll += y + control.rect.h - bottom;
                    }
                }
            }
        }
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if self.scroll != previous || self.home_scroll != previous_home {
            self.dirty = true;
        }
        self.scroll != previous || self.home_scroll != previous_home
    }

    /// The opened Today card's focus: on one of its lines (the first, once it opens or when its
    /// line leaves), scrolled into the list. With no lines left it closes.
    fn reveal_today_line(&mut self, controls: &[Control]) -> bool {
        let previous = (self.focus.clone(), self.today_scroll);
        let controls: Vec<&Control> = controls
            .iter()
            .filter(|c| !matches!(c.target, Target::TodayTab(_)))
            .collect();
        if !controls.iter().any(|c| c.target == self.focus) {
            match controls.first() {
                Some(first) => self.focus = first.target.clone(),
                None => {
                    self.overlay = None;
                    self.today_motion = None;
                    self.focus = Target::TodayCard;
                    self.dirty = true;
                    return true;
                }
            }
        }
        let first = controls.first().is_some_and(|c| c.target == self.focus);
        if first {
            self.today_scroll = 0.;
        } else if let Some(control) = controls.iter().find(|c| c.target == self.focus) {
            // The list may still be easing: measure where the line is once it settles.
            let shown = self
                .motion
                .shown(crate::motion::Lane::Today, self.today_scroll);
            let y = control.rect.y + shown - self.today_scroll;
            // Room above for a section's heading, below for the fade.
            let (top, bottom) = (TODAY_LIST.y + 48., TODAY_LIST.y + TODAY_LIST.h - 32.);
            if y < top {
                self.today_scroll = (self.today_scroll + y - top).max(0.);
            } else if y + control.rect.h > bottom {
                self.today_scroll += y + control.rect.h - bottom;
            }
        }
        let changed = previous != (self.focus.clone(), self.today_scroll);
        self.dirty |= changed;
        changed
    }

    /// Hover and pointer-down must not move the control before pointer-up.
    pub fn pointer_focus(&mut self, target: Target) {
        if !self.pointer {
            self.dirty = true;
        }
        self.pointer = true;
        if self.focus == target {
            return;
        }
        if let Target::Sidebar(section) = target {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            if self.section != section || self.subpage.is_some() {
                self.scroll = 0.;
            }
            self.section = section;
            self.subpage = None;
        }
        self.focus = target;
        self.dirty = true;
    }

    /// How many matches CS2 Scores lists.
    pub fn cs2_count(&self) -> usize {
        self.cs2.feed.as_ref().map_or(0, |feed| {
            crate::cs2::shown(
                feed,
                self.preferences.cs2_major_only,
                self.preferences.cs2_results,
            )
            .len()
        })
    }

    /// Opens the app that plays a show, or says which service has it.
    /// Lampa, where the shows of TV Shows and Today play, asked to open `card` ("tv_2190").
    fn open_lampa(&mut self, card: &str) -> Vec<Effect> {
        let lampa = self.inventory.apps.iter().find(|app| {
            app.id.to_lowercase().contains("lampa") || app.title.to_lowercase().contains("lampa")
        });
        match lampa {
            Some(app) => vec![Effect::OpenCard {
                app: app.id.clone(),
                card: card.to_owned(),
            }],
            None => {
                self.notify(self.preferences.language.t("lampaNoApp").to_owned());
                vec![]
            }
        }
    }

    /// The user confirmed CUB's code: the sign-in dialog gives way to Lampa's settings.
    pub fn cub_signed_in(&mut self) {
        if matches!(self.overlay, Some(Overlay::CubSignIn)) {
            self.overlay = None;
            self.focus = Target::LampaAccount;
            self.pressed = None;
        }
        self.notify(self.preferences.language.t("lampaSignedIn").to_owned());
    }

    /// Scrolls the TV Shows row so the card at `index` is in view.
    pub fn shows_reveal(&mut self, index: usize) {
        let visible = crate::shows::VISIBLE;
        if index < self.shows_first {
            self.shows_first = index;
        } else if index >= self.shows_first + visible {
            self.shows_first = index + 1 - visible;
        }
        self.shows_first = self
            .shows_first
            .min(self.lampa_cards().len().saturating_sub(visible));
    }

    /// The cards of Home's Lampa row, as Lampa's card IDs: the series Lampa watches now, or the
    /// account's history, latest first.
    pub fn lampa_cards(&self) -> Vec<String> {
        match self.preferences.lampa_feed {
            crate::model::LampaFeed::Watching => self
                .shows
                .list
                .iter()
                .map(|show| format!("tv_{}", show.id))
                .collect(),
            crate::model::LampaFeed::Continue if self.cub.signed_in() => self
                .cub
                .watched
                .iter()
                .take(crate::shows::SHOWN)
                .map(|watched| watched.card.clone())
                .collect(),
            crate::model::LampaFeed::Continue => Vec::new(),
        }
    }

    /// Scrolls the CS2 Scores row so the card at `index` is in view.
    pub fn cs2_reveal(&mut self, index: usize) {
        let visible = crate::cs2::VISIBLE;
        if index < self.cs2_first {
            self.cs2_first = index;
        } else if index >= self.cs2_first + visible {
            self.cs2_first = index + 1 - visible;
        }
        self.cs2_first = self.cs2_first.min(self.cs2_count().saturating_sub(visible));
    }

    pub fn hidden_count(&self) -> usize {
        self.inventory
            .apps
            .iter()
            .filter(|app| self.preferences.hidden.contains(&app.id))
            .count()
    }

    pub fn background_count(&self) -> usize {
        self.background_ids().count()
    }

    /// The background to draw: the chosen one, with its frame when it is a dynamic wallpaper
    /// (`id#frame`).
    pub fn background_key(&self) -> String {
        if self.preferences.background == crate::aerials::ID {
            let place = &self.aerial_place().id;
            if self.aerials.iter().any(|still| still.id == *place) {
                return format!("{}#{place}", crate::aerials::ID);
            }
            // Until its still arrives, the last packaged picture chosen stays.
            let splash = self.preferences.splash_background();
            return if splash == crate::aerials::ID {
                "dunes".into()
            } else {
                splash.to_owned()
            };
        }
        match &self.dynamic_frame {
            Some((id, frame, _)) if *id == self.preferences.background => format!("{id}#{frame}"),
            _ => self.preferences.background.clone(),
        }
    }

    /// Follows the sun with a dynamic wallpaper: at most every half minute, the frame nearest the
    /// sun at the weather location (else the time zone's meridian), or the time of day, fading
    /// slowly to it. A newly chosen wallpaper starts at its frame without a fade of its own.
    pub fn sync_dynamic_wallpaper(&mut self, now: time::OffsetDateTime) {
        use crate::dynamic_wallpaper::{FADE, choose, sun};
        let unix = now.unix_timestamp();
        let id = self.preferences.background.clone();
        let id = &id;
        let Some(item) = self.wallpapers.iter().find(|item| item.id == *id) else {
            self.dynamic_frame = None;
            return;
        };
        if item.frames.is_empty() {
            self.dynamic_frame = None;
            return;
        }
        let same = self
            .dynamic_frame
            .as_ref()
            .filter(|(chosen, _, _)| chosen == id)
            .cloned();
        if same
            .as_ref()
            .is_some_and(|(_, _, checked)| (unix - checked).abs() < 30)
        {
            return;
        }
        let (latitude, longitude) = self
            .preferences
            .weather_location
            .as_ref()
            .map_or((45., now.offset().whole_seconds() as f64 / 240.), |place| {
                (place.latitude, place.longitude)
            });
        let day =
            (now.hour() as f32 * 3600. + now.minute() as f32 * 60. + now.second() as f32) / 86400.;
        let frame = choose(&item.frames, sun(latitude, longitude, unix), day).unwrap_or(0);
        if let Some((_, previous, _)) = &same
            && *previous != frame
            && !self.preferences.reduced_motion
            && self.previous_background.is_none()
        {
            self.fade_wallpaper_from(self.background_key(), FADE);
        }
        self.dirty |= same
            .as_ref()
            .is_none_or(|(_, previous, _)| *previous != frame);
        self.dynamic_frame = Some((id.clone(), frame, unix));
    }

    /// Fades from the wallpaper `shown` to whichever is chosen now, over `fade` seconds; held
    /// on `shown` first when the renderer holds fades.
    fn fade_wallpaper_from(&mut self, shown: String, fade: f32) {
        self.previous_background = Some((shown, Instant::now()));
        self.background_fade = fade;
        self.wallpaper_waiting = self.hold_fades;
    }

    /// The renderer is ready for the held fade: it runs from now.
    pub fn release_wallpaper_fade(&mut self) {
        self.wallpaper_waiting = false;
        if let Some((_, start)) = &mut self.previous_background {
            *start = Instant::now();
        }
        self.dirty = true;
    }

    /// The Apple TV place the rotation is on, shown or not.
    pub fn aerial_place(&self) -> &'static crate::aerials::Picture {
        let pictures = &crate::aerials::catalog().pictures;
        &pictures[self.preferences.aerials.current(pictures.len())]
    }

    /// The Apple TV stills to keep on the TV: the place the gallery card shows and, while it is the
    /// background, the next one and the one fading out.
    pub fn aerial_ids(&self) -> Vec<String> {
        let pictures = &crate::aerials::catalog().pictures;
        let rotation = self.preferences.aerials;
        let mut ids = vec![self.aerial_place().id.clone()];
        if self.preferences.background == crate::aerials::ID {
            ids.push(pictures[rotation.next(pictures.len())].id.clone());
            if let Some(previous) = &self.aerials_previous
                && !ids.contains(previous)
            {
                ids.push(previous.clone());
            }
        }
        ids
    }

    /// An Apple TV still is on the TV; when it is the place waited for, fade to it.
    pub fn aerial_ready(&mut self, still: crate::wallpapers::Wallpaper) {
        let shown = self.background_key();
        let wanted = self.aerial_ids();
        self.aerials
            .retain(|known| known.id != still.id && wanted.contains(&known.id));
        if wanted.contains(&still.id) {
            self.aerials.push(still);
        }
        let key = self.background_key();
        if key != shown && !self.preferences.reduced_motion {
            self.fade_wallpaper_from(shown, CYCLE_FADE);
        }
        self.dirty = true;
    }

    /// Moves Apple TV to its next place when one is due and its still has arrived.
    pub fn advance_aerials(&mut self, now: u64) -> bool {
        let pictures = &crate::aerials::catalog().pictures;
        let rotation = self.preferences.aerials;
        if self.preferences.background != crate::aerials::ID || !rotation.due(now) {
            return false;
        }
        let next = &pictures[rotation.next(pictures.len())].id;
        if !self.aerials.iter().any(|still| still.id == *next) {
            return false;
        }
        let shown = self.background_key();
        self.aerials_previous = Some(self.aerial_place().id.clone());
        self.preferences.aerials = rotation.advanced(pictures.len(), now);
        if self.preferences.reduced_motion {
            self.previous_background = None;
        } else {
            self.fade_wallpaper_from(shown, CYCLE_FADE);
        }
        self.aerials_request = true;
        self.dirty = true;
        true
    }

    pub fn cycle_wallpaper(&mut self, now: u64) -> bool {
        // Apple TV changes its own place (advance_aerials).
        if self.preferences.background == crate::aerials::ID {
            return false;
        }
        let ids: Vec<_> = self
            .background_ids()
            .filter(|id| *id != crate::aerials::ID)
            .filter(|id| self.deleting_wallpaper.as_deref() != Some(*id))
            .map(str::to_owned)
            .collect();
        let next = crate::wallpaper_cycle::next(&ids, &self.preferences.background);
        self.preferences.wallpaper_changed = now;
        let Some(id) = next else {
            return false;
        };
        self.preferences.splash_background = Some(self.preferences.splash_background().to_owned());
        if self.preferences.reduced_motion {
            self.previous_background = None;
        } else {
            self.fade_wallpaper_from(self.background_key(), CYCLE_FADE);
        }
        self.preferences.background = id;
        self.dirty = true;
        true
    }

    /// Takes the saved place's names, and its id, from a lookup in the current language, if it
    /// is still the saved place. True when anything changed and the preferences need saving.
    pub fn rename_location(&mut self, place: crate::weather::Location) -> bool {
        let Some(location) = self
            .preferences
            .weather_location
            .as_mut()
            .filter(|location| location.key() == place.key())
        else {
            return false;
        };
        let id = place.id.or(location.id);
        if (
            &location.name,
            &location.region,
            &location.country,
            location.id,
        ) == (&place.name, &place.region, &place.country, id)
        {
            return false;
        }
        location.name = place.name;
        location.region = place.region;
        location.country = place.country;
        location.id = id;
        self.dirty = true;
        true
    }

    pub fn background_id(&self, index: usize) -> Option<&str> {
        self.background_ids().nth(index)
    }

    fn background_ids(&self) -> impl Iterator<Item = &str> {
        BACKGROUNDS
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| {
                *id == self.preferences.background
                    || !self
                        .preferences
                        .deleted_backgrounds
                        .iter()
                        .any(|deleted| deleted == id)
            })
            .chain(self.wallpapers.iter().map(|item| item.id.as_str()))
            .chain(std::iter::once(crate::aerials::ID))
    }

    /// Apple TV is never removed: its stills are only kept while shown, so there is nothing to free.
    pub fn can_delete_background(&self, id: &str) -> bool {
        id != self.preferences.background
            && id != crate::aerials::ID
            && self.background_ids().any(|known| known == id)
    }

    fn background_target(&self, id: &str) -> Target {
        Target::Background(
            self.background_ids()
                .position(|known| known == id)
                .unwrap_or(0),
        )
    }

    pub fn wallpaper_deleted(&mut self, id: String, result: Result<(), String>) {
        if self.deleting_wallpaper.as_deref() != Some(&id) {
            return;
        }
        self.deleting_wallpaper = None;
        if let Err(error) = result {
            eprintln!("Luma background deletion: {error}");
            self.notify(self.preferences.language.t("deleteBackgroundError"));
            return;
        }
        self.finish_background_delete(&id);
    }

    fn finish_background_delete(&mut self, id: &str) {
        let index = self
            .background_ids()
            .position(|known| known == id)
            .unwrap_or(0);
        let focused = match self.focus {
            Target::Background(index) => self.background_id(index).map(str::to_owned),
            _ => None,
        };
        self.wallpapers.retain(|item| item.id != id);
        if BACKGROUNDS.iter().any(|(known, _)| *known == id) {
            self.preferences.deleted_backgrounds.push(id.to_owned());
        }
        if matches!(&self.overlay, Some(Overlay::BackgroundOptions { id: current, .. } | Overlay::BackgroundDelete { id: current, .. }) if current == id)
        {
            self.overlay = None;
            self.panel_started = None;
            self.focus = Target::Background(index.min(self.background_count().saturating_sub(1)));
            self.pointer = false;
        } else if let Some(focused) = focused {
            self.focus = self.background_target(&focused);
        }
        self.pressed = None;
        self.notify(self.preferences.language.t("backgroundDeleted"));
    }

    pub fn set_wallpapers(&mut self, wallpapers: Vec<crate::wallpapers::Wallpaper>) {
        // A newly uploaded ID can sort before existing entries. Keep the focused
        // image, rather than moving the pointer's target to a different photograph.
        let focused_id = if let Target::Background(index) = self.focus {
            self.background_id(index).map(str::to_owned)
        } else {
            None
        };
        self.wallpapers = wallpapers;
        if let Some(
            Overlay::BackgroundOptions { id, restore } | Overlay::BackgroundDelete { id, restore },
        ) = &self.overlay
            && !self.background_ids().any(|known| known == id)
        {
            let index = if let Target::Background(index) = restore {
                *index
            } else {
                0
            };
            self.focus = Target::Background(index.min(self.background_count().saturating_sub(1)));
            self.overlay = None;
            self.panel_started = None;
            self.pressed = None;
            self.pointer = false;
        }
        if let Some(id) = focused_id {
            let index = (0..self.background_count())
                .find(|&i| self.background_id(i) == Some(id.as_str()))
                .unwrap_or(0);
            self.focus = Target::Background(index);
        }
        self.dirty = true;
    }

    fn section_entry(&self) -> Target {
        match self.section {
            BACKGROUND_SECTION => Target::Background(
                (0..self.background_count())
                    .position(|index| {
                        self.background_id(index) == Some(self.preferences.background.as_str())
                    })
                    .unwrap_or(0),
            ),
            WIDGETS_SECTION => Target::Widget(4),
            SYSTEM_SECTION if self.available_update.is_some() => Target::InstallUpdate,
            SYSTEM_SECTION => Target::CheckUpdates,
            _ => Target::Setting(0),
        }
    }

    fn close_subpage(&mut self) {
        if self.subpage == Some(Subpage::WeatherLocation) {
            self.weather.cancel_search();
        }
        self.panel_started = None;
        self.pointer = false;
        self.focus = match self.subpage.take() {
            Some(Subpage::AppNames) => Target::Setting(0),
            Some(Subpage::Greeting) => Target::Widget(0),
            Some(Subpage::Date) => Target::Widget(4),
            Some(Subpage::DateFormat) => {
                self.subpage = Some(Subpage::Date);
                Target::DateFormat
            }
            Some(Subpage::Weather) => Target::Widget(1),
            Some(Subpage::Cs2) => Target::Widget(6),
            Some(Subpage::Shows) => Target::Widget(7),
            Some(Subpage::Shortcuts) => Target::Widget(2),
            Some(Subpage::Inputs) => Target::Widget(3),
            Some(Subpage::Crypto) => Target::Widget(5),
            Some(Subpage::Clock) => Target::Setting(1),
            Some(Subpage::ClockFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockFormat
            }
            Some(Subpage::TimeFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockTimeFormat
            }
            Some(Subpage::Input(id)) => {
                self.subpage = Some(Subpage::Inputs);
                Target::InputMode(id)
            }
            Some(Subpage::WeatherLocation) => {
                self.subpage = Some(Subpage::Weather);
                Target::WeatherLocation
            }
            Some(Subpage::Temperature) => {
                self.subpage = Some(Subpage::Weather);
                Target::WeatherTemperature
            }
            Some(Subpage::Language) => Target::Setting(4),
            Some(Subpage::Appearance) => Target::Setting(INTERFACE_SCALE_ROW),
            Some(Subpage::TextRendering) => Target::Setting(7),
            Some(Subpage::HomeTakeover) => Target::Setting(HOME_TAKEOVER_ROW),
            None => Target::Sidebar(self.section),
        };
        self.scroll = 0.;
        self.dirty = true;
    }

    pub fn direction(&mut self, direction: Direction, controls: &[Control]) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.recents_open() {
            return self.recent_direction(direction);
        }
        if matches!(self.overlay, Some(Overlay::SystemKeyboard)) {
            // The TV keyboard owns its arrows until Done/Back dismisses it.
            return vec![];
        }
        if matches!(
            self.overlay,
            Some(
                Overlay::WallpaperUpload
                    | Overlay::TileEditor
                    | Overlay::CubSignIn
                    | Overlay::LampaPlugin
            )
        ) {
            return vec![];
        }
        if matches!(self.overlay, Some(Overlay::Menu { .. })) {
            self.menu_direction(direction);
            return vec![];
        }
        if matches!(self.overlay, Some(Overlay::Today { .. })) {
            // Left and Right switch between the widgets' tabs; Up and Down walk the tab's lines,
            // through its sections. Both stop at either end.
            if matches!(direction, Direction::Left | Direction::Right) {
                let tabs: Vec<crate::today::Tab> = controls
                    .iter()
                    .filter_map(|c| match c.target {
                        Target::TodayTab(tab) => Some(tab),
                        _ => None,
                    })
                    .collect();
                let current = self
                    .today_tab
                    .and_then(|tab| tabs.iter().position(|t| *t == tab))
                    .unwrap_or(0);
                let next = match direction {
                    Direction::Left => current.checked_sub(1),
                    _ => Some(current + 1),
                };
                if let Some(&tab) = next.and_then(|index| tabs.get(index)) {
                    self.show_today_tab(tab);
                }
                return vec![];
            }
            let lines: Vec<Control> = controls
                .iter()
                .filter(|c| !matches!(c.target, Target::TodayTab(_)))
                .cloned()
                .collect();
            let index = lines.iter().position(|c| c.target == self.focus);
            let next = match (direction, index) {
                (Direction::Up, Some(index)) => index.checked_sub(1),
                (Direction::Down, Some(index)) => Some(index + 1),
                (Direction::Up | Direction::Down, None) => Some(0),
                _ => None,
            };
            if let Some(control) = next.and_then(|index| lines.get(index)) {
                self.focus(control.target.clone(), controls);
            }
            return vec![];
        }
        if let Some(id) = self.moving.clone() {
            let mut ids = self.preferences.favorites(&self.inventory.apps);
            let slots: Vec<_> = ids
                .iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    (!self.preferences.hidden.contains(id)
                        && self.inventory.apps.iter().any(|app| &app.id == id))
                    .then_some(index)
                })
                .collect();
            if let Some(index) = slots.iter().position(|slot| ids[*slot] == id) {
                let next = match direction {
                    Direction::Left => index.saturating_sub(1),
                    Direction::Right => (index + 1).min(slots.len() - 1),
                    _ => index,
                };
                if next != index {
                    ids.swap(slots[index], slots[next]);
                    self.preferences.favorites = Some(ids);
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            return vec![];
        }
        if self.overlay.is_none() {
            if self.page == Page::Home
                && let Target::App(id) = &self.focus
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    match direction {
                        Direction::Left if index == 0 => self.open_menu(),
                        Direction::Left | Direction::Right => {
                            let next = if matches!(direction, Direction::Left) {
                                index.saturating_sub(1)
                            } else {
                                (index + 1).min(apps.len() - 1)
                            };
                            let id = apps[next].id.clone();
                            self.focus(Target::App(id), controls);
                        }
                        // Up reaches the center's nearest card, else Today's last line; nothing
                        // lies below.
                        Direction::Up => {
                            let next = across(controls, &self.focus, Target::in_center)
                                .or_else(|| today_last(controls));
                            if let Some(next) = next {
                                self.focus(next, controls)
                            }
                        }
                        Direction::Down => {}
                    }
                    return vec![];
                }
            }
            let setting_rows: Option<&[Target]> = match self.subpage {
                Some(Subpage::Greeting) => Some(&[
                    Target::GreetingToggle,
                    Target::GreetingName,
                    Target::GreetingNameClear,
                ]),
                Some(Subpage::AppNames) => Some(&[Target::AppNamesHome, Target::AppNamesApps]),
                Some(Subpage::Date) => Some(&[Target::DateToggle, Target::DateFormat]),
                Some(Subpage::Clock) => Some(&[Target::ClockFormat, Target::ClockTimeFormat]),
                _ => None,
            };
            if let Some(rows) = setting_rows
                && let Some(index) = rows.iter().position(|target| *target == self.focus)
            {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            self.focus.clone()
                        } else {
                            rows[index - 1].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Right
                        if matches!(
                            self.focus,
                            Target::GreetingName
                                | Target::ClockFormat
                                | Target::ClockTimeFormat
                                | Target::DateFormat
                        ) =>
                    {
                        return self.activate();
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Choice(index) = self.focus {
                let count = match self.subpage {
                    Some(
                        Subpage::ClockFormat
                        | Subpage::TimeFormat
                        | Subpage::Temperature
                        | Subpage::TextRendering
                        | Subpage::HomeTakeover,
                    ) => 2,
                    Some(Subpage::Language) => Language::ALL.len(),
                    Some(Subpage::Appearance) => UiSize::ALL.len(),
                    Some(Subpage::DateFormat) => crate::model::DateFormat::ALL.len(),
                    _ => 3,
                };
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            self.focus.clone()
                        } else {
                            Target::Choice(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down
                        if index + 1 == count
                            && matches!(self.subpage, Some(Subpage::Input(_))) =>
                    {
                        self.focus(Target::InputRename, controls)
                    }
                    Direction::Down => self.focus(Target::Choice((index + 1) % count), controls),
                    Direction::Right => {}
                }
                return vec![];
            }
            if matches!(self.focus, Target::InputRename | Target::InputNameReset) {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if self.focus == Target::InputNameReset {
                            Target::InputRename
                        } else {
                            Target::Choice(2)
                        },
                        controls,
                    ),
                    Direction::Down
                        if self.focus == Target::InputRename
                            && controls.iter().any(|c| c.target == Target::InputNameReset) =>
                    {
                        self.focus(Target::InputNameReset, controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Sidebar(index) = self.focus {
                match direction {
                    Direction::Right => self.focus(self.section_entry(), controls),
                    Direction::Up => self.focus(
                        if index == 0 {
                            self.focus.clone()
                        } else {
                            Target::Sidebar(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down => {
                        self.focus(Target::Sidebar((index + 1) % SECTION_COUNT), controls)
                    }
                    Direction::Left => self.open_menu(),
                }
                return vec![];
            }
            if (matches!(
                self.focus,
                Target::TileEditor
                    | Target::Tile(_)
                    | Target::Cs2Toggle
                    | Target::Cs2Layout
                    | Target::ShowsToggle
                    | Target::ShowsLayout
                    | Target::LampaAccount
                    | Target::LampaFeed
                    | Target::LampaPlugin
                    | Target::Cs2Major
                    | Target::Cs2Results
                    | Target::InputMode(_)
                    | Target::CryptoToggle
                    | Target::CryptoLayout
                    | Target::CryptoSearch
                    | Target::CryptoResult(_)
                    | Target::CryptoCoin(_)
                    | Target::WeatherToggle
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
                    | Target::Widget(_)
            ) || (self.focus == Target::Setting(3) && self.subpage == Some(Subpage::Inputs)))
                && matches!(direction, Direction::Left)
            {
                self.close_subpage();
                return vec![];
            }
            if matches!(
                self.focus,
                Target::Setting(0 | 1 | 4 | 7 | INTERFACE_SCALE_ROW)
                    | Target::Widget(_)
                    | Target::InputMode(_)
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
                    | Target::Setting(HOME_TAKEOVER_ROW)
            ) && matches!(direction, Direction::Right)
            {
                return self.activate();
            }
            if matches!(self.focus, Target::Setting(_) | Target::WeatherToggle)
                && matches!(direction, Direction::Right)
            {
                return vec![];
            }
            if let Target::InputMode(id) = &self.focus
                && matches!(direction, Direction::Up)
                && self
                    .inventory
                    .inputs
                    .first()
                    .is_some_and(|input| &input.id == id)
            {
                return vec![];
            }
            // Crypto's switches: Right does nothing and Up from the first reaches the tabs.
            if matches!(
                self.focus,
                Target::CryptoToggle
                    | Target::CryptoLayout
                    | Target::CryptoSearch
                    | Target::CryptoResult(_)
                    | Target::CryptoCoin(_)
            ) {
                match direction {
                    Direction::Right => return vec![],
                    Direction::Up if self.focus == Target::CryptoToggle => {
                        return vec![];
                    }
                    _ => {}
                }
            }
            if self.focus == Target::Setting(0) && matches!(direction, Direction::Up) {
                return vec![];
            }
            // The center's cards: Up reaches Today's last line, Down the nearest favorite.
            if self.page == Page::Home
                && self.focus.in_center()
                && matches!(direction, Direction::Up | Direction::Down)
            {
                let next = if matches!(direction, Direction::Up) {
                    today_last(controls)
                } else {
                    across(controls, &self.focus, |target| {
                        matches!(target, Target::App(_))
                    })
                };
                if let Some(next) = next {
                    self.focus(next, controls);
                }
                return vec![];
            }
            // CS2 Scores' cards: Left and Right step through every match, scrolling the row; Left
            // from the first opens the menu.
            if let Target::Cs2Match(index) = self.focus
                && matches!(direction, Direction::Left | Direction::Right)
            {
                let count = self.cs2_count();
                let next = match direction {
                    Direction::Left => index.checked_sub(1),
                    _ => (index + 1 < count).then_some(index + 1),
                };
                if let Some(next) = next {
                    self.cs2_reveal(next);
                    self.focus = Target::Cs2Match(next);
                    self.dirty = true;
                } else if matches!(direction, Direction::Left) {
                    self.open_menu();
                }
                return vec![];
            }
            // The Today card: Down reaches the center below, else the favorites; Left opens the
            // menu, and nothing lies above or to the right.
            if self.focus.in_today() {
                match direction {
                    Direction::Left => self.open_menu(),
                    Direction::Down => {
                        let next = across(controls, &self.focus, Target::in_center)
                            .unwrap_or_else(|| self.first_content());
                        self.focus(next, controls);
                    }
                    Direction::Up | Direction::Right => {}
                }
                return vec![];
            }
            // TV Shows' cards: Left and Right step through every show, scrolling the row; Left
            // from the first opens the menu.
            if let Target::Show(index) = self.focus
                && matches!(direction, Direction::Left | Direction::Right)
            {
                let next = match direction {
                    Direction::Left => index.checked_sub(1),
                    _ => (index + 1 < self.lampa_cards().len()).then_some(index + 1),
                };
                if let Some(next) = next {
                    self.shows_reveal(next);
                    self.focus = Target::Show(next);
                    self.dirty = true;
                } else if matches!(direction, Direction::Left) {
                    self.open_menu();
                }
                return vec![];
            }
            // CS2 Scores', TV Shows' and Lampa's rows: Right does nothing and Up from the first
            // reaches the tabs.
            if matches!(
                self.focus,
                Target::Cs2Toggle
                    | Target::Cs2Layout
                    | Target::Cs2Major
                    | Target::Cs2Results
                    | Target::ShowsToggle
                    | Target::ShowsLayout
                    | Target::LampaAccount
                    | Target::LampaFeed
                    | Target::LampaPlugin
            ) {
                match direction {
                    Direction::Right => return vec![],
                    Direction::Up
                        if matches!(self.focus, Target::Cs2Toggle | Target::ShowsToggle) =>
                    {
                        return vec![];
                    }
                    _ => {}
                }
            }
            if self.focus == Target::WeatherToggle && matches!(direction, Direction::Up) {
                return vec![];
            }
            if matches!(
                self.focus,
                Target::WeatherQuery | Target::WeatherSearch | Target::WeatherClear
            ) {
                match direction {
                    Direction::Left => {
                        self.close_subpage();
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(
                            if self.preferences.weather_location.is_some()
                                && self.focus != Target::WeatherClear
                            {
                                Target::WeatherClear
                            } else {
                                self.focus.clone()
                            },
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Down => {
                        let target = if self.focus == Target::WeatherClear {
                            Some(Target::WeatherQuery)
                        } else if !self.weather.results.is_empty() {
                            Some(Target::WeatherResult(0))
                        } else if self.focus == Target::WeatherQuery {
                            Some(Target::WeatherSearch)
                        } else {
                            None
                        };
                        if let Some(target) = target {
                            self.focus(target, controls);
                        }
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::WeatherResult(index) = self.focus {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::WeatherQuery
                        } else {
                            Target::WeatherResult(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < self.weather.results.len() => {
                        self.focus(Target::WeatherResult(index + 1), controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::Search {
                match direction {
                    Direction::Left => {
                        self.focus(
                            Target::Filter(if self.hidden_count() > 0 { 5 } else { 4 }),
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Up => return vec![],
                    _ => {}
                }
            }
            if self.page == Page::Apps
                && let Target::App(id) = &self.focus
                && matches!(direction, Direction::Up | Direction::Down)
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    // A preceding row may be above the visible scroll region.
                    // Its grid index still defines the route; reveal_focus then
                    // scrolls it into view, without letting the fixed nav steal Up.
                    let next = match direction {
                        Direction::Up if index < APPS_COLUMNS => Some(Target::Filter(0)),
                        Direction::Up => Some(Target::App(apps[index - APPS_COLUMNS].id.clone())),
                        Direction::Down
                            if (index / APPS_COLUMNS + 1) * APPS_COLUMNS < apps.len() =>
                        {
                            Some(Target::App(
                                apps[(index + APPS_COLUMNS).min(apps.len() - 1)].id.clone(),
                            ))
                        }
                        _ => None,
                    };
                    if let Some(next) = next {
                        self.focus(next, controls);
                    }
                    return vec![];
                }
            }
            if let Some(slot) = self.gallery_slot(&self.focus) {
                const COLUMNS: usize = crate::layout::GALLERY_COLUMNS;
                let slots = self.gallery_slots();
                let next = match direction {
                    Direction::Right => (slot + 1 < slots).then(|| self.gallery_target(slot + 1)),
                    Direction::Left if slot % COLUMNS != 0 => Some(self.gallery_target(slot - 1)),
                    Direction::Left => Some(Target::Sidebar(self.section)),
                    Direction::Up if slot >= COLUMNS => Some(self.gallery_target(slot - COLUMNS)),
                    Direction::Up if slot % COLUMNS == COLUMNS - 1 => Some(Target::WallpaperCycle),
                    Direction::Up => Some(Target::WallpaperGradient),
                    Direction::Down if slot + COLUMNS < slots => {
                        Some(self.gallery_target(slot + COLUMNS))
                    }
                    // A short last row: the last tile stands under the column.
                    Direction::Down if (slot / COLUMNS + 1) * COLUMNS < slots => {
                        Some(self.gallery_target(slots - 1))
                    }
                    Direction::Down => None,
                };
                if let Some(next) = next {
                    self.focus(next, controls);
                }
                return vec![];
            }
            if self.focus == Target::WallpaperGradient {
                match direction {
                    Direction::Right => self.focus(Target::WallpaperCycle, controls),
                    Direction::Left => self.focus(Target::Sidebar(self.section), controls),
                    Direction::Up => {}
                    Direction::Down => self.focus(
                        self.gallery_target(2.min(self.gallery_slots() - 1)),
                        controls,
                    ),
                }
                return vec![];
            }
            if self.focus == Target::WallpaperCycle {
                match direction {
                    Direction::Right => {}
                    Direction::Left => self.focus(Target::WallpaperGradient, controls),
                    Direction::Up => {}
                    Direction::Down => self.focus(
                        self.gallery_target(3.min(self.gallery_slots() - 1)),
                        controls,
                    ),
                }
                return vec![];
            }
            if self.page == Page::Settings
                && matches!(direction, Direction::Left)
                && !matches!(
                    self.focus,
                    Target::Nav(_) | Target::Sidebar(_) | Target::TvSettings
                )
            {
                self.focus(Target::Sidebar(self.section), controls);
                return vec![];
            }
        }
        let Some(current) = controls.iter().find(|c| c.target == self.focus) else {
            self.focus(self.first_content(), controls);
            return vec![];
        };
        let (x, y) = current.rect.center();
        let next = controls
            .iter()
            .filter(|candidate| candidate.target != self.focus)
            .filter(|candidate| {
                self.page != Page::Home
                    || !matches!(candidate.target, Target::App(_))
                    || (candidate.rect.x >= HOME_STRIP_LEFT - 0.1
                        && candidate.rect.x + candidate.rect.w <= HOME_STRIP_RIGHT + 0.1)
            })
            .filter_map(|candidate| {
                let (cx, cy) = candidate.rect.center();
                let (primary, cross, overlap) = match direction {
                    Direction::Up => (
                        y - cy,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Down => (
                        cy - y,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Left => (
                        x - cx,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                    Direction::Right => (
                        cx - x,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                };
                (primary > 1.).then_some((
                    candidate.target.clone(),
                    primary + cross * 2. + if cross >= overlap { ASIDE } else { 0. },
                ))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match next {
            // Nothing lies to the left in line, only in another row: the menu does.
            _ if matches!(direction, Direction::Left)
                && self.overlay.is_none()
                && next.as_ref().is_none_or(|(_, score)| *score >= ASIDE) =>
            {
                self.open_menu()
            }
            Some((next, _)) => self.focus(next, controls),
            None => {}
        }
        vec![]
    }

    pub fn options(&mut self) {
        if self.overlay.is_some() || self.moving.is_some() {
            return;
        }
        if let Target::Background(index) = self.focus {
            if let Some(id) = self.background_id(index).map(str::to_owned) {
                self.overlay = Some(Overlay::BackgroundOptions {
                    id,
                    restore: self.focus.clone(),
                });
                self.focus = Target::UseBackground;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.dirty = true;
            }
        } else if let Target::Tile(id) = &self.focus {
            if self.preferences.custom_tile(id).is_none() {
                return;
            }
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
            });
            self.focus = Target::TileTest(id.clone());
            self.panel_started = (!self.preferences.reduced_motion).then(Instant::now);
            self.dirty = true;
        } else if let Target::App(id) = &self.focus {
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
            });
            self.focus = Target::OpenApp;
            if !self.preferences.reduced_motion {
                self.panel_started = Some(Instant::now());
            }
            self.dirty = true;
        }
    }

    pub fn uninstall_response(&mut self, id: String, result: Result<(), String>) -> Vec<Effect> {
        if self.uninstalling.as_ref() != Some(&id) {
            return vec![];
        }
        self.uninstalling = None;
        self.dirty = true;
        if let Err(error) = result {
            eprintln!("Luma uninstall: {error}");
            self.notify(self.preferences.language.t("deleteAppError"));
            if matches!(&self.overlay, Some(Overlay::Delete { id: current, .. } | Overlay::Options { id: current, .. }) if current == &id)
            {
                if let Some(Overlay::Delete { restore, .. } | Overlay::Options { restore, .. }) =
                    self.overlay.take()
                {
                    self.focus = restore;
                }
                self.panel_started = None;
                self.pressed = None;
                self.reconcile_focus();
            }
            // Even an uncertain/failed mutation may have changed TV inventory.
            // Refresh is read-only, and also clears a stale card after late removal.
            return vec![Effect::Refresh];
        }
        let old_ids: Vec<_> = self.apps().iter().map(|app| app.id.clone()).collect();
        let index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
        let mut favorites = self.preferences.favorites(&self.inventory.apps);
        favorites.retain(|item| item != &id);
        self.preferences.favorites = Some(favorites);
        self.preferences.hidden.retain(|item| item != &id);
        self.inventory.apps.retain(|app| app.id != id);
        if self.filter == 5 && self.hidden_count() == 0 {
            self.filter = 0;
        }
        if matches!(&self.overlay, Some(Overlay::Delete { id: current, .. } | Overlay::Options { id: current, .. }) if current == &id)
        {
            self.overlay = None;
            self.panel_started = None;
            self.focus = Target::App(id.clone());
        }
        if self.focus == Target::App(id) {
            self.focus = self
                .apps()
                .get(index.min(self.apps().len().saturating_sub(1)))
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or_else(|| self.first_content());
        }
        self.reconcile_focus();
        self.notify(self.preferences.language.t("appDeleted"));
        vec![Effect::Save, Effect::Refresh]
    }

    /// Holding OK opens the focused item's options; a Home app's include Move.
    pub fn hold(&mut self) {
        self.options();
    }

    pub fn text(&mut self, text: &str) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Coin {
                let query = format!("{}{text}", self.crypto.search.query);
                self.crypto.search.edit(&query);
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(120usize.saturating_sub(query.chars().count())),
                );
                self.weather.edit(query);
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Name {
                let name = &mut self.greeting_name_draft;
                name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(40usize.saturating_sub(name.chars().count())),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::InputName {
                let length = self.input_name.chars().count();
                self.input_name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(INPUT_NAME_LIMIT.saturating_sub(length)),
                );
                self.dirty = true;
                return;
            }
            self.query.extend(
                text.chars()
                    .filter(|c| !c.is_control())
                    .take(120usize.saturating_sub(self.query.chars().count())),
            );
            self.scroll = 0.;
            self.dirty = true;
        }
    }

    pub fn delete(&mut self) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Coin {
                self.crypto.search.query.pop();
            } else if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.pop();
                self.weather.edit(query);
            } else if self.keyboard_field == TextField::Name {
                self.greeting_name_draft.pop();
            } else if self.keyboard_field == TextField::InputName {
                self.input_name.pop();
            } else {
                self.query.pop();
            }
            self.dirty = true;
        }
    }

    pub fn text_input_active(&self) -> bool {
        matches!(
            self.overlay,
            Some(Overlay::Keyboard | Overlay::SystemKeyboard)
        )
    }

    pub fn finish_text_input(&mut self) -> Vec<Effect> {
        if self.text_input_active() {
            self.overlay = None;
            self.panel_started = None;
            self.focus = self.keyboard_return();
            self.pointer = false;
            self.dirty = true;
            return self.commit_text_input();
        }
        vec![]
    }

    fn commit_text_input(&mut self) -> Vec<Effect> {
        if self.keyboard_field == TextField::Name {
            let name = std::mem::take(&mut self.greeting_name_draft);
            let name = name.trim();
            // Empty dismissal preserves the saved value; Clear name is explicit.
            if !name.is_empty() && self.preferences.greeting_name != name {
                self.preferences.greeting_name = name.into();
                return vec![Effect::Save];
            }
            return vec![];
        }
        self.commit_input_name()
    }

    /// Closing the keyboard by any route keeps a typed input name. An empty
    /// field leaves the saved name unchanged; Clear name removes it explicitly.
    fn commit_input_name(&mut self) -> Vec<Effect> {
        let name = std::mem::take(&mut self.input_name);
        if self.keyboard_field != TextField::InputName || clean_input_name(&name).is_empty() {
            return vec![];
        }
        let Some(Subpage::Input(id)) = self.subpage.clone() else {
            return vec![];
        };
        if self.preferences.set_input_name(&id, &name) {
            vec![Effect::Save]
        } else {
            vec![]
        }
    }

    /// Applies the tile list edited on a phone or computer. New tiles join
    /// the end of the Home favorites and removed ones leave favorites and hidden apps.
    pub fn replace_tiles(&mut self, tiles: Vec<CustomTile>) -> Vec<Effect> {
        let removed: Vec<_> = self
            .preferences
            .custom_tiles
            .iter()
            .filter(|old| !tiles.iter().any(|tile| tile.id == old.id))
            .map(|old| old.id.clone())
            .collect();
        for id in &removed {
            self.preferences.remove_tile(id);
        }
        let added: Vec<_> = tiles
            .iter()
            .filter(|tile| self.preferences.custom_tile(&tile.id).is_none())
            .map(|tile| tile.id.clone())
            .collect();
        self.preferences.custom_tiles = tiles;
        self.preferences.sanitize();
        self.sync_custom_tiles();
        if let Target::Tile(id) | Target::TileRemove(id) | Target::TileTest(id) = &self.focus
            && self.preferences.custom_tile(id).is_none()
        {
            if matches!(
                &self.overlay,
                Some(Overlay::Options {
                    restore: Target::Tile(_),
                    ..
                })
            ) {
                self.overlay = None;
                self.panel_started = None;
            }
            self.focus = Target::TileEditor;
        }
        if !added.is_empty() {
            let mut favorites = self.preferences.favorites(&self.inventory.apps);
            for id in added {
                if self.preferences.custom_tile(&id).is_some() && !favorites.contains(&id) {
                    favorites.push(id);
                }
            }
            self.preferences.favorites = Some(favorites);
        }
        vec![Effect::Save]
    }

    fn send_tile(&mut self, tile: CustomTile) -> Vec<Effect> {
        self.notify(
            self.preferences
                .language
                .format("tileSending", &[("tile", &tile.title)]),
        );
        vec![Effect::FireWebhook(tile)]
    }

    pub fn delete_surrounding_text(&mut self, offset: i32, length: i32) {
        if !self.text_input_active() || length <= 0 {
            return;
        }
        // SDL-webOS forwards the native text model's UTF-8 byte offsets,
        // relative to the insertion cursor (at the end of our query).
        let mut query = self.editing_text().to_owned();
        let mut start =
            (query.len() as i64 + i64::from(offset)).clamp(0, query.len() as i64) as usize;
        let mut end = start.saturating_add(length as usize).min(query.len());
        while !query.is_char_boundary(start) {
            start -= 1;
        }
        while !query.is_char_boundary(end) {
            end += 1;
        }
        query.replace_range(start..end, "");
        if self.keyboard_field == TextField::Coin {
            self.crypto.search.edit(&query);
        } else if self.keyboard_field == TextField::Location {
            self.weather.edit(query);
        } else if self.keyboard_field == TextField::Name {
            self.greeting_name_draft = query;
        } else if self.keyboard_field == TextField::InputName {
            self.input_name = query;
        } else {
            self.query = query;
            self.scroll = 0.;
        }
        self.dirty = true;
    }

    /// The coins Settings › Crypto lists: `crypto::COINS`, then the chosen coins found by search.
    pub fn crypto_choices(&self) -> Vec<String> {
        let mut choices: Vec<String> = crate::crypto::COINS
            .iter()
            .map(|(symbol, _)| (*symbol).to_owned())
            .collect();
        for coin in &self.preferences.crypto_coins {
            if !choices.contains(coin) {
                choices.push(coin.clone());
            }
        }
        choices
    }
    /// Adds or removes a coin; Home shows at most `crypto::MAX_COINS`.
    fn toggle_coin(&mut self, symbol: String) -> Vec<Effect> {
        let coins = &mut self.preferences.crypto_coins;
        if let Some(position) = coins.iter().position(|coin| *coin == symbol) {
            coins.remove(position);
        } else if coins.len() >= crate::crypto::MAX_COINS {
            self.notify(self.preferences.language.t("cryptoLimit"));
            return vec![];
        } else {
            coins.push(symbol);
            // Cards follow the Settings order, whatever order coins were picked in.
            crate::crypto::order(coins);
        }
        vec![Effect::Save]
    }
    pub fn editing_text(&self) -> &str {
        match self.keyboard_field {
            TextField::Location => &self.weather.query,
            TextField::Name => &self.greeting_name_draft,
            TextField::InputName => &self.input_name,
            TextField::Apps => &self.query,
            TextField::Coin => &self.crypto.search.query,
        }
    }
    pub fn open_keyboard(&mut self, field: TextField) {
        self.keyboard_field = field;
        if field == TextField::Name {
            // SDL-webOS cannot pre-fill the system keyboard's text model.
            self.greeting_name_draft.clear();
        }
        if field == TextField::InputName {
            // The TV keyboard starts empty, so a new name replaces the old one.
            self.input_name.clear();
        }
        if field == TextField::Coin {
            self.crypto.search.query.clear();
            let search = &mut self.crypto.search;
            search.wanted = search.catalog.is_empty() && !search.loading;
        }
        if cfg!(all(
            feature = "webos",
            target_os = "linux",
            target_arch = "arm"
        )) {
            self.overlay = Some(Overlay::SystemKeyboard);
            self.focus = match field {
                TextField::Location => Target::WeatherQuery,
                TextField::Name => Target::GreetingName,
                TextField::InputName => Target::InputRename,
                TextField::Apps => Target::Search,
                TextField::Coin => Target::CryptoSearch,
            };
            self.panel_started = None;
            self.dirty = true;
            return;
        }
        self.overlay = Some(Overlay::Keyboard);
        self.focus = Target::Key(0);
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn open_location(&mut self) {
        self.weather.edit(String::new());
        self.subpage = Some(Subpage::WeatherLocation);
        self.focus = Target::WeatherQuery;
        self.scroll = 0.;
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn keyboard_return(&self) -> Target {
        match self.keyboard_field {
            TextField::Location => Target::WeatherSearch,
            TextField::Name => Target::GreetingName,
            TextField::InputName => Target::InputRename,
            TextField::Apps => Target::Search,
            TextField::Coin => Target::CryptoSearch,
        }
    }

    pub fn activate(&mut self) -> Vec<Effect> {
        if matches!(self.overlay, Some(Overlay::Menu { .. })) {
            return self.menu_activate();
        }
        if matches!(self.overlay, Some(Overlay::Today { .. })) {
            return self.today_activate();
        }
        if self.recents_open() {
            use crate::recents::Command;
            return match self.focus.clone() {
                Target::Recent(id) => self.recent_request(Command::Launch(id)),
                Target::RecentRetry => self.recent_request(Command::List),
                Target::RecentCloseAll => self.close_all_recents(),
                _ => vec![],
            };
        }
        if self.moving.take().is_some() {
            // Order was saved at every move. A fresh OK release finishes the
            // mode and keeps the selected favorite, rather than launching it.
            self.dirty = true;
            return vec![];
        }
        self.dirty = true;
        if self.page != Page::Apps
            && let Target::App(id) = &self.focus
            && let Some(tile) = self.preferences.custom_tile(id).cloned()
        {
            return self.send_tile(tile);
        }
        match self.focus.clone() {
            Target::Nav(index) => self.open(index),
            Target::TodayCard => self.open_today(),
            Target::Browse => self.open(1),
            Target::Retry => return vec![Effect::Reload],
            Target::DismissNotice => self.notice = None,
            Target::CloseDialog => return self.back(),
            Target::UploadWallpaper => {
                self.wallpaper_upload = None;
                self.overlay = Some(Overlay::WallpaperUpload);
                self.focus = Target::CloseDialog;
                self.pressed = None;
            }
            Target::TileEditor => {
                self.tile_editor = None;
                self.overlay = Some(Overlay::TileEditor);
                self.focus = Target::CloseDialog;
                self.pressed = None;
            }
            Target::TvSettings => return vec![Effect::Launch("com.palm.app.settings".into())],
            Target::App(_) if self.page == Page::Apps && !self.launching => self.options(),
            Target::App(id) | Target::Input(id) if !self.launching => {
                return vec![Effect::Launch(id)];
            }
            Target::MoveApp => {
                if self.page == Page::Home
                    && let Some(Overlay::Options { id, restore }) = self.overlay.take()
                {
                    self.panel_started = None;
                    self.focus = restore;
                    self.moving = Some(id);
                    self.notice = None;
                }
            }
            Target::OpenApp if !self.launching => {
                if let Some(Overlay::Options { id, .. }) = &self.overlay
                    && self.inventory.apps.iter().any(|app| &app.id == id)
                {
                    let id = id.clone();
                    self.back();
                    if let Some(tile) = self.preferences.custom_tile(&id).cloned() {
                        return self.send_tile(tile);
                    }
                    return vec![Effect::Launch(id)];
                }
            }
            Target::Filter(index) => {
                self.filter = if self.filter == index { 0 } else { index };
                self.scroll = 0.;
            }
            Target::ClearFilter => {
                self.filter = 0;
                self.focus = Target::Filter(0);
                self.scroll = 0.;
            }
            Target::ClearSearch => {
                self.query.clear();
                self.focus = Target::Search;
                self.scroll = 0.;
            }
            Target::Search => {
                self.open_keyboard(TextField::Apps);
            }
            Target::Cs2Toggle => {
                self.preferences.show_cs2 = !self.preferences.show_cs2;
                return vec![Effect::Save];
            }
            Target::Cs2Major => {
                self.preferences.cs2_major_only = !self.preferences.cs2_major_only;
                return vec![Effect::Save];
            }
            Target::Cs2Results => {
                self.preferences.cs2_results = !self.preferences.cs2_results;
                return vec![Effect::Save];
            }
            Target::Cs2Match(index) => {
                let id = self.cs2.feed.as_ref().and_then(|feed| {
                    crate::cs2::shown(
                        feed,
                        self.preferences.cs2_major_only,
                        self.preferences.cs2_results,
                    )
                    .get(index)
                    .map(|m| m.id.clone())
                });
                if let Some(id) = id {
                    self.overlay = Some(Overlay::Cs2Match {
                        id,
                        restore: Target::Cs2Match(index),
                    });
                    self.focus = Target::Cs2Close;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                }
            }
            Target::TodayMatch(id) => {
                if self.cs2.find(&id).is_some() {
                    self.overlay = Some(Overlay::Cs2Match {
                        id: id.clone(),
                        restore: Target::TodayMatch(id),
                    });
                    self.focus = Target::Cs2Close;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                }
            }
            Target::TodayShow(id) => return self.open_lampa(&format!("tv_{id}")),
            Target::TodayLampa(card) => return self.open_lampa(&card),
            Target::LampaAccount => {
                if self.cub.signed_in() {
                    self.cub.sign_out();
                    self.cub_forget = true;
                } else {
                    self.cub.begin();
                    self.overlay = Some(Overlay::CubSignIn);
                    self.focus = Target::CloseDialog;
                    self.pressed = None;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                }
            }
            Target::LampaPlugin => {
                self.overlay = Some(Overlay::LampaPlugin);
                self.focus = Target::CloseDialog;
                self.pressed = None;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::LampaFeed => {
                use crate::model::LampaFeed;
                self.preferences.lampa_feed = match self.preferences.lampa_feed {
                    LampaFeed::Watching => LampaFeed::Continue,
                    LampaFeed::Continue => LampaFeed::Watching,
                };
                self.shows_first = 0;
                return vec![Effect::Save];
            }
            Target::Cs2Close => return self.back(),
            Target::ShowsToggle => {
                self.preferences.show_shows = !self.preferences.show_shows;
                return vec![Effect::Save];
            }
            Target::Show(index) => {
                if let Some(card) = self.lampa_cards().into_iter().nth(index) {
                    return self.open_lampa(&card);
                }
            }
            Target::WeatherConfigure => {
                self.open(2);
                self.section = WIDGETS_SECTION;
                self.open_location();
                self.open_keyboard(TextField::Location);
            }
            Target::WeatherToggle => {
                self.preferences.show_weather = !self.preferences.show_weather;
                self.weather.invalidate_forecast();
                return vec![Effect::Save];
            }
            Target::WeatherLocation => self.open_location(),
            Target::WeatherTemperature => {
                self.subpage = Some(Subpage::Temperature);
                self.focus = Target::Choice(usize::from(
                    self.preferences.temperature_unit == Unit::Fahrenheit,
                ));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::WeatherQuery => self.open_keyboard(TextField::Location),
            Target::GreetingToggle => {
                self.preferences.show_greeting = !self.preferences.show_greeting;
                return vec![Effect::Save];
            }
            Target::DateToggle => {
                self.preferences.show_date = !self.preferences.show_date;
                return vec![Effect::Save];
            }
            Target::DateFormat => {
                self.subpage = Some(Subpage::DateFormat);
                self.focus = Target::Choice(self.preferences.date_format.index());
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::GreetingName => self.open_keyboard(TextField::Name),
            Target::GreetingNameClear => {
                self.preferences.greeting_name.clear();
                self.focus = Target::GreetingName;
                return vec![Effect::Save];
            }
            Target::AppNamesHome => {
                self.preferences.show_home_names = !self.preferences.show_home_names;
                return vec![Effect::Save];
            }
            Target::AppNamesApps => {
                self.preferences.app_names = self.preferences.app_names.next();
                return vec![Effect::Save];
            }
            Target::ClockFormat | Target::ClockTimeFormat => {
                let format = self.focus == Target::ClockFormat;
                self.subpage = Some(if format {
                    Subpage::ClockFormat
                } else {
                    Subpage::TimeFormat
                });
                self.focus = Target::Choice(usize::from(if format {
                    self.preferences.hour_24
                } else {
                    self.preferences.show_seconds
                }));
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InputRename => self.open_keyboard(TextField::InputName),
            Target::CryptoToggle => {
                self.preferences.show_crypto = !self.preferences.show_crypto;
                return vec![Effect::Save];
            }
            Target::Cs2Layout | Target::ShowsLayout | Target::CryptoLayout => {
                use crate::model::Center;
                let widget = match self.focus {
                    Target::Cs2Layout => Center::Cs2,
                    Target::ShowsLayout => Center::Shows,
                    _ => Center::Crypto,
                };
                let center = self.preferences.center() != Some(widget);
                self.preferences.place_in_center(widget, center);
                return vec![Effect::Save];
            }
            Target::CryptoSearch => self.open_keyboard(TextField::Coin),
            Target::CryptoResult(index) => {
                let Some(symbol) = self
                    .crypto
                    .search
                    .matches()
                    .get(index)
                    .map(|symbol| symbol.to_string())
                else {
                    return vec![];
                };
                return self.toggle_coin(symbol);
            }
            Target::CryptoCoin(index) => {
                let Some(symbol) = self.crypto_choices().get(index).cloned() else {
                    return vec![];
                };
                return self.toggle_coin(symbol);
            }
            Target::Coin(_) => {
                self.open(2);
                self.section = WIDGETS_SECTION;
                self.subpage = Some(Subpage::Crypto);
                self.focus = Target::CryptoToggle;
                self.scroll = 0.;
            }
            Target::Widget(index) => {
                let (subpage, focus) = match index {
                    0 => (Subpage::Greeting, Target::GreetingToggle),
                    1 => (Subpage::Weather, Target::WeatherToggle),
                    6 => (Subpage::Cs2, Target::Cs2Toggle),
                    7 => (Subpage::Shows, Target::ShowsToggle),
                    2 => (Subpage::Shortcuts, Target::TileEditor),
                    4 => (Subpage::Date, Target::DateToggle),
                    5 => (Subpage::Crypto, Target::CryptoToggle),
                    3 => (
                        Subpage::Inputs,
                        self.inventory
                            .inputs
                            .first()
                            .map(|input| Target::InputMode(input.id.clone()))
                            .unwrap_or(Target::Setting(3)),
                    ),
                    _ => return vec![],
                };
                self.section = WIDGETS_SECTION;
                self.subpage = Some(subpage);
                self.focus = focus;
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InstallUpdate => {
                if !self.update_busy()
                    && let Some(version) = &self.available_update
                {
                    let version = version.clone();
                    self.update = crate::update::InstallStatus::Downloading(0);
                    return vec![Effect::UpdateDownload(version)];
                }
            }
            Target::CheckUpdates => {
                if !self.update_busy() {
                    self.update = crate::update::InstallStatus::Idle;
                    self.update_status = crate::update::Status::Checking;
                    self.update_check_requested = true;
                    self.dirty = true;
                }
            }
            Target::Tile(_) => self.options(),
            Target::TileTest(id) => {
                if matches!(&self.overlay, Some(Overlay::Options { id: current, restore: Target::Tile(_), .. }) if current == &id)
                    && let Some(tile) = self.preferences.custom_tile(&id).cloned()
                {
                    return self.send_tile(tile);
                }
            }
            Target::TileRemove(id) => {
                if matches!(&self.overlay, Some(Overlay::Options { id: current, restore: Target::Tile(_), .. }) if current == &id)
                    && let Some(tile) = self.preferences.remove_tile(&id)
                {
                    self.overlay = None;
                    self.notify(
                        self.preferences
                            .language
                            .format("tileRemoved", &[("tile", &tile.title)]),
                    );
                    self.subpage = Some(Subpage::Shortcuts);
                    self.panel_started = None;
                    self.scroll = 0.;
                    self.sync_custom_tiles();
                    self.focus = Target::TileEditor;
                    return vec![Effect::Save];
                }
            }
            Target::InputNameReset => {
                if let Some(Subpage::Input(id)) = &self.subpage
                    && self.preferences.input_names.remove(id).is_some()
                {
                    self.focus = Target::InputRename;
                    return vec![Effect::Save];
                }
            }
            Target::WeatherSearch => {
                return self
                    .weather
                    .search(self.preferences.language)
                    .map(|command| vec![Effect::Weather(command)])
                    .unwrap_or_default();
            }
            Target::WeatherClear => {
                self.preferences.weather_location = None;
                self.weather.remove_location();
                self.focus = Target::WeatherSearch;
                return vec![Effect::Save, Effect::Weather(weather::Command::ClearCache)];
            }
            Target::WeatherResult(index) => {
                if let Some(place) = self.weather.results.get(index).cloned() {
                    self.preferences.weather_location = Some(place);
                    self.weather.cancel_rename();
                    self.weather.invalidate_forecast();
                    self.weather.cancel_search();
                    self.focus = Target::WeatherSearch;
                    return vec![Effect::Save];
                }
            }
            Target::Sidebar(index) => {
                self.section = index;
                self.scroll = 0.;
                self.subpage = None;
                self.focus = self.section_entry();
            }
            Target::Setting(index) => {
                match index {
                    0 => {
                        self.subpage = Some(Subpage::AppNames);
                        self.focus = Target::AppNamesHome;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    1 => {
                        self.subpage = Some(Subpage::Clock);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        self.focus = Target::ClockFormat;
                        return vec![];
                    }
                    2 => self.preferences.reduced_motion = !self.preferences.reduced_motion,
                    FOCUS_GLOW_ROW => {
                        self.preferences.focus_glow = self.preferences.focus_glow.next()
                    }
                    4 => {
                        self.subpage = Some(Subpage::Language);
                        self.focus = Target::Choice(self.preferences.language.index());
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    HOME_TAKEOVER_ROW => {
                        self.subpage = Some(Subpage::HomeTakeover);
                        self.focus = Target::Choice(if self.home_takeover { 0 } else { 1 });
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    5 => return vec![Effect::Quit],
                    HOMEBREW_UPDATES_ROW => {
                        self.preferences.homebrew_updates = !self.preferences.homebrew_updates
                    }
                    7 => {
                        self.subpage = Some(Subpage::TextRendering);
                        self.focus = Target::Choice(self.preferences.text_rendering.index());
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    INTERFACE_SCALE_ROW => {
                        self.subpage = Some(Subpage::Appearance);
                        self.focus = Target::Choice(self.preferences.ui_scale.index());
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    _ => return vec![Effect::Reload],
                }
                return vec![Effect::Save];
            }
            Target::Background(_) => {
                self.options();
            }
            Target::WallpaperCycle => {
                self.preferences.wallpaper_cycle = !self.preferences.wallpaper_cycle;
                self.preferences.wallpaper_changed = unix_now();
                return vec![Effect::Save];
            }
            Target::WallpaperGradient => {
                self.preferences.wallpaper_gradient = !self.preferences.wallpaper_gradient;
                return vec![Effect::Save];
            }
            Target::UseBackground => {
                let Some(Overlay::BackgroundOptions { id, .. }) = self.overlay.clone() else {
                    return vec![];
                };
                if self.deleting_wallpaper.as_deref() == Some(&id) {
                    return vec![];
                }
                if !self.background_ids().any(|known| known == id) {
                    return self.back();
                }
                if !self.preferences.reduced_motion && self.preferences.background != id {
                    self.fade_wallpaper_from(self.background_key(), BACKGROUND_FADE);
                }
                // The TV's splash can only be a packaged picture.
                self.preferences.splash_background = Some(
                    if crate::wallpapers::valid_id(&id) || id == crate::aerials::ID {
                        self.preferences.splash_background().to_owned()
                    } else {
                        id.clone()
                    },
                );
                if id == crate::aerials::ID {
                    self.preferences.aerials.changed = unix_now();
                    self.aerials_previous = None;
                    self.aerials_request = true;
                }
                self.preferences.background = id;
                self.preferences.wallpaper_changed = unix_now();
                self.back();
                return vec![Effect::Save];
            }
            Target::NextAerial => {
                if !matches!(&self.overlay, Some(Overlay::BackgroundOptions { id, .. }) if id == crate::aerials::ID)
                    || self.preferences.background != crate::aerials::ID
                {
                    return vec![];
                }
                // Due at once: advance_aerials moves on as soon as the next still is on the TV.
                self.preferences.aerials.changed = 0;
                self.aerials_request = true;
                self.back();
                return vec![Effect::Save];
            }
            Target::DeleteBackground => {
                if let Some(Overlay::BackgroundOptions { id, restore }) = self.overlay.clone()
                    && self.can_delete_background(&id)
                    && self.deleting_wallpaper.is_none()
                {
                    self.overlay = Some(Overlay::BackgroundDelete { id, restore });
                    self.focus = Target::CancelDelete;
                    self.pointer = false;
                    self.pressed = None;
                    self.panel_started = None;
                }
            }
            Target::InputMode(id) => {
                let mode = self
                    .preferences
                    .input_modes
                    .get(&id)
                    .copied()
                    .unwrap_or_default();
                self.subpage = Some(Subpage::Input(id));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.focus = Target::Choice(match mode {
                    InputMode::On => 0,
                    InputMode::Off => 1,
                    InputMode::Auto => 2,
                });
            }
            Target::Choice(index) => {
                match self.subpage.clone() {
                    Some(Subpage::ClockFormat) => self.preferences.hour_24 = index == 1,
                    Some(Subpage::TimeFormat) => self.preferences.show_seconds = index == 1,
                    Some(Subpage::DateFormat) => {
                        self.preferences.date_format = crate::model::DateFormat::ALL[index.min(3)];
                    }
                    Some(Subpage::Input(id)) => {
                        self.preferences.input_modes.insert(
                            id,
                            [InputMode::On, InputMode::Off, InputMode::Auto][index.min(2)],
                        );
                    }
                    Some(Subpage::HomeTakeover) => {
                        let enable = index == 0;
                        self.close_subpage();
                        return if enable == self.home_takeover {
                            vec![]
                        } else {
                            vec![Effect::HomeTakeover(enable)]
                        };
                    }
                    Some(Subpage::Temperature) => {
                        self.preferences.temperature_unit = if index == 1 {
                            Unit::Fahrenheit
                        } else {
                            Unit::Celsius
                        }
                    }
                    // The scale stays open, so its preview and the screen show the choice.
                    Some(Subpage::Appearance) => {
                        self.preferences.ui_scale = UiSize::ALL[index.min(UiSize::ALL.len() - 1)];
                        return vec![Effect::Save];
                    }
                    Some(Subpage::Language) => {
                        let language = Language::ALL[index.min(Language::ALL.len() - 1)];
                        let previous = self.preferences.language;
                        self.preferences.language = language;
                        self.weather.cancel_search();
                        // Place names come from the search in the language it ran in. With
                        // the weather off, Luma contacts no weather service.
                        if language != previous
                            && self.preferences.show_weather
                            && let Some(location) = &self.preferences.weather_location
                        {
                            let command = self.weather.rename(location, previous, language);
                            self.close_subpage();
                            return vec![Effect::Save, Effect::Weather(command)];
                        }
                    }
                    Some(Subpage::TextRendering) => {
                        let mode = TextRendering::ALL[index.min(TextRendering::ALL.len() - 1)];
                        if mode != self.preferences.text_rendering {
                            self.preferences.text_rendering = mode;
                            self.notify(self.preferences.language.t("textRenderingRestart"));
                        }
                    }
                    Some(
                        Subpage::WeatherLocation
                        | Subpage::Greeting
                        | Subpage::Date
                        | Subpage::AppNames
                        | Subpage::Clock
                        | Subpage::Weather
                        | Subpage::Shortcuts
                        | Subpage::Inputs
                        | Subpage::Crypto,
                    ) => {
                        return vec![];
                    }
                    Some(Subpage::Cs2 | Subpage::Shows) => return vec![],
                    None => return vec![],
                }
                self.close_subpage();
                return vec![Effect::Save];
            }
            Target::Option(index) => {
                if index == 2 {
                    if let Some(Overlay::Options { id, restore }) = &self.overlay
                        && crate::platform::can_uninstall(id)
                        && self.uninstalling.is_none()
                        && self.inventory.apps.iter().any(|app| &app.id == id)
                    {
                        self.overlay = Some(Overlay::Delete {
                            id: id.clone(),
                            restore: restore.clone(),
                        });
                        self.focus = Target::CancelDelete;
                        self.pointer = false;
                        self.pressed = None;
                        self.panel_started = None;
                    }
                    return vec![];
                }
                let Some(Overlay::Options { id, restore }) = self.overlay.take() else {
                    return vec![];
                };
                self.panel_started = None;
                let old_ids: Vec<_> = self.apps().iter().map(|a| a.id.clone()).collect();
                let old_index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
                let mut favorites = self.preferences.favorites(&self.inventory.apps);
                let title = self
                    .inventory
                    .apps
                    .iter()
                    .find(|app| app.id == id)
                    .map_or(id.clone(), |app| app.title.clone());
                if index == 0 {
                    if favorites.contains(&id) {
                        favorites.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteRemoved", &[("app", &title)]),
                        );
                    } else {
                        favorites.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteAdded", &[("app", &title)]),
                        );
                    }
                    self.preferences.hidden.retain(|item| item != &id);
                    self.preferences.favorites = Some(favorites);
                } else if index == 1 {
                    if self.preferences.hidden.contains(&id) {
                        self.preferences.hidden.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("appRestored", &[("app", &title)]),
                        );
                    } else {
                        self.preferences.hidden.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("appHidden", &[("app", &title)]),
                        );
                    }
                }
                if self.filter == 5 && self.hidden_count() == 0 {
                    self.filter = 0;
                }
                self.focus = restore;
                self.pointer = false;
                if let Target::App(id) = &self.focus {
                    let apps = self.apps();
                    if !apps.iter().any(|app| &app.id == id) {
                        self.focus = apps
                            .get(old_index.min(apps.len().saturating_sub(1)))
                            .map(|app| Target::App(app.id.clone()))
                            .unwrap_or_else(|| self.first_content());
                    }
                }
                return if index < 2 {
                    vec![Effect::Save]
                } else {
                    vec![]
                };
            }
            Target::CancelDelete => return self.back(),
            Target::ConfirmDelete => {
                if let Some(Overlay::BackgroundDelete { id, .. }) = self.overlay.clone() {
                    if self.can_delete_background(&id) && self.deleting_wallpaper.is_none() {
                        if crate::wallpapers::valid_id(&id) {
                            self.deleting_wallpaper = Some(id.clone());
                            self.focus = Target::CancelDelete;
                            return vec![Effect::DeleteWallpaper(id)];
                        }
                        // Packaged assets remain intact; persist removal from the gallery.
                        self.finish_background_delete(&id);
                        return vec![Effect::Save];
                    }
                    return vec![];
                }
                if let Some(Overlay::Delete { id, .. }) = &self.overlay
                    && self.uninstalling.is_none()
                    && crate::platform::can_uninstall(id)
                    && self.inventory.apps.iter().any(|app| &app.id == id)
                {
                    self.uninstalling = Some(id.clone());
                    self.focus = Target::CancelDelete;
                    return vec![Effect::Uninstall(id.clone())];
                }
            }
            Target::Key(index) => match KEYS[index] {
                "Space" => self.text(" "),
                "Delete" => self.delete(),
                "Clear" => match self.keyboard_field {
                    TextField::Location => self.weather.edit(String::new()),
                    TextField::Name => self.greeting_name_draft.clear(),
                    TextField::InputName => self.input_name.clear(),
                    TextField::Apps => self.query.clear(),
                    TextField::Coin => self.crypto.search.query.clear(),
                },
                "Done" => {
                    self.overlay = None;
                    self.panel_started = None;
                    self.focus = self.keyboard_return();
                    return self.commit_text_input();
                }
                key => self.text(key),
            },
            _ => {}
        }
        vec![]
    }

    pub fn back(&mut self) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if matches!(self.overlay, Some(Overlay::Menu { .. })) {
            self.close_menu();
            return vec![];
        }
        if self.moving.take().is_some() {
            return vec![];
        }
        if let Some(overlay) = self.overlay.take() {
            self.panel_started = None;
            if let Overlay::BackgroundDelete { id, restore } = overlay {
                self.overlay = Some(Overlay::BackgroundOptions { id, restore });
                self.focus = Target::DeleteBackground;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::Cs2Match { restore, .. } = overlay {
                // A match opened from the Today card returns to the card, still open.
                if restore.today_line() && self.page == Page::Home {
                    self.overlay = Some(Overlay::Today {
                        restore: Target::TodayCard,
                    });
                }
                self.focus = restore;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::Today { restore } = overlay {
                self.today_motion = None;
                self.focus = restore;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::BackgroundOptions { id, .. } = overlay {
                self.focus = self.background_target(&id);
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::WallpaperUpload) {
                self.wallpaper_upload = None;
                self.focus = Target::UploadWallpaper;
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::TileEditor) {
                self.tile_editor = None;
                self.focus = Target::TileEditor;
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::LampaPlugin) {
                self.focus = Target::LampaPlugin;
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::CubSignIn) {
                self.cub.cancel();
                self.focus = Target::LampaAccount;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::Delete { id, restore } = overlay {
                self.overlay = Some(Overlay::Options { id, restore });
                self.focus = Target::Option(2);
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::Recents { .. }) {
                self.recents.closing_focus = Some(self.focus.clone());
                self.recents.updated = Instant::now();
                self.pressed = None;
                if self.preferences.reduced_motion {
                    self.recents.visibility = 0.;
                    self.recents.closing_focus = None;
                }
            }
            if let Overlay::Options { restore, .. } | Overlay::Recents { restore } = overlay {
                self.focus = restore;
                self.recents.pending = None;
                self.reconcile_focus();
                return vec![];
            }
            self.focus = self.keyboard_return();
            return self.commit_text_input();
        }
        if self.page == Page::Settings && self.subpage.is_some() {
            self.close_subpage();
            return vec![];
        }
        if self.page == Page::Settings && !matches!(self.focus, Target::Sidebar(_)) {
            self.focus = Target::Sidebar(self.section);
            self.scroll = 0.;
            return vec![];
        }
        if self.page == Page::Apps && matches!(self.focus, Target::Search | Target::ClearSearch) {
            self.query.clear();
            self.focus = self.first_content();
            self.scroll = 0.;
            return vec![];
        }
        if self.page != Page::Home {
            self.return_home()
        } else {
            self.open_recents()
        }
    }
}
