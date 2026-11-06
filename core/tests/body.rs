//! A body's own physics (laws.md 6): its upkeep by Kleiber's law at the pace of its temperature, its heat (it keeps
//! warm by burning and cool by evaporating), its key liquid (drinking, frost, boiling), digestion and its age. Damage:
//! `damage.rs`.

use pcore::body::{self, Body, Layout, COOL_T, HEAT, KEEP_T, PHASE};
use pcore::life::Handle;
use pcore::event::EventKind;
use pcore::{brain, World, AIR};
use worldgen::testkit::{add, cover, died, dry_lands, eater, held_day, hold_ground, kg, node_of, out, put_kg_m2, scale, small_world, start, stores, took_from_ground, warm, Books, Brain, Line, LIQUID, MATTER, RICH};

/// Days the takes from the ground run for (`matter_taken_from_outside_is_digested_into_the_solid`).
const TAKE_DAYS: usize = 50;

/// Body `s` until its next update (the first the bodies' month runs: its decision, then its physics over the month
/// ahead in sub-steps), the ground of its cell held at `t` K each day; returns the body after it.
fn after_update(w: &mut World, s: Handle, t: f64) -> Body {
    let (id, i, updates) = (w.life[s].id, w.life[s].cell as usize, w.counts.updates);
    while w.counts.updates == updates {
        held_day(w, &[i], t);
    }
    let h = w.life.find(id).unwrap_or_else(|| panic!("the body died on day {}", w.day()));
    w.life[h].clone()
}

/// A resting eater (a brain that only rests) of `copy` kg copy mass on dry land, its ground held at `t` K: placed.
fn resting(t: f64) -> (World, Handle) {
    let (mut w, i) = start();
    let b = eater(&mut w, i, brain::REST, 1.0);
    hold_ground(&mut w, i, t);
    let s = w.add(b);
    (w, s)
}

/// Upkeep goes by Kleiber's law and by the pace of the body's temperature: per kg of living body it is the same at
/// every size of member times (copy mass / `m_ref`)^(`intake_exp` - 1), and a warmer body pays more by the rate law of
/// its burning (`Body::pace`), the seasons' only clock.
#[test]
fn upkeep_goes_by_kleiber_and_by_the_pace_of_temperature() {
    let (mut w, i) = start();
    let b = eater(&mut w, i, brain::REST, 1.0);
    let (chem, p) = (&w.chem, &w.cfg.phys);
    let (mass, built) = (b.mass(chem), b.built(chem));
    let cost = |t: f64| body::upkeep(&w, &b, 30.0, mass, t, 1.0);
    assert!(cost(300.0) > 1.3 * cost(290.0), "the pace: {} at 290 K, {} at 300 K", cost(290.0), cost(300.0));
    // A body with ten times the copy mass pays 10^(intake_exp - 1) as much per kg of living body.
    let mut big = b.clone();
    big.genes[pcore::body::COPY_MASS] += 1.0;
    big.prepare(chem, &w.cfg);
    let per = |x: &Body| body::upkeep(&w, x, 30.0, mass, p.t_cat, 1.0) / built;
    let ratio = per(&big) / per(&b);
    println!("upkeep per kg at ten times the copy mass: x{ratio:.3}");
    assert!(ratio < 1.0 && ratio > 10f64.powf(p.intake_exp - 1.0) * 0.8, "x{ratio}");
}

/// A body on cold ground keeps warm by burning its fuel (`KEEP_T`): after an update it stands at its `KEEP_T` gene,
/// above the ground, and burnt more than one that stands there on warm ground (its `COOL_T` gene at its `KEEP_T`, so
/// its pace is the same); the books are kept.
#[test]
fn an_animal_keeps_warm_by_burning_fuel() {
    let burnt = |t: f64| {
        let (mut w, s) = resting(t);
        let b = &mut w.life[s];
        b.genes[COOL_T] = b.genes[KEEP_T];
        let books = Books::now(&w);
        let fuel0 = w.life[s].amount(w.chem.body(), RICH);
        let b = after_update(&mut w, s, t);
        books.assert_kept(&w, 1e-9);
        (b.temp(&w.chem), b.gene(KEEP_T), fuel0 - b.amount(w.chem.body(), RICH))
    };
    let (t_cold, keep, cold) = burnt(255.0);
    let (_, _, warm) = burnt(290.0);
    println!("on ground at 255 K it stands at {t_cold:.2} K (keeps {keep} K) and burnt {cold:.3} mol; at 290 K {warm:.3} mol");
    assert!((t_cold - keep).abs() < 0.5, "it stands at {t_cold} K, not its {keep} K");
    assert!(cold > 1.2 * warm, "it burnt {cold} mol in the cold, {warm} in the warm");
}

