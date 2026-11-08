use crate::i18n::Language;
use crate::weather::{Location, Unit};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// The Apps page's card names: as on Apple TV, only the focused card's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppNames {
    #[default]
    Focus,
    Always,
    Off,
}
impl AppNames {
    /// The next choice, for the setting's OK.
    pub fn next(self) -> Self {
        match self {
            Self::Focus => Self::Always,
            Self::Always => Self::Off,
            Self::Off => Self::Focus,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Focus => "namesFocus",
            Self::Always => "namesAlways",
            Self::Off => "stateOff",
        }
    }
}

/// Reads the names mode, or the former `show_app_names` switch: on was the default, so it becomes
/// names on focus; off stays off. Anything malformed is the default.
fn deserialize_app_names<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<AppNames, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Bool(false) => AppNames::Off,
        value @ serde_json::Value::String(_) => serde_json::from_value(value).unwrap_or_default(),
        _ => AppNames::Focus,
    })
}

/// The focused card's glow on Home, chosen in Settings › General.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusGlow {
    #[default]
    White,
    Blue,
    Violet,
    Green,
    Amber,
    Off,
}
impl FocusGlow {
    pub const ALL: [Self; 6] = [
        Self::White,
        Self::Blue,
        Self::Violet,
        Self::Green,
        Self::Amber,
        Self::Off,
    ];
    /// The next choice, for the setting's OK.
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|glow| *glow == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
    /// The glow's colour; Off has none.
    pub fn rgb(self) -> Option<u32> {
        match self {
            Self::White => Some(0xe8eef8),
            Self::Blue => Some(0x6fa8ff),
            Self::Violet => Some(0xa98bff),
            Self::Green => Some(0x6fe0a8),
            Self::Amber => Some(0xffc46b),
            Self::Off => None,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::White => "glowWhite",
            Self::Blue => "glowBlue",
            Self::Violet => "glowViolet",
            Self::Green => "glowGreen",
            Self::Amber => "glowAmber",
            Self::Off => "stateOff",
        }
    }
}

pub const CATEGORIES: [&str; 4] = ["Streaming", "Music", "Gaming", "Utilities"];
pub const BACKGROUNDS: [(&str, &str); 9] = [
    ("slate", "Slate"),
    ("midnight", "Midnight"),
    ("prism", "Prism"),
    ("dunes", "Monument Valley"),
    ("san-francisco", "San Francisco"),
    ("summit", "Summit"),
    ("forest", "Forest"),
    ("yosemite", "Yosemite"),
    ("northern-lights", "Northern lights"),
];

// Generic fallbacks use delimited words, so "game" does not match "endgame".
// Non-ASCII bytes count as boundaries for localized titles. Curated vector
// branding is selected separately by exact ID in branding.rs.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    haystack.match_indices(needle).any(|(start, matched)| {
        let end = start + matched.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        before_ok && after_ok
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct App {
    pub id: String,
    pub title: String,
    pub brand: String,
    pub category: String,
    #[serde(skip)]
    pub icon: Option<crate::app_artwork::Source>,
    /// The interface icon a shortcut tile shows; empty for real apps.
    #[serde(skip)]
    pub mark: &'static str,
}

