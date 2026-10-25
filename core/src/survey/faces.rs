//! What the bodies of a domain show one another in a batch (laws.md 0): their faces. A domain keeps its faces from the
//! end of its own task in a batch to the start of the next, as long as nothing they show changed; only the domains
//! that changed since are made again, before the batch's updates.

use super::{canopy_of, capture};
use crate::{brain::SIG, life::Domain, par, World};

/// What a body shows others in a batch, as it was at the batch's start (laws.md 0): what they perceive of it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Face {
    pub id: u64,
    /// Its members (`Body::n`), visible traits (`Body::vis`), signal and well-being at its last decision
    pub n: f64,
    pub vis: [f32; 7],
    pub sig: [f32; SIG],
    pub wb: f32,
    /// The root of the structure it is inside and how closed its walls are (`bond::room`)
    pub room: Option<(u32, f32)>,
    /// The chance that the cover of the others over it leaves it seen, exp(-a) with a their capture area per m²
    /// (`UNCOVERED` under no cover)
    pub show: f64,
}

/// The `Face::show` of a face under no cover: above any chance, so that nothing is drawn for it.
pub const UNCOVERED: f64 = 2.0;

/// The faces of the living bodies of one domain: of its `k`th cell, `all[at[k]..at[k + 1]]`, in the order they came,
/// and the canopy of each cell that their `show` was worked out with.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    all: Vec<Face>,
    at: Vec<u32>,
    canopy: Vec<f32>,
}

impl Faces {
    /// The faces of the bodies of the domain's `k`th cell.
    pub fn of(&self, k: usize) -> &[Face] {
        self.at.get(k + 1).map_or(&[], |&e| &self.all[self.at[k] as usize..e as usize])
    }
    /// Whether these faces are what `d` would show now: nothing they show was written since they were made, and the
    /// canopy of each cell is the one they were made with.
    fn holds(&self, w: &World, d: &Domain) -> bool {
        !d.touched && self.canopy.len() == d.cells.len() && d.cells.iter().zip(&self.canopy).all(|(&i, &c)| w.canopy[i as usize] == c)
    }
    /// No faces.
    pub(crate) fn clear(&mut self) {
        self.all.clear();
        self.at.clear();
        self.canopy.clear();
    }
    /// Made: there are faces.
    pub(crate) fn made(&self) -> bool {
        !self.at.is_empty()
    }
    /// The faces of the living bodies of `d` as they are now, cell by cell, in the room these faces held. The canopy of
    /// a cell is the survey's last (`World::canopy`), or with `next` the one the next survey works out from these
    /// bodies if none changes before it. Returns whether a body is inside a structure: its face then depends on the
    /// parts, which change outside the domain.
    fn make(&mut self, w: &World, d: &Domain, next: bool) -> bool {
        self.clear();
        self.at.push(0);
        let (area, mut inside) = (w.mesh.cells().area(), false);
        for (k, &i) in d.cells.iter().enumerate() {
            let (from, a) = (self.all.len(), area[i as usize]);
            for b in d.at[k].iter().map(|&h| &d[h]).filter(|b| b.alive) {
                let room = crate::bond::room(w, b).map(|(r, e)| (r as u32, e.closed));
                inside |= b.inside.is_some();
                self.all.push(Face { id: b.id, n: b.n(), vis: b.vis, sig: b.sig, wb: b.wb, room, show: UNCOVERED });
            }
            let canopy = if next { canopy_of(self.all[from..].iter().fold(0.0, |s, f| s + capture(f.vis[1], f.n)), a) } else { w.canopy[i as usize] };
            self.canopy.push(canopy);
            for f in &mut self.all[from..] {
                let cover = (canopy as f64 - capture(f.vis[1], f.n) / a).max(0.0);
                f.show = if cover <= 0.0 { UNCOVERED } else { (-cover).exp() };
            }
            self.at.push(self.all.len() as u32);
        }
        inside
    }
}

impl Domain {
    /// Make the faces its bodies show in the next batch, from them as they are now (the end of its task in a batch).
    pub(crate) fn make_next_faces(&mut self, w: &World) {
        let mut faces = std::mem::take(&mut self.made);
        self.touched = faces.make(w, self, true);
        self.made = faces;
    }
}

/// Before a batch: the domains whose faces no longer show them are made again, on the pool.
pub(super) fn remake(w: &mut World) {
    let (mut domains, mut faces) = (std::mem::take(&mut w.life.domains), std::mem::take(&mut w.life.faces));
    let mut stale: Vec<_> = faces.iter_mut().zip(&mut domains).filter(|(f, d)| !f.holds(w, d)).collect();
    let work = stale.iter().map(|(_, d)| d.at.iter().map(Vec::len).sum::<usize>()).sum();
    par::each_init(&mut stale, work, || (), |_, (f, d)| d.touched = f.make(w, d, false));
    drop(stale);
    debug_assert!(
        faces.iter().zip(&domains).all(|(f, d)| {
            let mut now = Faces::default();
            now.make(w, d, false);
            (now.all == f.all) && now.at == f.at
        }),
        "the faces kept are not what the domains show"
    );
    (w.life.domains, w.life.faces) = (domains, faces);
}

/// After a batch: the faces each domain made at the end of its task are the ones it keeps for the next batch.
pub(super) fn keep(w: &mut World) {
    for (f, d) in w.life.faces.iter_mut().zip(&mut w.life.domains).filter(|(_, d)| d.made.made()) {
        std::mem::swap(f, &mut d.made);
        d.made.clear();
    }
}
