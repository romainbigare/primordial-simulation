//! Activity needs liquid (14 Oct 2026, review 7 fix 6): a body's chemistry
//! runs in its liquid, so a dry body is nearly dormant. Activity a = min(1, l / l*)^n (`Body::activity`,
//! `Physics::dormancy`); upkeep and the cost of a decision, the speed-up of catalysts, building and
//! the share a freezing kills go with it. A child may get less liquid than its parent (`WET`).

use pcore::body::{Body, Lineage, CHILD, COPY_MASS, DISPERSE, LIQ, PHASE, REPAIR, STRENGTH, WET};
use pcore::event::EventKind;
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World};

/// A world with a lineage of brains that only rest, and the land cells with no liquid on them.
struct Set {
    w: World,
    lineage: u32,
    rows: Vec<f32>,
    land: Vec<usize>,
    /// The key liquid, the energy-rich solid, and the reaction that burns it
    liquid: usize,
    rich: usize,
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
    let (_, rich, liquid, _) = worldgen::life::roles(&w.chem).expect("roles");
    Set { w, lineage, rows, land, liquid, rich }
}

impl Set {
    /// A starting eater (30 kg, 10 days between updates) in land cell number `at`, resting, that does not soak
    /// liquid from the ground unless `soak`, repair 0.
    fn eater(&mut self, at: usize, soak: bool) -> Body {
        let g = self.w.life.new_genome(self.rows.clone());
        let mut b = worldgen::life::make(&mut self.w, 1, self.land[at], self.lineage, g).expect("body");
        b.genes[REPAIR] = 0.0;
        if !soak {
            b.genes[LIQ] = -4.0;
        }
        b.prepare(&self.w.chem, &self.w.cfg.phys);
        b
    }
    /// The same body with its key liquid at the share `x` of the level it keeps, at the same temperature.
    fn dried(&self, b: &Body, x: f64) -> Body {
        let mut d = b.clone();
        let t = d.temp(&self.w.chem);
        d.store[self.liquid] = x * b.level(&self.w.chem, self.liquid, 0.0);
        d.e = t * d.heat_cap(&self.w.chem);
        d
    }
}

/// Activity is min(1, l / l*)^n: 1 at the level the body keeps and above it; a third of the liquid is
/// about 512 times less (Harrington's rule for seeds, from memory); a body with no key liquid, or no
/// matter to keep a level of, has nothing to dry.
#[test]
fn activity_goes_with_the_liquid_a_body_holds() {
    let mut s = set();
    let b = s.eater(0, false);
    let (chem, p) = (&s.w.chem, s.w.cfg.phys);
    assert!((b.activity(chem, &p) - 1.0).abs() < 1e-9, "a starting body holds what it keeps: {}", b.activity(chem, &p));
    for x in [0.9, 0.5, 0.2] {
        let a = s.dried(&b, x).activity(chem, &p);
        assert!((a / x.powf(p.dormancy) - 1.0).abs() < 1e-9, "{x} of the liquid: {a}");
    }
    let third = s.dried(&b, 1.0 / 3.0).activity(chem, &p);
    println!("a third of the liquid: activity 1/{:.0}", 1.0 / third);
    assert!((1.0 / third / 512.0 - 1.0).abs() < 0.05, "a 3-fold drop is about 512 times less: 1/{}", 1.0 / third);
    assert_eq!(s.dried(&b, 3.0).activity(chem, &p), 1.0, "more than it keeps is full activity");
    // No key liquid (its safe-minimum gene off, so none was liquid at birth), or no matter: nothing to dry.
    let mut none = s.dried(&b, 0.1);
    none.wet = 0;
    assert_eq!(none.activity(chem, &p), 1.0);
    let mut bare = s.dried(&b, 0.1);
    bare.store[chem.matter().expect("matter")] = 0.0;
    assert_eq!(bare.activity(chem, &p), 1.0);
    // A node's activity is that of its mean member (totals over totals).
    let mut node = s.dried(&b, 0.5);
    node.count = 7.0;
    node.store.iter_mut().for_each(|a| *a *= 7.0);
    assert!((node.activity(chem, &p) - s.dried(&b, 0.5).activity(chem, &p)).abs() < 1e-12);
}

