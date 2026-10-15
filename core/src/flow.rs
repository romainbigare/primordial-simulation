//! Law 1: Flow. Stuff moves along edges from high push to low push, with loss.
//!
//! Ground nodes (here): liquid soaks into the pores of the loose ground up to their volume (the
//! porosity times the volume of the loose solids) and what is over that stands and runs down its
//! head in one ordered sweep; the liquid in the pores flows to neighbours by Darcy's law; the head
//! it loses heats the ground and does work, carrying loose solids and breaking weak bonds; loose
//! solids creep downhill and heat conducts. Air nodes (`air`): radiation, contact heat,
//! evaporation and condensation, and gas moved by pressure, overturning and mixing.
//! Every move takes from one node exactly what it gives to another, with its heat, and
//! books matter lifted or dropped in the gravity ledger, so elements and energy are kept.

pub(crate) mod air;
pub use air::{column, glow};

use crate::grid::{DIRS, FORWARD};
use crate::World;
use serde::{Deserialize, Serialize};

/// Moving liquid does work on its bed (carrying, wear) once in this many months (seven,
/// so the season sampled turns over the years).
const WORK_MONTHS: u64 = 7;
/// The slow-changing ground and the substances present are rechecked this often, months.
const REFRESH_MONTHS: u64 = 3;

/// What the bodies on cell `i` leave of carrying and creep there (1 when bare).
fn held(w: &World, i: usize) -> f64 {
    1.0 / (1.0 + w.life.sum.get(i).map_or(0.0, |s| s[1]) / w.grid.area() / w.cfg.phys.hold)
}

/// Numbers of one substance the laws read often: heat capacity, molar mass, fusion and
/// vaporisation enthalpy, molar volume as [solid, liquid], melting and boiling point,
/// reflectance as [solid, liquid].
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Sp {
    cp: f64,
    mm: f64,
    lf: f64,
    lv: f64,
    cond: f64,
    v: [f64; 2],
    melt: f64,
    boil: f64,
    refl: [f64; 2],
}

