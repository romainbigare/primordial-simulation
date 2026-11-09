//! What moves between a cell's parts and into each (laws.md 1.1, `parts`): what other laws moved, shared between them
//! (`reconcile`); the liquid that settles from the land (`settle`); the ground the liquid gains or gives as the share of
//! the cell under it follows what stands on it (`strip`); what Flow's liquid laws move of each (`to_liquid`); what the
//! air's step moves of the land and the top and the heat they pass each other at the shore (`after_air`); and how the
//! parts follow a split and a join (`remap`, `whole`).

use super::{Found, Parts, DEEP, LAND, PARTS, TOP};
use crate::flow::{land::wet_share, Scratch};
use crate::remap::{mean_by, sum4, Made, Renumber, Source};
use crate::{math::one_minus_exp, store::Class, units::T_FLOOR, World};

/// A number and the sum of its parts that differ by no more than this share of it differ by the rounding of that sum
/// alone: nothing moved.
pub(super) const ROUNDING: f64 = 4.0 * f64::EPSILON;

/// Do the node's `now` and its parts' sum `held` differ by more than their rounding (`ROUNDING`)?
fn moved(now: f64, held: f64) -> bool {
    (now - held).abs() > ROUNDING * now.abs()
}

/// Did no other law move anything in cell `i` since Flow last set its parts `p`: the node's amounts and heat are
/// theirs to their rounding (`moved`)?
pub(super) fn unmoved(w: &World, i: usize, p: &Parts) -> bool {
    let (n, amounts) = (w.mesh.cells().n(), w.store.amounts(Class::Ground));
    p.amt.len() == w.chem.ns()
        && p.amt.iter().enumerate().all(|(k, a)| !moved(amounts[k * n + i], a[LAND] + a[TOP] + a[DEEP]))
        && !moved(w.store.heat(Class::Ground)[i], p.heat[LAND] + p.heat[TOP] + p.heat[DEEP])
}

/// What other laws moved in cell `i` since Flow last set its parts `p` (the node's amounts and heat against theirs) is
/// shared between them: of each substance by the share of it each holds (by the share of the cell's ground each holds
/// where none holds any, `Parts::ground`), of the heat by their heat capacities, so a heat all share warms them alike;
/// and none holds less heat than its matter at `T_FLOOR`.
pub(super) fn reconcile(w: &World, s: &Scratch, i: usize, p: &mut Parts) {
    let (n, ground) = (w.mesh.cells().n(), [LAND, TOP, DEEP].map(|q| p.ground(q)));
    let amounts = w.store.amounts(Class::Ground);
    p.amt.resize(w.chem.ns(), [0.0; PARTS]);
    for (k, a) in p.amt.iter_mut().enumerate() {
        let (now, held) = (amounts[k * n + i], a[LAND] + a[TOP] + a[DEEP]);
        if moved(now, held) {
            let (top, deep) = if held > 0.0 {
                let deep = (a[DEEP] * (now / held)).min(now);
                ((a[TOP] * (now / held)).min(now - deep), deep)
            } else {
                (now * ground[TOP], now * ground[DEEP])
            };
            *a = [now - top - deep, top, deep];
        }
    }
    let (e, held) = (w.store.heat(Class::Ground)[i], p.heat[LAND] + p.heat[TOP] + p.heat[DEEP]);
    let cap = [LAND, TOP, DEEP].map(|q| s.skin[i] * p.ground(q) + s.state.loose.iter().map(|&k| p.amt[k][q] * s.sp[k].cp).sum::<f64>());
    if moved(e, held) {
        let all = (cap[LAND] + cap[TOP] + cap[DEEP]).max(f64::MIN_POSITIVE);
        let (top, deep) = (p.heat[TOP] + (e - held) * cap[TOP] / all, p.heat[DEEP] + (e - held) * cap[DEEP] / all);
        p.heat = [e - top - deep, top, deep];
    }
    // No part holds less heat than its matter at the coldest a draw may take a ground node (`T_FLOOR`): the first
    // other that can gives what it lacks.
    for q in [LAND, TOP, DEEP] {
        let lack = cap[q] * T_FLOOR - p.heat[q];
        if lack <= 0.0 {
            continue;
        }
        if let Some(r) = [LAND, TOP, DEEP].into_iter().find(|&r| r != q && p.heat[r] - lack >= cap[r] * T_FLOOR) {
            (p.heat[q], p.heat[r]) = (p.heat[q] + lack, p.heat[r] - lack);
        }
    }
}