/// A dry body pays its upkeep times its activity, brain's cost included, and as its wear grows with
/// the upkeep it hardly ages.
#[test]
fn a_dry_body_pays_upkeep_times_activity_and_ages_slower() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let wet = s.eater(0, false);
    let dry = s.dried(&wet, 0.5);
    let (d, m) = (wet.interval() as f64, wet.mass(&s.w.chem));
    let a = dry.activity(&s.w.chem, &s.w.cfg.phys);
    let (uw, ud) = (pcore::body::upkeep(&s.w, &wet, d, m), pcore::body::upkeep(&s.w, &dry, d, m));
    println!("upkeep {uw:.4e} J wet, {ud:.4e} dry, activity {a:.4}");
    assert!((ud / uw / a - 1.0).abs() < 1e-9, "the same body, dry, pays its activity's share: {}", ud / uw);
    // The brain's decision is in it: with the brain costing a thousand times more, the dry body still pays its activity's share.
    s.w.cfg.phys.c_hidden *= 1000.0;
    let (uw2, ud2) = (pcore::body::upkeep(&s.w, &wet, d, m), pcore::body::upkeep(&s.w, &dry, d, m));
    assert!(uw2 > 2.0 * uw && (ud2 / uw2 / a - 1.0).abs() < 1e-9, "{uw2} {ud2}");
    s.w.cfg.phys.c_hidden /= 1000.0;
    // Over two months of updates the dry body's wear is a small share of the wet one's.
    let w0 = s.w.life.add(wet, s.w.day, 1);
    let d0 = s.w.life.add(dry.clone(), s.w.day, 1);
    for _ in 0..2 {
        pcore::step(&mut s.w);
    }
    let (ww, wd) = (s.w.life.bodies[w0].wear, s.w.life.bodies[d0].wear);
    println!("wear after two months: wet {ww:.3e}, dry {wd:.3e}");
    assert!(s.w.life.bodies[w0].alive && s.w.life.bodies[d0].alive);
    assert!(ww > 0.0 && wd < 0.1 * ww, "wear {wd} against {ww}");
}

/// The speed-up of a catalyst is 1 + (f - 1) a: a body with no liquid has the bare reaction (as if the catalyst gene
/// were off but for its cost), half the liquid a small share of the speed-up, and a wet body all of it.
#[test]
fn a_dry_bodys_catalysed_rate_falls_toward_the_bare_rate() {
    let mut s = set();
    let wet = s.eater(0, false);
    let lay = pcore::body::Layout::of(&s.w.chem);
    let burn = (0..s.w.chem.reactions.len()).find(|&r| wet.genes[lay.cat + r] > 0.0 && s.w.chem.dh(r) < 0.0).expect("a catalyst for burning");
    let f = wet.dec(lay.cat + burn);
    // The same stores with the catalyst at the bare reaction (a gene of 1e-9: on, but factor 1).
    let bare = |b: &Body| {
        let mut x = b.clone();
        let on: Vec<usize> = (0..s.w.chem.reactions.len()).filter(|&r| x.genes[lay.cat + r] > 0.0 && s.w.chem.dh(r) < 0.0).collect();
        for r in on {
            x.genes[lay.cat + r] = 1e-9;
        }
        x.k = Default::default();
        x.prepare(&s.w.chem, &s.w.cfg.phys);
        x
    };
    let rel = |x: f64| {
        let b = s.dried(&wet, x);
        // (A time short enough for the reaction to stay first order.)
        pcore::body::release(&s.w, &b, 0.01) / pcore::body::release(&s.w, &bare(&b), 0.01)
    };
    let (full, half, none) = (rel(1.0), rel(0.5), rel(0.0));
    let a = s.dried(&wet, 0.5).activity(&s.w.chem, &s.w.cfg.phys);
    println!("catalysed over bare: wet {full:.1} (f {f:.1}), half the liquid {half:.2} (1 + (f - 1) a = {:.2}), none {none:.4}", 1.0 + (f - 1.0) * a);
    assert!((full / f - 1.0).abs() < 0.02, "a wet body has all of its speed-up: {full} of {f}");
    assert!((half / (1.0 + (f - 1.0) * a) - 1.0).abs() < 0.03, "{half} against {}", 1.0 + (f - 1.0) * a);
    assert!((none - 1.0).abs() < 1e-6, "a body with no liquid has the bare rate: {none}");
}

