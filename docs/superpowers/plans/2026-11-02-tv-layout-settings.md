# TV layout, phase 4: Settings — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings on the tvOS frame. A sidebar collapses to icons, the list is 820 px wide with rows that grow on focus, a live small Home preview sits on the right (a summary for System), and the wallpaper gallery has four 390 px columns.

**Architecture:**

- All geometry comes from new tokens in `src/layout.rs`.
- The sidebar width and the focused row's growth ride the existing springs in `src/motion.rs` (`Lane::Sidebar`, and the focus spring with the focused Settings target as `selected`).
- The preview is Home's own scene. `view::home_preview(&mut Scene, &mut State, …)` swaps the few view fields to Home, builds `scene_with_pill`, restores them, scales every draw into the frame and splices it in where `settings()` reserved a slot. `main::compose` calls it, and so does a test helper.
- Gallery navigation works on "slots": the backgrounds plus the upload tile, four per row.

**Tech Stack:** Rust, the existing GL draw list (`Draw`), `cargo test --bin luma`.

**Spec:** `docs/superpowers/specs/2026-11-01-tv-layout-design.md`. Sections «Настройки», «Галерея фонов», «Типографика и размеры», «Что остаётся исполнителю». Mockups: `2026-11-01-tv-layout/settings.jpg` and `background.jpg`.

## Global Constraints

- Safe area 80/60. Settings margins 160 → 80: the sidebar starts at x 80.
- Sidebar: 76 px icon column while focus is in the content, 360 px (icon + name) while focus is on it. The section name is a 44 px title above the list.
- List x 196–1016 (820 px). Rows 68 px. Text 26 px, value 24 px right at the list's right edge. The focused row shows its description (22 px) inside and grows to 104 px.
- Right: live Home preview 776 × 437 (scale ≈ 0.4), highlighting the affected place, with a couple of lines below it. In System, a summary instead (Luma version and update status: what the section already has).
- Subpages open in place of the list, titled with the path "Виджеты › Погода". The sidebar and the preview stay.
- Gallery: grid x 196–1840, 4 columns 390 × 219, gaps 28/32, 12 tiles on screen. «Градиент» and «Смена» sit pinned in the title row on the right. The grid scrolls under a mask. «Загрузить» is the last tile, dashed. The Apple TV tile is unchanged.
- Type: page title 44/600, body 26/550, secondary 24/450 at 62 %, captions 22/450. Card radius 14, panels 18–24.
- Text straight on the wallpaper always has a shadow (`TextStyle::shadow()`). Text on panels has none.
- Reduced motion: everything snaps (the springs already do this when `reduced`).
- New UI strings go into `assets/messages.json` for all 13 languages (`i18n::tests::every_message_has_all_languages_and_no_empty_values` enforces this).
- Never write a PandaScore key into any file.
- Tests that pin replaced geometry or behaviour (sidebar breadcrumbs, 3-column gallery, upload in the header, 640/1120 rows, `appearance_preview`) are updated to the spec's values in the task that replaces them. Each one is ledgered as a Ruling.

## Review Focus

1. Long German or Russian titles next to long values in an 820 px row: the title shrinks (`fit_text`) and never runs under the value.
2. Pointer and wheel: rows scrolled up under the pinned title, and gallery tiles under the mask, are neither drawn above the mask nor clickable. The collapsed sidebar's icons are clickable and expand it.
3. The preview while the wallpaper cross-fades, with an overlay open, or with every Home widget off: nothing escapes the frame, and nothing in it takes focus.
4. A gallery with many uploaded wallpapers: the upload tile stays last and reachable; Down from the last full row lands on the last slot.
5. Reduced motion: the sidebar and the growing row snap to their final size in one frame.

---

### Task 1: Sidebar that collapses to icons, and the page title

**Files:**

- Modify: `src/layout.rs` (Settings tokens), `src/motion.rs` (`Lane::Sidebar`), `src/state.rs` (`animate`, `sidebar_width`), `src/view.rs` (`settings_sidebar`, new `settings_title`, `breadcrumb`, content dim)
- Create: `assets/ui-icons/photo.svg` (Lucide "image", same stroke attributes as `settings.svg`)
- Create: `src/settings-layout-tests.rs` (registered in `src/tests.rs` like `home-layout-tests.rs`)

