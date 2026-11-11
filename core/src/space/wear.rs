//! The ground worn at the world's start (laws.md 0 and 8): the terrain's noise says how high the ground would stand;
//! the samples' heights come from the land rising there for `WEAR_YEARS` from the low stand of the open low ground's
//! liquid (`LOW_STAND_M`), which flowing liquid cuts it down to, while its slopes creep and it floats back up as it is
//! unloaded; then the liquid rises to 0 m and drowns what lies under it. The law is the stream power law (Howard 1994; Whipple and
//! Tucker 1999): the ground lowers by K √A S, A the area that drains through a sample (spread down every lower way,
//! Quinn et al. 1991) and S its slope to the sample its liquid runs to (m = 1/2, n = 1), solved each step exactly from
//! the open low ground up the ways (Braun and Willett 2013), so a step of any length is stable. Under a closed basin's
//! level (`space::drain`) the liquid does not cut, and what the ways above it cut settles there (`space::ponds`); what
//! reaches the open low ground lies there as a shelf (`space::shelf`), and the loose ground is what the last stage left
//! (`space::mantle`). K is the bed's: faster on its soft patches (`terrain::patch`). The time runs on grids coarser than the
//! samples and last on the samples' own (`STAGES`), each stage carried smooth to the next (`Grid::finer`). All in a
//! fixed serial order in 64-bit numbers, so a seed gives one ground on every machine.

use super::drain::{draws, flood, open_low, starts, Flood};
use super::grid::Grid;
use super::network::NONE;
use super::mantle::mantle;
use super::ponds::settle;
use super::shelf::lay;
use super::slopes::{creep, rebound};
use crate::math::spread_down;
use crate::rng::{Purpose, Rng};

/// The years the land rises and is worn.
const WEAR_YEARS: f64 = 2.0e6;
/// The height of the liquid of the open low ground while the land is worn, m: the low stand the valleys are cut to,
/// before it rises to 0 m and drowns them.
const LOW_STAND_M: f64 = -100.0;
/// The stream power law's K for m = 1/2 and n = 1, per year: ground under liquid that drains 1 m² lowers by K a year
/// on a unit slope.
const CUT: f64 = 3.0e-7;
/// The diffusivity of the slopes' creep, m² a year.
const CREEP: f64 = 3.0e-3;
/// The grids the time runs on, coarsest first, each half as coarse as the one before: the samples a side to one of
/// the grid's (on a map no larger than `MOST_SAMPLES` allows), the share of the time on it, and its steps.
const STAGES: [(usize, f64, usize); 3] = [(4, 0.4, 16), (2, 0.3, 8), (1, 0.3, 4)];
/// The terrain's noise is read on the grid of this many samples a side to one (on a map no larger than `MOST_SAMPLES`
/// allows), each finer grid taking it carried smooth.
const NOISE_BY: usize = 2;
/// The most samples the first stage's grid may hold: a larger map runs every stage on grids twice as coarse, until it
/// fits (the cost of a large map's start).
const MOST_SAMPLES: usize = 1 << 16;
/// The spread of the land's heights where it starts to rise, m: the roughness the first ways find.
const ROUGH_M: f64 = 1.0;
/// The loose ground on the floor of the open low ground under the shelf, m: the layer of the fine load that settles
/// out of the liquid everywhere that mixes with the liquid over it.
const OOZE_M: f64 = 1.0;

/// The scale of the grids on grid `g`: 1, or twice that while the first stage's grid would hold more than
/// `MOST_SAMPLES`.
fn scale(g: &Grid) -> usize {
    let first = STAGES[0].0;
    let mut scale = 1;
    while g.n() / (first * scale).pow(2) > MOST_SAMPLES && g.across.is_multiple_of(2 * first * scale) && g.down.is_multiple_of(2 * first * scale) {
        scale *= 2;
    }
    scale
}

/// The samples a side to one of the grid on grid `g` the terrain's noise is read on (`NOISE_BY`, at its scale).
pub(super) fn noise_grid(g: &Grid) -> usize {
    NOISE_BY * scale(g)
}

/// The ground the wear leaves on grid `g`: of each sample its height, m, whether it is of the open low ground
/// (`drain::open_low`), and the depth of its loose ground, m (`space::mantle`); and the wear's books (`Books`).
pub(super) struct Worn {
    pub(super) h: Vec<f64>,
    pub(super) open: Vec<bool>,
    pub(super) loose: Vec<f64>,
    pub(super) books: Books,
}

