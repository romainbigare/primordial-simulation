//! What moves between a cell's parts and into each (laws.md 1.1, `parts`): what other laws moved, shared between them
//! (`reconcile`); the liquid that settles from the land (`settle`); the ground the liquid gains or gives as the share of
//! the cell under it follows what stands on it (`strip`); what Flow's liquid laws move of each (`to_liquid`); what the
//! air's step moves of the land and the top and the heat they pass each other at the shore (`after_air`).

use super::{Found, Parts, DEEP, LAND, PARTS, TOP};
use crate::flow::Scratch;
use crate::{ground::amounts_cap, math::one_minus_exp, store::Class, units::T_FLOOR, World};

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
    let cap = [LAND, TOP, DEEP].map(|q| s.skin[i] * p.ground(q) + amounts_cap(s.state.loose.iter().copied(), |k| p.amt[k][q], |k| s.sp[k].cp));
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

/// The liquid over the pores of the land of `p` in cell `i` (found as `f`, porosity `por`) runs down to the liquid's top,
/// as the ground inside a cell drains to its lowest, each liquid (`Sp::liquid`) with its heat at the land's
/// temperature; the land keeps what its pores hold, its own soil's liquid. Whether any moved.
pub(super) fn settle(s: &Scratch, i: usize, p: &mut Parts, f: &[Found; PARTS], por: f64) -> bool {
    let over = f[LAND].over(por);
    if over <= 0.0 {
        return false;
    }
    land_to_top(s, i, p, (f[LAND].th.t, &|k| f[LAND].th.molten(k)), over / f[LAND].l[2]);
    true
}

/// The share `part` of the liquid of the land of `p` in cell `i` goes to the liquid's top: of each liquid (`Sp::liquid`,
/// at `t` K, the share `molten(k)` of it molten), with its heat at `t`.
fn land_to_top(s: &Scratch, i: usize, p: &mut Parts, (t, molten): (f64, &dyn Fn(usize) -> f64), part: f64) {
    let mut heat = 0.0;
    for &k in s.state.vary.iter().filter(|&&k| t < s.sp[k].boil) {
        let m = p.amt[k][LAND] * s.sp[k].liquid(t, molten(k), s.state.dissolved[i]) * part;
        p.amt[k][LAND] -= m;
        p.amt[k][TOP] += m;
        heat += m * (s.sp[k].cp * t + s.sp[k].lf);
    }
    p.heat[LAND] -= heat;
    p.heat[TOP] += heat;
}

/// The share `part` of the liquid of the land of cell `i` drains to its liquid's top, at the land's temperature and
/// phases as its last solve found them (`land_to_top`): the land's pores drain inside the cell to its liquid
/// (`drain::land_drains`).
pub(in crate::flow) fn drain_land(s: &mut Scratch, i: usize, part: f64) {
    let Some(land) = s.mixed(i).map(|m| m.piece[LAND].clone()) else { return };
    let mut parts = std::mem::take(&mut s.state.parts);
    if let Some(Some(p)) = parts.get_mut(i) {
        land_to_top(s, i, p, (land.tg, &|k| land.molten(k)), part);
    }
    s.state.parts = parts;
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
        let liquids = s.state.vary.iter().filter(|&&k| t < s.sp[k].boil).map(|&k| (k, s.sp[k].liquid(t, f.th.molten(k), s.state.dissolved[i]) * pore * y, s.sp[k].cp * t + s.sp[k].lf));
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

/// The part of a cell of parts what Flow's liquid laws take from it leaves (`to_liquid`): its liquid, what stands and
/// runs off; or its land, what the laws under the surface move (`drain`), which do not reach the ground under a body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::flow) enum Leaves {
    Liquid,
    Land,
}

