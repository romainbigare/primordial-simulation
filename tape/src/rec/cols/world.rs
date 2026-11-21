//! The world: elements, minerals, energy, temperatures, liquid and rain, the cover, dead matter and the heat-absorbing
//! gases.

use super::{Row, GIGA, MEGA};
use crate::layers::{river_count, river_flux, Ctx, COVER_MIN};
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
        let (area, air_area) = (w.mesh.cells().area(), w.mesh.level(AIR).area());
        // What the world holds in all (`check::totals`).
        let tot = pcore::check::totals(w);
        row.push("year", "yr", w.year());
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
        for k in (0..w.chem.ns()).filter(|&k| w.chem.absorbs_heat(k)) {
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
        // The bodies of standing liquid above the open low ground that hold some (`World::lakes`): how many, the area
        // their liquid covers and its volume; and the level of the open low ground's largest body.
        let lakes = w.pools();
        let up: Vec<_> = lakes.iter().filter(|l| !l.low && l.volume > 0.0).collect();
        row.push("lakes", "1", up.len() as f64);
        row.push("lake_area", "km2", up.iter().map(|l| l.area).sum::<f64>() / MEGA);
        row.push("lake_m3", "km3", up.iter().map(|l| l.volume).sum::<f64>() / GIGA);
        let low = lakes.iter().filter(|l| l.low).max_by(|a, b| a.area.total_cmp(&b.area));
        row.push("low_level", "m", low.map_or(f64::NAN, |l| l.level));
        // What crossed into the open low ground at the map's open edges since the start, and its heat.
        row.push("edge_liquid", "km3", w.counts.edge_liquid[0] / GIGA);
        row.push("edge_liquid_heat", "PJ", w.counts.edge_liquid[1] / 1e15);
        // The load of the running liquid since the start: picked up, into the open low ground, into the other bodies,
        // settled on land, the bed worn into loose ground; at the builds, what the lobes found no room for and what
        // their moves lacked.
        for (name, v) in ["load_picked", "load_sea", "load_lakes", "load_land", "bed_worn", "load_unlaid", "load_short"].iter().zip(w.counts.load) {
            row.push(*name, "Mm3", v / MEGA);
        }
        // Rivers as the web viewer draws them.
        row.push("rivers", "1", river_count(w, &c.f) as f64);
        row.push("river_flux", "mol/s", river_flux(w, &c.f));
        // The cells.
        row.push("cells", "1", w.mesh.cells().n() as f64);
        // The slots of cover and its mass.
        let cover = pcore::cover::sums(w);
        row.push("cover_slots", "1", cover.iter().map(|s| s.slots).sum::<f64>());
        row.push("cover_kg", "kg", cover.iter().map(|s| s.kg).sum::<f64>());
        // The cover that died in the year, kg, by cause (taken by a taker: `taken`; pressed loose: `broken`).
        for (cause, &x) in CAUSES.iter().zip(&self.year.cover_lost) {
            row.push(format!("cover_died_{cause}"), "kg/yr", x);
        }
        // The flows of the cover's budget in the year, + into the cover (`cover::BUDGET`).
        for (flow, &x) in pcore::cover::BUDGET.iter().zip(&self.year.cover_budget) {
            row.push(format!("cover_budget_{flow}"), "J/yr", x);
        }
        // The seeds lying on the ground, kg, and their flows in the year, kg/yr (`cover::SEED_FLOWS`).
        row.push("seeds_kg", "kg", pcore::cover::lying(w).iter().zip(w.mesh.cells().area()).map(|(s, a)| s * a).sum::<f64>());
        for (flow, &x) in pcore::cover::SEED_FLOWS.iter().zip(&self.year.seeds) {
            row.push(format!("seeds_{flow}"), "kg/yr", x);
        }
        // The flames of the year: the share of the land they burnt in the last year's months (`flame::burnt_year`) over
        // the land the flames burn on, every cell's (`contract::land`), what they burnt and killed, and what they gave the
        // air of each gas.
        let on = |i: usize| pcore::contract::land(w, i);
        let land_m2: f64 = (0..w.mesh.cells().n()).map(on).sum();
        let burnt: f64 = (0..w.mesh.cells().n()).map(|i| pcore::flame::burnt_year(w, i) * on(i)).sum();
        row.push("burnt_land", "1/yr", burnt / land_m2.max(f64::MIN_POSITIVE));
        for (flow, &x) in pcore::flame::FLOWS.iter().zip(&self.year.flame) {
            row.push(format!("flame_{flow}"), if *flow == "plants" { "1/yr" } else { "kg/yr" }, x);
        }
        for k in (0..w.chem.ns()).filter(|&k| w.chem.gas_at_surface(k)) {
            row.push(format!("flame_air_{k}"), "kg/yr", self.year.flame_air.get(k).copied().unwrap_or(0.0) * w.chem.subs[k].molar_mass);
        }
        // The cover's mass per m² on land: median and largest.
        let mut cover: Vec<f64> = land.iter().map(|&i| c.plants[i]).collect();
        cover.sort_by(|a, b| a.total_cmp(b));
        row.push("cover_median", "kg/m2", cover.get(cover.len() / 2).copied().unwrap_or(0.0));
        row.push("cover_max", "kg/m2", cover.last().copied().unwrap_or(0.0));
        dead(w, land, row);
    }
}

