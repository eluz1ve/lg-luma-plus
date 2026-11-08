//! The live part of a CS2 match card: HLTV's scoreboard with the round history between the
//! teams, from the feeds server's file for the open match.
use super::*;
use crate::cs2_live::{Event, Live, LivePlayer, RoundState, Side, Win};

/// The sides' bars and their rows' tint, as HLTV paints them.
const CT_BAR: u32 = 0x2d4f78;
const T_BAR: u32 = 0x6e5a24;
const CT_ROW: u32 = 0x466896;
const T_ROW: u32 = 0x967a32;
const PAD: f32 = 14.;
const HEADER: f32 = 40.;
const BAR: f32 = 32.;
const ROW: f32 = 27.;
const ROW_GAP: f32 = 2.;
const ROWS: usize = 5;
const HISTORY_ROW: f32 = 21.;
const HISTORY: f32 = HISTORY_ROW * 2. + 4.;
/// Rounds in a half of regulation and of overtime (MR12, MR3).
const HALF: u32 = 12;
const OVERTIME_HALF: u32 = 3;
/// Rounds the history shows at once.
const SHOWN: u32 = 24;
/// The columns after the nick: heading and width.
const COLUMNS: [(&str, f32); 8] = [
    ("HP", 70.),
    ("$", 70.),
    ("Op. duels", 62.),
    ("KAST", 66.),
    ("K (hs)", 74.),
    ("A", 38.),
    ("D", 38.),
    ("ADR", 56.),
];

/// How many times the teams have changed sides by `round`: at the half, and in overtime at each
/// of its halves, a new overtime starting on the sides the last one ended on.
fn switches(round: u32) -> u32 {
    if round <= HALF {
        0
    } else if round <= HALF * 2 {
        1
    } else {
        let into = round - HALF * 2 - 1;
        1 + into / (OVERTIME_HALF * 2) + into % (OVERTIME_HALF * 2) / OVERTIME_HALF
    }
}

/// The side `team` played in `round`, from the side it plays in the current round.
fn side_in(live: &Live, team: usize, round: u32) -> Side {
    let now = live.sides[team];
    if (switches(round) + switches(live.round.max(1))).is_multiple_of(2) {
        now
    } else {
        match now {
            Side::Ct => Side::T,
            Side::T => Side::Ct,
        }
    }
}

fn money(value: u32) -> String {
    if value >= 1000 {
        format!("{} {:03}", value / 1000, value % 1000)
    } else {
        value.to_string()
    }
}

/// The scoreboard filling `rect`: the round and score, each team's players, and the rounds
/// played between them.
pub(super) fn scoreboard(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    live: &Live,
    item: &crate::cs2::Match,
    rect: Rect,
) {
    let language = state.preferences.language;
    let table = BAR + (ROW + ROW_GAP) * ROWS as f32;
    // As tall as its content, within its place.
    let rect = Rect::new(
        rect.x,
        rect.y,
        rect.w,
        rect.h.min(10. + HEADER + table * 2. + HISTORY + 12.),
    );
    out.shape(rect, 18., c(0x1a1d24, 0.82));
    out.border(rect, 18., 1., c(0x333845, 1.));
    let x = rect.x + PAD;
    let width = rect.w - PAD * 2.;

    // The round and the map, the score in the sides' colours, what the round is doing.
    let top = rect.y + 10.;
    let round = format!("R: {} · {}", live.round, live.map);
    out.text(
        &round,
        Rect::new(x + 4., top + 9., 260., 20.),
        assets.fit_text(&round, text(15.2, 500, 0.), 260.),
        c(0xe7e9ee, 0.7),
        false,
    );
    let style = text(28., 650, 0.02);
    let parts = [
        (live.score[0].to_string(), cs2_side_color(live.sides[0], 1.)),
        (":".to_owned(), c(0xffffff, 0.6)),
        (live.score[1].to_string(), cs2_side_color(live.sides[1], 1.)),
    ];
    let total: f32 = parts
        .iter()
        .map(|(part, _)| assets.width(part, style))
        .sum::<f32>()
        + 12.;
    let mut at = rect.x + rect.w / 2. - total / 2.;
    for (part, tone) in parts {
        let part_w = assets.width(&part, style);
        out.text(
            &part,
            Rect::new(at, top, part_w + 2., 36.),
            style,
            tone,
            false,
        );
        at += part_w + 6.;
    }
    let status = match live.state {
        RoundState::Freeze => Some("cs2LiveFreeze"),
        RoundState::Ended => Some("cs2LiveRoundOver"),
        RoundState::Started if live.bomb_planted => Some("cs2LiveBombPlanted"),
        RoundState::Started => None,
    };
    if let Some(status) = status {
        out.text(
            language.t(status),
            Rect::new(x + width - 264., top + 9., 260., 20.),
            text(15.2, 600, 0.).right(),
            if live.bomb_planted {
                c(0xff7b72, 1.)
            } else {
                c(0xe7e9ee, 0.7)
            },
            false,
        );
    }

    let first = rect.y + 10. + HEADER;
    for team in 0..2 {
        let y = first + team as f32 * (table + HISTORY);
        players(out, assets, live, item, team, Rect::new(x, y, width, table));
    }
    history(
        out,
        assets,
        live,
        Rect::new(x, first + table, width, HISTORY),
    );
}

