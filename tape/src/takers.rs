//! The takers of a cell as a viewer draws them (laws.md 3b, docs/tape.md): each of its slots, its members as the field
//! holds them (real counts of adults and of young), their mean living mass and the genes a viewer shows them by. The
//! model holds no place inside a cell; the page sets each member at a place fixed by its cell, slot and number.

use pcore::fauna::{self, Fauna, COAT, FS, KEEP, REACH};
use pcore::life::Parts;
use pcore::World;

/// Slots a square's row holds: the slots of its cell, in their own order (a slot's number is part of where its members
/// stand).
pub const TAKER_SLOTS: usize = FS;
/// Numbers a slot holds in a square's row (`row`).
pub const TAKER_ROW: usize = 8;

/// The numbers a viewer draws slot `x` of takers `f` from, its members shared evenly over the `squares` squares its cell
/// holds: its starting kind; its adults and its young in the square; the living mass of an adult by its genes, kg
/// (`fauna::adult`: the frame it is built to, whatever its store holds now), and the mean living mass of a young one,
/// kg (`Fauna::living_of` over its count; 0 for no young); its coat (`COAT`), the body temperature it keeps, K
/// (`KEEP`), and the middle of the band of heights it searches, log10 m (`REACH`).
fn row(f: &Fauna, mm: &[f64; 2], x: usize, squares: f64) -> [f64; TAKER_ROW] {
    let (adults, young) = (f.n[x], f.n[FS + x]);
    let genes = f.genes[x];
    [
        f.root[x] as f64,
        adults / squares,
        young / squares,
        fauna::adult(&genes),
        if young > 0.0 { f.living_of(mm, FS + x) / young } else { 0.0 },
        genes[COAT] as f64,
        genes[KEEP] as f64,
        genes[REACH] as f64,
    ]
}

/// The takers of cell `i` of `w`, of which the share `land` is land and which `squares` squares of the map show,
/// `TAKER_SLOTS` rows of `TAKER_ROW` numbers (`row`): slot by slot; an empty slot is all 0 with its starting kind -1, and
/// a cell that holds no land has none (no land to draw them on).
pub fn rows(w: &World, i: usize, land: f64, squares: usize) -> [f64; TAKER_SLOTS * TAKER_ROW] {
    let mut out = [0.0; TAKER_SLOTS * TAKER_ROW];
    (0..TAKER_SLOTS).for_each(|x| out[x * TAKER_ROW] = -1.0);
    let Some(p) = Parts::of(&w.chem).filter(|_| land > 0.0 && squares > 0) else { return out };
    let (mm, f) = (fauna::mm(&w.chem, &p), &w.life.cell_at(i).fauna);
    for x in f.held() {
        out[x * TAKER_ROW..(x + 1) * TAKER_ROW].copy_from_slice(&row(f, &mm, x, squares as f64));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slot's row holds its members as the field holds them: the placed count of adults, no young, an adult's mass
    /// by its genes, each slot in its own place; the members are shared over the cell's squares, and a cell with no
    /// land has none.
    #[test]
    fn rows_hold_the_slots_members() {
        let mut w = worldgen::testkit::small_world();
        let i = worldgen::testkit::mild_land(&w);
        worldgen::testkit::takers(&mut w, i, 2, (7.0, 1.0));
        let f = &w.life.cell_at(i).fauna;
        let x = f.held().next().expect("a held slot");
        let r = rows(&w, i, 1.0, 1);
        let slot = &r[x * TAKER_ROW..(x + 1) * TAKER_ROW];
        assert_eq!((slot[0], slot[1], slot[2]), (2.0, 7.0, 0.0), "the starting kind, 7 adults, no young");
        let m = fauna::adult(&worldgen::life::TAKER_KINDS[2].genes);
        assert!((slot[3] / m - 1.0).abs() < 1e-9, "an adult's mass {} against {m}", slot[3]);
        assert!((0..TAKER_SLOTS).filter(|&y| y != x).all(|y| r[y * TAKER_ROW] == -1.0), "the other slots are empty");
        assert_eq!(rows(&w, i, 1.0, 2)[x * TAKER_ROW + 1], 3.5, "shared over two squares");
        assert!(rows(&w, i, 0.0, 1).chunks(TAKER_ROW).all(|s| s[0] == -1.0), "none where no land");
    }
}
