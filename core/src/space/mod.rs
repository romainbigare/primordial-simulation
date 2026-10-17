//! Space: the shape of the world, and nothing else. A mesh is a set of levels; each level is a set of cells (area,
//! centre, latitude) and the links between them (the two cells, the distance between their centres, the width of the
//! border they share); each cell of a level but the coarsest has one parent at the next coarser level. The height of
//! the ground is not here: it changes with what the ground holds, so it belongs to the ground.
//!
//! Links are stored as plain arrays, each link once (`a < b`), and each cell's links in compressed form (`start`,
//! then for each entry the link and the other cell), so a solver takes them as slices. Built once per mesh change.

pub mod square;
pub mod stack;

pub use stack::{Layer, Stack};

/// One level of a mesh: its cells, its links, and how it nests in the levels next to it.
#[derive(Clone, Debug, Default)]
pub struct Level {
    /// Area of each cell, m²; its centre on the map (east, south), m; its latitude, degrees
    area: Vec<f64>,
    centre: Vec<[f64; 2]>,
    lat: Vec<f64>,
    /// The two cells of each link, the distance between their centres, m, and the width of their shared border, m
    /// (0 for cells that touch only at a corner)
    a: Vec<u32>,
    b: Vec<u32>,
    dist: Vec<f64>,
    width: Vec<f64>,
    /// The links of cell `i` are entries `start[i]..start[i + 1]`, in link order: each entry's link and other cell
    start: Vec<u32>,
    adj_link: Vec<u32>,
    adj_cell: Vec<u32>,
    /// Each cell's parent at the next coarser level (empty at the coarsest)
    parent: Vec<u32>,
    /// The children of cell `i` at the next finer level are `kids[kid_start[i]..kid_start[i + 1]]`, in their order
    /// (empty at the finest)
    kid_start: Vec<u32>,
    kids: Vec<u32>,
}

/// A link while a level is built: the two cells (`a < b`), the distance and the width, m.
pub type LinkSpec = (u32, u32, f64, f64);

/// Compressed lists of the members of each group: `start` (len `groups + 1`) and the members in the order given.
fn compress(groups: usize, members: impl Iterator<Item = (u32, u32)> + Clone) -> (Vec<u32>, Vec<u32>) {
    let mut start = vec![0u32; groups + 1];
    for (g, _) in members.clone() {
        start[g as usize + 1] += 1;
    }
    for g in 0..groups {
        start[g + 1] += start[g];
    }
    let (mut next, mut list) = (start.clone(), vec![0u32; start[groups] as usize]);
    for (g, m) in members {
        list[next[g as usize] as usize] = m;
        next[g as usize] += 1;
    }
    (start, list)
}

impl Level {
    /// A level of cells and links (each link once, `a < b`).
    pub fn new(area: Vec<f64>, centre: Vec<[f64; 2]>, lat: Vec<f64>, links: Vec<LinkSpec>) -> Level {
        let n = area.len();
        assert!(centre.len() == n && lat.len() == n, "a level needs a centre and a latitude for each cell");
        assert!(links.iter().all(|&(a, b, ..)| a < b && (b as usize) < n), "each link joins two cells a < b of the level");
        let ends = links.iter().enumerate().flat_map(|(l, &(a, b, ..))| [(a, (l as u32, b)), (b, (l as u32, a))]);
        let (start, adj_link) = compress(n, ends.clone().map(|(c, (l, _))| (c, l)));
        let adj_cell = compress(n, ends.map(|(c, (_, o))| (c, o))).1;
        let mut lv = Level { area, centre, lat, start, adj_link, adj_cell, ..Default::default() };
        for (a, b, dist, width) in links {
            lv.a.push(a);
            lv.b.push(b);
            lv.dist.push(dist);
            lv.width.push(width);
        }
        lv
    }
    pub fn n(&self) -> usize {
        self.area.len()
    }
    pub fn area(&self) -> &[f64] {
        &self.area
    }
    pub fn centre(&self) -> &[[f64; 2]] {
        &self.centre
    }
    pub fn lat(&self) -> &[f64] {
        &self.lat
    }
    pub fn a(&self) -> &[u32] {
        &self.a
    }
    pub fn b(&self) -> &[u32] {
        &self.b
    }
    pub fn dist(&self) -> &[f64] {
        &self.dist
    }
    pub fn width(&self) -> &[f64] {
        &self.width
    }
    /// The compressed adjacency: `start` (len n + 1), and for each entry its link and its other cell.
    pub fn start(&self) -> &[u32] {
        &self.start
    }
    pub fn adj_link(&self) -> &[u32] {
        &self.adj_link
    }
    pub fn adj_cell(&self) -> &[u32] {
        &self.adj_cell
    }
    /// The links of cell `i` and the cells at their other ends.
    pub fn around(&self, i: usize) -> (&[u32], &[u32]) {
        let r = self.start[i] as usize..self.start[i + 1] as usize;
        (&self.adj_link[r.clone()], &self.adj_cell[r])
    }
    /// The link between cells `i` and `j`, if they are neighbours.
    pub fn link(&self, i: usize, j: usize) -> Option<usize> {
        let (links, cells) = self.around(i);
        cells.iter().position(|&c| c as usize == j).map(|e| links[e] as usize)
    }
    /// Each cell's parent at the next coarser level (empty at the coarsest).
    pub fn parent(&self) -> &[u32] {
        &self.parent
    }
    /// The children of cell `i` at the next finer level (none at the finest).
    pub fn children(&self, i: usize) -> &[u32] {
        self.kid_start.get(i + 1).map_or(&[], |&e| &self.kids[self.kid_start[i] as usize..e as usize])
    }
}

