//! The air's rise over the ground (laws.md 1.5): the low wind over each air node (`low_wind`), how much air it lifts or
//! lets sink where it climbs or descends the ground (`rise`), and what that does to the vapour the rain law finds
//! saturating a node (`saturates`). Air that rises carries its lower half's vapour up, where the cold air holds less of
//! it, and what it cannot hold condenses; air that sinks brings the upper half's dry air down, and the lower air holds
//! less than it would (the linear theory of rain over the ground, Smith and Barstad 2004). The rain law reads the rise
//! only here.

use super::super::Scratch;
use super::cell::Cell;
use super::daily::blocks;
use super::wind::{ends, gradient};
use crate::{World, AIR};

/// Each air node's low wind, m/s [east, south]: the overturning's lower half as the warmth's gradient drives it
/// (`wind::push`), `wind` times the gradient, down it inside the month's cell (`Cell::share`) and turned by the spin
/// across it. The gradient is the planet's at the node's latitude and time (`Planet::at`) where the map's edges are open;
/// else the air's own (`wind::ends`; its metres are the planet's there) where a node is a block alone (`daily::blocks`),
/// as the overturning runs between blocks; a node that shares its block, whose air the eddies even each day, has no wind
/// of its own. The air as the step starts.
pub(super) fn low_wind(w: &World, s: &Scratch) -> Vec<[f64; 2]> {
    let (planet, cell, air) = (w.planet.as_ref().filter(|_| w.edges_open()), Cell::of(w), w.mesh.level(AIR));
    let mut alone = vec![false; air.n()];
    blocks(w).iter().filter(|b| b.len() == 1).for_each(|b| alone[b[0] as usize] = true);
    ends(&w.cfg, s, air)
        .iter()
        .enumerate()
        .map(|(j, e)| {
            let own = if alone[j] { e.grad } else { [0.0; 2] };
            let g = planet.map_or(own, |p| [0.0, p.at(e.lat, w.season()).south]);
            let (down, turn) = (cell.share(e.lat), w.cfg.rotation * e.lat.to_radians().sin());
            [w.cfg.wind * (down * g[0] - turn * g[1]), w.cfg.wind * (down * g[1] + turn * g[0])]
        })
        .collect()
}

/// Each air node's rising mass flux, mol/s (sinking where below 0): over each of its cells the low wind of the node
/// (`low_wind`) climbing the ground's rise along it, the surface the air touches (`Scratch::head`, the liquid's on the
/// liquid) fitted to the bordering cells (`wind::gradient`), lifting the air at the ground's density (the column's mol
/// over its area and its scale height) over the cell's area.
pub(super) fn rise(w: &World, s: &Scratch, wind: &[[f64; 2]]) -> Vec<f64> {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let slope = gradient(cells, &s.head);
    let mut rise = vec![0.0; air.n()];
    for (i, d) in slope.iter().enumerate() {
        let j = w.mesh.up(i, 0, AIR);
        let dense = s.nt[j] / (air.area()[j] * super::node_scale_height(s, j, w.cfg.gravity));
        rise[j] += dense * (wind[j][0] * d[0] + wind[j][1] * d[1]) * cells.area()[i];
    }
    rise
}

/// The vapour of a gas that saturates a node's air for the rain law, mol, from what saturates it at its mean ground,
/// `full`, mol: its air rising at `rise` mol/s (sinking below 0), `lower` mol in its lower half, the gas's share of it in
/// the upper half `split` (`Scratch::split`, the column's profile, saturated in shape), over the rain law's time `tau`.
/// In that time the share f = min(1, |rise| tau / lower) of the lower air changes places with the upper half's, which
/// holds ρ = split / (1 - split) of the gas per mol the lower holds: rising, that share saturates where the upper
/// half's cold holds it, so the node at full × (1 - f (1 - ρ)); sinking, it brings that much drier air down, so the node
/// saturates only at full / (1 - f (1 - ρ)). None rising nor sinking: full.
pub(super) fn saturates(full: f64, rise: f64, lower: f64, split: f64, tau: f64) -> f64 {
    let f = (rise.abs() * tau / lower.max(f64::MIN_POSITIVE)).min(1.0);
    let rho = split / (1.0 - split).max(f64::MIN_POSITIVE);
    let g = 1.0 - f * (1.0 - rho.min(1.0));
    if rise >= 0.0 { full * g } else { full / g.max(f64::MIN_POSITIVE) }
}

#[cfg(test)]
mod tests {
    use super::saturates;

    /// Air that neither rises nor sinks saturates at what saturates it at its mean ground; rising air saturates sooner
    /// and sinking air later, the more the more of its lower half changes places with its upper half, by as much as the
    /// upper half's cold holds less of the gas; a gas spread like the air (split ½) is not changed by it.
    #[test]
    fn rising_air_saturates_sooner_and_sinking_air_later() {
        let (full, lower, tau, split) = (100.0, 1e6, 86_400.0, 0.2);
        assert_eq!(saturates(full, 0.0, lower, split, tau), full);
        let (up, down) = (saturates(full, 2.0, lower, split, tau), saturates(full, -2.0, lower, split, tau));
        let f = 2.0 * tau / lower;
        assert!((up - full * (1.0 - f * 0.75)).abs() < 1e-9 && (down - full / (1.0 - f * 0.75)).abs() < 1e-9, "{up} {down}");
        assert!(saturates(full, 1e9, lower, split, tau) == full * 0.25 && saturates(full, -1e9, lower, split, tau) == full * 4.0);
        assert_eq!(saturates(full, -2.0, lower, 0.5, tau), full);
    }
}
