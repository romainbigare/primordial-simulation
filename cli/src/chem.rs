//! `ew chem`: print chemistry 0 as a table.

use pcore::chem::{Chemistry, Phase, Reaction};
use worldgen::terrain::air_column;

/// The depth of loose ground the balance is shown for, m: a metre, about the median the wear leaves on land.
const LOOSE_M: f64 = 1.0;

/// Earth names of chemistry 0 (display only; the core never reads them).
const LIBRARY: &str = include_str!("../../library/chem0.toml");

/// The quoted strings of `key = [ ... ]` in the library file.
fn names(key: &str) -> Vec<String> {
    let start = LIBRARY.find(&format!("\n{key} = [")).map_or(0, |p| p + 1);
    let body = &LIBRARY[start..];
    let body = &body[..body.find(']').unwrap_or(0)];
    body.split('"').skip(1).step_by(2).map(String::from).collect()
}

fn formula(c: &Chemistry, k: usize, sym: &[String]) -> String {
    let mut s = String::new();
    for (j, &n) in c.subs[k].elems.iter().enumerate().filter(|p| *p.1 > 0) {
        s += &sym[j];
        if n > 1 {
            s += &n.to_string();
        }
    }
    s
}

/// The rate at which a reaction counts as running (its ignition), 1/s.
const IGNITION_RATE: f64 = 1e-6;

/// Temperature where a reaction runs at `IGNITION_RATE`: `k exp(-ea/T) = IGNITION_RATE`.
fn ignition(rx: &Reaction) -> f64 {
    let l = (rx.k / IGNITION_RATE).ln();
    if l > 0.0 {
        rx.ea / l
    } else {
        f64::INFINITY
    }
}

/// Depths of liquid, m, and temperatures, K, of the balance table.
const DEPTHS: [f64; 3] = [10.0, 100.0, 500.0];
const TEMPS: [f64; 3] = [278.0, 288.0, 298.0];

pub fn run() {
    let (c, init) = worldgen::chemistry0();
    let (sym, name, rnames) = (names("elements"), names("substances"), names("reactions"));
    println!("{:>3}  {:<22} {:<14} {:>7} {:>6} {:>6} {:>5} {:>5} {:>9}", "#", "name", "formula", "g/mol", "melt", "boil", "hard", "str", "hf kJ");
    for (k, s) in c.subs.iter().enumerate() {
        println!(
            "{k:>3}  {:<22} {:<14} {:>7.1} {:>6.0} {:>6.0} {:>5.2} {:>5.2} {:>9.1}",
            name.get(k).map_or("", |n| n.as_str()),
            formula(&c, k, &sym),
            s.molar_mass * 1000.0,
            s.melt,
            s.boil,
            s.hardness,
            s.strength,
            s.hf / 1000.0
        );
    }
    let side = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| format!("{}{k}", if n > 1 { format!("{n}*") } else { String::new() })).collect::<Vec<_>>().join(" + ");
    println!("\n{:>3}  {:<26} {:<24} {:>10} {:>9}", "#", "name", "reaction", "dH kJ", "T@1e-6/s");
    for (r, rx) in c.reactions.iter().enumerate() {
        let t = if rx.light { "light".to_string() } else { format!("{:.0} K", ignition(rx)) };
        println!(
            "{r:>3}  {:<26} {:<24} {:>10.1} {t:>9}",
            rnames.get(r).map_or("", |n| n.as_str()),
            format!("{} -> {}", side(&rx.inputs), side(&rx.outputs)),
            c.dh_at(r, pcore::units::T_SURF) / 1000.0
        );
    }
    balances(&c, &init);
}

/// For each pair of reactions that undo each other: what the first makes, per m2 of a column of
/// the starting liquid, at balance with the starting air (`Reactor::balance`, from the chemistry's
/// own rate constants and barriers), for three depths and three temperatures. Amounts in mol.
fn balances(c: &Chemistry, init: &worldgen::Init) {
    let pairs = pcore::change::reversible(c);
    if pairs.is_empty() {
        return;
    }
    let (ns, column) = (c.ns(), air_column(c, init, pcore::Config::default().gravity));
    let atot: f64 = init.air.iter().map(|p| p.1).sum();
    let gas: Vec<f64> = (0..ns).map(|k| init.air.iter().find(|p| p.0 == k).map_or(0.0, |p| p.1 / atot * column)).collect();
    println!("\nbalance with the starting air ({column:.0} mol/m2 of air over a cell), per m2 of cell and per m3 of liquid:");
    let mut rc = pcore::change::Reactor::new(c, &pcore::Config::default());
    for (f, b, _, u) in pairs {
        for depth in DEPTHS {
            let cols: Vec<String> = TEMPS
                .iter()
                .map(|&t| {
                    let mut n = vec![0.0; ns];
                    n[init.low] = depth / c.molar_volume(init.low, Phase::Liquid);
                    // The loose ground of the start over a bed of the first make, by volume (`worldgen::terrain`).
                    let all: f64 = init.makes[0].iter().map(|p| p.1).sum();
                    for &(k, share) in &init.makes[0] {
                        n[k] = share / all * LOOSE_M / c.molar_volume(k, Phase::Solid);
                    }
                    let x = rc.balance(c, &mut n, &gas, t).unwrap_or_else(|e| panic!("{e}")).iter().find(|p| p.0 == u).map_or(0.0, |p| p.1);
                    format!("{t:.0} K: {x:8.1} mol/m2 ({:.3} mol/m3)", x / depth)
                })
                .collect();
            println!("  {f} <-> {b}: substance {u}, liquid {depth:>4.0} m deep:  {}", cols.join("   "));
        }
    }
}
