//! The high stand at the world's start (laws.md 0): after the wear (`space::wear`) the liquid of the open low ground
//! rises to 0 m and stands there for `HIGH_STAND_YEARS`, and the channels fill the heads of the valleys it drowned. Each
//! mouth brings the load its catchment yields, by the law fitted to the load of the channels of the one planet known
//! (Syvitski and Milliman 2007): Qs = ω Q^0.31 A^0.5 R T, its discharge Q, its area A, its relief R and its mean
//! temperature T. The ground its catchment loses is taken where the wear's last stage lowered it fastest, no lower than
//! the sample its liquid runs to or the top of its mouth's lobe, and the load builds a lobe at the mouth
//! (`space::delta`).

use super::delta::{exposures, lobes, top, Liquid, Mouth};
use super::fill::offset;
use super::drain::{draws, flood, open_low, starts, Flood};
use super::grid::Grid;
use super::network::NONE;
use super::slopes::ROCK_KG_M3;
use super::wear::{Books, STAND_M};
use crate::units::{KG_PER_MT, M2_PER_KM2, M3_PER_KM3, M_PER_KM, T_SURF};

/// The years the liquid stands at its level before the world starts, the channels filling what it drowned.
const HIGH_STAND_YEARS: f64 = 6.0e3;
/// The yield law (Syvitski and Milliman 2007), in its own units: the load is `YIELD_OMEGA` Mt a year times the
/// discharge in km³ a year to `YIELD_Q_EXP`, the area in km² to `YIELD_A_EXP`, the relief in km and the temperature in
/// degrees over `YIELD_T0_K`.
const YIELD_OMEGA: f64 = 6.0e-4;
const YIELD_Q_EXP: f64 = 0.31;
const YIELD_A_EXP: f64 = 0.5;
const YIELD_T0_K: f64 = 273.15;
/// The liquid that runs off a catchment in a year, m (its discharge over its area).
const RUNOFF_M: f64 = 0.3;
/// The samples of a channel a mouth's heading is read along, upstream of its mouth.
const HEADING_SAMPLES: usize = 4;

/// The load a catchment of `area` m² whose highest ground stands `relief` m over the mouth yields in a year, m³ of bed
/// (Syvitski and Milliman 2007, at the typical surface temperature `T_SURF`).
fn yields(area: f64, relief: f64) -> f64 {
    let (km2, km3) = (area / M2_PER_KM2, RUNOFF_M * area / M3_PER_KM3);
    let mt = YIELD_OMEGA * km3.powf(YIELD_Q_EXP) * km2.powf(YIELD_A_EXP) * (relief / M_PER_KM) * (T_SURF - YIELD_T0_K);
    mt * KG_PER_MT / ROCK_KG_M3
}

/// What drains to the open low ground with no closed basin on its way: of each sample whether it does, the area that
/// drains through it, m², its highest ground, m, and its neighbour upstream that drains the most (`NONE` for none).
struct Catchments {
    free: Vec<bool>,
    area: Vec<f64>,
    high: Vec<f64>,
    up: Vec<u32>,
}

/// The catchments of flood `f` on grid `g` of heights `h` over the open low ground `open` (`Catchments`).
fn catchments(g: &Grid, h: &[f64], f: &Flood, open: &[bool]) -> Catchments {
    let n = g.n();
    let mut free = vec![false; n];
    for &k in &f.order {
        let (k, j) = (k as usize, f.to[k as usize]);
        free[k] = !open[k] && !f.under(h, k) && j != NONE && (open[j as usize] || free[j as usize]);
    }
    let (mut area, mut high, mut up) = (vec![0.0; n], h.to_vec(), vec![NONE; n]);
    for &k in f.order.iter().rev() {
        let k = k as usize;
        if !free[k] {
            continue;
        }
        area[k] += g.area(k);
        let j = f.to[k] as usize;
        if !open[j] {
            if up[j] == NONE || area[k] > area[up[j] as usize] {
                up[j] = k as u32;
            }
            (area[j], high[j]) = (area[j] + area[k], high[j].max(high[k]));
        }
    }
    Catchments { free, area, high, up }
}

/// The way the channel at mouth `k` comes into sample `entry` of grid `g`, a unit vector east and south: from the
/// sample `HEADING_SAMPLES` up its main channel (`Catchments::up`) to the entry.
fn heading(g: &Grid, c: &Catchments, (k, entry): (usize, usize)) -> [f64; 2] {
    let mut from = k;
    for _ in 0..HEADING_SAMPLES {
        if c.up[from] == NONE {
            break;
        }
        from = c.up[from] as usize;
    }
    let v = offset(g, from, entry);
    let len = v[0].hypot(v[1]);
    if len > 0.0 { [v[0] / len, v[1] / len] } else { [0.0; 2] }
}

