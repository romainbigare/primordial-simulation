//! Selection inside a node. A node's members are born and die in proportion whatever their genes, so without this
//! selection would act only between nodes. At each update a node without a brain also runs, as far as its growth, the
//! update of a sample member one standard deviation of its spread from its mean on one gene (`sample`), and the mean
//! of that gene moves by the sample's step times what the sample gained over the node (`select`): the breeder's
//! equation, the change of a mean is its genetic variance times the gradient of log fitness (Lande 1976; Falconer and
//! Mackay 1996), in the form the covariance of a gene with fitness gives for two halves a step either side of the
//! mean (Robertson 1966).

use crate::body::{gain_to_growth, range, spread_at, Body, Layout, LiveTmp, ALLOC, CHILD, DISPERSE, HIDDEN, LEARN, MEMORY, MUT, SEEN, SIGHT, WET};
use crate::change::Reactor;
use crate::chem::Chemistry;
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

/// Can a step from `x` to `y` of gene `g` change what node `b` (without a brain) gains in its update as far as its
/// growth? Not for a scalar gene in `UNREAD`, nor for a gene of a reaction or a substance that is off (at or below 0)
/// at both ends, which does nothing. Of the uptake genes the update reads only whether one is on, for the roots, which
/// take what does not dissolve (`physics::roots`). A keep gene sets the level of its substance (`Body::level`), which
/// the update reads only for the body matter and the solids it builds, its key liquids (their share, activity and
/// soaking), the products of the reactions it catalyses (their inhibition, `Body::inhibited`), the liquids that
/// dissolve, which it soaks up, and what its roots take.
fn read(chem: &Chemistry, lay: &Layout, b: &Body, g: usize, x: f32, y: f32) -> bool {
    if g < lay.cat {
        return !UNREAD.contains(&g);
    }
    if x <= 0.0 && y <= 0.0 {
        return false;
    }
    if g < lay.upt {
        return true;
    }
    let roots = |k: usize| chem.subs[k].solubility <= 0.0;
    if g < lay.keep {
        return roots(g - lay.upt) && (x > 0.0) != (y > 0.0);
    }
    let k = g - lay.keep;
    let product = chem.reactions.iter().enumerate().any(|(r, rx)| b.genes[lay.cat + r] > 0.0 && rx.outputs.iter().any(|p| p.0 as usize == k));
    chem.matter() == Some(k) || chem.is_material(k) || b.wet >> k & 1 == 1 || !roots(k) || b.genes[lay.upt + k] > 0.0 || product
}

/// The sample member of node `b` (slot `slot`, taken out of it, before its update; a node without a brain whose members
/// differ): the node's genes but one, drawn among the genes its members differ in, one standard deviation of its spread
/// up or down (the side drawn too, `Purpose::Select`), within its walls; its update as far as its growth
/// (`body::gain_to_growth`) on the world as it is, whose moves are then dropped and its events forgotten: a member that
/// might be, not one that is, so it changes nothing (it runs first in its node's update, when its node's moves hold no
/// move yet). None for other bodies, and where the step cannot change what the node gains in the
/// update (`read`), so its sample would gain what the node gains and its mean would not move. That draw costs no
/// update, and each gene is still drawn as often, so the others move at their own rate. A sample is run on one update
/// in `Config::sample_every`, drawn, and its weight is that many times larger, so each gene moves at the same mean rate.
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
    if step == 0.0 || !read(&w.chem, &lay, b, g, x, x + step) {
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

/// The mean of node `b` (taken out of its slot) after its update, in which its members gained `gain` (the log of what
/// they gained, `body::Lived::gain`), and its sample member `s` gained `s.gain`: the sample's gene moves by its step
/// d times tanh(s.gain - gain), times the sample's weight (each gene is drawn that much less often, so each moves at
/// its own rate), and at most by d times `Config::sample_every` (the one update in that many with a sample stands for
/// them all, so the mean moves as much on average). For two halves of a node at the mean plus and minus d, gaining W+
/// and W-, the covariance of the gene with relative fitness moves the mean by d (W+ - W-) / (W+ + W-) =
/// d tanh((ln W+ - ln W-) / 2) (Robertson 1966); the half difference of the logs is taken here as the sample's over
/// the node's own, and the side of the step is drawn, so the error of that one-sided difference (of second order)
/// averages out. A gene that does nothing gains nothing and does not move. Within the walls.
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
