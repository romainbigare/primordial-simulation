//! The world: elements, minerals, energy, temperatures, liquid and rain, the bodies and their members, births and
//! deaths, dead matter, the heat-absorbing gases and the catalysts life carries.

use super::{Row, GIGA};
use crate::layers::{dry_land_flux, Ctx, COVER_MIN, RIVER_WEB};
use crate::measure::area_mean;
use crate::rec::{Recorder, CAUSES};
use pcore::{World, AIR};
use pcore::store::Class;

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

impl Recorder {
    pub(super) fn world_cols(&self, c: &Ctx, land: &[usize], row: &mut Row) {
        let w = c.w;
        let (life, area, air_area) = (&w.life, w.mesh.cells().area(), w.mesh.level(AIR).area());
        let tot = pcore::check::totals(w);
        row.push("year", "yr", w.year());
        row.push("null", "1", w.cfg.null as u8 as f64);
        for (j, &v) in tot.elems.iter().enumerate() {
            row.push(format!("elem_{j}"), "mol", v);
        }
        minerals(c, row);
        row.push("energy", "J", tot.thermal + tot.chemical);
        row.push("t_ground", "K", area_mean(c.f.t.iter().copied().zip(area.iter().copied())));
        row.push("t_air", "K", area_mean(c.t_air.iter().copied().zip(air_area.iter().copied())));
        // The land's ground temperature: the mean of the year's months, the warmest month and the coldest.
        let t_land = &self.year.t_land;
        row.push("t_land", "K", mean(t_land));
        row.push("t_land_summer", "K", t_land.iter().copied().fold(f64::NAN, f64::max));
        row.push("t_land_winter", "K", t_land.iter().copied().fold(f64::NAN, f64::min));
        // The mole fraction in the air of each heat-absorbing gas (`absorber` is their total absorption, so the
        // one that rises shows in these).
        row.push("absorber", "1", area_mean(c.absorber.iter().copied().zip(air_area.iter().copied())));
        let (na, all) = (w.mesh.level(AIR).n(), w.store.amounts(Class::Air).iter().sum::<f64>());
        for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0) {
            row.push(format!("air_{k}"), "1", w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().sum::<f64>() / all);
        }
        row.push("ice_frac", "1", area_mean(c.f.ice.iter().zip(area).map(|(&v, &a)| ((v >= COVER_MIN) as u8 as f64, a))));
        row.push("liquid_land", "km3", land.iter().map(|&i| c.f.depth[i] * area[i]).sum::<f64>() / GIGA);
        row.push("ice_land", "km3", land.iter().map(|&i| c.f.ice[i] * area[i]).sum::<f64>() / GIGA);
        row.push("rain", "mol/m2/s", area_mean(w.rain.iter().zip(area).map(|(&v, &a)| (v as f64, a))));
        row.push("rain_land", "mol/m2/s", area_mean(land.iter().map(|&i| (w.rain[i] as f64, area[i]))));
        // The land's water budget: what rises from it, and what runs over it into the sea, per m² of land.
        row.push("evap_land", "mol/m2/s", area_mean(land.iter().map(|&i| (w.evap[i] as f64, area[i]))));
        let land_area = land.iter().map(|&i| area[i]).sum::<f64>().max(f64::MIN_POSITIVE);
        row.push("runoff_sea", "mol/m2/s", land.iter().map(|&i| w.off_land[i] as f64).sum::<f64>() / land_area);
        // Rivers as the web viewer draws them.
        row.push("rivers", "1", dry_land_flux(w, &c.f).iter().filter(|&&q| q > RIVER_WEB).count() as f64);
        // Bodies (nodes and individuals) and light-eaters; members of light-eaters and of the others; nodes of the others.
        let (mut members, mut nodes) = ([0.0; 2], 0.0);
        for (s, b) in life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            members[!c.light[s] as usize] += b.n();
            nodes += (!c.light[s] && b.n() > 1.0) as u8 as f64;
        }
        row.push("bodies", "1", life.alive().count() as f64);
        row.push("plants", "1", c.light.iter().filter(|&&l| l).count() as f64);
        row.push("plant_members", "1", members[0]);
        row.push("animal_members", "1", members[1]);
        row.push("animal_nodes", "1", nodes);
        // Light-eater mass per m² on land: median and largest.
        let mut cover: Vec<f64> = land.iter().map(|&i| c.plants[i]).collect();
        cover.sort_by(|a, b| a.total_cmp(b));
        row.push("cover_median", "kg/m2", cover.get(cover.len() / 2).copied().unwrap_or(0.0));
        row.push("cover_max", "kg/m2", cover.last().copied().unwrap_or(0.0));
        row.push("body_mass", "t", life.alive().map(|b| b.mass(&w.chem)).sum::<f64>() / 1e3);
        dead(w, land, row);
        row.push("births", "1/yr", self.year.births);
        for (cause, &x) in CAUSES.iter().zip(&self.year.deaths) {
            row.push(format!("died_{cause}"), "1/yr", x);
        }
        row.push("lineages", "1", life.lineages() as f64);
        row.push("lineages_ever", "1", life.brains.len() as f64);
        row.push("updates", "1/yr", w.counts.updates.saturating_sub(self.updates) as f64);
        // The mean catalyst gene (log10 speed-up; below 0 counts as off, 0) of each reaction over the live members:
        // which reactions life speeds up.
        let (nr, cat) = (w.chem.reactions.len(), pcore::body::Layout::of(&w.chem).cat);
        let mut g = vec![0.0; nr];
        for b in life.alive() {
            g.iter_mut().enumerate().for_each(|(r, x)| *x += b.genes[cat + r].max(0.0) as f64 * b.n());
        }
        for (r, &x) in g.iter().enumerate() {
            row.push(format!("cat_{r}"), "log10", x / (members[0] + members[1]).max(1.0));
        }
    }
}

