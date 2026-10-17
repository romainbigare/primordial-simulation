//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every day the updates due (`day`).

use crate::body::{die, live, LiveTmp, LACK};
use crate::chem::Phase;
use crate::change::Reactor;
use crate::life::Life;
use crate::math::mean_decay;
use crate::units::DAY;
use crate::{bond, choose, World};

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature (as of the last flow step), depth of the liquid that stands (what the pores of the loose ground
/// do not hold, `flow::liquid`) and surface height of a ground cell (with whole phases: no plateau share, unlike
/// `World::surface_z`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let t = w.t_ground_last(i).unwrap_or_else(|| w.t_ground(i));
    let (mut z, mut d, mut loose) = (0.0, 0.0, 0.0);
    for k in 0..w.chem.ns() {
        z += w.b(k, i) * w.chem.molar_volume(k, Phase::Solid);
        match w.chem.phase_in(k, t, |j| w.g(j, i)) {
            Phase::Solid => {
                loose += w.g(k, i) * w.chem.molar_volume(k, Phase::Solid);
                z += w.g(k, i) * w.chem.molar_volume(k, Phase::Solid);
            }
            Phase::Liquid => d += w.g(k, i) * w.chem.molar_volume(k, Phase::Liquid),
            Phase::Gas => {}
        }
    }
    let area = w.grid.area();
    [t, (d - w.cfg.phys.porosity * loose).max(0.0) / area, w.base_z + z / area]
}

/// Before each physics step: what each cell holds (`sum_cells`), the light and heat it holds aside for its bodies
/// (`hold_heat`), and once a month the merge of alike nodes, the brain groups, the traffic and each cell's surroundings.
pub fn survey(w: &mut World) {
    let n = w.grid.n();
    w.life.fit_grid(n, w.grid.w as u32);
    if w.canopy.len() != n {
        (w.canopy, w.light) = (vec![0.0; n], vec![0.0; n]);
    }
    bond::ready(w);
    let monthly = w.day.is_multiple_of(w.cfg.days_per_month as u64);
    if monthly {
        w.life.resum();
        crate::nodes::crowd(w);
    }
    shade(w);
    sum_cells(w);
    hold_heat(w);
    if monthly && w.day > 0 {
        choose::regroup(w);
        traffic(w);
    }
    if monthly || w.life.env.len() != n || w.life.seen.len() != n * w.chem.ns() {
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
        let (ns, area) = (w.chem.ns(), w.grid.area());
        w.life.seen = (0..n * ns).map(|x| LOOSE_SENSE * crate::brain::ln(1.0 + (w.ground[(x % ns) * n + x / ns] / area) as f32)).collect();
    }
    choose::cells(w);
}

/// Per cell: its members and their visible traits summed (`Life::sum`), their light-capture area per ground area
/// (`World::canopy`), the share of the light reaching it that they absorb and the share of that set aside for their
/// light reactions (`Life::absorb`, `Life::keep`). Set-aside light its bodies can no longer claim at their updates (one
/// died, moved or merged before its update, or caught less than its share; walls aside) becomes the ground's heat.
fn sum_cells(w: &mut World) {
    let n = w.grid.n();
    let keep = |s: usize, g: u32| w.light_keep.get(s, g) as f64;
    let (mut abs, mut kept, mut claim) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let mut sum = vec![[0.0; 7]; n];
    for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        let (i, m, k) = (b.cell as usize, b.n(), keep(slot, b.gen));
        sum[i][0] += m;
        for (x, &v) in sum[i][1..].iter_mut().zip(&b.vis) {
            *x += v as f64 * m;
        }
        let a = b.vis[1] as f64 * m * (1.0 - b.reflectance());
        let s = a * w.life.shade[slot];
        (abs[i], kept[i], claim[i]) = (abs[i] + s, kept[i] + s * k, claim[i] + a * k * (w.light[i] - b.mark));
    }
    let World { life, grid, canopy, light_pool, e_ground, .. } = w;
    life.sum = sum;
    life.absorb.resize(n, 0.0);
    life.keep.resize(n, 0.0);
    light_pool.resize(n, 0.0);
    let area = grid.area();
    for i in 0..n {
        let s = life.sum[i][2];
        canopy[i] = (s / area) as f32;
        life.absorb[i] = if s > 0.0 { (abs[i] / s) as f32 } else { 0.0 };
        life.keep[i] = if abs[i] > 0.0 { (kept[i] / abs[i]) as f32 } else { 0.0 };
        let over = (light_pool[i] - claim[i]).max(0.0);
        (light_pool[i], e_ground[i]) = (light_pool[i] - over, e_ground[i] + over);
    }
}