/// A mesh: its levels, the finest first.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    levels: Vec<Level>,
}

impl Mesh {
    /// A mesh of one level.
    pub(crate) fn new(cells: Level) -> Mesh {
        Mesh { levels: vec![cells] }
    }
    pub fn levels(&self) -> usize {
        self.levels.len()
    }
    pub fn level(&self, l: usize) -> &Level {
        &self.levels[l]
    }
    /// The finest level.
    pub fn cells(&self) -> &Level {
        &self.levels[0]
    }
    /// The cell at the coarser level `to` that holds cell `i` of level `from`.
    pub fn up(&self, mut i: usize, from: usize, to: usize) -> usize {
        for l in from..to {
            i = self.levels[l].parent[i] as usize;
        }
        i
    }
    /// Add a coarser level over the coarsest, given the parent of each of its cells (parents numbered 0..m, each with
    /// at least one child). A coarse cell holds the area of its children, at their area-weighted centre and latitude;
    /// two coarse cells are linked where any of their children are, across the sum of those children's widths.
    pub(crate) fn coarsen(&mut self, parent: Vec<u32>) {
        let fine = self.levels.last_mut().expect("a mesh has a level");
        assert_eq!(parent.len(), fine.n(), "one parent for each cell");
        let m = parent.iter().max().map_or(0, |&p| p as usize + 1);
        let (kid_start, kids) = compress(m, parent.iter().enumerate().map(|(c, &p)| (p, c as u32)));
        assert!(kid_start.windows(2).all(|s| s[0] < s[1]), "each coarse cell has a child");
        let (mut area, mut centre, mut lat) = (vec![0.0; m], vec![[0.0; 2]; m], vec![0.0; m]);
        for (c, &p) in parent.iter().enumerate() {
            let (p, ac) = (p as usize, fine.area[c]);
            area[p] += ac;
            centre[p] = [centre[p][0] + ac * fine.centre[c][0], centre[p][1] + ac * fine.centre[c][1]];
            lat[p] += ac * fine.lat[c];
        }
        for p in 0..m {
            centre[p] = [centre[p][0] / area[p], centre[p][1] / area[p]];
            lat[p] /= area[p];
        }
        let mut pairs: Vec<(u32, u32, f64)> = (0..fine.a.len())
            .map(|l| (parent[fine.a[l] as usize], parent[fine.b[l] as usize], fine.width[l]))
            .filter(|&(pa, pb, _)| pa != pb)
            .map(|(pa, pb, wd)| (pa.min(pb), pa.max(pb), wd))
            .collect();
        pairs.sort_by_key(|&(pa, pb, _)| (pa, pb));
        let mut links: Vec<LinkSpec> = Vec::new();
        for (pa, pb, wd) in pairs {
            match links.last_mut() {
                Some(last) if (last.0, last.1) == (pa, pb) => last.3 += wd,
                _ => {
                    let (ca, cb) = (centre[pa as usize], centre[pb as usize]);
                    links.push((pa, pb, (ca[0] - cb[0]).hypot(ca[1] - cb[1]), wd));
                }
            }
        }
        fine.parent = parent;
        let mut coarse = Level::new(area, centre, lat, links);
        (coarse.kid_start, coarse.kids) = (kid_start, kids);
        self.levels.push(coarse);
    }
}
