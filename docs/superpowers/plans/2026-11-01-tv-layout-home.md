# Главная по рамке tvOS и карточка «Сегодня»: план внедрения

> **Для агентов:** обязательный навык: superpowers:subagent-driven-development (рекомендуется) или
> superpowers:executing-plans, задача за задачей. Шаги отмечены чекбоксами (`- [ ]`).

**Цель:** всё, что включено на Главной, помещается на один экран 1920 × 1080. Шапка на 76 px, справа
карточка «Сегодня» (живые матчи CS2, серии на сегодня, криптовалюта рядом).

**Архитектура:** размеры берутся из нового модуля `src/layout.rs`: безопасная зона, сетка 260/40, шкала
текста. Строки «Сегодня» собирает чистая функция в `src/today.rs`, у неё отдельные тесты. `view.rs`
только рисует эти строки, а `state.rs` обрабатывает фокус и действия. Навигация пультом уже
пространственная, по прямоугольникам controls, поэтому новых маршрутов фокуса не нужно: она следует за
геометрией.

**Стек:** Rust, свой GL-рендерер Luma, `time` 0.3, `serde_json`. Тесты: `cargo test --bin luma`.

**Спека:** `docs/superpowers/specs/2026-11-01-tv-layout-design.md`. Это шаги 1–2 из её «Порядка
внедрения». Приложения, Настройки и стекло пойдут отдельными планами после проверки этого шага на ТВ.
Панель, уезжающая при переполнении Главной, — общий хром, она в плане Приложений.

## Глобальные ограничения

- Только форк: коммиты от eluz1ve, `git push fork HEAD:integration`. В upstream ничего не отправляется.
- Ключ PandaScore не пишется ни в какой файл, коммит или сборку.
- Безопасная зона: 80 px по бокам, 60 сверху и снизу. Сетка: колонки фиксированной ширины с зазором 40
  (6 → 260, 4 → 410, 3 → 560).
- Текст: приветствие 76/600, заголовок ряда 28/600, основной 26/550, вторичный 24/450 (62 %), подписи и
  описания 22/450, надстрочник (дата) 22/600 капс.
- Текст прямо на обоях — всегда `.shadow()`. Текст на карточке — без тени.
- «Масштаб элементов интерфейса» (`ui_scale`) масштабирует то же, что и сейчас: часы и текст входов.
- Никакого покадрового размытия и видео. Сетевой запрос добавляется только один: TVmaze на завтра.
- Каждая новая строка интерфейса добавляется в `assets/messages.json` на всех 13 языках.
- При «Меньше анимации» все переходы мгновенные.
- Проверка: `cargo test --bin luma` (около 6 с). Длинные smoke-прогоны не запускаем. В конце сборка и
  установка на ТВ, проверяет пользователь.

## Что проверить в ревью

1. **Длинное приветствие** (немецкий или русский с именем) при карточке «Сегодня» ужимается в левую
   колонку (80…1120) и не налезает на карточку. Тест в Task 1.
2. **Нет данных** (лента CS2 устарела или пуста, сериалов на сегодня нет, монеты грузятся): карточка
   скрыта, «Избранное» стоит на том же y, что и с карточкой. Тест в Task 4.
3. **Часовой пояс:** серия выходит сегодня по местному времени, но в другую дату UTC (вечер при UTC−5).
   Она всё равно попадает в «Сегодня». Тест в Task 2.
4. **Фокус:** «вверх» с первой карточки избранного ведёт к строкам «Сегодня», если они есть, иначе к
   панели. Если матч в фокусе пропал из ленты, фокус переходит на первую карточку избранного и ничего не
   падает. Тест в Task 4.
5. **Подпись в фокусе** у карточки избранного не налезает на заголовок «Входы», а увеличение карточки у
   x = 80 не обрезается. Тест в Task 1.

---

### Task 1: рамка и сетка Главной

**Файлы:**

- Создать: `src/layout.rs` (зарегистрировать `mod layout;` в `src/main.rs`).
- Изменить: `src/view.rs`: `home()`, `tile()`, `crypto_row()`, `cs2_row()`, `shows_row()`,
  `weather_summary*`.
- Изменить: `src/state.rs`: `HOME_STRIP_LEFT/RIGHT`, `HOME_TILE_WIDTH/PITCH`, `reveal_focus` (верх 120.8).
- Тесты: `src/tests.rs`, `src/crypto-ui-tests.rs`, `src/cs2-ui-tests.rs`. Обновить координаты в
  существующих тестах геометрии Главной и добавить новые.

