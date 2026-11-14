//! What a member of a slot of takers is in a month (laws.md 3b): its size, its body's temperature and pace, its upkeep
//! at rest, the heat its coat loses, how fast it walks, searches and chews, the band of heights it reaches, its gut's
//! capacity, and the least liquid it passes through.

use super::{Fauna, BODY, COAT, ENZYME, FS, KEEP, NF, REACH, REPAIR};
use crate::contract;
use crate::life::{Parts, WET_TISSUE};
use crate::math::dec;
use crate::units::DAY;
use crate::World;

/// The band of heights a taker searches, decades each side of its `REACH` gene (a rule of the model).
pub const BAND: f64 = 0.5;
/// The body temperature of the animals the rates of a taker's work were measured on, K (`walk_froude`, `chew`):
/// mammals, whose bodies stand at 36 to 38 °C (a fact of those measurements). A body works at the pace of its burning
/// at its own temperature over the pace at this one.
pub const WORK_T: f64 = 310.0;

/// A member of a group of a slot in a month, at its group's mean.
#[derive(Clone, Copy, Default)]
pub(super) struct Body {
    /// Members of the group
    pub(super) n: f64,
    /// Living mass, kg
    pub(super) m: f64,
    /// Body temperature, K; its burning's pace there over at `t_cat`; whether it is held above its ground
    pub(super) tb: f64,
    pub(super) pace: f64,
    pub(super) held: bool,
    /// Its upkeep at rest (Kleiber's and its gut's), W; the heat its coat loses to its ground in the open, W, and at rest
    /// in its shell of what lies on the ground (`shell`), W
    pub(super) rest: f64,
    pub(super) open: f64,
    pub(super) shelled: f64,
    /// Its walking speed, m/s; the width of ground it searches a s, m²/s; its chewing power, W; its working time, s a day
    pub(super) speed: f64,
    pub(super) search: f64,
    /// Its speed over the ground while it searches, m/s: it walks between bites and stands while it clips
    pub(super) walking: f64,
    pub(super) chew: f64,
    pub(super) busy: f64,
    /// The work a m walked costs it, J
    pub(super) per_m: f64,
    /// The force its jaw closes with, N (`jaw` × m^`jaw_exp`)
    pub(super) force: f64,
    /// The heights it reaches, m, and its bite's floor above the snow, m
    pub(super) band: (f64, f64),
    pub(super) floor: f64,
    /// Walls its gut can split, mol a s; the least key liquid it passes through, mol a s
    pub(super) gut: f64,
    pub(super) least: f64,
}

/// A member of group `x` of `f` (`GROUPS`: its slot's adults or young; its parts' molar masses `mm`) of cell `i` on ground at `t` K, its gut splitting walls
/// into `fuel` J of fuel a mol (`gut::split`).
/// - Its body stands at its `KEEP` gene where the ground is colder, else at the ground's temperature; its upkeep and its
///   work run at the pace of its burning there (the bare rate law, Gillooly et al. 2001): its upkeep over the pace at
///   `t_cat`, where Kleiber's numbers stand; its work (walking, chewing) over the pace at `WORK_T`, where the rows of
///   work were measured, at most 1 (a cold body runs and bites slower: Bennett 1990).
/// - Upkeep at rest: Kleiber's law, (`c_basal` + `c_muscle`) × m^`upkeep_exp` × pace × its repair factor a day (Kleiber
///   1932; Elia 1992), and its gut's capacity, `c_capacity` of the fuel it could free (as the cover's light capacity).
/// - Heat: held above its ground it loses g (T_b − t) through its coat, g = `meeh` m^(2/3) / (`r_air` + COAT ×
///   `r_coat`) (Meeh 1879; Scholander et al. 1950), at work in the open; at rest through the shell it gathers of what
///   lies on the ground (`shell`), `gathered` kg of it lying `remains` kg a m² of its cell.
/// - The walk: speed √(`walk_froude` g L) (Alexander and Jayes 1983), at `c_move` × (m / `move_ref`)^`move_exp` J per
///   kg and m (Taylor et al. 1982); it searches a strip as wide as its body, `busy` of the day,
///   bite by bite: a bite of area (`bite_breadth` m^`bite_exp`)² (Illius and Gordon 1987; Gordon, Illius and Milne 1996)
///   takes `bite_time` to clip (Shipley et al. 1994), so the ground it clips is bounded by its bites.
/// - Its chewing power `chew` × m^`upkeep_exp`; its jaw's force `jaw` × m^`jaw_exp` (what frame it can cut,
///   `contract::cut`); the band ± `BAND` decades about its `REACH` gene, no higher than `reach_len` lengths; its bite's
///   floor `bite_floor` lengths.
/// - Liquid: `drink_least` × m^`drink_exp` kg a day pass through it besides its cooling (Nagy and Peterson 1988).
pub(super) fn body(w: &World, p: &Parts, f: &Fauna, (x, mm): (usize, &[f64; NF]), (i, t): (usize, f64), (fuel, remains, gathered): (f64, f64, f64)) -> Body {
    let (chem, ph, g, n) = (&w.chem, &w.cfg.phys, &f.genes[x % FS], f.n[x]);
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
    let work = (rx.arrhenius(tb.max(1.0)) / rx.arrhenius(WORK_T)).min(1.0);
    let speed = (ph.walk_froude * w.cfg.gravity * len).sqrt() * work;
    let (area, busy) = (ph.meeh * m.powf(2.0 / 3.0), ph.busy * DAY);
    let coat = area / (ph.r_air + g[COAT] as f64 * ph.r_coat);
    let lying = gathered.min(speed * len * busy * remains);
    let (open, shelled) = if held { (coat * (keep - t), shell(ph, area, coat, lying) * (keep - t)) } else { (0.0, 0.0) };
    let (reach, wide) = (dec(g[REACH]), 10f64.powf(BAND));
    let band = (reach / wide, (reach * wide).min(ph.reach_len * len));
    // Its search by bites: each bite costs the walk over the bite's area at its swath and the clipping (work, at its
    // pace), so the ground it clips a s is 1 / (1 / (v L) + t_bite / A_bite) (Spalinger and Hobbs 1992; Shipley et al.
    // 1994).
    let bite = (ph.bite_breadth * m.powf(ph.bite_exp)).powi(2);
    let bitten = if speed > 0.0 { 1.0 / (1.0 / (speed * len) + ph.bite_time / work / bite) } else { 0.0 };
    Body {
        n,
        m,
        tb,
        pace,
        held,
        rest,
        open,
        shelled,
        speed,
        search: bitten,
        walking: if speed * len > 0.0 { speed * bitten / (speed * len) } else { 0.0 },
        chew: ph.chew * m.powf(ph.upkeep_exp) / DAY * work,
        busy,
        per_m: ph.c_move * (m / ph.move_ref).powf(ph.move_exp) * m,
        force: ph.jaw * m.powf(ph.jaw_exp),
        band,
        floor: ph.bite_floor * len + contract::place(w, i).snow,
        gut,
        least: ph.drink_least * m.powf(ph.drink_exp) / DAY / chem.subs[p.liquid].molar_mass,
    }
}

