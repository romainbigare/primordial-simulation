//! What the bodies show one another in a batch (laws.md 0): their faces, made for every cell at the batch's start.

use crate::{brain::SIG, World};

/// What a body shows others in a batch, as it was at the batch's start (laws.md 0): what they perceive of it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Face {
    pub id: u64,
    /// Its place in its cell's list (`life::Handle::slot`), which holds through the batch
    pub slot: u32,
    /// Its members (`Body::n`), visible traits (`Body::vis`), signal and well-being at its last decision
    pub n: f64,
    pub vis: [f32; 7],
    pub sig: [f32; SIG],
    pub wb: f32,
    /// The root of the structure it is inside and how closed its walls are (`bond::room`)
    pub room: Option<(u64, f32)>,
    /// The chance that the cover of its cell leaves it seen, exp(-a) with a its capture area per m² (`UNCOVERED` under
    /// no cover)
    pub show: f64,
}

/// The `Face::show` of a face under no cover: above any chance, so that nothing is drawn for it.
pub const UNCOVERED: f64 = 2.0;

/// The faces of the living bodies: of cell `i`, `all[at[i]..at[i + 1]]`, in the order they came.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    all: Vec<Face>,
    at: Vec<u32>,
}

impl Faces {
    /// The faces of the bodies of cell `i`.
    pub fn of(&self, i: usize) -> &[Face] {
        self.at.get(i + 1).map_or(&[], |&e| &self.all[self.at[i] as usize..e as usize])
    }
}

/// At a batch's start: the faces of the living bodies as they are now, cell by cell, under the cover's canopy of the
/// survey's last (`World::canopy`), in the room these faces held; on the calling thread (a cell's take well under a
/// microsecond).
pub(super) fn make(w: &mut World) {
    let mut f = std::mem::take(&mut w.life.faces);
    f.all.clear();
    f.at.clear();
    f.at.push(0);
    for (i, c) in w.life.cells.iter().enumerate() {
        let cover = w.canopy[i] as f64;
        let show = if cover <= 0.0 { UNCOVERED } else { (-cover).exp() };
        for (slot, b) in c.bodies.iter().enumerate().filter(|x| x.1.alive) {
            let room = crate::bond::room(c, b).map(|(r, e)| (r, e.closed));
            f.all.push(Face { id: b.id, slot: slot as u32, n: b.n(), vis: b.vis, sig: b.sig, wb: b.wb, room, show });
        }
        f.at.push(f.all.len() as u32);
    }
    w.life.faces = f;
}