/// The high stand on grid `g` of heights `h` (the module's doc) under the channel law `channel`, the wear's last stage
/// having lowered each sample at `lowers`, m a year: lowers each catchment by its load where it lowered fastest, builds
/// the lobes at the mouths (adding them to the loose ground `loose`, m), and books both in `books`. Returns what it
/// laid on each sample, m, and how long ago its top was laid, years.
pub(super) fn high_stand(seed: u64, g: &Grid, (h, loose): (&mut [f64], &mut [f64]), (lowers, channel): (&[f64], [f64; 2]), books: &mut Books) -> (Vec<f64>, Vec<f64>) {
    let n = g.n();
    let open = open_low(g, h, STAND_M);
    let f = flood(g, h, &open, (&starts(g, &open, false), &draws(seed, 1, n)));
    let c = catchments(g, h, &f, &open);
    let exposed = exposures(g, &open);
    // The mouths: their entries, loads over the stand and heads; the channels that share an entry share their flood.
    let mut drains = vec![0.0; n];
    let ends: Vec<usize> = (0..n).filter(|&k| c.free[k] && open[f.to[k] as usize]).collect();
    ends.iter().for_each(|&k| drains[f.to[k] as usize] += c.area[k]);
    let mouth_top = |k: usize| top(STAND_M, (drains[f.to[k] as usize], channel));
    let mut load = vec![0.0; n];
    ends.iter().for_each(|&k| load[k] = yields(c.area[k], (c.high[k] - STAND_M).max(0.0)) * HIGH_STAND_YEARS);
    let mouth = mouths_of(&f, &c, &open);
    let given = take(g, h, (&f, &c, &mouth), (lowers, &load, &mouth_top));
    books.held += load.iter().sum::<f64>() - given.iter().sum::<f64>();
    books.yields = load.iter().sum::<f64>() / HIGH_STAND_YEARS;
    let highest = ends.iter().map(|&k| mouth_top(k)).fold(STAND_M, f64::max);
    let behind: Vec<bool> = (0..n).map(|k| c.free[k] && h[k] < highest).collect();
    let mut mouths: Vec<Mouth> = ends.iter().filter(|&&k| given[k] > 0.0).map(|&k| {
        let entry = f.to[k] as usize;
        Mouth { entry, load: given[k], top: mouth_top(k), heading: heading(g, &c, (k, entry)) }
    }).collect();
    mouths.sort_by(|a, b| b.load.total_cmp(&a.load).then(a.entry.cmp(&b.entry)));
    let taken: f64 = given.iter().sum();
    let liquid = Liquid::new(g, (&open, &exposed), STAND_M);
    let laid = lobes(g, (h, loose), (&liquid, &behind), &mouths, HIGH_STAND_YEARS);
    (books.taken, books.high, books.unlaid) = (books.taken + taken, laid.volume, laid.unlaid);
    books.beyond += taken - laid.volume - laid.unlaid;
    (laid.on, laid.age)
}

/// Of each sample of the catchments `c` of flood `f`, the mouth its liquid leaves the land by into the open low ground
/// `open` (`NONE` for none).
fn mouths_of(f: &Flood, c: &Catchments, open: &[bool]) -> Vec<u32> {
    let mut mouth = vec![NONE; c.free.len()];
    for &k in &f.order {
        let (k, j) = (k as usize, f.to[k as usize] as usize);
        if c.free[k] {
            mouth[k] = if open[j] { k as u32 } else { mouth[j] };
        }
    }
    mouth
}

/// Each catchment of the mouths `load` (m³ at each mouth's sample) of flood `f`, its catchments `c` and each sample's
/// `mouth`, gives its load from where the wear's last stage lowered it fastest (`lowers`, m a year; by area where none
/// lowered), each sample no lower than the sample its liquid runs to (the liquid's level where it runs into it) or its
/// mouth's top (`mouth_top(k)` at mouth `k`): lowers the heights `h`, m, and returns what each mouth's catchment gave,
/// m³, which is the load it brings.
fn take(g: &Grid, h: &mut [f64], (f, c, mouth): (&Flood, &Catchments, &[u32]), (lowers, load, mouth_top): (&[f64], &[f64], &dyn Fn(usize) -> f64)) -> Vec<f64> {
    let n = g.n();
    let rate = |k: usize| lowers[k].max(0.0) * g.area(k);
    let mut lowered = vec![0.0; n];
    (0..n).filter(|&k| c.free[k]).for_each(|k| lowered[mouth[k] as usize] += rate(k));
    let mut given = vec![0.0; n];
    // (From the open low ground up, so each sample's outlet is lowered first.)
    for k in f.order.iter().map(|&k| k as usize).filter(|&k| c.free[k]) {
        let (m, j) = (mouth[k] as usize, f.to[k] as usize);
        let share = if lowered[m] > 0.0 { rate(k) / lowered[m] } else { g.area(k) / c.area[m] };
        let floor = h[j].max(STAND_M).max(mouth_top(m));
        let down = (load[m] * share / g.area(k)).min((h[k] - floor).max(0.0));
        h[k] -= down;
        given[m] += down * g.area(k);
    }
    given
}