/// Working memory of the law, rebuilt from the world when missing. It is saved with the
/// world (but for the space used only within a call, and the tables that depend on the grid and
/// the chemistry alone, which `tables` rebuilds on load), as the slower parts of the law run
/// from what it held at their last turn: a resumed run must go on exactly as an unbroken one.
///
/// Who writes and who reads each field (R `refresh`, G `ground_state`, S `air::state`, Rad
/// `air::radiation`, X `air::exchange`, W `air::wind`, L `liquid`, C `creep`, Law 2 `change`
/// through `view`). Ground fields are per ground node, air fields per air node; "all" is every part.
///
/// | field | unit | written by | read by |
/// | --- | --- | --- | --- |
/// | `month`, `year`, `pending`, `pending_liq`, `pending_exc`, `owe_work` | month, year, s, s, s, flag | `step` | `step` |
/// | `area`, `sp` | m², `Sp` per substance | `tables` (on load) | all |
/// | `loose`, `fixed`, `vary`, `bedk`, `gas` | substance lists | R (`loose`, `vary` also X, L) | all |
/// | `fix` | m³ (volume, volume × reflectance) | R | `surface` |
/// | `bed` | [J/K skin, m³, reflectance, W/(m K)] | R (L wears `[1]`) | G, Rad (via `alb`), C, Law 2 |
/// | `cap` | J/K | G (every step), X, L, C (moves of heat capacity) | G, Rad, X, L, C |
/// | `tg` | K | G | all, Law 2, bodies (`temps`) |
/// | `pk`, `phi` | substance, share | G | `molten`, Rad |
/// | `hold` | [J held, J low, J high, stage] | G, L (`[0]`), `lift` (marks stale) | G, L |
/// | `z`, `head` | m (solid surface, surface of all the liquid, pores filled first) | `surface`, `settle`, `lift` | S, Rad, X, L, C |
/// | `pd` | m (liquid the pores of the loose ground hold, over the cell's area) | `surface` | L |
/// | `alb`, `rc` | reflectance, m² K/W | `surface` | Rad |
/// | `rain` | mol, condensed this call | X | X (to `diag.rain`) |
/// | `order` | cell keys by head | L | L |
/// | `nbr`, `done` | neighbour cells, visited flag | `space`; L | L |
/// | `sed`, `sed_e`, `mob` | mol carried, J carried, m³ mobile | L (`sed`, `sed_e`); C (`mob`) | L; C |
/// | `air_of`, `near`, `wt`, `per` | ground cell to air nodes and weights, share | `tables` (on load) | S, Rad, X, `lerp` |
/// | `tl` | K, air touching the ground | Rad | X |
/// | `seen`, `lay` | [drop/m, air height, absorbing share, cloud reflectance, air K]; K below the ground's air, per layer | S (X sets `[3]`) | Rad |
/// | `ta0` | K, air at the last S | S | Rad |
/// | `cloud`, `snap` | [mol, mol × reflectance]; mol per gas | X | X |
/// | `cells`, `za`, `vol` | count, m, m³ | `tables`; S | S, W |
/// | `nt`, `capa`, `mass` | mol, J/K, kg | S; X, W (moves) | X, W, Rad |
/// | `fabs`, `gam`, `zair`, `ta` | optical depth, K/m, m, K | S (`ta` also Rad, W) | Rad, X, W |
/// | `node_tmp` | reused: J/K (S), J (Rad), reflectance (X), Pa (W) | S, Rad, X, W | the same part |
/// | `warmth`, `grad` | K at equal height, K per node | W | W |
/// | `cond`, `hk`, `split` | may condense, m, share (per gas) | S | X, W |
/// | `nt_new` | mol, amounts after the pushes of a direction | W | W |
/// | `rad` | a row of numbers per ground cell | Rad | Rad |
/// | `lanes` | a row of fluxes per line of air nodes | W | W |
/// | `trace` | s, calls | X | X |
/// | `slow` | [tick last, tick next, count] | Law 2 | Law 2 |
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scratch {
    month: Option<u64>,
    year: Option<u64>,
    /// Seconds since air and rain last moved, and since liquid (and creep) last moved.
    pending: f64,
    pending_liq: f64,
    pending_exc: f64,
    /// The substances present changed: rebuild the state of ground and air at the next step.
    #[serde(skip)]
    stale: bool,
    /// Moving liquid's work on its bed is due at the next liquid step.
    owe_work: bool,
    /// Tables of the grid and the chemistry alone (not saved): ground cell area, substance
    /// constants, and the map from ground cells to air nodes (own node, four nearest nodes and
    /// weights, ground cells per air node, share of a node).
    #[serde(skip)]
    area: f64,
    #[serde(skip)]
    sp: Vec<Sp>,
    /// Substances present in ground, bonded ground and air nodes. Loose ones that stay solid
    /// and cannot evaporate at any ground temperature are `fixed`: their volume and volume ×
    /// reflectance per node are kept between refreshes in `fix`; the others `vary`.
    loose: Vec<usize>,
    fixed: Vec<usize>,
    vary: Vec<usize>,
    fix: Vec<[f64; 2]>,
    bedk: Vec<usize>,
    gas: Vec<usize>,
    // Ground nodes: bonded ground [heat capacity of the skin, volume, reflectance,
    // conductivity]; heat capacity, temperature, solid surface, head, reflectance.
    bed: Vec<[f64; 4]>,
    cap: Vec<f64>,
    tg: Vec<f64>,
    /// Substance held at its melting point in each ground node (or `NONE`), and its molten share.
    pk: Vec<usize>,
    phi: Vec<f64>,
    /// Off a plateau: the fusion enthalpy held, and the energies between which it holds
    /// (until the amounts change); and how many of `vary` are molten.
    hold: Vec<[f64; 4]>,
    z: Vec<f64>,
    head: Vec<f64>,
    /// Pore space of the loose ground, as a depth of liquid over the cell (the porosity times the
    /// volume of its loose solids over the area): the first liquid of a cell fills it.
    #[serde(default)]
    pd: Vec<f64>,
    alb: Vec<f64>,
    /// Heat resistance of the soft solid cover (depth / conductivity), m² K / W.
    rc: Vec<f64>,
    #[serde(skip)]
    rain: Vec<f64>,
    order: Vec<u64>,
    /// Neighbours of each ground node along `DIRS` (`NONE` off the map).
    #[serde(skip)]
    nbr: Vec<[u32; 8]>,
    #[serde(skip)]
    done: Vec<bool>,
    /// Loose solids carried by liquid, [cell * carried kinds + kind], and their heat.
    #[serde(skip)]
    sed: Vec<f64>,
    #[serde(skip)]
    sed_e: Vec<f64>,
    #[serde(skip)]
    mob: Vec<f64>,
    // The temperature of the air touching the ground.
    #[serde(skip)]
    air_of: Vec<u32>,
    #[serde(skip)]
    near: Vec<[u32; 4]>,
    #[serde(skip)]
    wt: Vec<[f64; 4]>,
    #[serde(skip)]
    per: Vec<f64>,
    tl: Vec<f64>,
    /// Per ground cell, from the last full step: temperature drop per metre, mean height of
    /// the air's mass, heat-absorbing share, sunlight reflected by condensate in the air,
    /// air temperature; and the temperature of each air node then.
    seen: Vec<[f64; 5]>,
    /// Per ground cell, from the last full step: how much colder than the air at the ground each
    /// layer of the air is that sends heat radiation [down, up] (`air::column`).
    #[serde(default)]
    lay: Vec<[[f64; air::LAYERS]; 2]>,
    ta0: Vec<f64>,
    /// Per air node: condensed in the last full step, mol, and × its reflectance.
    #[serde(skip)]
    cloud: Vec<[f64; 2]>,
    #[serde(skip)]
    snap: Vec<f64>,
    // Air nodes: ground cells (a table), mean ground height, volume, amount, heat capacity, mass,
    // optical depth, temperature drop per metre, mean height of the air's mass,
    // temperature, pressure, warmth at equal height and its gradient.
    #[serde(skip)]
    cells: Vec<f64>,
    za: Vec<f64>,
    vol: Vec<f64>,
    nt: Vec<f64>,
    capa: Vec<f64>,
    mass: Vec<f64>,
    fabs: Vec<f64>,
    gam: Vec<f64>,
    zair: Vec<f64>,
    ta: Vec<f64>,
    #[serde(skip)]
    node_tmp: Vec<f64>,
    #[serde(skip)]
    warmth: Vec<f64>,
    #[serde(skip)]
    grad: Vec<[f64; 2]>,
    /// Per gas [g * na + j]: may it condense in the column; height its amount fills near the
    /// ground, m; share of it in the upper half of the column.
    cond: Vec<bool>,
    hk: Vec<f64>,
    split: Vec<f64>,
    // Amounts of air nodes after the pushes of a direction, mol; the rows of numbers that each
    // ground cell (`air::radiation`) and each line of air nodes (`air::wind`) make in a call.
    #[serde(skip)]
    nt_new: Vec<f64>,
    #[serde(skip)]
    rad: Vec<f64>,
    #[serde(skip)]
    lanes: Vec<f64>,
    /// Time since trace gases last evaporated and condensed, s, and the calls since.
    trace: [f64; 2],
    /// Law 2 (`change`): per ground node, the tick it last reacted slowly, the tick it next does,
    /// and the steps it reacted on its own since.
    pub(crate) slow: Vec<[u32; 3]>,
}

impl Scratch {
    /// Temperature of each ground node and of each air node at the last flow step, K (empty before it).
    pub fn temps(&self) -> (&[f64], &[f64]) {
        (&self.tg, &self.ta)
    }
    /// For Law 2, from the last flow step: per ground node its temperature, the heat capacity of
    /// its bonded ground's skin (as of the last refresh) and its air node; per air node its
    /// temperature. Empty before the first step.
    pub(crate) fn view(&self) -> (&[f64], &[[f64; 4]], &[u32], &[f64]) {
        (&self.tg, &self.bed, &self.air_of, &self.ta)
    }
}