/// A body on hot ground keeps cool by evaporating its key liquid (`COOL_T`): after an update it stands below its ground
/// and holds less liquid than the level it drinks to (on wet ground it drinks as it sweats, laws.md 6.4); the vapour
/// went to the air with its latent heat, and the books are kept.
#[test]
fn an_animal_keeps_cool_by_evaporating() {
    let (mut w, s) = resting(330.0);
    let i = w.life[s].cell as usize;
    put_kg_m2(&mut w, LIQUID, i, 50.0);
    let books = Books::now(&w);
    let b = after_update(&mut w, s, 330.0);
    let (chem, at) = (&w.chem, w.chem.body().place(LIQUID).expect("a store"));
    let (liquid, level) = (b.amount(chem.body(), LIQUID), b.level(chem, at, b.mass(chem)));
    println!("on ground at 330 K it stands at {:.2} K (cools from {} K), liquid {liquid:.2} mol against its level {level:.2}", b.temp(chem), b.gene(COOL_T));
    assert!(b.temp(chem) < 330.0 && liquid < level, "no cooling");
    books.assert_kept(&w, 1e-9);
}

/// A body drinks its key liquid from its ground up to the level it keeps: a body that lost most of its liquid holds
/// more after an update on wet ground.
#[test]
fn a_body_drinks_its_liquid_from_the_ground() {
    let (mut w, s) = resting(290.0);
    let i = w.life[s].cell as usize;
    put_kg_m2(&mut w, LIQUID, i, 50.0);
    let dry = 0.5 * w.life[s].amount(w.chem.body(), LIQUID);
    let mut b = std::mem::take(&mut w.life[s]);
    let kg_dry = dry * w.chem.subs[LIQUID].molar_mass;
    add(&mut w, &mut b, LIQUID, -kg_dry);
    let low = b.amount(w.chem.body(), LIQUID);
    w.life[s] = b;
    let books = Books::now(&w);
    let b = after_update(&mut w, s, 290.0);
    println!("liquid {low:.2} -> {:.2} mol", b.amount(w.chem.body(), LIQUID));
    assert!(b.amount(w.chem.body(), LIQUID) > low + 5.0, "it did not drink");
    books.assert_kept(&w, 1e-9);
}

/// The first cell below 0 m whose standing liquid is deeper than twice the height of a body of `EATER_KG`.
fn deep_cell(w: &World) -> usize {
    let side = w.cfg.phys.side(worldgen::testkit::EATER_KG);
    (0..w.mesh.cells().n()).find(|&i| !w.is_land(i) && w.life.env[i][1] > 2.0 * side).expect("a cell of deep liquid")
}