impl App {
    pub fn new(id: &str, title: &str) -> Self {
        if id.starts_with(CUSTOM_TILE_PREFIX) {
            return Self {
                id: id.into(),
                title: title.into(),
                brand: "custom".into(),
                category: "Utilities".into(),
                icon: None,
                mark: TILE_ICONS[0].0,
            };
        }
        let identity = format!("{id} {title}").to_lowercase();
        let generic = [
            ("kodi", "kodi", "Streaming"),
            ("moonlight", "moonlight", "Gaming"),
            ("limelight", "moonlight", "Gaming"),
            ("game", "gaming", "Gaming"),
            ("hbchannel", "homebrew", "Utilities"),
        ]
        .into_iter()
        .find_map(|(needle, brand, category)| {
            contains_word(&identity, needle).then_some((brand, category))
        })
        .unwrap_or(("generic", "Utilities"));
        let (brand, category) =
            crate::branding::for_app(id).map_or(generic, |brand| (brand.key, brand.category));
        Self {
            id: id.into(),
            title: title.into(),
            brand: brand.into(),
            category: category.into(),
            icon: None,
            mark: "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Input {
    pub id: String,
    pub title: String,
    pub port: String,
    pub connected: Option<bool>,
    /// Optional signal information; never overrides the cable state or Auto visibility.
    pub signal: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub apps: Vec<App>,
    pub inputs: Vec<Input>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum InputMode {
    #[default]
    Auto,
    On,
    Off,
}

/// The interface scale chosen in General › Interface scale.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiSize {
    #[default]
    Normal,
    Large,
    Larger,
}
impl UiSize {
    pub fn scale(self) -> f32 {
        match self {
            Self::Normal => 1.,
            Self::Large => 1.25,
            Self::Larger => 1.5,
        }
    }
    pub const ALL: [Self; 3] = [Self::Normal, Self::Large, Self::Larger];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|size| *size == self).unwrap_or(0)
    }
    /// Message keys of the option's name and description.
    pub fn keys(self) -> (&'static str, &'static str) {
        match self {
            Self::Normal => ("sizeNormal", "sizeNormalHint"),
            Self::Large => ("sizeLarge", "sizeLargeHint"),
            Self::Larger => ("sizeLarger", "sizeLargerHint"),
        }
    }
}
/// Unknown or malformed sizes fall back to normal instead of failing the preferences.
impl<'de> Deserialize<'de> for UiSize {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match serde_json::Value::deserialize(deserializer)?.as_str() {
                Some("large") => Self::Large,
                Some("larger") => Self::Larger,
                _ => Self::Normal,
            },
        )
    }
}

/// POST is the default because Home Assistant webhooks accept only POST and PUT
/// unless GET is enabled on the trigger.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TileMethod {
    Get,
    #[default]
    Post,
}

impl TileMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// A home-screen tile that fires a one-way HTTP request instead of launching
/// an app. Its id lives in `favorites`/`hidden` like any real app id, so
/// ordering, favoriting and hiding reuse the existing machinery unchanged.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CustomTile {
    pub id: String,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub method: TileMethod,
    /// One of `TILE_ICONS`; empty shows the first.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
}

impl CustomTile {
    pub fn icon(&self) -> &'static str {
        TILE_ICONS
            .iter()
            .find(|(icon, _)| *icon == self.icon)
            .map_or(TILE_ICONS[0].0, |(icon, _)| icon)
    }
}

/// Interface icons a shortcut tile can show, with their message keys.
pub const TILE_ICONS: [(&str, &str); 15] = [
    ("weather-moon", "iconMoon"),
    ("weather-sun", "iconSun"),
    ("weather-cloud", "iconCloud"),
    ("tv", "iconTv"),
    ("power", "iconPower"),
    ("globe", "iconGlobe"),
    ("clock", "iconClock"),
    ("coffee", "iconCoffee"),
    ("gaming", "iconGaming"),
    ("heart", "iconHeart"),
    ("location", "iconLocation"),
    ("thermometer", "iconThermometer"),
    ("sparkles", "iconSparkles"),
    ("alert", "iconAlert"),
    ("eye", "iconEye"),
];

pub const CUSTOM_TILE_PREFIX: &str = "custom.";
pub const TILE_LIMIT: usize = 64;
pub const TILE_URL_LIMIT: usize = 512;

/// What the Lampa widget lists: series with a new episode that Lampa's viewers watch now (CUB's
/// feed), or the account's history to continue with its bookmarks' new episodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LampaFeed {
    #[default]
    Watching,
    Continue,
}

/// Anything but a known feed leaves the default.
fn deserialize_lampa_feed<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<LampaFeed, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DateFormat {
    #[default]
    Localized,
    DayMonthYear,
    MonthDayYear,
    YearMonthDay,
}

