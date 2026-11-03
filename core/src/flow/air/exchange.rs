//! The exchange of ground nodes with the air: evaporation, condensation (rain and cloud), and gases dissolving in the
//! ground's liquid; and how what dissolves is booked (`Crossing`).

use super::super::{land::{boil_off, wet}, range, Scratch};
use super::surf::patches;
use super::touching;
use super::super::potential::apply;
use crate::{chem::Chemistry, math::one_minus_exp, store::{Class, LinkFlux}, units::R_GAS, AIR, World};

/// Entries of the table of a saturation pressure (`Sat`) per K.
const SAT_PER_K: f64 = 20.0;
/// The table starts at `SAT_LOWEST`, K, and reaches this far above the warmest temperature it is asked for, K.
const SAT_MARGIN: f64 = 1.0;
const SAT_LOWEST: f64 = 1.0;

/// Saturation pressure of one substance (`Chemistry::p_sat`), tabulated every 1 / `SAT_PER_K` K from `SAT_LOWEST`, Pa;
/// each entry depends only on its place, so a table rebuilt further up holds the same numbers.
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Sat(Vec<f64>);

impl Sat {
    /// Reaches `hi` + `SAT_MARGIN` K at least for substance `k`.
    pub(super) fn fit(&mut self, chem: &Chemistry, k: usize, hi: f64) {
        let len = ((hi + SAT_MARGIN - SAT_LOWEST) * SAT_PER_K) as usize + 2;
        if self.0.len() < len {
            self.0 = (0..len).map(|i| chem.p_sat(k, SAT_LOWEST + i as f64 / SAT_PER_K)).collect();
        }
    }
    pub(super) fn at(&self, t: f64) -> f64 {
        let x = ((t - SAT_LOWEST) * SAT_PER_K).clamp(0.0, (self.0.len() - 2) as f64);
        let i = x as usize;
        let f = x - i as f64;
        self.0[i] + (self.0[i + 1] - self.0[i]) * f
    }
}

/// What evaporates over `dt` seconds from ground holding `a` mol that gets `r` mol of rain evenly: `e` mol/s while it
/// holds `wet` mol or more, in proportion below that (the bucket of Manabe 1969), integrated exactly over the step; and
/// its slope with the open rate `e`, s.
pub(super) fn bucket(a: f64, r: f64, e: f64, wet: f64, dt: f64) -> (f64, f64) {
    let rate = r / dt;
    if e <= 0.0 {
        return (0.0, 0.0);
    }
    // No more than the ground holds and gets, and no less than nothing.
    let held = |m: f64, dm: f64| (m.min(a + r).max(0.0), if m > 0.0 && m < a + r { dm } else { 0.0 });
    // What has evaporated and the time left when the ground comes down to `wet`, holding `now`; each with its slope.
    let (mut m, mut left, mut now, mut dm, mut d_left) = (0.0, dt, a, 0.0, 0.0);
    if now > wet {
        let t1 = if e > rate { (now - wet) / (e - rate) } else { dt };
        if t1 >= dt {
            return held(e * dt, dt);
        }
        let d_t1 = -t1 / (e - rate);
        (m, left, now, dm, d_left) = (e * t1, dt - t1, wet, t1 + e * d_t1, -d_t1);
    }
    if wet <= 0.0 {
        return held(m + rate.min(e) * left, dm + if e < rate { left } else { 0.0 } + rate.min(e) * d_left);
    }
    // Below `wet`: d(held)/dt = rate - k held, toward `rate / k`; when that is over `wet` the ground wets again.
    let (k, eq) = (e / wet, rate * wet / e);
    let (d_k, d_eq) = (1.0 / wet, -eq / e);
    let t2 = if eq > wet { ((eq - now) / (eq - wet)).ln() / k } else { f64::INFINITY };
    if t2 >= left {
        let gone = one_minus_exp(k * left);
        held(m + rate * left + (now - eq) * gone, dm + rate * d_left - d_eq * gone + (now - eq) * (1.0 - gone) * (d_k * left + k * d_left))
    } else {
        let d_t2 = (d_eq * (1.0 / (eq - now) - 1.0 / (eq - wet)) - t2 * d_k) / k;
        held(m + rate * t2 - (wet - now) + e * (left - t2), dm + (rate - e) * d_t2 + left - t2 + e * d_left)
    }
}

