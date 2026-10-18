//! Counter-based random numbers keyed by (seed, tick, node, purpose).
//! The same key always gives the same stream, so parts of the world can run in any order.

use crate::store::NodeId;

#[inline]
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Purposes keep independent streams apart: each place that draws has its own (the numbers are part of the streams: do
/// not renumber).
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum Purpose {
    Terrain = 1,
    /// A child's genes and its mate (`copy::copy`)
    Mutate = 5,
    Choose,
    Bond = 8,
    Chem,
    /// The gene a node splits along (`nodes::split`)
    Split,
    /// Whether a frozen individual dies (`body::live`)
    Frost,
    /// The order among equally alike mates (`copy::partner`)
    Partner,
    /// The born rows of a body joining a node, carried over to its brain (`nodes::merge`)
    Merge,
    /// Members drawn from a node as individuals (`nodes::open`)
    Open,
    /// The new weights of a resized brain (`lineage::resize`)
    Resize,
    /// The random base of a founder lineage's brain and where its founders go (`worldgen`)
    Brain,
    /// The mutation rounds of a founder's born rows and of its genes (`worldgen`)
    Spray,
    Make,
    /// Which founders carry the fixing reaction's gene (`worldgen`)
    Fixer,
    /// The side of each gene of the sample member of a node (`copy::select`)
    Select,
}

pub struct Rng(u64);

/// Tag of the stream keys by node id and seconds (`Rng::of_node`); it is never zero, so they differ from `Rng::new`.
const NODE_STREAM: u64 = 0x5851f42d4c957f2d;

impl Rng {
    fn key(seed: u64, tick: u64, node: u64, purpose: Purpose) -> u64 {
        mix(seed ^ mix(tick.wrapping_add(0x9e3779b97f4a7c15)))
            ^ mix(node.wrapping_mul(0xd1b54a32d192ed03) ^ (purpose as u64).wrapping_mul(0xa24baed4963ee407))
    }
    /// The stream of a purpose at a day and a node index.
    pub fn new(seed: u64, tick: u64, node: u64, purpose: Purpose) -> Self {
        Rng(mix(Self::key(seed, tick, node, purpose)))
    }
    /// The stream of a purpose for a node (slot and generation) at a time in seconds. The id packs into one `u64`
    /// without loss, so two ids or two times never give the same key. The tag is XORed into the key before the last
    /// mix, which is a bijection: the same inputs in `new` and here give different streams, and any other pair of
    /// keys meets only by chance of 2^-64 (as two `new` keys do).
    pub fn of_node(seed: u64, time: u64, id: NodeId, purpose: Purpose) -> Self {
        let node = (id.gen as u64) << 32 | id.slot as u64;
        Rng(mix(Self::key(seed, time, node, purpose) ^ NODE_STREAM))
    }
    #[inline]
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        mix(self.0)
    }
    /// Uniform in [0, 1).
    #[inline]
    pub fn f64(&mut self) -> f64 {
        (self.u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        (((self.u64() >> 32) * n as u64) >> 32) as usize
    }
    /// Standard normal (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u = self.f64().max(1e-300);
        let v = self.f64();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
