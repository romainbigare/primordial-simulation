//! The tree of cells: under each air block of the map a quadtree whose leaves are the cells. The block is the root
//! (depth 0); a leaf at depth `d` is 1/2^d of the block's side, aligned to its own size. Only the leaves are kept
//! (`Leaf`); `Tree` finds the leaf that holds a point. Corners and points are counted in leaves of the most depth
//! (`DEPTH_MOST`), east and south from the north-west corner of the map, as 64-bit numbers.

use serde::{Deserialize, Serialize};

/// The most depth of a cell below its air block: a leaf of this depth is 1/2^24 of the block's side (0.5 mm on the
/// island's block of 8 km, 7.5 cm on the block of 1,250 km of a globe of 64 rows), finer than any body.
pub const DEPTH_MOST: u8 = 24;
/// The side of an air block, in leaves of the most depth.
pub const BLOCK: u64 = 1 << DEPTH_MOST;
/// A map is less than this many leaves of the most depth along each side (2^53, under 2^29 air blocks), so a place
/// counted in them is exact as a 64-bit float.
pub const EXTENT_MOST: u64 = 1 << f64::MANTISSA_DIGITS;
/// An entry of the tree that is a leaf (the rest of its bits: the leaf's cell), and one that holds nothing yet.
const LEAF: u32 = 1 << 31;
const EMPTY: u32 = LEAF - 1;

/// A cell of the tree: the column and row of its north-west corner, in leaves of the most depth, and its depth below
/// its block. Leaves order by their north-west corners, row by row (the order of the cells).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Leaf {
    pub y: u64,
    pub x: u64,
    pub depth: u8,
}

impl Leaf {
    /// Its side, in leaves of the most depth.
    pub fn side(&self) -> u64 {
        BLOCK >> self.depth
    }
    /// The north-west corner, (column, row).
    pub fn corner(&self) -> [u64; 2] {
        [self.x, self.y]
    }
    /// The leaves of `depth` (no less than its own) that tile it, row by row.
    pub fn split(self, depth: u8) -> impl Iterator<Item = Leaf> {
        assert!((self.depth..=DEPTH_MOST).contains(&depth), "a leaf splits into leaves of a depth from its own to the most");
        let (n, side) = (1u64 << (depth - self.depth), BLOCK >> depth);
        (0..n).flat_map(move |r| (0..n).map(move |c| Leaf { y: self.y + r * side, x: self.x + c * side, depth }))
    }
    /// The leaf it is a quarter of (none for an air block, a root).
    pub fn parent(&self) -> Option<Leaf> {
        let side = BLOCK >> self.depth.checked_sub(1)?;
        Some(Leaf { y: self.y / side * side, x: self.x / side * side, depth: self.depth - 1 })
    }
    /// Its four quarters, row by row (a leaf above the most depth).
    pub fn quarters(self) -> [Leaf; 4] {
        let mut q = self.split(self.depth + 1);
        std::array::from_fn(|_| q.next().expect("a leaf has four quarters"))
    }
}

/// The ways across the sides of a leaf: east, south, west and north, (east, south).
pub(crate) const SIDES: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// The tree of a map's leaves (`Leaf`), to find the leaf that holds a point: an entry for each air block, an inner
/// node four entries for its quarters.
#[derive(Clone, Debug, Default)]
pub struct Tree {
    /// The map's width and height, in leaves of the most depth, and whether its rows wrap east to west
    across: u64,
    down: u64,
    wrap: bool,
    /// The entry of each air block, row by row; the four quarters of each inner node, row by row
    roots: Vec<u32>,
    quarters: Vec<[u32; 4]>,
}

/// The quarter of an inner node at `depth` that holds the point (`x`, `y`): its place, row by row.
fn quarter(x: u64, y: u64, depth: u8) -> usize {
    let bit = DEPTH_MOST - depth - 1;
    (((y >> bit) & 1) * 2 + ((x >> bit) & 1)) as usize
}

