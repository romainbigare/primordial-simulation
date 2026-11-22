//! Earth-word ban (handover 14.4): `core/` and `worldgen/` describe physics only.

use std::path::Path;

const BANNED: &str = "farm, crop, harvest, sow, herd, livestock, domestic, tame, pasture, house, home, hut, village, town, city, settlement, road, route, trail, river, lake, ocean, sea, market, trade, money, price, coin, wage, debt, loan, property, owner, king, queen, lord, council, kingdom, empire, nation, war, army, weapon, church, temple, religion, god, tool, axe, plough, wheel, cart, car, boat, ship, plane, engine, metal, iron, copper, bronze, steel, gold, wood, stone, coal, oil, fire, food, grain, meat, water, medicine, disease, tribe, family, household, culture";

/// The Earth names of chemistry 0's substances and reactions (`library/chem0.toml`) that are not already above.
const SUBSTANCES: &str = "nitrogen, oxygen, carbon, dioxide, monoxide, methane, sugar, fat, charcoal, sand, quartz, limestone, quicklime, lime, ore, tin, calcium, bicarbonate, carbonate, calcite, nitrate, ammonium, fertiliser, salt, cellulose, cellulase, mortar, smelt, tannin, glycoside, cyanide, alkaloid, amygdalin";

/// Words that look like an inflection of a banned word but are not one: Rust's own names (`u64::trailing_zeros`).
const ALLOWED: [&str; 1] = ["trailing"];

/// Is `w` a banned word, or one with a plural or simple ending (-s, -es, -ing, -ed; a final e drops before -ing and
/// -ed: "fired", "firing")?
fn is_banned(w: &str, banned: &[&str]) -> bool {
    let plain = |s: &str| banned.contains(&s);
    let dropped_e = |s: &str| banned.iter().any(|b| b.strip_suffix('e') == Some(s));
    !ALLOWED.contains(&w)
        && (plain(w)
            || ["s", "es"].iter().any(|e| w.strip_suffix(e).is_some_and(plain))
            || ["ing", "ed"].iter().any(|e| w.strip_suffix(e).is_some_and(|s| plain(s) || dropped_e(s))))
}

/// Words of a line: split on non-alphanumerics, then at camelCase boundaries.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for run in line.split(|c: char| !c.is_alphanumeric()) {
        let ch: Vec<char> = run.chars().collect();
        let mut start = 0;
        for i in 1..ch.len() {
            let lower_upper = ch[i - 1].is_lowercase() && ch[i].is_uppercase();
            let acronym_end = ch[i - 1].is_uppercase() && ch[i].is_uppercase() && ch.get(i + 1).is_some_and(|c| c.is_lowercase());
            if lower_upper || acronym_end {
                out.push(ch[start..i].iter().collect::<String>().to_lowercase());
                start = i;
            }
        }
        out.push(ch[start..].iter().collect::<String>().to_lowercase());
    }
    out
}

/// Every banned word in the files under `dir`, in their text and in their names (and the names of folders).
fn scan(dir: &Path, banned: &[&str], hits: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        for w in words(&name).iter().filter(|w| is_banned(w, banned)) {
            hits.push(format!("{}: (name) {w}", p.display()));
        }
        if p.is_dir() {
            if name != "target" {
                scan(&p, banned, hits);
            }
        } else if let Ok(text) = std::fs::read_to_string(&p) {
            for (i, line) in text.lines().enumerate() {
                for w in words(line).iter().filter(|w| is_banned(w, banned)) {
                    hits.push(format!("{}:{}: {w}", p.display(), i + 1));
                }
            }
        }
    }
}

#[test]
fn no_earth_words_in_core_or_worldgen() {
    let banned: Vec<&str> = BANNED.split(", ").chain(SUBSTANCES.split(", ")).collect();
    let caught = ["lakes", "oxygen", "fired", "trading", "salted", "ores"].iter().all(|w| is_banned(w, &banned));
    assert!(caught && !["or", "plan", "lim", "trailing"].iter().any(|w| is_banned(w, &banned)), "the inflection rule");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut hits = Vec::new();
    for d in ["core", "worldgen"] {
        scan(&root.join(d), &banned, &mut hits);
    }
    assert!(hits.is_empty(), "banned words:\n{}", hits.join("\n"));
}