/// A body in liquid deeper than its height passes its heat to the liquid (laws.md 6.3): through the liquid's film and
/// the share `wet_coat` of its coat. A resting eater that drinks none and keeps warm at 285 K, in deep liquid and on dry
/// land, each ground held at 275 K: each loses through each m² of its coat (`meeh` × mass^(2/3)) per K over its ground
/// what its insulation passes there, 1 / (`r_air` + coat) in the air and 1 / (`r_liquid` + `wet_coat` × coat) in the
/// liquid, about six times as much: between what its last sub-step's loss gives over the surface it had at the start of
/// its update and at its end, as it burns its fuel through it. The books are kept.
#[test]
fn a_body_in_deep_liquid_loses_its_heat_to_the_liquid() {
    let per_m2_k = |wet: bool| {
        let (mut w, land) = start();
        let i = if wet { deep_cell(&w) } else { land };
        let mut b = eater(&mut w, i, brain::REST, 1.0);
        (b.genes[KEEP_T], b.genes[COOL_T]) = (285.0, 295.0);
        // (It drinks no cold liquid, whose warming would add to what leaves through its coat.)
        b.genes[Layout::of(&w.chem).upt + w.chem.body().place(LIQUID).expect("a store")] = 0.0;
        b.prepare(&w.chem, &w.cfg);
        hold_ground(&mut w, i, 275.0);
        let mass = b.mass(&w.chem);
        let s = w.add(b);
        let books = Books::now(&w);
        let b = after_update(&mut w, s, 275.0);
        books.assert_kept(&w, 1e-9);
        let t = b.temp(&w.chem);
        assert!((t - 285.0).abs() < 0.5, "it keeps warm at 285 K, not {t} K");
        let per = |m: f64| b.lived.lost / (w.cfg.phys.meeh * m.powf(2.0 / 3.0)) / (t - 275.0);
        (per(mass), per(b.mass(&w.chem)), b.gene(HEAT).clamp(0.0, 1.0) * w.cfg.phys.r_coat)
    };
    let ((dry_lo, dry_hi, coat), (wet_lo, wet_hi, _)) = (per_m2_k(false), per_m2_k(true));
    let p = pcore::Physics::default();
    let (air, liquid) = (1.0 / (p.r_air + coat), 1.0 / (p.r_liquid + p.wet_coat * coat));
    println!("W per m² and K: on land {dry_lo:.3} to {dry_hi:.3} (its insulation passes {air:.3}), in deep liquid {wet_lo:.3} to {wet_hi:.3} ({liquid:.3})");
    assert!(liquid > 4.0 * air, "a wet coat passes {liquid}, a dry one {air}");
    for (lo, hi, want) in [(dry_lo, dry_hi, air), (wet_lo, wet_hi, liquid)] {
        assert!(lo <= want * (1.0 + 1e-3) && want <= hi * (1.0 + 1e-3), "it lost {lo} to {hi} W per m² and K, not {want}");
    }
}

/// A body takes only what lies within its reach (laws.md 5.3, 3.3): it keeps to the surface where it breathes, so under
/// liquid deeper than its height and its reach (each its size) it takes neither the solids of the ground nor the
/// cover. An eater that only takes, with the energy-rich solid on its ground and cover over it, takes both on dry land
/// and nothing in deep liquid, over ten days; the books are kept.
#[test]
fn a_body_cannot_take_from_the_ground_under_deep_liquid() {
    let took = |wet: bool| {
        let (mut w, land) = start();
        let i = if wet { deep_cell(&w) } else { land };
        put_kg_m2(&mut w, RICH, i, 3.0);
        cover(&mut w, i, 0.5);
        let b = eater(&mut w, i, brain::TAKE, 6.0);
        w.add(b);
        let books = Books::now(&w);
        let (mut ground, mut grazed) = (0.0, 0.0);
        for _ in 0..TAKE_DAYS / 5 {
            w.events.clear();
            pcore::day_with_month_at_start(&mut w);
            ground += took_from_ground(&w);
            grazed += w.events.iter().map(|e| if let EventKind::Took { from: pcore::choose::COVER, .. } = e.kind { e.value } else { 0.0 }).sum::<f64>();
        }
        books.assert_kept(&w, 1e-9);
        (ground, grazed)
    };
    let (land, deep) = (took(false), took(true));
    println!("in ten days it took {:.3} kg from the ground and {:.3} kg of cover on land, {:.3} and {:.3} in deep liquid", land.0, land.1, deep.0, deep.1);
    assert!(land.0 > 0.01 && land.1 > 0.01, "on land it took {} kg from the ground and {} kg of cover", land.0, land.1);
    assert!(deep.0 == 0.0 && deep.1 == 0.0, "in deep liquid it took {} kg from the ground and {} kg of cover", deep.0, deep.1);
}

