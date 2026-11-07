# TV layout, phase 5: glass — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Luma's chrome is frosted glass over the wallpaper. There is no per-frame blur: each wallpaper gets one blurred 480 × 270 copy, made once when it loads.

**Architecture:**

- A new draw, `Draw::Glass`, paints a rounded rect with the wallpaper's blurred copy sampled at the panel's own screen position. It uses the existing paint shader: a textured quad with a radius.
- The blurred copy is a pure function, `glass::frost`. The renderer runs it on the decoded pixels it already holds when a full-screen wallpaper texture loads. This covers bundled, uploaded and dynamic wallpapers and Apple TV stills, once per frame or still.
- The view gets one helper, `Scene::glass`, that lays the copy (both copies while wallpapers cross-fade) under the spec's tint, border and highlight. Every chrome surface that today paints `c(0x171c20, 0.72)`-style fills calls it.

**Tech Stack:** Rust, GLES2 (webOS), `image` 0.25.

**Spec:** `docs/superpowers/specs/2026-11-01-tv-layout-design.md`, sections "Материалы", "Оверлеи" and "Читаемость на обоях"; phase 5 of "Порядок внедрения".

## Global Constraints

- No per-frame blur and no video: one blurred copy per wallpaper, frame or still, made once.
- The copy is 480 × 270.
- Panel stack: the copy, then tint `rgba(18,22,30,.46)`, then a 1 px border `rgba(255,255,255,.10)`, then a 1 px top highlight `rgba(255,255,255,.16)`.
- During a wallpaper cross-fade, both copies are blended, as the wallpapers are.
- Glass goes on: the top bar, the collapsed sidebar, «Сегодня», input and crypto chips, action buttons and overlays. App cards stay as they are.
- Text on wallpaper always has a shadow; text on glass has none.
- Commit as `eluz1ve <8470376+eluz1ve@users.noreply.github.com>`. Push only with `git push fork HEAD:integration`.

## Review Focus

1. **4K mode** (`@2x` wallpapers, 3840 × 2160). The copy is still 480 × 270, and glass samples it at the same screen place.
2. **The scaled Home inside the Settings preview.** Glass there samples the wallpaper as drawn in the frame, not the screen behind the frame.
3. **A wallpaper whose texture failed to load** (missing file). Glass draws nothing and does not crash; the tint alone stays.
4. **Partial redraw (damage).** A glass panel redraws only its own rect. Moving it, as the Home page slides, redraws old and new rects.
5. **Texture budget eviction.** A copy evicted with an old wallpaper is rebuilt when that wallpaper comes back.

---

### Task 1: `Draw::Glass` and the blurred copy

**Files:**

- Create: `src/glass.rs` (`frost`), `src/glass-tests.rs`
- Modify: `src/main.rs` (mod), `src/renderer.rs` (`Draw::Glass`, copy built on wallpaper load, `glass()` quad, `profile_skip`), `src/damage.rs` (`bounds`, `same`), `src/view.rs` (`Scene::slide`, `Scene::fade`, `home_preview` arm)

**Interfaces:**

- Produces:
  - `pub fn frost(image: &image::RgbaImage) -> image::RgbaImage`, which always returns 480 × 270.
  - `Draw::Glass(String, Rect, f32, Rect, Color)`, in this order:
    - the wallpaper's path, as its `Image` draw names it;
    - the panel;
    - the corner radius;
    - where the whole wallpaper is laid out (the screen, or the preview frame);
    - a tint, where rgb scales brightness and a is opacity.

- [ ] **Step 1: Write the failing tests**
  - `glass-tests.rs` → `frost_makes_one_small_soft_copy`:
    - Input: a 1920 × 1080 image, black for x < 960 and white elsewhere.
    - The output is 480 × 270.
    - Pixel (10, 135) < 8; pixel (470, 135) > 247.
    - Pixel (240, 135) is within 128 ± 24.
    - Along row 135, the luminance rises monotonically from 200 to 280. The band from 10 % to 90 % is wider than 12 px.
  - `glass-tests.rs` → `frost_takes_any_wallpaper_size`: 3840 × 2160 and 1001 × 563 inputs both give 480 × 270, and a uniform input colour stays that colour (±1).
  - `damage` → `glass_damages_its_panel`:
    - `bounds(&Draw::Glass(p, r, 18., screen, tint))` equals `r.inset(-1.)`.
    - Two equal Glass draws are `same`; a moved one is not.
  - `view` → `glass_slides_and_fades_with_the_page`:
    - `Scene::slide` moves the panel rect but not the source rect.
    - `Scene::fade(.., 0.5, 0.8)` multiplies the tint to `[0.8, 0.8, 0.8, 0.5]`.

- [ ] **Step 2: Run, verify they fail.** Run `cargo test --bin luma glass`. Expected: FAIL (no `glass` module, no `Draw::Glass`).

