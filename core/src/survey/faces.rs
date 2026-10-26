//! What the bodies of a domain show one another in a batch (laws.md 0): their faces, made for every domain at the
//! batch's start.

use super::capture;
use crate::{brain::SIG, life::Domain, World};

/// What a body shows others in a batch, as it was at the batch's start (laws.md 0): what they perceive of it.
#[derive(Clone, Copy, Debug, Default)]
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

/// The faces of the living bodies of one domain: of its `k`th cell, `all[at[k]..at[k + 1]]`, in the order they came.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    all: Vec<Face>,
    at: Vec<u32>,
}

impl Faces {
    /// The faces of the bodies of the domain's `k`th cell.
    pub fn of(&self, k: usize) -> &[Face] {
        self.at.get(k + 1).map_or(&[], |&e| &self.all[self.at[k] as usize..e as usize])
    }
    /// The faces of the living bodies of `d` as they are now, cell by cell, under the canopy of the survey's last
    /// (`World::canopy`), in the room these faces held.
    fn make(&mut self, w: &World, d: &Domain) {
        self.all.clear();
        self.at.clear();
        self.at.push(0);
        let area = w.mesh.cells().area();
        for (k, &i) in d.cells.iter().enumerate() {
            let (canopy, a) = (w.canopy[i as usize] as f64, area[i as usize]);
            for b in d.at[k].iter().map(|&h| &d[h]).filter(|b| b.alive) {
                let room = crate::bond::room(w, b).map(|(r, e)| (r as u32, e.closed));
                let cover = (canopy - capture(b.vis[1], b.n()) / a).max(0.0);
                let show = if cover <= 0.0 { UNCOVERED } else { (-cover).exp() };
                self.all.push(Face { id: b.id, n: b.n(), vis: b.vis, sig: b.sig, wb: b.wb, room, show });
            }
            self.at.push(self.all.len() as u32);
        }
    }
}

/// At a batch's start: the faces of every domain, one after another on the calling thread (a domain's take a few
/// microseconds, less than a call to the pool).
pub(super) fn make(w: &mut World) {
    let mut faces = std::mem::take(&mut w.life.faces);
    faces.iter_mut().zip(&w.life.domains).for_each(|(f, d)| f.make(w, d));
    w.life.faces = faces;
}
