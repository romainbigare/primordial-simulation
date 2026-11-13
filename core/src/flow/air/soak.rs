//! The rain of a step in the ground's buckets (laws.md 1.5): a cell's exchange over the step is solved without the rain
//! that falls in it (`cells::exchange`); its rain lands day by day as the air's days make it (`land`), and each day's
//! rain wets the bucket it falls on (`Soak`): what the bucket then evaporates over what it would have without it is what
//! the rain adds, its cell's other outputs moving with it by the slopes of its balance.

use super::cells::{GAINS, UP};
use super::daily::Mean;
use super::exchange::bucket;
use super::rain::{rain_air, Wet};
use super::step::Active;

/// The numbers per active gas at the end of a cell's row (`cells::Response`), before its outputs per mol of what the
/// rain adds to what evaporates of it (`cols`): the share of the cell's rain that falls on the surface that solved it,
/// the share of that surface the bucket is, what the bucket holds and holds wet, mol, as the surface's whole area would,
/// and its open rate, mol/s (`surf::Vapour::soak`). No bucket where the second is 0.
pub(super) const SOAK: usize = 5;

/// The numbers per active gas at the end of a cell's row: `SOAK`, then each changing output per mol (`cells::GAINS`).
pub(super) fn cols(nx: usize) -> usize {
    SOAK + UP + nx
}

/// A bucket the rain of a step wets as it falls: its cell, the air node its outputs go to, its active gas, the share of
/// the cell's rain on its surface, its share of that surface, what it holds at the step's start and holds wet, mol, the
/// most it holds as the rain falls (liquid over the wet bucket sinks as it falls, as the bed's store takes it after the
/// step, `drain::sink`; no less than it holds at the start), its open rate, mol/s, the time it has gone without rain, s,
/// and once rain has fallen on it what it holds with the rain and without, mol; the share of what it holds under its wet
/// amount that stays a day; and its cell's changing outputs per mol the rain adds to what evaporates.
#[derive(Clone)]
pub(super) struct Soak {
    pub(super) cell: u32,
    node: u32,
    y: usize,
    rain: f64,
    share: f64,
    held: f64,
    wet: f64,
    most: f64,
    e: f64,
    dry: f64,
    now: Option<[f64; 2]>,
    decay: Option<f64>,
    pub(super) per: Vec<f64>,
}

impl Soak {
    /// The bucket of active gas `y` of cell `cell` under air node `node` from its numbers in its row (`cols`), if any.
    pub(super) fn of(cell: usize, node: u32, y: usize, block: &[f64]) -> Option<Soak> {
        let [rain, share, held, wet, e] = block[..SOAK].try_into().expect("SOAK numbers");
        (share > 0.0).then(|| Soak { cell: cell as u32, node, y, rain, share, held, wet, most: held.max(wet), e, dry: 0.0, now: None, decay: None, per: block[SOAK..].to_vec() })
    }
    /// A day of `dd` seconds on which `r` mol of its gas land on its cell: what the rain adds to what evaporates, mol
    /// (each bucket followed exactly through the day, `exchange::bucket`; until rain first falls the two are one, found
    /// from the step's start when it does).
    fn day(&mut self, r: f64, dd: f64) -> f64 {
        let [with, without] = match self.now {
            Some(now) => now,
            None if r > 0.0 && self.dry > 0.0 => [self.held - bucket(self.held, 0.0, self.e, self.wet, self.dry).0; 2],
            None if r > 0.0 => [self.held; 2],
            None => {
                self.dry += dd;
                return 0.0;
            }
        };
        let r = r * self.rain;
        // Both under their wet amount through the day, each falls as e^(-k t) (k the open rate over the wet amount), so
        // what the rain adds is what it leaves of their difference, which follows the rain alone.
        let k = self.e / self.wet;
        let decay = *self.decay.get_or_insert_with(|| (-k * dd).exp());
        let apart = (with - without) * decay + r * (1.0 - decay) / (k * dd);
        if with < self.wet && without * decay + apart <= self.wet {
            self.now = Some([without * decay + apart, without * decay]);
            return self.share * (r - (apart - (with - without)));
        }
        let (m_with, m_without) = (bucket(with, r, self.e, self.wet, dd).0, bucket(without, 0.0, self.e, self.wet, dd).0);
        self.now = Some([(with + r - m_with).min(self.most), without - m_without]);
        self.share * (m_with - m_without)
    }
}

/// The buckets the rain of a step wets as it falls (`Soak`, in the order of their cells), and the room their days keep:
/// per active gas and node (`[y * na + j]`) the share of a day's rain each entry of the node gets, and per active gas and
/// cell (`[y * n + i]`) what lands on it in a day, mol.
pub(super) struct Soaks {
    pub(super) list: Vec<Soak>,
    shares: Vec<Vec<f64>>,
    on: Vec<f64>,
}

impl Soaks {
    /// `list` under the air of `wet`, `nx` active gases.
    pub(super) fn new(list: Vec<Soak>, wet: &Wet, nx: usize) -> Soaks {
        let on = vec![0.0; if list.is_empty() { 0 } else { nx * wet.cells() }];
        Soaks { list, shares: vec![Vec::new(); nx * wet.nodes.len()], on }
    }
}