/// A team's bar in the colour of its side, then a row for each player.
fn players(
    out: &mut Scene,
    assets: &Assets,
    live: &Live,
    item: &crate::cs2::Match,
    team: usize,
    rect: Rect,
) {
    let side = live.sides[team];
    let (bar, row_tint) = match side {
        Side::Ct => (CT_BAR, CT_ROW),
        Side::T => (T_BAR, T_ROW),
    };
    let columns_w: f32 = COLUMNS.iter().map(|(_, w)| w).sum();
    let first_column = rect.x + rect.w - columns_w;
    let nick_room = first_column - rect.x - 10. - 8.;
    let bar_rect = Rect::new(rect.x, rect.y, rect.w, BAR);
    out.shape(bar_rect, 8., c(bar, 1.));
    let name = item.teams[team]
        .as_ref()
        .map_or("TBD", |team| team.name.as_str());
    out.text(
        name,
        Rect::new(rect.x + 10., rect.y + 6., nick_room, 22.),
        assets.fit_text(name, text(17.6, 650, 0.), nick_room),
        WHITE,
        false,
    );
    let mut column = first_column;
    for (heading, column_w) in COLUMNS {
        let style = assets.fit_text(heading, text(12.8, 600, 0.), column_w - 4.);
        out.text(
            heading,
            Rect::new(column, rect.y + 9., column_w, 16.),
            style,
            c(0xe7e9ee, 0.92),
            true,
        );
        column += column_w;
    }
    for (index, player) in live.players[team].iter().take(ROWS).enumerate() {
        let y = rect.y + BAR + ROW_GAP + index as f32 * (ROW + ROW_GAP);
        let row = Rect::new(rect.x, y, rect.w, ROW);
        out.shape(row, 8., c(row_tint, 0.28));
        player_row(out, assets, player, row, first_column, nick_room);
    }
}

fn player_row(
    out: &mut Scene,
    assets: &Assets,
    player: &LivePlayer,
    row: Rect,
    first_column: f32,
    nick_room: f32,
) {
    // The dead wait out the round dimmed; HLTV can still call a player at 0 HP alive.
    let alpha = if player.alive && player.hp > 0 {
        1.
    } else {
        0.4
    };
    let ink = c(0xffffff, 0.94 * alpha);
    out.text(
        &player.nick,
        Rect::new(row.x + 10., row.y + 4., nick_room, 20.),
        assets.fit_text(&player.nick, text(16., 650, 0.), nick_room),
        ink,
        false,
    );
    let dash = || "—".to_owned();
    let cells = [
        None,
        Some(money(player.money)),
        Some(
            player
                .entry
                .map_or_else(dash, |[won, lost]| format!("{won}:{lost}")),
        ),
        Some(player.kast.map_or_else(dash, |kast| format!("{kast:.1}%"))),
        Some(format!("{} ({})", player.kills, player.hs)),
        Some(player.assists.to_string()),
        Some(player.deaths.to_string()),
        Some(format!("{:.1}", player.adr)),
    ];
    let mut x = first_column;
    for ((_, column_w), cell) in COLUMNS.iter().zip(cells) {
        match cell {
            Some(cell) => out.text(
                &cell,
                Rect::new(x, row.y + 5., *column_w, 19.),
                assets.fit_text(&cell, text(15.2, 500, 0.), column_w - 4.),
                ink,
                true,
            ),
            // HP as a bar.
            None => {
                let track = Rect::new(x + 8., row.y + ROW / 2. - 3., column_w - 16., 6.);
                out.shape(track, 3., c(0x3a3f4b, alpha));
                if player.hp > 0 {
                    let fill = Rect::new(track.x, track.y, track.w * player.hp as f32 / 100., 6.);
                    out.shape(fill, 3., c(0x59c26b, alpha));
                }
            }
        }
        x += column_w;
    }
}

