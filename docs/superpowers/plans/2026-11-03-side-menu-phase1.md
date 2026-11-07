# Side menu, phase 1: the menu replaces the tabs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The tab bar and the TV-settings button leave the screen. A glass menu on the left takes their place, opened by Home and by Left at the left edge.

**Architecture:**

- The menu is a modal overlay, `Overlay::Menu { restore }`. Its items reuse `Target::Nav(0..=2)` (Главная, Приложения, Настройки), `Target::Input(id)` and `Target::TvSettings`.
- One new target, `Target::MenuInputs`, folds the inputs list open and closed (`State::menu_inputs`).
- The slide is time-based, `State::menu_motion: Option<(Instant, bool)>`, read through `State::menu_amount() -> f32` (0 is closed, 1 is open). The view shifts the page right by `56 × amount`, dims it by `0.5 × amount`, and slides the panel in from x = −430.

**Tech Stack:** Rust, SDL2, GLES2 (webOS).

**Spec:** `docs/superpowers/specs/2026-11-03-side-menu-design.md` (sections «Меню», «Мои решения»; phase 1 of «Порядок внедрения»).

## Global Constraints

- **Panel:** glass, `Rect(32, 32, 430, 1016)`, radius 36.
  - Items are 76 high with radius 20; the focused one is a white pill with dark text.
  - Input rows are 64 high, indented by the icon's width.
  - The clock sits at the top of the panel.
- **Item order:** Главная, Приложения, Входы, then the inputs when unfolded; at the bottom, Настройки and Настройки ТВ.
- **Motion:**
  - opens in 0.25 s with ease-out, closes in 0.2 s;
  - the page shifts 56 px right and dims to 50 %;
  - reduced motion makes it instant.
- **Home key:**
  - with Luma in front, it toggles the menu over the current page;
  - a relaunch from another app runs `return_home()` and then opens the menu on Главная.
- **Left opens the menu** wherever nothing lies to the left. Up never leads to a tab any more.
- **Closing the menu:** Back, Right and Home close it and restore the focus.
  - OK on the current page's item also closes it.
  - OK on another page's item opens that page at its first content.
  - OK on an input switches to it; OK on Настройки ТВ launches the TV settings. Both close the menu.
- **Back at a page's top** (the Settings sidebar, the Apps search) goes Home, as Back from a non-Home page does now.
- **Commits:** as `eluz1ve <8470376+eluz1ve@users.noreply.github.com>`, with the Co-Authored-By line. Push only with `git push fork HEAD:integration`.

## Review Focus

1. **Home while an overlay is open** (options, keyboard, a CS2 match, Recents). The overlay closes, text drafts are committed, and the menu opens.
2. **Pointer:** a click on a menu item works like OK, and a click outside the panel closes the menu.
3. **A page with no content** (no apps yet). Left still opens the menu, and closing it lands on `first_content()`.
4. **The Settings preview** renders the scaled Home without a tab bar or menu.
5. **Inputs hidden in Settings › Inputs** are not listed in the menu.

---

### Task 1: Menu state and keys

**Files:**

- Modify: `src/state.rs`
- Create: `src/menu-tests.rs`, wired from `src/tests.rs` with `#[path = "menu-tests.rs"] mod menu_tests;`

**Interfaces:**

- Produces:
  - `Overlay::Menu { restore: Target }` and `Target::MenuInputs`;
  - the fields `State::menu_inputs: bool` and `State::menu_motion: Option<(Instant, bool)>`;
  - `pub fn menu_items(&self) -> Vec<Target>`, in drawing order;
  - `pub fn open_menu(&mut self)` (focus on `Nav(page_index())`) and `pub fn close_menu(&mut self)`;
  - `pub fn menu_amount(&self) -> f32`, `pub fn home_key(&mut self) -> Vec<Effect>` and `pub fn relaunch_home(&mut self) -> Vec<Effect>`.

Tests (each written first, then watched failing):

- `home_opens_the_menu_on_the_current_page_and_home_again_closes_it`
- `the_menu_lists_pages_then_visible_inputs_when_unfolded_then_settings`
- `ok_on_another_page_opens_it_at_its_first_content`
- `ok_on_an_input_switches_to_it_and_closes_the_menu`
- `left_from_the_first_favorite_opens_the_menu`
- `left_from_the_settings_sidebar_opens_the_menu`
- `back_and_right_close_the_menu_and_restore_the_focus`
- `a_relaunch_returns_home_with_the_menu_open`
- `up_from_the_favorites_without_today_stays`

Run: `cargo test --bin luma menu_tests`. Then the whole suite: the tests that expected `Nav` after Up or Back get updated to the new routes.

### Task 2: Menu view, without the tab bar

**Files:**

- Modify:
  - `src/view.rs`: drop `navigation()` and the pill and TV button from `header()`; draw the menu;
  - `src/main.rs`: drop the pill's animation;
  - test helpers that call `scene_with_pill` or `navigation`.
- Create: `assets/ui-icons/home.svg` (Lucide house, stroke 1.65).

**Interfaces:**

- Produces:
  - `pub fn scene(state: &State, assets: &Assets, now: OffsetDateTime) -> Scene`, which replaces `scene_with_pill`;
  - `fn menu(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime)`;
  - `Scene::shift_x(draws, controls, dx)`.

Tests:

- `the_menu_draws_its_items_on_glass_over_the_dimmed_page`
- `no_tab_bar_is_drawn`
- `menu_items_are_clickable`

Run: `cargo test --bin luma`, then clippy.

### Task 3: Home key and relaunch, on the TV

**Files:** Modify `src/main.rs`: the Home keycode calls `state.home_key()`, and the lifecycle relaunch calls `state.relaunch_home()`. The relaunch calls `home_key()` instead when Luma is already focused and visible.

Verify: build, install on the TV, launch. The user checks the menu.