impl DateFormat {
    pub const ALL: [Self; 4] = [
        Self::Localized,
        Self::DayMonthYear,
        Self::MonthDayYear,
        Self::YearMonthDay,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|format| *format == self).unwrap()
    }

    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::Localized => language.t("dateLocalized"),
            Self::DayMonthYear => "DD.MM.YYYY",
            Self::MonthDayYear => "MM/DD/YYYY",
            Self::YearMonthDay => "YYYY-MM-DD",
        }
    }

    pub fn format(self, language: Language, now: time::OffsetDateTime) -> String {
        let (year, month, day) = (now.year(), now.month() as u8, now.day());
        match self {
            Self::Localized => language.date(now),
            Self::DayMonthYear => format!("{day:02}.{month:02}.{year:04}"),
            Self::MonthDayYear => format!("{month:02}/{day:02}/{year:04}"),
            Self::YearMonthDay => format!("{year:04}-{month:02}-{day:02}"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub favorites: Option<Vec<String>>,
    pub custom_tiles: Vec<CustomTile>,
    pub hidden: Vec<String>,
    pub background: String,
    /// Last manually chosen splash, independent of automatic rotation.
    pub splash_background: Option<String>,
    pub deleted_backgrounds: Vec<String>,
    pub show_home_names: bool,
    /// Names under the Apps page's cards: the focused card's only, every card's, or none.
    #[serde(alias = "show_app_names", deserialize_with = "deserialize_app_names")]
    pub app_names: AppNames,
    pub hour_24: bool,
    pub reduced_motion: bool,
    /// The colour of the focused card's glow on Home.
    pub focus_glow: FocusGlow,
    pub show_seconds: bool,
    /// Optional name appended to the Home greeting ("Good evening, Ana."). Empty = plain greeting.
    pub greeting_name: String,
    pub show_greeting: bool,
    pub show_date: bool,
    pub date_format: DateFormat,
    /// Home's TV Shows row: series with a new episode that Lampa watches now, from CUB.
    #[serde(deserialize_with = "crate::cs2::deserialize_flag")]
    pub show_shows: bool,
    /// Home's CS2 Scores row below the inputs, and what it lists.
    #[serde(deserialize_with = "crate::cs2::deserialize_flag")]
    pub show_cs2: bool,
    /// CS2 Scores' live and coming matches in the Today card instead of a row of their own.
    pub cs2_today: bool,
    /// Today's new episodes in the Today card instead of TV Shows' own row.
    pub shows_today: bool,
    /// What the Lampa widget lists.
    #[serde(deserialize_with = "deserialize_lampa_feed")]
    pub lampa_feed: LampaFeed,
    /// Liquipedia's tier 1 and 2 events only.
    #[serde(deserialize_with = "crate::cs2::deserialize_enabled")]
    pub cs2_major_only: bool,
    /// Finished matches after the live and upcoming ones.
    #[serde(deserialize_with = "crate::cs2::deserialize_enabled")]
    pub cs2_results: bool,
    pub input_modes: BTreeMap<String, InputMode>,
    /// Custom names by input app ID; the TV's port and label stay visible as detail.
    pub input_names: BTreeMap<String, String>,
    /// The crypto prices, a line of the Today card.
    #[serde(deserialize_with = "crate::crypto::deserialize_flag")]
    pub show_crypto: bool,
    /// Up to `crypto::MAX_COINS` Binance coins: `crypto::COINS` in its order, then the others.
    #[serde(deserialize_with = "crate::crypto::deserialize_coins")]
    pub crypto_coins: Vec<String>,
    /// The block beside the greeting sits on a card; off, straight on the wallpaper.
    pub crypto_card: bool,
    #[serde(deserialize_with = "crate::weather::deserialize_enabled")]
    pub show_weather: bool,
    #[serde(deserialize_with = "crate::weather::deserialize_location")]
    pub weather_location: Option<Location>,
    #[serde(deserialize_with = "crate::weather::deserialize_unit")]
    pub temperature_unit: Unit,
    #[serde(deserialize_with = "crate::i18n::deserialize_language")]
    pub language: Language,
    pub ui_scale: UiSize,
    /// Applied at launch because MSAA is selected when SDL creates the GL window.
    #[serde(deserialize_with = "crate::text_rendering::deserialize")]
    pub text_rendering: crate::text_rendering::TextRendering,
    #[serde(deserialize_with = "crate::wallpaper_cycle::deserialize")]
    pub wallpaper_cycle: bool,
    pub wallpaper_gradient: bool,
    /// Unix seconds of the last automatic or chosen wallpaper change, for the timed modes.
    pub wallpaper_changed: u64,
    /// Where the Apple TV wallpaper is in its shuffled places (aerials.rs).
    pub aerials: crate::aerials::Rotation,
    /// Opt-in: install newer versions of Homebrew apps automatically (homebrew_updates.rs).
    pub homebrew_updates: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            favorites: None,
            custom_tiles: vec![],
            hidden: vec![],
            background: "dunes".into(),
            splash_background: None,
            deleted_backgrounds: vec![],
            show_home_names: false,
            app_names: AppNames::default(),
            hour_24: false,
            reduced_motion: false,
            focus_glow: FocusGlow::default(),
            show_seconds: false,
            greeting_name: String::new(),
            show_greeting: true,
            show_date: true,
            date_format: DateFormat::default(),
            show_cs2: false,
            show_shows: false,
            cs2_today: true,
            shows_today: true,
            lampa_feed: LampaFeed::Watching,
            cs2_major_only: true,
            cs2_results: true,
            input_modes: BTreeMap::new(),
            input_names: BTreeMap::new(),
            show_crypto: false,
            crypto_coins: crate::crypto::default_coins(),
            crypto_card: true,
            show_weather: true,
            weather_location: None,
            temperature_unit: Unit::Celsius,
            language: Language::En,
            ui_scale: UiSize::Normal,
            text_rendering: crate::text_rendering::TextRendering::default(),
            wallpaper_cycle: false,
            wallpaper_gradient: true,
            wallpaper_changed: 0,
            aerials: crate::aerials::Rotation::default(),
            homebrew_updates: false,
        }
    }
}

