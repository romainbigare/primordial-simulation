//! The big-brain test: copies of living eaters, with a big brain and with their own (the control).

use super::book_body;
use super::kinds::KINDS;
use pcore::body::{self, Body};
use pcore::lineage::Lineage;
use pcore::rng::{Purpose, Rng};
use pcore::{brain, lineage, World};

/// The root labels of the big-brain lineage `seed_clever` places and of its control, past the rows of `KINDS`: each
/// is its own kind on the tape.
pub const CLEVER_ROOT: u32 = KINDS.len() as u32;
pub const CONTROL_ROOT: u32 = CLEVER_ROOT + 1;
/// The brain sizes of the big-brain lineage: hidden units, memory numbers, bodies perceived, sight.
pub const CLEVER_SIZES: [usize; 4] = [48, 12, 6, 3];
/// The root of the ordinary eaters copied (`KINDS[1]`).
const EATERS: u32 = 1;
/// The random streams of the test (with `Purpose::Clever`): the big brain's base of each source lineage, the delay of
/// each pair, and the big brain's born rows of each copy.
const REBASE_STREAM: u64 = 1 << 40;
const DELAY_STREAM: u64 = 2 << 40;
const REROW_STREAM: u64 = 3 << 40;

/// A fair test of a big brain (`ew run --clever-year`): copy `n` living ordinary eaters (an even sample of the bodies
/// of the eaters' root, one member of each; if there are fewer, some are copied again) twice, into the same cells
/// with the same stores per member. One set gets the big brain (`CLEVER_SIZES`; the lineage's base and the born rows
/// carried over by `brain::rebase` and `brain::rerow`, as `lineage::resize` does), a new lineage of root `CLEVER_ROOT`;
/// the other keeps its sizes, a new lineage of root `CONTROL_ROOT`. Genes and born rows are the source's, no brain
/// bias is set by hand, and the sets differ in brain size only (and what that costs). The copies are new matter,
/// booked like an edit. Returns what was placed, in one line.
pub fn seed_clever(w: &mut World, n: usize) -> String {
    let src: Vec<usize> = (0..w.life.bodies.len())
        .filter(|&s| {
            let b = &w.life.bodies[s];
            b.alive && w.life.root(b) == EATERS && !body::light_eater(&w.chem, &b.genes)
        })
        .collect();
    if src.is_empty() || n == 0 {
        return format!("year {:.1}: no living ordinary eaters to copy for the big-brain test", w.year());
    }
    let (ns, noise) = (w.chem.ns(), w.cfg.phys.new_noise);
    let big = lineage::shape(ns, CLEVER_SIZES[0], CLEVER_SIZES[1]);
    // The two new lineages made from each source lineage: (source, big, control).
    let mut made: Vec<(u32, u32, u32)> = Vec::new();
    let (mut members, mut kg, mut sources) = (0.0f64, 0.0f64, 0usize);
    let mut last = usize::MAX;
    for j in 0..n {
        let s = src[j * src.len() / n];
        sources += (s != last) as usize;
        last = s;
        let b = &w.life.bodies[s];
        let (cell, lin, m) = (b.cell as usize, b.lineage, b.n());
        let (genes, born, wear, wet) = (b.genes.clone(), w.life.born(b).to_vec(), if m > 1.0 { 0.0 } else { b.wear }, b.wet);
        let (store, e): (Vec<f64>, f64) = (b.store.iter().map(|a| a / m).collect(), b.e / m);
        let src_lin = w.life.brains[lin as usize].clone().expect("lineage");
        if !made.iter().any(|x| x.0 == lin) {
            let mut r = Rng::new(w.seed, w.day(), REBASE_STREAM | lin as u64, Purpose::Clever);
            let base = brain::rebase(&src_lin.base, src_lin.shape, big, &mut r, noise);
            let a = w.life.new_lineage(Lineage { shape: big, seen: CLEVER_SIZES[2], sight: CLEVER_SIZES[3], base, root: CLEVER_ROOT });
            let c = w.life.new_lineage(Lineage { root: CONTROL_ROOT, ..src_lin.clone() });
            made.push((lin, a, c));
        }
        let (_, a, c) = *made.iter().find(|x| x.0 == lin).expect("made");
        let mut r = Rng::new(w.seed, w.day(), DELAY_STREAM | j as u64, Purpose::Clever);
        for (lineage, is_big) in [(a, true), (c, false)] {
            let (mut g, mut rows) = (genes.clone(), born.clone());
            if is_big {
                for (k, gene) in body::BRAIN_SIZES.into_iter().enumerate() {
                    g[gene] = CLEVER_SIZES[k] as f32;
                }
                rows = brain::rerow(&born, src_lin.shape, big, &mut Rng::new(w.seed, w.day(), REROW_STREAM | j as u64, Purpose::Clever), noise);
            }
            let group = w.life.new_group(lineage, rows.clone());
            let genome = w.life.new_genome(rows);
            let mut b = Body { cell: cell as u32, count: 1.0, lineage, group, near: u32::MAX, mark: w.light[cell], e, store: store.clone(), genes: g, genome, wear, wet, born: w.day(), ..Default::default() };
            b.vis = b.visible(&w.chem, &w.cfg.phys);
            book_body(&mut w.ledger, &w.chem, &b);
            if is_big {
                (members, kg) = (members + 1.0, kg + b.mass(&w.chem));
            }
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day(), delay);
        }
    }
    format!(
        "year {:.1}: placed {n} copies of living ordinary eaters (from {sources} bodies of the eaters' root) twice, in the same cells with the same stores ({:.1} kg a member, {kg:.3e} kg for each set), same genes and born rows, no hand-set biases: as a big-brain lineage (root {CLEVER_ROOT}: {} hidden, {} memory, {} seen, sight {}; rows carried over by rerow) and as its control (root {CONTROL_ROOT}: brain sizes unchanged)",
        w.year(),
        kg / members.max(1.0),
        CLEVER_SIZES[0],
        CLEVER_SIZES[1],
        CLEVER_SIZES[2],
        CLEVER_SIZES[3]
    )
}
