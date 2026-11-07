# Приложения по рамке tvOS: план внедрения

> **Для агентов:** обязательный навык: superpowers:subagent-driven-development (рекомендуется) или
> superpowers:executing-plans, задача за задачей. Шаги отмечены чекбоксами (`- [ ]`).

**Цель:** на странице Приложений помещаются 4 ряда по 6 (24 приложения вместо 18). Подпись есть только у
карточки в фокусе. Верхняя панель и фильтры уезжают вверх вместе с прокруткой, а не висят поверх карточек.

**Архитектура:** размеры берутся из `src/layout.rs` (Главная уже на нём). Режим подписей становится
перечислением в `model.rs` и переносит старое `show_app_names: bool`. «Уезжающий хром» — один сдвиг
`chrome_offset()` в `State`: его применяют `header()` и панель фильтров, а хит-тест его учитывает. Плавная
вертикальная прокрутка и подъём карточки в фокусе берутся из общего `motion.rs` (`Lane::Page`,
`Motion::focus`). Своей анимации у страницы нет.

**Стек:** Rust, свой GL-рендерер Luma, `serde`. Тесты: `cargo test --bin luma`.

**Спека:** `docs/superpowers/specs/2026-11-01-tv-layout-design.md`, шаг 3 «Порядка внедрения», раздел
«Приложения». Настройки и стекло пойдут отдельными планами после проверки этого шага на ТВ.

## Глобальные ограничения

- Только форк: `git push fork HEAD:integration`. В upstream и в PR автора ничего не отправляется.
- Безопасная зона 80/60, сетка 6 × 260, зазор 40 (`layout::column(6)`).
- Текст: основной 26/550, вторичный 24/450 (62 %), подписи 22/450. Вкладки 25/500.
- Текст прямо на обоях всегда с `.shadow()`.
- «Масштаб элементов интерфейса» (`ui_scale`) масштабирует то же, что сейчас: чипы фильтров, поиск,
  текст подписей.
- При «Меньше анимации» все переходы мгновенные (так уже работает `motion.rs`).
- Никакого покадрового размытия. Подложку-стекло под панелью делает план 5. Здесь панель просто уезжает.
- Каждая новая строка интерфейса добавляется в `assets/messages.json` на всех 13 языках.
- Проверка: `cargo test --bin luma` (около 6 с). Smoke не запускаем. В конце сборка и установка на ТВ.

## Что проверить в ревью

1. **Длинные фильтры** (немецкий, с «Скрытые (N)» и «Все · 140») не налезают на поиск. Что не влезло,
   ужимается через `fit_text`. Тест в Task 2.
2. **Указатель (Magic Remote)** при уехавшей панели: клик по месту, где была вкладка, не открывает
   вкладку. Хит-тест идёт по сдвинутым прямоугольникам. Тест в Task 3.
3. **Окно опций приложения** (долгое нажатие) в режиме «при фокусе» встаёт ровно на карточку. Высота
   подписи у якоря не вычитается, если подписи нет. Тест в Task 1.
4. **Возврат наверх:** «вверх» из первого ряда ведёт на фильтр «Все», прокрутка становится 0, панель
   видна целиком. Тест в Task 3.
5. **Пустые состояния** (поиск без совпадений, нет скрытых) стоят под фильтрами по новой сетке и не
   прячутся под панелью. Тест в Task 2.

---

### Task 1: подписи «при фокусе»

**Файлы:**

- Изменить: `src/model.rs`: `show_app_names: bool` заменить на `app_names: AppNames`.
- Изменить: `src/state.rs`: `Target::AppNamesApps` перебирает режимы по кругу.
- Изменить: `src/view.rs`: `tile()`, `library_grid()`, якорь `Overlay::Options`, строка `AppNamesApps` на
  подстранице «Названия приложений».
- Изменить: `assets/messages.json`: `namesFocus` («При фокусе»), `namesAlways` («Всегда»). Для «Нет» уже
  есть `stateOff`.
- Тесты: `src/tests.rs`.

**Интерфейсы:**

- Даёт: `pub enum AppNames { Focus, Always, Off }` (`#[default] Focus`, serde lowercase),
  `AppNames::next(self) -> Self` (Focus → Always → Off → Focus), `AppNames::key(self) -> &'static str`.
- Миграция: в `Preferences` поле `#[serde(alias = "show_app_names", deserialize_with =
  "crate::model::deserialize_app_names")]`. `true` даёт `Focus`, `false` даёт `Off`, строка разбирается как
  режим, а всё некорректное даёт `Focus`.

- [ ] **Шаг 1: падающие тесты.**
      `app_names_default_to_focus_and_old_preferences_migrate`: `{}` даёт `Focus`,
      `{"show_app_names": false}` даёт `Off`, `{"show_app_names": true}` даёт `Focus`,
      `{"app_names": "always"}` даёт `Always`.
      `apps_name_only_the_focused_card_by_default`: на странице Приложений текст с названием есть только у
      `Target::App` в фокусе, а у `Always` он есть у всех 6 карточек первого ряда.
      `options_anchor_sits_on_the_card_when_names_show_on_focus`: у якоря опций высота 150.
- [ ] **Шаг 2:** `cargo test --bin luma`. Тесты падают.
- [ ] **Шаг 3:** реализовать. В `tile()` подпись рисуется при `Always` или при `Focus && focused`. Хит-бокс
      растёт на подпись только при `Always`. Ряд резервирует под подпись столько же, сколько сейчас, только
      при `Always`. При `Focus` подпись помещается в межрядный зазор (см. Task 2).
- [ ] **Шаг 4:** `cargo test --bin luma`. Всё зелёное. Использования `show_app_names` в старых тестах
      заменены на `app_names`.