/// The rounds played, one column each: the winner's row holds how it won, in the colour of the
/// side it played; a line marks where the teams changed sides.
fn history(out: &mut Scene, assets: &Assets, live: &Live, rect: Rect) {
    let last = live
        .history
        .iter()
        .map(|(round, ..)| *round)
        .max()
        .unwrap_or(0);
    // Regulation fits; overtime shows from the start of the overtime being played.
    let first = if last <= SHOWN {
        1
    } else {
        let overtime = SHOWN + 1 + (last - SHOWN - 1) / (OVERTIME_HALF * 2) * OVERTIME_HALF * 2;
        overtime + OVERTIME_HALF * 2 - SHOWN
    };
    let pitch = rect.w / SHOWN as f32;
    for slot in 1..SHOWN {
        let round = first + slot;
        if switches(round) != switches(round - 1) {
            let line_x = rect.x + slot as f32 * pitch;
            out.shape(
                Rect::new(line_x, rect.y + 2., 1., rect.h - 4.),
                0.,
                c(0x4a4f5b, 1.),
            );
        }
    }
    for &(round, winner, how) in &live.history {
        if round < first || round >= first + SHOWN {
            continue;
        }
        let name = match how {
            Win::Elimination => "cs-win-elimination",
            Win::Bomb => "cs-win-bomb",
            Win::Defuse => "cs-win-defuse",
            Win::Time => "cs-win-time",
        };
        let slot = (round - first) as f32;
        let icon = Rect::new(
            rect.x + slot * pitch + (pitch - 15.) / 2.,
            rect.y + 2. + winner as f32 * (HISTORY_ROW + 2.) + (HISTORY_ROW - 15.) / 2.,
            15.,
            15.,
        );
        out.icon(
            assets,
            name,
            icon,
            cs2_side_color(side_in(live, winner, round), 1.),
        );
    }
}

/// How tall the scoreboard is; the game log beside it matches it.
pub(super) fn height() -> f32 {
    10. + HEADER + (BAR + (ROW + ROW_GAP) * ROWS as f32) * 2. + HISTORY + 12.
}

const KILL_SIZE: f32 = 16.;
const NOTE_SIZE: f32 = 14.;
const PILL_PAD: f32 = 10.;
const PILL_GAP: f32 = 4.;
/// Extra room between one round's events and the last round's.
const ROUND_GAP: f32 = 8.;
const KILL_FRAME: u32 = 0x9b2b2b;
const CT_FRAME: u32 = 0x3d86d6;
const T_FRAME: u32 = 0xd49a22;
const START_FRAME: u32 = 0xbdbdbd;

/// A piece of a log line: text in a colour, or one of the game's icons at a height.
enum Piece {
    Text(String, Color),
    /// An icon's name, width and height.
    Icon(String, f32, f32),
}

fn icon_piece(name: String, height: f32) -> Option<Piece> {
    super::cs2_icons::aspect(&name).map(|aspect| Piece::Icon(name, height * aspect, height))
}

