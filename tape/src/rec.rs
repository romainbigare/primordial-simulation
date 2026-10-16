//! The recorder: driven once per simulated month by the shell.

use crate::layers::{frame, Ctx, COVER_MIN, LAYERS};
use crate::measure;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use pcore::check;
use pcore::event::{Event, EventKind};
use pcore::World;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::path::Path;

type Column = (String, &'static str, Vec<f64>);

/// What bodies did to and with each other in the current year, and how they move and signal.
#[derive(Default)]
struct Social {
    /// Watch events, of them within a lineage; watcher groups per watched group
    watched: f64,
    kin_watched: f64,
    spread: BTreeMap<u32, BTreeSet<u32>>,
    /// Puts into other bodies, kg put, (giver, receiver) pairs
    gave: f64,
    gave_kg: f64,
    /// Of the puts, those by bodies that are not light-eaters (still alive at the month's end)
    gave_eaters: f64,
    pairs: BTreeSet<(u64, u64)>,
    /// Per body slot: its generation and cell at the last month, for the bodies that are not
    /// light-eaters (for movement by month); generation u32::MAX: none
    last: Vec<(u32, u32)>,
    /// By lineage, per context (cold, hurt, low): counts of (context, signal quarter)
    sig: Vec<[[[f64; 4]; 2]; 3]>,
    /// Top traffic cells at the last decade
    top: Vec<usize>,
    persist: f64,
    /// Worn edges (cheaper than 2x less-worn ground) last year and at the last decade, and the
    /// share of the decade's still worn
    worn: Vec<u32>,
    worn_old: Vec<u32>,
    worn_persist: f64,
    /// kg that bodies that are not light-eaters could burn of what they took this year: from
    /// light-eaters, from others, from the ground
    intake: [f64; 3],
    /// Per body slot: its generation, and what it took from other bodies in its life, kg: from
    /// light-eaters, from the others
    took: Vec<(u32, [f32; 2])>,
    /// Decisions drawn this year, and those where the action drawn had the top score
    decisions: [f64; 2],
    /// Members the monthly merge joined however unlike this year (the core's tally, `Diag::unlike`)
    unlike: f64,
    /// Members that joined a node unlike it by more than `alike` steps in some gene this year (`Diag::wide`)
    wide: f64,
    /// Members that moved to a neighbouring cell in the year, by starting kind (the core's tally, `Diag::moves`)
    moves: Vec<f64>,
    /// Body matter, kg, hits tore off members who lived on in the year, by starting kind (the core's tally, `Diag::hurt`)
    hurt: Vec<f64>,
    /// Members that could build in the year, by starting kind: [tried, held back by any mineral, by the first, by the second...] (the core's tally, `Diag::nlim`)
    nlim: Vec<Vec<f64>>,
    /// The land's liquid in the year, m³ (`Diag::water`)
    water: [f64; 5],
    months: String,
    /// Members born in each month by starting kind, one row per row of `months` (after its
    /// header), written as the last columns of `months.csv` (`root_<k>_births`)
    month_births: Vec<Vec<f64>>,
    /// Members born by kind since the last month (the row to come)
    births_now: Vec<f64>,
    /// Like `month_births`, the members that died in each month by starting kind and cause (`CAUSES`; index
    /// kind x 6 + cause), written after the births as `root_<k>_died_<cause>` (15 Oct 2026), and the row to come
    month_deaths: Vec<Vec<f64>>,
    deaths_now: Vec<f64>,
    /// Members of the others that died of SPENT this month, and the sum of their rows (y)
    spent: [f64; 2],
    /// Bonds made this year by kind (held, joined, anchored); broken by bodies; failed by load
    made: [f64; 3],
    broken: f64,
    failed: f64,
    /// Body-months of bodies that are not light-eaters, [cold, warm]: all, inside walls
    inside: [[f64; 2]; 2],
    /// What bodies did with parts this year (from the core's tally)
    tally: pcore::bond::Tally,
    /// By starting kind (`Lineage::root`) since the last yearly row: members born, members died by
    /// cause (the causes of `Recorder::deaths`), members summed over the months (standing members
    /// are this over `stand_months`) and the months counted
    root_births: Vec<f64>,
    /// Of those, the members born of two parents (`Born::mixed`), by starting kind
    root_mixed: Vec<f64>,
    root_deaths: Vec<[f64; 6]>,
    /// Members died while their body was below / at or past their copy mass (`Died::adult`)
    root_young: Vec<f64>,
    root_adult: Vec<f64>,
    root_stand: Vec<f64>,
    /// Like `root_stand`, the members whose body is at or past their copy mass (`Body::adult`)
    root_stand_adult: Vec<f64>,
    stand_months: f64,
    /// Mean ground temperature over land (K) at the end of each month since the last yearly row;
    /// land is what the tape counts as land everywhere: solid surface above 0 m (the cells are
    /// listed at the first month of a year, `land`)
    t_land: Vec<f64>,
    land: Vec<u32>,
}

impl Social {
    /// What the body with this id took from other bodies in its life, kg (none if nothing).
    fn eaten(&self, id: u64) -> Option<[f32; 2]> {
        self.took.get(id as u32 as usize).filter(|t| t.0 == (id >> 32) as u32).map(|t| t.1)
    }
}

/// Mutual information (bits) between a binary context and the signal quarter, given lineage,
/// less its small-sample bias.
fn info(sig: &[[[[f64; 4]; 2]; 3]], c: usize) -> f64 {
    let (mut sum, mut all) = (0.0, 0.0);
    for t in sig {
        let j = &t[c];
        let n: f64 = j.iter().flatten().sum();
        if n < 20.0 {
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
        sum += n * (i - 3.0 / (2.0 * n * std::f64::consts::LN_2)).max(0.0);
        all += n;
    }
    sum / all.max(1.0)
}

/// Sums over the live members of one starting kind (`Recorder::push_series`): members, bodies, member-years of life,
/// mass (kg), body matter (kg), body (kg, `Body::size`), member-copy-mass (kg), the members whose body (`Body::built`) is
/// past their copy mass (mature), their brood (kg), the genes of `GENES` (by members), and per catalyst gene the sum over the members
/// that carry it on and the members that do.
#[derive(Clone)]
struct Root {
    members: f64,
    bodies: f64,
    life: f64,
    mass: f64,
    matter: f64,
    body: f64,
    copy_mass: f64,
    past_copy: f64,
    /// Matter held for offspring (`Body::brood`), kg
    brood: f64,
    /// The `ALLOC` and `CHILD` genes by members
    alloc: f64,
    child: f64,
    genes: [f64; 9],
    /// Activity (`Body::activity`) summed over the members, the members below 0.5, and the `WET` gene by members
    act: f64,
    dry: f64,
    wet: f64,
    /// The members' temperature above their ground's (`Body::temp`), K, summed by members
    warm: f64,
    /// Each mineral (`Chemistry::minerals`) their stores hold, kg (15 Oct 2026)
    mineral: Vec<f64>,
    cat: Vec<(f64, f64)>,
}

impl Root {
    fn new(nr: usize) -> Self {
        Root { members: 0.0, bodies: 0.0, life: 0.0, mass: 0.0, matter: 0.0, body: 0.0, copy_mass: 0.0, past_copy: 0.0, brood: 0.0, alloc: 0.0, child: 0.0, genes: [0.0; 9], act: 0.0, dry: 0.0, wet: 0.0, warm: 0.0, mineral: Vec::new(), cat: vec![(0.0, 0.0); nr] }
    }
}

/// The genes averaged per starting kind, and their column names (`root_<k>_<name>`); the repair gene is the old
/// `root_<k>_repair`, so it comes last here and is printed in its old place.
const GENES: [usize; 9] = [pcore::body::GAS, pcore::body::LEAF, pcore::body::COPY_MASS, pcore::body::INTERVAL, pcore::body::HIDDEN, pcore::body::HEAT, pcore::body::KEEP_T, pcore::body::COOL_T, pcore::body::REPAIR];
const GENE_NAMES: [&str; 8] = ["gas", "leaf", "copy_log", "interval", "hidden", "heat", "keep_t", "cool_t"];
/// The first columns of `months.csv`; `root_<k>_births` for each starting kind follow.
const MONTHS: &str = "year,month,mobile,mean_y,mean_z,mean_dy,mean_dz,inside,t_mobile,spent,spent_y";
/// Death causes in the order of the core's constants (`LACK` .. `AGE`).
const CAUSES: [&str; 6] = ["lack", "phase", "spent", "taken", "broken", "age"];

pub struct Recorder {
    /// Meta: width, height, cell size, seed, days per year
    meta: (usize, usize, f64, u64, u64),
    every: u64,
    monthly: u64,
    seen: Vec<bool>,
    frames: Vec<u64>,
    bytes: Vec<u8>,
    series: Vec<Column>,
    log: Vec<(u64, String)>,
    /// Members born and died by cause since the last yearly row; born and died, of light-eaters
    /// and of the others
    births: f64,
    deaths: [f64; 6],
    /// Members of the others (not light-eaters) died by cause since the last yearly row
    deaths_a: [f64; 6],
    split: [[f64; 2]; 2],
    /// Body updates counted at the last yearly row
    updates: u64,
    /// The run's `Physics` and `Config` as JSON text, written next to the series (`physics.json`)
    physics: String,
    social: Social,
}

/// Years a member of `b` lives at its upkeep now (`body::live`): `e_life` x 10^`REPAIR` per kg
/// over its upkeep per kg.
fn life_years(w: &World, b: &pcore::body::Body) -> f64 {
    let mut x = pcore::body::Body::default();
    let b = if b.k.cat.is_empty() {
        x.clone_from(b);
        x.prepare(&w.chem, &w.cfg.phys);
        &x
    } else {
        b
    };
    let (m, d) = (b.mass(&w.chem), b.interval() as f64);
    w.cfg.phys.e_life * b.dec(pcore::body::REPAIR) * m * d / pcore::body::upkeep(w, b, d, m).max(1e-300) / w.days_per_year() as f64
}

/// The run's numbers: `{"physics": {..}, "config": {..}}`, every field of `Physics` and of `Config`
/// (the config without its `phys`), keys sorted, one line each, so two runs' files can be
/// compared with a diff or `ew numbers`.
fn physics_json(w: &World) -> String {
    let mut cfg = serde_json::to_value(&w.cfg).unwrap_or_default();
    if let Some(o) = cfg.as_object_mut() {
        o.remove("phys");
    }
    let doc = json!({"physics": w.cfg.phys, "config": cfg});
    serde_json::to_string_pretty(&doc).unwrap_or_default() + "\n"
}

fn describe(e: &Event) -> String {
    match e.kind {
        EventKind::FirstSubstance(k) => format!("substance {k} first appears"),
        EventKind::Imposed => format!("god action at node {}", e.node),
        // Counted before they get here (see `record_month`).
        _ => String::new(),
    }
}

impl Recorder {
    /// Frames are kept every `every` years, and every month in the first `monthly` years.
    pub fn new(w: &World, every: u64, monthly: u64) -> Self {
        let g = &w.grid;
        Recorder {
            meta: (g.w, g.h, g.cell_m, w.seed, w.days_per_year()),
            every,
            monthly,
            seen: vec![false; w.chem.ns()],
            frames: vec![],
            bytes: vec![],
            series: vec![],
            log: vec![],
            births: 0.0,
            deaths: [0.0; 6],
            deaths_a: [0.0; 6],
            split: [[0.0; 2]; 2],
            updates: 0,
            physics: physics_json(w),
            social: Social { months: format!("{MONTHS}\n"), ..Default::default() },
        }
    }

    /// Start a run resumed from a snapshot: treat substances already present as known, and load
    /// what an earlier run wrote to `dir` (series, months, frames, events) up to the snapshot's
    /// day, so the files go on from there. What it wrote after that day is dropped (a resume
    /// from an earlier snapshot replays it). A `dir` without a tape starts the tape empty.
    pub fn resume(&mut self, w: &mut World, dir: impl AsRef<Path>) -> io::Result<()> {
        self.detect(w);
        self.physics = physics_json(w);
        w.events.clear();
        self.updates = w.life.updates;
        let dir = dir.as_ref();
        let Ok(f) = std::fs::File::open(dir.join("world.ewf.gz")) else { return Ok(()) };
        let bad = |e: &str| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", dir.display()));
        let mut raw = vec![];
        GzDecoder::new(f).read_to_end(&mut raw)?;
        let n = raw.get(4..8).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let head = raw.get(8..8 + n).filter(|_| raw.starts_with(b"EWF1")).ok_or_else(|| bad("not a tape"))?;
        let g: serde_json::Value = serde_json::from_slice(head)?;
        let dpy = w.days_per_year();
        if [g["w"].as_u64(), g["h"].as_u64(), g["seed"].as_u64(), g["days_per_year"].as_u64()] != [self.meta.0 as u64, self.meta.1 as u64, self.meta.3, dpy].map(Some) {
            return Err(bad("the tape is of another world"));
        }
        // Frames and events carry their day: keep those up to the snapshot's day.
        let list = |k: &str| g[k].as_array().cloned().unwrap_or_default();
        let day = |e: &serde_json::Value| e["day"].as_u64().unwrap_or(u64::MAX);
        let (frames, events) = (list("frames"), list("events"));
        let frame = self.meta.0 * self.meta.1 * LAYERS.len();
        if raw.len() - 8 - n != frame * frames.len() {
            return Err(bad("the frames do not fit the tape's size"));
        }
        self.frames = frames.iter().map(day).take_while(|&d| d <= w.day).collect();
        self.bytes = raw[8 + n..8 + n + frame * self.frames.len()].to_vec();
        self.log = events.iter().take_while(|e| day(e) <= w.day).map(|e| (day(e), e["text"].as_str().unwrap_or("").to_string())).collect();
        // Series: the rows of years up to this one (a null is a NaN that was written).
        let years = g["series"]["year"].as_array().map_or(0, |a| a.iter().take_while(|y| y.as_f64().is_some_and(|y| y <= w.year())).count());
        let cols = g["series"].as_object().into_iter().flatten();
        self.series = cols.map(|(k, v)| (k.clone(), "", v.as_array().unwrap()[..years].iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect())).collect();
        // Months: the header must be ours, then the rows up to this month.
        let (dpm, mpy) = (w.cfg.days_per_month as u64, w.cfg.months_per_year as u64);
        let now = (w.day / dpy, (w.day / dpm) % mpy);
        let old = std::fs::read_to_string(dir.join("months.csv")).unwrap_or_default();
        let mut lines = old.lines();
        let head = lines.next();
        if head.is_some_and(|h| !h.starts_with(MONTHS)) {
            return Err(bad("months.csv has other columns"));
        }
        // The kinds with a births column (an older file has none, nor deaths).
        let nb = head.map_or(0, |h| h.split(',').filter(|c| c.ends_with("_births")).count());
        for l in lines.take_while(|l| {
            let mut f = l.split(',').map(|v| v.parse::<u64>().unwrap_or(u64::MAX));
            (f.next().unwrap_or(0), f.next().unwrap_or(0)) <= now
        }) {
            // The columns of `MONTHS`, then the births of each kind (an older file has none).
            let at = l.match_indices(',').nth(MONTHS.matches(',').count()).map_or(l.len(), |p| p.0);
            let (head, rest) = l.split_at(at);
            self.social.months += &(head.to_string() + "\n");
            let v: Vec<f64> = rest.split(',').skip(1).map(|v| v.parse().unwrap_or(0.0)).collect();
            self.social.month_births.push(v[..nb.min(v.len())].to_vec());
            self.social.month_deaths.push(v[nb.min(v.len())..].to_vec());
        }
        Ok(())
    }

    /// A substance with a non-zero amount anywhere that was zero everywhere before.
    fn detect(&mut self, w: &mut World) {
        let (n, na) = (w.grid.n(), w.grid.na());
        for k in 0..self.seen.len() {
            if self.seen[k] {
                continue;
            }
            let hit = [(&w.ground, n, 0), (&w.bed, n, 0), (&w.air, na, n)]
                .iter()
                .find_map(|&(v, len, off)| v[k * len..(k + 1) * len].iter().position(|&x| x > 0.0).map(|p| (off + p, v[k * len + p])));
            if let Some((node, value)) = hit {
                self.seen[k] = true;
                w.events.push(Event { day: w.day, kind: EventKind::FirstSubstance(k as u16), node: node as u32, value });
            }
        }
    }

    /// Call at the start and after every simulated month.
    pub fn record_month(&mut self, w: &mut World) {
        self.detect(w);
        let events: Vec<Event> = w.events.drain(..).collect();
        for e in events {
            match e.kind {
                EventKind::Born { cat, root, mixed } => {
                    self.births += e.value;
                    if root != u32::MAX {
                        let v = &mut self.social.root_mixed;
                        v.resize(v.len().max(root as usize + 1), 0.0);
                        v[root as usize] += if mixed { e.value } else { 0.0 };
                        for v in [&mut self.social.root_births, &mut self.social.births_now] {
                            v.resize(v.len().max(root as usize + 1), 0.0);
                            v[root as usize] += e.value;
                        }
                    }
                    self.split[0][(cat < measure::LIGHT_EATER) as usize] += e.value;
                }
                EventKind::Died { cause, cat, root, adult } => {
                    self.deaths[(cause as usize).min(5)] += e.value;
                    if root != u32::MAX {
                        let v = &mut self.social.root_deaths;
                        v.resize(v.len().max(root as usize + 1), [0.0; 6]);
                        v[root as usize][(cause as usize).min(5)] += e.value;
                        let v = &mut self.social.deaths_now;
                        v.resize(v.len().max(6 * (root as usize + 1)), 0.0);
                        v[6 * root as usize + (cause as usize).min(5)] += e.value;
                        let v = if adult { &mut self.social.root_adult } else { &mut self.social.root_young };
                        v.resize(v.len().max(root as usize + 1), 0.0);
                        v[root as usize] += e.value;
                    }
                    if cat < measure::LIGHT_EATER {
                        self.deaths_a[(cause as usize).min(5)] += e.value;
                        if cause == pcore::body::SPENT {
                            let s = &mut self.social.spent;
                            (s[0], s[1]) = (s[0] + e.value, s[1] + e.value * (e.node / w.grid.w as u32) as f64);
                        }
                    }
                    self.split[1][(cat < measure::LIGHT_EATER) as usize] += e.value;
                }
                EventKind::Watched { by, of } => {
                    let s = &mut self.social;
                    (s.watched, s.kin_watched) = (s.watched + 1.0, s.kin_watched + e.value);
                    s.spread.entry(of).or_default().insert(by);
                }
                EventKind::Gave { from, to } => {
                    let s = &mut self.social;
                    (s.gave, s.gave_kg) = (s.gave + 1.0, s.gave_kg + e.value);
                    s.pairs.insert((from, to));
                    let b = w.life.bodies.get(from as u32 as usize).filter(|b| b.alive && b.gen == (from >> 32) as u32);
                    s.gave_eaters += b.is_some_and(|b| !crate::light_eater(&w.chem, &b.genes)) as u8 as f64;
                }
                EventKind::Took { by, from, burn, cat } => {
                    let s = &mut self.social;
                    if from != u64::MAX {
                        if s.took.len() <= by as u32 as usize {
                            s.took.resize(by as u32 as usize + 1, (0, [0.0; 2]));
                        }
                        let t = &mut s.took[by as u32 as usize];
                        if t.0 != (by >> 32) as u32 {
                            *t = ((by >> 32) as u32, [0.0; 2]);
                        }
                        t.1[(cat[1] < measure::LIGHT_EATER) as usize] += e.value as f32;
                    }
                    if cat[0] < measure::LIGHT_EATER {
                        s.intake[if from == u64::MAX { 2 } else { (cat[1] < measure::LIGHT_EATER) as usize }] += burn as f64;
                    }
                }
                EventKind::BondMade => self.social.made[(e.value as usize).min(2)] += 1.0,
                EventKind::BondBroken if e.value >= pcore::bond::LOAD => self.social.failed += 1.0,
                EventKind::BondBroken => self.social.broken += 1.0,
                _ => self.log.push((e.day, describe(&e))),
            }
        }
        let dpy = w.days_per_year();
        let (year, yearly) = (w.day / dpy, w.day % dpy == 0);
        let framed = year < self.monthly || (yearly && self.every > 0 && year % self.every == 0);
        self.month(w);
        if !yearly && !framed {
            return;
        }
        if yearly {
            self.social.decisions = [std::mem::take(&mut w.diag.decisions), std::mem::take(&mut w.diag.greedy)];
            self.social.unlike = std::mem::take(&mut w.diag.unlike);
            self.social.wide = std::mem::take(&mut w.diag.wide);
            self.social.moves = std::mem::take(&mut w.diag.moves);
            self.social.hurt = std::mem::take(&mut w.diag.hurt);
            self.social.nlim = std::mem::take(&mut w.diag.nlim);
            self.social.water = std::mem::take(&mut w.diag.liquid_land);
            self.social.tally = std::mem::take(&mut w.parts.tally);
        }
        let c = Ctx::new(w);
        // A resumed run finds the snapshot's own row and frame already in the tape it loaded.
        if yearly && self.series.iter().find(|c| c.0 == "year").and_then(|c| c.2.last()) != Some(&w.year()) {
            self.push_series(&c);
        }
        if framed && self.frames.last() != Some(&w.day) {
            self.frames.push(w.day);
            frame(&c, &mut self.bytes);
        }
    }

    /// Monthly: where mobile bodies are and how they moved since last month; signals by context.
    fn month(&mut self, w: &World) {
        let (chem, gw, s) = (&w.chem, w.grid.w as u32, &mut self.social);
        let (mut n, mut sum, mut inside, mut tm) = (0.0f64, [0.0; 4], 0.0, 0.0);
        let mut last = vec![(u32::MAX, 0); w.life.bodies.len()];
        s.stand_months += 1.0;
        for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            let i = b.cell as usize;
            if let Some(l) = &w.life.brains[b.lineage as usize] {
                s.root_stand.resize(s.root_stand.len().max(l.root as usize + 1), 0.0);
                s.root_stand[l.root as usize] += b.n();
                s.root_stand_adult.resize(s.root_stand.len(), 0.0);
                s.root_stand_adult[l.root as usize] += b.n() * b.adult(chem) as u8 as f64;
            }
            let t = w.life.env.get(i).map_or(285.0, |e| e[0]);
            let ctx = [t < 283.0, b.known.iter().any(|k| k.1 < 0.0), b.wb < 1.0];
            let q = (b.sig[0] > 0.0) as usize * 2 + (b.sig[1] > 0.0) as usize;
            if s.sig.len() <= b.lineage as usize {
                s.sig.resize(b.lineage as usize + 1, Default::default());
            }
            let tab = &mut s.sig[b.lineage as usize];
            for (c, &on) in ctx.iter().enumerate() {
                tab[c][on as usize][q] += 1.0;
            }
            if crate::light_eater(chem, &b.genes) {
                continue;
            }
            last[slot] = (b.gen, b.cell);
            let ins = pcore::bond::room(w, b).is_some();
            let c = &mut s.inside[(t >= 278.0) as usize];
            (c[0], c[1], inside, tm) = (c[0] + 1.0, c[1] + ins as u8 as f64, inside + ins as u8 as f64, tm + t);
            let z = w.life.env.get(i).map_or(0.0, |e| e[2]);
            let (dy, dz) = match s.last.get(slot).filter(|l| l.0 == b.gen) {
                Some(&(_, c)) => ((b.cell / gw) as f64 - (c / gw) as f64, z - w.life.env.get(c as usize).map_or(z, |e| e[2])),
                None => (0.0, 0.0),
            };
            n += 1.0;
            for (a, v) in sum.iter_mut().zip([(b.cell / gw) as f64, z, dy, dz]) {
                *a += v;
            }
        }
        s.last = last;
        // The land's mean ground temperature this month (the land is listed again each year).
        if s.t_land.is_empty() {
            s.land = (0..w.grid.n()).filter(|&i| w.surface_z(i) > 0.0).map(|i| i as u32).collect();
        }
        s.t_land.push(s.land.iter().map(|&i| w.thaw(i as usize).0).sum::<f64>() / s.land.len().max(1) as f64);
        let dpm = w.cfg.days_per_month as u64;
        let (y, m) = (w.day / w.days_per_year(), (w.day / dpm) % w.cfg.months_per_year as u64);
        let mean = sum.map(|v| v / n.max(1.0));
        let [sp, sy] = std::mem::take(&mut s.spent);
        let row = format!("{y},{m},{n},{:.3},{:.1},{:.4},{:.3},{inside},{:.2},{sp:.0},{:.2}\n", mean[0], mean[1], mean[2], mean[3], tm / n.max(1.0), sy / sp.max(1e-9));
        // (A resumed run has this month's row already, from the tape it loaded.)
        let (births, deaths) = (std::mem::take(&mut s.births_now), std::mem::take(&mut s.deaths_now));
        if s.months.lines().next_back().is_none_or(|l| !l.starts_with(&format!("{y},{m},"))) {
            s.months += &row;
            s.month_births.push(births);
            s.month_deaths.push(deaths);
        }
    }

    /// How far learning has moved the brains from their start, how often the best-scored
    /// action was the one drawn, and the mean sizes of the live bodies' brains and senses
    /// (hidden units, memory numbers, bodies perceived, sight) with the largest hidden layer.
    fn learning(&self, w: &World) -> [(String, &'static str, f64); 7] {
        let [n, g] = self.social.decisions;
        let (mut sz, mut top, mut k) = ([0.0; 4], 0.0f64, 0.0);
        for l in w.life.alive().filter_map(|b| w.life.brains[b.lineage as usize].as_ref()) {
            let x = pcore::choose::sizes_of(l);
            sz.iter_mut().zip(x).for_each(|(s, v)| *s += v as f64);
            (top, k) = (top.max(x[0] as f64), k + 1.0);
        }
        let sz = sz.map(|v| v / f64::max(k, 1.0));
        let name = |s: &str| s.to_string();
        [(name("learned"), "1", measure::learned(w)), (name("greedy"), "1", g / n.max(1.0)), (name("hidden"), "1", sz[0]), (name("memory"), "1", sz[1]), (name("seen"), "1", sz[2]), (name("sight"), "cells", sz[3]), (name("hidden_max"), "1", top)]
    }

    /// Kinds of live bodies by their genes and diet (see `measure::kinds`: `kinds` counts each body once, `kinds_members` each member), the mass range of their members
    /// (kg), and the share of what a kind took from other bodies that came from bodies that are
    /// not light-eaters: for the kind with most bodies, and the highest of any kind.
    fn variety(&self, w: &World) -> [(String, &'static str, f64); 8] {
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
        let high = (0..k).map(share).fold(0.0, f64::max);
        [
            ("kinds".to_string(), "1", if live.is_empty() { 0.0 } else { k as f64 }),
            ("kinds_members".to_string(), "1", if live.is_empty() { 0.0 } else { measure::kinds_by_members(w) as f64 }),
            ("mass_min".to_string(), "kg", at(0)),
            ("mass_median".to_string(), "kg", at(mass.len() / 2)),
            ("mass_max".to_string(), "kg", at(mass.len().wrapping_sub(1))),
            ("mass_max_root".to_string(), "1", heavy.last().map_or(f64::NAN, |m| m.1)),
            ("trophic_max".to_string(), "1", big.map_or(f64::NAN, share)),
            ("trophic_high".to_string(), "1", high),
        ]
    }

    fn push_series(&mut self, c: &Ctx) {
        let life = &c.w.life;
        let light_eaters = life.alive().filter(|b| crate::light_eater(&c.w.chem, &b.genes)).count() as f64;
        let bodies = life.alive().count() as f64;
        let w = c.w;
        // Members (the sum of counts) of light-eaters and of the others; nodes and individuals of the others.
        let (mut members, mut nodes) = ([0.0; 2], 0.0);
        // Bodies and members by the starting kind they descend from (`Lineage::root`), with the sums for their means by
        // members (`Root`). The columns `root_<k>_*` are for every k up to the largest seen so far (also in the columns of a
        // resumed tape), so a kind that died out reads 0 (its means NaN).
        let nr = w.chem.reactions.len();
        let seen = self.series.iter().filter(|c| c.0.starts_with("root_") && c.0.ends_with("_members")).count();
        let mut by_root: Vec<Root> = vec![Root::new(nr); seen.max(self.social.root_births.len()).max(self.social.root_deaths.len()).max(self.social.moves.len()).max(self.social.hurt.len()).max(self.social.nlim.len())];
        let cat = pcore::body::Layout::of(&w.chem).cat;
        let minerals = w.chem.minerals();
        for b in life.alive() {
            if let Some(l) = &life.brains[b.lineage as usize] {
                let k = l.root as usize;
                if k >= by_root.len() {
                    by_root.resize(k + 1, Root::new(nr));
                }
                let (r, n) = (&mut by_root[k], b.n());
                r.members += n;
                r.bodies += 1.0;
                r.life += n * life_years(w, b);
                r.mass += b.mass(&w.chem);
                r.matter += w.chem.matter().map_or(0.0, |m| b.store[m] * w.chem.subs[m].molar_mass);
                r.body += b.size(&w.chem);
                r.copy_mass += n * b.dec(pcore::body::COPY_MASS);
                r.past_copy += n * b.adult(&w.chem) as u8 as f64;
                (r.alloc, r.child) = (r.alloc + n * b.gene(pcore::body::ALLOC), r.child + n * b.gene(pcore::body::CHILD));
                r.brood += b.brooded(&w.chem) * w.chem.matter().map_or(0.0, |m| w.chem.subs[m].molar_mass);
                let a = b.activity(&w.chem, &w.cfg.phys);
                (r.act, r.dry, r.wet) = (r.act + n * a, r.dry + n * (a < 0.5) as u8 as f64, r.wet + n * b.gene(pcore::body::WET));
                r.warm += n * (b.temp(&w.chem) - c.f.t[b.cell as usize]);
                r.mineral.resize(minerals.len(), 0.0);
                for (x, &(k, _)) in r.mineral.iter_mut().zip(&minerals) {
                    *x += b.store[k] * w.chem.subs[k].molar_mass;
                }
                for (x, g) in r.genes.iter_mut().zip(GENES) {
                    *x += n * b.gene(g);
                }
                for (x, &g) in r.cat.iter_mut().zip(&b.genes[cat..cat + nr]) {
                    if g >= 0.0 {
                        (x.0, x.1) = (x.0 + n * g as f64, x.1 + n);
                    }
                }
            }
            members[!crate::light_eater(&w.chem, &b.genes) as usize] += b.n();
            nodes += (!crate::light_eater(&w.chem, &b.genes) && b.n() > 1.0) as u8 as f64;
        }
        // Light-eater mass per m² on land: median and largest.
        let mut cover: Vec<f64> = (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).map(|i| c.plants[i]).collect();
        cover.sort_by(|a, b| a.total_cmp(b));
        let n = w.grid.n() as f64;
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let tot = check::totals(w);
        let land_area = (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).count().max(1) as f64 * w.grid.area();
        let land_liquid: f64 = (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).map(|i| c.f.depth[i]).sum::<f64>() * w.grid.area();
        // 15 Oct 2026 (soil water): of it, the liquid that stands (what the pores of the loose ground do not hold, `body::env_of`).
        let land_stands: f64 = (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).map(|i| pcore::body::env_of(w, i)[1]).sum::<f64>() * w.grid.area();
        let mut row = vec![("year".to_string(), "yr", w.year()), ("null".to_string(), "1", w.cfg.null as u8 as f64)];
        row.extend(tot.elems.iter().enumerate().map(|(j, &v)| (format!("elem_{j}"), "mol", v)));
        // 15 Oct 2026, the minerals (`Chemistry::minerals`; the first is the nitrogen mineral in chemistry 0, the second the rock salt): the mass of each in the
        // ground per m² of land and of sea (kg/m2), and where it is in all, in 10^9 mol of it (matter counted at its share of
        // it): the ground of land and of the sea, dead matter on the ground of each, and the bodies.
        for (q, &(mk, per)) in minerals.iter().enumerate() {
            let (gn, area, mm) = (w.grid.n(), w.grid.area(), w.chem.subs[mk].molar_mass);
            let matter = w.chem.matter();
            let mut t = [[0.0f64; 2]; 3];
            let mut cells = [0.0f64; 2];
            for i in 0..gn {
                let sea = (c.f.z[i] <= 0.0) as usize;
                cells[sea] += 1.0;
                t[0][sea] += w.ground[mk * gn + i];
                t[1][sea] += matter.map_or(0.0, |m| w.ground[m * gn + i]) * per;
            }
            for b in life.alive() {
                t[2][0] += b.store[mk] + matter.map_or(0.0, |m| b.store[m]) * per;
            }
            row.extend([
                (format!("min{q}_land"), "kg/m2", t[0][0] * mm / (cells[0].max(1.0) * area)),
                (format!("min{q}_sea"), "kg/m2", t[0][1] * mm / (cells[1].max(1.0) * area)),
                (format!("min{q}_dead_land"), "1e9 mol", t[1][0] / 1e9),
                (format!("min{q}_dead_sea"), "1e9 mol", t[1][1] / 1e9),
                (format!("min{q}_bodies"), "1e9 mol", t[2][0] / 1e9),
                (format!("min{q}_ground_land"), "1e9 mol", t[0][0] / 1e9),
                (format!("min{q}_ground_sea"), "1e9 mol", t[0][1] / 1e9),
                (format!("min{q}_deep"), "1e9 mol", w.deep.get(mk * gn..(mk + 1) * gn).map_or(0.0, |v| v.iter().sum::<f64>()) / 1e9),
            ]);
        }
        row.extend([
            ("energy".to_string(), "J", tot.thermal + tot.chemical),
            ("t_ground".to_string(), "K", mean(&c.f.t)),
            ("t_air".to_string(), "K", mean(&c.t_air)),
            ("absorber".to_string(), "1", mean(&c.absorber)),
            ("ice_frac".to_string(), "1", c.f.ice.iter().filter(|&&v| v >= COVER_MIN).count() as f64 / n),
            ("liquid_land".to_string(), "km3", land_liquid / 1e9),
            ("stand_land".to_string(), "km3", land_stands / 1e9),
            // 15 Oct 2026 (drainage): the share of land cells where liquid stands over full pores, and the liquid in the stores of the bed under the land.
            ("wet_land".to_string(), "1", (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0 && pcore::body::env_of(w, i)[1] > 0.0).count() as f64 / (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).count().max(1) as f64),
            ("deep_land".to_string(), "km3", (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).map(|i| (0..w.chem.ns()).filter(|&k| w.chem.phase(k, c.f.t[i]) == pcore::chem::Phase::Liquid).map(|k| w.deep.get(k * w.grid.n() + i).copied().unwrap_or(0.0) * w.chem.molar_volume(k, pcore::chem::Phase::Liquid)).sum::<f64>()).sum::<f64>() / 1e9),
            ("rain".to_string(), "mol/m2/s", w.diag.rain.iter().map(|&v| v as f64).sum::<f64>() / n),
            // 16 Oct 2026: the land's liquid in the year, m over the land: rain, evaporation from the ground, from light-eaters
            // and from the other bodies, and what light-eaters took of the ground's liquid (rain caught and soaked).
            ("water_rain".to_string(), "m/yr", self.social.water[0] / land_area),
            ("water_ground".to_string(), "m/yr", self.social.water[1] / land_area),
            ("water_plants".to_string(), "m/yr", self.social.water[2] / land_area),
            ("water_animals".to_string(), "m/yr", self.social.water[3] / land_area),
            ("water_plants_took".to_string(), "m/yr", self.social.water[4] / land_area),
            ("bodies".to_string(), "1", bodies),
            ("plants".to_string(), "1", light_eaters),
            ("animals".to_string(), "1", bodies - light_eaters),
            ("plant_members".to_string(), "1", members[0]),
            ("animal_members".to_string(), "1", members[1]),
            ("animal_nodes".to_string(), "1", nodes),
            ("animal_single".to_string(), "1", bodies - light_eaters - nodes),
            ("cover_median".to_string(), "kg/m2", cover.get(cover.len() / 2).copied().unwrap_or(0.0)),
            ("cover_max".to_string(), "kg/m2", cover.last().copied().unwrap_or(0.0)),
            ("births_plants".to_string(), "1/yr", self.split[0][0]),
            ("births_animals".to_string(), "1/yr", self.split[0][1]),
            ("died_plants".to_string(), "1/yr", self.split[1][0]),
            ("died_animals".to_string(), "1/yr", self.split[1][1]),
            ("eat_animals".to_string(), "1", life.bodies.iter().enumerate().filter(|(i, b)| b.alive && self.social.eaten(b.id(*i)).is_some_and(|t| t[1] > t[0])).count() as f64),
            ("lineages".to_string(), "1", life.lineages() as f64),
            ("updates".to_string(), "1/yr", life.updates.saturating_sub(self.updates) as f64),
            ("lineages_ever".to_string(), "1", life.brains.len() as f64),
            ("births".to_string(), "1/yr", self.births),
            ("died_lack".to_string(), "1/yr", self.deaths[0]),
            ("died_phase".to_string(), "1/yr", self.deaths[1]),
            ("died_spent".to_string(), "1/yr", self.deaths[2]),
            ("died_taken".to_string(), "1/yr", self.deaths[3]),
            ("a_lack".to_string(), "1/yr", self.deaths_a[0]),
            ("a_phase".to_string(), "1/yr", self.deaths_a[1]),
            ("a_spent".to_string(), "1/yr", self.deaths_a[2]),
            ("a_taken".to_string(), "1/yr", self.deaths_a[3]),
            ("a_broken".to_string(), "1/yr", self.deaths_a[4]),
            ("a_age".to_string(), "1/yr", self.deaths_a[5]),
            ("body_mass".to_string(), "t", life.alive().map(|b| b.mass(&w.chem)).sum::<f64>() / 1e3),
        ]);
        // Per starting kind: members, bodies, the members' mean life at their upkeep now (years) and repair gene (the
        // old four), then what happened in the year (members born and died by cause, `life_real`: the standing members
        // over the year over the deaths in it, years), and the means by members: mass (kg), copy mass (kg, the gene: the body it copies
        // at), `matter` (the share of the members' mass that is body matter, 14 Oct 2026), `body` (kg, `Body::size`), the genes
        // (log10, except `hidden`, units) and, for each catalyst gene on in some member of the kind so far, its mean over
        // the members that carry it on (log10) and the share of members that carry it on.
        let ever: BTreeSet<(usize, usize)> = self.series.iter().filter_map(|c| c.0.strip_prefix("root_")?.split_once("_cat_")).filter_map(|(k, r)| Some((k.parse().ok()?, r.parse().ok()?))).collect();
        let (st, dths, births) = (&self.social, &self.social.root_deaths, &self.social.root_births);
        for (k, v) in by_root.iter().enumerate() {
            let per = |x: f64| if v.members > 0.0 { x / v.members } else { f64::NAN };
            let d = dths.get(k).copied().unwrap_or_default();
            let stand = st.root_stand.get(k).copied().unwrap_or(0.0) / st.stand_months.max(1.0);
            let stand_adult = st.root_stand_adult.get(k).copied().unwrap_or(0.0) / st.stand_months.max(1.0);
            let (young, adult) = (st.root_young.get(k).copied().unwrap_or(0.0), st.root_adult.get(k).copied().unwrap_or(0.0));
            let c = |n: &str| format!("root_{k}_{n}");
            row.extend([(c("members"), "1", v.members), (c("kg"), "kg", v.mass), (c("bodies"), "1", v.bodies), (c("life"), "yr", per(v.life)), (c("repair"), "1", per(v.genes[8]))]);
            row.extend(CAUSES.iter().zip(d).map(|(n, x)| (c(&format!("died_{n}")), "1/yr", x)));
            row.extend([
                (c("births"), "1/yr", births.get(k).copied().unwrap_or(0.0)),
                // 15 Oct 2026 (two-parent copying): the share of the year's births with two parents (a partner alike in the cell).
                (c("mixed"), "1", if births.get(k).copied().unwrap_or(0.0) > 0.0 { st.root_mixed.get(k).copied().unwrap_or(0.0) / births[k] } else { f64::NAN }),
                (c("life_real"), "yr", if d.iter().sum::<f64>() > 0.0 { stand / d.iter().sum::<f64>() } else { f64::NAN }),
                (c("mass"), "kg", per(v.mass)),
                (c("copy_mass"), "kg", per(v.copy_mass)),
                (c("matter"), "1", if v.mass > 0.0 { v.matter / v.mass } else { f64::NAN }),
                (c("body"), "kg", per(v.body)),
                (c("past_copy"), "1", per(v.past_copy)),
                (c("died_young"), "1/yr", young),
                (c("died_adult"), "1/yr", adult),
                (c("life_adult"), "yr", if adult > 0.0 { stand_adult / adult } else { f64::NAN }),
                (c("moves"), "1/yr", if stand > 0.0 { st.moves.get(k).copied().unwrap_or(0.0) / stand } else { f64::NAN }),
                // 15 Oct 2026 (partial damage): the body matter hits tore off members who lived on (bites that left them smaller,
                // bodies trampled; fire and the deaths of whole members are not in it), kg a year per standing member.
                (c("hurt"), "kg/yr", if stand > 0.0 { st.hurt.get(k).copied().unwrap_or(0.0) / stand } else { f64::NAN }),
                // 14 Oct 2026: the mean activity of the members (`Body::activity`), the share of them below 0.5, and the `WET` gene.
                (c("activity"), "1", per(v.act)),
                (c("dry"), "1", per(v.dry)),
                (c("wet_gene"), "log10", per(v.wet)),
                // 15 Oct 2026: how much warmer the members are than the ground they stand on (`Body::temp` less the cell's), K, mean by members.
                (c("temp"), "K", per(v.warm)),
                // 14 Oct 2026: the genes of reproduction (means by members: the share of building that goes to offspring, and a
                // child's body as log10 of its share of the copy mass) and the brood as a share of the members' mass.
                (c("alloc"), "1", per(v.alloc)),
                (c("child"), "log10", per(v.child)),
                (c("brood"), "1", if v.mass > 0.0 { v.brood / v.mass } else { f64::NAN }),
                // 15 Oct 2026 (the minerals): the share of the building attempts of the year (members that could build,
                // `copy::grow`) that the store of any mineral held back (`nlim`) and of each (`nlim<q>`), and the mineral each of their
                // stores holds per kg of body (`nstore<q>`).
                (c("nlim"), "1", st.nlim.get(k).map_or(f64::NAN, |x| x.first().filter(|&&t| t > 0.0).map_or(f64::NAN, |t| x[1] / t))),
            ]);
            for q in 0..minerals.len() {
                row.push((c(&format!("nlim{q}")), "1", st.nlim.get(k).map_or(f64::NAN, |x| x.first().filter(|&&t| t > 0.0).map_or(f64::NAN, |t| x[2 + q] / t))));
                row.push((c(&format!("nstore{q}")), "1", if v.body > 0.0 { v.mineral.get(q).copied().unwrap_or(0.0) / v.body } else { f64::NAN }));
            }
            row.extend(GENE_NAMES.iter().zip(&v.genes).map(|(n, &x)| (c(n), match *n { "hidden" => "1", "keep_t" | "cool_t" => "K", _ => "log10" }, per(x))));
            for r in (0..nr).filter(|&r| v.cat[r].1 > 0.0 || ever.contains(&(k, r))) {
                row.push((c(&format!("cat_{r}")), "log10", if v.cat[r].1 > 0.0 { v.cat[r].0 / v.cat[r].1 } else { f64::NAN }));
                row.push((c(&format!("cat_{r}_on")), "1", per(v.cat[r].1)));
            }
        }
        // The land's ground temperature in the warmest and the coldest month of the year, and the swing between them.
        let (hot, cold) = (self.social.t_land.iter().copied().fold(f64::NAN, f64::max), self.social.t_land.iter().copied().fold(f64::NAN, f64::min));
        row.extend([("t_land_summer".to_string(), "K", hot), ("t_land_winter".to_string(), "K", cold), ("t_land_swing".to_string(), "K", hot - cold)]);
        row.extend(self.learning(w));
        row.extend(self.variety(w));
        // Members the monthly merge joined however unlike in the year (the safeguard past twice `crowd`, `body::crowd`).
        row.push(("merged_unlike".to_string(), "1/yr", self.social.unlike));
        // Members that joined a node (merged monthly, or children joining) whose means differ by more than `alike` steps in some gene.
        row.push(("merged_wide".to_string(), "1/yr", self.social.wide));
        // Traffic: share of crossings on the top 2% cells, their overlap with the top cells a
        // decade ago, and the body mass per area there and on other land.
        let (traffic, gn) = (&w.diag.traffic, w.grid.n());
        let mut order: Vec<usize> = (0..gn).filter(|&i| traffic.get(i).is_some_and(|&t| t > 0.0)).collect();
        order.sort_by(|&a, &b| traffic[b].total_cmp(&traffic[a]).then(a.cmp(&b)));
        let top = &order[..(gn / 50).min(order.len())];
        let all: f64 = order.iter().map(|&i| traffic[i] as f64).sum();
        let on_top: f64 = top.iter().map(|&i| traffic[i] as f64).sum();
        let s = &mut self.social;
        if w.day / w.days_per_year() % 10 == 0 && !top.is_empty() {
            let old: BTreeSet<usize> = s.top.iter().copied().collect();
            s.persist = if old.is_empty() { f64::NAN } else { top.iter().filter(|i| old.contains(i)).count() as f64 / top.len() as f64 };
            s.top = top.to_vec();
        }
        // Worn edges: cleared by movers so that crossing costs under half of uncleared ground.
        let now: Vec<u32> = (0..4 * gn).filter(|&e| pcore::choose::eased(w, e / 4, e % 4) > 2.0).map(|e| e as u32).collect();
        let fresh = now.iter().filter(|e| s.worn.binary_search(e).is_err()).count() as f64;
        let widest_ease = (0..4 * gn).map(|e| pcore::choose::eased(w, e / 4, e % 4)).fold(1.0, f64::max);
        if w.day / w.days_per_year() % 10 == 0 {
            s.worn_persist = if s.worn_old.is_empty() { f64::NAN } else { s.worn_old.iter().filter(|e| now.binary_search(e).is_ok()).count() as f64 / s.worn_old.len() as f64 };
            s.worn_old = now.clone();
        }
        s.worn = now;
        let mass = |i: usize| (c.plants[i] + c.animals[i]) as f64;
        let land: Vec<usize> = (0..gn).filter(|&i| c.f.z[i] > 0.0 && !top.contains(&i)).collect();
        let mean = |v: &[usize]| v.iter().map(|&i| mass(i)).sum::<f64>() / v.len().max(1) as f64;
        let groups = life.groups.iter().filter(|g| g.members > 0).count() as f64;
        let shared = life.groups.iter().filter(|g| g.members > 1).map(|g| g.members as f64).sum::<f64>();
        let reciprocal = s.pairs.iter().filter(|&&(a, b)| a < b && s.pairs.contains(&(b, a))).count() as f64;
        let widest = s.spread.values().map(|v| v.len()).max().unwrap_or(0) as f64;
        row.extend([
            ("died_broken".to_string(), "1/yr", self.deaths[4]),
            ("died_age".to_string(), "1/yr", self.deaths[5]),
            ("crossings".to_string(), "1/yr", all),
            ("traffic_top2".to_string(), "1", on_top / all.max(1.0)),
            ("traffic_persist".to_string(), "1", s.persist),
            ("worn_edges".to_string(), "1", s.worn.len() as f64),
            ("worn_new".to_string(), "1/yr", fresh),
            ("worn_persist".to_string(), "1", s.worn_persist),
            ("worn_max".to_string(), "1", widest_ease),
            ("mass_top2".to_string(), "kg/m2", mean(top)),
            ("mass_land".to_string(), "kg/m2", mean(&land)),
            ("groups".to_string(), "1", groups),
            ("in_shared".to_string(), "1", shared / bodies.max(1.0)),
            ("watched".to_string(), "1/yr", s.watched),
            ("watched_kin".to_string(), "1/yr", s.kin_watched),
            ("watched_groups".to_string(), "1", s.spread.len() as f64),
            ("spread_max".to_string(), "groups", widest),
            ("gave".to_string(), "1/yr", s.gave),
            ("gave_kg".to_string(), "kg/yr", s.gave_kg),
            ("gave_eaters".to_string(), "1/yr", s.gave_eaters),
            ("gave_pairs".to_string(), "1", s.pairs.len() as f64),
            ("gave_reciprocal".to_string(), "1", reciprocal),
            ("ate_light_eaters".to_string(), "kg/yr", s.intake[0]),
            ("ate_others".to_string(), "kg/yr", s.intake[1]),
            ("ate_ground".to_string(), "kg/yr", s.intake[2]),
            ("mi_cold".to_string(), "bit", info(&s.sig, 0)),
            ("mi_hurt".to_string(), "bit", info(&s.sig, 1)),
            ("mi_low".to_string(), "bit", info(&s.sig, 2)),
        ]);
        // Made objects (parts) and their bonds.
        let (parts, chem) = (&w.parts, &w.chem);
        let roots: Vec<&pcore::bond::Part> = parts.parts.iter().enumerate().filter(|(r, p)| p.alive && p.root as usize == *r).map(|p| p.1).collect();
        let links = |g: bool| parts.alive().flat_map(|p| &p.links).filter(|l| (l.0 == pcore::bond::GROUND) == g).count() as f64;
        let filled: Vec<f64> = parts.alive().map(|p| p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.molar_mass).sum::<f64>()).filter(|&m| m > 0.0).collect();
        let t = &s.tally;
        let mean = |x: [f64; 2]| x[1] / x[0].max(1.0);
        let share = |c: [f64; 2]| c[1] / c[0].max(1.0);
        row.extend([
            ("parts".to_string(), "1", parts.alive().count() as f64),
            ("parts_held".to_string(), "1", parts.alive().filter(|p| p.held != pcore::bond::NOBODY).count() as f64),
            ("parts_mass".to_string(), "t", parts.alive().map(|p| p.mass(chem)).sum::<f64>() / 1e3),
            ("parts_formed".to_string(), "1/yr", t.formed),
            ("put_to_parts".to_string(), "kg/yr", t.to_parts),
            ("taken_from_parts".to_string(), "kg/yr", t.from_parts),
            ("bonds_joined".to_string(), "1", links(false) / 2.0),
            ("bonds_ground".to_string(), "1", links(true)),
            ("structures".to_string(), "1", roots.iter().filter(|r| r.s.parts > 1).count() as f64),
            ("enclosed".to_string(), "1", roots.iter().filter(|r| r.s.encl >= 0.5).count() as f64),
            ("made_held".to_string(), "1/yr", s.made[0]),
            ("made_joined".to_string(), "1/yr", s.made[1]),
            ("made_ground".to_string(), "1/yr", s.made[2]),
            ("bonds_broken".to_string(), "1/yr", s.broken),
            ("bonds_failed".to_string(), "1/yr", s.failed),
            ("inside_now".to_string(), "1", life.alive().filter(|b| pcore::bond::room(w, b).is_some()).count() as f64),
            ("inside_cold".to_string(), "1", share(s.inside[0])),
            ("inside_warm".to_string(), "1", share(s.inside[1])),
            ("cold_months".to_string(), "1/yr", s.inside[0][0]),
            ("holding_liquid".to_string(), "1", filled.len() as f64),
            ("liquid_held".to_string(), "kg", filled.iter().sum()),
            ("afloat".to_string(), "1", roots.iter().filter(|r| r.s.afloat).count() as f64),
            ("afloat_moves".to_string(), "1/yr", t.afloat),
            ("take_bare".to_string(), "1/yr", t.take[0][0]),
            ("take_held".to_string(), "1/yr", t.take[1][0]),
            ("take_bare_yield".to_string(), "1", mean(t.take[0])),
            ("take_held_yield".to_string(), "1", mean(t.take[1])),
            ("dig_bare".to_string(), "1/yr", t.dig[0][0]),
            ("dig_held".to_string(), "1/yr", t.dig[1][0]),
            ("dig_bare_yield".to_string(), "1", mean(t.dig[0])),
            ("dig_held_yield".to_string(), "1", mean(t.dig[1])),
        ]);
        (s.made, s.broken, s.failed, s.inside, s.intake) = Default::default();
        (s.root_births, s.root_deaths, s.root_stand, s.stand_months, s.t_land) = Default::default();
        s.root_mixed = Default::default();
        (s.root_young, s.root_adult, s.root_stand_adult, s.moves, s.hurt) = Default::default();
        s.nlim = Default::default();
        (s.watched, s.kin_watched, s.gave, s.gave_kg, s.gave_eaters) = (0.0, 0.0, 0.0, 0.0, 0.0);
        (s.spread, s.pairs, s.sig) = Default::default();
        // Mole fraction in the air of each heat-absorbing gas (`air_<substance>`; `absorber` above is
        // their total absorption, so the one that rises shows in these numbers).
        let (na, ns) = (w.grid.na(), w.chem.ns());
        let air: f64 = w.air.iter().sum();
        row.extend((0..ns).filter(|&k| w.chem.subs[k].ir_abs > 0.0).map(|k| (format!("air_{k}"), "1", w.air[k * na..(k + 1) * na].iter().sum::<f64>() / air)));
        // Mean catalyst gene (log10 speed-up; below 0 counts as off, 0) of each reaction over the
        // live bodies, by members: which reactions life speeds up (`cat_<reaction>`).
        let (nr, cat) = (w.chem.reactions.len(), pcore::body::Layout::of(&w.chem).cat);
        let mut g = vec![0.0; nr];
        for b in life.alive() {
            g.iter_mut().enumerate().for_each(|(r, x)| *x += b.genes[cat + r].max(0.0) as f64 * b.n());
        }
        row.extend(g.iter().enumerate().map(|(r, &x)| (format!("cat_{r}"), "log10", x / (members[0] + members[1]).max(1.0))));
        (self.births, self.deaths, self.deaths_a, self.split, self.updates) = (0.0, [0.0; 6], [0.0; 6], [[0.0; 2]; 2], life.updates);
        // The columns of a tape loaded by `resume` may come in another order, or differ (gaps are NaN).
        if self.series.iter().map(|c| &c.0).ne(row.iter().map(|r| &r.0)) {
            let rows = self.series.first().map_or(0, |c| c.2.len());
            let old = std::mem::take(&mut self.series);
            self.series = row.iter().map(|r| (r.0.clone(), r.1, old.iter().find(|c| c.0 == r.0).map_or(vec![f64::NAN; rows], |c| c.2.clone()))).collect();
        }
        for (col, r) in self.series.iter_mut().zip(row) {
            col.1 = r.1;
            col.2.push(r.2);
        }
    }

    /// The latest value of a series column (NaN if there is none), for the run's printed line.
    pub fn last(&self, name: &str) -> f64 {
        self.series.iter().find(|c| c.0 == name).and_then(|c| c.2.last().copied()).unwrap_or(f64::NAN)
    }

    /// Write `world.ewf.gz` (viewer frames) and `series.csv` into `dir`.
    pub fn finish(&self, dir: impl AsRef<Path>) -> io::Result<()> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let (w, h, cell_m, seed, dpy) = self.meta;
        let year = |d: u64| d as f64 / dpy as f64;
        let meta = json!({
            "w": w, "h": h, "cell_m": cell_m, "seed": seed, "days_per_year": dpy,
            "layers": LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect::<Vec<_>>(),
            "frames": self.frames.iter().map(|&d| json!({"day": d, "year": year(d)})).collect::<Vec<_>>(),
            "series": self.series.iter().map(|c| (c.0.clone(), json!(c.2))).collect::<serde_json::Map<_, _>>(),
            "series_units": self.series.iter().map(|c| (c.0.clone(), json!(c.1))).collect::<serde_json::Map<_, _>>(),
            "events": self.log.iter().map(|(d, t)| json!({"day": d, "year": year(*d), "text": t})).collect::<Vec<_>>(),
        });
        let meta = serde_json::to_vec(&meta)?;
        // Four large writes, so no `BufWriter` is needed (for small writes it goes before the compressor, see `save_snapshot`).
        let mut z = GzEncoder::new(std::fs::File::create(dir.join("world.ewf.gz"))?, Compression::default());
        z.write_all(b"EWF1")?;
        z.write_all(&(meta.len() as u32).to_le_bytes())?;
        z.write_all(&meta)?;
        z.write_all(&self.bytes)?;
        z.finish()?.flush()?;

        let mut csv = self.series.iter().map(|c| c.0.as_str()).collect::<Vec<_>>().join(",");
        for row in 0..self.series.first().map_or(0, |c| c.2.len()) {
            csv += &format!("\n{}", self.series.iter().map(|c| c.2[row].to_string()).collect::<Vec<_>>().join(","));
        }
        std::fs::write(dir.join("physics.json"), &self.physics)?;
        // One births column per starting kind seen in any month (a month before a kind's first birth reads 0).
        let nk = self.social.month_births.iter().map(Vec::len).chain(self.social.month_deaths.iter().map(|d| d.len().div_ceil(6))).max().unwrap_or(0);
        let mut months = self.social.months.lines().next().unwrap_or(MONTHS).to_string() + &(0..nk).map(|k| format!(",root_{k}_births")).collect::<String>();
        months += &(0..nk).flat_map(|k| CAUSES.iter().map(move |c| format!(",root_{k}_died_{c}"))).collect::<String>();
        for ((l, b), d) in self.social.months.lines().skip(1).zip(&self.social.month_births).zip(&self.social.month_deaths) {
            months += &format!("\n{l}{}", (0..nk).map(|k| format!(",{}", b.get(k).copied().unwrap_or(0.0))).collect::<String>());
            months += &(0..6 * nk).map(|q| format!(",{}", d.get(q).copied().unwrap_or(0.0))).collect::<String>();
        }
        std::fs::write(dir.join("months.csv"), months + "\n")?;
        std::fs::write(dir.join("series.csv"), csv + "\n")
    }
}