**Interfaces:**

- Produces in `layout.rs`:
  - `SIDEBAR_COLLAPSED = 76.`, `SIDEBAR_EXPANDED = 360.`, `SIDEBAR_ITEM = 60.`, `SIDEBAR_PITCH = 68.`
  - `LIST_X = 196.`, `LIST_WIDTH = 820.`, `LIST_TOP = 224.`
  - `ROW_HEIGHT = 68.`, `ROW_GROWN = 104.`, `ROW_SPACING = 10.`
  - `PREVIEW_X = 1064.`, `PREVIEW_Y = 190.`, `PREVIEW_WIDTH = 776.`, `PREVIEW_HEIGHT = 437.`
  - `GALLERY_COLUMNS: usize = 4`, `GALLERY_TILE_WIDTH = 390.`, `GALLERY_TILE_HEIGHT = 219.`, `GALLERY_GAP_X = 28.`, `GALLERY_GAP_Y = 32.`, `GALLERY_TOP = 236.`
  - `type_scale::PAGE_TITLE = 44.`
- Produces in `state.rs`: `pub fn sidebar_width(&self) -> f32`, the eased width: `motion.shown(Lane::Sidebar, target)`, where target is `SIDEBAR_EXPANDED` when `focus` is `Target::Sidebar(_)`, else `SIDEBAR_COLLAPSED`.
- Produces in `view.rs`: `fn breadcrumb(state: &State) -> Vec<&str>` (the section name, then the subpage path; built from the strings the old sidebar pill used) and `fn sidebar_dim(state: &State) -> f32` (1.0 collapsed → 0.55 expanded, linear in the width).

- [ ] **Step 1: Write the failing tests** in `src/settings-layout-tests.rs`:

```rust
#[test]
fn the_sidebar_is_an_icon_column_while_the_list_has_focus() {
    let mut state = settings(0);
    state.focus = Target::Setting(0);
    settle(&mut state);
    for index in 0..4 {
        assert_eq!(sidebar_item(&state, index), Rect::new(88., 158. + index as f32 * 68., 60., 60.));
    }
    state.focus = Target::Sidebar(1);
    settle(&mut state);
    assert_eq!(sidebar_item(&state, 1), Rect::new(88., 226., 344., 60.));
}

#[test]
fn the_sidebar_eases_open_unless_motion_is_reduced() {
    let mut state = settings(0);
    state.focus = Target::Setting(0);
    settle(&mut state);
    state.focus = Target::Sidebar(0);
    state.animate(1. / 60.);
    let w = state.sidebar_width();
    assert!(w > 76. && w < 360., "{w}");
    state.preferences.reduced_motion = true;
    state.animate(1. / 60.);
    assert_eq!(state.sidebar_width(), 360.);
}

#[test]
fn the_page_title_names_the_section_and_the_subpage_path() {
    let mut state = settings(WIDGETS_SECTION);
    state.subpage = Some(Subpage::Weather);
    state.focus = Target::Choice(0);
    let titles: Vec<_> = texts(&state).into_iter().filter(|(_, r, s)| r.y == 150. && s.size <= 44.).collect();
    assert_eq!(titles.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(), ["Widgets", "Weather"]);
    assert_eq!(titles[0].1.x, 196.);
    assert!(titles.iter().all(|(_, _, s)| s.shadow && s.weight == 600));
}
```

Helpers in the file:

- `settings(section)`: a `ready()` state with `page = Settings` and `section` set.
- `settle(&mut state)`: runs `animate(1.)` three times.
- `sidebar_item(&state, i)`: the `Target::Sidebar(i)` control rect.
- `texts(&state)`: as in `home-layout-tests.rs`.

- [ ] **Step 2: Run them, verify they fail**

  Run: `cargo test --bin luma settings_layout -- --nocapture`

  Expected: FAIL. The sidebar rects are still `160, 164.8…, 403.2, 72` and there is no title.