/// Where the rain of a day lands, and what it adds to what evaporates: the rain of each active gas and node, `rain`
/// (`[y * na + j]`, mol), at the saturation ratio it falls at, `ratio` times it, lands on the places that see the node
/// by `Seen::landing` (into `out.landed`, per entry of the node), meeting the air touching them (`touched`); returned,
/// per node and gas the air it meets, K (`rain_air`). The rain on each cell wets its buckets (`Soak::day`): what each
/// adds over `dd` seconds goes into its air node's gas `x` (`[gi * na + j]`) and heat `e` by its cell's outputs, and
/// into `out.soaked`.
pub(super) fn land(wet: &Wet, act: &[Active], (rain, ratio, touched): (&[f64], &[f64], &[f64]), (soaks, dd): (&mut Soaks, f64), (x, e, out): (&mut [f64], &mut [f64], &mut Mean)) -> Vec<Option<f64>> {
    let (na, n) = (wet.nodes.len(), wet.cells());
    let work = crate::par::shareable((0..rain.len()).filter(|&i| rain[i] > 0.0).map(|i| wet.nodes[i % na].cell.len() * MET_WORK));
    let mut fell: Vec<(usize, &mut Vec<f64>, Option<f64>)> = soaks.shares.iter_mut().enumerate().filter(|s| rain[s.0] > 0.0).map(|(i, v)| (i, v, None)).collect();
    crate::par::each_init(&mut fell, work, || (), |_, (i, shares, met)| {
        shares.clear();
        shares.resize(wet.nodes[*i % na].cell.len(), 0.0);
        *met = rain_air(wet, (*i / na, *i % na, ratio[*i] / rain[*i]), touched, shares);
    });
    let mut met = vec![None; rain.len()];
    for (i, shares, air) in fell {
        let (node, landed) = (&wet.nodes[i % na], &mut out.landed[i]);
        landed.resize(node.cell.len(), 0.0);
        for (e, &share) in shares.iter().enumerate().filter(|s| *s.1 > 0.0) {
            landed[e] += rain[i] * share;
            if !soaks.on.is_empty() && (node.cell[e] as usize) < n {
                soaks.on[(i / na) * n + node.cell[e] as usize] += rain[i] * share;
            }
        }
        met[i] = air;
    }
    for (k, soak) in soaks.list.iter_mut().enumerate() {
        let m = soak.day(soaks.on[soak.y * n + soak.cell as usize], dd);
        if m != 0.0 {
            let j = soak.node as usize;
            e[j] += m * soak.per[GAINS];
            act.iter().enumerate().for_each(|(z, ac)| x[ac.gi * na + j] += m * soak.per[UP + z]);
            out.soaked[k] += m;
        }
    }
    soaks.on.iter_mut().for_each(|r| *r = 0.0);
    met
}

/// Instructions of the air a node's rain meets and where it lands, per entry of a node that rains (`rain_air`,
/// `par::pays`).
const MET_WORK: usize = 25;

#[cfg(test)]
mod tests {
    use super::Soak;
    use crate::flow::air::exchange::bucket;

    /// Rain that falls evenly through a step, day by day, onto a bucket below its wet amount (none of it rising over it)
    /// adds what one bucket under that rain evaporates over one without it, to rounding: the days, first dry then wet,
    /// follow the bucket exactly. A bucket that stays at its open rate through the step takes no more for the rain.
    #[test]
    fn the_rain_of_a_step_wets_its_bucket_as_one_bucket_under_it() {
        let (days, dd, e, wet) = (15, 86_400.0, 2e-5, 100.0);
        let dt = days as f64 * dd;
        for (held, rain, dry_days) in [(40.0, 0.5, 0), (40.0, 0.5, 4), (5.0, 1.2, 0)] {
            let mut soak = Soak::of(0, 0, 0, &[1.0, 0.6, held, wet, e, 0.0]).expect("a bucket");
            let added: f64 = (0..days).map(|d| soak.day(if d < dry_days { 0.0 } else { rain }, dd)).sum();
            let wet_days = (days - dry_days) as f64 * dd;
            let left = held - bucket(held, 0.0, e, wet, dry_days as f64 * dd).0;
            let one = bucket(left, rain * wet_days / dd, e, wet, wet_days).0 - bucket(left, 0.0, e, wet, wet_days).0;
            assert!((added - 0.6 * one).abs() < 1e-9 * one, "held {held}, rain {rain} from day {dry_days}: {added} against {}", 0.6 * one);
            assert!(added > 0.0 && added < 0.6 * rain * (days - dry_days) as f64);
        }
        let mut open = Soak::of(0, 0, 0, &[1.0, 1.0, 2.0 * wet + e * dt, wet, e, 0.0]).expect("a bucket");
        assert_eq!((0..days).map(|_| open.day(1.0, dd)).sum::<f64>(), 0.0);
    }
}
