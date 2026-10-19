//! Young that grow up. A kind in a cell lives as an adult node and a young node joined by a link (`Body::link`): the
//! adult's children are booked on its young node (`copy`), and the young members mature one by one along the link at
//! the rate their growth gives (`mature`). A node keeps the spread of its members' sizes (`Body::size_spread`), so the
//! members who joined earlier, and so have grown more, pass the copy mass first: maturing is a flux, not a jump of the
//! mean. This is a population of two stages (Lefkovitch 1965) whose rates come from growth; real stands of plants have
//! such size hierarchies (Weiner 1990).

use super::kin;
use crate::body::{Body, COPY_MASS};
use crate::math::upper_tail;
use crate::nodes::{join, may_be_node, part};
use crate::rng::{Purpose, Rng};
use crate::World;
use std::f64::consts::{LN_10, PI};

/// The node that node `b` (slot `slot`, taken out of it) is linked to, if that is alive, a node of `b`'s cell and mature
/// as `mature` asks; else the first node of the cell at that stage and of `b`'s kind (`copy::kin`), linked from then on.
pub(crate) fn linked(w: &mut World, slot: usize, b: &mut Body, mature: bool) -> Option<usize> {
    let at = b.link.map(|id| id as u32 as usize).filter(|&t| {
        let o = &w.life.bodies[t];
        t != slot && o.alive && Some(o.id(t)) == b.link && o.n() > 1.0 && o.cell == b.cell && o.adult(&w.chem) == mature
    });
    at.or_else(|| {
        let t = kin(w, b.cell as usize, &b.genes, &b.spread, mature)?;
        link(w, slot, b, t);
        Some(t)
    })
}

/// Node `b` (slot `slot`, taken out of it) and the node in slot `t` are linked.
pub(crate) fn link(w: &mut World, slot: usize, b: &mut Body, t: usize) {
    b.link = Some(w.life.bodies[t].id(t));
    w.life.bodies[t].link = Some(b.id(slot));
}

/// What `mature` left of a young node.
pub enum Grown {
    /// The node, perhaps with fewer members
    Lives,
    /// Less than one member: the rest joins its adult node, in this slot
    Joins(usize),
}

/// The density of a standard normal at `z`.
fn density(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * PI).sqrt()
}

/// The share of a standard normal between `x` and `y` (x <= y), from the tail on the side away from them, which keeps
/// its digits far out in either tail (`math::upper_tail`).
fn between(x: f64, y: f64) -> f64 {
    if x + y >= 0.0 { upper_tail(x) - upper_tail(y) } else { upper_tail(-y) - upper_tail(-x) }
}

/// The mean and the variance of a standard normal cut above `a`: -λ and 1 - aλ - λ², λ = φ(a)/Φ(a) the inverse Mills
/// ratio (Johnson, Kotz and Balakrishnan 1994, 13.10).
fn cut(a: f64) -> (f64, f64) {
    let l = density(a) / upper_tail(-a);
    (-l, (1.0 - a * l - l * l).max(0.0))
}

/// The deepest cut `fit` takes, in standard units: a normal cut there is within 2% of its limit, an exponential tail
/// at the cut, and `cut` keeps three digits of its variance.
const DEEPEST: f64 = -8.0;
/// A cut this far above the mean takes nothing that counts (a share 6e-16 of the normal).
const UNCUT: f64 = 8.0;
/// Halvings of the interval from `DEEPEST` to `UNCUT` in `fit`: to 1e-11 standard units.
const FIT_STEPS: usize = 40;