impl Body {
    /// The heat its body loses a s in the mean over the day where it makes `made` W: in the open for its working time,
    /// in its shell for the rest, where it loses no less than it makes (a body sheds its shell before it must sweat in
    /// it) and no more than in the open.
    fn lost(&self, made: f64) -> f64 {
        let out = self.busy / DAY;
        out * self.open + (1.0 - out) * self.shelled.max(made).min(self.open)
    }
    /// The heat it burns a s while it works the share `work` of its working time, searching `walk` m a s at full work,
    /// W: its rest and its walk, and no less than its body loses where it keeps its heat (`lost`).
    pub(super) fn heat(&self, walk: f64, work: f64) -> f64 {
        let made = self.rest + work * walk * self.per_m;
        if self.held { made.max(self.lost(made)) } else { made }
    }
    /// The key liquid it sheds as vapour a s at that, mol (`heat`): what it makes beyond what its body loses where it
    /// keeps its heat, at the liquid's heat of vaporisation `lv`; none at its ground's temperature, where its heat goes to
    /// its ground.
    pub(super) fn vapour(&self, walk: f64, work: f64, lv: f64) -> f64 {
        let made = self.rest + work * walk * self.per_m;
        if self.held { (made - self.lost(made)).max(0.0) / lv } else { 0.0 }
    }
    /// Its wear over `dt` s at `heat` W: the energy it burns per kg over `e_life` × 10^REPAIR (Rubner 1908).
    pub(super) fn wear(&self, ph: &crate::Physics, repair: f32, heat: f64, dt: f64) -> f64 {
        heat * dt / (self.m * ph.e_life * dec(repair)).max(1e-300)
    }
}

/// The conductance from a body of surface `area` m² and coat conductance `coat` W/K to its ground at rest in a shell of
/// `lying` kg of what lies on the ground, W/K: the shell, at `litter_rho` kg/m³, closes around the sphere of the body's
/// area (radius r = √(area / 4π)) out to R, R³ = r³ + 3 V / 4π; heat crosses it by conduction through a spherical shell,
/// (1/r − 1/R) / (4π `litter_k`) K/W, in series with the coat. A shell without end passes 4π k r: a nest about halves
/// a small body's loss, and a large one's open loss is already under the heat it makes (laws.md 3b).
pub fn shell(ph: &crate::Physics, area: f64, coat: f64, lying: f64) -> f64 {
    let four_pi = 4.0 * std::f64::consts::PI;
    let r = (area / four_pi).sqrt();
    let outer = (r.powi(3) + 3.0 * lying.max(0.0) / ph.litter_rho / four_pi).cbrt();
    1.0 / (1.0 / coat + (1.0 / r - 1.0 / outer) / (four_pi * ph.litter_k))
}