**Интерфейсы:**

- Даёт: `layout::{SAFE_X = 80., SAFE_Y = 60., GUTTER = 40., CONTENT_TOP = 150., HEADER_BOTTOM = 400.}`,
  `layout::column(count: usize) -> f32` (1760 − 40·(n−1)) / n,
  `layout::type_scale::{GREETING = 76., ROW_TITLE = 28., BODY = 26., SECONDARY = 24., CAPTION = 22.,
  EYEBROW = 22.}`.
- Даёт: `TODAY_RECT: Rect = Rect::new(1160., 150., 680., 250.)` в `view.rs`, это область карточки в
  задаче 4. Левая колонка шапки кончается на `TODAY_RECT.x - GUTTER` (1120), когда карточка показана,
  и на `1840.`, когда её нет.

Раскладка (y при `scroll = 0`):

| Что                        | Геометрия                                                                                       |
| -------------------------- | ----------------------------------------------------------------------------------------------- |
| Шапка, левая колонка       | дата 22/600, приветствие 76/600, погода 28. Блок центрируется по вертикали в 150…400            |
| «Избранное»                | заголовок y 416 (28/600), карточки y 466, 260 × 150, шаг 300, полоса 80…1840, затухание до 1910 |
| Подпись                    | только у карточки в фокусе, когда `show_home_names`; под ряд резервируется 40 px                |
| Следующий ряд              | `низ ряда + 48`; заголовок → чипы 50                                                            |
| Входы, крипто-ряд          | чипы 410 × 96, шаг 450, 4 в ряд                                                                 |
| CS2, сериалы (когда рядом) | карточки по сетке: CS2 3 × 560 × 150, постеры 260 × 390                                         |

- [ ] **Шаг 1: падающие тесты.**
      `layout_columns_follow_the_tvos_grid`: `column(6) == 260.`, `column(4) == 410.`, `column(3) == 560.`,
      `column(2) == 860.`.
      `home_fits_one_screen_with_weather_six_favorites_two_inputs_and_four_coins`: при погоде, 6 избранных,
      2 входах и 4 монетах рядом нижний край `max(control.rect.y + h)` не больше 1020, у первой плитки
      `rect == (80, 466, 260, 150)`, у второй `x == 380`, ширина чипа входа 410.
      `focused_favorite_alone_shows_its_name`: при `show_home_names` текст подписи есть только у
      `Target::App` в фокусе, а заголовок «Входы» стоит ниже `466 + 150 + 40`.
      `long_greeting_shrinks_to_the_left_column_beside_today`: приветствие с именем на 40 символов при
      показанной карточке «Сегодня» рисуется не шире 1040 px.
- [ ] **Шаг 2:** `cargo test --bin luma`. Новые тесты падают, остальные зелёные.
- [ ] **Шаг 3:** `src/layout.rs` с константами выше. `home()` переписан под таблицу. В `tile()` подпись
      рисуется только при `focused`, хит-бокс не растёт на подпись (резерв делает `home()`). Константы
      `state.rs`: `HOME_STRIP_LEFT = 80.`, `HOME_STRIP_RIGHT = 1840.`, `HOME_TILE_WIDTH = 260.`,
      `HOME_TILE_PITCH = 300.`. Заголовки рядов: `text(ROW_TITLE, 600, -0.02).shadow()`. Блок погоды и дата
      берут размеры из `type_scale`.
- [ ] **Шаг 4:** `cargo test --bin luma`. Все тесты зелёные, координаты старых тестов Главной обновлены
      под таблицу.
- [ ] **Шаг 5:** коммит `feat(home): tvOS frame — 80 px safe area, 260/40 grid, 76 px greeting`.

### Task 2: серии на сегодня из расписания TVmaze

**Файлы:**

- Изменить: `src/shows.rs` (`Show`, `show()`, `parse_schedules`), `src/shows-service.rs` (запрос на
  завтра).
- Тесты: `src/shows-tests.rs`.

**Интерфейсы:**

- Даёт: `pub struct Episode { pub airs: i64, pub season: Option<u32>, pub number: Option<u32> }` и поле
  `Show::episodes: Vec<Episode>`. Это все записи расписания этого сериала, без повторов, по
  возрастанию `airs`, не больше 8.
- Даёт: `Show::airing(&self, from: i64, to: i64) -> Option<&Episode>`, первая серия с
  `from <= airs < to`.