- [ ] **Step 3: Implement**
  - `Lane::Sidebar` in `motion.rs`.
  - In `animate()`, Settings passes `vec![(Lane::Sidebar, sidebar_target)]`.
  - `settings_sidebar` draws one panel at x 80, y 150, width `sidebar_width()`, height `8 + 4 * 68`, radius 38, fill `c(0x14181d, 0.92)`, 1 px border `c(0xffffff, 0.08)`.
  - Items: `Rect::new(88., 158. + i * 68., width - 16., 60.)`.
  - Icons 28 px at x 104: `settings`, `photo`, `sparkles`, `tv`.
  - Selected section: a `c(0xffffff, 0.16)` fill. Focused: the white `RoundedGradient` with dark icon and label.
  - Labels 26/550 at x 148, with alpha `(width - 76) / 284` under a `Clip` to the panel.
  - Drop the breadcrumb code from the sidebar.
  - `settings_title(out, state, assets, max_width)` draws `breadcrumb(state)` at (196, 150): 44/600, tracking −0.02, `.shadow()`.
    - Separators are a 26 px `right` icon with 10 px gaps.
    - Earlier parts are at 62 % alpha.
    - The whole path shrinks uniformly to fit `max_width` (`LIST_WIDTH` here; Task 4 passes the gallery's width).
  - The content drawn by `settings_content` is faded with `out.fade(start, 1., sidebar_dim(state))`.

- [ ] **Step 4: Run the tests, verify they pass**

  Run: `cargo test --bin luma settings_layout`, then `cargo test --bin luma > <workspace>/t1.log 2>&1; tail -5 <workspace>/t1.log`

  Expected: the new tests PASS. Old tests pinning the sidebar breadcrumb or the 160 px sidebar are updated to the new geometry and ledgered. The whole suite is green.

- [ ] **Step 5: Commit** `feat(settings): the sidebar folds to icons and the page has a title`

### Task 2: The 820 px list, rows that grow on focus, descriptions

**Files:**

- Modify: `src/view.rs`:
  - `Row` and `row()`;
  - every Settings row site (main sections, choice subpages);
  - the subpage functions `weather_settings`, `crypto_settings`, `cs2_settings`, `location_settings`, `inputs_settings`, `greeting_settings`, `date_settings`, `clock_settings`, `app_name_settings`, `input_name_rows`, `shortcut_settings`, `shows_settings`;
  - remove `appearance_preview`.
- Modify: `src/state.rs`:
  - `animate()` selects the focused Settings row target;
  - `reveal_focus` top is `LIST_TOP`;
  - new `list_viewport()` used by `hit_test` and `wheel_scroll` for Settings' scrollable controls.
- Modify: `assets/messages.json` (descriptions).
- Test: `src/settings-layout-tests.rs`

**Interfaces:**

- Consumes: Task 1 tokens.
- Produces:
  - `fn list_rects(state: &State, top: f32, rows: &[(Target, bool)]) -> Vec<Rect>`. The `bool` is "has a description". Each height is `ROW_HEIGHT + (ROW_GROWN - ROW_HEIGHT) * state.motion.focus(target)` when the row has a description, else `ROW_HEIGHT`. Spacing is `ROW_SPACING`, then `- state.scroll`.
  - `pub fn list_viewport(&self) -> Rect` = `Rect::new(0., LIST_TOP - 10., 1920., content bottom - (LIST_TOP - 10.))`.
- Message keys, each with a one-line, sentence-case description in all 13 languages:
  - `appNamesAbout`, `clockAbout`, `languageAbout`, `reducedMotionAbout`, `focusGlowAbout`, `uiScaleAbout`;
  - `dateAbout`, `greetingAbout`, `weatherAbout`, `shortcutsAbout`, `inputsAbout`, `cryptoAbout`, `cs2ScoreAbout`, `showsAbout`;
  - `refreshAppsAbout`, `textRenderingAbout`, `homebrewUpdatesAbout`, `quitAppAbout`, `homeTakeoverAbout`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn rows_fill_the_820_list_under_the_title() {
    let mut state = settings(0);
    state.focus = Target::Sidebar(0);
    settle(&mut state);
    let rows = setting_rects(&state); // Target::Setting(_) controls in draw order
    assert_eq!(rows[0], Rect::new(196., 224., 820., 68.));
    assert_eq!(rows[1].y - rows[0].y, 78.);
}

#[test]
fn the_focused_row_grows_to_show_its_description() {
    let mut state = settings(0);
    state.focus = Target::Setting(1); // Clock, the second row
    settle(&mut state);
    let rows = setting_rects(&state);
    assert_eq!(rows[1].h, 104.);
    assert_eq!(rows[2].y, 224. + 2. * 78. + 36.);
    let about = crate::i18n::Language::En.t("clockAbout");
    let (_, rect, style) = texts(&state).into_iter().find(|t| t.0 == about).expect("described");
    assert_eq!(style.size, 22.);
    assert!(rect.y > rows[1].y + 40. && rect.y + rect.h <= rows[1].y + 104.);
    assert!(!texts(&state).iter().any(|t| t.0 == crate::i18n::Language::En.t("languageAbout")));
}

#[test]
fn values_sit_at_the_right_edge_of_the_list_and_titles_never_reach_them() {
    let mut state = settings(0);
    state.preferences.language = crate::i18n::Language::De;
    state.focus = Target::Setting(4); // Language
    settle(&mut state);
    let drawn = texts(&state);
    let (_, value, style) = drawn.iter().find(|t| t.0 == "Deutsch").unwrap();
    assert_eq!(style.size, 24.);
    assert!(value.x + value.w <= 1016. - 24. && value.x + value.w >= 1016. - 80.);
    for (text, rect, style) in drawn.iter().filter(|t| t.2.size <= 26. && t.1.y < value.y + 10. && t.1.y > value.y - 30.) {
        if *text != "Deutsch" { assert!(rect.x + measured(text, *style) <= value.x + value.w - measured("Deutsch", *style), "{text}"); }
    }
}

#[test]
fn rows_scrolled_under_the_title_are_masked_and_cannot_be_clicked() {
    let mut state = settings(WIDGETS_SECTION);
    state.focus = Target::Widget(7);
    settle(&mut state);
    let controls = controls(&state);
    assert!(state.scroll > 0.);
    let first = controls.iter().find(|c| c.target == Target::Widget(4)).unwrap().rect;
    assert!(first.y < 214.);
    assert_ne!(state.hit_test(&controls, first.x + 10., first.y + first.h - 2.), Some(Target::Widget(4)));
}
```

- [ ] **Step 2: Run, verify they fail**

  Run: `cargo test --bin luma settings_layout`

  Expected: FAIL. Rows are at 640/1120/72, and the `*About` keys are missing.

- [ ] **Step 3: Implement**
  - `row()`:
    - radius 16, icon 26 px at x + 24;
    - title 26/550 at x + 68 (x + 24 without an icon), top-anchored at y + 18.5, shrunk with `fit_text` to end 16 px before the value;
    - value 24/450 right-aligned, ending at `right − (chevron ? 56 : 24)`, colour `c(0xffffff, 0.62)` (focused `c(0x2c2c2c, 0.62)`);
    - chevron 20 px at `right − 44`;
    - description 22/450 at y + 56, drawn only while `rect.h > ROW_HEIGHT + 1`, alpha `(h − 68) / 36`.
  - Remove the `WallpaperCycle`/`WallpaperGradient`/`UploadWallpaper` special cases. Those controls get their own drawing in Task 4. Until then they use a temporary `x = 1840 − width` placement.
  - Every Settings list uses `list_rects(state, LIST_TOP, …)` at `LIST_X`/`LIST_WIDTH`. Choice rows keep their existing descriptions, now shown only on focus.
  - Subpage panels (shortcut card, weather search, input name rows) move to `LIST_X`, `LIST_WIDTH`, `LIST_TOP` with unchanged heights.
  - The list is drawn inside `Draw::Clip(Some(state.list_viewport()))` with `Draw::EdgeFade(Some([214., 236., 1040., 1080.]))`, reset after.
  - `appearance_preview` is removed: the live preview (Task 3) shows the scale.

- [ ] **Step 4: Run, verify they pass; whole suite**

  Run: `cargo test --bin luma > <workspace>/t2.log 2>&1; tail -5 <workspace>/t2.log`

  Expected: green. Updated old tests are ledgered (640/1120 geometry, `long_captions_expand_rows…`, `language_list_scrolls_under_fixed_ui_without_clipping_or_fades`, `interface_scale_enlarges_the_clock_and_the_preview_is_not_focusable`).

- [ ] **Step 5: Commit** `feat(settings): an 820 px list whose focused row grows to describe itself`

### Task 3: Live Home preview, and the System summary

**Files:**

- Modify: `src/view.rs`:
  - `Scene` gains `regions: Vec<(Region, Rect)>` and `preview: Option<PreviewSlot>`;
  - `home()`/`header()` push regions;
  - `settings()` reserves the slot;
  - new `pub fn home_preview`;
  - System section layout.
- Modify: `src/main.rs` (`compose` calls `view::home_preview`), `assets/messages.json` (`previewTitle`, `previewHint`).
- Test: `src/settings-layout-tests.rs`

**Interfaces:**

- Produces:
  - `pub enum Region { Clock, Date, Greeting, Weather, Favorites, Inputs, Crypto, Cs2, Shows }`. Crypto, Cs2 and Shows push the Today card's rect when they live in it, their row's rect otherwise.
  - `pub struct PreviewSlot { at: usize, region: Option<Region> }`.
  - `pub fn home_preview(scene: &mut Scene, state: &mut State, assets: &Assets, now: OffsetDateTime)`:
    1. It does nothing unless `scene.preview` is set.
    2. It swaps `page → Home`, `focus → Nav(0)`, `scroll`/`home_scroll → 0`, and takes `overlay`, `notice`, `error`, `pressed`, `moving`.
    3. It builds `scene_with_pill` with the Home pill and restores the fields.
    4. It maps every draw by `k = PREVIEW_WIDTH / 1920`. Rects, radii, blur and stroke widths are scaled (strokes `max(1.)`), text sizes too. `Clip(None)` becomes the frame. `Clip(Some)` becomes the scaled rect cut to the frame. Edge fades are scaled. Full-screen images get radius 18.
    5. It fades by `sidebar_dim`, splices the draws at `at`, then a 2 px white ring (radius 10, inset −6) around the scaled region rect when that region exists.
  - `fn preview_region(state: &State) -> Option<Region>`, from the focused row or the open subpage:
    - app names, shortcuts and focus glow → Favorites;
    - clock and its formats → Clock;
    - date and its format → Date;
    - greeting → Greeting;
    - weather, location and temperature → Weather;
    - inputs and an input → Inputs;
    - crypto → Crypto;
    - CS2 → Cs2;
    - shows → Shows;
    - otherwise None.
- Copy:
  - `previewTitle`: "Так будет на Главной" (caps eyebrow 22/600);
  - `previewHint`: "Изменения применяются сразу. Превью — это Главная с вашими обоями и виджетами." (22/450 at 62 %, at y 650, width 776).

- [ ] **Step 1: Write the failing tests**

```rust
fn previewed(state: &mut State) -> Scene { let mut scene = scene(state); view::home_preview(&mut scene, state, &assets(), UNIX_EPOCH); scene }

#[test]
fn general_and_widgets_show_home_small_beside_the_list() {
    for section in [0, WIDGETS_SECTION] {
        let mut state = settings(section);
        let scene = previewed(&mut state);
        let greeting = greeting_text(&state);
        let (rect, style) = text_in(&scene, &greeting).expect("Home's greeting in the preview");
        assert!((style.size - 76. * 776. / 1920.).abs() < 0.01);
        assert!(rect.x >= 1064. && rect.x + rect.w <= 1840. && rect.y >= 190. && rect.y <= 190. + 437.);
        assert!(scene.controls.iter().all(|c| c.rect.x < 1064.), "the preview takes no focus");
    }
    for section in [1, SYSTEM_SECTION] {
        let mut state = settings(section);
        assert!(text_in(&previewed(&mut state), &greeting_text(&state)).is_none());
    }
}

#[test]
fn the_preview_rings_the_part_the_focused_setting_changes() {
    let mut state = settings(0);
    state.focus = Target::Setting(1); // Clock
    let scene = previewed(&mut state);
    let clock = scene_region(&state, Region::Clock); // the Home scene's own rect, scaled: 1064 + x*k, 190 + y*k
    assert!(scene.draw.iter().any(|d| matches!(d, Draw::Shape(r, _, w, _) if *w >= 2. && r.contains(clock.x, clock.y) && r.contains(clock.x + clock.w, clock.y + clock.h))));
}

#[test]
fn system_summarises_luma_and_its_updates_on_the_right() {
    let mut state = settings(SYSTEM_SECTION);
    state.focus = Target::Sidebar(SYSTEM_SECTION);
    let drawn = texts(&state);
    let version = crate::i18n::Language::En.format("currentVersion", &[("current", "test")]);
    assert!(drawn.iter().any(|(t, r, _)| *t == version && r.x >= 1064.));
    let check = controls(&state).into_iter().find(|c| c.target == Target::CheckUpdates).unwrap();
    assert_eq!((check.rect.x, check.rect.y, check.rect.w), (196., 224., 820.));
}
```

- [ ] **Step 2: Run, verify they fail**

  Run: `cargo test --bin luma settings_layout`

  Expected: FAIL. `home_preview` and `Region` are not defined.

- [ ] **Step 3: Implement**
  - Regions, slot, `home_preview`, and the `compose` call, as in Interfaces.
  - `settings()` draws the eyebrow at (1064, 150), the frame shadow, and records the slot. After the slot it draws the 1 px `c(0xffffff, 0.14)` frame border (radius 18) and the hint.
  - System:
    - the summary panel at the preview's place: `c(0x0a0d12, 0.42)`, radius 22, eyebrow "Luma", status title 34/600, version line 24/450 at 62 %;
    - the list's first row is the Check/Install row (title `update_value(state)`), then the existing rows;
    - the old in-list status card goes.

- [ ] **Step 4: Run, verify they pass; whole suite**

  Run: `cargo test --bin luma > <workspace>/t3.log 2>&1; tail -5 <workspace>/t3.log`

  Expected: green. `system_version_is_inside_the_update_status_card…` and the other update-card tests are updated to the summary and ledgered.

- [ ] **Step 5: Commit** `feat(settings): Home, live and small, beside the list; System sums itself up`

### Task 4: The gallery on four 390 px columns

**Files:**

- Modify: `src/view.rs` (section 1 drawing, new `gallery_action`, `dashed_border`, the `BackgroundOptions` anchor height), `src/state.rs` (`gallery_slots`, `gallery_target`, `gallery_slot`, the Background/Upload/Gradient/Cycle routes, sidebar Right → `section_entry()`, `backgrounds_viewport`, reveal top, wheel max).
- Test: `src/settings-layout-tests.rs`, `src/wallpapers-ui-tests.rs`, `src/tests.rs` (updated)

**Interfaces:**

- Consumes: Task 1 gallery tokens, `settings_title(.., max_width)`.
- Produces:
  - `fn gallery_slots(&self) -> usize` (`background_count() + 1`);
  - `fn gallery_target(&self, slot: usize) -> Target` (`Background(slot)`, the last one is `UploadWallpaper`);
  - `fn gallery_slot(&self, target: &Target) -> Option<usize>`.
  - Slot `n` sits at `Rect::new(196. + (n % 4) * 418., 236. + (n / 4) * 251. - scroll, 390., 219.)`.
  - `backgrounds_viewport()` = `Rect::new(176., 226., 1684., content bottom − 226.)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_gallery_fills_four_columns_with_upload_last() {
    let state = settings(1);
    let c = controls(&state);
    let at = |t: Target| c.iter().find(|x| x.target == t).unwrap().rect;
    assert_eq!(at(Target::Background(0)), Rect::new(196., 236., 390., 219.));
    assert_eq!(at(Target::Background(1)).x, 614.);
    assert_eq!(at(Target::Background(4)).y, 487.);
    let n = state.background_count();
    assert_eq!(at(Target::UploadWallpaper), Rect::new(196. + (n % 4) as f32 * 418., 236. + (n / 4) as f32 * 251., 390., 219.));
    let cycle = at(Target::WallpaperCycle);
    assert_eq!(cycle.x + cycle.w, 1840.);
    assert!(at(Target::WallpaperGradient).x + at(Target::WallpaperGradient).w < cycle.x);
    assert!(cycle.y < 236. - 20.);
}

#[test]
fn the_remote_moves_on_the_four_column_grid() {
    let mut state = settings(1);
    let routes = [
        (Target::Background(0), Direction::Down, Target::Background(4)),
        (Target::Background(4), Direction::Up, Target::Background(0)),
        (Target::Background(0), Direction::Up, Target::WallpaperGradient),
        (Target::Background(3), Direction::Up, Target::WallpaperCycle),
        (Target::Background(4), Direction::Left, Target::Sidebar(1)),
        (Target::WallpaperGradient, Direction::Down, Target::Background(2)),
        (Target::WallpaperCycle, Direction::Down, Target::Background(3)),
    ];
    for (from, direction, to) in routes {
        state.focus = from.clone();
        let c = controls(&state);
        state.navigate(direction, &c);
        assert_eq!(state.focus, to, "{from:?} {direction:?}");
    }
    let last = Target::Background(state.background_count() - 1);
    state.focus = last;
    let c = controls(&state);
    state.navigate(Direction::Right, &c);
    assert_eq!(state.focus, Target::UploadWallpaper);
}

#[test]
fn the_sidebar_enters_the_gallery_on_the_applied_background() {
    let mut state = settings(1);
    state.focus = Target::Sidebar(1);
    let c = controls(&state);
    state.navigate(Direction::Right, &c);
    assert!(matches!(state.focus, Target::Background(i) if state.background_id(i) == Some(state.preferences.background.as_str())));
}
```

`navigate` here stands for the existing remote-direction entry point the other tests use. Use that same call.

- [ ] **Step 2: Run, verify they fail**

  Run: `cargo test --bin luma settings_layout`

  Expected: FAIL on the 3-column positions and the routes.

- [ ] **Step 3: Implement**
  - Title «Фон» via `settings_title(.., 1840 − actions_left − 24 − 196)`.
  - `gallery_action(out, state, assets, target, rect, icon, title, value)`:
    - a 56 px pill, radius 28, panel fill as the old row surface (white gradient when focused);
    - icon 24, title 24/550, value 24/450 at 62 %;
    - laid out right to left from 1840 with a 16 px gap, at y 148.
  - The grid is drawn between `Clip(Some(backgrounds_viewport()))` + `EdgeFade(Some([226., 246., 1040., 1080.]))` and the resets.
  - Tile chrome is unchanged:
    - card, border, check;
    - `aerials_caption` for Apple TV;
    - ring on focus.
  - The upload tile:
    - `dashed_border` (2 px dashes 14 long, 10 apart, `c(0xffffff, 0.45)`, straight edges only, outside the 16 px corners);
    - `upload` icon 32 centred, above the label `uploadWallpaper` at 24/550.
  - Routes as in the tests:
    - Right goes to the next slot, crossing rows;
    - Left goes to the previous slot within the row, else `Sidebar(1)`;
    - Up goes to `slot − 4`, else the Cycle (column 3) or the Gradient;
    - Down goes to `slot + 4`, else the last slot if another row exists.
    - Gradient: Down → `gallery_target(2.min(slots − 1))`, Left → `Sidebar(1)`, Right → Cycle, Up → `Nav(2)`.
    - Cycle: Down → `gallery_target(3.min(slots − 1))`, Left → Gradient, Up → `Nav(2)`.
  - `hit_test`, `wheel_scroll` and reveal (top `GALLERY_TOP`) treat `UploadWallpaper` like a background.

- [ ] **Step 4: Run, verify they pass; whole suite**

  Run: `cargo test --bin luma > <workspace>/t4.log 2>&1; tail -5 <workspace>/t4.log`

  Expected: green. The updated 3-column, upload-in-header and sidebar-to-upload tests are ledgered.

- [ ] **Step 5: Commit** `feat(settings): the gallery on four 390 px columns, upload as its last tile`

### After the tasks

- Render Settings General, Widgets › Weather, System and Background with the scratch `render.sh`, and compare them with the mockups.
- Build the IPK, install it on the TV (`ares` profile `lgtv`), and push to `fork integration`. The user checks.
