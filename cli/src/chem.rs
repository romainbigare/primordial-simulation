//! `ew chem [--seed N]`: print chemistry 0 (or a generated one) as a table.

use pcore::chem::Chemistry;
use worldgen::richness::ignition;

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

pub fn run(args: &[String]) {
    let seed = args.iter().position(|a| a == "--seed").and_then(|p| args.get(p + 1)).and_then(|v| v.parse::<u64>().ok());
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
            c.dh(r) / 1000.0
        );
    }
    println!("\nrichness:");
    for (what, ok) in worldgen::richness(&c, &init) {
        println!("  {} {what}", if ok { "pass" } else { "FAIL" });
    }
}
