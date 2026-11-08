# CS2 live: серверный сервис (luma-feeds) — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Сервис `cs2-live` на сервере фидов держит скорбот HLTV для идущих матчей и пишет
`www/cs2/live/<liquipedia id>.json` (раз в секунду) и `www/cs2/live/index.json` (раз в 30 с).

**Architecture:** Node-процесс под systemd рядом с `/opt/LumaFeeds`. Вход — `www/cs2/matches.json`
(Liquipedia, его пишет `run.sh`) и список live-матчей HLTV; выход — JSON-файлы, которые Caddy уже
раздаёт. Логика разбита на чистые модули (сопоставление, сборка состояния, стримы, запись) с
тестами на записанных в разведке данных; сеть — только в `service.js`.

**Tech Stack:** Node 24 (есть на сервере), форк `cybersamuurai/HLTV` (socket.io-client, Puppeteer),
`node:test`, systemd, Caddy.

**Spec:** `docs/superpowers/specs/2026-11-08-cs2-live-scorebot-design.md` (ветка `cs2-live`
в lg-luma). TV-часть — отдельный план `2026-11-08-cs2-live-tv.md`, он потребляет формат из Task 2.

## Global Constraints

- Репозиторий `eluz1ve/luma-feeds`, код в `live/`. Локальный клон `/Users/eluzive/lg/luna/luma-feeds`
  отстаёт и имеет чужую незакоммиченную правку `scripts/esports.py` — работать в отдельном worktree
  от `origin/main`, правку не трогать.
- Сервер `root@159.195.44.56`, `/opt/LumaFeeds`; IP сервера и адрес прокси — никогда в репозиторий
  (репо публичное, Cloudflare прячет origin). Секреты — в `feeds.env`.
- Liquipedia: сервис к ней не обращается вовсе.
- Соседи на Caddy (bottlebeer.md, eu.take-vpn.com, youtube.linked.ovh) не должны пострадать.
  Правка Caddyfile: diff `caddy adapt` старого и нового, копия `Caddyfile.bak-<epoch>`,
  `caddy validate`, `systemctl reload caddy` (не restart), проверка соседей.
- Chromium только по требованию, `MemoryMax=900M` в unit (на сервере свободно ≈2.4 ГБ, swap нет).
- Тесты не ходят в сеть. Длинные прогоны не запускать.
- Коммиты от eluz1ve, без Co-Authored-By; пуш в `main` luma-feeds — только после показа результата
  и текста коммита пользователю (в main сразу деплоится `run.sh`).
- Задержка до ТВ 2–3 с — норма; быстрее не оптимизировать.

## Review Focus

- Liquipedia-матч помечен live, а на HLTV его нет (перенос, нет скорбота) → файла нет, ничего не падает.
- Две команды с похожими названиями (`SAW` и `SAW Youngsters`, `MOUZ` и `MOUZ NXT`) → сопоставление не путает их.
- Сервис подключился посреди карты → hs неполные, файл помечает `hs_complete: false`.
- Смена карты в BO3 (новый `mapName`, счёт 0:0) → log и hs текущей карты сбрасываются, история раундов новой карты.
- Скорбот замолчал/разорвал соединение → переподключение с бэкоффом; файл перестаёт обновляться (ТВ сам считает его старым через 30 с).

---

### Task 0: Разведка на сервере (spike, без тестов)

**Files:** нет в репо; scratch `/root/cs2-live-spike/` на сервере, результат — фикстуры в Task 2.