/// Advance flow by `dt` seconds: radiation every call; air and rain once `cfg.air_days` have
/// passed, liquid and creep once `cfg.flow_days` have, each for all the time since it last moved.
pub fn step(w: &mut World, dt: f64) {
    let mut s = std::mem::take(&mut w.scratch);
    if s.air_of.len() != w.grid.n() {
        tables(w, &mut s);
    }
    let first = s.cap.len() != w.grid.n();
    if first {
        refresh(w, &mut s);
    }
    if s.done.len() != w.grid.n() {
        space(w, &mut s);
    }
    (s.pending, s.pending_liq) = (s.pending + dt, s.pending_liq + dt);
    let day = 86_400.0;
    let (air_due, liq_due) = (first || s.pending >= w.cfg.air_days as f64 * day - 1.0, first || s.pending_liq >= w.cfg.flow_days as f64 * day - 1.0);
    // The state of ground and air is rebuilt at liquid steps (and when a refresh changed what is
    // present, or a snapshot from before 13 October 2026 has no air layers).
    let full = liq_due || std::mem::take(&mut s.stale) || s.lay.len() != w.grid.n();
    ground_state(w, &mut s, full);
    air::state(w, &mut s, full);
    air::radiation(w, &mut s, dt);
    if !air_due {
        w.scratch = s;
        return;
    }
    let (dt, month) = (std::mem::take(&mut s.pending), w.day / w.cfg.days_per_month as u64);
    let monthly = s.month != Some(month);
    s.pending_exc += dt;
    if first || s.pending_exc >= w.cfg.exchange_days as f64 * day - 1.0 {
        let dt = std::mem::take(&mut s.pending_exc);
        air::exchange(w, &mut s, dt);
    }
    air::wind(w, &mut s, dt);
    s.owe_work |= monthly && month % WORK_MONTHS == 0;
    if liq_due {
        let dt = std::mem::take(&mut s.pending_liq);
        let work = if std::mem::take(&mut s.owe_work) { (WORK_MONTHS * w.cfg.days_per_month as u64) as f64 * day / dt } else { 0.0 };
        liquid(w, &mut s, dt, work);
        drain(w, &mut s, dt);
        let year = w.day / w.days_per_year();
        if s.year != Some(year) {
            if s.year.is_some() {
                creep(w, &mut s, w.days_per_year() as f64 * day);
            }
            s.year = Some(year);
        }
    }
    if monthly && month % REFRESH_MONTHS == 0 {
        refresh(w, &mut s);
    }
    s.month = Some(month);
    w.scratch = s;
}

const NONE: usize = usize::MAX;

/// Molten share of substance `k` in ground node `i` (liquid, or gas above boiling).
#[inline]
fn molten(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.pk[i] == k { s.phi[i] } else { (s.tg[i] >= s.sp[k].melt) as u8 as f64 }
}

/// Liquid share of substance `k` in ground node `i`.
#[inline]
fn wet(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.tg[i] < s.sp[k].boil { molten(s, k, i) } else { 0.0 }
}

/// Rise of the solid surface and of the head of a ground node at `t` for `m` mol of a
/// substance (negative `m`: fall).
#[inline]
fn rise(p: &Sp, t: f64, m: f64, area: f64) -> [f64; 2] {
    if t < p.melt {
        let d = m * p.v[0] / area;
        [d, d]
    } else if t < p.boil {
        [0.0, m * p.v[1] / area]
    } else {
        [0.0; 2]
    }
}

/// Raise (or lower) the surface of ground node `i` by `[solid, head]` m, and mark it for a new surface.
#[inline]
fn lift(s: &mut Scratch, i: usize, [dz, dh]: [f64; 2]) {
    s.hold[i][2] = f64::MIN;
    s.z[i] += dz;
    s.head[i] += dh;
}

/// Raise (or with negative `m`, lower) the surface of ground node `i` for `m` mol of `k`.
#[inline]
fn settle(s: &mut Scratch, k: usize, i: usize, m: f64) {
    let d = rise(&s.sp[k], s.tg[i], m, s.area);
    lift(s, i, d);
}

fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

/// Neighbours of cell `i` along `dirs`, or `usize::MAX` off the map.
#[inline]
fn around<const N: usize>(dirs: &[(i32, i32); N], gw: usize, gh: usize, i: usize) -> [usize; N] {
    let (x, y) = ((i % gw) as i32, (i / gw) as i32);
    let inside = x > 0 && y > 0 && x + 1 < gw as i32 && y + 1 < gh as i32;
    dirs.map(|(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        if inside || (nx >= 0 && ny >= 0 && (nx as usize) < gw && (ny as usize) < gh) {
            ny as usize * gw + nx as usize
        } else {
            usize::MAX
        }
    })
}

