//! The lobes the high stand's mouths build (laws.md 0, `space::stand`): each mouth fills the liquid it runs into with
//! its own load (`space::fill`), on the ground the lobes before it left; the hollows they leave are filled from the
//! lobe that shut each (`space::hollows`); each lobe holds its own load to the end. The waves shape where a lobe goes, not how high it builds: on a coast they build beaches up to the level and
//! over it, so a lobe's top is the surface of its channel's flood (`top`).

use super::fill::{fill, offset, Lobe, Seen};
use super::grid::Grid;
use super::hollows::{hollows, Lift};
use super::network::NONE;

/// The flood that sorts what settles and that a delta's top builds up to: its runoff from the area that drains
/// through a sample, m/s.
pub(crate) const FLOOD_RUNOFF: f64 = 1e-6;
/// The run of open liquid at which the waves are those of an open coast, m; a longer run counts as this.
const FETCH_REF_M: f64 = 50e3;
/// The share of the stand ago what fills a hollow was laid: it fills over the whole stand.
const HOLLOW_AGE: f64 = 0.5;

/// A mouth: the sample of the open low ground its channel comes into, the load it brings, m³, the top its lobe builds to,
/// m, and the way its channel comes in, a unit vector east and south.
pub(super) struct Mouth {
    pub(super) entry: usize,
    pub(super) load: f64,
    pub(super) top: f64,
    pub(super) heading: [f64; 2],
}

/// How open to the waves each sample of grid `g` lies on the open low ground `open`, from none to 1: the run of open
/// liquid the waves come over (`Grid::fetches`) over `FETCH_REF_M`.
pub(super) fn exposures(g: &Grid, open: &[bool]) -> Vec<f64> {
    g.fetches(open, FETCH_REF_M).iter().map(|f| f / FETCH_REF_M).collect()
}

/// The height a deposit builds up to in liquid standing at `level`, m, where a channel that drains `drains` m² comes in:
/// the surface of the channel's flood there (`flood_depth`).
pub(super) fn top(level: f64, (drains, channel): (f64, [f64; 2])) -> f64 {
    level + flood_depth(drains, channel)
}

/// How deep the flood runs in a channel that drains `drains` m², m: as deep as a channel of the terrain's law `channel`
/// (`Terrain::channel`: the depth at 1 m³/s, m, and its power of the discharge, `Physics::channel`; Leopold and Maddock
/// 1953) carries `FLOOD_RUNOFF` from that area.
pub(crate) fn flood_depth(drains: f64, channel: [f64; 2]) -> f64 {
    channel[0] * (FLOOD_RUNOFF * drains).powf(channel[1])
}

/// What the lobes read of the liquid they fill: which samples are open liquid, how open to the waves each lies, the way
/// out from the coast at each (a unit vector east and south along the rise of the distance from the land; none where it
/// has none), the level of the liquid at each, m (its body's; any for a sample of no body), and the body each lies in
/// (`network::NONE` for none): a lobe stays in its mouth's body.
pub(super) struct Liquid<'a> {
    pub(super) open: &'a [bool],
    pub(super) exposed: &'a [f64],
    pub(super) seaward: Vec<[f64; 2]>,
    pub(super) level: &'a [f64],
    pub(super) body: &'a [u32],
}

impl<'a> Liquid<'a> {
    /// The open liquid `open` of grid `g`, its samples as open to the waves as `exposed`, its liquid at `level`, m, each
    /// in its body `body`.
    pub(super) fn new(g: &Grid, (open, exposed): (&'a [bool], &'a [f64]), (level, body): (&'a [f64], &'a [u32])) -> Liquid<'a> {
        let edge = g.edge_distance(open);
        let seaward = (0..g.n()).map(|k| {
            let [(e, _), (w, _), (north, _), (s, _)] = g.sides(k);
            let (across, down) = (offset(g, w, e), offset(g, north, s));
            let v = [(edge[e] - edge[w]) / across[0].abs().max(f64::MIN_POSITIVE), (edge[s] - edge[north]) / down[1].abs().max(f64::MIN_POSITIVE)];
            let len = v[0].hypot(v[1]);
            if open[k] && len.is_finite() && len > 0.0 { [v[0] / len, v[1] / len] } else { [0.0; 2] }
        }).collect();
        Liquid { open, exposed, seaward, level, body }
    }
}

/// What the lobes laid: the volume, m³; of each sample what was laid, m, and how long ago its top was laid, years; the
/// load the lobes found no room for, m³; and each sample each lobe laid on, with the volume, m³, and the lobe (its
/// mouth's place in the mouths).
pub(super) struct Laid {
    pub(super) volume: f64,
    pub(super) on: Vec<f64>,
    pub(super) age: Vec<f64>,
    pub(super) unlaid: f64,
    pub(super) parts: Vec<(usize, f64, u32)>,
}

