//! Booking the rows of a step of Change (`Rows`): each array of the store that the rows change is one task of
//! `store::book_by_array`, which walks the rows in their order and adds both sides of each move, so each number has one
//! writer and its changes come in the order of the rows, whatever thread runs the task.

use super::rows::Rows;
use crate::store::{book_by_array, held, held_more, Booker, Class};
use crate::World;
use std::ops::Range;

/// Rows that make one item of work of the booking (`par::SERIAL_BELOW` items go to the pool): a row costs less than a
/// microsecond to book, and a pool call tens.
const ROWS_PER_ITEM: usize = 64;

/// The rows and what booking them needs of the world.
struct Columns<'a> {
    rows: &'a Rows,
    ns: usize,
    /// The count of nodes of the rows' class, and of the air nodes
    len: usize,
    na: usize,
    /// The air node of each ground node
    parent: &'a [u32],
    /// What a mol of each substance holds apart from heat in a node of the rows' class, J/mol (`held`)
    held: Vec<f64>,
    /// What a mol of each holds more in the air than in the ground, J/mol (`held_more`)
    more: Vec<f64>,
}

impl Rows {
    /// Book the rows on the world: the reactions of each node, and the gas a ground node gives its air node with its
    /// heat and latent heat, the air paying the lift out of the potential. What rounding leaves below zero is set to
    /// zero and counted (`Counters::rounded`).
    pub(super) fn book(&self, w: &mut World) {
        let World { chem, store, mesh, counts, .. } = w;
        let ns = chem.ns();
        let cols = Columns {
            rows: self,
            ns,
            len: store.amounts(self.class).len() / ns,
            na: store.amounts(Class::Air).len() / ns,
            parent: mesh.cells().parent(),
            held: (0..ns).map(|k| held(chem, self.class, k)).collect(),
            more: (0..ns).map(|k| held_more(chem, Class::Ground, Class::Air, k)).collect(),
        };
        let classes: &[Class] = if self.class == Class::Ground { &[Class::Ground, Class::Air] } else { &[Class::Air] };
        let rounded = book_by_array(store, ns, classes, self.nodes.len() / ROWS_PER_ITEM, |b| cols.book(b));
        (counts.rounded[0], counts.rounded[1]) = (counts.rounded[0] + rounded[0], counts.rounded[1] + rounded[1]);
        // The potential holds a whole history's sum: the lifts are summed apart, in the order of the air nodes, and
        // added once.
        store.add_far_heat(Class::Potential, 0, self.blocks.iter().fold(0.0, |far, b| far + b.lift));
    }
}

impl Columns<'_> {
    /// The changes to the numbers of the array of booker `b`, both sides of each move (`book_by_array`).
    fn book(&self, b: &mut Booker) {
        match (b.class, b.ks.clone()) {
            (class, Some(ks)) if class == self.rows.class => self.own_amounts(b, ks),
            (class, None) if class == self.rows.class => self.own_heat(b),
            (_, Some(ks)) => self.air_amounts(b, ks),
            _ => self.air_heat(b),
        }
    }
    /// The amounts of the nodes that reacted: the gas they gave their air node, then what their reactions made.
    fn own_amounts(&self, b: &mut Booker, ks: Range<usize>) {
        for (i, row) in self.rows.each() {
            for k in ks.clone() {
                let (gas, made) = (row[self.ns + k], row[k]);
                if gas != 0.0 {
                    b.add(k * self.len + i, -gas);
                }
                if made != 0.0 {
                    b.add(k * self.len + i, made);
                }
            }
        }
    }
    /// The heat of the nodes that reacted: what their gas takes with it, then what their reactions hold apart from heat
    /// less after them than before (the heat of reaction).
    fn own_heat(&self, b: &mut Booker) {
        let ns = self.ns;
        for (i, row) in self.rows.each() {
            (0..ns).filter(|&k| row[ns + k] != 0.0).for_each(|k| b.add(i, -row[2 * ns + k]));
            if row[..ns].iter().any(|&d| d != 0.0) {
                b.add(i, (0..ns).filter(|&k| row[k] != 0.0).fold(0.0, |q, k| q - row[k] * self.held[k]));
            }
        }
    }
    /// The gas the ground nodes gave goes into their air nodes.
    fn air_amounts(&self, b: &mut Booker, ks: Range<usize>) {
        for (i, row) in self.rows.each() {
            for k in ks.clone().filter(|&k| row[self.ns + k] != 0.0) {
                b.add(k * self.na + self.parent[i] as usize, row[self.ns + k]);
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
                    b.add(block.domain, row[2 * ns + k] - row[ns + k] * self.more[k]);
                }
            }
            if block.lift != 0.0 {
                b.add(block.domain, -block.lift);
            }
        }
    }
}
