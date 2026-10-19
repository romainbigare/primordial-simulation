//! Selection inside a node. A node's members are born and die in proportion whatever their genes, so without this
//! selection would act only between nodes. At each update a node without a brain also runs, as far as its growth, the
//! update of a sample member one standard deviation of its spread from its mean on one gene (`sample`), and the mean
//! of that gene moves by the sample's step times what the sample gained over the node (`select`): the breeder's
//! equation, the change of a mean is its genetic variance times the gradient of log fitness (Lande 1976; Falconer and
//! Mackay 1996), in the form the covariance of a gene with fitness gives for two halves a step either side of the
//! mean (Robertson 1966).

use crate::body::{gain_to_growth, range, spread_at, Body, Layout, LiveTmp, ALLOC, CHILD, COOL_T, DISPERSE, HIDDEN, KEEP_T, LEARN, MEMORY, MUT, SEEN, SIGHT, WET};
use crate::change::Reactor;
use crate::chem::Phase;
use crate::rng::{Purpose, Rng};
use crate::store::{Class, Node};
use crate::World;

/// A sample member of a node: the gene it differs in, its step from the node's mean, its weight (the number of genes it
/// was drawn from, times `Config::sample_every`), and the log of what it gained in the update.
pub(crate) struct Sample {
    gene: usize,
    step: f32,
    genes: f32,
    gain: f64,
}

/// The scalar genes an update as far as its growth never reads in a node without a brain: those of its children (read
/// when children are made, `copy::copy`, after the growth a sample runs to; the mutation size too) and those of a
/// brain. They move only by selection between nodes.
const UNREAD: [usize; 10] = [CHILD, MUT, DISPERSE, WET, ALLOC, LEARN, HIDDEN, MEMORY, SEEN, SIGHT];

/// A safe minimum is read only where its store's share of the body lies under this many times the larger end of the
/// step: further above, the update cannot bring the store down to it.
const MIN_REACH: f32 = 2.0;

/// Can a step from `x` to `y` of gene `g` change what node `b` (without a brain) gains as far as its growth? Not for
/// genes in `UNREAD`, genes off at both ends, or thresholds the step cannot reach. A skipped sample costs no update; a
/// wrong skip only slows that gene.
fn read(w: &World, lay: &Layout, b: &Body, g: usize, (x, y): (f32, f32)) -> bool {
    let (chem, i) = (&w.chem, b.cell as usize);
    let (hi, step) = (x.max(y) as f64, (y - x).abs() as f64);
    if g < lay.cat {
        if g == KEEP_T || g == COOL_T {
            let (tb, tg) = (b.temp(chem), w.t_ground_last(i));
            return if g == KEEP_T { tb.min(tg) < hi + step } else { tb.max(tg) > x.min(y) as f64 - step };
        }
        return !UNREAD.contains(&g);
    }
    if x <= 0.0 && y <= 0.0 {
        return false;
    }
    if g < lay.min {
        return true;
    }
    let roots = |k: usize| chem.subs[k].solubility <= 0.0;
    if g < lay.upt {
        let k = g - lay.min;
        return b.store[k] * chem.subs[k].molar_mass < (MIN_REACH * x.max(y)) as f64 * b.size(chem);
    }
    if g < lay.keep {
        let k = g - lay.upt;
        return roots(k) && (x > 0.0) != (y > 0.0) && chem.phase_in(k, w.t_ground_last(i), |j| w.g(j, i)) == Phase::Liquid && w.g(k, i) > 0.0;
    }
    let (k, tg) = (g - lay.keep, w.t_ground_last(i));
    let product = b.store[k] > 0.0 && chem.phase(k, tg) != Phase::Gas && chem.reactions.iter().enumerate().any(|(r, rx)| b.genes[lay.cat + r] > 0.0 && rx.outputs.iter().any(|p| p.0 as usize == k));
    let liquid = chem.phase_in(k, tg, |j| w.g(j, i)) == Phase::Liquid && w.g(k, i) > 0.0;
    let taken = (!roots(k) || b.genes[lay.upt + k] > 0.0) && liquid && b.store[k] < b.level_at(chem, k, b.mass(chem), hi);
    chem.is_material(k) || b.wet >> k & 1 == 1 || product || taken
}

/// The sample member of node `b` (without a brain, taken out of slot `slot`, before its update): its genes but one, a
/// standard deviation of its spread up or down, run as far as its growth on the world as it is, its moves and events
/// then dropped. None where the step cannot change the gain (`read`); run on one update in `Config::sample_every`.
pub(crate) fn sample(w: &mut World, slot: usize, b: &Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> Option<Sample> {
    if b.n() <= 1.0 || !w.life.brainless(b) {
        return None;
    }
    let every = w.cfg.sample_every as usize;
    let mut r = Rng::new(w.seed, w.day(), b.id(slot), Purpose::Select);
    if every > 1 && r.below(every) != 0 {
        return None;
    }
    let lay = Layout::of(&w.chem);
    let differ: Vec<usize> = (0..b.genes.len()).filter(|&g| spread_at(&b.spread, g) > 0.0).collect();
    let g = *differ.get(r.below(differ.len().max(1)))?;
    let ((_, lo, hi), x, sd) = (range(&lay, g), b.genes[g], spread_at(&b.spread, g).sqrt());
    let step = (if r.below(2) == 0 { x + sd } else { x - sd }).clamp(lo, hi) - x;
    if step == 0.0 || !read(w, &lay, b, g, (x, x + step)) {
        return None;
    }
    let mut s = b.clone();
    let mut genes = b.genes.clone();
    genes[g] += step;
    s.set_genes(genes, &w.chem, &w.cfg.phys);
    let events = w.events.len();
    let gain = gain_to_growth(w, slot, &mut s, rc, tmp);
    w.open_moves(b.cell as usize, Node::new(Class::Body, slot));
    w.events.truncate(events);
    Some(Sample { gene: g, step, genes: (differ.len() * every) as f32, gain })
}

/// The mean of node `b` after its update, in which it gained `gain`: the sample's gene moves by its step times
/// tanh(sample's gain - gain) times the sample's weight, at most `sample_every` steps (Robertson 1966). Within the
/// walls.
pub(crate) fn select(w: &mut World, b: &mut Body, s: Sample, gain: f64) {
    let f = (s.gain - gain).tanh() as f32;
    if f == 0.0 || !f.is_finite() {
        return;
    }
    let (_, lo, hi) = range(&Layout::of(&w.chem), s.gene);
    let mut genes = b.genes.clone();
    let cap = w.cfg.sample_every as f32 * s.step.abs();
    genes[s.gene] = (genes[s.gene] + (s.genes * s.step * f).clamp(-cap, cap)).clamp(lo, hi);
    b.set_genes(genes, &w.chem, &w.cfg.phys);
}