- [ ] **Шаг 5:** коммит `feat(apps): names on the focused card by default`.

### Task 2: сетка 6 × 260 и фильтр «Все»

**Файлы:**

- Изменить: `src/view.rs`: `library_toolbar()`, `category_chips()`, `search_field()`, `library_grid()`,
  `FACE_HEIGHT`/`GRID_PITCH` для Приложений.
- Изменить: `src/state.rs`: `apps_viewport()`, маршрут «вверх» из первого ряда (`Filter(1)` → `Filter(0)`),
  `Target::Filter(0)` в `activate` (сбросить фильтр).
- Изменить: `assets/messages.json`: `allApps` («Все · {count}»).
- Тесты: `src/tests.rs`.

**Интерфейсы:**

- Даёт: `const APPS_TOOLBAR_TOP: f32 = 150.`, высота чипов 60, `APPS_GRID_TOP = 150. + 60. + 40.` (250),
  `APPS_ROW_PITCH = 206.`, карточка 260 × 150 с шагом 300 (x = 80 + 300·col).
- `Target::Filter(0)` — чип «Все · N», где N — число видимых приложений без фильтра (`state.apps()` при
  `filter == 0` и пустом поиске). Чип выбран, когда `filter == 0`. Кнопка `ClearFilter` остаётся.

- [ ] **Шаг 1: падающие тесты.**
      `apps_fit_four_rows_on_the_tvos_grid`: у первой карточки `rect == (80, 250, 260, 150)`, у второй
      `x == 380`, у первой карточки 4-го ряда `y == 250 + 3·206` и низ `<= 1020`, при 30 приложениях.
      `the_all_filter_counts_the_apps_and_clears_the_category`: текст «All · 30», `Filter(0)` после выбора
      `Filter(2)` возвращает `filter == 0`.
      `long_filters_stay_clear_of_search`: на немецком со скрытыми приложениями правый край последнего чипа
      `<= search.x - 16`.
      `empty_states_sit_below_the_filters`: при поиске без совпадений верх блока пустого состояния `>= 250`.
- [ ] **Шаг 2:** `cargo test --bin luma`. Тесты падают.
- [ ] **Шаг 3:** реализовать. Текст чипов и поиска 24/550 · `ui_scale`. Если чипы не помещаются до поиска,
      ужимается текст (`fit_text`), чипы не переносятся.
- [ ] **Шаг 4:** `cargo test --bin luma`. Всё зелёное, координаты старых тестов Приложений обновлены.
- [ ] **Шаг 5:** коммит `feat(apps): six 260 px columns, four rows, an All filter with the count`.

### Task 3: уезжающая панель, плавная прокрутка, подъём карточки

**Файлы:**

- Изменить: `src/state.rs`: `chrome_offset()`, `animate()` (страница Приложений получает `Lane::Page` и
  подъём `Target::App`), `page_scroll()`, `reveal_focus` (верх 60 при прокрутке, первый ряд ставит
  `scroll = 0`), `hit_test` (вкладки и кнопка ТВ по сдвинутым прямоугольникам).
- Изменить: `src/view.rs`: `header()` и `library_toolbar()` рисуются со сдвигом `-chrome_offset()`.
  `tile()` поднимает карточку на Приложениях тем же `lift`, что на Главной.
- Тесты: `src/tests.rs`.

**Интерфейсы:**

- Даёт: `State::chrome_offset(&self) -> f32` = `page_scroll().min(CHROME_TRAVEL)` на Главной и
  Приложениях, `0.` в Настройках. `CHROME_TRAVEL = 150.`: панель и фильтры уходят целиком.
- Берёт: `motion::Lane::Page`, `Motion::focus`, `lift()` из `view.rs`.

- [ ] **Шаг 1: падающие тесты.**
      `the_tab_bar_and_filters_slide_away_with_the_grid`: после «вниз» во второй ряд и `animate` до покоя у
      `Target::Nav(0)` `rect.y + rect.h <= 0`, а у `Filter(0)` тоже.
      `up_from_the_first_row_brings_the_bar_back`: «вверх» из первого ряда даёт фокус `Filter(0)`,
      `scroll == 0`, `chrome_offset() == 0`.
      `a_hidden_tab_cannot_be_clicked`: `hit_test` в точке (900, 76) при уехавшей панели не даёт
      `Target::Nav(_)`.
      `apps_scroll_and_focus_ease_like_home`: после одного кадра `animate(1/60)` `page_scroll()` лежит строго
      между старым и новым `scroll`, а `motion.focus(App)` в `(0, 1)`. При `reduced_motion` всё сразу.
- [ ] **Шаг 2:** `cargo test --bin luma`. Тесты падают.
- [ ] **Шаг 3:** реализовать. На Главной панель тоже уезжает при прокрутке страницы (тот же
      `chrome_offset`), в Настройках закреплена. Текст вкладок 25/500, ширина пилюль по `navigation()`.
- [ ] **Шаг 4:** `cargo test --bin luma`. Всё зелёное.
- [ ] **Шаг 5:** коммит `feat(apps): the bar slides away with the grid, scrolling eases`.

### Task 4: снимки и проверка на ТВ

- [ ] **Шаг 1:** снимки Приложений (`target/debug/luma --screenshot`): наверху, прокрученные на 2 ряда,
      на немецком с фильтрами. Отправить пользователю.
- [ ] **Шаг 2:** `cargo fmt`, `cargo clippy -p luma --all-targets --all-features -- -D warnings`,
      `cargo test --bin luma`. Чисто.
- [ ] **Шаг 3:** сборка и установка: `pnpm run package && ares-install -d lgtv
  dist/com.ecklf.luma_*_arm.ipk && ares-launch -d lgtv com.ecklf.luma`, затем
      `git push fork HEAD:integration`.