/// Its key liquid frozen kills it: a body with no fuel to keep warm on ground below the liquid's melting point dies of
/// `PHASE`, and all it held goes to the ground.
#[test]
fn a_frozen_body_dies() {
    let (mut w, s) = resting(250.0);
    let mut b = std::mem::take(&mut w.life[s]);
    let fuel = kg(&w, &b, RICH);
    add(&mut w, &mut b, RICH, -0.999 * fuel);
    b.genes[KEEP_T] = 200.0;
    b.prepare(&w.chem, &w.cfg);
    w.life[s] = b;
    let (books, id) = (Books::now(&w), w.life[s].id);
    w.events.clear();
    for _ in 0..40 {
        let i = w.life[s].cell as usize;
        held_day(&mut w, &[i], 250.0);
        if w.life.find(id).is_none() {
            break;
        }
    }
    assert!(w.life.find(id).is_none() && died(&w, PHASE) > 0.0, "it did not freeze to death");
    books.assert_kept(&w, 1e-9);
}

/// Digestion: body matter that a body takes in from outside is broken into the energy-rich solid and the minerals as it
/// comes in (the reverse of growth), so it enters as fuel, never as body, and the body builds its own matter from it at
/// the cost of work. Elements and energy are kept. A body whose catalyst for burning matter is off takes none.
#[test]
fn matter_taken_from_outside_is_digested_into_the_solid() {
    let take_from_ground = |digests: bool| {
        let (mut w, i) = start();
        w.cfg.phys.c_handle = 0.0;
        w.cfg.phys.c_build = 0.0;
        // 3 kg of body matter per m² on its ground (booked as an edit).
        put_kg_m2(&mut w, MATTER, i, 3.0);
        let mut b = eater(&mut w, i, brain::TAKE, 6.0);
        let lay = Layout::of(&w.chem);
        let (_, burn_matter) = w.chem.matter_roles().expect("matter");
        if !digests {
            b.genes[lay.diet(&w.chem, burn_matter).expect("a diet gene")] = -1.0;
        }
        // Only matter on the ground to take: it takes neither the liquid nor the solid.
        let at = |k: usize| w.chem.body().place(k).expect("a store");
        (b.genes[lay.upt + at(LIQUID)], b.genes[lay.upt + at(RICH)]) = (-0.1, -0.1);
        let (rich0, matter0, size0) = (b.amount(w.chem.body(), RICH), b.amount(w.chem.body(), MATTER), b.size(&w.chem));
        let s = w.add(b);
        let books = Books::now(&w);
        // (What the body took from the ground, by the events, not what left it: dead matter on the ground also rots.)
        let mut took = 0.0;
        for _ in 0..TAKE_DAYS {
            w.events.clear();
            pcore::day_with_month_at_start(&mut w);
            took += took_from_ground(&w);
        }
        let b = &w.life[s];
        books.assert_kept(&w, 1e-9);
        (b.amount(w.chem.body(), RICH) - rich0, b.amount(w.chem.body(), MATTER) - matter0, took / w.chem.subs[MATTER].molar_mass, b.size(&w.chem) - size0)
    };
    let (rich, matter, taken, size) = take_from_ground(true);
    println!("an eater took {taken:.2} mol of matter from the ground: solid +{rich:.2} mol, its own matter {matter:+.3} mol, body {size:+.3} kg");
    assert!(taken > 0.5, "it took none");
    // The solid rose by 118 per mol taken, less what it burnt in those days; its own matter did not rise.
    assert!(rich > 20.0 * taken, "solid +{rich} mol for {taken} mol of matter");
    assert!(matter <= 1e-9 && size <= 1e-9, "matter entered as body: {matter} mol, body {size} kg");
    let (_, _, none, _) = take_from_ground(false);
    assert!(none < 1e-12, "a body that cannot burn matter took {none} mol of it");
    // The reverse of growth, exactly, called directly.
    let (mut w, i) = start();
    let mut b = eater(&mut w, i, brain::REST, 6.0);
    add(&mut w, &mut b, MATTER, 34.56);
    let (s, mut b) = out(&mut w, b);
    let (books, before) = (Books::now(&w), stores(&w, &b));
    let x = b.digest(&w.chem, 10.0, &mut 0.0);
    assert!((x - 10.0).abs() < 1e-12);
    // 10 of matter and the liquid growth makes with it become what growth is made of, 10 times its counts.
    let growth = w.chem.build.as_ref().expect("growth");
    for (side, sign) in [(&growth.inputs, 1.0), (&growth.outputs, -1.0)] {
        for p in side.iter() {
            let k = p.0 as usize;
            assert!((sign * (b.amount(w.chem.body(), k) - before[k]) - 10.0 * p.1 as f64).abs() < 1e-9, "substance {k}: {} -> {}", before[k], b.amount(w.chem.body(), k));
        }
    }
    w.life[s] = b;
    books.assert_kept(&w, 1e-9);
}

