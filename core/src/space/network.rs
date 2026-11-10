//! The terrain's channels (laws.md 0): each sample's liquid runs where the flood over the samples sends it
//! (`space::drain`): down the steepest fall of the ground where it drains, inside a closed basin toward its pass. The
//! area that drains through a sample is its own and all that runs to it. It is a function of the seed and the map
//! alone.


/// No sample: where a sample's liquid runs when it runs to none on the map (a sink), and the sample upstream of one that
/// none runs to.
pub const NONE: u32 = u32::MAX;

/// The area that drains through each sample, m², and its neighbour upstream that the most drains through (`NONE`
/// where none runs to it), for samples whose liquid runs `to` (`NONE` for a sink) of areas `area`, m², in an `order`
/// where each comes after the one its liquid runs to: the area accumulated from the last sample of the order to the
/// first.
pub(super) fn accumulate(to: &[u32], order: &[u32], area: impl Fn(usize) -> f64) -> (Vec<f64>, Vec<u32>) {
    let n = to.len();
    let mut up: Vec<f64> = (0..n).map(area).collect();
    let mut main = vec![NONE; n];
    for &k in order.iter().rev() {
        let (k, j) = (k as usize, to[k as usize]);
        if j != NONE {
            let j = j as usize;
            up[j] += up[k];
            if main[j] == NONE || up[k] > up[main[j] as usize] {
                main[j] = k as u32;
            }
        }
    }
    (up, main)
}
