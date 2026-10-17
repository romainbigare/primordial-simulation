use super::air;
use super::land::NONE;
use super::range;
use crate::grid::DIRS;
use crate::World;
use serde::{Deserialize, Serialize};

/// Numbers of one substance the laws read often: heat capacity, molar mass, fusion and
/// vaporisation enthalpy, molar volume as [solid, liquid], melting and boiling point,
/// reflectance as [solid, liquid].
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) struct Sp {
    pub(super) cp: f64,
    pub(super) mm: f64,
    pub(super) lf: f64,
    pub(super) lv: f64,
    pub(super) cond: f64,
    pub(super) v: [f64; 2],
    pub(super) melt: f64,
    pub(super) boil: f64,
    pub(super) refl: [f64; 2],
}

/// Working memory of the law, rebuilt from the world when missing. It is saved with the world
/// (but for the space used within a call and the tables `tables` rebuilds on load): the slower
/// parts of the law run from what it held at their last turn, so a resumed run matches an unbroken one.
///
/// Who writes and reads each field (R `refresh`, G `ground_state`, S `air::state`, Rad
/// `air::radiation`, X `air::exchange`, W `air::wind`, L `liquid`, C `creep`, Law 2 `change`
/// through `view`). Ground fields are per ground node, air fields per air node.
///
/// | field | unit | written by | read by |
/// | --- | --- | --- | --- |
/// | `month`, `year`, `pending`, `pending_liq`, `pending_exc`, `owe_work` | month, year, s, s, s, flag | `step` | `step` |
/// | `area`, `sp`, `cells`, `air_of`, `near`, `wt`, `per` | m², `Sp` per substance, count, ground cell to air nodes, weights, share | `tables` (on load) | all |
/// | `loose`, `fixed`, `vary`, `bedk`, `gas` | substance lists | R (`loose`, `vary` also X, L) | all |
/// | `fix` | m³ (volume, volume × reflectance) | R | `surface` |
/// | `bed` | [J/K skin, m³, reflectance, W/(m K)] | R (L wears `[1]`) | G, Rad (via `alb`), C, Law 2 |
/// | `cap` | J/K | G (every step), X, L, C (moves of heat capacity) | G, Rad, X, L, C |
/// | `tg` | K | G | all, Law 2, bodies (`temps`) |
/// | `pk`, `phi` | substance, share | G | `molten`, Rad |
/// | `hold` | [J held, J low, J high, molten bits] | G, L (`[0]`), `lift` (marks stale) | G, L |
/// | `z`, `head` | m (solid surface, surface of all the liquid, pores filled first) | `surface`, `settle`, `lift` | S, Rad, X, L, C |
/// | `pd`, `alb`, `rc` | m (pore liquid over the cell), reflectance, m² K/W | `surface` | L; Rad |
/// | `order`, `done`, `sed`, `sed_e` | cell keys by head, visited flag, mol carried, J carried | L | L |
/// | `nbr`, `mob` | neighbour cells, m³ mobile | `space`; C | L; C |
/// | `tl`, `ta0` | K, air touching the ground; K, air at the last S | Rad; S | X; Rad |
/// | `seen`, `lay` | [drop/m, air height, absorbing share, cloud reflectance, air K]; K below the ground's air, per layer | S (X sets `[3]`) | Rad |
/// | `rain`, `cloud`, `snap`, `trace` | mol condensed, [mol, mol × reflectance], mol per gas, s and calls | X | X (`rain` to `World::rain`) |
/// | `za`, `vol` | m, m³ | S | S, W |
/// | `nt`, `capa`, `mass` | mol, J/K, kg | S; X, W (moves) | X, W, Rad |
/// | `fabs`, `gam`, `zair`, `ta` | optical depth, K/m, m, K | S (`ta` also Rad, W) | Rad, X, W |
/// | `node_tmp` | reused: J/K (S), J (Rad), reflectance (X), Pa (W) | S, Rad, X, W | the same part |
/// | `cond`, `hk`, `split` | may condense, m, share (per gas) | S | X, W |
/// | `warmth`, `grad`, `nt_new`, `lanes`, `rad` | work space of W (warmth, gradient, amounts after a direction, a row of fluxes per line) and Rad (a row per ground cell) | W; Rad | W; Rad |
/// | `slow` | [tick last, tick next, count] | Law 2 | Law 2 |
/// What Law 2 reads of the last flow step (`Scratch::view`): ground temperatures, bed skins,
/// the air node of each ground node, air temperatures.
pub(crate) type FlowView<'a> = (&'a [f64], &'a [[f64; 4]], &'a [u32], &'a [f64]);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scratch {
    pub(super) month: Option<u64>,
    pub(super) year: Option<u64>,
    /// Seconds since air and rain last moved, and since liquid (and creep) last moved.
    pub(super) pending: f64,
    pub(super) pending_liq: f64,
    pub(super) pending_exc: f64,
    /// The substances present changed: rebuild the state of ground and air at the next step.
    #[serde(skip)]
    pub(super) stale: bool,
    /// Moving liquid's work on its bed is due at the next liquid step.
    pub(super) owe_work: bool,
    /// Tables of the grid and the chemistry alone (not saved): cell area, substance constants,
    /// and the map from ground cells to air nodes (own node, four nearest and weights, share).
    #[serde(skip)]
    pub(super) area: f64,
    #[serde(skip)]
    pub(super) sp: Vec<Sp>,
    /// Substances present in ground, bonded ground and air nodes. Loose ones that stay solid and
    /// cannot evaporate are `fixed` (volume and volume × reflectance kept in `fix`); the others `vary`.
    pub(super) loose: Vec<usize>,
    pub(super) fixed: Vec<usize>,
    pub(super) vary: Vec<usize>,
    pub(super) fix: Vec<[f64; 2]>,
    pub(super) bedk: Vec<usize>,
    pub(super) gas: Vec<usize>,
    pub(super) bed: Vec<[f64; 4]>,
    pub(super) cap: Vec<f64>,
    pub(super) tg: Vec<f64>,
    /// Substance held at its melting point in each ground node (or `NONE`), and its molten share.
    pub(super) pk: Vec<usize>,
    pub(super) phi: Vec<f64>,
    /// Off a plateau: the fusion enthalpy held, and the energies between which it holds
    /// (until the amounts change); and what is molten, one bit per substance (`melted`).
    pub(super) hold: Vec<[f64; 4]>,
    pub(super) z: Vec<f64>,
    pub(super) head: Vec<f64>,
    /// Pore space of the loose ground, as a depth of liquid over the cell: the first liquid fills it.
    pub(super) pd: Vec<f64>,
    pub(super) alb: Vec<f64>,
    /// Heat resistance of the soft solid cover (depth / conductivity), m² K / W.
    pub(super) rc: Vec<f64>,
    #[serde(skip)]
    pub(super) rain: Vec<f64>,
    pub(super) order: Vec<u64>,
    /// Neighbours of each ground node along `DIRS` (`NONE` off the map).
    #[serde(skip)]
    pub(super) nbr: Vec<[u32; 8]>,
    #[serde(skip)]
    pub(super) done: Vec<bool>,
    /// Loose solids carried by liquid, [cell * carried kinds + kind], and their heat.
    #[serde(skip)]
    pub(super) sed: Vec<f64>,
    #[serde(skip)]
    pub(super) sed_e: Vec<f64>,
    #[serde(skip)]
    pub(super) mob: Vec<f64>,
    #[serde(skip)]
    pub(super) air_of: Vec<u32>,
    #[serde(skip)]
    pub(super) near: Vec<[u32; 4]>,
    #[serde(skip)]
    pub(super) wt: Vec<[f64; 4]>,
    #[serde(skip)]
    pub(super) per: Vec<f64>,
    pub(super) tl: Vec<f64>,
    pub(super) seen: Vec<[f64; 5]>,
    pub(super) lay: Vec<[[f64; air::LAYERS]; 2]>,
    pub(super) ta0: Vec<f64>,
    #[serde(skip)]
    pub(super) cloud: Vec<[f64; 2]>,
    #[serde(skip)]
    pub(super) snap: Vec<f64>,
    #[serde(skip)]
    pub(super) cells: Vec<f64>,
    pub(super) za: Vec<f64>,
    pub(super) vol: Vec<f64>,
    pub(super) nt: Vec<f64>,
    pub(super) capa: Vec<f64>,
    pub(super) mass: Vec<f64>,
    pub(super) fabs: Vec<f64>,
    pub(super) gam: Vec<f64>,
    pub(super) zair: Vec<f64>,
    pub(super) ta: Vec<f64>,
    #[serde(skip)]
    pub(super) node_tmp: Vec<f64>,
    #[serde(skip)]
    pub(super) warmth: Vec<f64>,
    #[serde(skip)]
    pub(super) grad: Vec<[f64; 2]>,
    /// Per gas [g * na + j]: may it condense in the column; height its amount fills near the
    /// ground, m; share of it in the upper half of the column.
    pub(super) cond: Vec<bool>,
    pub(super) hk: Vec<f64>,
    pub(super) split: Vec<f64>,
    #[serde(skip)]
    pub(super) nt_new: Vec<f64>,
    #[serde(skip)]
    pub(super) rad: Vec<f64>,
    #[serde(skip)]
    pub(super) lanes: Vec<f64>,
    /// Time since trace gases last evaporated and condensed, s, and the calls since.
    pub(super) trace: [f64; 2],
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
    /// its bonded skin (as of the last refresh) and its air node; per air node its temperature
    /// (`FlowView`).
    pub(crate) fn view(&self) -> FlowView<'_> {
        (&self.tg, &self.bed, &self.air_of, &self.ta)
    }
}

