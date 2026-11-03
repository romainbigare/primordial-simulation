//! What the recorder counts between its rows: the sums of the year (started again at each yearly row), the sums of
//! the month to come, and what it keeps across years (where bodies were, what they took in their lives, the busiest
//! cells and the worn edges at the last decade).

use super::{Recorder, CAUSES};
use pcore::body::SPENT;
use pcore::event::{Event, EventKind};
use pcore::World;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Death causes counted (`CAUSES`).
pub(super) const NC: usize = CAUSES.len();

/// Entry `k` of `v`, grown with defaults to hold it.
pub(super) fn slot<T: Clone + Default>(v: &mut Vec<T>, k: usize) -> &mut T {
    if v.len() <= k {
        v.resize(k + 1, T::default());
    }
    &mut v[k]
}

/// Add `x` to entry `k` of `v`, grown with zeros to hold it.
pub(super) fn bump(v: &mut Vec<f64>, k: usize, x: f64) {
    *slot(v, k) += x;
}

/// One starting kind's sums of the year (by `Lineage::root`): members born, of them those with two parents
/// (`Born::mixed`), and members died by cause.
#[derive(Clone, Copy, Default)]
pub(super) struct KindYear {
    pub births: f64,
    pub mixed: f64,
    pub deaths: [f64; NC],
}

/// The sums of the year so far.
#[derive(Default)]
pub(super) struct YearTally {
    /// Members born and died by cause, of all kinds and of each
    pub births: f64,
    pub deaths: [f64; NC],
    pub kinds: Vec<KindYear>,
    /// Members of each starting kind summed over the month ends, and the months counted (standing members are the
    /// one over the other)
    pub stand: Vec<f64>,
    pub months: f64,
    /// The land's mean ground temperature at each month end, K, and the land cells it is of (listed at the first
    /// month of the year, `World::is_land`)
    pub t_land: Vec<f64>,
    pub land: Vec<u32>,
    /// Watch events, of them within a lineage; watchers per watched body
    pub watched: f64,
    pub kin_watched: f64,
    pub spread: BTreeMap<u64, BTreeSet<u64>>,
    /// Puts into other bodies, kg put, of them those by bodies that are not light-eaters (still alive at the month's
    /// end), and the (giver, receiver) pairs
    pub gave: f64,
    pub gave_kg: f64,
    pub gave_eaters: f64,
    pub pairs: BTreeSet<(u64, u64)>,
    /// kg of what they use (`Body::uses`) that bodies that are not light-eaters took: from light-eaters, from others, from the
    /// ground
    pub intake: [f64; 3],
    /// Bonds made by kind (held, joined); broken by bodies; failed by load
    pub made: [f64; 2],
    pub broken: f64,
    pub failed: f64,
    /// Body-months of bodies that are not light-eaters, [cold, warm] x [all, inside walls]
    pub inside: [[f64; 2]; 2],
    /// The core's tallies of the year, taken at the yearly row: decisions drawn and those where the action drawn had
    /// the top score; batches of bodies' updates and the busiest chunk's share of each summed; what bodies did with parts
    pub decisions: [f64; 2],
    pub busiest: [f64; 2],
    pub parts: pcore::bond::Tally,
    /// Cover that died, kg, by cause (`Counters::cover_lost`)
    pub cover_lost: [f64; 6],
}

/// The sums of the month to come (a row of `months.csv`).
#[derive(Default)]
pub(super) struct MonthTally {
    /// Members born by starting kind, and died by kind and cause (index kind x `NC` + cause)
    pub births: Vec<f64>,
    pub deaths: Vec<f64>,
    /// Members of the others (not light-eaters) that died of `SPENT`, and the sum of their rows (y)
    pub spent: [f64; 2],
}

/// What the recorder keeps across years of what bodies do (a snapshot carries it: `Kept`).
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Social {
    /// The cell of each body that is not a light-eater at the last month, by id (for movement by month)
    pub last: BTreeMap<u64, u32>,
    /// The busiest cells at the last decade, and the share of them still busiest then
    pub top: Vec<usize>,
    pub persist: f64,
    /// Worn edges last year and at the last decade, and the share of the decade's still worn
    pub worn: Vec<u32>,
    pub worn_old: Vec<u32>,
    pub worn_persist: f64,
    /// What each living body took from other bodies in its life, by id, kg: from light-eaters, from the others
    pub took: BTreeMap<u64, [f32; 2]>,
}

impl Social {
    /// What the body with this id took from other bodies in its life, kg (none if nothing).
    pub fn eaten(&self, id: u64) -> Option<[f32; 2]> {
        self.took.get(&id).copied()
    }
}

impl Recorder {
    /// Count one event into the sums of the year and of the month (and what the taker took in its life).
    pub(super) fn count(&mut self, e: &Event, w: &World) {
        let (y, m, s) = (&mut self.year, &mut self.month, &mut self.social);
        match e.kind {
            EventKind::FirstSubstance(k) => self.files.log.push((e.day, format!("substance {k} first appears"))),
            EventKind::Born { root, mixed, .. } => {
                y.births += e.value;
                if root != u32::MAX {
                    let k = slot(&mut y.kinds, root as usize);
                    k.mixed += if mixed { e.value } else { 0.0 };
                    k.births += e.value;
                    bump(&mut m.births, root as usize, e.value);
                }
            }
            EventKind::Died { cause, root } => {
                let c = (cause as usize).min(NC - 1);
                y.deaths[c] += e.value;
                if root != u32::MAX {
                    slot(&mut y.kinds, root as usize).deaths[c] += e.value;
                    bump(&mut m.deaths, NC * root as usize + c, e.value);
                }
                if cause == SPENT {
                    m.spent = [m.spent[0] + e.value, m.spent[1] + e.value * w.map.xy(w.map.square_of(e.node as usize)).1 as f64];
                }
            }
            EventKind::Watched { by, of } => {
                (y.watched, y.kin_watched) = (y.watched + 1.0, y.kin_watched + e.value);
                y.spread.entry(of).or_default().insert(by);
            }
            EventKind::Gave { from, to } => {
                (y.gave, y.gave_kg) = (y.gave + 1.0, y.gave_kg + e.value);
                y.pairs.insert((from, to));
                y.gave_eaters += w.life.find(from).is_some() as u8 as f64;
            }
            EventKind::Took { by, from, used } => {
                // From the cover, from other bodies, from the ground.
                let source = match from {
                    u64::MAX => 2,
                    pcore::choose::COVER => 0,
                    _ => 1,
                };
                if source < 2 {
                    s.took.entry(by).or_default()[source] += e.value as f32;
                }
                y.intake[source] += used as f64;
            }
            EventKind::BondMade => y.made[(e.value as usize).min(1)] += 1.0,
            EventKind::BondBroken if e.value >= pcore::bond::LOAD => y.failed += 1.0,
            EventKind::BondBroken => y.broken += 1.0,
        }
    }
}
