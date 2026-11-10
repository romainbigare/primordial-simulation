//! What bodies do with and to each other: who eats whom, where they go (traffic and worn edges), imitation and
//! gifts.

use super::Row;
use crate::layers::Ctx;
use crate::measure::area_mean;
use crate::rec::Recorder;
use std::collections::BTreeSet;

/// The busiest cells are one in this many cells (the top 2%).
const TOP_PER: usize = 50;
/// Years between the looks back at the busiest cells and the worn edges.
const DECADE: u64 = 10;
/// An edge is worn where movers cleared it so that crossing costs under 1 / `WORN_EASE` of uncleared ground.
const WORN_EASE: f64 = 2.0;

impl Recorder {
    pub(super) fn social_cols(&mut self, c: &Ctx, land: &[usize], row: &mut Row) {
        let (w, gn) = (c.w, c.w.mesh.cells().n());
        let life = &w.life;
        // Bodies that took more from other bodies than from the cover in their life.
        let eat = life.alive().filter(|b| self.social.eaten(b.id).is_some_and(|t| t[1] > t[0])).count();
        row.push("eat_animals", "1", eat as f64);
        // Traffic: the share of crossings on the busiest cells, their overlap with the busiest a decade ago, and the
        // body mass per area there and on other land.
        let traffic = &w.counts.traffic;
        let mut order: Vec<usize> = (0..gn).filter(|&i| traffic.get(i).is_some_and(|&t| t > 0.0)).collect();
        order.sort_by(|&a, &b| traffic[b].total_cmp(&traffic[a]).then(a.cmp(&b)));
        let top = &order[..(gn / TOP_PER).min(order.len())];
        let all: f64 = order.iter().map(|&i| traffic[i] as f64).sum();
        let on_top: f64 = top.iter().map(|&i| traffic[i] as f64).sum();
        let (s, decade) = (&mut self.social, w.cfg.calendar().years(w.time).is_multiple_of(DECADE));
        // (Cells by their serials, `Life::serial`.)
        let serial = |i: usize| life.serial(i);
        if decade && !top.is_empty() {
            let old: BTreeSet<u32> = s.top.iter().copied().collect();
            s.persist = if old.is_empty() { f64::NAN } else { top.iter().filter(|&&i| old.contains(&serial(i))).count() as f64 / top.len() as f64 };
            s.top = top.iter().map(|&i| serial(i)).collect();
        }
        let cells = w.mesh.cells();
        let edge = |l: usize| {
            let (a, b) = (serial(cells.a()[l] as usize), serial(cells.b()[l] as usize));
            (a.min(b), a.max(b))
        };
        let mut now: Vec<(u32, u32)> = (0..w.links.cleared.len()).filter(|&l| pcore::lanes::eased(w, l) > WORN_EASE).map(edge).collect();
        now.sort_unstable();
        let fresh = now.iter().filter(|e| s.worn.binary_search(e).is_err()).count() as f64;
        if decade {
            s.worn_persist = if s.worn_old.is_empty() { f64::NAN } else { s.worn_old.iter().filter(|e| now.binary_search(e).is_ok()).count() as f64 / s.worn_old.len() as f64 };
            s.worn_old = now.clone();
        }
        s.worn = now;
        let mass = |i: usize| c.plants[i] + c.animals[i];
        let rest: Vec<usize> = land.iter().copied().filter(|i| !top.contains(i)).collect();
        let mean = |v: &[usize]| area_mean(v.iter().map(|&i| (mass(i), w.mesh.cells().area()[i])));
        row.push("crossings", "1/yr", all);
        row.push("traffic_top2", "1", on_top / all.max(1.0));
        row.push("traffic_persist", "1", s.persist);
        row.push("worn_edges", "1", s.worn.len() as f64);
        row.push("worn_new", "1/yr", fresh);
        row.push("worn_persist", "1", s.worn_persist);
        row.push("mass_top2", "kg/m2", mean(top));
        row.push("mass_land", "kg/m2", mean(&rest));
        // Imitation: watches (of them within a lineage), bodies watched and the most watchers of one.
        let y = &self.year;
        row.push("watched", "1/yr", y.watched);
        row.push("watched_kin", "1/yr", y.kin_watched);
        row.push("watched_bodies", "1", y.spread.len() as f64);
        row.push("spread_max", "bodies", y.spread.values().map(|v| v.len()).max().unwrap_or(0) as f64);
        // Gifts: puts into other bodies, kg, those by bodies still alive, giving pairs and pairs giving both ways.
        row.push("gave", "1/yr", y.gave);
        row.push("gave_kg", "kg/yr", y.gave_kg);
        row.push("gave_eaters", "1/yr", y.gave_eaters);
        row.push("gave_pairs", "1", y.pairs.len() as f64);
        row.push("gave_reciprocal", "1", y.pairs.iter().filter(|&&(a, b)| a < b && y.pairs.contains(&(b, a))).count() as f64);
        // What bodies could use of what they took, from each source.
        for (name, x) in ["ate_cover", "ate_others", "ate_ground"].into_iter().zip(y.intake) {
            row.push(name, "kg/yr", x);
        }
    }
}