/// Building (`copy::grow`) goes with the activity: the allowance `c_build` is times it.
#[test]
fn a_dry_body_builds_in_proportion_to_its_activity() {
    let mut s = set();
    // The allowance sets how much it builds (a tiny `c_build`, so nothing else binds).
    s.w.cfg.phys.c_build = 1e-2;
    let wet = s.eater(0, false);
    let mut dry = s.dried(&wet, 0.5);
    let mut wet = wet;
    let a = dry.activity(&s.w.chem, &s.w.cfg.phys);
    let m = s.w.chem.matter().expect("matter");
    let (m0, m1) = (wet.store[m], dry.store[m]);
    // (What it may build goes with its mass^0.75 too, and the dry body has the lighter liquid.)
    let eq = dry.supply(&s.w.chem, &s.w.cfg.phys) / wet.supply(&s.w.chem, &s.w.cfg.phys);
    assert!(pcore::copy::grow(&mut s.w, &mut wet, 10.0) && pcore::copy::grow(&mut s.w, &mut dry, 10.0));
    let (gw, gd) = (wet.store[m] - m0, dry.store[m] - m1);
    println!("matter built: wet {gw:.4e} mol, dry {gd:.4e}, activity {a:.4}");
    assert!(gw > 0.0 && (gd / gw / (a * eq) - 1.0).abs() < 0.02, "built {gd} against {gw} (activity {a}, mass {eq})");
}

