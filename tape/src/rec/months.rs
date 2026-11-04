//! `months.csv`, one row a month: where the mobile bodies (all but light-eaters) are and how they moved, and the
//! members born and died of each starting kind, so seasons show. The month is also when the recorder adds to the
//! year's sums what is counted at month ends (standing members, bodies inside walls, the land's temperature).

use super::tally::{bump, NC};
use crate::measure::area_mean;
use pcore::par;
use super::{Recorder, CAUSES};
use pcore::body::Body;
use pcore::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The columns after `year` and `month`, with the decimals each is written with (none: as it is).
const COLS: [(&str, Option<usize>); 9] =
    [("mobile", None), ("mean_y", Some(3)), ("mean_z", Some(1)), ("mean_dy", Some(4)), ("mean_dz", Some(3)), ("inside", None), ("t_mobile", Some(2)), ("spent", Some(0)), ("spent_y", Some(2))];
/// Ground below this is cold for the count of bodies inside walls, K.
const INSIDE_COLD_T: f64 = 278.0;
/// Of the bodies that died of `SPENT`: the least to divide their sum of rows by.
const MIN_SPENT: f64 = 1e-9;

/// One row of `months.csv`: the year and month, the values of `COLS`, members born by starting kind and died by
/// kind and cause (index kind x `NC` + cause).
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MonthRow {
    pub year: u64,
    pub month: u64,
    pub values: [f64; COLS.len()],
    pub births: Vec<f64>,
    pub deaths: Vec<f64>,
}

/// The starting kinds with columns: one births column per kind seen in any month, then one deaths column per kind and
/// cause (a month before a kind's first birth or death reads 0).
pub(super) fn kinds(rows: &[MonthRow]) -> usize {
    rows.iter().map(|r| r.births.len().max(r.deaths.len().div_ceil(NC))).max().unwrap_or(0)
}

/// The file's first line, for `nk` kinds.
pub(super) fn head(nk: usize) -> String {
    let mut out = format!("year,month,{}", COLS.map(|c| c.0).join(","));
    out += &(0..nk).map(|k| format!(",root_{k}_births")).collect::<String>();
    out += &(0..nk).flat_map(|k| CAUSES.iter().map(move |c| format!(",root_{k}_died_{c}"))).collect::<String>();
    out
}

/// One row of the file, for `nk` kinds.
pub(super) fn line(r: &MonthRow, nk: usize) -> String {
    let mut out = format!("{},{}", r.year, r.month);
    for (&v, (_, decimals)) in r.values.iter().zip(COLS) {
        out += &match decimals {
            Some(d) => format!(",{v:.d$}"),
            None => format!(",{v}"),
        };
    }
    out += &(0..nk).map(|k| format!(",{}", r.births.get(k).copied().unwrap_or(0.0))).collect::<String>();
    out += &(0..NC * nk).map(|q| format!(",{}", r.deaths.get(q).copied().unwrap_or(0.0))).collect::<String>();
    out
}

/// The world's year and month of the year now.
pub(super) fn year_month(w: &World) -> (u64, u64) {
    (w.cfg.calendar().years(w.time), w.month() % w.cfg.months_per_year as u64)
}

/// What the month's count needs of one living body, worked out in parallel and added up in the bodies' order.
struct Look {
    /// The starting kind of its lineage (none: no brain)
    root: Option<u32>,
    members: f64,
    /// None for a light-eater
    mobile: Option<Mobile>,
}

/// A body that is not a light-eater: its place, whether it is inside walls and on warm ground, the ground's
/// temperature (K), and how far it moved since the last month (rows, height; none if it was not counted then).
struct Mobile {
    cell: u32,
    row: f64,
    z: f64,
    inside: bool,
    warm: bool,
    t: f64,
    moved: (f64, f64),
}

/// The living bodies and what the month's count needs of each (`look`), and the ground's temperature of each cell.
pub(super) struct Seen<'a> {
    bodies: Vec<&'a Body>,
    looks: Vec<Look>,
    ground_t: Vec<f64>,
}

