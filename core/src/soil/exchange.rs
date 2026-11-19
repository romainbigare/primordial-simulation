//! The loose ground's exchange sites (laws.md 4): the charged faces of its fine particles and its organic matter hold
//! a share of the minerals dissolved in its liquid, in a linear balance with what stays dissolved (a distribution
//! coefficient that grows with the sites, Brady and Weil 2008), so only the dissolved share moves with the liquid that
//! drains and runs off. Coarse ground, with few sites, lets the rain wash its minerals away; fine and organic ground
//! keeps them.

/// The exchange sites of a m³ of fine particles and of a m³ of organic matter, mol of charge: 0.4 mol/kg of mixed
/// fine minerals at 2,650 kg/m³, and 2 mol/kg of organic matter at 1,400 kg/m³.
const SITES_FINE: f64 = 1.06e3;
const SITES_ORGANIC: f64 = 2.8e3;
/// The liquid each mol of sites holds the dissolved minerals of, m³: what a site holds over what a m³ of the liquid
/// holds.
const HELD_PER_SITE: f64 = 0.04;

/// The liquid whose dissolved minerals the sites of loose ground hold as much of as it does, m³ (`Soil::holds`): its
/// sites on `mineral` m³ of mineral particles, the share `fine` of them fine, and `organic` m³ of organic matter.
pub(super) fn holds(mineral: f64, fine: f64, organic: f64) -> f64 {
    HELD_PER_SITE * (SITES_FINE * fine * mineral + SITES_ORGANIC * organic)
}

/// The share of a mineral in loose ground whose sites hold the minerals of `holds` m³ of liquid (`Soil::holds`) that
/// stays dissolved in `liquid` m³ of it: the rest sits on the sites.
pub fn dissolved(holds: f64, liquid: f64) -> f64 {
    if liquid > 0.0 { liquid / (liquid + holds) } else { 0.0 }
}