/// Every few months: which substances are present, the slow-changing bonded ground and fixed
/// solids, and (once) the state vectors.
fn refresh(w: &World, s: &mut Scratch) {
    let (n, na, ns) = (w.grid.n(), w.grid.na(), w.chem.ns());
    let any = |v: &[f64], len: usize, k: usize| v[k * len..(k + 1) * len].iter().any(|&x| x > 0.0);
    let tmax = if s.tg.len() == n { range(&s.tg).1 } else { (0..n).map(|i| w.t_ground(i)).fold(0.0, f64::max) } + 20.0;
    let up = |k: usize| w.chem.p_sat(k, tmax) > 1.0;
    s.loose = (0..ns).filter(|&k| any(&w.ground, n, k)).collect();
    s.fixed = s.loose.iter().copied().filter(|&k| w.cfg.phys.soft * s.sp[k].melt > tmax && !up(k)).collect();
    s.vary = s.loose.iter().copied().filter(|k| !s.fixed.contains(k)).collect();
    s.vary.sort_by(|&a, &b| s.sp[a].melt.total_cmp(&s.sp[b].melt));
    s.order.clear();
    s.bedk = (0..ns).filter(|&k| any(&w.bed, n, k)).collect();
    let gas = (0..ns).filter(|&k| any(&w.air, na, k) || (s.loose.contains(&k) && up(k))).collect();
    s.stale |= gas != s.gas;
    s.gas = gas;
    s.fix = vec![[0.0; 2]; n];
    for &k in &s.fixed {
        let (v, r) = (s.sp[k].v[0], s.sp[k].refl[0]);
        for (f, &a) in s.fix.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            f[0] += a * v;
            f[1] += a * v * r;
        }
    }
    s.bed = (0..n)
        .map(|i| {
            let (mut c, mut v, mut r, mut kc) = (0.0, 0.0, 0.0, 0.0);
            for &k in &s.bedk {
                let (b, sub) = (w.bed[k * n + i], &w.chem.subs[k]);
                c += b * sub.cp;
                v += b * s.sp[k].v[0];
                r += b * s.sp[k].v[0] * s.sp[k].refl[0];
                kc += b * s.sp[k].v[0] * sub.conductivity;
            }
            let c = if v > 0.0 { c / v * w.cfg.skin_m * s.area } else { 0.0 };
            [c, v, r / v.max(1e-9), kc / v.max(1e-9)]
        })
        .collect();
    if s.cap.len() == n {
        return;
    }
    (s.pk, s.phi, s.hold) = (vec![NONE; n], vec![0.0; n], vec![[0.0, 0.0, f64::MIN, 0.0]; n]);
    for v in [&mut s.cap, &mut s.tg, &mut s.z, &mut s.head, &mut s.pd, &mut s.alb, &mut s.rc, &mut s.tl] {
        *v = vec![0.0; n];
    }
    for v in [&mut s.za, &mut s.vol, &mut s.nt, &mut s.capa, &mut s.mass, &mut s.fabs, &mut s.gam, &mut s.zair, &mut s.ta, &mut s.ta0] {
        *v = vec![0.0; na];
    }
    s.seen = vec![[0.0; 5]; n];
}

/// Tables that depend on the grid and the chemistry alone: ground cell area, substance constants,
/// and the map from ground cells to air nodes. Rebuilt when missing (they are not saved).
fn tables(w: &World, s: &mut Scratch) {
    let (n, na, g) = (w.grid.n(), w.grid.na(), &w.grid);
    let mean = |v: [f32; 3]| v.iter().map(|&x| x as f64).sum::<f64>() / 3.0;
    s.area = g.area();
    s.sp = (w.chem.subs.iter())
        .map(|b| Sp {
            cp: b.cp,
            mm: b.molar_mass,
            lf: b.lf,
            lv: b.lv,
            cond: b.conductivity.max(1e-3),
            v: [b.molar_mass / b.density[0], b.molar_mass / b.density[1]],
            melt: b.melt,
            boil: b.boil,
            refl: [mean(b.refl[0]), mean(b.refl[1])],
        })
        .collect();
    s.air_of = (0..n).map(|i| g.air_of(i) as u32).collect();
    let axis = |c: usize, len: usize| {
        let f = ((c as f64 + 0.5) / g.air_scale as f64 - 0.5).clamp(0.0, (len - 1) as f64);
        let a = (f as usize).min(len.saturating_sub(2));
        (a, (a + 1).min(len - 1), f - a as f64)
    };
    (s.near, s.wt) = (0..n)
        .map(|i| {
            let (x, y) = g.xy(i);
            let ((x0, x1, tx), (y0, y1, ty)) = (axis(x, g.aw()), axis(y, g.ah()));
            let j = |x: usize, y: usize| (y * g.aw() + x) as u32;
            ([j(x0, y0), j(x1, y0), j(x0, y1), j(x1, y1)], [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty])
        })
        .unzip();
    s.cells = vec![0.0; na];
    for &j in &s.air_of {
        s.cells[j as usize] += 1.0;
    }
    s.per = (0..n).map(|i| 1.0 / air::lerp(s, &s.cells, i)).collect();
}

/// Space used only within a call, and the neighbour table (not saved).
fn space(w: &World, s: &mut Scratch) {
    let (n, na) = (w.grid.n(), w.grid.na());
    s.nbr = (0..n).map(|i| around(&DIRS, w.grid.w, w.grid.h, i).map(|j| j as u32)).collect();
    for v in [&mut s.rain, &mut s.sed_e, &mut s.mob] {
        *v = vec![0.0; n];
    }
    (s.node_tmp, s.warmth) = (vec![0.0; na], vec![0.0; na]);
    (s.done, s.grad, s.cloud) = (vec![false; n], vec![[0.0; 2]; na], vec![[0.0; 2]; na]);
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node. Between
/// full steps the amounts change little: only nodes whose phases change (on or off a
/// melting plateau, or past a melting point) get a new surface.
fn ground_state(w: &World, s: &mut Scratch, full: bool) {
    // Heat capacity from the amounts now, at every step: bodies and Law 2 also move loose matter
    // (with its heat) between liquid steps, and a node read with the capacity it had before
    // would seem far too hot or cold (then evaporation, say, takes heat it does not hold).
    let n = w.grid.n();
    let mut cap: Vec<f64> = s.bed.iter().map(|b| b[0]).collect();
    for &k in &s.loose {
        let cp = s.sp[k].cp;
        for (c, &a) in cap.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            *c += a * cp;
        }
    }
    // Temperature: molten amounts hold their fusion enthalpy, so a node stays at a melting
    // point (a plateau) until all of that substance has frozen or melted.
    for i in 0..n {
        let same = (cap[i] - s.cap[i]).abs() <= 1e-9 * cap[i];
        s.cap[i] = cap[i];
        let (c, e, [held, lo, hi, was]) = (s.cap[i].max(1.0), w.e_ground[i], s.hold[i]);
        if !full && same && e >= lo && e < hi {
            s.tg[i] = (e - held) / c;
            continue;
        }
        let (mut held, mut lo, mut hi, pk) = (0.0, f64::MIN, f64::MAX, s.pk[i]);
        s.pk[i] = NONE;
        let mut stage = 0.0;
        for &k in &s.vary {
            let (p, a) = (&s.sp[k], w.ground[k * n + i]);
            let at = c * p.melt + held;
            if e < at {
                hi = at;
                break;
            }
            if a > 0.0 && e < at + a * p.lf {
                (s.pk[i], s.phi[i], hi) = (k, (e - at) / (a * p.lf), f64::MIN);
                break;
            }
            held += a * p.lf;
            lo = at + a * p.lf;
            stage += 1.0;
        }
        s.hold[i] = [held, lo, hi, stage];
        s.tg[i] = if s.pk[i] == NONE { (e - held) / c } else { s.sp[s.pk[i]].melt };
        if full || pk != NONE || s.pk[i] != NONE || stage != was {
            surface(w, s, i);
        }
    }
}

