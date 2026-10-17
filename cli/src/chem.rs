//! `ew chem [--seed N]`: print chemistry 0 (or a generated one) as a table.

use crate::args::Args;
use pcore::chem::{Chemistry, Phase};
use worldgen::richness::ignition;
use worldgen::terrain::{air_column, LOOSE_M};

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

/// Depths of liquid, m, and temperatures, K, of the balance table.
const DEPTHS: [f64; 3] = [10.0, 100.0, 500.0];
const TEMPS: [f64; 3] = [278.0, 288.0, 298.0];

pub fn run(args: &Args) {
    let seed = args.value::<u64>("--seed");
    let (c, init) = match seed {
        Some(s) => worldgen::generate(s),
        None => worldgen::chemistry0(),
    };
    let (sym, name, rnames) = if seed.is_none() {
        (names("elements"), names("substances"), names("reactions"))
    } else {
        ((0..c.n_elements).map(|j| ((b'A' + j as u8) as char).to_string()).collect(), vec![], vec![])
    };
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
    println!("\nrichness:");
    for (what, ok) in worldgen::richness(&c, &init) {
        println!("  {} {what}", if ok { "pass" } else { "FAIL" });
    }
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
    let mut rc = pcore::change::Reactor::new(c, pcore::Config::default().seconds_per_year());
    for (f, b, _, u) in pairs {
        for depth in DEPTHS {
            let cols: Vec<String> = TEMPS
                .iter()
                .map(|&t| {
                    let mut n = vec![0.0; ns];
                    n[init.low] = depth / c.molar_volume(init.low, Phase::Liquid);
                    // The loose ground of the start, by volume (`worldgen::terrain`).
                    let all: f64 = init.loose.iter().map(|p| p.1).sum();
                    for &(k, share) in &init.loose {
                        n[k] = share / all * LOOSE_M / c.molar_volume(k, Phase::Solid);
                    }
                    let x = rc.balance(c, &mut n, &gas, t).iter().find(|p| p.0 == u).map_or(0.0, |p| p.1);
                    format!("{t:.0} K: {x:8.1} mol/m2 ({:.3} mol/m3)", x / depth)
                })
                .collect();
            println!("  {f} <-> {b}: substance {u}, liquid {depth:>4.0} m deep:  {}", cols.join("   "));
        }
    }
}