/// The liquid over the pores of the land of `p` (found as `f`, porosity `por`) runs down to the liquid's top, as the
/// ground inside a cell drains to its lowest, each liquid with its heat at the land's temperature; the land keeps what
/// its pores hold, its own soil's liquid. Whether any moved.
pub(super) fn settle(s: &Scratch, p: &mut Parts, f: &[Found; PARTS], por: f64) -> bool {
    let over = f[LAND].over(por);
    if over <= 0.0 {
        return false;
    }
    let (part, t) = (over / f[LAND].l[2], f[LAND].th.t);
    let mut heat = 0.0;
    for &k in s.state.vary.iter().filter(|&&k| t < s.sp[k].boil) {
        let m = p.amt[k][LAND] * f[LAND].th.molten(k) * part;
        p.amt[k][LAND] -= m;
        p.amt[k][TOP] += m;
        heat += m * (s.sp[k].cp * t + s.sp[k].lf);
    }
    p.heat[LAND] -= heat;
    p.heat[TOP] += heat;
    true
}

/// The liquid of `p` in cell `i` comes to cover the share `to` of the cell's area, with the ground it covers or leaves,
/// which its bottom holds (`Parts::bottom`, found as `f`, porosity `por`): a land that goes under it gives the bottom
/// all it holds there (its share of its matter and heat); a ground it leaves gives the land its bed's skin, its solids
/// that stay solid and the liquid its pores hold, at the bottom's temperature, what stands over them staying where it
/// stands. So what stands over the pores is the same either way.
pub(super) fn strip(s: &Scratch, i: usize, p: &mut Parts, (f, por): (&Found, f64), to: f64) {
    let (d, b) = (to - p.wet, p.bottom());
    if d > 0.0 {
        let x = d / (1.0 - p.wet);
        p.amt.iter_mut().for_each(|a| {
            let m = a[LAND] * x;
            (a[LAND], a[b]) = (a[LAND] - m, a[b] + m);
        });
        let h = p.heat[LAND] * x;
        (p.heat[LAND], p.heat[b]) = (p.heat[LAND] - h, p.heat[b] + h);
    } else {
        let (y, t) = (-d / p.wet, f.th.t);
        // The share of its liquid its pores hold.
        let pore = if f.l[2] > 0.0 { f.pores(por).min(f.l[2]) / f.l[2] } else { 0.0 };
        // Of each substance it gives, the share of what the bottom holds and the heat of a mol.
        let solids = s.state.fixed.iter().map(|&k| (k, y, s.sp[k].cp * t));
        let liquids = s.state.vary.iter().filter(|&&k| t < s.sp[k].boil).map(|&k| (k, f.th.molten(k) * pore * y, s.sp[k].cp * t + s.sp[k].lf));
        let mut h = -d * s.skin[i] * t;
        for (k, x, per) in solids.chain(liquids) {
            let m = p.amt[k][b] * x;
            (p.amt[k][b], p.amt[k][LAND]) = (p.amt[k][b] - m, p.amt[k][LAND] + m);
            h += m * per;
        }
        (p.heat[b], p.heat[LAND]) = (p.heat[b] - h, p.heat[LAND] + h);
    }
    p.wet = to;
}

/// Part `q` of `p` gains `d` mol of substance `k` (negative: loses it); where it holds less than it loses, the others
/// give the rest in their order, so none holds less than none.
fn gain(p: &mut Parts, k: usize, q: usize, d: f64) {
    let a = &mut p.amt[k];
    a[q] += d;
    if a[q] >= 0.0 {
        return;
    }
    let mut rest = -a[q];
    a[q] = 0.0;
    for r in (0..PARTS).filter(|&r| r != q) {
        let x = rest.min(a[r]);
        (a[r], rest) = (a[r] - x, rest - x);
    }
}