/// The game log in `rect`: one pill an event, newest on top, as many as fit.
pub(super) fn game_log(out: &mut Scene, state: &State, assets: &Assets, live: &Live, rect: Rect) {
    let language = state.preferences.language;
    let mut y = rect.y;
    // Events are newest first; a round's start ends the events of the round before it.
    let mut round = live.round.max(1);
    for event in &live.log {
        let side = |team: usize| side_in(live, team, round);
        let (frame, lines, size) = match event {
            Event::Kill {
                killer,
                killer_team,
                assister,
                victim,
                victim_team,
                weapon,
                headshot,
                smoke,
                wallbang,
                noscope,
                blind,
                air: _,
            } => {
                let mut pieces = vec![Piece::Text(
                    killer.clone(),
                    cs2_side_color(side(*killer_team), 1.),
                )];
                if let Some(assister) = assister {
                    pieces.push(Piece::Text(
                        format!("+ {assister}"),
                        cs2_side_color(side(*killer_team), 0.7),
                    ));
                }
                if *blind {
                    pieces.extend(icon_piece("cs-blind".into(), 16.));
                }
                let weapon_icon = format!("cs-weapon-{}", weapon.replace('_', "-"));
                pieces.push(
                    icon_piece(weapon_icon, 18.)
                        .unwrap_or_else(|| Piece::Text(weapon.clone(), c(0xffffff, 0.8))),
                );
                for (on, name) in [
                    (*noscope, "cs-noscope"),
                    (*smoke, "cs-smoke"),
                    (*wallbang, "cs-wallbang"),
                    (*headshot, "cs-headshot"),
                ] {
                    if on {
                        pieces.extend(icon_piece(name.into(), 16.));
                    }
                }
                pieces.push(Piece::Text(
                    victim.clone(),
                    cs2_side_color(side(*victim_team), 1.),
                ));
                (KILL_FRAME, vec![pieces], KILL_SIZE)
            }
            Event::Suicide {
                player,
                team,
                weapon,
            } => {
                let mut pieces = vec![];
                pieces.extend(icon_piece("cs-suicide".into(), 16.));
                pieces.extend(icon_piece(
                    format!("cs-weapon-{}", weapon.replace('_', "-")),
                    18.,
                ));
                pieces.push(Piece::Text(player.clone(), cs2_side_color(side(*team), 1.)));
                (KILL_FRAME, vec![pieces], KILL_SIZE)
            }
            Event::RoundEnd {
                winner,
                score,
                reason,
            } => {
                let played = score[0] + score[1];
                let winner_side = side_in(live, *winner, played.max(1));
                let side_name = match winner_side {
                    Side::Ct => "CT",
                    Side::T => "T",
                };
                let team = state
                    .cs2
                    .find(&live.id)
                    .and_then(|item| item.teams[*winner].as_ref())
                    .map(|team| format!("{} ({side_name})", team.name))
                    .unwrap_or_else(|| side_name.to_owned());
                let head = format!(
                    "{} · {} {} - {}",
                    language.t("cs2LogRoundOver"),
                    language.format("cs2LogWinner", &[("team", &team)]),
                    score[0],
                    score[1]
                );
                let why = language
                    .t(match reason {
                        Win::Elimination => "cs2LogEliminated",
                        Win::Bomb => "cs2LogExploded",
                        Win::Defuse => "cs2LogDefused",
                        Win::Time => "cs2LogTime",
                    })
                    .to_owned();
                let ink = c(0xe7e9ee, 0.92);
                let style = text(NOTE_SIZE, 550, 0.);
                let whole = format!("{head} · {why}");
                let lines = if assets.width(&whole, style) <= rect.w - PILL_PAD * 2. {
                    vec![vec![Piece::Text(whole, ink)]]
                } else {
                    vec![vec![Piece::Text(head, ink)], vec![Piece::Text(why, ink)]]
                };
                let frame = match winner_side {
                    Side::Ct => CT_FRAME,
                    Side::T => T_FRAME,
                };
                (frame, lines, NOTE_SIZE)
            }
            Event::RoundStart => (
                START_FRAME,
                vec![vec![Piece::Text(
                    language.t("cs2LogRoundStarted").to_owned(),
                    c(0xe7e9ee, 0.92),
                )]],
                NOTE_SIZE,
            ),
            Event::BombPlanted { player, team, site } => {
                let site = site.map(String::from).unwrap_or_default();
                let mut pieces = vec![];
                pieces.extend(icon_piece("cs-bomb".into(), 16.));
                pieces.push(Piece::Text(player.clone(), cs2_side_color(side(*team), 1.)));
                pieces.push(Piece::Text(
                    language.t("cs2LogPlanted").to_owned(),
                    c(0xe7e9ee, 0.92),
                ));
                if !site.is_empty() {
                    pieces.push(Piece::Text(site, WHITE));
                }
                (T_FRAME, vec![pieces], NOTE_SIZE)
            }
            Event::BombDefused { player, team } => {
                let mut pieces = vec![];
                pieces.extend(icon_piece("cs-win-defuse".into(), 16.));
                pieces.push(Piece::Text(player.clone(), cs2_side_color(side(*team), 1.)));
                pieces.push(Piece::Text(
                    language.t("cs2LogDefusedIt").to_owned(),
                    c(0xe7e9ee, 0.92),
                ));
                (CT_FRAME, vec![pieces], NOTE_SIZE)
            }
        };
        let line_h = size + 8.;
        let height = PILL_PAD + line_h * lines.len() as f32 + 2.;
        if y + height > rect.y + rect.h {
            break;
        }
        let pill = Rect::new(rect.x, y, rect.w, height);
        out.shape(pill, 9., c(0x1c2028, 0.92));
        out.border(pill, 9., 1.5, c(frame, 0.95));
        for (index, pieces) in lines.into_iter().enumerate() {
            let line = Rect::new(
                pill.x + PILL_PAD,
                pill.y + PILL_PAD / 2. + 1. + index as f32 * line_h,
                pill.w - PILL_PAD * 2.,
                line_h,
            );
            draw_pieces(out, assets, pieces, line, size);
        }
        y += height + PILL_GAP;
        if matches!(event, Event::RoundStart) {
            y += ROUND_GAP;
            round = round.saturating_sub(1).max(1);
        }
    }
}

