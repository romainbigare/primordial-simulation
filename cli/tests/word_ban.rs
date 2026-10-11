//! Earth-word ban (handover 14.4): `core/` and `worldgen/` describe physics only.

use std::path::Path;

const BANNED: &str = "farm, crop, harvest, sow, herd, livestock, domestic, tame, pasture, house, home, hut, village, town, city, settlement, road, route, trail, river, lake, ocean, sea, market, trade, money, price, coin, wage, debt, loan, property, owner, king, queen, lord, council, kingdom, empire, nation, war, army, weapon, church, temple, religion, god, tool, axe, plough, wheel, cart, car, boat, ship, plane, engine, metal, iron, copper, bronze, steel, gold, wood, stone, coal, oil, fire, food, grain, meat, water, medicine, disease, tribe, family, household, culture";

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

fn scan(dir: &Path, banned: &[&str], hits: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                scan(&p, banned, hits);
            }
        } else if let Ok(text) = std::fs::read_to_string(&p) {
            for (i, line) in text.lines().enumerate() {
                for w in words(line).iter().filter(|w| banned.contains(&w.as_str())) {
                    hits.push(format!("{}:{}: {w}", p.display(), i + 1));
                }
            }
        }
    }
}

#[test]
fn no_earth_words_in_core_or_worldgen() {
    let banned: Vec<&str> = BANNED.split(", ").collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut hits = Vec::new();
    for d in ["core", "worldgen"] {
        scan(&root.join(d), &banned, &mut hits);
    }
    assert!(hits.is_empty(), "banned words:\n{}", hits.join("\n"));
}
