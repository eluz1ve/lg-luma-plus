# Side menu, phase 2: Home with one widget in the center — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Home's rows below the favorites go away. The favorites move to the bottom of the screen, and at most one widget stands in the middle.

**Architecture:**

- `Preferences` gains `center()` and `in_today()`. Every caller asks these two instead of reading the three place flags (`cs2_today`, `shows_today`, `crypto_beside`) directly.
  - `center() -> Option<Center>` picks one widget, CS2 → crypto → shows, among those shown and not placed in Today.
  - `in_today(Center) -> bool` puts every other shown widget in Today.
- Setting a widget's place to "В центре" moves the widget that was there into Today.
- `view::home` draws the header, the center widget, then the favorites at a fixed top, `FAVORITES_TOP`.

**Tech Stack:** Rust.

**Spec:** `docs/superpowers/specs/2026-11-03-side-menu-design.md` (sections «Компоновка Главной», «Мои решения»; phase 2).

## Global Constraints

- **Favorites:**
  - cards 260 × 150 at a pitch of 300;
  - the cards' bottom is 60 + 40 px above the screen's bottom, which leaves room for the focused card's name: cards at y 830, heading at 780.
- **Center:** the widget block (heading and cards) is centered between `HEADER_BOTTOM` (400) and the favorites heading (780).
- **Show posters in the center:** 170 × 255 at a pitch of 210.
- **No inputs row on Home.** Inputs are in the menu.
- **Focus:**
  - Up from the favorites goes to the nearest card of the center, else Today's last line.
  - Down from the center goes to the nearest favorite.
  - Down from Today's last line goes to the center, else to the favorites.
  - Left from the center's first card opens the menu.
- **Copy:** `placeRow` becomes "В центре" (in every language).
- **Commits:** as `eluz1ve <8470376+eluz1ve@users.noreply.github.com>`, with the Co-Authored-By line. Push only with `git push fork HEAD:integration`.

## Review Focus

1. **Saved preferences with several widgets outside Today** (legacy rows). Only the first by order is in the center; the rest show in Today.
2. **Crypto in the center with no coins chosen.** Nothing is in the center; the next widget by order takes it.
3. **The Settings preview** shows the new layout scaled, with its regions highlighted where the widget now is.
4. **No favorites (empty state) with a widget in the center.** The empty state takes the favorites' place at the bottom.

### Task 1: The center, in preferences and settings

**Files:** `src/model.rs`, `src/today.rs`, `src/state.rs`, `assets/messages.json`. Tests go in `src/home-center-tests.rs`.

Tests:

- `one_widget_takes_the_center_cs2_then_crypto_then_shows`
- `placing_a_widget_in_the_center_moves_the_one_there_into_today`

### Task 2: The layout and its routes

**Files:** `src/view.rs` (`home`, `shows_row`) and `src/state.rs` (Home's routes).

Tests:

- `favorites_sit_at_the_bottom_and_home_has_no_inputs`
- `the_center_widget_sits_between_the_header_and_the_favorites`
- `up_from_the_favorites_reaches_the_center_then_today_and_down_returns`
- `left_from_the_first_card_in_the_center_opens_the_menu`
- `shows_in_the_center_have_smaller_posters`

Verify: `cargo test --bin luma`, clippy, a desktop render, then the TV.
