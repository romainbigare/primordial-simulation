//! `months.csv`, one row a month: where the mobile bodies (all but light-eaters) are and how they moved, and the
//! members born and died of each starting kind, so seasons show. The month is also when the recorder adds to the
//! year's sums what is counted at month ends (standing members, signals, bodies inside walls, the land's temperature).

use super::tally::{bump, slot, NC};
use super::{Recorder, CAUSES};
use pcore::World;
use serde::{Deserialize, Serialize};

/// The columns after `year` and `month`, with the decimals each is written with (none: as it is).
const COLS: [(&str, Option<usize>); 9] =
    [("mobile", None), ("mean_y", Some(3)), ("mean_z", Some(1)), ("mean_dy", Some(4)), ("mean_dz", Some(3)), ("inside", None), ("t_mobile", Some(2)), ("spent", Some(0)), ("spent_y", Some(2))];
/// Ground below this is cold for the signal contexts, K.
const SIGNAL_COLD_T: f64 = 283.0;
/// Ground below this is cold for the count of bodies inside walls, K.
const INSIDE_COLD_T: f64 = 278.0;
/// Of the bodies that died of `SPENT`: the least to divide their sum of rows by.
const MIN_SPENT: f64 = 1e-9;

/// One row of `months.csv`: the year and month, the values of `COLS`, members born by starting kind and died by
/// kind and cause (index kind x `NC` + cause).
#[derive(Serialize, Deserialize)]
pub(super) struct MonthRow {
    pub year: u64,
    pub month: u64,
    pub values: [f64; COLS.len()],
    pub births: Vec<f64>,
    pub deaths: Vec<f64>,
}

/// The whole file: one births column per starting kind seen in any month, then one deaths column per kind and
/// cause (a month before a kind's first birth or death reads 0).
pub(super) fn csv(rows: &[MonthRow]) -> String {
    let nk = rows.iter().map(|r| r.births.len().max(r.deaths.len().div_ceil(NC))).max().unwrap_or(0);
    let mut out = format!("year,month,{}", COLS.map(|c| c.0).join(","));
    out += &(0..nk).map(|k| format!(",root_{k}_births")).collect::<String>();
    out += &(0..nk).flat_map(|k| CAUSES.iter().map(move |c| format!(",root_{k}_died_{c}"))).collect::<String>();
    for r in rows {
        out += &format!("\n{},{}", r.year, r.month);
        for (&v, (_, decimals)) in r.values.iter().zip(COLS) {
            out += &match decimals {
                Some(d) => format!(",{v:.d$}"),
                None => format!(",{v}"),
            };
        }
        out += &(0..nk).map(|k| format!(",{}", r.births.get(k).copied().unwrap_or(0.0))).collect::<String>();
        out += &(0..NC * nk).map(|q| format!(",{}", r.deaths.get(q).copied().unwrap_or(0.0))).collect::<String>();
    }
    out + "\n"
}

/// The world's year and month of the year now.
pub(super) fn year_month(w: &World) -> (u64, u64) {
    (w.cfg.calendar().years(w.time), w.month() % w.cfg.months_per_year as u64)
}

impl Recorder {
    /// The month: its row (where the mobile bodies are and how they moved since last month), and what the year
    /// counts at month ends: standing members by kind, signals by context, bodies inside walls, the land's
    /// temperature.
    pub(super) fn month(&mut self, w: &World) {
        let (chem, gw, y, s) = (&w.chem, w.map.w as u32, &mut self.year, &mut self.social);
        let (mut n, mut sum, mut inside, mut tm) = (0.0f64, [0.0; 4], 0.0, 0.0);
        let mut last = vec![(u32::MAX, 0); w.life.bodies.len()];
        // The surface heights of the last survey (`Life::env`, not saved), or, before the first survey of a resumed world,
        // as the survey works them out.
        let z_of = |i: usize| w.life.env.get(i).map_or_else(|| pcore::survey::env_of(w, i)[2], |e| e[2]);
        y.months += 1.0;
        for (at, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            let i = b.cell as usize;
            if let Some(l) = &w.life.brains[b.lineage as usize] {
                bump(&mut y.stand, l.root as usize, b.n());
            }
            // The ground's temperature as the bodies feel it (`World::t_ground_last`).
            let (t, z) = (w.t_ground_last(i), z_of(i));
            let ctx = [t < SIGNAL_COLD_T, b.known.iter().any(|k| k.1 < 0.0), b.wb < 1.0];
            let q = (b.sig[0] > 0.0) as usize * 2 + (b.sig[1] > 0.0) as usize;
            let tab = slot(&mut y.sig, b.lineage as usize);
            for (c, &on) in ctx.iter().enumerate() {
                tab[c][on as usize][q] += 1.0;
            }
            if pcore::body::light_eater(chem, &b.genes) {
                continue;
            }
            last[at] = (b.gen, b.cell);
            let ins = pcore::bond::room(w, b).is_some();
            let c = &mut y.inside[(t >= INSIDE_COLD_T) as usize];
            (c[0], c[1], inside, tm) = (c[0] + 1.0, c[1] + ins as u8 as f64, inside + ins as u8 as f64, tm + t);
            let (dy, dz) = match s.last.get(at).filter(|l| l.0 == b.gen) {
                Some(&(_, c)) => ((b.cell / gw) as f64 - (c / gw) as f64, z - z_of(c as usize)),
                None => (0.0, 0.0),
            };
            n += 1.0;
            for (a, v) in sum.iter_mut().zip([(b.cell / gw) as f64, z, dy, dz]) {
                *a += v;
            }
        }
        s.last = last;
        // The land's mean ground temperature this month (the land is listed again each year).
        if y.t_land.is_empty() {
            y.land = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| i as u32).collect();
        }
        y.t_land.push(y.land.iter().map(|&i| w.t_ground_last(i as usize)).sum::<f64>() / y.land.len().max(1) as f64);
        let (year, month) = year_month(w);
        let mean = sum.map(|v| v / n.max(1.0));
        let [sp, sy] = std::mem::take(&mut self.month.spent);
        let values = [n, mean[0], mean[1], mean[2], mean[3], inside, tm / n.max(1.0), sp, sy / sp.max(MIN_SPENT)];
        let (births, deaths) = (std::mem::take(&mut self.month.births), std::mem::take(&mut self.month.deaths));
        self.months.push(MonthRow { year, month, values, births, deaths });
    }
}