/// The volumes the wear moves, m³: what it takes off the land (cut, and crept into the open low ground), what settles
/// in the closed basins, what lies on the open low ground's shelf (`space::shelf`), and what leaves the map where the
/// shelf fills the open low ground; the first is the sum of the others.
#[derive(Clone, Copy, Debug, Default)]
pub struct Books {
    pub taken: f64,
    pub settled: f64,
    pub shelf: f64,
    pub beyond: f64,
}

/// What a stage marks on each sample of its grid: the load it passes to the open low ground, m³ per m² (`offshore`),
/// what settles on it, m, how fast its last step lowered it (cut and creep), m a year, and the area that drained through
/// it in that step, m².
struct Marks {
    offshore: Vec<f64>,
    settled: Vec<f64>,
    lowers: Vec<f64>,
    drained: Vec<f64>,
}

/// The ground worn on grid `g` (`Worn`), where the terrain's noise stands at `rise`, m, on the grid `by` times coarser
/// (`noise_grid`), and flowing liquid wears its bed `soft` times as fast as `CUT` (`terrain::patch`), on that grid too.
/// Each stage's grid takes the noise's mean over each of its samples (a grid finer than the noise's, the noise carried
/// to it); the first starts from the low stand on land (`LOW_STAND_M`; the noise's heights under it) and a roughness of
/// `ROUGH_M` drawn from the seed, each later one from the last carried to it, down to the samples'. On every grid the
/// open low ground (under the low stand) stands at the noise's heights and land no lower than the low stand. What
/// reaches the open low ground lies there at the end as a shelf under the low stand's shore (`shelf::lay`) over
/// `OOZE_M` of loose ground, and the last stage's marks give the land's loose ground (`space::mantle`); the open low
/// ground it returns is what lies under 0 m joined to it.
pub(super) fn wear(seed: u64, g: &Grid, (by, rise, soft): (usize, &[f64], &[f64])) -> Worn {
    let rough = |k: usize| ROUGH_M * (Rng::new(seed, 0, k as u64, Purpose::Terrain).f64() - 0.5);
    let noise = g.coarser(by);
    let on = |x: &[f64], to: usize| if to >= by { noise.mean_over(x, to / by) } else { g.carried((by, x), to) };
    let mut last: Option<(usize, Vec<f64>, Marks)> = None;
    let (mut offshore, mut books) = (Vec::new(), Books::default());
    for &(stage_by, share, steps) in &STAGES {
        let to = stage_by * by / NOISE_BY;
        let (stage, rise_s) = (g.coarser(to), on(rise, to));
        let open_s = open_low(&stage, &rise_s, LOW_STAND_M);
        let mut h = match &last {
            None => (0..stage.n()).map(|k| if open_s[k] { rise_s[k] } else { rise_s[k].min(LOW_STAND_M) + rough(k) }).collect(),
            Some((from, z, _)) => held(&g.carried((*from, z), to), &rise_s, &open_s),
        };
        offshore = last.as_ref().map_or(vec![0.0; stage.n()], |(from, _, _)| g.carried((*from, &offshore), to));
        let cut_s: Vec<f64> = on(soft, to).iter().map(|s| CUT * s).collect();
        let marks = run(&stage, &mut h, (&rate(&rise_s, &open_s), &open_s, &draws(seed, to, stage.n())), (&cut_s, WEAR_YEARS * share, steps), &mut books);
        offshore.iter_mut().zip(&marks.offshore).for_each(|(s, m)| *s += m);
        last = Some((to, h, marks));
    }
    let (from, z, marks) = last.expect("a stage");
    let rise_g = on(rise, 1);
    let low = open_low(g, &rise_g, LOW_STAND_M);
    let mut h = held(&g.carried((from, &z), 1), &rise_g, &low);
    let soil = mantle(&g.coarser(from), (&z, &marks.drained), (&marks.settled, &marks.lowers), (CREEP, WEAR_YEARS * STAGES[STAGES.len() - 1].1));
    let mut loose: Vec<f64> = g.carried((from, &soil), 1).iter().zip(&low).map(|(&d, &o)| if o { OOZE_M } else { d.max(0.0) }).collect();
    let offshore = g.carried((from, &offshore), 1);
    books.shelf = lay(g, &mut h, (&low, LOW_STAND_M, &offshore), &mut loose);
    books.beyond = (0..g.n()).map(|k| offshore[k] * g.area(k)).sum::<f64>() - books.shelf;
    // The liquid rises to 0 m: what lies under it and joins the open low ground is of it.
    let open = open_low(g, &h, 0.0);
    Worn { h, open, loose, books }
}