/// The cloud where `m` mol condense over `area` m² in `dt` seconds, a layer of droplets whose optical depth grows with
/// the rate: [its reflectance, its optical depth to heat radiation]. A layer of scattering depth x reflects x / (1 + x)
/// (two-stream, Lacis and Hansen 1974), half at the rate `cloud`, times the condensate's mean reflectance `mr / m` as
/// it scatters like its powder; the same droplets absorb heat radiation, with the depth `cloud_ir` at that rate
/// (Stephens 1978).
pub(super) fn cloud((cloud, cloud_ir): (f64, f64), (m, mr): (f64, f64), area: f64, dt: f64) -> [f64; 2] {
    let x = m / (area * dt) / cloud;
    if m > 0.0 { [x / (1.0 + x) * mr / m, cloud_ir * x] } else { [0.0; 2] }
}

/// The gases of the air that cannot condense on the ground, colder than `tmin` K at its coldest, and dissolve in its
/// liquid, each with its index in `State::gas`: chosen once a month, at the full state of the ground (`Scratch::henry`).
pub(in crate::flow) fn henry(w: &World, s: &Scratch, tmin: f64) -> Vec<(usize, usize)> {
    s.state.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect()
}

/// The gases chosen for the month (`Scratch::henry`) dissolve in each ground node's liquid toward Henry's share and
/// come out as it warms, thins or freezes, through the air's film and the liquid's in series, solved exactly over the
/// `dt` seconds of a month, at the end of the liquid step (laws.md 1.5): the air changes its gases over years, and the
/// low basin takes months to take them up. Each node in parallel with `threads`, at its temperature as the liquid step
/// leaves it (`State::tg`) and under the air of the last air step's start. Dissolving frees the vaporisation enthalpy
/// into the node; coming out takes it back. What crosses is booked in one call: a link from each ground node that takes
/// or gives some to its air node with the mol of each gas (+ up) and the heat that leaves the ground, no air node giving
/// more than it holds (in the order of the cells), and the lift of the gas, paid by the air (`potential`); each ground
/// node notes what it took up (`Scratch::hold`). The low basin under one air node takes up gas as one liquid
/// (`surf::patches`), each of its cells by its share of that liquid, and gives it up by what each holds of it.
pub(in crate::flow) fn dissolve(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let henry = &std::mem::take(&mut s.henry)[..];
    let (n, na, nh) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), henry.len());
    if nh == 0 {
        s.henry = henry.to_vec();
        return;
    }
    let patches = &patches(w, s)[..];
    let solv = s.liquids(&w.chem, (tmin, tmax), false);
    // What each ground node would take up of each gas over the step, mol (`[i * nh + x]`; negative: it comes out).
    let mut want = vec![0.0; n * nh];
    let (wr, sr) = (&*w, &*s);
    let mut alone = vec![true; n];
    patches.iter().flatten().for_each(|&i| alone[i as usize] = false);
    let (area, ground) = (wr.mesh.cells().area(), wr.store.amounts(Class::Ground));
    let liquid = |i: usize| solv.iter().fold((0.0, 0.0), |(l, v), &k| (l + ground[k * n + i] * wet(sr, k, i), v + ground[k * n + i] * wet(sr, k, i) * sr.sp[k].v[1]));
    crate::par::rows(&mut want, nh, || (), |_, i, row| {
        if alone[i] {
            let pool = Liquid { area: area[i], t: sr.state.tg[i], tl: touching(sr, &sr.ta, i), j: wr.mesh.cells().parent()[i] as usize, cap: sr.cap[i], lv: liquid(i) };
            take_up(wr, sr, &pool, |k| ground[k * n + i], (henry, dt), row);
        }
    });
    let shared = crate::par::map(patches, |cells| {
        let each: Vec<(f64, f64)> = cells.iter().map(|&i| liquid(i as usize)).collect();
        let (sum, by_area) = (|f: &dyn Fn(usize, usize) -> f64| cells.iter().enumerate().map(|(c, &i)| f(c, i as usize)).sum::<f64>(), area_of(cells, area));
        let cap = sum(&|_, i| sr.cap[i]);
        let pool = Liquid {
            area: by_area,
            t: sum(&|_, i| sr.state.tg[i] * sr.cap[i]) / cap,
            tl: sum(&|_, i| touching(sr, &sr.ta, i) * area[i]) / by_area,
            j: wr.mesh.cells().parent()[cells[0] as usize] as usize,
            cap,
            lv: (sum(&|c, _| each[c].0), sum(&|c, _| each[c].1)),
        };
        let held = |k: usize| sum(&|_, i| ground[k * n + i]);
        let mut row = vec![0.0; nh];
        take_up(wr, sr, &pool, held, (henry, dt), &mut row);
        // Each cell's share: of what is taken up, by its liquid; of what comes out, by what it holds.
        let mut rows = vec![0.0; cells.len() * nh];
        for (c, &i) in cells.iter().enumerate() {
            for (x, &(_, k)) in henry.iter().enumerate() {
                let share = if row[x] >= 0.0 { each[c].0 / pool.lv.0.max(f64::MIN_POSITIVE) } else { ground[k * n + i as usize] / held(k).max(f64::MIN_POSITIVE) };
                rows[c * nh + x] = row[x] * share;
            }
        }
        rows
    });
    for (cells, rows) in patches.iter().zip(shared) {
        cells.iter().zip(rows.chunks_exact(nh)).for_each(|(&i, r)| want[i as usize * nh..(i as usize + 1) * nh].copy_from_slice(r));
    }
    let ks: Vec<u16> = henry.iter().map(|h| h.1 as u16).collect();
    let parent = w.mesh.cells().parent();
    // What each air node holds of each gas as the crossing leaves it, mol (`[x * na + j]`); the cells that take or give
    // some, and per gas and cell what goes up, mol, then per cell the heat that leaves the ground, J.
    let mut left: Vec<f64> = henry.iter().flat_map(|h| w.store.amounts(Class::Air)[h.1 * na..(h.1 + 1) * na].iter().copied()).collect();
    let crossing: Vec<u32> = (0..n as u32).filter(|&i| want[i as usize * nh..(i as usize + 1) * nh].iter().any(|&m| m != 0.0)).collect();
    let (links, mut up) = (crossing.len(), vec![0.0; (nh + 1) * crossing.len()]);
    for (l, &i) in crossing.iter().enumerate() {
        let (i, j) = (i as usize, parent[i as usize] as usize);
        for (x, &(_, k)) in henry.iter().enumerate() {
            let (m, sp) = (want[i * nh + x].min(left[x * na + j]), s.sp[k]);
            // The gas brings its heat (the air's, or the ground's when it comes out) and frees its latent heat in the
            // node; `-m` mol go up from the ground node into its air node, and the ground node's state follows.
            let tq = if m > 0.0 { s.ta[j] } else { s.state.tg[i] };
            up[x * links + l] = -m;
            up[nh * links + l] -= m * (sp.cp * tq + sp.lf + sp.lv);
            left[x * na + j] -= m;
        }
    }
    let to: Vec<u32> = crossing.iter().map(|&i| parent[i as usize]).collect();
    let (mol, heat) = up.split_at(nh * links);
    apply(w, s, &[LinkFlux { class: [Class::Ground, Class::Air], a: &crossing, b: &to, ks: &ks, mol, heat }], &[]);
    for &(_, k) in henry {
        if (0..n).any(|i| w.store.amounts(Class::Ground)[k * n + i] > 0.0) {
            s.add_loose(k, true);
        }
    }
    // Each ground node's heat capacity and fusion enthalpy follow what it took up (it takes no room).
    for (x, &(_, k)) in henry.iter().enumerate() {
        crossing.iter().zip(&mol[x * links..(x + 1) * links]).for_each(|(&i, &m)| s.hold(k, i as usize, -m));
    }
    s.henry = henry.to_vec();
}

