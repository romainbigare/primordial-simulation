//! Booking the rows of a step of Change (`Rows`): each array of the store that the rows change is one task of
//! `store::book_by_array`, which walks the rows in their order and adds both sides of each move, so each number has one
//! writer and its changes come in the order of the rows, whatever thread runs the task.

use super::rows::Rows;
use crate::store::{book_by_array, heat_received, held, held_more, Booker, Class};
use crate::World;
use std::ops::Range;

/// Rows that make one item of work of the booking (`par::SERIAL_BELOW` items go to the pool): a row costs less than a
/// microsecond to book, and a pool call tens.
const ROWS_PER_ITEM: usize = 64;

/// The rows and what booking them needs of the world.
struct Columns<'a> {
    rows: &'a Rows,
    ns: usize,
    /// What a mol of each substance holds apart from heat in a node of the rows' class, J/mol (`held`)
    held: Vec<f64>,
    /// What a mol of each holds more in the air than in the ground, J/mol (`held_more`)
    more: Vec<f64>,
}

impl Rows {
    /// Book the rows on the world: the reactions of each node, and the gas a ground node gives its air node with its
    /// heat and latent heat, the air paying its lift from its heat into the potential energy (`Store::lifted`). What rounding leaves below zero is set to
    /// zero and counted (`Counters::rounded`). No domain's view of the store is open then (`Moves::open`; checked in
    /// debug builds).
    pub(super) fn book(&self, w: &mut World) {
        debug_assert!(w.life.domains.iter().all(|d| d.moves.open.is_none()), "the store changes while a domain is open");
        let World { chem, store, counts, .. } = w;
        let ns = chem.ns();
        let cols = Columns {
            rows: self,
            ns,
            held: (0..ns).map(|k| held(chem, self.class, k)).collect(),
            more: (0..ns).map(|k| held_more(chem, Class::Ground, Class::Air, k)).collect(),
        };
        let classes: &[Class] = if self.class == Class::Ground { &[Class::Ground, Class::Air] } else { &[Class::Air] };
        counts.add_rounded(book_by_array(store, ns, classes, self.nodes.len() / ROWS_PER_ITEM, |b| cols.changes(b)));
        // The potential energy is a whole history's sum: the lifts are summed apart, in the order of the air nodes, and
        // added once.
        store.lift(self.blocks.iter().fold(0.0, |far, b| far + b.lift));
    }
}

impl Columns<'_> {
    /// The changes to the numbers of the array of booker `b`, both sides of each move (`book_by_array`).
    fn changes(&self, b: &mut Booker) {
        match (b.class, b.ks.clone()) {
            (class, Some(ks)) if class == self.rows.class => self.own_amounts(b, ks),
            (class, None) if class == self.rows.class => self.own_heat(b),
            (_, Some(ks)) => self.air_amounts(b, ks),
            _ => self.air_heat(b),
        }
    }
    /// The amounts of the nodes that reacted: the gas they gave their air node, then what their reactions made.
    fn own_amounts(&self, b: &mut Booker, ks: Range<usize>) {
        let (ns, len) = (self.ns, b.len);
        for (i, row) in self.rows.each() {
            b.column(|q| (ks.start + q) * len + i, &row[ns + ks.start..ns + ks.end], -1.0);
            b.column(|q| (ks.start + q) * len + i, &row[ks.clone()], 1.0);
        }
    }
    /// The heat of the nodes that reacted: what their gas takes with it, then what their reactions hold apart from heat
    /// less after them than before (the heat of reaction).
    fn own_heat(&self, b: &mut Booker) {
        let ns = self.ns;
        for (i, row) in self.rows.each() {
            b.column(|_| i, &row[2 * ns..3 * ns], -1.0);
            if row[..ns].iter().any(|&d| d != 0.0) {
                b.add(i, (0..ns).filter(|&k| row[k] != 0.0).fold(0.0, |q, k| q - row[k] * self.held[k]));
            }
        }
    }
    /// The gas the ground nodes gave goes into their air nodes.
    fn air_amounts(&self, b: &mut Booker, ks: Range<usize>) {
        let (ns, len, width) = (self.ns, b.len, self.rows.width);
        for (block, rows) in self.rows.by_block() {
            for row in rows.chunks_exact(width) {
                b.column(|q| (ks.start + q) * len + block.domain, &row[ns + ks.start..ns + ks.end], 1.0);
            }
        }
    }
    /// The heat of the air nodes: the heat of the gas their ground nodes gave less what it holds more in the air, one gas
    /// after the other (the rows of one air node stay near each other), then the lift they pay.
    fn air_heat(&self, b: &mut Booker) {
        let (ns, width) = (self.ns, self.rows.width);
        for (block, rows) in self.rows.by_block() {
            for k in 0..ns {
                for row in rows.chunks_exact(width).filter(|row| row[ns + k] != 0.0) {
                    b.add(block.domain, heat_received(row[2 * ns + k], row[ns + k], self.more[k]));
                }
            }
            if block.lift != 0.0 {
                b.add(block.domain, -block.lift);
            }
        }
    }
}
