//! The bodies of standing liquid (laws.md 1.2) and the terrain's network the runoff takes to them, found once for the
//! world's cells (`Bodies::of`). A body is a pond of the terrain's samples (`Samples::ponds`: the samples of a closed
//! basin under its pass, or a part of the open low ground), held by every cell that holds samples of it, each by its
//! part (`curve::Part`): a cell may hold land that drains and parts of one body or more. Its liquid stays in the ground
//! nodes of those cells. A cell sends what runs off down the samples' ways from each of its samples by its share of the
//! cell's area; a way reaches the body of the first sample of a pond on it, in the cell that holds that sample, and so
//! does the way down from a body's pass.

use super::curve::Curve;
use super::runoff::Carriage;
use crate::space::network::NONE;
use crate::space::samples::Samples;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;

/// A body of standing liquid: its pond (`Samples::ponds`); its cells in order, each with the index of its part in the
/// cell's curve (`Curve::parts`); the sample of its pass (`NONE` for a part of the open low ground) and the level of
/// the pass, m (infinite for none); where what leaves over the pass goes (`Reach`); and of a part of the open low ground,
/// which of its cells lie at the map's edge (their places in `cells`; none for a closed basin, or on a globe).
#[derive(Clone, Debug)]
pub(super) struct Body {
    pub(super) pond: u32,
    pub(super) cells: Vec<u32>,
    pub(super) part: Vec<usize>,
    pub(super) pass: u32,
    pub(super) level: f64,
    pub(super) next: Reach,
    pub(super) edge: Vec<usize>,
}

/// Where liquid that runs down the samples' ways from a sample goes: the body it reaches (`NONE` where the way ends in
/// no body), the cell it lands in: the cell that holds the first sample of a pond on the way (where the way ends in
/// no body, the cell of its last sample), and that cell's part of the body (`curve::Curve::ponds`; `NONE` for none).
#[derive(Clone, Copy, Debug)]
pub(super) struct Reach {
    pub(super) body: u32,
    pub(super) cell: u32,
    pub(super) part: u32,
}

/// Where no way goes.
const NOWHERE: Reach = Reach { body: NONE, cell: NONE, part: NONE };

/// The bodies of the world's cells and the ways to them: of each cell the body of its largest part (`NONE` for none);
/// the bodies, each before the one its pass leads to (`Body::next`); of each sample its cell, its share of the cell's
/// area and its body (`NONE` where it lies in no pond); of each cell, the cells and bodies its runoff reaches (`Reach`),
/// each with the share of the cell's area whose ways lead there, in order (its own cell among them, for the samples of
/// its own parts); the samples in no pond, in order; and of those whose liquid runs on, each before the one it runs to
/// (the last the flood reached first), the ways the runoff's pass sums down; and the key samples its load is followed
/// at (`Carriage`), found when the load first reads them (`carriage`: a settle carries no load).
#[derive(Clone, Debug, Default)]
pub(super) struct Bodies {
    pub(super) of: Vec<u32>,
    pub(super) list: Vec<Body>,
    pub(super) cell_of: Vec<u32>,
    pub(super) share: Vec<f64>,
    pub(super) at: Vec<u32>,
    pub(super) ways: Vec<Vec<(Reach, f64)>>,
    pub(super) land: Vec<u32>,
    pub(super) down: Vec<u32>,
    carriage: std::sync::OnceLock<Carriage>,
}

