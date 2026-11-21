//! The rows of a step of Change: each due node of a class reacts into a row of numbers (what its reactions changed, the
//! gas it gives its air node, its heat and lift, and how long it sleeps). The nodes of one block (those under
//! one air node) are one task of the pool (`par::longest_first`); the tasks make rows and book nothing
//! (`Rows::book`).

use super::Reactor;
use crate::{chem::Heat, flow::FlowView, ground::{ground_cap, share}, store::Class, World, AIR};

/// A node sleeps until its reactions would use this share of their inputs (but for those on a `SPECK` of what it
/// holds) or warm it `CALM_K` kelvin: for so little, one long span is as good as many steps.
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;

/// Nodes that react in the time of one item of work of the runner (`par::SERIAL_BELOW` items go to the pool): a node
/// reacts in a few microseconds, and a pool call costs tens.
const NODES_PER_ITEM: usize = 8;

/// A node's row holds this many numbers per substance: what its reactions made less what they used, mol; the gas it
/// gives its air node (+) or takes, mol, its heat, J, and its lift paid by the air, J; then how long it sleeps, s.
const ROW: usize = 4;

/// The rows of the nodes of one class that were due, ordered by block, then by index. A ground node is in the block of
/// its air node; the air nodes are cut in blocks of `NODES_PER_ITEM`.
pub(super) struct Rows {
    pub(super) class: Class,
    pub(super) nodes: Vec<usize>,
    /// Their rows, `ROW * ns + 1` numbers each
    data: Vec<f64>,
    pub(super) width: usize,
    /// The blocks that have a due node, in order
    pub(super) blocks: Vec<Block>,
}

/// A block with due nodes: where it is (for ground nodes, their air node), where its nodes end in `Rows::nodes`, and
/// the lift their gas costs the air node, J, summed in the order of the nodes.
pub(super) struct Block {
    pub(super) at: usize,
    end: usize,
    pub(super) lift: f64,
}

/// A node's amounts and the gas around it (a ground node's share of its air node) as it reacts, mol.
struct Held {
    amt: Vec<f64>,
    pool: Vec<f64>,
}

/// The reactor and buffers of a thread.
type Scratch = (Reactor, Held);

/// What the nodes of one class need to react for a step.
struct Step<'w> {
    w: &'w World,
    flow: FlowView<'w>,
    class: Class,
    /// The count of nodes of the class
    len: usize,
    /// The time they react until, s, and the longest they sleep, s
    end: u64,
    most: u64,
}

impl Rows {
    /// The `due` nodes of `class` react each for all the time since they last did, until `end`, at the temperatures of
    /// the last flow step, from the world as it is: the nodes of a block together, the most nodes first. A node sleeps at
    /// most `most` s.
    pub(super) fn make(w: &World, class: Class, due: &[usize], (end, most): (u64, u64)) -> Rows {
        let (ns, parent) = (w.chem.ns(), w.mesh.cells().parent());
        let (width, len) = (ROW * ns + 1, w.store.amounts(class).len() / ns);
        let block_of = |i: usize| if class == Class::Ground { parent[i] as usize } else { i / NODES_PER_ITEM };
        // A stable sort: the nodes of a block stay in the order of their indices.
        let mut nodes = due.to_vec();
        nodes.sort_by_key(|&i| block_of(i));
        let mut blocks: Vec<Block> = Vec::new();
        for (x, &i) in nodes.iter().enumerate() {
            match blocks.last_mut() {
                Some(b) if b.at == block_of(i) => b.end = x + 1,
                _ => blocks.push(Block { at: block_of(i), end: x + 1, lift: 0.0 }),
            }
        }
        let mut data = vec![0.0; nodes.len() * width];
        let step = Step { w, flow: w.scratch.view(), class, len, end, most };
        let (mut rest, mut from) = (&mut data[..], 0);
        let work = blocks
            .iter_mut()
            .map(|b| {
                let (mine, tail) = std::mem::take(&mut rest).split_at_mut((b.end - from) * width);
                let list = &nodes[from..b.end];
                (rest, from) = (tail, b.end);
                (b.at, list.len().div_ceil(NODES_PER_ITEM), (list, mine, &mut b.lift))
            })
            .collect();
        let held = || Held { amt: vec![0.0; ns], pool: vec![0.0; ns] };
        crate::par::longest_first(work, || (Reactor::new(&w.chem, &w.cfg), held()), |s, (list, rows, lift)| {
            for (&i, row) in list.iter().zip(rows.chunks_exact_mut(width)) {
                step.react(s, i, row);
                **lift = row[3 * ns..4 * ns].iter().fold(**lift, |sum, q| sum + q);
            }
        });
        Rows { class, nodes, data, width, blocks }
    }

    /// Each node and its row, in order.
    pub(super) fn each(&self) -> impl Iterator<Item = (usize, &[f64])> {
        self.nodes.iter().copied().zip(self.data.chunks_exact(self.width))
    }

    /// Each block and the rows of its nodes together.
    pub(super) fn by_block(&self) -> impl Iterator<Item = (&Block, &[f64])> {
        let mut from = 0;
        self.blocks.iter().map(move |b| {
            let rows = &self.data[from * self.width..b.end * self.width];
            from = b.end;
            (b, rows)
        })
    }