- [ ] **Step 1:** На сервере: `git clone https://github.com/cybersamuurai/HLTV /root/cs2-live-spike/hltv && cd ... && npm ci && npm run build` (или как велит его README). Если Puppeteer тянет Chromium — дать ему скачать (в `~/.cache/puppeteer`), систему не трогать.
- [ ] **Step 2:** Скрипт `spike.js`: `HLTV.getMatches()` → выбрать один live-матч → `connectToScorebot({id, onScoreboardUpdate, onLogUpdate, onFullLogUpdate})`, писать каждое сообщение строкой JSONL в `raw-<id>.jsonl` 3–5 минут; плюс `HLTV.getMatch({id})` в `match-<id>.json` (стримы, команды).
- [ ] **Step 3:** Запуск. Успех = в jsonl есть и `scoreboard`, и `log`, в `match-*.json` есть `streams` со `viewers`.
- [ ] **Step 4 (если HLTV не пускает):** решение пользователя — прокси. Попросить у пользователя адрес прокси (HTTP/SOCKS), положить в `/opt/LumaFeeds/feeds.env` как `HLTV_PROXY=...`, прокинуть в Puppeteer (`--proxy-server`) и в socket.io (`agent` из `https-proxy-agent`/`socks-proxy-agent`). Повторить Step 3. Не помогло — **остановиться и написать пользователю**, дальше не идти.
- [ ] **Step 5:** Скопировать в worktree luma-feeds `live/test/fixtures/scoreboard.json` (2–3 разных scoreboard, включая смену раунда), `log.json` (несколько пачек `log` с Kill/RoundStart/RoundEnd/BombPlanted), `match.json` (ответ getMatch, урезанный до `id, team1, team2, streams`). Записать, есть ли в Kill поля для смока/прострела (`throughSmoke`, `penetrated` или иначе) — это нужно TV-плану (Task B6).
- [ ] **Step 6:** Показать пользователю итог одной строкой (работает напрямую / через прокси / не работает). Удалить `/root/cs2-live-spike` после переноса фикстур.

### Task 1: Каркас `live/` и сопоставление матчей

**Files:**
- Create: `live/package.json` (`"type": "module"`, `engines.node >=24`, зависимость на форк HLTV через git-ссылку с конкретным коммитом; `"test": "node --test test/"`), `live/pair.js`, `live/test/pair.test.js`
- Modify: `.gitignore` (`live/node_modules/`)

**Interfaces:**
- Produces: `normalize(name: string): string`; `pair(liquipedia: Match[], hltv: {id:number, team1:string, team2:string, date:number}[]): Map<string, {hltv:number, swapped:boolean}>` — ключ liquipedia `id`; `swapped` = порядок команд HLTV обратный Liquipedia.

- [ ] **Step 1: Тесты** `pair.test.js`:
  - `normalize("SAW Youngsters") === normalize("saw youngsters")`; убираются `Team `, `Esports`, `Gaming`, `Clan`, диакритика, не-буквы.
  - Пара находится при обратном порядке команд → `swapped: true`.
  - `SAW` vs `SAW Youngsters` — **не** совпадают (сравнение целых нормализованных имён, плюс `short`, без подстрок).
  - Кандидат с началом дальше ±3 ч от `begin_at` отбрасывается.
  - Liquipedia-матч с `teams: [null, …]` пропускается.
  - Два HLTV-кандидата на один матч → берётся ближайший по времени.
- [ ] **Step 2:** `cd live && npm test` → FAIL (нет модуля).
- [ ] **Step 3:** Реализовать `normalize`, `pair` в `live/pair.js`. Совпадение команды: нормализованное `name` или `short` Liquipedia == нормализованному имени HLTV.
- [ ] **Step 4:** `npm test` → PASS.
- [ ] **Step 5:** Commit `live: pair Liquipedia matches with HLTV's`.

### Task 2: Сборка состояния матча (формат для ТВ)

**Files:** Create `live/state.js`, `live/test/state.test.js`, `live/test/fixtures/*` (из Task 0).

**Interfaces:**
- Consumes: `{hltv, swapped}` из Task 1.
- Produces: `createMatch(id: string, hltv: number, swapped: boolean)`; `applyScoreboard(m, sb)`; `applyLog(m, events)`; `snapshot(m, now): object` — ровно этот JSON (его читает TV-план):

