//! Brains and kinds: how far learning moved the brains, how often the best-scored action was drawn, the brains'
//! sizes, how much signals say about their sender's state, and the kinds of bodies by their genes and diet.

use super::Row;
use crate::measure;
use crate::rec::tally::Signals;
use crate::rec::Recorder;
use pcore::World;
use std::f64::consts::LN_2;

/// Fewest signals of a lineage in a context for its information to count.
const MIN_SIGNALS: f64 = 20.0;

/// Mutual information (bits) between a binary context and the signal quarter, given lineage, less its small-sample
/// bias, (2 - 1)(4 - 1) / (2 n ln 2) for 2 contexts and 4 quarters (Treves and Panzeri 1995).
fn info(sig: &[Signals], c: usize) -> f64 {
    let (mut sum, mut all) = (0.0, 0.0);
    for t in sig {
        let j = &t[c];
        let n: f64 = j.iter().flatten().sum();
        if n < MIN_SIGNALS {
            continue;
        }
        let pc = [0, 1].map(|a| j[a].iter().sum::<f64>() / n);
        let ps: Vec<f64> = (0..4).map(|q| (j[0][q] + j[1][q]) / n).collect();
        let mut i = 0.0;
        for a in 0..2 {
            for q in 0..4 {
                let p = j[a][q] / n;
                if p > 0.0 {
                    i += p * (p / pc[a] / ps[q]).log2();
                }
            }
        }
        sum += n * (i - 3.0 / (2.0 * n * LN_2)).max(0.0);
        all += n;
    }
    sum / all.max(1.0)
}

impl Recorder {
    pub(super) fn brain_cols(&self, w: &World, row: &mut Row) {
        self.learning(w, row);
        self.variety(w, row);
        // What signals say about cold ground, harm met and low well-being.
        for (c, name) in ["mi_cold", "mi_hurt", "mi_low"].into_iter().enumerate() {
            row.push(name, "bit", info(&self.year.sig, c));
        }
    }

    /// How far learning has moved the brains from their start, how often the best-scored action was the one drawn,
    /// and the mean sizes of the live bodies' brains and senses (hidden units, memory numbers, bodies perceived,
    /// sight) with the largest hidden layer.
    fn learning(&self, w: &World, row: &mut Row) {
        let [n, g] = self.year.decisions;
        let (mut sz, mut top, mut k) = ([0.0; 4], 0.0f64, 0.0);
        for l in w.life.alive().filter_map(|b| w.life.brains[b.lineage as usize].as_ref()) {
            let x = pcore::choose::sizes_of(l);
            sz.iter_mut().zip(x).for_each(|(s, v)| *s += v as f64);
            (top, k) = (top.max(x[0] as f64), k + 1.0);
        }
        let sz = sz.map(|v| v / f64::max(k, 1.0));
        row.push("learned", "1", measure::learned(w));
        row.push("greedy", "1", g / n.max(1.0));
        for (name, v) in ["hidden", "memory", "seen"].into_iter().zip(sz) {
            row.push(name, "1", v);
        }
        row.push("sight", "cells", sz[3]);
        row.push("hidden_max", "1", top);
    }

    /// Kinds of live bodies by their genes and diet (`measure::kinds`: `kinds` counts each body once,
    /// `kinds_members` each member), the mass range of their members (kg), and the share of what a kind took from
    /// other bodies that came from bodies that are not light-eaters: for the kind with most bodies, and the highest
    /// of any kind.
    fn variety(&self, w: &World, row: &mut Row) {
        let (of, k) = measure::kinds(w);
        let live: Vec<(usize, &pcore::body::Body)> = w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive).collect();
        // Each member's mass with the starting kind of its lineage (NaN: none), the heaviest last.
        let mut heavy: Vec<(f64, f64)> = live.iter().map(|(_, b)| (b.mass(&w.chem) / b.n(), w.life.brains[b.lineage as usize].as_ref().map_or(f64::NAN, |l| l.root as f64))).collect();
        heavy.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mass: Vec<f64> = heavy.iter().map(|m| m.0).collect();
        let at = |q: usize| mass.get(q).copied().unwrap_or(f64::NAN);
        let (mut cnt, mut took) = (vec![0usize; k], vec![[0f64; 2]; k]);
        for (&j, (i, b)) in of.iter().zip(&live) {
            cnt[j] += 1;
            let t = self.social.eaten(b.id(*i)).unwrap_or([0.0; 2]);
            (took[j][0], took[j][1]) = (took[j][0] + t[0] as f64, took[j][1] + t[1] as f64);
        }
        let share = |j: usize| took[j][1] / (took[j][0] + took[j][1]).max(1e-9);
        let big = (0..k).max_by(|&a, &b| cnt[a].cmp(&cnt[b]).then(b.cmp(&a)));
        let none = live.is_empty();
        row.push("kinds", "1", if none { 0.0 } else { k as f64 });
        row.push("kinds_members", "1", if none { 0.0 } else { measure::kinds_by_members(w) as f64 });
        row.push("mass_min", "kg", at(0));
        row.push("mass_median", "kg", at(mass.len() / 2));
        row.push("mass_max", "kg", at(mass.len().wrapping_sub(1)));
        row.push("mass_max_root", "1", heavy.last().map_or(f64::NAN, |m| m.1));
        row.push("trophic_max", "1", big.map_or(f64::NAN, share));
        row.push("trophic_high", "1", (0..k).map(share).fold(0.0, f64::max));
    }
}