/// Solid surface, head, heat resistance of the soft cover and reflectance of node `i`, from
/// the volume and volume × reflectance of its layers [soft solid, liquid, loose solid].
fn surface(w: &World, s: &mut Scratch, i: usize) {
    let (n, area, t) = (w.grid.n(), s.area, s.tg[i]);
    let mut l = [0.0, 0.0, 0.0, 0.0, s.fix[i][0], s.fix[i][1]];
    let (mut z, mut rc) = (w.base_z + (s.bed[i][1] + s.fix[i][0]) / area, 0.0);
    for &k in &s.vary {
        let (p, a) = (&s.sp[k], w.ground[k * n + i]);
        if a <= 0.0 || t >= p.boil {
            continue;
        }
        let liq = molten(s, k, i);
        let solid = a * (1.0 - liq) * p.v[0];
        let o = if t > w.cfg.phys.soft * p.melt { 0 } else { 4 };
        (l[o], l[o + 1]) = (l[o] + solid, l[o + 1] + solid * p.refl[0]);
        (l[2], l[3]) = (l[2] + a * liq * p.v[1], l[3] + a * liq * p.v[1] * p.refl[1]);
        z += solid / area;
        if o == 0 {
            rc += solid / area / p.cond;
        }
    }
    let (mut a, mut seen) = (0.0, 1.0);
    for c in 0..3 {
        if l[2 * c] > 0.0 {
            let d = l[2 * c] / area;
            let cover = d / (d + w.cfg.phys.hide[c]);
            a += seen * cover * l[2 * c + 1] / l[2 * c];
            seen *= 1.0 - cover;
        }
    }
    (s.z[i], s.head[i], s.rc[i], s.alb[i]) = (z, z + l[2] / area, rc, a + seen * s.bed[i][2]);
    s.pd[i] = w.cfg.phys.porosity * (l[0] + l[4]) / area;
}

/// Head of the liquid that stands on node `i` (the solid surface when none does), m: the liquid
/// over what the pores hold.
#[inline]
fn stand(s: &Scratch, i: usize) -> f64 {
    (s.head[i] - s.pd[i]).max(s.z[i])
}

/// Head of the liquid table of node `i`, m (Darcy's law reads it): in a node whose pores are not
/// full, the bottom of its loose ground plus the depth they fill; else the head that stands.
#[inline]
fn table(s: &Scratch, i: usize, por: f64) -> f64 {
    let full = s.head[i] - s.z[i] >= s.pd[i];
    if full { s.head[i] - s.pd[i] } else { s.z[i] - s.pd[i] / por + (s.head[i] - s.z[i]) / por }
}