/// One line of a pill: its pieces left to right, the texts sharing what room is left after the
/// icons.
fn draw_pieces(out: &mut Scene, assets: &Assets, pieces: Vec<Piece>, line: Rect, size: f32) {
    let gap = 7.;
    let style = text(size, if size >= KILL_SIZE { 650 } else { 550 }, 0.);
    let icons: f32 = pieces
        .iter()
        .map(|piece| match piece {
            Piece::Icon(_, width, _) => *width,
            Piece::Text(..) => 0.,
        })
        .sum();
    let texts: f32 = pieces
        .iter()
        .map(|piece| match piece {
            Piece::Text(text, _) => assets.width(text, style),
            Piece::Icon(..) => 0.,
        })
        .sum();
    let room = line.w - icons - gap * (pieces.len().saturating_sub(1)) as f32;
    // Too long: every text gives up the same share.
    let scale = if texts > room && texts > 0. {
        (room / texts).max(0.)
    } else {
        1.
    };
    let mut x = line.x;
    for piece in pieces {
        match piece {
            Piece::Text(value, tone) => {
                let width = assets.width(&value, style) * scale;
                let fitted = if scale < 1. {
                    assets.ellipsize(&value, style, width)
                } else {
                    value
                };
                out.text(
                    &fitted,
                    Rect::new(
                        x,
                        line.y + (line.h - size * 1.25) / 2.,
                        width.max(1.),
                        size * 1.25,
                    ),
                    style,
                    tone,
                    false,
                );
                x += width + gap;
            }
            Piece::Icon(name, width, height) => {
                let rect = Rect::new(x, line.y + (line.h - height) / 2., width, height);
                out.icon(assets, &name, rect, WHITE);
                x += width + gap;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs2_live_sides_switch_at_the_half_and_in_overtime() {
        let changes: Vec<u32> = (2..=40)
            .filter(|&r| switches(r) != switches(r - 1))
            .collect();
        assert_eq!(changes, vec![13, 28, 34, 40]);
    }
}