- [ ] **Шаг 1: падающие тесты.**
      `schedule_entries_keep_each_episodes_air_time`: две записи одного сериала с
      `"airstamp": "2026-11-01T21:00:00+00:00", "season": 2, "number": 5` и следующей серией дают
      `episodes.len() == 2`, у первой `airs == 1793566800`, `season == Some(2)`, `number == Some(5)`.
      `an_episode_airing_this_local_evening_counts_even_on_the_next_utc_date`: при `airs` в
      `2026-11-02T01:00Z` и окне местного дня 1 ноября для UTC−5 `airing(from, to)` возвращает `Some`.
      Запись без `airstamp` даёт сериал с пустым `episodes`, и сериал не теряется.
- [ ] **Шаг 2:** `cargo test --bin luma shows`. Тесты падают.
- [ ] **Шаг 3:** `airstamp` разбирается через `time::format_description::well_known::Rfc3339`.
      `season` и `number` берутся из записи расписания: верхний уровень `entry`, у `/schedule/web` это тоже
      верхний уровень. В `Client::shows` к 7 дням `schedule/web` добавляется день `today + 1`.
- [ ] **Шаг 4:** `cargo test --bin luma`. Всё зелёное.
- [ ] **Шаг 5:** коммит `feat(shows): keep each episode's air time from TVmaze's schedule`.

### Task 3: что попадает в «Сегодня»

**Файлы:**

- Создать: `src/today.rs` (`mod today;` в `main.rs`), `src/today-tests.rs`.
- Изменить: `src/model.rs`: `cs2_today: bool` и `shows_today: bool`, по умолчанию `true`, через
  `#[serde(default = ...)]`, чтобы старые настройки грузились. `crypto_beside` остаётся и означает
  «криптовалюта в „Сегодня“».

**Интерфейсы:**

- Даёт:
  ```rust
  pub enum Line { Match { id: String }, Episode { show: i64, airs: i64 }, Crypto }
  pub struct Today { pub lines: Vec<Line>, pub more: usize }
  pub fn today(state: &State, now: i64, day: (i64, i64)) -> Today
  ```
  Здесь `day` — начало и конец местного дня в unix-секундах. Их считает вызывающий из `OffsetDateTime`
  с местным смещением.
- Правила (моё решение, в спеке не задано):
  - CS2 при `show_cs2 && cs2_today`: все `State::Live`, затем `Upcoming` с `begins < now + 12 ч`. Список
    берётся из `cs2::shown(feed, cs2_major_only, false)`.
  - Сериалы при `show_shows && shows_today`: сериалы с `app.is_some()` и `airing(day.0, day.1)`.
  - Порядок: живые матчи, потом будущие события по возрастанию времени, потом вышедшие сегодня серии,
    свежие первыми.
  - `Crypto` при `show_crypto && crypto_beside && !coins.is_empty()` всегда последней строкой и всегда
    помещается.
  - Всего строк не больше 3. Что не влезло, считается в `more`.
- Виджет с `*_today == true` не рисует свой ряд ниже, даже когда строк нет.

- [ ] **Шаг 1: падающие тесты** в `today-tests.rs`:
      `live_matches_come_first_then_events_by_time_then_aired_episodes`;
      `crypto_beside_is_always_the_last_line_and_the_rest_counts_as_more`: 3 live-матча и крипто дают
      `[Match, Match, Crypto]` и `more == 1`;
      `widgets_kept_as_rows_or_switched_off_add_nothing`;
      `episodes_need_an_installed_app_and_todays_air_time`;
      `old_preferences_without_today_flags_put_cs2_and_shows_in_today`: десериализация JSON без полей даёт
      `true`.
- [ ] **Шаг 2:** `cargo test --bin luma today`. Тесты падают.
- [ ] **Шаг 3:** реализовать `today()` и поля настроек. В `home()` пропускать `cs2_row` и `shows_row`,
      когда `cs2_today` / `shows_today` включены, а `crypto_block` убрать (его место займёт карточка в
      задаче 4).
- [ ] **Шаг 4:** `cargo test --bin luma`. Тест `crypto_can_sit_beside_the_greeting_…` переписан под
      крипто-строку в «Сегодня».
- [ ] **Шаг 5:** коммит `feat(home): choose what the Today card shows`.

### Task 4: карточка «Сегодня»

**Файлы:**

- Изменить: `src/view.rs`: новая `today_card(out, state, assets, now)`, вызов из `home()`, удалить
  `crypto_block`, `CRYPTO_BLOCK_*`, `crypto_block_shown`.