/// Each mineral (`Chemistry::minerals`): its mass in the ground per m² of land and of sea, and where it is in all, in
/// 10^9 mol of it (matter counted at its share of it, and what particles hold, `Substance::bound`, at what its rot
/// gives back of it, `held_mineral`): dead matter on the ground of land and of the sea, the cover, and the stores of
/// the bed (`World::deep`).
fn minerals(c: &Ctx, row: &mut Row) {
    let w = c.w;
    let (n, area, matter, minerals) = (w.mesh.cells().n(), w.mesh.cells().area(), w.chem.matter(), w.chem.minerals());
    let cover = pcore::cover::totals(w);
    for (q, &(k, per)) in minerals.iter().enumerate() {
        // [ground, dead matter] x [land, sea], and the area of each
        let (mut t, mut areas) = ([[0.0f64; 2]; 2], [0.0f64; 2]);
        let held = held_mineral(w, k);
        for (i, &a) in area.iter().enumerate() {
            let sea = (!w.is_land(i)) as usize;
            areas[sea] += a;
            t[0][sea] += w.store.amounts(Class::Ground)[k * n + i];
            t[1][sea] += matter.map_or(0.0, |m| w.store.amounts(Class::Ground)[m * n + i]) * per;
            t[1][sea] += held.iter().map(|&(b, per_b)| w.store.amounts(Class::Ground)[b * n + i] * per_b).sum::<f64>();
        }
        let mm = w.chem.subs[k].molar_mass;
        row.push(format!("min{q}_land"), "kg/m2", t[0][0] * mm / areas[0].max(f64::MIN_POSITIVE));
        row.push(format!("min{q}_sea"), "kg/m2", t[0][1] * mm / areas[1].max(f64::MIN_POSITIVE));
        row.push(format!("min{q}_dead_land"), "1e9 mol", t[1][0] / GIGA);
        row.push(format!("min{q}_dead_sea"), "1e9 mol", t[1][1] / GIGA);
        row.push(format!("min{q}_cover"), "1e9 mol", (cover[k] + matter.map_or(0.0, |m| cover[m]) * per) / GIGA);
        row.push(format!("min{q}_deep"), "1e9 mol", w.store.amounts(Class::Deep).get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) / GIGA);
    }
}

/// Of each substance particles hold (`Substance::bound`), the mol of mineral `k` its rot gives back per mol: the first
/// reaction that takes it and gives the mineral.
fn held_mineral(w: &World, k: usize) -> Vec<(usize, f64)> {
    let bound = (0..w.chem.ns()).filter(|&b| w.chem.subs[b].bound);
    let back = |b: usize| w.chem.reactions.iter().find_map(|r| {
        let took = r.inputs.iter().find(|p| p.0 as usize == b)?.1 as f64;
        Some(r.outputs.iter().find(|p| p.0 as usize == k)?.1 as f64 / took)
    });
    bound.filter_map(|b| back(b).map(|per| (b, per))).collect()
}

/// Dead fuel, matter, each solid life builds and what particles hold (`Substance::bound`) on the ground of the land,
/// kg/m².
fn dead(w: &World, land: &[usize], row: &mut Row) {
    let n = w.mesh.cells().n();
    let fuel = w.chem.build.as_ref().map(|b| b.inputs[0].0 as usize);
    let bound = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].bound);
    for k in fuel.into_iter().chain(w.chem.matter()).chain(w.chem.materials()).chain(bound) {
        let kg = land.iter().map(|&i| w.store.amounts(Class::Ground)[k * n + i]).sum::<f64>() * w.chem.subs[k].molar_mass;
        row.push(format!("dead_{k}"), "kg/m2", kg / land.iter().map(|&i| w.mesh.cells().area()[i]).sum::<f64>().max(f64::MIN_POSITIVE));
    }
}