```json
{"id":"…","hltv":2400227,"updated":1794160563,"map":"Mirage","round":24,
 "score":[10,13],"sides":["ct","t"],"frozen":false,"bomb_planted":false,"hs_complete":true,
 "players":[[{"nick":"snowiee","alive":true,"hp":100,"money":4150,"kills":18,"hs":10,
   "assists":3,"deaths":16,"adr":79.9,"kast":60.9,"entry":[0,2],"weapon":"m4a1_silencer"}],[…]],
 "history":[{"round":1,"winner":1,"type":"elimination"}],
 "log":[{"type":"kill","killer":"jERK0z","killer_team":0,"assister":null,"victim":"Slexa",
   "victim_team":1,"weapon":"m4a1_silencer","headshot":true,"smoke":false,"wallbang":false},
  {"type":"round_end","winner":0,"score":[12,10],"reason":"elimination"},
  {"type":"round_start"},{"type":"bomb_planted","player":"Botman","team":1},
  {"type":"bomb_defused","player":"tuxa","team":0}],
 "streams":[]}
```

Все пары/индексы — в порядке команд **Liquipedia** (0/1), стороны — `"ct"`/`"t"` для каждой команды.
`history[].type` ∈ `elimination|bomb|defuse|time`; `round_end.reason` — то же множество.
`log` — новые первыми, не больше 40, только текущая карта. `smoke`/`wallbang` — `false`, если в
сырых данных нет флагов (Task 0 Step 5).

- [ ] **Step 1: Тесты** на фикстурах:
  - Порядок команд по Liquipedia при `swapped: true` (игроки, score, history, `killer_team`).
  - `kast`, `entry`, `adr` берутся из `advancedStats`/`damagePrRound` как есть.
  - `hs` растёт на Kill с `headShot`; если первый полученный scoreboard пришёл не в раунде 1 — `hs_complete: false`.
  - WinType → `type`: `Terrorists_Win`/`CTs_Win` → `elimination` (или `time`, если победили CT и бомба не ставилась — по `survivingPlayers > 0` у обеих сторон), `Target_Bombed` → `bomb`, `Bomb_Defused` → `defuse`.
  - Новый `mapName` → `log` и `hs` сброшены, `hs_complete: true`.
  - `log` обрезается до 40, новые первыми.
  - `snapshot` не меняется (deepEqual) без новых данных — для «писать только при изменении».
- [ ] **Step 2:** `npm test` → FAIL.
- [ ] **Step 3:** Реализовать `live/state.js`. Решение «time» vs «elimination» для победы CT — если сырые данные дают точнее (Task 0), использовать их.
- [ ] **Step 4:** `npm test` → PASS.
- [ ] **Step 5:** Commit `live: build the match file from the scorebot`.

### Task 3: Стримы

**Files:** Create `live/streams.js`, `live/test/streams.test.js`.

**Interfaces:**
- Produces: `streams(match: object): {platform:"twitch"|"youtube"|"kick", channel:string, url:string, language:string|null, viewers:number}[]` — из `getMatch().streams`, по убыванию `viewers`.

- [ ] **Step 1: Тесты** (на `fixtures/match.json` + синтетика): платформа по хосту ссылки (`twitch.tv`, `youtube.com`/`youtu.be`, `kick.com`); прочие (HLTV live, Facebook, VK…) отбрасываются; YouTube оставляет прямую ссылку на видео (`watch?v=` или `/live/<id>`), Twitch/Kick — ссылку на канал, `channel` = последний сегмент пути; сортировка по `viewers` убыв., без `viewers` — в конец; язык из флага/названия стрима, нет — `null`.
- [ ] **Step 2:** FAIL. **Step 3:** реализовать. **Step 4:** PASS.
- [ ] **Step 5:** Commit `live: the match's streams by viewers`.

### Task 4: Запись файлов

**Files:** Create `live/writer.js`, `live/test/writer.test.js`.