/// Look at the world for the month's count: it reads the world and the cells the mobile bodies were in at the last
/// month (`Social::last`), and changes nothing.
pub(super) fn look<'a>(w: &'a World, last: &BTreeMap<u64, u32>) -> Seen<'a> {
    // The row of the map of a cell (of its north-west square).
    let row = |i: u32| w.map.xy(w.mesh.corner(i as usize)).1 as f64;
    // The surface heights of the last survey (`Life::env`, not saved), or, before the first survey of a resumed world,
    // from the amounts (`World::surface_z`), each cell's once.
    let cells = w.mesh.cells().n();
    let fresh: Vec<f64> = if w.life.env.len() < cells { par::map_n(cells, par::UNCOUNTED, |i| w.surface_z(i)) } else { Vec::new() };
    let z_of = |i: usize| w.life.env.get(i).map_or_else(|| fresh[i], |e| e[2]);
    let bodies: Vec<&Body> = w.life.alive().collect();
    // The ground's temperature as the bodies feel it (`World::t_ground_last`), each cell's once.
    let ground_t: Vec<f64> = (0..cells).map(|i| w.t_ground_last(i)).collect();
    let looks = par::map(&bodies, |b| {
        let t = ground_t[b.cell as usize];
        let mobile = Some({
            let z = z_of(b.cell as usize);
            let moved = last.get(&b.id).map_or((0.0, 0.0), |&c| (row(b.cell) - row(c), z - z_of(c as usize)));
            let inside = pcore::bond::room(w.life.cell_at(b.cell as usize), b).is_some();
            Mobile { cell: b.cell, row: row(b.cell), z, inside, warm: t >= INSIDE_COLD_T, t, moved }
        });
        Look {
            root: w.life.brains[b.lineage as usize].as_ref().map(|l| l.root),
            members: b.n(),
            mobile,
        }
    });
    Seen { bodies, looks, ground_t }
}

impl Recorder {
    /// The month: its row, and what the year counts at month ends (members, bodies inside walls, the land's
    /// temperature), from what `look` saw.
    pub(super) fn month(&mut self, w: &World, seen: Seen) {
        let Seen { bodies, looks, ground_t } = seen;
        let cells = w.mesh.cells().n();
        let (y, s) = (&mut self.year, &mut self.social);
        let (mut n, mut sum, mut inside, mut tm) = (0.0f64, [0.0; 4], 0.0, 0.0);
        let mut mobile = Vec::new();
        y.months += 1.0;
        for (b, look) in bodies.iter().zip(&looks) {
            if let Some(root) = look.root {
                bump(&mut y.stand, root as usize, look.members);
            }
            let Some(m) = &look.mobile else { continue };
            mobile.push((b.id, m.cell));
            let c = &mut y.inside[m.warm as usize];
            (c[0], c[1], inside, tm) = (c[0] + 1.0, c[1] + m.inside as u8 as f64, inside + m.inside as u8 as f64, tm + m.t);
            n += 1.0;
            for (a, v) in sum.iter_mut().zip([m.row, m.z, m.moved.0, m.moved.1]) {
                *a += v;
            }
        }
        s.last = mobile.into_iter().collect();
        // What the bodies that left the world took is forgotten.
        s.took.retain(|&id, _| w.life.find(id).is_some());
        // The land's mean ground temperature this month (the land is listed again each year).
        if y.t_land.is_empty() {
            let land = par::map_n(cells, par::UNCOUNTED, |i| w.is_land(i));
            y.land = (0..cells).filter(|&i| land[i]).map(|i| i as u32).collect();
        }
        y.t_land.push(area_mean(y.land.iter().map(|&i| (ground_t[i as usize], w.mesh.cells().area()[i as usize]))));
        let (year, month) = year_month(w);
        let mean = sum.map(|v| v / n.max(1.0));
        let [sp, sy] = std::mem::take(&mut self.month.spent);
        let values = [n, mean[0], mean[1], mean[2], mean[3], inside, tm / n.max(1.0), sp, sy / sp.max(MIN_SPENT)];
        let (births, deaths) = (std::mem::take(&mut self.month.births), std::mem::take(&mut self.month.deaths));
        self.files.months.push(MonthRow { year, month, values, births, deaths });
    }
}