/// What Flow's liquid laws moved of the cells of parts among `cells` (`liquid::book`): what came in gathers in the
/// liquid's top; what left, of each part by its share of the cell's liquid (only liquid moves), and the heat that left
/// by the share of the mol each gave. Of each of `ks` what each cell holds now less `before` (`[q * cells.len() + c]`),
/// mol, and of its heat now less `heat`, J.
pub(in crate::flow) fn to_liquid(w: &World, s: &mut Scratch, cells: &[u32], (ks, before): (&[usize], &[f64]), heat: &[f64]) {
    let (n, len) = (w.mesh.cells().n(), cells.len());
    for (c, &i) in cells.iter().enumerate() {
        // Each part's liquid of each substance, mol, as its last solve found it.
        let liquid = s.mixed(i as usize).map(|m| m.piece.each_ref().map(|q| ks.iter().map(|&k| q.liquid(k, &s.sp[k])).collect::<Vec<f64>>()));
        let Some(Some(p)) = s.state.parts.get_mut(i as usize) else { continue };
        // What left, mol, and of it what the land and the deep gave.
        let (mut left, mut land, mut deep) = (0.0, 0.0, 0.0);
        for (q, &k) in ks.iter().enumerate() {
            let d = w.store.amounts(Class::Ground)[k * n + i as usize] - before[q * len + c];
            if d >= 0.0 {
                gain(p, k, TOP, d);
                continue;
            }
            // Each part's share of the cell's liquid of `k`.
            let by = |x: usize| {
                liquid.as_ref().map_or(0.0, |l| {
                    let all = l[LAND][q] + l[TOP][q] + l[DEEP][q];
                    if all > 0.0 { l[x][q] / all } else { 0.0 }
                })
            };
            let (by_land, by_deep, was) = (by(LAND), by(DEEP), [p.amt[k][LAND], p.amt[k][DEEP]]);
            gain(p, k, LAND, d * by_land);
            gain(p, k, TOP, d * (1.0 - by_land - by_deep));
            gain(p, k, DEEP, d * by_deep);
            (left, land, deep) = (left - d, land + was[0] - p.amt[k][LAND], deep + was[1] - p.amt[k][DEEP]);
        }
        let e = w.store.heat(Class::Ground)[i as usize] - heat[c];
        let (by_land, by_deep) = if left > 0.0 { (land / left, deep / left) } else { (0.0, 0.0) };
        p.heat = [p.heat[LAND] + e * by_land, p.heat[TOP] + e * (1.0 - by_land - by_deep), p.heat[DEEP] + e * by_deep];
    }
}

/// What the air's step of `dt` seconds moved of the parts of cell `i` that face it: `heat`, J, and of each substance
/// `k` of `moved` its mol, [land, top]; then the heat the two pass each other at the shore over the step, from their
/// temperatures at its start, through the conductance between them (`parts::conductance`): two heat capacities that
/// even their temperatures exactly as they would alone, (1 - e^(-g dt (1/c_land + 1/c_top))) of the way.
pub(in crate::flow) fn after_air(s: &mut Scratch, i: usize, (heat, moved): ([f64; 2], &[(usize, [f64; 2])]), dt: f64) {
    let shore = s.mixed(i).map_or(0.0, |m| {
        let (l, x) = (&m.piece, 1.0 / m.piece[LAND].cap.max(1.0) + 1.0 / m.piece[TOP].cap.max(1.0));
        (l[TOP].tg - l[LAND].tg) * one_minus_exp(m.link * dt * x) / x
    });
    let Some(Some(p)) = s.state.parts.get_mut(i) else { return };
    for &(k, m) in moved {
        gain(p, k, LAND, m[0]);
        gain(p, k, TOP, m[1]);
    }
    (p.heat[LAND], p.heat[TOP]) = (p.heat[LAND] + heat[0] + shore, p.heat[TOP] + heat[1] - shore);
}

/// Cell `i` of `w`, one node, as parts (`Parts`): all its matter and heat the liquid's top's where what stands over
/// its pores covers it (`land::wet_share`), else the land's.
fn whole_of(w: &World, i: usize) -> Parts {
    // (In a remap the world holds its new mesh already: the old cells are those of Flow's state and of the store.)
    let (s, n) = (&w.scratch, w.scratch.state.parts.len());
    let wet = s.curves.len() == n && s.head.len() == n && wet_share(s, i) >= 1.0;
    let q = if wet { TOP } else { LAND };
    let mut whole = Parts { wet: wet as u8 as f64, heat: [0.0; PARTS], amt: vec![[0.0; PARTS]; w.chem.ns()], t: [w.t_ground_last(i); PARTS], on: [None; PARTS], dry: 0.0 };
    whole.heat[q] = w.store.heat(Class::Ground)[i];
    whole.amt.iter_mut().enumerate().for_each(|(k, a)| a[q] = w.store.amounts(Class::Ground)[k * n + i]);
    whole
}

/// The parts of the quarters `kids` of a join where one of them has parts, those that are one node as parts
/// (`whole_of`); none where all are one. What a join keeps of them (`remap::Joined`), so its split gives them back.
pub(crate) fn of_join(w: &World, kids: [usize; 4]) -> Option<[Parts; 4]> {
    let s = &w.scratch;
    kids.iter().any(|&q| super::has(s, q)).then(|| kids.map(|q| s.state.parts[q].clone().unwrap_or_else(|| whole_of(w, q))))
}