/// A widget that can stand in the middle of Home; one at most does, the rest go into Today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Center {
    Cs2,
    Shows,
}

impl Preferences {
    /// The widget in the middle of Home: the first shown outside Today, CS2, then shows. Rows
    /// saved before the center existed can leave both outside; the other goes into Today.
    pub fn center(&self) -> Option<Center> {
        [Center::Cs2, Center::Shows]
            .into_iter()
            .find(|&widget| self.shown(widget) && !self.placed_in_today(widget))
    }

    /// Whether the crypto prices have their line in the Today card, the only place they show.
    pub fn crypto_in_today(&self) -> bool {
        self.show_crypto && !self.crypto_coins.is_empty()
    }

    /// Whether `widget` has lines in the Today card.
    pub fn in_today(&self, widget: Center) -> bool {
        self.shown(widget) && self.center() != Some(widget)
    }

    /// Puts `widget` in the middle of Home, or back into Today; the one it replaces moves
    /// into Today.
    pub fn place_in_center(&mut self, widget: Center, center: bool) {
        if center && let Some(previous) = self.center() {
            self.set_in_today(previous, true);
        }
        self.set_in_today(widget, !center);
    }

    fn shown(&self, widget: Center) -> bool {
        match widget {
            Center::Cs2 => self.show_cs2,
            Center::Shows => self.show_shows,
        }
    }

    fn placed_in_today(&self, widget: Center) -> bool {
        match widget {
            Center::Cs2 => self.cs2_today,
            Center::Shows => self.shows_today,
        }
    }

    fn set_in_today(&mut self, widget: Center, today: bool) {
        match widget {
            Center::Cs2 => self.cs2_today = today,
            Center::Shows => self.shows_today = today,
        }
    }

    pub fn splash_background(&self) -> &str {
        self.splash_background
            .as_deref()
            .unwrap_or(&self.background)
    }

    pub fn favorites(&self, apps: &[App]) -> Vec<String> {
        self.favorites.clone().unwrap_or_else(|| {
            let mut ids = Vec::new();
            for brand in [
                "netflix",
                "youtube",
                "apple-tv",
                "disney-plus",
                "prime-video",
                "live-tv",
            ] {
                if let Some(app) = apps.iter().find(|app| app.brand == brand) {
                    ids.push(app.id.clone());
                }
            }
            for app in apps {
                if ids.len() >= 6 {
                    break;
                }
                if !ids.contains(&app.id) {
                    ids.push(app.id.clone());
                }
            }
            ids
        })
    }

    /// Synthetic `App` entries for custom tiles, merged into the real
    /// inventory so favoriting, hiding, search and the Apps grid treat them
    /// exactly like any installed app.
    pub fn custom_apps(&self) -> Vec<App> {
        self.custom_tiles
            .iter()
            .map(|tile| App {
                mark: tile.icon(),
                ..App::new(&tile.id, &tile.title)
            })
            .collect()
    }

    pub fn custom_tile(&self, id: &str) -> Option<&CustomTile> {
        self.custom_tiles.iter().find(|tile| tile.id == id)
    }

