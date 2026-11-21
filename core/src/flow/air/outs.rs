//! What a cell's exchange gives (`cells::exchange`): its changing outputs from its parts (`put`), what each mol of gas
//! it gives carries (`carried`), and its parts held to what it holds (`hold`).

use super::cells::{Ctx, GAINS, GAS, LINK, SPACE, TO_AIR, TO_SPACE, UP};
use super::surf::Surf;
use crate::{chem::Heat, units::T_FLOOR};

/// The air a cell sees moves in a step by up to this, K, as its slopes are held to what it holds (`hold`).
const AIR_SWING_K: f64 = 60.0;

/// Holds the parts of the ground `g` (`p`, `exchange`) so that it gives no more than it holds however the air it sees
/// moves in the step: its temperature by up to `AIR_SWING_K`, each gas between none and twice what it held at the start
/// (`seen`). What evaporates of each gas stays within what the cell holds of it; then the heat over its link and to
/// space, with what each mol carries (`carry`), within its heat and the sunlight it absorbs, `sun`, J. Where it would
/// give more than that already at the air of the step's start (a node far over a boiling point: its balance counts the
/// heat capacity of what boils away, which leaves with its own heat), all its parts are cut to what it holds over its
/// heat at `T_FLOOR`, the coldest a draw may take a ground node.
pub(super) fn hold(cx: &Ctx, g: &Surf, (seen, sun): (&[f64], f64), carry: &[[f64; 2]], (p, loss): (&mut [f64], &mut Vec<f64>)) {
    let (nx, stride) = (cx.act.len(), 2 + cx.act.len());
    // The share of a row's slopes that keeps its value within `most` however far the air moves.
    let keep = |r: &[f64], most: f64| {
        let far = r[1].abs() * AIR_SWING_K + (0..nx).map(|v| (r[2 + v] * seen[1 + v]).abs()).sum::<f64>();
        if r[0] + far > most && far > 0.0 { ((most - r[0]) / far).clamp(0.0, 1.0) } else { 1.0 }
    };
    for (y, ac) in cx.act.iter().enumerate() {
        let r = &mut p[(GAS + y) * stride..(GAS + y + 1) * stride];
        let k = keep(r, g.amt[ac.k]);
        r[1..].iter_mut().for_each(|d| *d *= k);
    }
    let lost = |p: &[f64], v: usize| p[TO_AIR * stride + v] + p[TO_SPACE * stride + v] + carry.iter().enumerate().map(|(y, c)| p[(GAS + y) * stride + v] * c[0]).sum::<f64>();
    let (mut most, now) = (g.heat + sun, lost(p, 0));
    if now > most {
        most = (most - g.cap * T_FLOOR).max(0.0);
        let cut = most / now;
        p.iter_mut().for_each(|x| *x *= cut);
    }
    loss.clear();
    loss.extend((0..stride).map(|v| lost(p, v)));
    let k = keep(loss, most);
    if k < 1.0 {
        p.chunks_mut(stride).for_each(|r| r[1..].iter_mut().for_each(|d| *d *= k));
    }
}

/// What a mol of active gas `k` that evaporates from the ground `g` carries, J: the heat over its link as the store books
/// it (its heat at the ground's temperature and its latent heat), and what its air node gains (that heat less its latent
/// heat and its lift from its head to its air node's mean ground, which its air pays, `FlowView::lift`).
pub(super) fn carried(cx: &Ctx, g: &Surf, k: usize) -> [f64; 2] {
    let sp = cx.s.sp[k];
    let lift = sp.mm * (cx.w.cfg.gravity * (cx.s.za[g.j] - g.head));
    [sp.carried(g.tg, 1.0) + sp.lv, sp.carried(g.tg, 0.0) - lift]
}

/// A cell's changing outputs (`out`) from its parts (`p`), each a row of its value and its slopes, with what each mol of
/// each gas carries (`carry`).
pub(super) fn put(p: &[f64], carry: &[[f64; 2]], out: &mut [f64]) {
    let stride = 2 + carry.len();
    for v in 0..stride {
        column(|r| p[r * stride + v], carry, |o, x| out[o * stride + v] = x);
    }
}

/// One column of a cell's outputs from that column of its parts, `part(r)`, with what each mol of each gas carries
/// (`carry`): `set(output, value)`.
pub(super) fn column(part: impl Fn(usize) -> f64, carry: &[[f64; 2]], mut set: impl FnMut(usize, f64)) {
    let gas = |c: usize| carry.iter().enumerate().map(|(y, h)| part(GAS + y) * h[c]).sum::<f64>();
    set(GAINS, part(TO_AIR) + gas(1));
    set(LINK, part(TO_AIR) + gas(0));
    set(SPACE, part(TO_SPACE));
    (0..carry.len()).for_each(|y| set(UP + y, part(GAS + y)));
}
