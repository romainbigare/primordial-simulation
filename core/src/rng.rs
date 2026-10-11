//! Counter-based random numbers keyed by (seed, tick, node, purpose).
//! The same key always gives the same stream, so parts of the world can run in any order.

#[inline]
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Purposes keep independent streams apart.
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum Purpose {
    Terrain = 1,
    Weather,
    Change,
    Body,
    Mutate,
    Choose,
    Learn,
    Bond,
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64, tick: u64, node: u64, purpose: Purpose) -> Self {
        let k = mix(seed ^ mix(tick.wrapping_add(0x9e3779b97f4a7c15)))
            ^ mix(node.wrapping_mul(0xd1b54a32d192ed03) ^ (purpose as u64).wrapping_mul(0xa24baed4963ee407));
        Rng(mix(k))
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
        ((self.u64() >> 32) * n as u64 >> 32) as usize
    }
    /// Standard normal (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u = self.f64().max(1e-300);
        let v = self.f64();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