/// Liquid by head, as one ordered sweep from the highest head down: each node passes liquid
/// to its lower neighbours, more to the steeper, so it can cross the map in one sweep while
/// depressions fill and spill. The head lost heats the node. Every few months (`work` > 0,
/// their share of this call) the power lost also does work: it carries loose solids up to
/// a capacity, drops the rest, and breaks bonded ground by power over bond strength.
fn liquid(w: &mut World, s: &mut Scratch, dt: f64, work: f64) {
    let (n, area, g) = (w.grid.n(), s.area, w.cfg.gravity);
    let inv = DIRS.map(|d| 1.0 / w.grid.dist(d));
    let (tmin, tmax) = range(&s.tg);
    // (With the gases dissolved in it, which are gas at every node: `air::dissolve`.)
    let liq: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt <= tmax && (s.sp[k].boil > tmin || w.chem.subs[k].solubility > 0.0)).collect();
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let ns = sol.len();
    s.sed.resize(n * ns, 0.0);
    // Highest head first (heads to 0.1 m, then by cell). The order is kept between refreshes:
    // any order is safe, it only sets how far liquid gets in one call.
    if s.order.len() != n {
        s.order = (0..n).map(|i| (((2e4 - stand(s, i)).max(0.0) * 10.0) as u64) << 20 | i as u64).collect();
        s.order.sort_unstable();
    }
    s.done.fill(false);
    let ema = (dt / (w.days_per_year() as f64 * 86_400.0)) as f32;
    let mut heat = 0.0;
    let mut wa = vec![0.0; liq.len()];
    for r in 0..n {
        let i = (s.order[r] & 0xf_ffff) as usize;
        let (t, hd, zi) = (s.tg[i], stand(s, i), s.z[i]);
        s.done[i] = true;
        // The head above the surface is the liquid standing here, and liquid in the pores is not at rest
        // either: it flows by Darcy's law (`seep`).
        let damp = s.pd[i] > 0.0 && s.head[i] > zi;
        if hd <= zi && !damp && work <= 0.0 {
            w.diag.liquid_flux[i] *= 1.0 - ema;
            continue;
        }
        // Lower neighbours, and how far this node's head may fall for each: down to a
        // neighbour still to come (which passes the liquid on) or to its own bed; half way
        // to one already done, so heads never cross.
        let (mut nb, mut q, mut room) = ([(0usize, 0.0f64); 8], [0.0f64; 8], [0.0f64; 8]);
        let (mut cnt, mut wsum, mut rmax, mut qmax) = (0, 0.0, 0.0f64, 0.0f64);
        for (d, &j) in s.nbr[i].iter().enumerate() {
            if j == u32::MAX || hd <= zi {
                continue;
            }
            let (j, hj) = (j as usize, stand(s, j as usize));
            let (dh, r) = (hd - hj, if s.done[j as usize] { 0.5 * (hd - hj) } else { hd - hj.max(zi) });
            if dh > w.cfg.phys.still && r > 0.0 {
                (nb[cnt], room[cnt], q[cnt]) = ((j, dh), r, dh * inv[d]);
                (wsum, rmax, qmax) = (wsum + q[cnt], rmax.max(r), qmax.max(q[cnt]));
                cnt += 1;
            }
        }
        if cnt == 0 && !damp && work <= 0.0 {
            w.diag.liquid_flux[i] *= 1.0 - ema;
            continue;
        }
        let mut lvol = 0.0;
        for (a, &k) in wa.iter_mut().zip(&liq) {
            // Dissolved gas moves with the liquid and adds no volume.
            let gas = s.sp[k].boil <= tmin;
            *a = w.ground[k * n + i] * if gas { 1.0 } else { wet(s, k, i) };
            lvol += if gas { 0.0 } else { *a * s.sp[k].v[1] };
        }
        // Neighbours much less steep than the steepest get nothing.
        for c in 0..cnt {
            if q[c] < w.cfg.phys.split * qmax {
                wsum -= q[c];
                q[c] = 0.0;
            }
        }
        // What leaves carries the node's heat at its temperature now: radiation and evaporation
        // earlier in this call may have cooled it below `tg`, and liquid that came in from above
        // mixed in at its own temperature (its fusion enthalpy is counted held as it moves, so
        // a hot node that a flood of cooler liquid crosses does not send it on at the old
        // temperature, more heat than it holds).
        let tcur = t.min((w.e_ground[i] - s.hold[i][0]) / s.cap[i].max(1.0)).max(0.0);
        let (mut power, mut out) = (0.0, 0.0);
        // Only the liquid over the pores runs off (`qs`, the load it carries goes with it alone); each
        // neighbour's drop in head, for the power lost.
        let (mut fall, mut qs, mut qss) = ([0.0; 8], [0.0; 8], 0.0);
        if lvol > 0.0 && cnt > 0 {
            let x = (lvol - s.pd[i] * area).max(0.0).min(area * rmax) / wsum;
            for c in 0..cnt {
                q[c] = (x * q[c]).min(area * room[c]);
                (fall[c], qs[c]) = (nb[c].1, q[c]);
                qss += q[c];
            }
        }
        // The pore liquid flows on by Darcy's law: through the thickness of ground the liquid table fills, at
        // `seep` times the slope of its head (to the neighbour with the lower table, no more than half way to a
        // level table), with the same liquid and heat as the standing liquid.
        if lvol > 0.0 && damp {
            let por = w.cfg.phys.porosity;
            let ti = table(s, i, por);
            for (d, &j) in s.nbr[i].iter().enumerate() {
                if j == u32::MAX {
                    continue;
                }
                let j = j as usize;
                let dh = ti - table(s, j, por);
                let b = ti - (s.z[i] - s.pd[i] / por).max(s.z[j] - s.pd[j] / por);
                if dh <= w.cfg.phys.still || b <= 0.0 {
                    continue;
                }
                let (si, sj) = (area * if s.head[i] - zi >= s.pd[i] { 1.0 } else { por }, area * if s.head[j] - s.z[j] >= s.pd[j] { 1.0 } else { por });
                let v = (w.cfg.phys.seep * b * w.grid.cell_m * dh / w.grid.dist(DIRS[d]) * dt).min(0.5 * dh / (1.0 / si + 1.0 / sj));
                if v <= 0.0 {
                    continue;
                }
                let c = (0..cnt).find(|&c| nb[c].0 == j).unwrap_or_else(|| {
                    (nb[cnt], q[cnt], fall[cnt]) = ((j, 0.0), 0.0, 0.0);
                    cnt += 1;
                    cnt - 1
                });
                fall[c] = (fall[c] * q[c] + dh * v) / (q[c] + v);
                q[c] += v;
            }
        }
        if lvol > 0.0 && cnt > 0 {
            let qsum: f64 = q[..cnt].iter().sum();
            let (part, inv) = ((qsum / lvol).min(1.0), 1.0 / qsum);
            for (&a, &k) in wa.iter().zip(&liq).filter(|p| *p.0 > 0.0) {
                let (p, m) = (s.sp[k], (a * part).min(w.ground[k * n + i]));
                // The liquid leaves with its heat and fusion enthalpy; less leaves if the node cannot pay.
                let want = m * (p.cp * tcur + p.lf);
                let e = crate::draw(&mut w.e_ground[i], &mut w.ledger, s.cap[i] - m * p.cp, want);
                let m = if want > 0.0 { m * crate::share(e, want) } else { m };
                (w.ground[k * n + i], s.cap[i]) = (w.ground[k * n + i] - m, s.cap[i] - m * p.cp);
                s.hold[i][0] -= if s.pk[i] != k { m * p.lf } else { 0.0 };
                settle(s, k, i, -m);
                out += m;
                for ((&(j, _), &dh), &q) in nb[..cnt].iter().zip(&fall).zip(&q).filter(|p| *p.1 > 0.0) {
                    let (m, e) = (m * q * inv, e * q * inv);
                    (w.ground[k * n + j], s.cap[j], w.e_ground[j]) = (w.ground[k * n + j] + m, s.cap[j] + m * p.cp, w.e_ground[j] + e);
                    s.hold[j][0] += if s.pk[j] != k { m * p.lf } else { 0.0 };
                    settle(s, k, j, m);
                    power += m * p.mm * g * dh;
                }
            }
        }
        w.e_ground[i] += power;
        heat += power;
        w.diag.liquid_flux[i] += ((out / dt) as f32 - w.diag.liquid_flux[i]) * ema;
        if work <= 0.0 {
            continue;
        }

        // The work, on the bed below the liquid left standing (which hides it): drop
        // what is over capacity (all of it where nothing flows on), else pick up loose solids
        // and break bonded ground; pass the load on with the flow.
        let bedwork = power * work * w.cfg.phys.hide[1] / (w.cfg.phys.hide[1] + (lvol - s.pd[i] * area - qss).max(0.0) / area);
        let load: f64 = sol.iter().enumerate().map(|(si, &k)| s.sed[i * ns + si] * s.sp[k].v[0]).sum();
        let capv = w.cfg.carry * bedwork * held(w, i);
        if load <= 0.0 && capv < 1.0 {
            continue;
        }
        if load > capv || qss <= 0.0 {
            let f = if qss > 0.0 { 1.0 - capv / load } else { 1.0 };
            for (si, &k) in sol.iter().enumerate() {
                let m = s.sed[i * ns + si] * f;
                s.sed[i * ns + si] -= m;
                w.ground[k * n + i] += m;
                s.cap[i] += m * s.sp[k].cp;
                settle(s, k, i, m);
            }
            w.e_ground[i] += s.sed_e[i] * f;
            s.sed_e[i] -= s.sed_e[i] * f;
        } else if bedwork > 0.0 {
            let solid = |k: usize| w.ground[k * n + i] * (1.0 - molten(s, k, i));
            let avail: f64 = sol.iter().map(|&k| solid(k) * s.sp[k].v[0]).sum();
            let f = if avail > 0.0 { ((capv - load) / avail).min(0.5) } else { 0.0 };
            for (si, &k) in sol.iter().enumerate() {
                if f > 0.0 {
                    let m = w.ground[k * n + i] * (1.0 - molten(s, k, i)) * f;
                    // Picked up with its heat; less is picked up if the node cannot pay.
                    let want = m * s.sp[k].cp * tcur;
                    let e = w.take_heat_with(i, want, m * s.sp[k].cp);
                    let m = if want > 0.0 { m * crate::share(e, want) } else { m };
                    w.ground[k * n + i] -= m;
                    s.sed[i * ns + si] += m;
                    s.cap[i] -= m * s.sp[k].cp;
                    s.sed_e[i] += e;
                    settle(s, k, i, -m);
                }
            }
            for bi in 0..s.bedk.len() {
                let (k, bv) = (s.bedk[bi], s.bed[i][1]);
                let b = w.bed[k * n + i];
                if b > 0.0 && bv > 0.0 {
                    let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(0.05);
                    let m = (v / s.sp[k].v[0]).min(0.5 * b);
                    w.bed[k * n + i] -= m;
                    w.ground[k * n + i] += m;
                    s.bed[i][1] -= m * s.sp[k].v[0];
                    s.cap[i] += m * s.sp[k].cp;
                    if !s.loose.contains(&k) {
                        s.loose.push(k);
                        s.vary.push(k);
                    }
                }
            }
        }
        if qss <= 0.0 {
            continue;
        }
        for (&(j, dh), &q) in nb[..cnt].iter().zip(&qs).filter(|p| *p.1 > 0.0) {
            let (f, mut mass) = (q / qss, 0.0);
            for (si, &k) in sol.iter().enumerate() {
                let m = s.sed[i * ns + si] * f;
                if m <= 0.0 {
                    continue;
                }
                mass += m * s.sp[k].mm;
                if s.done[j] {
                    w.ground[k * n + j] += m;
                    s.cap[j] += m * s.sp[k].cp;
                    settle(s, k, j, m);
                } else {
                    s.sed[j * ns + si] += m;
                }
            }
            let e = s.sed_e[i] * f + mass * g * dh;
            heat += mass * g * dh;
            if s.done[j] {
                w.e_ground[j] += e;
            } else {
                s.sed_e[j] += e;
            }
        }
        s.sed[i * ns..(i + 1) * ns].fill(0.0);
        s.sed_e[i] = 0.0;
    }
    w.ledger.gravity += heat;
}