impl Tree {
    /// The tree of `leaves` (leaf `i`: cell `i`) on a map `across` × `down` leaves of the most depth. The leaves tile
    /// the map: each lies inside it, aligned to its size, and none overlaps another.
    pub(crate) fn new(leaves: &[Leaf], across: u64, down: u64, wrap: bool) -> Tree {
        let blocks = (across.div_ceil(BLOCK) * down.div_ceil(BLOCK)) as usize;
        let mut t = Tree { across, down, wrap, roots: vec![EMPTY; blocks], quarters: Vec::new() };
        let mut covered = 0u128;
        for (i, l) in leaves.iter().enumerate() {
            let s = l.side();
            assert!(l.depth <= DEPTH_MOST && l.x.is_multiple_of(s) && l.y.is_multiple_of(s) && l.x + s <= across && l.y + s <= down, "leaf {i} lies inside the map, aligned to its size");
            // The entry that holds the leaf: an air block's (`None`), or a quarter of an inner node.
            let mut slot: (Option<usize>, usize) = (None, t.block(l.x, l.y));
            for d in 0..l.depth {
                let mut e = t.entry(slot);
                if e == EMPTY {
                    e = t.quarters.len() as u32;
                    assert!(e < EMPTY, "the tree has room for its nodes");
                    t.quarters.push([EMPTY; 4]);
                    t.set(slot, e);
                }
                assert!(e & LEAF == 0, "leaf {i} lies inside no other leaf");
                slot = (Some(e as usize), quarter(l.x, l.y, d));
            }
            assert_eq!(t.entry(slot), EMPTY, "leaf {i} holds no other leaf");
            t.set(slot, LEAF | i as u32);
            covered += (s as u128).pow(2);
        }
        assert_eq!(covered, across as u128 * down as u128, "the leaves cover the map");
        t
    }
    /// The air block that holds the point (`x`, `y`), numbered row by row.
    pub fn block(&self, x: u64, y: u64) -> usize {
        ((y / BLOCK) * self.across.div_ceil(BLOCK) + x / BLOCK) as usize
    }
    /// The column and row of air block `j` among the blocks.
    pub fn block_xy(&self, j: usize) -> (usize, usize) {
        let across = self.across.div_ceil(BLOCK) as usize;
        (j % across, j / across)
    }
    /// The cell that holds the point `p` of the map.
    pub fn cell_at(&self, p: [u64; 2]) -> usize {
        self.at(p[0] as i64, p[1] as i64).expect("a point inside the map")
    }
    /// The entry an air block's (`None`) or an inner node's (`Some`) place `q` holds.
    fn entry(&self, (node, q): (Option<usize>, usize)) -> u32 {
        match node {
            Some(n) => self.quarters[n][q],
            None => self.roots[q],
        }
    }
    fn set(&mut self, (node, q): (Option<usize>, usize), e: u32) {
        match node {
            Some(n) => self.quarters[n][q] = e,
            None => self.roots[q] = e,
        }
    }
    /// The leaf that holds the point (`x`, `y`), round a globe east and west; none outside the map.
    pub(crate) fn at(&self, x: i64, y: i64) -> Option<usize> {
        let x = if self.wrap { x.rem_euclid(self.across as i64) } else { x };
        if x < 0 || y < 0 || x >= self.across as i64 || y >= self.down as i64 {
            return None;
        }
        let (x, y) = (x as u64, y as u64);
        let (mut e, mut d) = (self.roots[self.block(x, y)], 0);
        while e & LEAF == 0 {
            e = self.quarters[e as usize][quarter(x, y, d)];
            d += 1;
        }
        Some((e & !LEAF) as usize)
    }
    /// The cell that holds the point (`x`, `y`) in leaves of the most depth, round a globe east and west; none off the
    /// map.
    pub(crate) fn holding(&self, x: f64, y: f64) -> Option<usize> {
        self.at(x.floor() as i64, y.floor() as i64)
    }
    /// The leaves across the side of `leaf` that lies the way `way` (`SIDES`), in order from north to south or west to
    /// east: those at the starts of the side's two halves (of the side, for a leaf of the most depth), each once. Where
    /// leaves across a border differ by one depth at most, they are all the leaves there; where one is more than a
    /// depth finer, so is one of them.
    pub(crate) fn beside(&self, leaf: &Leaf, way: (i64, i64)) -> impl Iterator<Item = usize> {
        let (s, x, y) = (leaf.side() as i64, leaf.x as i64, leaf.y as i64);
        let from = match way {
            (1, _) => (x + s, y),
            (-1, _) => (x - 1, y),
            (_, 1) => (x, y + s),
            _ => (x, y - 1),
        };
        let half = (s / 2).max(1);
        let along = if way.0 == 0 { (half, 0) } else { (0, half) };
        let first = self.at(from.0, from.1);
        let second = (s > 1).then(|| self.at(from.0 + along.0, from.1 + along.1)).flatten().filter(|&c| Some(c) != first);
        first.into_iter().chain(second)
    }
}

/// `leaves` on a map `across` × `down` (`Tree::new`), each split where it borders a leaf more than one depth finer,
/// until none does (2:1 balance), in order.
pub(crate) fn balance(mut leaves: Vec<Leaf>, across: u64, down: u64, wrap: bool) -> Vec<Leaf> {
    loop {
        leaves.sort_unstable();
        let tree = Tree::new(&leaves, across, down, wrap);
        let coarse: Vec<bool> = leaves.iter().map(|l| SIDES.iter().any(|&way| tree.beside(l, way).any(|c| leaves[c].depth > l.depth + 1))).collect();
        if !coarse.contains(&true) {
            return leaves;
        }
        leaves = leaves.iter().zip(coarse).flat_map(|(&l, split)| l.split(if split { l.depth + 1 } else { l.depth })).collect();
    }
}