/// A key liquid that froze kills the share of a node that is free liquid to freeze: its activity.
/// A wet node dies of PHASE, a dry one (a half of the liquid it keeps: 1.9% active) loses 2% of its members a
/// month's update and lives; a dry individual dies by that chance. Matter and energy are kept.
#[test]
fn frost_kills_in_proportion_to_activity() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let count = 10_000.0;
    let node = |s: &mut Set, at: usize, x: f64| {
        let b = s.eater(at, false);
        let mut b = s.dried(&b, x);
        // A node of `count` such members (its matter and energy booked as an edit).
        let w = &mut s.w;
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (count - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (count - 1.0) * b.store[k] * w.chem.subs[k].hf, count * b.store[k]);
        }
        (w.ledger.imposed_energy, b.e, b.count) = (w.ledger.imposed_energy + (count - 1.0) * b.e, count * b.e, count);
        w.life.add(b, w.day, 1)
    };
    let (wet, dry) = (node(&mut s, 0, 1.0), node(&mut s, 1, 0.5));
    let a = s.w.life.bodies[dry].activity(&s.w.chem, &s.w.cfg.phys);
    let (t0, l0) = (pcore::check::totals(&s.w), s.w.ledger.clone());
    // The key liquid freezes at the temperature the bodies are at (they are at their cell's).
    s.w.chem.subs[s.liquid].melt = 400.0;
    pcore::step(&mut s.w);
    let killed: f64 = s.w.events.iter().filter(|e| matches!(e.kind, EventKind::Died { cause: PHASE, .. })).map(|e| e.value).sum();
    let (cw, cd) = (s.w.life.bodies[wet].alive, s.w.life.bodies[dry].count);
    println!("wet node alive {cw}; dry node {cd:.0} of {count} left (activity {a:.4}); died of PHASE {killed:.0}");
    assert!(!cw, "a wet node frozen dies");
    assert!(s.w.life.bodies[dry].alive, "a dry node lives");
    // Three updates in the month, each takes the share `a` (of those left; some other deaths may add a few).
    let left = (1.0 - a).powi(3) * count;
    assert!((cd / left - 1.0).abs() < 0.02, "left {cd} of {count}, expected {left:.0}");
    assert!(killed >= count - cd - 1.0 && killed >= 0.9 * count, "died of PHASE {killed}");
    let (de, dq) = pcore::check::drift(&s.w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A body whose `WET` gene is below 0 gives its children less of its key liquid than of the rest, and
/// they start as warm as it is. A dry child pays almost nothing to be, so it waits for a long time
/// where a wet one dies of want; given liquid it soaks it up and wakes. Matter and energy are kept.
#[test]
fn a_dry_child_waits_and_wakes_when_it_soaks_liquid() {
    let wet_gene = -0.5f32;
    let run = |wet: f32| {
        let mut s = set();
        s.w.cfg.nodes = false;
        let (t0, l0) = (pcore::check::totals(&s.w), s.w.ledger.clone());
                let mut b = s.eater(0, false);
        (b.genes[WET], b.genes[DISPERSE], b.genes[STRENGTH]) = (wet, 0.0, 0.0);
        // A mature body (a body of 21 kg, its copy mass 1.6 kg) with a brood of three children.
        b.genes[COPY_MASS] = 0.2;
        b.prepare(&s.w.chem, &s.w.cfg.phys);
        let m = s.w.chem.matter().expect("body matter");
        let (_, h) = b.wet_h(&s.w.chem);
        b.brood = 3.0 * b.dec(CHILD) * b.dec(COPY_MASS) * (1.0 - h) / s.w.chem.subs[m].molar_mass;
        let (before, t) = (b.store.clone(), b.temp(&s.w.chem));
        let (slot, id) = {
            let slot = s.w.life.add(b, s.w.day, 1);
            let mut b = std::mem::take(&mut s.w.life.bodies[slot]);
            assert!(pcore::copy::copy(&mut s.w, slot, &mut b));
            let id = b.id(slot);
            s.w.life.bodies[slot] = b;
            (slot, id)
        };
        let kids: Vec<usize> = (0..s.w.life.bodies.len()).filter(|&k| k != slot && s.w.life.bodies[k].alive && s.w.life.bodies[k].parent == id).collect();
        assert!(!kids.is_empty(), "no children");
        let c = &s.w.life.bodies[kids[0]];
        let (share_rich, share_liquid) = (c.store[s.rich] / before[s.rich], c.store[s.liquid] / before[s.liquid]);
        assert!((share_liquid / share_rich / 10f64.powf(wet as f64) - 1.0).abs() < 1e-9, "the child's liquid is 10^WET of its share: {share_liquid} {share_rich}");
        assert!((c.temp(&s.w.chem) / t - 1.0).abs() < 1e-9, "the child is as warm as the parent was");
        let a0 = c.activity(&s.w.chem, &s.w.cfg.phys);
        // Matter and energy are exactly kept by the birth.
        assert_eq!(pcore::check::broken(&s.w), None);
        let (de, dq) = pcore::check::drift(&s.w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        // A month with nothing to take in and no soaking (the children's gene for it is -4): the fuel they keep, and how active.
        let fuel = |s: &Set| kids.iter().filter(|&&k| s.w.life.bodies[k].alive).map(|&k| s.w.life.bodies[k].store[s.rich]).sum::<f64>();
        let acts = |s: &Set| kids.iter().filter(|&&k| s.w.life.bodies[k].alive).map(|&k| s.w.life.bodies[k].activity(&s.w.chem, &s.w.cfg.phys)).collect::<Vec<_>>();
        let f0 = fuel(&s);
        pcore::step(&mut s.w);
        let (alive, a1, f1) = (acts(&s).len(), acts(&s).into_iter().fold(0.0, f64::max), fuel(&s) * kids.len() as f64 / (acts(&s).len().max(1)) as f64 / f0);
        // Then they can soak the liquid on the ground up (their gene for it is set, as a mutation would): they wake.
        for &k in &kids {
            let c = &mut s.w.life.bodies[k];
            c.genes[LIQ] = -0.5;
            c.k = Default::default();
        }
        let mut a2: f64 = 0.0;
        for _ in 0..3 {
            pcore::step(&mut s.w);
            a2 = a2.max(acts(&s).into_iter().fold(0.0, f64::max));
        }
        println!("WET {wet}: {} children, activity at birth {a0:.2e}; after a month {alive} alive, most active {a1:.2e}, fuel kept {f1:.4}; most active in three months of soaking {a2:.3}", kids.len());
        (kids.len(), a0, alive, a1, f1, a2)
    };
    let (n_dry, a_dry, alive_dry, act_dry, fuel_dry, wake) = run(wet_gene);
    let (n_wet, a_wet, _, _, fuel_wet, _) = run(0.0);
    assert!(n_dry > 0 && n_wet > 0);
    assert!(a_wet > 0.3 && a_dry < 0.01, "activity at birth: {a_wet} against {a_dry}");
    assert!(alive_dry == n_dry && act_dry < 0.01, "dry children wait: {alive_dry} of {n_dry} alive, activity {act_dry}");
    assert!(fuel_dry > 0.995 && fuel_wet < 0.95, "a dry child pays almost nothing to be: it keeps {fuel_dry} of its fuel in a month, a wet one {fuel_wet}");
    // (Over 0.3 before the upkeep went by the kg of the body, 0.28 after it: what is asked is that it wakes, by
    // orders of magnitude.)
    assert!(wake > 0.2 && wake > 100.0 * act_dry, "a dry child that soaks liquid wakes: activity {wake}");
}