- Изменить: `src/state.rs`: `Target::TodayMatch(String)`, `Target::TodayShow(i64)` и их обработка в
  `activate`.
- Изменить: `assets/messages.json`: `today` («Сегодня»), `todayMore` («ещё {count}»), `aired` («вышла»).
- Тесты: `src/tests.rs` или новый `src/today-ui-tests.rs`.

**Интерфейсы:**

- Берёт: `today::today`, `TODAY_RECT`, `Show::airing`.
- Вид: карточка в стиле нынешних чипов входов (`RoundedGradient` `0x20252b/.60` → `0x171c22/.55`, рамка
  `0xffffff/.063`, радиус 24). Стекло будет в плане 5. Высота `20·2 + 28 + 58·n`, центр по вертикали в
  150…400. Надстрочник «СЕГОДНЯ · ещё N» 20/600 капс, 60 %.
- Строки высотой 58, слева иконка 40 × 40, основной текст 26/550, справа вторичный 22/450 62 %:
  - матч: «NAVI 1–0 FaZe», справа «LIVE · Mirage» или «21:00»;
  - серия: «Severance — S2E5», справа «Apple TV · 21:00» или «Apple TV · вышла»;
  - крипто: монеты подряд «BTC 77.9K ▲0.4%», без фокуса. Новая
    `crypto::format_compact(price: f64) -> String`: 77881.97 → «77.9K», 2262.98 → «2,263», 94.3 →
    «94.30».
- Действия: `TodayMatch(id)` открывает `Overlay::Cs2Match { id, restore: TodayMatch(id) }`.
  `TodayShow(id)` запускает `show.app`, без приложения показывает те же уведомления, что `Show(index)`.
- Если строк нет, карточку не рисовать. Высота шапки от этого не меняется.

- [ ] **Шаг 1: падающие тесты.**
      `today_card_lists_its_lines_and_opens_the_match_or_the_app`.
      `no_today_lines_hide_the_card_without_moving_favorites`: y первой плитки одинаков с карточкой и без.
      `up_from_the_first_favorite_reaches_today_then_the_tabs`.
      `a_focused_match_that_leaves_the_feed_returns_focus_to_favorites`.
      `format_compact_keeps_four_coins_on_one_line`: 4 монеты в 640 px.
- [ ] **Шаг 2:** `cargo test --bin luma`. Тесты падают.
- [ ] **Шаг 3:** реализовать. Фокус, который ссылается на пропавшую строку, исправлять там же, где
      сейчас исправляется пропавший `Cs2Match(index)`, иначе в `reveal_focus` / `first_content`.
- [ ] **Шаг 4:** `cargo test --bin luma`. Всё зелёное.
- [ ] **Шаг 5:** коммит `feat(home): Today card beside the greeting`.

### Task 5: настройки размещения и проверка на ТВ

**Файлы:**

- Изменить: `src/view.rs`: в `cs2_settings` и `shows_settings` строка «Показывать: В „Сегодня“ / Рядом»,
  в `crypto_settings` подпись `CryptoLayout`. Строку `CryptoCard` скрыть: карточка «Сегодня» всегда на
  подложке.
- Изменить: `src/state.rs`: `Target::Cs2Layout`, `Target::ShowsLayout` переключают `cs2_today` и
  `shows_today` и сохраняют.
- Изменить: `assets/messages.json`: `placeToday` («В „Сегодня“»), `placeRow` («Отдельным рядом»).
  `cryptoBeside` переименовать в «В „Сегодня“».

- [ ] **Шаг 1: падающий тест** `cs2_and_shows_move_between_today_and_their_rows_from_widgets`.
- [ ] **Шаг 2:** реализовать. `cargo test --bin luma`, всё зелёное. `cargo fmt`.
- [ ] **Шаг 3:** снимки Главной (`target/debug/luma --screenshot`) на светлом и тёмном кадре Apple, с
      карточкой «Сегодня» и без неё. Отправить пользователю.
- [ ] **Шаг 4:** сборка и установка: `pnpm build && node tools/package.mjs && ares-install -d lgtv
  dist/com.ecklf.luma_*_arm.ipk && ares-launch -d lgtv com.ecklf.luma`, затем
      `git push fork HEAD:integration`.
- [ ] **Шаг 5:** коммит `feat(widgets): put CS2 and TV Shows in Today or their own rows`.
