//! What a member of a slot of takers is in a month (laws.md 3b): its size, its body's temperature and pace, its upkeep
//! at rest, the heat its coat loses, how fast it walks, searches and chews, the band of heights it reaches, its gut's
//! capacity, and the least liquid it passes through.

use super::{Fauna, BODY, COAT, ENZYME, KEEP, NF, REACH, REPAIR};
use crate::contract;
use crate::life::{Parts, WET_TISSUE};
use crate::math::dec;
use crate::units::DAY;
use crate::World;

/// The band of heights a taker searches, decades each side of its `REACH` gene (a rule of the model).
pub const BAND: f64 = 0.5;

/// A member of a slot in a month, at its slot's mean.
#[derive(Clone, Copy, Default)]
pub(super) struct Body {
    /// Members of the slot
    pub(super) n: f64,
    /// Living mass, kg
    pub(super) m: f64,
    /// Body temperature, K; its burning's pace there over at `t_cat`; whether it is held above its ground
    pub(super) tb: f64,
    pub(super) pace: f64,
    pub(super) held: bool,
    /// Its upkeep at rest (Kleiber's and its gut's), W; the heat its coat loses to its ground, W
    pub(super) rest: f64,
    pub(super) lost: f64,
    /// Its walking speed, m/s; the width of ground it searches a s, m²/s; its chewing power, W; its working time, s a day
    pub(super) speed: f64,
    pub(super) search: f64,
    pub(super) chew: f64,
    pub(super) busy: f64,
    /// The work a m walked costs it, J
    pub(super) per_m: f64,
    /// The heights it reaches, m, and its bite's floor above the snow, m
    pub(super) band: (f64, f64),
    pub(super) floor: f64,
    /// Walls its gut can split, mol a s; the least key liquid it passes through, mol a s
    pub(super) gut: f64,
    pub(super) least: f64,
}

/// A member of slot `x` of `f` (its parts' molar masses `mm`) of cell `i` on ground at `t` K, its gut splitting walls
/// into `fuel` J of fuel a mol (`gut::split`).
/// - Its body stands at its `KEEP` gene where the ground is colder, else at the ground's temperature; its upkeep and its
///   work run at the pace of its burning there (the bare rate law, Gillooly et al. 2001); work at no more than 1.
/// - Upkeep at rest: Kleiber's law, (`c_basal` + `c_muscle`) × m^`upkeep_exp` × pace × its repair factor a day (Kleiber
///   1932; Elia 1992), and its gut's capacity, `c_capacity` of the fuel it could free (as the cover's light capacity).
/// - Heat: held above its ground it loses g (T_b − t) through its coat, g = `meeh` m^(2/3) / (`r_air` + COAT ×
///   `r_coat`) (Meeh 1879; Scholander et al. 1950).
/// - The walk: speed √(`walk_froude` g L) (Alexander and Jayes 1983), at `c_move` × (m / `move_ref`)^`move_exp` J per
///   kg and m (Taylor et al. 1982); it searches a strip as wide as its body, `busy` of the day.
/// - Its chewing power `chew` × m^`upkeep_exp`; the band ± `BAND` decades about its `REACH` gene, no higher than
///   `reach_len` lengths; its bite's floor `bite_floor` lengths.
/// - Liquid: `drink_least` × m^`drink_exp` kg a day pass through it besides its cooling (Nagy and Peterson 1988).
pub(super) fn body(w: &World, p: &Parts, f: &Fauna, (x, mm): (usize, &[f64; NF]), (i, t): (usize, f64), fuel: f64) -> Body {
    let (chem, ph, g, n) = (&w.chem, &w.cfg.phys, &f.genes[x], f.adults[x]);
    if n <= 0.0 || f.mol[x][BODY] <= 0.0 {
        return Body::default();
    }
    let m = f.mol[x][BODY] * mm[BODY] / (1.0 - WET_TISSUE) / n;
    let len = (m / ph.rho_body).cbrt();
    let (keep, rx) = (g[KEEP] as f64, &chem.reactions[p.burn]);
    let tb = keep.max(t);
    let pace = rx.arrhenius(tb.max(1.0)) / rx.arrhenius(ph.t_cat);
    let k = pace * ph.repair(g[REPAIR] as f64);
    let gut = dec(g[ENZYME]) * m * pace;
    let rest = (m.powf(ph.upkeep_exp) * (ph.c_basal + ph.c_muscle) / DAY + ph.c_capacity * dec(g[ENZYME]) * m * fuel) * k;
    let held = keep > t;
    let lost = if held { ph.meeh * m.powf(2.0 / 3.0) / (ph.r_air + g[COAT] as f64 * ph.r_coat) * (keep - t) } else { 0.0 };
    let work = pace.min(1.0);
    let speed = (ph.walk_froude * w.cfg.gravity * len).sqrt() * work;
    let (reach, wide) = (dec(g[REACH]), 10f64.powf(BAND));
    let band = (reach / wide, (reach * wide).min(ph.reach_len * len));
    Body {
        n,
        m,
        tb,
        pace,
        held,
        rest,
        lost,
        speed,
        search: speed * len,
        chew: ph.chew * m.powf(ph.upkeep_exp) / DAY * work,
        busy: ph.busy * DAY,
        per_m: ph.c_move * (m / ph.move_ref).powf(ph.move_exp) * m,
        band,
        floor: ph.bite_floor * len + contract::place(w, i).snow,
        gut,
        least: ph.drink_least * m.powf(ph.drink_exp) / DAY / chem.subs[p.liquid].molar_mass,
    }
}

impl Body {
    /// The heat it burns a s while it works the share `work` of its working time, searching `walk` m a s at full work,
    /// W: its rest and its walk, and no less than its coat loses where it keeps its heat.
    pub(super) fn heat(&self, walk: f64, work: f64) -> f64 {
        let made = self.rest + work * walk * self.per_m;
        if self.held { made.max(self.lost) } else { made }
    }
    /// The key liquid it sheds as vapour a s at that, mol (`heat`): what it makes beyond its coat's loss where it keeps
    /// its heat, at the liquid's heat of vaporisation `lv`; none at its ground's temperature, where its heat goes to its
    /// ground.
    pub(super) fn vapour(&self, walk: f64, work: f64, lv: f64) -> f64 {
        if self.held { (self.rest + work * walk * self.per_m - self.lost).max(0.0) / lv } else { 0.0 }
    }
    /// Its wear over `dt` s at `heat` W: the energy it burns per kg over `e_life` × 10^REPAIR (Rubner 1908).
    pub(super) fn wear(&self, ph: &crate::Physics, repair: f32, heat: f64, dt: f64) -> f64 {
        heat * dt / (self.m * ph.e_life * dec(repair)).max(1e-300)
    }
}