/// Neighbours of cell `i` along `dirs`, or `usize::MAX` off the map.
#[inline]
pub(super) fn around<const N: usize>(dirs: &[(i32, i32); N], gw: usize, gh: usize, i: usize) -> [usize; N] {
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
pub(super) fn refresh(w: &World, s: &mut Scratch) {
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
            let (mut v, mut r, mut kc) = (0.0, 0.0, 0.0);
            for &k in &s.bedk {
                let b = w.bed[k * n + i];
                v += b * s.sp[k].v[0];
                r += b * s.sp[k].v[0] * s.sp[k].refl[0];
                kc += b * s.sp[k].v[0] * w.chem.subs[k].conductivity;
            }
            let c = crate::ground::skin_cap(&w.chem, &w.cfg, s.area, |k| w.bed[k * n + i]);
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
pub(super) fn tables(w: &World, s: &mut Scratch) {
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
pub(super) fn space(w: &World, s: &mut Scratch) {
    let (n, na) = (w.grid.n(), w.grid.na());
    s.nbr = (0..n).map(|i| around(&DIRS, w.grid.w, w.grid.h, i).map(|j| j as u32)).collect();
    for v in [&mut s.rain, &mut s.sed_e, &mut s.mob] {
        *v = vec![0.0; n];
    }
    (s.node_tmp, s.warmth) = (vec![0.0; na], vec![0.0; na]);
    (s.done, s.grad, s.cloud) = (vec![false; n], vec![[0.0; 2]; na], vec![[0.0; 2]; na]);
}