/// The cut `a` (standard units) at which a normal cut above lies `r` of its standard deviations below the cut (r rises
/// with a from 1, an exponential tail, to a, the whole normal), by halving between `DEEPEST` and `UNCUT`.
fn fit(r: f64) -> f64 {
    let depth = |a: f64| {
        let (m, v) = cut(a);
        (a - m) / v.sqrt()
    };
    if r >= UNCUT {
        return r;
    }
    let (mut lo, mut hi) = (DEEPEST, UNCUT);
    for _ in 0..FIT_STEPS {
        let mid = 0.5 * (lo + hi);
        if depth(mid) < r { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

/// A young node `b` (more than one member and immature; slot `slot`, taken out of it) after an update in which its mean
/// member grew from `before` kg of body: the members who grew past the copy mass in it join its adult node (`linked`)
/// with their share of the stores, heat and brood (`nodes::part`), or start one if there is none and they are at least
/// one member. Young members lie below the copy mass (they are born smaller and leave when they reach it), so their
/// log10 bodies before the update are a normal cut above at the copy mass, fitted to their mean and variance
/// (`Body::size_moments`; the cut `fit`, the uncut normal's sd `s`). They grow alike in proportion (their log10 bodies
/// all rise by the growth of the mean, d), so those who passed the copy mass are the top d of them: the flux across a
/// size of the size-structured population (McKendrick 1926, von Foerster 1959), none without growth. With the copy mass
/// at `a - d/s` and the top at `a` in standard units of the uncut normal after growth, they are the share
/// (Φ(a) - Φ(a - d/s)) / Φ(a) of the members, holding the share (Φ(a - k) - Φ(a - d/s - k)) / Φ(a - k), k = s ln 10, of
/// the body (the partial means of a lognormal); the stores go with the body. Those who stay are again a normal cut at
/// the copy mass, and those who leave a slice of it: each keeps the mean and the variance of its part. Less than one
/// member left young: all of it joins the adult node.
pub fn mature(w: &mut World, slot: usize, b: &mut Body, before: f64) -> Grown {
    if b.n() <= 1.0 || b.adult(&w.chem) || !may_be_node(w, b) {
        return Grown::Lives;
    }
    let (mean, v) = b.size_moments(&w.chem);
    let (mean, sd, d) = (mean as f64, (v as f64).sqrt(), (b.member_body(&w.chem) / before.max(f64::MIN_POSITIVE)).log10());
    if sd <= 0.0 || d <= 0.0 {
        return Grown::Lives;
    }
    let top = b.dec(COPY_MASS).log10();
    let a = fit((top - (mean - d)) / sd);
    let s = sd / cut(a).1.sqrt();
    // The uncut normal's mean after growth, the copy mass in its standard units, and the shares.
    let (mu, lo, k) = (top + d - a * s, a - d / s, s * LN_10);
    let (slice, whole) = (between(lo, a), upper_tail(-a));
    let (members, rest, held) = (slice / whole, upper_tail(-lo) / whole, between(lo - k, a - k) / upper_tail(k - a));
    let adult = linked(w, slot, b, true);
    match adult {
        Some(t) if rest * b.count < 1.0 => return Grown::Joins(t),
        None if members * b.count < 1.0 || rest * b.count < 1.0 => return Grown::Lives,
        _ if members <= 0.0 => return Grown::Lives,
        _ => {}
    }
    // The mean and the mean square of the slice of a standard normal between lo and a, and the mean and the variance
    // of the normal cut at lo.
    let (m1, m2) = ((density(lo) - density(a)) / slice, 1.0 + (lo * density(lo) - a * density(a)) / slice);
    let (r1, r2) = cut(lo);
    let mut c = part(&mut w.life, b, members, held);
    b.set_size(&w.chem, mu + s * r1, s * s * r2);
    c.set_size(&w.chem, mu + s * m1, s * s * (m2 - m1 * m1));
    match adult {
        Some(t) => {
            let mut r = Rng::new(w.seed, w.day(), b.id(slot), Purpose::Merge);
            let (born, rows) = w.life.carry_rows(w.life.born(b).to_vec(), std::mem::take(&mut c.rows), b.lineage, w.life.bodies[t].lineage, &mut r, w.cfg.new_noise());
            c.rows = rows;
            join(w, t, c, &born);
        }
        None => {
            let t = w.place(c, Some(w.light_keep.get(slot, b.gen)));
            link(w, slot, b, t);
        }
    }
    Grown::Lives
}