/// The survey adds up what each cell holds (`Life::sum`): a cell whose bodies are all gone (and that has no cover) holds
/// nothing at the next survey.
#[test]
fn a_cell_left_with_no_bodies_holds_nothing_at_the_next_survey() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::testkit::eater_nodes(&mut w, 10.0);
    pcore::survey::survey(&mut w);
    let i = w.life.living().map(|x| w.life[x.0].cell as usize).next().expect("a body");
    let members: f64 = w.life.cell(i).map(|s| w.life[s].n()).sum();
    assert!(members > 0.0 && w.life.sum(i)[0] == members, "{members} members, sum {}", w.life.sum(i)[0]);
    let all: Vec<Handle> = w.life.cell(i).collect();
    for h in all {
        let b = std::mem::take(&mut w.life[h]);
        w.in_cell(i, |w, d| body::die(w, d, h, b, body::LACK));
    }
    pcore::survey::survey(&mut w);
    assert!(w.life.sum(i).iter().all(|&x| x == 0.0), "the emptied cell still holds something");
    assert_eq!(pcore::check::broken(&w), None);
}

/// Living wears a body: its members die at the rate of the energy their upkeep turns over per kg, over `e_life` x
/// 10^REPAIR, so they live that life on average: a node loses the share 1 - e^-wear of its members at each sub-step of
/// its updates, an individual dies with that chance. More repair: a longer life (10 times per unit) at a higher upkeep
/// (the repair share, a quarter at gene 0, doubles per unit). Matter and energy are kept.
#[test]
fn bodies_die_of_age_by_their_rate_of_living() {
    use pcore::body::{AGE, COPY_MASS, DISPERSE, LEARN, REPAIR, STRENGTH};
    use pcore::brain;
    const ONES: usize = 60;
    let mut w = small_world();
    // Bodies that do not grow (their mass, which the wear goes by, stays): no building work. Their copy mass is half their body: the matter that rots is not starving. No one
    // ages until they have settled, and each individual stands alone in its cell, so none merges into a node.
    (w.cfg.phys.c_build, w.cfg.phys.e_life) = (0.0, 1e30);
    // Upkeep goes with the temperature: the bodies stand on ground held at the temperature the costs are anchored to
    // (`Physics::t_cat`, booked as an edit), so that they pay what the costs say, in two cells of high ground (rain
    // does not flood it) with other air nodes.
    let mut land = dry_lands(&w);
    land.sort_by(|&a, &b| w.life.env[b][2].total_cmp(&w.life.env[a][2]));
    let air = w.mesh.up(land[0], 0, AIR);
    let cells = [land[0], *land.iter().find(|&&i| w.mesh.up(i, 0, AIR) != air).expect("another air node")];
    let lone: Vec<usize> = land.iter().copied().filter(|i| !cells.contains(i)).take(ONES).collect();
    assert_eq!(lone.len(), ONES, "a cell of high ground for each individual");
    let held: Vec<usize> = cells.iter().chain(&lone).copied().collect();
    // Starting eaters whose brain only rests, weak so their stores last.
    let line = Line::new(&mut w, &Brain::only(brain::REST), 1);
    let make = |w: &mut pcore::World, i: usize, repair: f32, count: f64| {
        let mut b = line.founder(w, 1, i);
        // (And they do not learn: a body that learns from its well-being may learn to walk away in the months of the test.)
        (b.genes[STRENGTH], b.genes[DISPERSE], b.genes[REPAIR], b.genes[LEARN]) = (0.0, 0.0, repair, -6.0);
        // (No coat: bare skin holds them near their ground's temperature, which holds their pace.)
        b.genes[pcore::body::HEAT] = 0.0;
        b.genes[COPY_MASS] -= std::f32::consts::LOG10_2;
        // A node of `count` such members (its matter and energy booked as an edit), with three times the fuel.
        let fuel = 2.0 * kg(w, &b, RICH);
        add(w, &mut b, RICH, fuel);
        scale(w, &mut b, count);
        b.count = count;
        b.prepare(&w.chem, &w.cfg);
        w.add(b)
    };
    // A node of members at repair 0 alone in one cell, one at repair 0.5 in the other, and individuals at repair 0, each
    // alone in a cell (an individual's death is of one member, a node's of a share).
    let (sa, sb) = (make(&mut w, cells[1], 0.0, 1000.0), make(&mut w, cells[0], 0.5, 1000.0));
    let ones: Vec<Handle> = lone.iter().map(|&i| make(&mut w, i, 0.0, 1.0)).collect();
    // A day with the ground held; the age deaths of the day, as (cell, day, members).
    let t_cat = w.cfg.phys.t_cat;
    let day = |w: &mut pcore::World, deaths: &mut Vec<(usize, u64, f64)>| {
        w.events.clear();
        held_day(w, &held, t_cat);
        deaths.extend(w.events.iter().filter(|e| matches!(e.kind, pcore::event::EventKind::Died { cause: AGE, .. })).map(|e| (e.node as usize, e.day, e.value)));
    };
    // Two months for the bodies to settle at the temperature their heat and the ground give them, then their upkeep per
    // kg of body and day.
    let mut settle = Vec::new();
    for _ in 0..60 {
        day(&mut w, &mut settle);
    }
    let per_kg = |w: &mut pcore::World, s: Handle| {
        let mut b = std::mem::take(&mut w.life[s]);
        b.prepare(&w.chem, &w.cfg);
        let (d, m) = (w.cfg.days_per_month as f64, b.mass(&w.chem));
        let (u, act) = (pcore::body::upkeep(w, &b, d, m, b.temp(&w.chem), b.activity(&w.chem, &w.cfg.phys)) / (d * b.built(&w.chem)), b.activity(&w.chem, &w.cfg.phys));
        w.life[s] = b;
        (u, act)
    };
    assert!(ones.iter().all(|&s| w.life[s].alive), "an individual died while settling");
    let ((ua, aa), (ub, ab), (ui, ai)) = (per_kg(&mut w, sa), per_kg(&mut w, sb), per_kg(&mut w, ones[0]));
    println!("upkeep per kg of body and day: {ua:.1}, {ub:.1}, an individual {ui:.1} J");
    // (The repair gene costs 0.75 + 0.25 x 2^0.5 times as much at 0.5.)
    assert!(((ub / ab) / (ua / aa) / (0.75 + 0.25 * 2f64.sqrt()) - 1.0).abs() < 0.03 && ((ui / ai) / (ua / aa) - 1.0).abs() < 0.07, "upkeep per kg {ua} {ub} {ui}");
    // A life of 38 days at gene 0, 10^0.5 ua / ub times that at 0.5. Each counts its days from where its physics reached.
    let (life, x) = (38.0, 10f64.sqrt() * ua / ub);
    w.cfg.phys.e_life = life * ua;
    let (books, n0, every) = (Books::now(&w), w.life[sa].count, w.phys_days(cells[1]));
    let from: Vec<f64> = [sa, sb].iter().chain(&ones).map(|&s| w.life[s].last).collect();
    let day0 = w.day() as f64;
    let mut deaths = Vec::new();
    for _ in 0..240 {
        day(&mut w, &mut deaths);
    }
    // A node's mean life: the days its members lived over those who died; it loses a share at each sub-step of members
    // who die evenly between sub-steps, so over a span they live dead / ln(n / (n - dead)) times its days.
    let node_life = |cell: usize, t0: f64| {
        let (mut n, mut t, mut lived) = (n0, t0, 0.0);
        for &(_, d, dead) in deaths.iter().filter(|x| x.0 == cell && x.2 != 1.0) {
            (lived, n, t) = (lived + dead * (d as f64 - t) / (n / (n - dead).max(f64::MIN_POSITIVE)).ln(), n - dead, d as f64);
        }
        lived / (n0 - n)
    };
    let (la, lb) = (node_life(cells[1], from[0]), node_life(cells[0], from[1]));
    // The individuals dead of age in one life: each dies at a sub-step with the chance its wear since gives, so by day
    // `life` one whose physics reached u has died with the chance 1 - e^-((u - its first day) / its life).
    let (li, by) = (life * ua / ui, day0 + life);
    let chance = |f: f64| -(-((by - f) / every).floor() * every / li).exp_m1();
    let expected: f64 = from[2..].iter().map(|&f| chance(f)).sum();
    let sd = from[2..].iter().map(|&f| chance(f) * (1.0 - chance(f))).sum::<f64>().sqrt();
    let dead = deaths.iter().filter(|x| lone.contains(&x.0) && x.2 == 1.0 && (x.1 as f64) <= by).count() as f64;
    println!("lives, days: nodes {la:.1} and {lb:.1} at genes 0 and 0.5 (predicted {life} and {:.0}); individuals dead in a life {dead} of {ONES} (expected {expected:.1} +- {sd:.1})", life * x);
    assert!((la / life - 1.0).abs() < 0.1 && (lb / (life * x) - 1.0).abs() < 0.1, "node lives {la} and {lb}");
    assert!((dead - expected).abs() < 3.5 * sd, "individuals dead in a life: {dead}, expected {expected} +- {sd}");
    books.assert_kept(&w, 1e-9);
}

