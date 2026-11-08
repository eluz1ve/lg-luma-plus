//! The live part of a CS2 match card: HLTV's scoreboard with the round history between the
//! teams, from the feeds server's file for the open match.
use super::*;
use crate::cs2_live::{Live, LivePlayer, RoundState, Side, Win};

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
    if (switches(round) + switches(live.round.max(1))) % 2 == 0 {
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
