//! The cost of living follows temperature, and the heat a body makes stays in it (15 Oct 2026, decided
//! 14 Oct): upkeep goes by the rate law of the body's reactions, exp(-Ea (1/T - 1/T_ref)),
//! T the body's temperature, Ea the barrier of its main burning reaction, T_ref = `Physics::t_cat`
//! where the costs are anchored (`body::upkeep`, `Body::pace`); and the heat its reactions and its
//! work release leaves through its surface by the conductance of its genes, cover and walls, so the
//! body stands above its ground by that heat over that conductance (`body::live`, `body::pay`).

use pcore::body::{Body, Lineage, DISPERSE, LIQ, REPAIR};
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World};

/// A world with a lineage of brains that only rest, and the land cells with no liquid on them.
struct Set {
    w: World,
    lineage: u32,
    rows: Vec<f32>,
    land: Vec<usize>,
}

fn set() -> Set {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).collect();
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 1 });
    Set { w, lineage, rows, land }
}

impl Set {
    /// A starting eater (30 kg, 10 days between updates) in land cell number `at`, resting, soaking no liquid, repair 0.
    fn eater(&mut self, at: usize) -> Body {
        let g = self.w.life.new_genome(self.rows.clone());
        let mut b = worldgen::life::make(&mut self.w, 1, self.land[at], self.lineage, g).expect("body");
        (b.genes[REPAIR], b.genes[LIQ], b.genes[DISPERSE]) = (0.0, -4.0, 0.0);
        b.k = Default::default();
        b.prepare(&self.w.chem, &self.w.cfg.phys);
        b
    }
}

/// The same body at the temperature `t`.
fn at(w: &World, b: &Body, t: f64) -> Body {
    let mut x = b.clone();
    x.e = x.heat_cap(&w.chem) * t;
    x
}

/// Upkeep at a temperature is its upkeep at `t_cat` times exp(-Ea (1/T - 1/T_ref)), Ea the barrier of the
/// first reaction the body burns with (`Consts::burn`): the same rate law as the reactions that pay it, so
/// about 2.5 times more for each 10 K at 290 K for the solid's 7,760 K. No new number. A body that burns nothing
/// pays the same at every temperature. Wear goes with upkeep, so a cold body ages slower (`body::live`).
#[test]
fn upkeep_follows_the_rate_law_of_the_reactions() {
    let mut s = set();
    let b = s.eater(0);
    let (w, p) = (&s.w, s.w.cfg.phys);
    let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
    let up = |x: &Body| pcore::body::upkeep(w, x, days, mass);
    let ea = w.chem.reactions[b.k.burn[0].0].ea;
    let u0 = up(&at(w, &b, p.t_cat));
    assert!(u0 > 0.0);
    for t in [250.0, 270.0, 280.0, 285.0, 295.0, 300.0, 310.0] {
        let (want, got) = ((-ea * (1.0 / t - 1.0 / p.t_cat)).exp(), up(&at(w, &b, t)) / u0);
        println!("{t} K: upkeep x {got:.4} (the law: {want:.4})");
        assert!((got / want - 1.0).abs() < 1e-9, "{t} K: {got} against {want}");
    }
    // About 2.5 times per 10 K at 290 K (Ea 7,760 K): the lives review found "about twice per 10 K".
    let q10 = up(&at(w, &b, p.t_cat + 10.0)) / u0;
    assert!(q10 > 2.0 && q10 < 3.0, "10 K warmer: x {q10}");
    // A body that burns nothing pays the same at every temperature.
    let mut dull = b.clone();
    dull.k.burn.clear();
    assert!((up(&at(w, &dull, 250.0)) / up(&at(w, &dull, 310.0)) - 1.0).abs() < 1e-12);
}

/// A cold body wears slower than a warm one in the same cell, in the ratio of their upkeep (wear is the energy the
/// upkeep turns over), and the update keeps matter and energy exactly.
#[test]
fn a_cold_body_ages_slower() {
    let mut s = set();
    let (cold, warm) = (s.eater(0), s.eater(0));
    let (t_cold, t_warm) = (270.0, 300.0);
    let (mut cold, mut warm) = (at(&s.w, &cold, t_cold), at(&s.w, &warm, t_warm));
    // (The temperatures are edits.)
    let fix = |w: &mut World, b: &mut Body, t: f64| {
        let e = t * b.heat_cap(&w.chem);
        w.ledger.imposed_energy += e - b.e;
        b.e = e;
    };
    fix(&mut s.w, &mut cold, t_cold);
    fix(&mut s.w, &mut warm, t_warm);
    let w = &mut s.w;
    let (days, mass) = (1.0, cold.mass(&w.chem));
    cold.prepare(&w.chem, &w.cfg.phys);
    warm.prepare(&w.chem, &w.cfg.phys);
    let (uc, uw) = (pcore::body::upkeep(w, &cold, days, mass), pcore::body::upkeep(w, &warm, days, mass));
    let (c, k) = (w.life.add(cold, w.day, 1), w.life.add(warm, w.day, 1));
    let (t0, l0) = (pcore::check::totals(w), w.ledger.clone());
    pcore::body::survey(w);
    w.day += 1;
    pcore::body::day(w);
    assert!(w.life.bodies[c].alive && w.life.bodies[k].alive);
    let (wc, wk) = (w.life.bodies[c].wear, w.life.bodies[k].wear);
    println!("wear after a day: cold (270 K) {wc:.4e}, warm (300 K) {wk:.4e}; upkeep {uc:.4e} and {uw:.4e} J");
    assert!(wc > 0.0 && wk > 10.0 * wc, "the warm body ages {} times as fast", wk / wc);
    assert!((wk / wc / (uw / uc) - 1.0).abs() < 1e-6, "wear {wk:e} / {wc:e} against the upkeep {uw:e} / {uc:e}");
    let (de, dq) = pcore::check::drift(w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
