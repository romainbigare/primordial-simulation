//! Where a node's drifting rain lands (laws.md 1.5): what condenses and does not land where it condenses drifts with the
//! low wind for `rain_drift_s` while its drops form and fall (the linear theory of rain over the ground, Smith and
//! Barstad 2004), across the nodes' borders, so no cell's rain ends where the lattice of the air's nodes does.

use crate::{World, AIR};

/// Points a drift's path is sampled at: the middles of equal shares of an exponential along the wind.
const ALONG: usize = 16;

/// Per air node, where what it does not land where it condenses goes (`Seen::landing`), from the places that see each
/// node and the share of its air over each (`own`) and the low wind over each (`wind`, `rise::low_wind`): the places
/// only the drift reaches, after the node's own, and the share of the drift each entry gets, summing to one. The drift
/// leaves the node's centre and lands along its wind by an exponential of mean the wind's speed times `rain_drift_s`;
/// each point of its path lays its share over the nodes round it as a cell reads them (bilinear, by the nodes'
/// spacing), and each node over the places that see it by its air's share of each. A share whose path leaves the map
/// lands on the map by the rest's shares, so all of it lands. With no wind it stays with the node.
pub(super) fn drift(w: &World, wind: &[[f64; 2]], own: &[(Vec<u32>, Vec<f64>)]) -> Vec<(Vec<u32>, Vec<f64>)> {
    crate::par::map_n(w.mesh.level(AIR).n(), crate::par::UNCOUNTED, |q| {
        let nodes = path(w, q, wind[q]);
        let (place, _) = &own[q];
        let mut entry: std::collections::HashMap<u32, usize> = place.iter().enumerate().map(|(e, &p)| (p, e)).collect();
        let (mut beyond, mut share) = (Vec::new(), vec![0.0; place.len()]);
        for (k, wk) in nodes {
            let (places, of) = &own[k];
            let all: f64 = of.iter().sum::<f64>().max(f64::MIN_POSITIVE);
            for (&p, &x) in places.iter().zip(of) {
                let e = *entry.entry(p).or_insert_with(|| {
                    beyond.push(p);
                    share.push(0.0);
                    share.len() - 1
                });
                share[e] += wk * x / all;
            }
        }
        (beyond, share)
    })
}

/// The nodes node `q`'s drift lands on under its low wind `u`, m/s [east, south], with the share of it each gets
/// (summing to one): the points of its path (the middles of `ALONG` equal shares of an exponential of mean the wind's
/// speed times `rain_drift_s`, along the wind from the node's centre), each laid bilinearly on the nodes round it by
/// their spacing; with no wind, the node itself.
fn path(w: &World, q: usize, u: [f64; 2]) -> Vec<(usize, f64)> {
    let (air, speed) = (w.mesh.level(AIR), u[0].hypot(u[1]));
    if speed <= 0.0 {
        return vec![(q, 1.0)];
    }
    let (dir, mean) = ([u[0] / speed, u[1] / speed], speed * w.cfg.rain_drift_s);
    let pts: Vec<[f64; 2]> = (0..ALONG).map(|i| -mean * (-(i as f64 + 0.5) / ALONG as f64).ln_1p()).map(|d| [d * dir[0], d * dir[1]]).collect();
    let far = pts.iter().map(|p| p[0].hypot(p[1])).fold(0.0, f64::max);
    // The nodes within its reach and two of their spacings, found outward from it through their links.
    let within = |k: usize| air.offset(q, k)[0].hypot(air.offset(q, k)[1]) <= far + 2.0 * air.area()[k].sqrt();
    let (mut found, mut at) = (vec![q], 0);
    while at < found.len() {
        for &k in air.around(found[at]).1 {
            if !found.contains(&(k as usize)) && within(k as usize) {
                found.push(k as usize);
            }
        }
        at += 1;
    }
    let near: Vec<(usize, [f64; 2], f64)> = found.into_iter().map(|k| (k, air.offset(q, k), air.area()[k].sqrt())).collect();
    let mut got: Vec<(usize, f64)> = Vec::new();
    for p in &pts {
        let tent = |o: [f64; 2], a: f64| (1.0 - (o[0] - p[0]).abs() / a).max(0.0) * (1.0 - (o[1] - p[1]).abs() / a).max(0.0);
        let sum: f64 = near.iter().map(|&(_, o, a)| tent(o, a)).sum();
        if sum <= 0.0 {
            continue;
        }
        for &(k, o, a) in &near {
            let x = tent(o, a) / sum;
            if x > 0.0 {
                match got.iter_mut().find(|g| g.0 == k) {
                    Some(g) => g.1 += x,
                    None => got.push((k, x)),
                }
            }
        }
    }
    let all: f64 = got.iter().map(|g| g.1).sum();
    if all <= 0.0 {
        return vec![(q, 1.0)];
    }
    got.into_iter().map(|(k, x)| (k, x / all)).collect()
}