/// The liquid a ground takes up gas into (`take_up`): its area, m², temperature, K, the air touching it, K, its air
/// node, its heat capacity, J/K, and its liquid, mol, with that liquid's volume, m³.
struct Liquid {
    area: f64,
    t: f64,
    tl: f64,
    j: usize,
    cap: f64,
    lv: (f64, f64),
}

/// The area of the cells `cells`, m².
fn area_of(cells: &[u32], area: &[f64]) -> f64 {
    cells.iter().map(|&i| area[i as usize]).sum()
}

/// What the liquid `pool` holding `held(k)` mol of each gas takes up of each gas of `henry` over `dt` seconds toward
/// Henry's share under its air node's column over it, through the air's film and the liquid's in series, into `row`
/// (negative: it comes out, cooling it no further than the boiling point).
fn take_up(w: &World, s: &Scratch, pool: &Liquid, held: impl Fn(usize) -> f64, (henry, dt): (&[(usize, usize)], f64), row: &mut [f64]) {
    let (na, air, (l, v)) = (w.mesh.level(AIR).n(), w.mesh.level(AIR), pool.lv);
    for (x, &(gi, k)) in henry.iter().enumerate().filter(|h| l > 0.0 || held(h.1 .1) > 0.0) {
        // The column of its air node over it, its height, mol dissolved per mol at balance, and the time to balance.
        let (col, hk) = (w.store.amounts(Class::Air)[k * na + pool.j] * pool.area / air.area()[pool.j], s.hk[gi * na + pool.j]);
        let b = l * w.chem.henry(k, pool.t, R_GAS * pool.tl / (pool.area * hk));
        let tau = b * hk / w.cfg.evap + v / pool.area / w.cfg.dissolve;
        let a = held(k);
        let m = (b * (a + col) / (1.0 + b) - a) * one_minus_exp(dt * (1.0 + b) / tau);
        row[x] = if m >= 0.0 { m } else { -(-m).min(boil_off(&s.sp[k], pool.cap, pool.t)) };
    }
}

#[cfg(test)]
mod tests {
    use super::bucket;

    #[test]
    fn the_bucket_slope_is_its_derivative() {
        // Ground that stays wet, dries to `wet`, dries further, wets again under rain, and has no bucket; with the
        // derivative by central differences.
        let dt = 1e5;
        for (a, r, e, wet) in [(10.0, 0.0, 1e-5, 1.0), (2.0, 0.0, 1e-4, 1.0), (0.5, 0.0, 1e-5, 1.0), (0.5, 0.3, 1e-5, 1.0), (0.5, 3.0, 1e-5, 1.0), (2.0, 0.5, 1e-4, 0.0), (0.0, 3.0, 1e-5, 0.0)] {
            let (m, slope) = bucket(a, r, e, wet, dt);
            let h = e * 1e-6;
            let fd = (bucket(a, r, e + h, wet, dt).0 - bucket(a, r, e - h, wet, dt).0) / (2.0 * h);
            assert!(m > 0.0 && m <= a + r, "{m}");
            assert!((slope - fd).abs() <= 1e-6 * fd.abs().max(dt * 1e-3), "a {a} r {r} e {e} wet {wet}: {slope} against {fd}");
        }
    }
}