    /// Each node and how long it sleeps, s.
    pub(super) fn rests(&self) -> impl Iterator<Item = (usize, u64)> + '_ {
        self.each().map(|(i, row)| (i, row[self.width - 1] as u64))
    }
}

impl Step<'_> {
    /// Node `i` reacts into `row`, and the time it can sleep goes at its end.
    fn react(&self, (rc, h): &mut Scratch, i: usize, row: &mut [f64]) {
        let (w, ns) = (self.w, self.w.chem.ns());
        let amounts = w.store.amounts(self.class);
        h.amt.iter_mut().enumerate().for_each(|(k, a)| *a = amounts[k * self.len + i]);
        let (all, span) = (h.amt.iter().sum::<f64>(), self.end.saturating_sub(w.sleep.since(self.class, i)) as f64);
        if self.class == Class::Ground {
            react_ground(w, rc, h, &self.flow, i, span, row);
        } else {
            // An air node holds its gas itself: no liquid's film stands between them, and it is no moist ground
            // (the reactor's `film` and `moist` stay 0: it reacts only air nodes).
            rc.react(&w.chem, &mut h.amt, None, &mut { w.store.heat(Class::Air)[i] }, 0.0, span);
            rc.made(&w.chem, &mut row[..ns]);
        }
        debug_assert!(
            w.chem.keeps_elements(row[..ns].iter().enumerate().map(|(k, &d)| (k as u16, d)), &mut vec![0.0; w.chem.n_elements]),
            "a reaction that does not keep its elements: node {i} of {:?}",
            self.class
        );
        row[ROW * ns] = (rc.rest(CALM, SPECK * all, CALM_K).max(0.0) as u64).min(self.most) as f64;
    }
}

/// React ground node `i` for `time` s with the gas over it, noting in its row what it changes (laws.md 2.2). The
/// reactor sees sensible heat: the fusion enthalpy stays apart (`FlowView::fused`). A node gives no more heat than it
/// holds: each gas it gives is cut by the same share.
fn react_ground(w: &World, rc: &mut Reactor, h: &mut Held, nodes: &FlowView, i: usize, time: f64, row: &mut [f64]) {
    let ns = w.chem.ns();
    let (chem, na) = (&w.chem, w.mesh.level(AIR).n());
    let (j, air_share) = (w.mesh.up(i, 0, AIR), w.mesh.share(i, AIR));
    h.pool.iter_mut().enumerate().for_each(|(k, p)| *p = w.store.amounts(Class::Air)[k * na + j] * air_share);
    // The time the liquid standing over it takes to renew its gas through its film, s: depth / `dissolve` (the air's
    // side is 100 times faster or more, left out).
    (rc.film, rc.moist) = (nodes.standing(i).0 / w.cfg.dissolve, nodes.moist(i, &w.cfg.phys));
    // Ground under no standing liquid: its decomposers breathe in the share of it whose pores hold gas
    // (`inside::airless`; under standing liquid the liquid's film brings their gas).
    rc.aired = if rc.film > 0.0 { 1.0 } else { 1.0 - crate::inside::airless(w.pores(i), crate::inside::saturated_share(w, i), nodes.filled(i)) };
    rc.shelter = w.soil[i].texture.shelters();
    let cap = |amt: &[f64]| ground_cap(nodes.skin[i], 0..chem.ns(), |k| amt[k], |k| chem.subs[k].cp);
    let fused = nodes.fused(i, w.store.heat(Class::Ground)[i], cap(&h.amt), |k| h.amt[k] * chem.subs[k].lf);
    let mut e = w.store.heat(Class::Ground)[i] - fused;
    rc.react(chem, &mut h.amt, Some(&mut h.pool), &mut e, nodes.skin[i], time);
    let (made, out) = row.split_at_mut(ns);
    rc.made(chem, made);
    // What they gave to the gas around the node, or took from it, mol: what they made less what the node itself gained
    // (its amount's change), so the node's books end at what it really holds. The reactor's own sum of the gas it
    // exchanged sums large terms that cancel apart from `made`, and could differ from it by their rounding.
    let kept = |k: usize| h.amt[k] - w.store.amounts(Class::Ground)[k * w.mesh.cells().n() + i];
    let gas = |k: usize| if rc.dpool[k] != 0.0 { made[k] - kept(k) } else { 0.0 };
    // Gas given to the air takes the node's heat by heat capacity, shared with the node as it is after its reactions.
    let c_out = (0..chem.ns()).map(|k| gas(k).max(0.0) * chem.subs[k].cp).sum::<f64>();
    let per_c = e.max(0.0) / (cap(&h.amt) + c_out).max(1e-9);
    let pay = (0..chem.ns()).map(|k| gas(k).max(0.0) * chem.subs[k].carried_gas(per_c)).sum::<f64>();
    let (paid, lift) = (share(e.max(0.0), pay), nodes.lift(i, j, w.cfg.gravity));
    for (k, m) in (0..ns).map(|k| (k, gas(k))).filter(|e| e.1 != 0.0) {
        let (s, m) = (&chem.subs[k], if m > 0.0 { m * paid } else { m });
        let heat = m * s.carried_gas(if m > 0.0 { per_c } else { nodes.ta[j] });
        (out[k], out[ns + k], out[2 * ns + k]) = (m, heat, m * s.molar_mass * lift);
    }
}