/// The lobes of the `mouths` (largest load first, then by entry) on grid `g` of heights `h`, laid over the open low
/// ground `liquid` over a stand of `years`: each fills its own load (`fill::fill`) on the heights the lobes before it
/// left; the hollows they leave, in the liquid and in the valleys behind them (`behind`: ground that drains to a mouth
/// and lies under the highest top), are filled from the lobe that shut each (`hollows::hollows`), at most what that
/// lobe holds (`charge`); each lobe holds its own load, laid on the heights and the loose ground `loose`, m (`Laid`).
pub(super) fn lobes(g: &Grid, (h, loose): (&mut [f64], &mut [f64]), (liquid, behind): (&Liquid, &[bool]), mouths: &[Mouth], years: f64) -> Laid {
    let n = g.n();
    let mut built = h.to_vec();
    let (mut seen, mut near) = (Seen::new(n), Seen::new(n));
    let mut shut_by = vec![NONE; n];
    let mut lobes: Vec<Lobe> = Vec::with_capacity(mouths.len());
    for (m, mouth) in mouths.iter().enumerate() {
        let lobe = fill(g, &mut built, (liquid, mouth, m as u32), (&mut seen, &mut near));
        lobe.top.iter().chain(&lobe.front).for_each(|&(k, _)| shut_by[k] = m as u32);
        lobes.push(lobe);
    }
    let lifts = hollows(g, (&mut built, h), &shut_by, (liquid, behind));
    let held = charge(g, &mut built, &mut lobes, &lifts);
    let was = h.to_vec();
    let (mut age, mut holds, mut parts) = (vec![0.0; n], vec![0.0; mouths.len()], Vec::new());
    for (m, (lobe, mouth)) in lobes.iter().zip(mouths).enumerate() {
        for (k, put, laid) in laid_by(lobe, (mouth.load, years)) {
            (h[k], loose[k], age[k]) = (h[k] + put / g.area(k), loose[k] + put / g.area(k), laid);
            holds[m] += put;
            parts.push((k, put, m as u32));
        }
    }
    for &(k, by, m) in &held {
        (h[k], loose[k], age[k]) = (h[k] + by, loose[k] + by, HOLLOW_AGE * years);
        holds[m as usize] += by * g.area(k);
        parts.push((k, by * g.area(k), m));
    }
    let volume = holds.iter().sum();
    let unlaid = mouths.iter().zip(&holds).map(|(mouth, held)| mouth.load - held).sum();
    let on = (0..n).map(|k| (h[k] - was[k]).max(0.0)).collect();
    Laid { volume, on, age, unlaid, parts }
}

/// The hollows' raises `lifts` charged to the lobes that shut them: each lobe gives what its hollows take, at most all
/// it holds (`unfill`); where it holds less, its hollows are raised by that share of what they would take. Returns the
/// raises as laid, on the heights `built`.
fn charge(g: &Grid, built: &mut [f64], lobes: &mut [Lobe], lifts: &[Lift]) -> Vec<Lift> {
    let mut want = vec![0.0; lobes.len()];
    lifts.iter().for_each(|&(k, by, m)| want[m as usize] += by * g.area(k));
    let share: Vec<f64> = lobes.iter().zip(&want).map(|(lobe, &w)| {
        let holds: f64 = lobe.top.iter().chain(&lobe.front).map(|f| f.1).sum();
        if w > 0.0 { (holds / w).min(1.0) } else { 1.0 }
    }).collect();
    for (m, lobe) in lobes.iter_mut().enumerate() {
        unfill(g, built, lobe, want[m] * share[m]);
    }
    lifts.iter().map(|&(k, by, m)| {
        built[k] -= by * (1.0 - share[m as usize]);
        (k, by * share[m as usize], m)
    }).collect()
}

/// What `volume` m³ a lobe gives back from the heights `built`: its front thinned evenly, and what that cannot give from
/// the samples of its top, the last filled first.
fn unfill(g: &Grid, built: &mut [f64], lobe: &mut Lobe, mut volume: f64) {
    let front: f64 = lobe.front.iter().map(|r| r.1).sum();
    if front > 0.0 {
        let share = (volume / front).min(1.0);
        for (k, put) in lobe.front.iter_mut() {
            built[*k] -= *put * share / g.area(*k);
            *put *= 1.0 - share;
        }
        volume -= front * share;
    }
    while volume > 0.0 {
        let Some((k, put)) = lobe.top.last_mut() else { break };
        let back = put.min(volume);
        built[*k] -= back / g.area(*k);
        (*put, volume) = (*put - back, volume - back);
        if *put <= 0.0 {
            lobe.top.pop();
        }
    }
}

/// A sample a lobe laid on: the sample, the m³ it holds there and how long ago its top was laid, years.
type Part = (usize, f64, f64);

/// What lobe `lobe` (of a mouth that brought `load` m³) laid out of its hollows, each sample with the m³ it holds and how
/// long ago its top was laid, of a stand of `years`: its top by the share of its load laid after it (in the order
/// filled), its front none.
fn laid_by(lobe: &Lobe, (load, years): (f64, f64)) -> Vec<Part> {
    let mut laid = 0.0;
    let mut parts: Vec<(usize, f64, f64)> = lobe.top.iter().map(|&(k, put)| {
        laid += put;
        (k, put, years * (1.0 - laid / load).max(0.0))
    }).collect();
    parts.extend(lobe.front.iter().map(|&(k, put)| (k, put, 0.0)));
    parts.retain(|p| p.1 > 0.0);
    parts
}