/// Drainage (15 Oct 2026): the loose ground holds its liquid against gravity up to a share `field` of its pores; the
/// rest sinks, `seep` a second at the most (Darcy's law under its own weight), into the store of the bed below
/// (`World::deep`: a zone `bed_depth` deep with pores `bed_porosity` of it). The stores flow to their neighbours by
/// Darcy's law, as the pores do: `bed_seep` x the depth the store fills x the slope of its table, toward a lower table,
/// no more than half way to a level one; what reaches a full store comes out into its node's loose ground (a spring,
/// or the standing liquid of a low cell). Each move takes its share of every liquid and what is dissolved in it, with
/// its heat. Bodies and Law 2 do not reach the store.
fn drain(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, ns, area, p) = (w.grid.n(), w.chem.ns(), s.area, w.cfg.phys);
    if w.deep.len() != ns * n {
        (w.deep, w.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
    }
    let (tmin, tmax) = range(&s.tg);
    let liq: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt <= tmax && (s.sp[k].boil > tmin || w.chem.subs[k].solubility > 0.0)).collect();
    let (full, sy) = (p.bed_porosity * p.bed_depth * area, p.bed_porosity * area);
    // Volume of the liquid of a store (dissolved gas adds none), and its table.
    let vm: Vec<f64> = (0..ns).map(|k| if s.sp[k].boil > tmin { s.sp[k].v[1] } else { 0.0 }).collect();
    let dv = |w: &World, i: usize| (0..ns).map(|k| w.deep[k * n + i] * vm[k]).sum::<f64>();
    let table = |w: &World, s: &Scratch, i: usize| s.z[i] - s.pd[i] / p.porosity - p.bed_depth + dv(w, i) / sy;
    for i in 0..n {
        let lvol: f64 = liq.iter().filter(|&&k| s.sp[k].boil > tmin).map(|&k| w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]).sum();
        let v = (lvol - p.field * s.pd[i] * area).min(full - dv(w, i)).min(p.seep * area * dt);
        if v <= 0.0 {
            continue;
        }
        let tcur = s.tg[i].min((w.e_ground[i] - s.hold[i][0]) / s.cap[i].max(1.0)).max(0.0);
        for &k in &liq {
            let sp = s.sp[k];
            let m = w.ground[k * n + i] * if sp.boil <= tmin { 1.0 } else { wet(s, k, i) } * v / lvol;
            let want = m * (sp.cp * tcur + sp.lf);
            let e = crate::draw(&mut w.e_ground[i], &mut w.ledger, s.cap[i] - m * sp.cp, want);
            let m = if want > 0.0 { m * crate::share(e, want) } else { m };
            (w.ground[k * n + i], w.deep[k * n + i], w.e_deep[i], s.cap[i]) = (w.ground[k * n + i] - m, w.deep[k * n + i] + m, w.e_deep[i] + e, s.cap[i] - m * sp.cp);
            s.hold[i][0] -= if s.pk[i] != k { m * sp.lf } else { 0.0 };
            settle(s, k, i, -m);
        }
    }
    for i in 0..n {
        let nbr = s.nbr[i];
        for (d, &j) in nbr.iter().enumerate().filter(|p| *p.1 != u32::MAX) {
            let (j, held) = (j as usize, dv(w, i));
            let dh = table(w, s, i) - table(w, s, j);
            if dh <= p.still || held <= 0.0 {
                continue;
            }
            let v = (p.bed_seep * held / sy * w.grid.cell_m * dh / w.grid.dist(DIRS[d]) * dt).min(0.25 * dh * sy).min(held);
            // The share of the store that moves, and the share of that the neighbour's store has room for.
            let (f, into) = (v / held, ((full - dv(w, j)) / v).clamp(0.0, 1.0));
            let e = w.e_deep[i] * f;
            (w.e_deep[i], w.e_deep[j], w.e_ground[j]) = (w.e_deep[i] - e, w.e_deep[j] + e * into, w.e_ground[j] + e * (1.0 - into));
            for k in 0..ns {
                let (m, sp) = (w.deep[k * n + i] * f, s.sp[k]);
                if m <= 0.0 {
                    continue;
                }
                let out = m * (1.0 - into);
                (w.deep[k * n + i], w.deep[k * n + j], w.ground[k * n + j], s.cap[j]) = (w.deep[k * n + i] - m, w.deep[k * n + j] + m * into, w.ground[k * n + j] + out, s.cap[j] + out * sp.cp);
                s.hold[j][0] += if s.pk[j] != k { out * sp.lf } else { 0.0 };
                settle(s, k, j, out);
            }
        }
    }
}

