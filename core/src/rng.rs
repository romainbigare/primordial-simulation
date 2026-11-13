//! Counter-based random numbers keyed by (seed, tick, node, purpose).
//! The same key always gives the same stream, so parts of the world can run in any order.


/// A bijection of 64 bits that spreads every input bit over the output (the finaliser of SplitMix64, Steele et al.
/// 2014).
#[inline]
pub(crate) fn mix(mut z: u64) -> u64 {
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
    Chem = 9,
    /// Whether an individual dies of a share that kills members (`body::live::lose_share`)
    Death,
    /// The order among equally alike mates (`copy::partner`)
    Partner,
    /// The new weights of a resized brain (`lineage::resize`)
    Resize = 15,
    /// The random base of a founder lineage's brain and where its founders go (`worldgen`)
    Brain,
    /// The mutation rounds of a founder's born rows and of its genes (`worldgen`)
    Spray,
    Make,
    /// Where a slot of cover sends its seeds, and their mutation (`cover::grow`)
    Cover,
    /// The quarter a body or a structure goes to whole when its cell splits (`life::tree`)
    Quarter,
    /// The genotypes a nursery draws and where its champions go (`worldgen::life::nursery`)
    Nursery,
}

pub struct Rng(u64);

impl Rng {
    fn key(seed: u64, tick: u64, node: u64, purpose: Purpose) -> u64 {
        mix(seed ^ mix(tick.wrapping_add(0x9e3779b97f4a7c15)))
            ^ mix(node.wrapping_mul(0xd1b54a32d192ed03) ^ (purpose as u64).wrapping_mul(0xa24baed4963ee407))
    }
    /// The stream of a purpose at a day and a node index.
    pub fn new(seed: u64, tick: u64, node: u64, purpose: Purpose) -> Self {
        Rng(mix(Self::key(seed, tick, node, purpose)))
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
        let (u, v) = (self.f64().max(1e-300), self.f64());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