/// Frost kills in proportion to the cold and to activity, per unit of time (laws.md 6.5): a node of 10,000 resting
/// eaters with no fuel to keep warm, on ground half `frost_k` below its key liquid's melting point, loses over its
/// update of a month, in its sub-steps, the share (melt - T) / `frost_k` (each sub-step 1 - (1 - that)^(its days /
/// month)), T the temperature it stands at (its ground's: it keeps no warmth, in a bare coat), about half; the same
/// node half dry (its activity a) loses a times that share. Both live, and matter and energy are kept.
#[test]
fn frost_kills_in_proportion_to_activity() {
    let lost = |dry: bool| {
        let (mut w, s) = resting(290.0);
        let mut b = std::mem::take(&mut w.life[s]);
        let fuel = kg(&w, &b, RICH);
        add(&mut w, &mut b, RICH, -0.99 * fuel);
        if dry {
            let liquid = kg(&w, &b, LIQUID);
            add(&mut w, &mut b, LIQUID, -0.5 * liquid);
        }
        // No warmth of its own: it keeps none, and it makes no heat (no upkeep), so it stands at its ground's
        // temperature.
        let p = &mut w.cfg.phys;
        (p.c_basal, p.c_strength, p.c_learn, p.c_brain, p.c_hidden, p.c_memory, p.c_seen) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        b.genes[KEEP_T] = 200.0;
        b.prepare(&w.chem, &w.cfg);
        let mut b = node_of(&mut w, b, 10_000.0);
        b.prepare(&w.chem, &w.cfg);
        let a = b.activity(&w.chem, &w.cfg.phys);
        // It starts at the ground's temperature.
        let t = w.chem.subs[LIQUID].melt - 0.5 * w.cfg.phys.frost_k;
        warm(&mut w, &mut b, t);
        w.life[s] = b;
        let (books, n0) = (Books::now(&w), w.life[s].count);
        let b = after_update(&mut w, s, t);
        books.assert_kept(&w, 1e-9);
        (1.0 - b.count / n0, a)
    };
    // (Its first update integrates the month ahead.)
    let ((wet, a_wet), (dry, a_dry)) = (lost(false), lost(true));
    println!("lost {wet:.4} a month wet (activity {a_wet:.3}), {dry:.4} half dry (activity {a_dry:.3})");
    assert!(a_wet > 0.999 && a_dry < 0.9, "activities {a_wet} and {a_dry}");
    assert!((0.45..=0.5).contains(&wet), "the wet node lost {wet} a month, against a ramp of 0.5 at the ground's temperature");
    assert!((dry / wet / a_dry - 1.0).abs() < 0.1, "the dry node lost {dry} against {wet} x {a_dry}");
}
