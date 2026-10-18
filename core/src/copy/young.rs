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

/// A young node `b` (more than one member and immature; slot `slot`, taken out of it) after an update in which its mean
/// member grew from `before` kg of body: the members who grew past the copy mass in it join its adult node (`linked`)
/// with their share of the stores, heat and brood (`nodes::part`), or start one if there is none and they are at least
/// one member. Its members grow alike in proportion (their log10 bodies all rise by the growth of the mean, d), so
/// those who passed the copy mass are those who were within d below it: the flux across a size of the size-structured
/// population (McKendrick 1926, von Foerster 1959), none without growth. With the log10 bodies normal now
/// (`Body::size_moments`: mean `mu`, variance `Body::size_spread`, sd `s`), they lie between a = (log10 copy mass -
/// mu) / s and a + d / s in standard units: the share Q(a) - Q(a + d/s) of the members (`math::upper_tail`), holding
/// the share Q(a - s ln 10) - Q(a + d/s - s ln 10) of the body (the partial mean of a lognormal); the stores go with the
/// body. Those who stay and those who leave keep the variances of their parts of the normal. Less than one member left
/// young: all of it joins the adult node.
pub fn mature(w: &mut World, slot: usize, b: &mut Body, before: f64) -> Grown {
    if b.n() <= 1.0 || b.adult(&w.chem) || !may_be_node(w, b) {
        return Grown::Lives;
    }
    let (mu, v) = b.size_moments(&w.chem);
    let (v, s, d) = (v as f64, (v as f64).sqrt(), (b.member_body(&w.chem) / before.max(f64::MIN_POSITIVE)).log10());
    if s <= 0.0 || d <= 0.0 {
        return Grown::Lives;
    }
    let lo = (b.dec(COPY_MASS).log10() - mu as f64) / s;
    let (hi, k) = (lo + d / s, s * LN_10);
    let (members, held) = (upper_tail(lo) - upper_tail(hi), upper_tail(lo - k) - upper_tail(hi - k));
    let adult = linked(w, slot, b, true);
    match adult {
        Some(t) if (1.0 - members) * b.count < 1.0 => return Grown::Joins(t),
        None if members * b.count < 1.0 || (1.0 - members) * b.count < 1.0 => return Grown::Lives,
        _ if members <= 0.0 => return Grown::Lives,
        _ => {}
    }
    // The moments of the part of a standard normal between lo and hi (its mean and its mean square), and of the rest.
    let (m1, m2) = ((density(lo) - density(hi)) / members, 1.0 + (lo * density(lo) - hi * density(hi)) / members);
    let rest = 1.0 - members;
    let (r1, r2) = (-members * m1 / rest, (1.0 - members * m2) / rest);
    let mut c = part(&mut w.life, b, members, held);
    b.size_spread = (v * (r2 - r1 * r1)).max(0.0) as f32;
    c.size_spread = (v * (m2 - m1 * m1)).max(0.0) as f32;
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