impl Bodies {
    /// The bodies of the cells of `w` on its terrain's samples, the cells' parts as `curves` hold them (`Bodies`).
    pub(super) fn of(w: &World, curves: &[Curve]) -> Bodies {
        let s = w.terrain_samples();
        let (leaves, lat) = (w.map.leaves(), (w.cfg.lat_south, w.cfg.lat_north));
        let n = leaves.len();
        let (mut cell_of, mut share) = (vec![NONE; s.h.len()], vec![0.0; s.h.len()]);
        for (i, l) in leaves.iter().enumerate() {
            let whole = w.mesh.cells().area()[i];
            for f in l.split(SAMPLE_DEPTH.max(l.depth)) {
                let k = s.index(&f);
                let area = if f.depth == SAMPLE_DEPTH { s.area(k) } else { w.map.place_of(&f, lat.0, lat.1).0 };
                (cell_of[k], share[k]) = (i as u32, area / whole);
            }
        }
        let mut list: Vec<Body> = s.ponds.iter().enumerate().map(|(p, pond)| {
            Body { pond: p as u32, cells: Vec::new(), part: Vec::new(), pass: pond.pass, level: pond.level, next: NOWHERE, edge: Vec::new() }
        }).collect();
        let mut of = vec![NONE; n];
        for (i, c) in curves.iter().enumerate() {
            for (q, part) in c.ponds().iter().enumerate() {
                let body = &mut list[part.pond as usize];
                body.cells.push(i as u32);
                body.part.push(q);
            }
            let most = c.ponds().iter().max_by(|a, b| a.under(f64::INFINITY).total_cmp(&b.under(f64::INFINITY)).then(b.pond.cmp(&a.pond)));
            of[i] = most.map_or(NONE, |p| p.pond);
        }
        let land = (0..s.h.len() as u32).filter(|&k| s.pond[k as usize] == NONE).collect();
        let down = s.order.iter().rev().copied().filter(|&k| s.pond[k as usize] == NONE && s.to[k as usize] != NONE).collect();
        let mut b = Bodies { of, list, cell_of, share, at: s.pond.clone(), ways: Vec::new(), land, down, carriage: Default::default() };
        let reach = b.reaches(&s, curves);
        b.ways = vec![Vec::new(); n];
        for (k, r) in reach.iter().enumerate() {
            let ways = &mut b.ways[b.cell_of[k] as usize];
            match ways.iter_mut().find(|x| (x.0.cell, x.0.part) == (r.cell, r.part)) {
                Some(x) => x.1 += b.share[k],
                None => ways.push((*r, b.share[k])),
            }
        }
        b.ways.iter_mut().for_each(|v| v.sort_by_key(|x| (x.0.cell, x.0.part)));
        let at_edge = |i: u32| {
            let [west, north, east, south] = w.map.bounds(i as usize);
            !w.map.wrap && (west <= 0.0 || north <= 0.0 || east >= w.map.w as f64 || south >= w.map.h as f64)
        };
        for body in b.list.iter_mut() {
            body.next = if body.pass == NONE { NOWHERE } else { reach[body.pass as usize] };
            if body.pass == NONE {
                body.edge = (0..body.cells.len()).filter(|&c| at_edge(body.cells[c])).collect();
            }
        }
        b.order();
        b
    }

    /// The key samples the load is followed at (`Carriage::of`), found the first time they are read: they are of the
    /// world's terrain's samples and its numbers, which stay till the bodies are found again.
    pub(super) fn carriage(&self, w: &World) -> &Carriage {
        self.carriage.get_or_init(|| Carriage::of(w, self))
    }

    /// Where liquid goes that runs down the samples' ways from each sample (`Reach`): from the first sample the flood
    /// reached on, each from the one its liquid runs to, so each is found once. A sample of a pond reaches its body in
    /// its own cell.
    fn reaches(&self, s: &Samples, curves: &[Curve]) -> Vec<Reach> {
        let mut reach = vec![NOWHERE; s.h.len()];
        for &k in &s.order {
            let k = k as usize;
            reach[k] = if self.at[k] != NONE || s.to[k] == NONE {
                let (body, cell) = (self.at[k], self.cell_of[k]);
                let c = &curves[cell as usize];
                let part = c.ponds().iter().position(|p| p.pond == body).map_or(NONE, |q| q as u32);
                Reach { body, cell, part }
            } else {
                reach[s.to[k] as usize]
            };
        }
        reach
    }

    /// The bodies in the order of their passes: each before the body its pass leads to (the most bodies down the way
    /// first; the ways down only fall, so they make no loop), renumbered.
    fn order(&mut self) {
        let m = self.list.len();
        let hops = |mut x: usize| {
            let mut h = 0;
            while self.list[x].next.body != NONE {
                (x, h) = (self.list[x].next.body as usize, h + 1);
            }
            h
        };
        let mut rank: Vec<usize> = (0..m).collect();
        let depth: Vec<usize> = (0..m).map(hops).collect();
        rank.sort_by(|&a, &b| depth[b].cmp(&depth[a]).then(a.cmp(&b)));
        let mut new = vec![0u32; m];
        rank.iter().enumerate().for_each(|(r, &x)| new[x] = r as u32);
        let renum = |x: u32| if x == NONE { NONE } else { new[x as usize] };
        let mut list: Vec<Body> = rank.iter().map(|&x| self.list[x].clone()).collect();
        list.iter_mut().for_each(|b| b.next.body = renum(b.next.body));
        self.list = list;
        self.of.iter_mut().for_each(|x| *x = renum(*x));
        self.at.iter_mut().for_each(|x| *x = renum(*x));
        self.ways.iter_mut().flatten().for_each(|x| x.0.body = renum(x.0.body));
    }
}