/// The parts of the quarters of each join of the remap `r` (`of_join`), by cell. Read before the store follows it.
pub(crate) fn whole(w: &World, r: &Renumber) -> Vec<Option<Parts>> {
    let mut out = vec![None; w.scratch.state.parts.len()];
    if out.is_empty() {
        return out;
    }
    for src in r.sources() {
        let Source::Made(Made::Join { kids, .. }) = src else { continue };
        if let Some(parts) = of_join(w, kids.map(|q| q as usize)) {
            kids.iter().zip(parts).for_each(|(&q, p)| out[q as usize] = Some(p));
        }
    }
    out
}

/// Quarter `q`'s share of a number of its joined parent's parts, by what each quarter held of it at the join (`kept`;
/// by its `area` share where none held any).
fn kept_share(kept: &[Parts; 4], (q, area): (usize, f64), of: &dyn Fn(&Parts) -> f64) -> f64 {
    let all = sum4(kept.each_ref().map(of));
    if all != 0.0 { of(&kept[q]) / all } else { area }
}

/// The parts of each cell follow a remap (`Scratch::remap`): an old cell's go with it; a quarter of a joined cell holds
/// its share of its parent's by the share of each number it held at the join (`Joined::parts`), at its area and
/// temperatures then, as the store gives it its share of each amount and heat by that record, so the split gives the
/// quarters back. A quarter of a cell all under its liquid takes its area's share of each of its parts (what the split
/// moves between the quarters is then shared between their parts, `reconcile`). A quarter of another cell is one node:
/// the split stands the cell's liquid over the quarters' own ground (`flow::share_ground`), so its land and its liquid
/// lie in other shares than its parent's; one with liquid over part of its ground or deeper than `h_mix` is made parts
/// at its next build (`create`). A joined cell holds the sum of its
/// quarters' (`whole` for those that are one node), the mean of their areas and temperatures by area, its plateaus found
/// at its next solve. A new cell is one node.
pub(in crate::flow) fn remap(parts: Vec<Option<Parts>>, r: &Renumber, joined: &[Option<crate::remap::Joined>], whole: &[Option<Parts>]) -> Vec<Option<Parts>> {
    if parts.is_empty() {
        return parts;
    }
    let made = |m: &Made| -> Option<Parts> {
        match *m {
            Made::Quarter { parent, q, area, .. } => {
                let p = parts[parent as usize].as_ref()?;
                if let Some(kept) = joined.get(parent as usize).and_then(Option::as_ref).and_then(|r| r.parts.as_ref()) {
                    let by = |of: &dyn Fn(&Parts) -> f64| kept_share(kept, (q as usize, area), of);
                    let heat = [LAND, TOP, DEEP].map(|x| p.heat[x] * by(&|k: &Parts| k.heat[x]));
                    let amt = (0..p.amt.len()).map(|s| [LAND, TOP, DEEP].map(|x| p.amt[s][x] * by(&|k: &Parts| k.amt[s][x]))).collect();
                    return Some(Parts { heat, amt, dry: p.dry * by(&|k: &Parts| k.dry), ..kept[q as usize].clone() });
                }
                // A quarter of a cell all under its liquid takes its area's share of each part, so its top and its deep
                // keep their heat apart.
                (!p.land()).then(|| Parts { heat: p.heat.map(|h| h * area), amt: p.amt.iter().map(|a| a.map(|x| x * area)).collect(), dry: 0.0, ..p.clone() })
            }
            Made::Join { kids, area } => {
                let k4 = kids.map(|q| parts[q as usize].as_ref().or_else(|| whole.get(q as usize).and_then(Option::as_ref)));
                let k4: [&Parts; 4] = [k4[0]?, k4[1]?, k4[2]?, k4[3]?];
                let ns = k4[0].amt.len();
                let amt = (0..ns).map(|k| [LAND, TOP, DEEP].map(|q| sum4(k4.map(|p| p.amt[k][q])))).collect();
                let t = [LAND, TOP, DEEP].map(|q| mean_by(k4.map(|p| p.t[q]), area));
                let (wet, heat, dry) = (mean_by(k4.map(|p| p.wet), area), [LAND, TOP, DEEP].map(|q| sum4(k4.map(|p| p.heat[q]))), sum4(k4.map(|p| p.dry)));
                Some(Parts { wet, heat, amt, t, on: [None; PARTS], dry })
            }
        }
    };
    r.sources()
        .iter()
        .map(|src| match src {
            Source::Old(i) => parts[*i as usize].clone(),
            Source::New => None,
            Source::Made(m) => made(m),
        })
        .collect()
}