    pub fn remove_tile(&mut self, id: &str) -> Option<CustomTile> {
        let index = self.custom_tiles.iter().position(|tile| tile.id == id)?;
        if let Some(favorites) = &mut self.favorites {
            favorites.retain(|item| item != id);
        }
        self.hidden.retain(|item| item != id);
        Some(self.custom_tiles.remove(index))
    }

    pub fn input_visible(&self, input: &Input) -> bool {
        match self.input_modes.get(&input.id).copied().unwrap_or_default() {
            InputMode::On => true,
            InputMode::Off => false,
            InputMode::Auto => input.connected == Some(true),
        }
    }

    pub fn input_name(&self, input: &Input) -> Option<&str> {
        self.input_names.get(&input.id).map(String::as_str)
    }

    /// The custom name if one is saved, otherwise the port ("HDMI 2").
    pub fn input_title<'a>(&'a self, input: &'a Input) -> &'a str {
        self.input_name(input).unwrap_or(&input.port)
    }

    /// Saves a cleaned name; an empty name restores the port title.
    pub fn set_input_name(&mut self, id: &str, name: &str) -> bool {
        if !valid_app_id(id) {
            return false;
        }
        let name = clean_input_name(name);
        if name.is_empty() {
            self.input_names.remove(id).is_some()
        } else {
            self.input_names.insert(id.into(), name.clone()) != Some(name)
        }
    }

    pub fn sanitize(&mut self) {
        if self.splash_background.as_ref().is_some_and(|id| {
            !BACKGROUNDS.iter().any(|(known, _)| *known == id) && !crate::wallpapers::valid_id(id)
        }) {
            self.splash_background = None;
        }
        if !BACKGROUNDS.iter().any(|(id, _)| *id == self.background)
            && !crate::wallpapers::valid_id(&self.background)
            && self.background != crate::aerials::ID
        {
            self.background = "dunes".into();
        }
        let mut seen = HashSet::new();
        self.deleted_backgrounds.retain(|id| {
            id != &self.background
                && BACKGROUNDS.iter().any(|(known, _)| *known == id)
                && seen.insert(id.clone())
        });
        fn clean(ids: &mut Vec<String>) {
            let mut seen = HashSet::new();
            ids.retain(|id| valid_app_id(id) && seen.insert(id.clone()));
            ids.truncate(1000);
        }
        if let Some(ids) = &mut self.favorites {
            clean(ids);
        }
        clean(&mut self.hidden);
        {
            let mut seen = HashSet::new();
            for tile in &mut self.custom_tiles {
                tile.title = clean_input_name(&tile.title);
                tile.url = tile.url.trim().into();
                if tile.icon() != tile.icon {
                    tile.icon.clear();
                }
            }
            self.custom_tiles.retain(|tile| {
                valid_app_id(&tile.id)
                    && tile.id.starts_with(CUSTOM_TILE_PREFIX)
                    && !tile.title.is_empty()
                    && valid_tile_url(&tile.url)
                    && seen.insert(tile.id.clone())
            });
            self.custom_tiles.truncate(TILE_LIMIT);
        }
        self.input_modes.retain(|id, _| valid_app_id(id));
        self.input_names = std::mem::take(&mut self.input_names)
            .into_iter()
            .filter(|(id, _)| valid_app_id(id))
            .map(|(id, name)| (id, clean_input_name(&name)))
            .filter(|(_, name)| !name.is_empty())
            .take(128)
            .collect();
    }
}

pub const INPUT_NAME_LIMIT: usize = 40;

pub fn clean_input_name(name: &str) -> String {
    let clean: String = name.chars().filter(|c| !c.is_control()).collect();
    let clean: String = clean.trim().chars().take(INPUT_NAME_LIMIT).collect();
    clean.trim_end().into()
}

/// The id for a new tile titled `title`, unique among `tiles`.
pub fn new_tile_id(tiles: &[CustomTile], title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(48)
        .collect();
    let slug = match slug.trim_end_matches('-') {
        "" => "tile",
        slug => slug,
    };
    let base = format!("{CUSTOM_TILE_PREFIX}{slug}");
    (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !tiles.iter().any(|tile| &tile.id == id))
        .unwrap()
}