/// The heat its bodies drew from the ground at their last update goes on being drawn as time passes: the cell holds
/// aside what each would settle now at that rate (`World::heat_pool`; it gives back what none can claim), as far as
/// the ground has it.
fn hold_heat(w: &mut World) {
    let n = w.grid.n();
    let mut owed = vec![0.0; n];
    for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        let q = w.heat_keep.get(slot, b.gen) as f64;
        owed[b.cell as usize] += q * w.day.saturating_sub(b.last) as f64 * DAY;
    }
    w.heat_pool.resize(n, 0.0);
    for (i, &o) in owed.iter().enumerate() {
        let mut d = o - w.heat_pool[i];
        if d > 0.0 {
            d = d.min(crate::ground::heat_above_floor(w.e_ground[i], w.heat_cap_ground(i)));
        }
        (w.heat_pool[i], w.e_ground[i]) = (w.heat_pool[i] + d, w.e_ground[i] - d);
    }
}

/// The yearly moving average of the crossings of each cell (`Counters::traffic`), once a month.
fn traffic(w: &mut World) {
    let (n, d) = (w.grid.n(), &mut w.counts);
    d.traffic.resize(n, 0.0);
    d.crossings.resize(n, 0.0);
    let a = 1.0 / w.cfg.months_per_year as f32;
    for (t, c) in d.traffic.iter_mut().zip(d.crossings.iter_mut()) {
        *t += a * (*c * w.cfg.months_per_year as f32 - *t);
        *c = 0.0;
    }
}

/// The share of what falls on a cell that a layer of capture area `x` per ground area intercepts, per unit of `x`:
/// (1 - exp(-x)) / x (`mean_decay`).
pub(crate) fn intercept(x: f64) -> f64 {
    mean_decay(x)
}

/// Light passes through the bodies of a cell from the tallest member down (`Body::height`), and
/// bodies of equal height share a layer. A layer of capture area `x` per ground area absorbs
/// `1 - exp(-x)` of what reaches it, so a tall member shades a low one and holding up pays. Together
/// they still intercept `1 - exp(-canopy)`: the cell's light is only shared out (the shades,
/// weighted by capture area, have mean 1). `flow::air::radiation` moves each body's mark by it.
fn shade(w: &mut World) {
    let g = intercept;
    let (Life { bodies, at, shade, .. }, area, chem, cfg) = (&mut w.life, w.grid.area(), &w.chem, &w.cfg);
    shade.resize(bodies.len(), 1.0);
    let mut l: Vec<(f32, f64, usize)> = Vec::new();
    for s in at.iter() {
        l.clear();
        l.extend(s.iter().map(|&s| (s as usize, &bodies[s as usize])).map(|(s, b)| (b.height(chem, cfg) as f32, b.vis[1] as f64 * b.n() / area, s)));
        l.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (all, mut above, mut k) = (g(l.iter().map(|x| x.1).sum()), 0f64, 0);
        while k < l.len() {
            let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
            let x: f64 = l[k..e].iter().map(|x| x.1).sum();
            let f = (-above).exp() * g(x) / all;
            l[k..e].iter().for_each(|y| shade[y.2] = f);
            (above, k) = (above + x, e);
        }
    }
}

/// Update the bodies due today.
pub fn day(w: &mut World) {
    let due = w.life.take_due(w.day);
    if due.is_empty() {
        return;
    }
    // Group by group, cell by cell (a fixed order that keeps memory reads close together).
    let bodies = &w.life.bodies;
    let mut list: Vec<(u64, u32, u32)> = due.iter().filter(|&&(s, g)| bodies[s as usize].alive && bodies[s as usize].gen == g).map(|&(s, g)| ((bodies[s as usize].group as u64) << 32 | bodies[s as usize].cell as u64, s, g)).collect();
    list.sort_unstable();
    let mut rc = Reactor::new(&w.chem);
    let (mut tmp, mut act) = (LiveTmp::default(), choose::ActTmp::default());
    for (_, slot, gen) in list {
        let s = slot as usize;
        if !w.life.bodies[s].alive || w.life.bodies[s].gen != gen {
            continue;
        }
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.prepare(&w.chem, &w.cfg.phys);
        w.counts.updates += 1;
        match live(w, s, &mut b, &mut rc, &mut tmp) {
            Err(cause) => die(w, s, b, cause),
            Ok(lived) => {
                // A body with no brain takes no decision: its update is its physics only.
                if !w.life.brainless(&b) {
                    choose::act(w, s, &mut b, &lived, &mut act);
                    // An action can leave nothing of a member (all its stores put or given
                    // away): it dies at once, before another body deals with it.
                    if b.empty_at(b.mass(&w.chem)) {
                        die(w, s, b, LACK);
                        continue;
                    }
                }
                crate::lineage::resize(w, s, &mut b);
                crate::nodes::split(w, s, &mut b);
                b.vis = b.visible(&w.chem, &w.cfg.phys);
                w.life.schedule(w.day + b.interval(), slot, gen);
                w.life.bodies[s] = b;
            }
        }
    }
}