/// What Flow's liquid laws moved of the cells of parts among `cells` (`liquid::book`): what came in gathers in the
/// liquid's top; what left, as `leaves` says: of the top and the deep by their shares of their liquid and of the land
/// only what they lack (only liquid moves), or of a land all of it; the land and the deep give the heat of what they
/// gave at their own temperature and phases (`Piece::heat_of`), and the top takes the rest of the node's change of heat,
/// as it takes what came in. Of each of `ks` what each cell holds now less `before` (`[q * cells.len() + c]`), mol, and
/// of its heat now less `heat`, J.
pub(in crate::flow) fn to_liquid(w: &World, s: &mut Scratch, cells: &[u32], (ks, before): (&[usize], &[f64]), heat: &[f64], leaves: Leaves) {
    let (n, len) = (w.mesh.cells().n(), cells.len());
    for (c, &i) in cells.iter().enumerate() {
        // Each part's liquid of each substance, mol, as its last solve found it.
        let liquid = s.mixed(i as usize).map(|m| m.piece.each_ref().map(|q| ks.iter().map(|&k| q.liquid(k, &s.sp[k])).collect::<Vec<f64>>()));
        // The heat a mol of each of `ks` holds in the land and in the deep, J.
        let per_mol = s.mixed(i as usize).map(|m| [LAND, DEEP].map(|x| ks.iter().map(|&k| m.piece[x].heat_of(k, &s.sp[k])).collect::<Vec<f64>>()));
        let Some(Some(p)) = s.state.parts.get_mut(i as usize) else { continue };
        // The heat of what the land and the deep gave, J.
        let (mut land, mut deep) = (0.0, 0.0);
        for (q, &k) in ks.iter().enumerate() {
            let d = w.store.amounts(Class::Ground)[k * n + i as usize] - before[q * len + c];
            if d >= 0.0 {
                gain(p, k, TOP, d);
                continue;
            }
            // Each part's share of what leaves: from a land, all; else the liquid's top and deep give first, by their
            // shares of the liquid they hold (what stands leaves from the liquid); the land, which keeps its own
            // pores' liquid, gives only what they lack.
            let shares = match &liquid {
                _ if leaves == Leaves::Land && p.land() => [1.0, 0.0, 0.0],
                None => [0.0, 1.0, 0.0],
                Some(l) => {
                    let wet = l[TOP][q] + l[DEEP][q];
                    let from_land = if wet >= -d { 0.0 } else { ((-d - wet) / -d).min(1.0) };
                    let of_wet = |x: usize| if wet > 0.0 { (1.0 - from_land) * l[x][q] / wet } else { 0.0 };
                    [from_land, 1.0 - from_land - of_wet(DEEP), of_wet(DEEP)]
                }
            };
            let (by_land, by_deep, was) = (shares[LAND], shares[DEEP], [p.amt[k][LAND], p.amt[k][DEEP]]);
            gain(p, k, LAND, d * by_land);
            gain(p, k, TOP, d * (1.0 - by_land - by_deep));
            gain(p, k, DEEP, d * by_deep);
            if let Some(h) = &per_mol {
                (land, deep) = (land + (was[0] - p.amt[k][LAND]) * h[0][q], deep + (was[1] - p.amt[k][DEEP]) * h[1][q]);
            }
        }
        let e = w.store.heat(Class::Ground)[i as usize] - heat[c];
        p.heat = [p.heat[LAND] - land, p.heat[TOP] + e + land + deep, p.heat[DEEP] - deep];
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

/// Halvings of the span of shares in which `covers` finds the share of a cell under its liquid.
const COVER_HALVINGS: usize = 50;

/// The share of cell `i`'s area under its liquid where `v` m³ stand over the pores of the ground under it (`Fill::wet`),
/// the pores of the land it comes to cover filling first from what stands, as `moves::strip` takes them (pores first):
/// the share the liquid left after filling the pores of the ground between `p.wet` and it covers, found by halving the
/// span of shares. So what a body's level gives a cell's part, with what its newly covered pores lack
/// (`runoff::body_level`), covers the ground that level covers.
pub(super) fn covers(s: &Scratch, i: usize, p: &Parts, f: &[Found; PARTS], (v, por): (f64, f64)) -> f64 {
    let (fill, area) = (s.fill(i), s.curves[i].area());
    let top = fill.wet(v);
    let land = area * (1.0 - p.wet);
    let lack = if land > 0.0 { (f[LAND].pores(por) - f[LAND].l[2]).max(0.0) / land } else { 0.0 };
    if top <= p.wet || lack <= 0.0 {
        return top;
    }
    let left = |x: f64| fill.wet((v - (x - p.wet) * area * lack).max(0.0));
    let (mut lo, mut hi) = (p.wet, top);
    for _ in 0..COVER_HALVINGS {
        let mid = 0.5 * (lo + hi);
        if left(mid) >= mid {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    left(lo)
}
