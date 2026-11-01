//! What a snapshot keeps of `Life` (`Saved`) and how it is written and read back.

use super::{Genome, Life, ID_SHIFT, RING};
use crate::body::Body;
use crate::lineage::Lineage;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// What a snapshot keeps of `Life`: per domain its cells, its count of bodies placed, what moved into its cells and its
/// living bodies in the order of `Life::alive`; the lineages and born rows. Where bodies are stored, the ring and the id
/// table follow from the bodies, and the survey's caches are rebuilt (`Life::sum`). Written from the world's own
/// (borrowed), read back as new (owned).
#[derive(Serialize, Deserialize)]
struct Saved<'a> {
    domains: Vec<SavedDomain<'a>>,
    brains: Cow<'a, [Option<Lineage>]>,
    members: Cow<'a, [u32]>,
    genomes: Cow<'a, [Genome]>,
    free_genomes: Cow<'a, [u32]>,
}

/// A domain as a snapshot keeps it (`Saved`): its cells, its count of bodies placed, per cell the energy movers spent
/// pushing through its bodies since the last survey and the members that moved into it since the last month, its
/// cover and its living bodies.
#[derive(Serialize, Deserialize)]
struct SavedDomain<'a> {
    cells: Cow<'a, [u32]>,
    placed: u64,
    pushed: Cow<'a, [f64]>,
    crossings: Cow<'a, [f32]>,
    cover: Cow<'a, crate::cover::Cover>,
    bodies: Vec<Cow<'a, Body>>,
}

impl Serialize for Life {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        let mut domains = Vec::with_capacity(self.domains.len());
        for d in &self.domains {
            let bodies = d.at.iter().flatten().map(|&h| Cow::Borrowed(&self[h])).filter(|b| b.alive).collect();
            domains.push(SavedDomain { cells: (&d.cells).into(), placed: d.placed, pushed: (&d.pushed).into(), crossings: (&d.crossings).into(), cover: Cow::Borrowed(&d.cover), bodies });
        }
        let (brains, members) = ((&self.brains).into(), (&self.members).into());
        Saved { domains, brains, members, genomes: (&self.genomes).into(), free_genomes: (&self.free_genomes).into() }.serialize(to)
    }
}

/// A loaded world stores its bodies anew, in the order saved; the caches come back empty.
impl<'de> Deserialize<'de> for Life {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let x = Saved::deserialize(from)?;
        let (brains, members) = (x.brains.into_owned(), x.members.into_owned());
        let mut life = Life { brains, members, genomes: x.genomes.into_owned(), free_genomes: x.free_genomes.into_owned(), ..Default::default() };
        life.cut(x.domains.iter().map(|d| d.cells.to_vec()).collect());
        for (d, saved) in x.domains.into_iter().enumerate() {
            let SavedDomain { placed, pushed, crossings, cover, bodies, .. } = saved;
            let dom = &mut life.domains[d];
            (dom.placed, dom.pushed, dom.crossings, dom.cover) = (placed, pushed.into_owned(), crossings.into_owned(), cover.into_owned());
            for b in bodies.into_iter().map(Cow::into_owned) {
                if (b.id >> ID_SHIFT) as usize != d {
                    life.away.insert(b.id, d as u32);
                }
                life.domains[d].ring[(b.due % RING) as usize].push(b.id);
                life.domains[d].store(b);
            }
        }
        Ok(life)
    }
}