- [ ] **Step 3: Implement**
  - `frost`:
    1. Box-average down to 480 × 270. Each output pixel averages the input pixels its cell covers, using integer cell bounds.
    2. Run three horizontal-plus-vertical box-blur passes of radius 8, clamping at the edges.
  - Renderer:
    - When `folded_image` loads a texture for a full-screen, non-`contain` rect, it builds `frost` from the same decoded pixels. It stores the copy in `textures` under `"{path}#glass"`, with the same eviction.
    - `glass(path, rect, radius, source, tint)` looks the copy up, building it by decoding `path` when it is missing (evicted, or the wallpaper was never drawn full screen).
    - It then draws `quad(rect, uv, plain_radius(radius), 0., tint, id)`, where `uv` = `(rect − source.xy) / source.wh`.
    - Missing copy: draw nothing.
    - `profile_skip` label: `"glass"`.
  - `home_preview` maps Glass: `rect` → `place(rect)`, `source` → `place(source)`, `radius` → `radius * k`.

- [ ] **Step 4: Run, verify they pass; whole suite.** Run `cargo test --bin luma`. Expected: green.

- [ ] **Step 5: Commit** `feat(renderer): glass, a panel over one blurred copy of the wallpaper`

### Task 2: Chrome on glass

**Files:**

- Modify: `src/view.rs`
- Test: `src/glass-tests.rs` (view part), existing tests whose fill colours change

**Interfaces:**

- Consumes: `Draw::Glass`.
- Produces:
  - `Scene.backdrop: Vec<(String, f32)>` (wallpaper path and its opacity), set by `scene_with_pill` before anything else is drawn.
  - `Scene::glass(&mut self, rect: Rect, radius: f32)` and `Scene::modal_glass(&mut self, rect: Rect, radius: f32)`.

- [ ] **Step 1: Write the failing tests** (all in `glass-tests.rs`, view part)
  - `chrome_sits_on_glass_over_the_wallpaper`:
    - Home with two inputs, crypto on and «Сегодня» showing has a `Glass` draw exactly at each of: the tab bar surface, the TV button, the «Сегодня» card, every input chip and every coin chip.
    - Each such draw's path equals the first `Image` draw's path, and its source is (0, 0, 1920, 1080).
  - `settings_chrome_sits_on_glass`:
    - Settings General has Glass at the sidebar panel and at every row.
    - Background has Glass at both switches.
  - `overlays_sit_on_glass`: an open app options overlay has Glass at its panel (`scene.modal`).
  - `glass_cross_fades_with_the_wallpaper`:
    - With `previous_background` set, each surface gets two Glass draws.
    - Their paths are the two wallpapers', with alphas 1 and the new image's fade alpha (equal to its `TintedImage` alpha).
  - `text_on_glass_has_no_shadow`: tab labels, «Сегодня» lines, chip names and row titles all have `style.shadow == false`.
  - `the_preview_samples_its_own_wallpaper`: in Settings General, every Glass inside `PREVIEW` has source == `PREVIEW`.

- [ ] **Step 2: Run, verify they fail.** Run `cargo test --bin luma glass`. Expected: FAIL (no Glass draws).

- [ ] **Step 3: Implement**
  - `Scene::glass` pushes, in order:
    1. one `Draw::Glass(path, rect, radius, SCREEN, [1, 1, 1, alpha])` per backdrop layer;
    2. `shape(rect, radius, c(0x12161e, 0.46))`;
    3. `border(rect, radius, 1., c(0xffffff, 0.10))`;
    4. the highlight, `shape(Rect::new(rect.x + radius, rect.y, rect.w − 2·radius, 1.), 0.5, c(0xffffff, 0.16))`.
  - `Scene::modal_glass` is the same, but its tint is `RoundedGradient(rect, radius, c(0x34383b, 0.86), c(0x24282d, 0.90))` and its border is 0.12. Overlays stay readable over a bright blurred wallpaper.
  - Callers (each replaces its fill and border):
    - the header nav surface;
    - `pill()` when neither focused nor selected (the TV button, Apps filters, overlay buttons);
    - `settings_surface` (rows, gallery switches);
    - the sidebar panel;
    - the System summary panel;
    - `today_card`;
    - input chips and coin chips, unfocused;
    - the Apps search field and the keyboard's field, unfocused;
    - `toast`.
  - Focused chips keep their lighter look: glass, then `c(0x343a42, 0.50)`, then a border at 0.24.
  - `modal_glass`: companion, options, delete confirmation, keyboard, CS2 match.
  - Keep the shadows that already sit under these surfaces.

- [ ] **Step 4: Run, verify they pass; whole suite.** Run `cargo test --bin luma`. Expected: green. Old tests that pinned the old fills are updated and ledgered.

- [ ] **Step 5: Commit** `feat(view): chrome and overlays on glass`

### After the tasks

- Render Home, Apps, Settings General and Background, and an options overlay, then compare against the spec's materials.
- Build the IPK and install on the TV (profile `lgtv`, 192.168.0.237). Push to `fork integration`. The user checks.
