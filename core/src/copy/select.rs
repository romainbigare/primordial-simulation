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
use crate::World;

/// A sample member of a node: the gene it differs in, its step from the node's mean, its weight (the number of genes it
/// was drawn from, times `Config::sample_every`), and the log of what it gained in the update.
pub(crate) struct Sample {
    gene: usize,
    step: f32,
    genes: f32,
    gain: f64,
}

/// What an update of a body in ground cell `i` and air node `j`, in slot `slot` of generation `gen`, may change in the
/// world (the stores and heat of its ground and air nodes, the potential energy, the light and heat set aside for its
/// bodies and for its slot, the heat a ground node could not give, the events), kept to give back after a sample's.
struct Shadow {
    i: usize,
    j: usize,
    slot: usize,
    gen: u32,
    ground: Vec<f64>,
    air: Vec<f64>,
    heat: [f64; 6],
    keep: [f32; 2],
    events: usize,
}

impl Shadow {
    fn of(w: &World, slot: usize, b: &Body) -> Shadow {
        let (i, ns, n, na) = (b.cell as usize, w.chem.ns(), w.mesh.cells().n(), w.mesh.level(crate::AIR).n());
        let j = w.mesh.up(i, 0, crate::AIR);
        let heat = [w.e_ground[i], w.e_air[j], w.potential, w.light_pool[i], w.heat_pool.get(i).copied().unwrap_or(0.0), w.counts.clamped_heat];
        let keep = [w.light_keep.get(slot, b.gen), w.heat_keep.get(slot, b.gen)];
        let (ground, air) = ((0..ns).map(|k| w.ground[k * n + i]).collect(), (0..ns).map(|k| w.air[k * na + j]).collect());
        Shadow { i, j, slot, gen: b.gen, ground, air, heat, keep, events: w.events.len() }
    }
    fn restore(self, w: &mut World) {
        let (i, j, n, na) = (self.i, self.j, w.mesh.cells().n(), w.mesh.level(crate::AIR).n());
        for (k, (&g, &a)) in self.ground.iter().zip(&self.air).enumerate() {
            (w.ground[k * n + i], w.air[k * na + j]) = (g, a);
        }
        let pool = self.heat[4];
        [w.e_ground[i], w.e_air[j], w.potential, w.light_pool[i], _, w.counts.clamped_heat] = self.heat;
        if let Some(h) = w.heat_pool.get_mut(i) {
            *h = pool;
        }
        w.light_keep.set(self.slot, self.gen, self.keep[0]);
        w.heat_keep.set(self.slot, self.gen, self.keep[1]);
        w.events.truncate(self.events);
    }
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
/// (`body::gain_to_growth`) on the world as it is, which is then given back as it was: a member that might be, not one
/// that is, so it changes nothing. None for other bodies, and where the step cannot change what the node gains in the
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
    let shadow = Shadow::of(w, slot, b);
    let gain = gain_to_growth(w, slot, &mut s, rc, tmp);
    shadow.restore(w);
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
