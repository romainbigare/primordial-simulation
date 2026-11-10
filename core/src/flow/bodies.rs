//! The bodies of standing liquid (laws.md 1.2) and the terrain's network the runoff takes to them, found once for the
//! world's cells (`Bodies::of`). A body is a pond of the terrain's samples (`Samples::ponds`: the samples of a closed
//! basin under its pass, or a part of the open low ground) held by the cells whose lowest sample lies in it: where a
//! cell's liquid leaves it, as its own ground fills from its lowest point up (`flow::curve`). Its liquid stays in the
//! ground nodes of those cells. A cell in no body sends what runs off down the samples' ways from each of its samples
//! by its share of the cell's area; the first sample of a body on a way names the body it reaches, and so does the
//! way down from a body's pass.

use crate::space::network::NONE;
use crate::space::samples::Samples;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;

/// A body of standing liquid: its pond (`Samples::ponds`), its cells in order, the sample of its pass (`NONE` for a
/// part of the open low ground) and the level of the pass, m (infinite for none); and where what leaves over the pass
/// goes (`Reach`).
#[derive(Clone, Debug)]
pub(super) struct Body {
    pub(super) pond: u32,
    pub(super) cells: Vec<u32>,
    pub(super) pass: u32,
    pub(super) level: f64,
    pub(super) next: Reach,
}

/// Where liquid that runs down the samples' ways from a sample goes: the body it reaches (`NONE` where the way ends in
/// no body) and the cell it lands in (a cell of that body: the one that holds the sample it reaches if that cell is the
/// body's, else the body's first; where the way ends in no body, the cell of its last sample).
#[derive(Clone, Copy, Debug)]
pub(super) struct Reach {
    pub(super) body: u32,
    pub(super) cell: u32,
}

/// The bodies of the world's cells and the ways to them: the body of each cell (`NONE` for none); the bodies, each
/// before the one its pass leads to (`Body::next`); of each sample its cell, its share of the cell's area, its body
/// (`NONE` where its pond has no cell, or none) and the cell its way lands in (`Reach`); and of each cell in no body,
/// the other cells its runoff lands in, each with the share of the cell's area whose ways lead there, in order.
#[derive(Clone, Debug, Default)]
pub(super) struct Bodies {
    pub(super) of: Vec<u32>,
    pub(super) list: Vec<Body>,
    pub(super) cell_of: Vec<u32>,
    pub(super) share: Vec<f64>,
    pub(super) at: Vec<u32>,
    pub(super) lands: Vec<u32>,
    pub(super) ways: Vec<Vec<(u32, f64)>>,
}

impl Bodies {
    /// The bodies of the cells of `w` on its terrain's samples (`Bodies`).
    pub(super) fn of(w: &World) -> Bodies {
        let s = w.terrain_samples();
        let (leaves, lat) = (w.map.leaves(), (w.cfg.lat_south, w.cfg.lat_north));
        let n = leaves.len();
        let (mut cell_of, mut share, mut low) = (vec![NONE; s.h.len()], vec![0.0; s.h.len()], vec![NONE; n]);
        for (i, l) in leaves.iter().enumerate() {
            let whole = w.mesh.cells().area()[i];
            for f in l.split(SAMPLE_DEPTH.max(l.depth)) {
                let k = s.index(&f);
                (cell_of[k], share[k]) = (i as u32, w.map.place_of(&f, lat.0, lat.1).0 / whole);
                if low[i] == NONE || s.h[k] < s.h[low[i] as usize] {
                    low[i] = k as u32;
                }
            }
        }
        // The body of each pond that a cell holds: its pond the pond of the cell's lowest sample.
        let mut by_pond = vec![NONE; s.ponds.len()];
        let mut of = vec![NONE; n];
        let mut list: Vec<Body> = Vec::new();
        for i in 0..n {
            let p = s.pond[low[i] as usize];
            if p == NONE {
                continue;
            }
            if by_pond[p as usize] == NONE {
                by_pond[p as usize] = list.len() as u32;
                let pond = s.ponds[p as usize];
                list.push(Body { pond: p, cells: Vec::new(), pass: pond.pass, level: pond.level, next: Reach { body: NONE, cell: NONE } });
            }
            of[i] = by_pond[p as usize];
            list[of[i] as usize].cells.push(i as u32);
        }
        let at: Vec<u32> = s.pond.iter().map(|&p| if p == NONE { NONE } else { by_pond[p as usize] }).collect();
        let mut b = Bodies { of, list, cell_of, share, at, lands: Vec::new(), ways: Vec::new() };
        let reach = b.reaches(&s);
        b.ways = vec![Vec::new(); n];
        for (k, r) in reach.iter().enumerate() {
            let i = b.cell_of[k] as usize;
            if b.of[i] == NONE && r.cell as usize != i {
                match b.ways[i].iter_mut().find(|x| x.0 == r.cell) {
                    Some(x) => x.1 += b.share[k],
                    None => b.ways[i].push((r.cell, b.share[k])),
                }
            }
        }
        b.ways.iter_mut().for_each(|v| v.sort_by_key(|x| x.0));
        b.lands = reach.iter().map(|r| r.cell).collect();
        for x in 0..b.list.len() {
            let pass = b.list[x].pass;
            b.list[x].next = if pass == NONE { Reach { body: NONE, cell: NONE } } else { reach[pass as usize] };
        }
        b.order();
        b
    }

    /// Where liquid goes that runs down the samples' ways from each sample (`Reach`): from the first sample the flood
    /// reached on, each from the one its liquid runs to, so each is found once. A sample of a body reaches its body.
    fn reaches(&self, s: &Samples) -> Vec<Reach> {
        let mut reach = vec![Reach { body: NONE, cell: NONE }; s.h.len()];
        for &k in &s.order {
            let k = k as usize;
            let x = self.at[k];
            reach[k] = if x != NONE {
                let c = self.cell_of[k];
                Reach { body: x, cell: if self.of[c as usize] == x { c } else { self.list[x as usize].cells[0] } }
            } else if s.to[k] == NONE {
                Reach { body: NONE, cell: self.cell_of[k] }
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
    }
}