**Interfaces:**
- Produces: `writeMatch(dir, snapshot): boolean` (true — файл изменился); `writeIndex(dir, snapshots, now)` → `cs2/live/index.json` = `{"updated": now, "matches": {"<id>": {"map","round","score":[a,b]}}}`; `prune(dir, liveIds, now)` — удаляет `<id>.json`, чей матч не live больше 10 мин.

- [ ] **Step 1: Тесты** во временной папке: запись атомарная (tmp + `rename`, в папке не остаётся `*.tmp`); повторная запись того же snapshot (без учёта `updated`) возвращает `false` и файл не трогает; `index.json` содержит только live-матчи; `prune` не удаляет `index.json` и свежие файлы.
- [ ] **Step 2:** FAIL. **Step 3:** реализовать. **Step 4:** PASS.
- [ ] **Step 5:** Commit `live: write match files atomically`.

### Task 5: Сервис, деплой, Caddy

**Files:**
- Create: `live/service.js` (точка входа), `server/cs2-live.service` (systemd unit), `server/live-update.sh`
- Modify: `server/install.sh` (ставит `live/` через `npm ci --omit=dev` и unit), `server/Caddyfile.example` (`@live path /cs2/live/*` → `Cache-Control "no-store"`), `server/feeds.env.example` (`HLTV_PROXY=` пустой), `server/README.md`, `README.md` (раздел `cs2/live/`)

**Interfaces:**
- Consumes: Tasks 1–4.

Цикл `service.js`: каждые 30 с читает `$WWW/cs2/matches.json`; каждые 60 с — `HLTV.getMatches()` (live); пары через `pair`; кандидаты — Liquipedia `live` или `upcoming` с началом в ближайший час **и** live на HLTV. Новое — `connectToScorebot`, плюс `getMatch` для стримов раз в 5 мин. Пропало с HLTV live — `done()`/close. Каждую секунду `writeMatch` для изменившихся, каждые 30 с `writeIndex` + `prune`. Обрыв — переподключение с бэкоффом 5 → 60 с. Браузер Puppeteer открывается на запрос и закрывается после. `HLTV_PROXY` из окружения — как в Task 0 Step 4. Лог в stdout (journald).

Unit: `User=root` как у cron-раннера, `WorkingDirectory=/opt/LumaFeeds/repo/live`, `EnvironmentFile=/opt/LumaFeeds/feeds.env`, `Environment=WWW=/opt/LumaFeeds/www`, `Restart=always`, `RestartSec=10`, `MemoryMax=900M`, `Nice=10`. `live-update.sh`: `git -C repo pull` уже делает `run.sh`, поэтому скрипт = `cd repo/live && npm ci --omit=dev && systemctl restart cs2-live`.

- [ ] **Step 1:** Реализовать `service.js` (без юнит-тестов — только сеть и таймеры; логика уже покрыта).
- [ ] **Step 2:** `node --check live/service.js` и `npm test` → PASS.
- [ ] **Step 3:** Показать пользователю diff и текст коммита, после «ок» — commit `live: the CS2 live service` и push в `main`.
- [ ] **Step 4:** На сервере: `run.sh` подтянет main; затем `./install.sh --live` (новый флаг: ставит unit и зависимости) → `systemctl enable --now cs2-live`; `journalctl -u cs2-live -n 50` без ошибок.
- [ ] **Step 5:** Caddy по процедуре из Global Constraints: добавить `@live` с `no-store` (до `@feeds`, и `@feeds` исключает `/cs2/live/*`), reload, проверить соседей (`curl -sI` каждого — код как до правки).
- [ ] **Step 6:** Проверка: во время live-матча `curl -s https://luma.eluzive.dev/cs2/live/index.json` и `…/<id>.json` — `updated` свежий, повторный запрос через 2 с отдаёт новое `updated` при идущем раунде; заголовок `cache-control: no-store`; `systemctl status cs2-live` — память в пределах лимита.
- [ ] **Step 7:** Обновить память (`luma-feeds-server`): сервис `cs2-live`, unit, `live-update.sh`, `HLTV_PROXY`.