/// Whether the URL uses https, and the address after the scheme.
pub fn split_tile_url(url: &str) -> Option<(bool, &str)> {
    let start = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    if start.starts_with("https://") {
        Some((true, &url[8..]))
    } else if start.starts_with("http://") {
        Some((false, &url[7..]))
    } else {
        None
    }
}

/// Whether the URL can be sent as typed. It must parse with the request
/// client's URI rules and name a host, so a tile never saves with an address
/// that only fails once it is pressed.
pub fn valid_tile_url(url: &str) -> bool {
    split_tile_url(url).is_some()
        && url.chars().count() <= TILE_URL_LIMIT
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && url.parse::<ureq::http::Uri>().is_ok_and(|uri| {
            uri.authority()
                .is_some_and(|authority| valid_tile_authority(authority.as_str()))
        })
}

/// Host and optional port after any user info: a host name, an IPv4 address
/// or a bracketed IPv6 address, then a port from 1 to 65535.
fn valid_tile_authority(authority: &str) -> bool {
    let address = authority.rsplit('@').next().unwrap_or_default();
    let (host, port) = match address.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((ip, port)) if ip.parse::<std::net::Ipv6Addr>().is_ok() => (None, port),
            _ => return false,
        },
        None => match address.find(':') {
            Some(colon) => (Some(&address[..colon]), &address[colon..]),
            None => (Some(address), ""),
        },
    };
    let valid_port = port.is_empty()
        || port.strip_prefix(':').is_some_and(|digits| {
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && digits.parse::<u16>().is_ok_and(|port| port != 0)
        });
    valid_port && host.is_none_or(valid_host_name)
}

/// DNS-style labels, or a dotted IPv4 address when every label is a number.
fn valid_host_name(host: &str) -> bool {
    let labels: Vec<_> = host.split('.').collect();
    if labels
        .iter()
        .all(|label| !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit()))
    {
        return host.parse::<std::net::Ipv4Addr>().is_ok();
    }
    host.len() <= 253
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

pub fn valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .split(['.', '_', '-'])
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric()))
}

pub fn demo_inventory() -> Inventory {
    let mut inventory = Inventory {
        apps: [
            ("com.webos.app.livetv", "Live TV"),
            ("netflix", "Netflix"),
            ("youtube.leanback.v4", "YouTube"),
            ("com.apple.appletv", "Apple TV"),
            ("com.disney.disneyplus-prod", "Disney+"),
            ("amazon", "Prime Video"),
            ("plex", "Plex"),
            ("spotify-beehive", "Spotify"),
            ("org.jellyfin.webos", "Jellyfin"),
            ("org.xbmc.kodi", "Kodi"),
            ("com.limelight.webos", "Moonlight"),
            ("org.webosbrew.hbchannel", "Homebrew Channel"),
            ("com.webos.app.browser", "Web Browser"),
            ("com.arte.tv", "ARTE"),
            ("de.zdf.app.zdfm3", "ZDF"),
            ("ard.mediathek", "Das Erste"),
            ("coupangplay", "Coupang Play"),
            ("com.frograms.watchaplay.webos", "Watcha Play"),
            ("pooq", "Wavve"),
            ("com.cmb.rainbowtv", "Rainbow TV"),
            ("com.lgtvod.kr.app", "LG TV VOD"),
            ("com.webos.app.igallery", "올레드 갤러리"),
            ("com.webos.app.roomconnect", "룸투룸 셰어"),
        ]
        .into_iter()
        .map(|(id, title)| App::new(id, title))
        .collect(),
        inputs: vec![
            Input {
                id: "com.webos.app.hdmi1".into(),
                title: "PlayStation 5".into(),
                port: "HDMI 1".into(),
                connected: Some(true),
                signal: None,
            },
            Input {
                id: "com.webos.app.hdmi2".into(),
                title: "Apple TV".into(),
                port: "HDMI 2".into(),
                connected: Some(true),
                signal: None,
            },
            Input {
                id: "com.webos.app.hdmi3".into(),
                title: "HDMI 3".into(),
                port: "HDMI 3".into(),
                connected: Some(false),
                signal: None,
            },
        ],
    };
    // Desktop-only artwork gallery. TV inventory is always read from services.
    for brand in crate::branding::BRANDS {
        if !inventory.apps.iter().any(|app| app.brand == brand.key) {
            inventory.apps.push(App::new(brand.ids[0], brand.title));
        }
    }
    inventory
}
