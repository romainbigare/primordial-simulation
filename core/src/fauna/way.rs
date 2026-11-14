//! The way to the liquid (laws.md 3b): a member whose cell cannot meet its need of liquid walks to the nearest cell with
//! open liquid (`contract::to_liquid`) and back, as often as its need and the share of its mass it can lose as liquid
//! between drinks ask; the way is work it burns by the law of walking. What the day's hours cannot walk stays unmet.

use crate::units::DAY;

/// What a slot fetches by the way: the liquid, mol, and the work of the walks, J, over the month.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fetched {
    pub mol: f64,
    pub work: f64,
}

/// A slot of `n` members of `m` kg living, walking at `speed` m/s at `per_m` J a m, short of `short` mol of the key
/// liquid (of molar mass `mm`, kg/mol) over `days` days, the nearest open liquid `way` m off: each trip there and back
/// brings what a member can lose between drinks, `dry_most` × m kg; it walks as many trips as its shortfall asks, no
/// more than fill the day at its speed. Returns what it fetches and the work of its walks.
pub fn fetch((n, m, speed, per_m): (f64, f64, f64, f64), (short, mm, days): (f64, f64, f64), way: f64, dry_most: f64) -> Fetched {
    if short <= 0.0 || n <= 0.0 || !way.is_finite() {
        return Fetched::default();
    }
    let need = short * mm / (n * days);
    let (carry, trip) = (dry_most * m, 2.0 * way);
    if trip <= 0.0 {
        return Fetched { mol: short, work: 0.0 };
    }
    let most = if speed > 0.0 { DAY * speed / trip } else { 0.0 };
    let trips = (need / carry).min(most);
    Fetched { mol: trips * carry * n * days / mm, work: trips * trip * per_m * n * days }
}