/// Each mineral (`Chemistry::minerals`): its mass in the ground per m² of land and of sea, and where it is in all, in
/// 10^9 mol of it (matter counted at its share of it): dead matter on the ground of land and of the sea, the bodies,
/// and the stores of the bed (`World::deep`).
fn minerals(c: &Ctx, row: &mut Row) {
    let w = c.w;
    let (n, area, matter, minerals) = (w.mesh.cells().n(), w.mesh.cells().area(), w.chem.matter(), w.chem.minerals());
    let mut bodies = vec![0.0f64; minerals.len()];
    for b in w.life.alive() {
        for (x, &(k, per)) in bodies.iter_mut().zip(minerals) {
            *x += b.store[k] + matter.map_or(0.0, |m| b.store[m]) * per;
        }
    }
    for (q, &(k, per)) in minerals.iter().enumerate() {
        // [ground, dead matter] x [land, sea], and the area of each
        let (mut t, mut areas) = ([[0.0f64; 2]; 2], [0.0f64; 2]);
        for (i, &a) in area.iter().enumerate() {
            let sea = (c.f.z[i] <= 0.0) as usize;
            areas[sea] += a;
            t[0][sea] += w.store.amounts(Class::Ground)[k * n + i];
            t[1][sea] += matter.map_or(0.0, |m| w.store.amounts(Class::Ground)[m * n + i]) * per;
        }
        let mm = w.chem.subs[k].molar_mass;
        row.push(format!("min{q}_land"), "kg/m2", t[0][0] * mm / areas[0].max(f64::MIN_POSITIVE));
        row.push(format!("min{q}_sea"), "kg/m2", t[0][1] * mm / areas[1].max(f64::MIN_POSITIVE));
        row.push(format!("min{q}_dead_land"), "1e9 mol", t[1][0] / GIGA);
        row.push(format!("min{q}_dead_sea"), "1e9 mol", t[1][1] / GIGA);
        row.push(format!("min{q}_bodies"), "1e9 mol", bodies[q] / GIGA);
        row.push(format!("min{q}_deep"), "1e9 mol", w.store.amounts(Class::Deep).get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) / GIGA);
    }
}

/// Dead fuel, matter and each solid bodies build on the ground of the land, kg/m².
fn dead(w: &World, land: &[usize], row: &mut Row) {
    let n = w.mesh.cells().n();
    let fuel = w.chem.build.as_ref().map(|b| b.inputs[0].0 as usize);
    for k in fuel.into_iter().chain(w.chem.matter()).chain(w.chem.materials()) {
        let kg = land.iter().map(|&i| w.store.amounts(Class::Ground)[k * n + i]).sum::<f64>() * w.chem.subs[k].molar_mass;
        row.push(format!("dead_{k}"), "kg/m2", kg / land.iter().map(|&i| w.mesh.cells().area()[i]).sum::<f64>().max(f64::MIN_POSITIVE));
    }
}
