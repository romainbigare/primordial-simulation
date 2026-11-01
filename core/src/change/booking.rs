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
    /// Per row of ground nodes, the heat its node gives the potential energy, J (`fall`)
    fall: Vec<f64>,
}

impl Rows {
    /// Book the rows on the world: the reactions of each node, and the gas a ground node gives its air node with its
    /// heat and latent heat, the air paying its lift from its heat into the potential energy (`Store::lifted`), and a
    /// ground node what its stack gains (`fall`). What rounding leaves below zero is set to zero and counted
    /// (`Counters::rounded`).
    pub(super) fn book(&self, w: &mut World) {
        let fall = self.fall(w);
        let World { chem, store, counts, .. } = w;
        let ns = chem.ns();
        let cols = Columns {
            rows: self,
            ns,
            held: (0..ns).map(|k| held(chem, self.class, k)).collect(),
            more: (0..ns).map(|k| held_more(chem, Class::Ground, Class::Air, k)).collect(),
            fall,
        };
        let classes: &[Class] = if self.class == Class::Ground { &[Class::Ground, Class::Air] } else { &[Class::Air] };
        counts.add_rounded(book_by_array(store, ns, classes, self.nodes.len() / ROWS_PER_ITEM, |b| cols.changes(b)));
        // The potential energy is a whole history's sum: the lifts are summed apart, in the order of the air nodes, and
        // added once; then what the ground nodes' stacks gained, in the order of the rows.
        store.lift(self.blocks.iter().fold(0.0, |far, b| far + b.lift));
        store.lift(-cols.fall.iter().sum::<f64>());
    }

    /// Per row of ground nodes, what its node's heat gives the potential energy, J: its gas leaves from its head (the
    /// air pays the rest of its lift, `rows`), and its reactions and the gas it gave move the matter of its stack
    /// (`flow::stack_gain`); none for the rows of air nodes, whose matter all lies at their mean ground (laws.md 0, "The
    /// books").
    fn fall(&self, w: &World) -> Vec<f64> {
        if self.class != Class::Ground {
            return Vec::new();
        }
        let (ns, g, head) = (w.chem.ns(), w.cfg.gravity, w.scratch.view().head);
        let rows: Vec<(usize, &[f64])> = self.each().collect();
        crate::par::map(&rows, |&(i, row)| {
            let up: f64 = (0..ns).map(|k| row[ns + k] * w.chem.subs[k].molar_mass).sum();
            -(crate::flow::stack_gain(w, i, |k| row[k] - row[ns + k]) + up * g * head[i])
        })
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
    /// less after them than before (the heat of reaction), then what their stacks gained (`fall`).
    fn own_heat(&self, b: &mut Booker) {
        let ns = self.ns;
        for (r, (i, row)) in self.rows.each().enumerate() {
            b.column(|_| i, &row[2 * ns..3 * ns], -1.0);
            if row[..ns].iter().any(|&d| d != 0.0) {
                b.add(i, (0..ns).filter(|&k| row[k] != 0.0).fold(0.0, |q, k| q - row[k] * self.held[k]));
            }
            if let Some(&q) = self.fall.get(r).filter(|q| **q != 0.0) {
                b.add(i, q);
            }
        }
    }
    /// The gas the ground nodes gave goes into their air nodes.
    fn air_amounts(&self, b: &mut Booker, ks: Range<usize>) {
        let (ns, len, width) = (self.ns, b.len, self.rows.width);
        for (block, rows) in self.rows.by_block() {
            for row in rows.chunks_exact(width) {
                b.column(|q| (ks.start + q) * len + block.at, &row[ns + ks.start..ns + ks.end], 1.0);
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
                    b.add(block.at, heat_received(row[2 * ns + k], row[ns + k], self.more[k]));
                }
            }
            if block.lift != 0.0 {
                b.add(block.at, -block.lift);
            }
        }
    }
}
