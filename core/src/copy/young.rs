//! Young that grow up. A kind in a cell lives as an adult node and a young node joined by a link (`Body::link`): the
//! adult's children are booked on its young node (`copy`), and the young members mature one by one along the link at
//! the rate their growth gives (`mature`). A node keeps the spread of its members' sizes (`Body::size_spread`), so the
//! members who joined earlier, and so have grown more, pass the copy mass first: maturing is a flux, not a jump of the
//! mean. This is a population of two stages (Lefkovitch 1965) whose rates come from growth; real stands of plants have
//! such size hierarchies (Weiner 1990).

use super::kin;
use crate::body::Body;
use crate::math::Pass;
use crate::nodes::{may_be_node, merge, part};
use crate::World;

/// The node that node `b` (taken out of its slot) is linked to, if that is alive, a node of `b`'s cell and lineage
/// and mature as `mature` asks; else the first node of the cell at that stage and of `b`'s kind (`copy::kin`), linked from then on.
pub(crate) fn linked(w: &mut World, b: &mut Body, mature: bool) -> Option<usize> {
    let at = b.link.and_then(|id| w.life.find(id)).filter(|&t| {
        let o = &w.life[t];
        o.alive && o.n() > 1.0 && o.cell == b.cell && o.lineage == b.lineage && o.adult(&w.chem) == mature
    });
    at.or_else(|| {
        let t = kin(w, b.cell as usize, b.lineage, (&b.genes, &b.spread), mature)?;
        link(w, b, t);
        Some(t)
    })
}

/// Node `b` (taken out of its slot) and the node in slot `t` are linked.
pub(crate) fn link(w: &mut World, b: &mut Body, t: usize) {
    b.link = Some(w.life[t].id);
    w.life[t].link = Some(b.id);
}

/// What `mature` left of a young node.
pub enum Grown {
    /// The node, perhaps with fewer members
    Lives,
    /// Less than one member: the rest joins its adult node, in this slot
    Joins(usize),
}

/// Young node `b` (taken out of its slot) after an update in which its mean member grew from `before` kg: those who
/// passed the copy mass join its adult node with their share (`math::Pass`; McKendrick 1926, von Foerster 1959).
pub fn mature(w: &mut World, b: &mut Body, before: f64) -> Grown {
    if b.n() <= 1.0 || b.adult(&w.chem) || !may_be_node(w, b) {
        return Grown::Lives;
    }
    let (mean, v) = b.size_moments(&w.chem);
    let (mean, v, d) = (mean as f64, v as f64, (b.member_body(&w.chem) / before.max(f64::MIN_POSITIVE)).log10());
    let Some(pass) = Pass::new(mean - d, v, b.k.copy_mass.log10(), d) else { return Grown::Lives };
    let ((members, rest), held) = (pass.shares(0.0), pass.shares(std::f64::consts::LN_10).0);
    let adult = linked(w, b, true);
    match adult {
        Some(t) if rest * b.count < 1.0 => return Grown::Joins(t),
        None if members * b.count < 1.0 || rest * b.count < 1.0 => return Grown::Lives,
        _ if members <= 0.0 => return Grown::Lives,
        _ => {}
    }
    let (slice, stay) = pass.moments();
    let mut c = part(&mut w.life, b, members, held);
    b.set_size(&w.chem, stay.0, stay.1);
    c.set_size(&w.chem, slice.0, slice.1);
    let s = w.place(c);
    match adult {
        Some(t) => merge(w, s, t),
        None => link(w, b, s),
    }
    Grown::Lives
}
