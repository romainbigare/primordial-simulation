//! Edges remember use: a body going back and forth through cover between two cells clears the
//! ground along their edge (it gets cheaper to cross), and the cover grows back over years
//! once the moving stops. A body too weak to give the push of a mover does not move.

use pcore::body;
use pcore::life::Lineage;
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World};

/// Cover per area in the two cells, kg/m².
const COVER: f64 = 20.0;

fn scene() -> (World, usize, usize, usize) {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: 5.0, lat_north: 10.0, ..Default::default() };
    let mut w = worldgen::terrain::build(3, 12, 8, chem, &init, cfg);
    pcore::step(&mut w);
    let n = w.grid.n();
    let a = (0..n - 1).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i + 1][2] > 0.0 && i % 12 < 11).expect("two land cells");
    let mut r = Rng::new(3, 0, 0, Purpose::Body);
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut base = brain::random(shape, &mut r, 0.1, [0.0; 8], 0.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 });
    let genome = w.life.new_genome(rows);
    for cell in [a, a + 1] {
        let mut p = worldgen::life::make(&mut w, 0, cell, lineage, genome).expect("plant");
        // Scale the node to the cover wanted (stores are totals).
        let k = COVER * w.grid.area() / p.mass(&w.chem);
        p.store.iter_mut().for_each(|x| *x *= k);
        p.count *= k;
        w.life.add(p, w.day, 1);
    }
    let m = worldgen::life::make(&mut w, 1, a, lineage, genome).expect("mover");
    let slot = w.life.add(m, w.day, 1);
    (w, a, a + 1, slot)
}

#[test]
fn going_back_and_forth_clears_the_edge_and_cover_closes_it() {
    let (mut w, a, b, slot) = scene();
    pcore::survey::survey(&mut w);
    assert!((choose::eased(&w, a, 0) - 1.0).abs() < 1e-9);
    let (rich, mut crossed) = (worldgen::life::roles(&w.chem).expect("roles").1, 0);
    // Moving needs force (review 7 fix 7): with the strength of the starting plants (0.05: a force of 0.025 of its weight)
    // the mover cannot give the push of a mover (`bump`, 0.05 of its weight) and stays; with its own (0.6) it goes, below.
    let mut weak = w.life.bodies[slot].clone();
    weak.genes[body::STRENGTH] = 0.05;
    choose::go(&mut w, slot, &mut weak, 0, b);
    assert_eq!(weak.cell as usize, a, "a body too weak to push walked");
    for t in 0..5000 {
        // Fuel for the trip is given each time (the scene is not about eating),
        // given with its heat at the body's temperature, so the body does not cool.
        let mut m = std::mem::take(&mut w.life.bodies[slot]);
        m.e += (1e3 - m.store[rich]) * w.chem.subs[rich].cp * m.temp(&w.chem);
        m.store[rich] = 1e3;
        let (from, d, to) = if t % 2 == 0 { (a, 0, b) } else { (b, 4, a) };
        assert_eq!(m.cell as usize, from);
        choose::go(&mut w, slot, &mut m, d, to);
        crossed += (m.cell as usize == to) as u32;
        w.life.bodies[slot] = m;
        pcore::survey::survey(&mut w);
        if choose::eased(&w, a, 0) > 3.0 {
            break;
        }
    }
    let worn = choose::eased(&w, a, 0);
    assert!(crossed > 10 && worn > 3.0, "{crossed} crossings, edge {worn} times easier");
    // Moving stops: the cover grows back into the cleared ground over some decades.
    let year = 360.0 * 86_400.0;
    let mut left = vec![worn];
    for _ in 0..40 {
        choose::regrow(&mut w, year);
        left.push(choose::eased(&w, a, 0));
    }
    assert!(left[1] > 1.2, "recovers slowly: {left:?}");
    assert!(left.windows(2).all(|p| p[1] <= p[0] + 1e-9) && left[40] < 1.01, "and fully: {left:?}");
}
