//! The cover's roots and the liquid of their ground (laws.md 3, `cover::soak`): roots in pores full of liquid get the
//! oxidising gas only down their own gas spaces (Armstrong 1979), so wetter ground is not always better ground.

use pcore::cover::{self, Parts, Soak, GAS};
use pcore::World;
use worldgen::life::{KINDS, LIGHT_ROW};
use worldgen::testkit;

/// The row of the big slow kind of cover (a tree) among the starting kinds.
const TREE_ROW: usize = 2;
/// The key liquid a leaf loses a mol of gas it takes in, mol: a leaf in moist air (the transpiration ratio of real leaves
/// is some hundred in half-saturated air; Farquhar and Sharkey 1982), so the stand's fine roots cost a share of its gain.
const THIRST: f64 = 20.0;

/// The flows of the cover's budget (`cover::BUDGET`) up to here: what its light reaction stored and each part of its
/// upkeep.
const UPKEEP_END: usize = 8;

/// The genes of the light-eaters with the share `phi` of their roots' volume gas space.
fn herb(phi: f32) -> [f32; cover::NG] {
    let mut g = KINDS[LIGHT_ROW].genes;
    g[GAS] = phi;
    g
}

/// A small world after a month (the air near its ground found), a land cell of it and the cover's parts.
fn world() -> (World, usize, Parts) {
    let (mut w, i) = testkit::start();
    pcore::step(&mut w);
    let p = Parts::of(&w.chem).expect("cover parts");
    (w, i, p)
}

/// Up to field capacity every root breathes, whatever its gas spaces. In full pores a root works only as far down as
/// its gas spaces keep it in the gas (`cover::aired`), so a kind of few gas spaces is less active on full pores than on
/// ground at field capacity, a kind of many is more active there than it, and liquid standing as deep as a kind's
/// aired length leaves none of its roots working.
#[test]
fn roots_in_full_pores_breathe_only_through_their_gas_spaces() {
    let (w, i, p) = world();
    let ph = &w.cfg.phys;
    let breath = cover::breath(&w, &p, i, ph.t_cat);
    assert!(breath > 0.0, "no oxidising gas reaches the roots");
    let (need, loose) = (1.0, 1.0);
    let soak = |filled: f64| Soak { moist: ph.available(filled), filled, loose, breath, pond: 0.0 };
    // What it runs of its light reaction: its activity times the share of its roots that work (`grow::light`).
    let act = |phi: f32, filled: f64| cover::activity(&herb(phi), ph.available(filled)) * cover::working(phi as f64, need, &soak(filled), ph);
    let (few, many) = (0.05, 0.4);
    assert!(act(few, ph.field) == 1.0 && act(many, ph.field) == 1.0, "at field capacity: {} {}", act(few, ph.field), act(many, ph.field));
    let (low, high) = (act(few, 1.0), act(many, 1.0));
    println!("in full pores: {low:.3} with gas share {few}, {high:.3} with {many}; aired {:.3} and {:.3} m", cover::aired(breath, few as f64), cover::aired(breath, many as f64));
    assert!(low < 1.0 && low < high, "wetter ground is no worse: {low} against {high}");
    assert!((low - cover::aired(breath, few as f64) / need).abs() < 1e-12, "{low} of its roots work");
    let drowned = 1.0 + cover::aired(breath, many as f64) / (loose * ph.porosity);
    assert_eq!(act(many, drowned * (1.0 + 1e-9)), 0.0, "roots work under liquid deeper than their gas spaces reach");
}

/// Cold ground lets the gas reach farther down a root (`cover::breath`): its living tissue burns the gas at the
/// burning's pace, which falls in the cold faster than the gas held in the air rises, so a flood in a cold season
/// costs a kind less than one in a warm season.
#[test]
fn the_gas_reaches_farther_down_a_root_in_the_cold() {
    let (w, i, p) = world();
    let (cold, warm) = (cover::breath(&w, &p, i, 275.0), cover::breath(&w, &p, i, 300.0));
    let burn = &w.chem.reactions[p.burn];
    let law = burn.arrhenius(300.0) / burn.arrhenius(275.0) * 300.0 / 275.0;
    println!("aired at gas share 0.3: {:.3} m at 275 K, {:.3} m at 300 K", cover::aired(cold, 0.3), cover::aired(warm, 0.3));
    assert!((cold / warm / law - 1.0).abs() < 1e-9, "the reach in the cold is {} times the warm one's, the law says {law}", cold / warm);
    assert!(cover::aired(cold, 0.3) > cover::aired(warm, 0.3));
}