/// Heights `h` with the open low ground `open` at the noise's heights `rise` and land (where the noise lies at the low
/// stand or higher) no lower than the low stand.
fn held(h: &[f64], rise: &[f64], open: &[bool]) -> Vec<f64> {
    (0..h.len()).map(|k| if open[k] { rise[k] } else if rise[k] >= LOW_STAND_M { h[k].max(LOW_STAND_M) } else { h[k] }).collect()
}

/// The rate each sample rises at, m a year: from the low stand to its noise's height in `WEAR_YEARS` on land, none on
/// ground under the low stand or on the open low ground.
fn rate(rise: &[f64], open: &[bool]) -> Vec<f64> {
    rise.iter().zip(open).map(|(&z, &o)| if o { 0.0 } else { (z - LOW_STAND_M).max(0.0) / WEAR_YEARS }).collect()
}

/// `steps` equal steps over `years` of heights `h` on grid `g`, rising at `up` (m a year), the samples `open` held,
/// flowing liquid cutting by `k` (`CUT` of each sample): each the flood from the shore (`drain::flood`), the cut, the
/// load settling, the creep and the rebound. It books what is taken and what settles in `books`, and returns the
/// stage's marks (`Marks`): the load to the open low ground and what settled over all its steps, the lowering of its
/// last.
fn run(g: &Grid, h: &mut [f64], (up, open, draw): (&[f64], &[bool], &[f64]), (k, years, steps): (&[f64], f64, usize), books: &mut Books) -> Marks {
    let dt = years / steps as f64;
    let mut marks = Marks { offshore: vec![0.0; g.n()], settled: vec![0.0; g.n()], lowers: vec![0.0; g.n()], drained: Vec::new() };
    let volume = |x: &dyn Fn(usize) -> f64| (0..g.n()).filter(|&i| !open[i]).map(|i| x(i) * g.area(i)).sum::<f64>();
    let start = starts(g, open, true);
    for _ in 0..steps {
        let f = flood(g, h, open, (&start, draw));
        let was = h.to_vec();
        marks.drained = cut(g, h, &f, (up, open, k), dt);
        let cut_to = h.to_vec();
        settle(g, h, &f, &was, (up, open), (dt, &mut marks.offshore));
        let settled = h.to_vec();
        creep(g, h, (open, LOW_STAND_M), CREEP * dt, &mut marks.offshore);
        books.taken += volume(&|i| was[i] + up[i] * dt - cut_to[i]) + volume(&|i| settled[i] - h[i]);
        books.settled += volume(&|i| settled[i] - cut_to[i]);
        for i in 0..g.n() {
            marks.settled[i] += settled[i] - cut_to[i] + (h[i] - settled[i]).max(0.0);
            marks.lowers[i] = (was[i] + up[i] * dt - cut_to[i] + (settled[i] - h[i]).max(0.0)) / dt;
        }
        rebound(g, h, &was, (up, open), dt);
    }
    marks.offshore.iter_mut().enumerate().for_each(|(i, s)| *s /= g.area(i));
    marks
}

/// One step of `dt` years of the stream power law (n = 1, solved exactly, Braun and Willett 2013): each sample, in the
/// flood's order (the one its liquid runs to first), rises by its rate and then falls toward the height of that one
/// (the low stand on the open low ground, the level of the liquid there) as h = (h₀ + f h_down) / (1 + f),
/// f = K dt √A / d. A
/// sample under its basin's level, or whose liquid runs to higher ground, only rises. Returns the area that drains
/// through each sample, m².
fn cut(g: &Grid, h: &mut [f64], f: &Flood, (up, open, k_cut): (&[f64], &[bool], &[f64]), dt: f64) -> Vec<f64> {
    let own = (0..g.n()).map(|k| g.area(k)).collect();
    let area = spread_down(&f.surface, f.order.iter().rev().map(|&k| k as usize), |k| f.earlier(g, k), own);
    for &k in &f.order {
        let k = k as usize;
        if open[k] {
            continue;
        }
        let (j, mut z) = (f.to[k], h[k] + up[k] * dt);
        if j != NONE && !f.under(h, k) {
            let j = j as usize;
            let down = if open[j] { h[j].max(LOW_STAND_M) } else { h[j] };
            if down < z {
                let c = k_cut[k] * dt * area[k].sqrt() / f.apart[k];
                z = (z + c * down) / (1.0 + c);
            }
        }
        h[k] = z;
    }
    area
}
