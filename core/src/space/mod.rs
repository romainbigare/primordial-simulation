//! Space: the shape of the world, and nothing else. A mesh is levels of cells (area, centre, latitude, longitude) and
//! links (two cells, the distance between centres, the width of their border); each cell but at the coarsest level has
//! a parent. A level lies on a flat map or on a sphere. Each link of a cell takes one of its compass directions (`way`).
//! Links are plain arrays, each once (`a < b`), with each cell's links in compressed lists, so solvers take slices.

pub mod square;
pub mod stack;

pub use stack::Stack;

use std::f64::consts::TAU;

/// Read-only views of array fields of the same names, `name: element type` (the shape is built once and then only read).
macro_rules! views {
    ($($(#[$doc:meta])* $field:ident: $t:ty),* $(,)?) => {
        $($(#[$doc])* pub fn $field(&self) -> &[$t] {
            &self.$field
        })*
    };
}
use views;

/// One level of a mesh: its cells, its links, and how it nests in the levels next to it.
#[derive(Clone, Debug, Default)]
pub struct Level {
    /// Area of each cell, m²; its centre on the map (east, south), m; its latitude and longitude, degrees
    area: Vec<f64>,
    centre: Vec<[f64; 2]>,
    lat: Vec<f64>,
    lon: Vec<f64>,
    /// The radius of the sphere the level lies on, m (0: a flat map, where the centres place the cells)
    radius: f64,
    /// The two cells of each link, the distance between their centres, m, and their border's width, m (0 at a corner)
    a: Vec<u32>,
    b: Vec<u32>,
    dist: Vec<f64>,
    width: Vec<f64>,
    /// The links of cell `i` are entries `start[i]..start[i + 1]`, in link order: each entry's link and other cell
    start: Vec<u32>,
    adj_link: Vec<u32>,
    adj_cell: Vec<u32>,
    /// The heading of each entry's link from its cell, a unit vector (east, south) along `offset`, and each cell's entry
    /// in each compass direction (`NONE` where it has none)
    heading: Vec<[f64; 2]>,
    compass: Vec<[u32; COMPASS]>,
    /// Each cell's parent at the next coarser level (empty at the coarsest)
    parent: Vec<u32>,
    /// The children of cell `i` at the next finer level are `kids[kid_start[i]..kid_start[i + 1]]` (empty at the finest)
    kid_start: Vec<u32>,
    kids: Vec<u32>,
}

/// The degrees of longitude east from `from` to `to`, the shorter way round.
fn east_of(from: f64, to: f64) -> f64 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

/// The compass directions of a cell's links, clockwise from east in eighths of a turn; and no link in a direction.
pub const COMPASS: usize = 8;
const NONE: u32 = u32::MAX;

/// A link while a level is built: the two cells (`a < b`), the distance and the width, m.
pub type LinkSpec = (u32, u32, f64, f64);

/// Compressed lists of the members of each group: `start` (len `groups + 1`) and the members in the order given.
fn compress(groups: usize, members: impl Iterator<Item = (u32, u32)> + Clone) -> (Vec<u32>, Vec<u32>) {
    let mut start = vec![0u32; groups + 1];
    members.clone().for_each(|(g, _)| start[g as usize + 1] += 1);
    (0..groups).for_each(|g| start[g + 1] += start[g]);
    let (mut next, mut list) = (start.clone(), vec![0u32; start[groups] as usize]);
    for (g, m) in members {
        list[next[g as usize] as usize] = m;
        next[g as usize] += 1;
    }
    (start, list)
}

impl Level {
    /// A level of cells on a sphere of `radius`, m (0: a flat map), with no links yet.
    pub(crate) fn of_cells(area: Vec<f64>, centre: Vec<[f64; 2]>, (lat, lon): (Vec<f64>, Vec<f64>), radius: f64) -> Level {
        let n = area.len();
        assert!(centre.len() == n && lat.len() == n && lon.len() == n, "a level needs a centre, a latitude and a longitude for each cell");
        Level { area, centre, lat, lon, radius, start: vec![0; n + 1], ..Default::default() }
    }
    /// Join the cells by `links`, each once (`a < b`).
    pub(crate) fn join(&mut self, links: Vec<LinkSpec>) {
        let n = self.n();
        assert!(links.iter().all(|&(a, b, ..)| a < b && (b as usize) < n), "each link joins two cells a < b of the level");
        let ends = links.iter().enumerate().flat_map(|(l, &(a, b, ..))| [(a, (l as u32, b)), (b, (l as u32, a))]);
        (self.start, self.adj_link) = compress(n, ends.clone().map(|(c, (l, _))| (c, l)));
        self.adj_cell = compress(n, ends.map(|(c, (_, o))| (c, o))).1;
        ((self.a, self.b), (self.dist, self.width)) = links.into_iter().map(|(a, b, dist, width)| ((a, b), (dist, width))).unzip();
        let ends = (0..n).flat_map(|i| self.around(i).1.iter().map(move |&j| (i, j as usize)));
        self.heading = ends.map(|(i, j)| self.offset(i, j)).map(|o| [o[0] / o[0].hypot(o[1]), o[1] / o[0].hypot(o[1])]).collect();
        self.compass = (0..n).map(|i| self.points(i)).collect();
    }
    /// The entries of cell `i` in the compass directions: each link takes the free direction nearest its heading, the
    /// links nearest a direction first, so a square's 8 neighbours take the 8 directions even where a globe's rows
    /// narrow toward a pole.
    fn points(&self, i: usize) -> [u32; COMPASS] {
        let (r, eighths) = (self.start[i] as usize..self.start[i + 1] as usize, |e: usize| self.heading[e][1].atan2(self.heading[e][0]) / (TAU / COMPASS as f64));
        assert!(r.len() <= COMPASS, "a cell has at most one link in each compass direction");
        // How far bearing `x` lies from direction `d`, in eighths of a turn either way.
        let off = |x: f64, d: usize| (x - d as f64).rem_euclid(COMPASS as f64).min((d as f64 - x).rem_euclid(COMPASS as f64));
        let mut order: Vec<usize> = r.collect();
        order.sort_by(|&p, &q| (eighths(p) - eighths(p).round()).abs().total_cmp(&(eighths(q) - eighths(q).round()).abs()));
        let mut points = [NONE; COMPASS];
        for e in order {
            let d = (0..COMPASS).filter(|&d| points[d] == NONE).min_by(|&p, &q| off(eighths(e), p).total_cmp(&off(eighths(e), q)));
            points[d.expect("a free direction")] = e as u32;
        }
        points
    }
    pub fn n(&self) -> usize {
        self.area.len()
    }
    views! {
        area: f64, centre: [f64; 2], lat: f64, lon: f64, a: u32, b: u32, dist: f64, width: f64,
        /// The compressed adjacency: `start` (len n + 1), and for each entry its link and its other cell.
        start: u32, adj_link: u32, adj_cell: u32, heading: [f64; 2],
        /// Each cell's parent at the next coarser level (empty at the coarsest).
        parent: u32,
    }
    /// The way from the centre of cell `i` to that of cell `j`, (east, south), m: on a sphere along its parallel at
    /// their mean latitude and its meridian, the shorter way round.
    pub fn offset(&self, i: usize, j: usize) -> [f64; 2] {
        if self.radius == 0.0 {
            return [self.centre[j][0] - self.centre[i][0], self.centre[j][1] - self.centre[i][1]];
        }
        let east = east_of(self.lon[i], self.lon[j]);
        let mean = ((self.lat[i] + self.lat[j]) / 2.0).to_radians();
        [self.radius * east.to_radians() * mean.cos(), self.radius * (self.lat[i] - self.lat[j]).to_radians()]
    }
    /// The entry of cell `i`'s link in compass direction `d` (0 east, clockwise in eighths of a turn), if it has one.
    pub fn way(&self, i: usize, d: usize) -> Option<usize> {
        Some(self.compass[i][d]).filter(|&e| e != NONE).map(|e| e as usize)
    }
    /// The steepest way down from cell `i` (D-infinity, Tarboton 1997), given the drop to the cell at the end of each
    /// of its links, m, by its place in `around(i)` (`None`: no way down that link). Each two links next in compass
    /// order that turn by less than half a turn make a triangle with the centre (where the centres lie, `offset`, not
    /// the links' distance across a border); the flat surface through their drops gives its steepest way down, shared
    /// between its two links by angle, or, where that way leaves the triangle, its edges do. Returns the steepest slope
    /// and its one or two links by their place, each with its share (`None`: no drop).
    pub fn steepest(&self, i: usize, drop: impl Fn(usize) -> Option<f64>) -> Option<(f64, [(usize, f64); 2])> {
        let s0 = self.start[i] as usize;
        let ring: Vec<usize> = self.compass[i].iter().filter(|&&e| e != NONE).map(|&e| e as usize - s0).collect();
        let to = |k: usize| self.offset(i, self.adj_cell[s0 + k] as usize);
        let cross = |u: [f64; 2], v: [f64; 2]| u[0] * v[1] - u[1] * v[0];
        let dot = |u: [f64; 2], v: [f64; 2]| u[0] * v[0] + u[1] * v[1];
        let (mut top, mut best) = (0.0, None);
        let mut keep = |slope: f64, way: [(usize, f64); 2]| if slope > top { (top, best) = (slope, Some(way)) };
        for (&k1, &k2) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let (Some(s1), u, v) = (drop(k1), to(k1), to(k2)) else { continue };
            keep(s1 / u[0].hypot(u[1]), [(k1, 1.0), (k1, 0.0)]);
            // The way down `g` with g·u = s1 and g·v = s2, inside the triangle where it lies between u and v.
            let (Some(s2), det) = (drop(k2), cross(u, v)) else { continue };
            let g = [(s1 * v[1] - s2 * u[1]) / det, (u[0] * s2 - v[0] * s1) / det];
            if det > 0.0 && cross(g, v) >= 0.0 && cross(u, g) >= 0.0 {
                let (a, b) = (cross(u, g).atan2(dot(u, g)), cross(g, v).atan2(dot(g, v)));
                keep(g[0].hypot(g[1]), [(k1, b / (a + b)), (k2, a / (a + b))]);
            }
        }
        best.map(|b| (top, b))
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
    pub(crate) fn of(cells: Level) -> Mesh {
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
    /// The share of its cell at level `to` that cell `i` of the finest level covers: their areas' ratio.
    pub fn share(&self, i: usize, to: usize) -> f64 {
        self.cells().area[i] / self.levels[to].area[self.up(i, 0, to)]
    }
    /// The way on the map from cell `i` of level `from` to cell `j` of level `to`, (east, south), m: between their
    /// centres on a flat map; on a sphere, on its map of latitude and longitude at the scale of the equator (the shorter
    /// way round), where reading a coarser level bilinearly is linear in both.
    pub fn on_map(&self, (from, i): (usize, usize), (to, j): (usize, usize)) -> [f64; 2] {
        let (p, q) = (&self.levels[from], &self.levels[to]);
        if p.radius == 0.0 {
            return [q.centre[j][0] - p.centre[i][0], q.centre[j][1] - p.centre[i][1]];
        }
        [p.radius * east_of(p.lon[i], q.lon[j]).to_radians(), p.radius * (p.lat[i] - q.lat[j]).to_radians()]
    }
    /// Add a coarser level over the coarsest, given each cell's parent (`Level::coarser`).
    pub(crate) fn coarsen(&mut self, parent: Vec<u32>) {
        let fine = self.levels.last_mut().expect("a mesh has a level");
        let (mut coarse, links) = fine.coarser(&parent);
        fine.parent = parent;
        coarse.join(links);
        self.levels.push(coarse);
    }
}

impl Level {
    /// The cells of a coarser level over this one and their links, given each cell's parent (0..m, each with a child; on
    /// a sphere, children that do not straddle the longitude 0). A coarse cell holds its children's area at their
    /// area-weighted centre; coarse cells link where children do, across the sum of their widths.
    pub(crate) fn coarser(&self, parent: &[u32]) -> (Level, Vec<LinkSpec>) {
        let fine = self;
        assert_eq!(parent.len(), fine.n(), "one parent for each cell");
        let m = parent.iter().max().map_or(0, |&p| p as usize + 1);
        let (kid_start, kids) = compress(m, parent.iter().enumerate().map(|(c, &p)| (p, c as u32)));
        assert!(kid_start.windows(2).all(|s| s[0] < s[1]), "each coarse cell has a child");
        let (mut area, mut centre, mut lat, mut lon) = (vec![0.0; m], vec![[0.0; 2]; m], vec![0.0; m], vec![0.0; m]);
        for (c, &p) in parent.iter().enumerate() {
            let (p, ac) = (p as usize, fine.area[c]);
            area[p] += ac;
            centre[p] = [centre[p][0] + ac * fine.centre[c][0], centre[p][1] + ac * fine.centre[c][1]];
            lat[p] += ac * fine.lat[c];
            lon[p] += ac * fine.lon[c];
        }
        for p in 0..m {
            centre[p] = [centre[p][0] / area[p], centre[p][1] / area[p]];
            lat[p] /= area[p];
            lon[p] /= area[p];
        }
        let mut coarse = Level::of_cells(area, centre, (lat, lon), fine.radius);
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
                    let way = coarse.offset(pa as usize, pb as usize);
                    links.push((pa, pb, way[0].hypot(way[1]), wd));
                }
            }
        }
        (coarse.kid_start, coarse.kids) = (kid_start, kids);
        (coarse, links)
    }
}
