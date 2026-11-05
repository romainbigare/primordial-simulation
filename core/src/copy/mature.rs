//! Maturity by share: the members of a young node mature as each reaches the copy mass (`mature`).

use crate::body::Body;
use crate::nodes::{add, part, Lot, Own};
use crate::{life::Chunk, World};

/// The members of young node `b` (taken out of its slot; a node whose mean member is below its copy mass) that have
/// reached its copy mass, at the end of a sub-step of its physics, move to the mature node of its lineage in its cell
/// (`kin`, as children join its young node), or start one, a copy of its brain, if there is none and they are at least
/// one member (fewer wait). The flux of members across the size of maturity is a McKendrick–von Foerster boundary flux
/// (McKendrick 1926; von Foerster 1959): the node holds members born at different times, their sizes spread around
/// its mean member by `Body::body_spread`, taken as spread evenly over the width that variance gives (at most from
/// nothing to twice the mean: no member is smaller than nothing). Those of the even spread past the copy mass move,
/// as a share of the members and the larger share of the amounts their larger bodies hold (`Lot::split`: members,
/// stores, heat and brood exact); each part keeps its own even spread, the movers' pooled into the node they join
/// (`nodes::add`). Members grow alike in a sub-step, so the spread stays as growth moves it up, and the children who
/// join at the size of birth widen it (`nodes::add`). At least one member stays young. A node of members all alike
/// (no spread) matures whole when its mean does, as an individual does (`Body::adult`).
pub(crate) fn mature(w: &World, d: &mut Chunk, b: &mut Body) {
    let chem = &w.chem;
    if b.count <= 1.0 || b.adult(chem) {
        return;
    }
    let (mean, mc) = (b.member_body(chem), b.k.copy_mass);
    let half = (3.0 * b.body_spread.max(0.0) as f64).sqrt().min(mean);
    let (low, high) = (mean - half, mean + half);
    if high <= mc {
        return;
    }
    // The share of members past the copy mass, the size the stayers reach up to, and the share of the amounts the
    // movers hold (their mean member over the node's).
    let f = ((high - mc) / (2.0 * half)).min(1.0 - 1.0 / b.count);
    let cut = high - 2.0 * half * f;
    let fa = f * (cut + high) / 2.0 / mean;
    let i = b.cell as usize;
    let movers = ((high - cut).powi(2) / 12.0) as f32;
    match super::kin(w, d, i, b.lineage, true) {
        Some(t) => {
            let lot = Lot::split(b, f, fa);
            add(w, d, t, lot, (&b.genes, &b.spread, movers), (b.lineage, b.genome, Some(Own::Block(b.brain)), b.row_spread));
        }
        None if f * b.count >= 1.0 => {
            let mut m = part(b, f, fa);
            (m.brain, m.body_spread) = (d.brains(i).copy(b.brain), movers);
            let s = d.place_part(w, m);
            // (`place_part` starts a body's time now; theirs ran with the node's to the sub-step's end.)
            (d[s].last, d[s].decided) = (b.last, b.decided);
        }
        None => return,
    }
    b.body_spread = ((cut - low).powi(2) / 12.0) as f32;
}
