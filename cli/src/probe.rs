//! PROBE (wip/patch-probe): logs and edits driven by environment variables.
//! EW_PROBE=dir      where the CSVs go (nothing happens without it)
//! EW_CLEAR=A|B|C    A: kill share s of every slot; B: and make it a patch; C: B, and fire makes patches too
//! EW_CLEAR_YEAR=60, EW_CLEAR_S=0.1, EW_CLEAR_N=20

use pcore::cover::{Parts, FUEL, LEAF, LIGHT, MATTER, WALL, WALLS};
use pcore::World;
use std::fs::File;
use std::io::Write;

pub struct Probe {
    dir: String,
    mode: Option<char>,
    year: u64,
    s: f64,
    n: usize,
    cells: Vec<usize>,
    fire: Option<File>,
    slots: Option<File>,
    /// F: the field of each cell (cell, patch)
    field: Vec<(usize, u8)>,
    wild: Option<File>,
}

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

impl Probe {
    pub fn new(w: &World) -> Option<Probe> {
        let dir = std::env::var("EW_PROBE").ok()?;
        std::fs::create_dir_all(&dir).ok()?;
        let mode = std::env::var("EW_CLEAR").ok().and_then(|m| m.chars().next());
        if std::env::var("EW_SLOTS12").is_ok() {
            pcore::cover::patches::MORE_SLOTS.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if mode == Some('C') {
            pcore::cover::patches::FIRE_PATCHES.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut fire = File::create(format!("{dir}/fire.csv")).ok()?;
        writeln!(fire, "year,month,cell,share,length").ok()?;
        let p = Probe { dir, mode, year: env("EW_CLEAR_YEAR", 60), s: env("EW_CLEAR_S", 0.1), n: env("EW_CLEAR_N", 20), cells: Vec::new(), fire: Some(fire), slots: None, field: Vec::new(), wild: None };
        if mode.is_none() {
            p.terrain(w);
        }
        Some(p)
    }

    /// Each cell's terrain samples: index in the cell (col, row), wetness index, loose depth, height.
    fn terrain(&self, w: &World) {
        let mut f = File::create(format!("{}/terrain.csv", self.dir)).unwrap();
        writeln!(f, "cell,land,col,row,twi,loose,h").unwrap();
        let lat = (w.cfg.lat_south, w.cfg.lat_north);
        let s = w.terrain.samples(w.terrain.seed, &w.map, lat);
        let depth = pcore::space::terrain::SAMPLE_DEPTH;
        for (i, l) in w.map.leaves().iter().enumerate() {
            let land = pcore::contract::land(w, i) / w.mesh.cells().area()[i];
            let subs: Vec<_> = l.split(depth).collect();
            let (x0, y0) = (subs.iter().map(|f| f.x).min().unwrap(), subs.iter().map(|f| f.y).min().unwrap());
            let side = subs.iter().map(|f| f.x).filter(|&x| x > x0).min().map_or(1, |x| x - x0);
            for q in subs {
                let k = s.index(&q);
                writeln!(f, "{i},{land:.4},{},{},{:.5},{:.5},{:.3}", (q.x - x0) / side, (q.y - y0) / side, s.twi[k], s.loose[k], s.h[k]).unwrap();
            }
        }
    }

    /// After each month: the fire log.
    pub fn month(&mut self, w: &World, y: u64, m: u32) {
        if let Some(f) = self.fire.as_mut() {
            for (i, fl) in w.counts.flame_month.iter().enumerate().filter(|x| x.1.share > 0.0) {
                writeln!(f, "{y},{m},{i},{},{}", fl.share, fl.length).unwrap();
            }
        }
    }

    /// After each year `y`: at the clearing year pick the cells and clear them; from then the slots of those cells.
    pub fn year(&mut self, w: &mut World, y: u64) {
        let Some(mode) = self.mode else { return };
        if mode != 'A' && mode != 'N' && mode != 'S' {
            pcore::cover::patches::year(w);
        }
        if y == self.year {
            let mut l = File::create(format!("{}/land.csv", self.dir)).unwrap();
            writeln!(l, "cell,land").unwrap();
            (0..w.mesh.cells().n()).for_each(|i| writeln!(l, "{i},{}", pcore::contract::land(w, i) / w.mesh.cells().area()[i]).unwrap());
            self.cells = pick(w, self.n);
            let mut f = File::create(format!("{}/slots.csv", self.dir)).unwrap();
            writeln!(f, "year,phase,cell,slot,patch,pshare,page,root,light,leaf,walls,height,built,kg,count,on,ground,seeds").unwrap();
            self.slots = Some(f);
            self.dump(w, y, "pre");
            let s = self.s;
            for &i in &self.cells.clone() {
                if mode == 'F' {
                    pcore::cover::patches::clear(w, i, s, true);
                    let c = w.life.cover(i);
                    let k = (0..pcore::cover::PATCHES).filter(|&k| c.pshare[k] > 0.0).min_by(|&a, &b| c.pshare[a].total_cmp(&c.pshare[b])).unwrap() as u8;
                    let genes = worldgen::life::KINDS[0].genes;
                    let ok = pcore::cover::place_in(w, i, genes, 0, worldgen::life::KINDS[0].kg, k);
                    println!("field: cell {i} patch {k} share {} sown {ok}", w.life.cover(i).pshare[k as usize]);
                    self.field.push((i, k));
                } else if mode == 'S' {
                    assert!(pcore::cover::patches::split(w, i, s), "split {i}");
                } else if mode != 'N' {
                    pcore::cover::patches::clear(w, i, s, mode != 'A');
                }
            }
            let mut c = File::create(format!("{}/cells.txt", self.dir)).unwrap();
            self.cells.iter().for_each(|i| writeln!(c, "{i}").unwrap());
        }
        if y >= self.year {
            self.dump(w, y, "post");
        }
        if y > self.year && mode == 'F' {
            let mut killed = 0.0;
            for &(i, k) in &self.field {
                if w.life.cover(i).pshare[k as usize] > 0.0 {
                    killed += pcore::cover::patches::weed(w, i, k, 0);
                }
            }
            println!("year {y}: weeding killed {killed:.3e} kg");
        }
        if y >= self.year {
            self.wild_dump(w, y);
        }
        // The seeds of the year are read: start the next year's count.
        pcore::cover::patches::reset_seeded(w);
        let h = &pcore::cover::patches::HITS;
        if y % 10 == 0 {
            println!("year {y}: founders capped in their patch {}, founders in a full cell {}", h[0].load(std::sync::atomic::Ordering::Relaxed), h[1].load(std::sync::atomic::Ordering::Relaxed));
        }
    }

    /// Yearly, every cell of one patch where the first kind grows: its seeds of the year and its cover, kg.
    fn wild_dump(&mut self, w: &World, y: u64) {
        let Some(p) = Parts::of(&w.chem) else { return };
        let mm = p.mm(&w.chem);
        if self.wild.is_none() {
            let mut f = File::create(format!("{}/wild.csv", self.dir)).unwrap();
            writeln!(f, "year,cell,patches,ground,seeds,kg,on").unwrap();
            self.wild = Some(f);
        }
        let f = self.wild.as_mut().unwrap();
        for i in 0..w.mesh.cells().n() {
            let c = w.life.cover(i);
            let xs: Vec<usize> = c.held().filter(|&x| c.root[x] == 0).collect();
            if xs.is_empty() {
                continue;
            }
            let np = c.pshare.iter().filter(|&&s| s > 0.0).count();
            let seeds: f64 = xs.iter().map(|&x| c.seeded[x][0] * mm[FUEL] + c.seeded[x][1] * mm[MATTER]).sum();
            let kg: f64 = xs.iter().map(|&x| c.kg(&mm, x)).sum();
            let on = xs.iter().map(|&x| c.on(x)).fold(0.0, f64::max);
            writeln!(f, "{y},{i},{np},{},{seeds},{kg},{on}", pcore::cover::ground(w, i)).unwrap();
        }
    }

    fn dump(&mut self, w: &World, y: u64, phase: &str) {
        let Some(p) = Parts::of(&w.chem) else { return };
        let (mm, rho, ph) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0], &w.cfg.phys);
        let f = self.slots.as_mut().unwrap();
        for &i in &self.cells {
            let c = w.life.cover(i);
            let ground = pcore::cover::ground(w, i);
            // A row for each patch even when it holds no slot (slot -1).
            for k in 0..pcore::cover::PATCHES {
                if c.pshare[k] > 0.0 {
                    writeln!(f, "{y},{phase},{i},-1,{k},{},{},0,0,0,0,0,0,0,0,0,{ground},0", c.pshare[k], c.page[k]).unwrap();
                }
            }
            for x in c.held() {
                let g = c.genes[x];
                let seeds = c.seeded[x][0] * mm[FUEL] + c.seeded[x][1] * mm[MATTER];
                writeln!(f, "{y},{phase},{i},{x},{},{},{},{},{},{},{},{:.4},{},{},{},{},{ground},{seeds}", c.patch[x], c.pshare[c.patch[x] as usize], c.page[c.patch[x] as usize], c.root[x], g[LIGHT], g[LEAF], g[WALLS], c.height(ph, &mm, rho, x), c.built(&mm, x), c.kg(&mm, x), c.count(x), c.on(x)).unwrap();
            }
        }
    }
}

/// The `n` land cells of the tallest closed cover: capture area at least `CLOSED` m² a m² of ground, by the height of
/// their tallest slot.
fn pick(w: &World, n: usize) -> Vec<usize> {
    let closed: f64 = env("EW_PICK_CLOSED", 3.0);
    let p = Parts::of(&w.chem).unwrap();
    let (mm, rho, ph) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0], &w.cfg.phys);
    let mut v: Vec<(f64, usize)> = (0..w.mesh.cells().n())
        .filter(|&i| pcore::contract::land(w, i) > 0.0)
        .filter_map(|i| {
            let c = w.life.cover(i);
            let lai = c.held().map(|x| c.leaf(&mm, x)).sum::<f64>() / pcore::cover::ground(w, i).max(1e-9);
            let top = c.held().map(|x| c.height(ph, &mm, rho, x)).fold(0.0, f64::max);
            let most: usize = env("EW_PICK_MAXSLOTS", 99);
            (lai >= closed && c.held().count() <= most).then_some((top, i))
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().take(n).map(|x| x.1).collect()
}