/// Yearly: loose solids creep to lower neighbours, softer ones faster and thicker layers
/// more; heat conducts between neighbouring ground nodes; bodies spread back into the ground
/// cleared along edges (`choose::regrow`, here only, so it closes at the documented speed).
fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, area, g, cell) = (w.grid.n(), s.area, w.cfg.gravity, w.grid.cell_m);
    let (tmin, _) = range(&s.tg);
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let soft: Vec<f64> = (0..w.chem.ns()).map(|k| 1.0 / (w.chem.subs[k].hardness + 0.05)).collect();
    s.mob.fill(0.0);
    for &k in &sol {
        for i in 0..n {
            s.mob[i] += w.ground[k * n + i] * (1.0 - molten(s, k, i)) * s.sp[k].v[0] * soft[k];
        }
    }
    let mut heat = 0.0;
    for i in 0..n {
        for (d, j) in around(&FORWARD, w.grid.w, w.grid.h, i).into_iter().enumerate() {
            if j == usize::MAX {
                continue;
            }
            let dist = w.grid.dist(FORWARD[d]);
            let (ti, tj) = (s.tg[i], s.tg[j]);
            let gk = (s.bed[i][3] + s.bed[j][3]) / 2.0 * w.cfg.skin_m * cell / dist;
            let qmax = 0.2 * (ti - tj).abs() * s.cap[i].min(s.cap[j]);
            let qh = (gk * (ti - tj) * dt).clamp(-qmax, qmax);
            // Heat goes from the warmer node; it gives no more than it holds above `T_FLOOR`.
            let (from, to) = if qh >= 0.0 { (i, j) } else { (j, i) };
            let took = w.take_heat(from, qh.abs());
            w.e_ground[to] += took;

            let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
            let dz = s.z[a] - s.z[b];
            if s.mob[a] <= 0.0 || dz <= 0.0 {
                continue;
            }
            let r = w.cfg.creep * dz / dist * cell * dt / area * held(w, a);
            let f = (area * dz / 4.0 / (r * s.mob[a])).min(1.0);
            let (ta, tb) = (s.tg[a], s.tg[b]);
            // What moves, summed over the substances: heat capacity, mass, mobility, and the
            // rise of each node's surface and head.
            let (mut c, mut mass, mut mob, mut ua, mut ub) = (0.0, 0.0, 0.0, [0.0; 2], [0.0; 2]);
            for &k in &sol {
                let x = w.ground[k * n + a] * (1.0 - molten(s, k, a));
                if x > 0.0 {
                    let (p, m) = (&s.sp[k], (x * r * f * soft[k]).min(0.25 * x));
                    w.ground[k * n + a] -= m;
                    w.ground[k * n + b] += m;
                    let (da, db) = (rise(p, ta, -m, area), rise(p, tb, m, area));
                    (c, mass, mob) = (c + m * p.cp, mass + m * p.mm, mob + m * p.v[0] * soft[k]);
                    (ua, ub) = ([ua[0] + da[0], ua[1] + da[1]], [ub[0] + db[0], ub[1] + db[1]]);
                }
            }
            if c > 0.0 {
                let drop = mass * g * dz / 2.0;
                (s.cap[a], s.cap[b], s.mob[a], s.mob[b]) = (s.cap[a] - c, s.cap[b] + c, s.mob[a] - mob, s.mob[b] + mob);
                // The heat of what moved goes with it; the node it leaves gives no more than it holds above `T_FLOOR`.
                let took = w.take_heat(a, c * ta);
                w.e_ground[b] += took + drop;
                lift(s, a, ua);
                lift(s, b, ub);
                heat += drop;
            }
        }
    }
    w.ledger.gravity += heat;
    crate::choose::regrow(w, dt);
}