/// Gas spaces cost and pay (`upkeep::uptake`, `cover::working`): the same stand of trees with more of its roots'
/// volume gas space keeps less of what its light reaction stores past its upkeep over a warm half year on drained
/// ground, where its fine roots carry less liquid a kg, and more on ground whose pores are full, where its roots get the
/// gas. (Its leaves lose `THIRST` mol of the key liquid
/// a mol of the gas they take in, `cover::thirst`.)
#[test]
fn gas_spaces_cost_on_drained_ground_and_pay_on_full_pores() {
    let (mut w, i, p) = world();
    assert!(cover::place(&mut w, i, KINDS[TREE_ROW].genes, TREE_ROW as u32, 2.0));
    let month = w.cfg.calendar().month as f64;
    let breath = cover::breath(&w, &p, i, 295.0);
    let grown = |phi: f32, full: bool| {
        let mut c = w.life.cover(i).clone();
        c.held().collect::<Vec<_>>().into_iter().for_each(|x| c.genes[x][GAS] = phi);
        let mut counts = w.counts.clone();
        for m in 0..6u64 {
            let mut a = testkit::garden(&w, &c, (295.0, 1.0, 200.0, m));
            a.thirst = THIRST;
            if full {
                a.soak = Soak { moist: 1.0, filled: 1.0, loose: 1.0, breath, pond: 0.0 };
            }
            let _ = cover::live(&w, &mut c, &p, &a, (i, month), &mut counts);
        }
        counts.cover_budget[..UPKEEP_END].iter().sum::<f64>()
    };
    let (dry_few, dry_many, wet_few, wet_many) = (grown(0.05, false), grown(0.4, false), grown(0.05, true), grown(0.4, true));
    println!("stored past upkeep in half a year, J: drained {dry_few:.4e} (gas share 0.05) {dry_many:.4e} (0.4); full pores {wet_few:.4e} {wet_many:.4e}");
    assert!(dry_few > dry_many, "on drained ground the gas spaces cost nothing: {dry_few} against {dry_many}");
    assert!(wet_many > wet_few, "in full pores the gas spaces gain nothing: {wet_many} against {wet_few}");
}

/// A kind stands out into the liquid as far as its roots work there (`cover::footing`): in every cell of the small
/// world a kind of many gas spaces stands on at least the ground a kind of few stands on, and in some cell on more,
/// where liquid stands over ground shallower than its gas reaches down its roots; where it stands under the liquid its
/// roots work less than on full pores at the ground's top.
#[test]
fn a_kind_stands_out_into_the_liquid_as_far_as_its_roots_breathe() {
    let (w, _, p) = world();
    let ph = &w.cfg.phys;
    let (few, many) = (0.05, 0.6);
    let mut further = 0;
    for i in 0..w.mesh.cells().n() {
        let soak = cover::soak(&w, &p, i, w.g(p.liquid, i)).1;
        let (a, b) = (cover::footing(&w, i, few, ph.root_least, &soak), cover::footing(&w, i, many, ph.root_least, &soak));
        assert!(b.share >= a.share, "cell {i}: the kind of many gas spaces stands on {} of the ground, of few on {}", b.share, a.share);
        if b.share > a.share {
            further += 1;
            assert!(b.wet > a.wet && b.work > 0.0 && b.work <= 1.0, "cell {i}: {b:?} against {a:?}");
        }
    }
    println!("{further} cells where the kind of many gas spaces stands farther into the liquid");
    assert!(further > 0, "no cell holds ground under shallow liquid");
}
