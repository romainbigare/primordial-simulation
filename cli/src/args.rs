//! The command line: `ew COMMAND --name value ...`, the usage text, and what the flags set.

use pcore::space::terrain::Shape;
use pcore::{Config, World};
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;

pub const USAGE: &str = "usage:
  ew run      simulate a new world, write the tape and a snapshot at the end to --out
    --seed N              world seed (1)
    --size N              cells along a side (128); of a globe, its rows from pole to pole (even)
    --globe               a globe whose rows wrap east to west, twice as many columns as rows (not an island)
    --band S,N            an island's rows from latitude S to latitude N, degrees (40,42.3): its air the air over
                          that band, its edges open on the planet's air (laws.md 1.6)
    --shape NAME          an island's large shape: drawn (one to several rises drawn from the seed: several lands, a
                          ring, a long land) or one (one rise in the middle of the map) (drawn)
    --weather NAME        an island's weather: strong (the eddies' measured speed, 6 m/s) or soft (the speed their
                          diffusion alone gives, 2 m/s: worldgen::terrain::soft_weather) (strong)
    --fine X,Y,W,H        on a globe, the W x H air blocks from column X and row Y in squares, the rest one cell an
                          air block (none)
    --years N             years to run (10)
    --every N             print a line every N years (10)
    --out DIR             where the tape, pictures and snapshots go (out)
    --frames-every N      a viewer frame every N years (10)
    --snapshots-every N   a snapshot every N years (0: only at the end)
    --snapshots-at Y,Y    a snapshot also at each of these dates, years since the start with whole months (42.75:
                          9 months into year 42), as snap_YYYYY_MM.bin.gz (the page opens them; ew resume does not
                          go on from inside a year)
    --pictures-every N    a PNG picture every N years, and at the start and the end (0: none)
    --monthly-years N     a viewer frame every month in the first N years (2)
    --life-year N        the year life starts: soil, then the kinds (2; -1: no life)
    --fauna-year N        the year the starting kinds of takers are placed (-1: none, the default)
    --roster              the six kinds of cover of worldgen::life::ROSTER, each at its year after the first life, not
                          the one
    --spray N             rounds of mutation of the cover of every cell placed (20)
    --fixers X            share of the starting light-eaters that carry the fixing gene (0.1)
    --sun X               the sun's strength, times the default (1)
    --absorber X          the heat-absorbing gases of the air at the start, times chemistry 0's (1)
    --variety N           kinds of cover a cell may hold, a setting of the world from 1 to 12 (cover::SLOTS) (8)
    --chunks N            a speed setting: the cells cut into N runs for the cover's parallel passes (64; any count
                          gives one world)
  ew resume   go on from a snapshot, into the tape in --out, placing what its run's plan still has due
    --from FILE           the snapshot (snap_NNNNN.bin.gz)
    --years N, --every N, --out DIR, --frames-every N, --snapshots-every N, --snapshots-at Y,Y, --pictures-every N,
    --monthly-years N, --chunks N
                          as for run
  ew view     draw a window of a snapshot's map as a PNG, each pixel the colour of the cell that holds its centre
    --from FILE           the snapshot (snap_NNNNN.bin.gz)
    --window X,Y,W,H      the window from column X and row Y, W x H squares, fractions allowed (the whole map)
    --px N                pixels across; the height follows the window (512)
    --layer NAME          map, temperature, rain, life, depth, land (the map with no rivers), kinds (the starting
                          kind whose cover leads each cell), or of the loose ground: conductivity, water (what it
                          holds for roots), yield (rain less what rises), wetness (of its pores), rise (what rises
                          from it), soil (the cell's texture), organic (its organic share), minerals (it holds),
                          erodibility, slow (its slow organic matter), rot (how fast its dead frame rots), texture
                          (the texture of the terrain's sample under each pixel) (map)
    --edges               darken the edges between cells
    --out FILE            the PNG (view.png)
  ew cells    write a table of a snapshot's cells as CSV, a row a cell: area, open land, height, frozen cover (m),
              ground temperature, the year's rain and what rises (mm), cover (kg/m²) and its leading starting kind,
              and the loose ground's texture, pores, depth, liquid for roots, wetness, slow organic matter, minerals
              and erodibility
    --from FILE           the snapshot (snap_NNNNN.bin.gz)
    --out FILE            the table (cells.csv)
  ew card     write the test card, a small island whose squares are set by hand (worldgen::testkit::card), as a
              snapshot the page opens; nothing is placed later
    --out FILE            the snapshot (card.bin.gz)
  ew chem     print chemistry 0 as a table
  ew numbers OLD NEW      the fields of Physics and Config that differ between two runs (directories or physics.json)";

/// Years between viewer frames when `--frames-every` is not given.
pub const FRAMES_EVERY: u64 = 10;

/// Years between snapshots when `--snapshots-every` is not given (0: only at the end of the run).
pub const SNAPSHOTS_EVERY: u64 = 0;

/// Years between PNG pictures when `--pictures-every` is not given (0: none).
pub const PICTURES_EVERY: u64 = 0;

/// How far a date of `--snapshots-at` may lie from a whole month, months: the rounding of a decimal year (42.75).
const WHOLE_MONTH: f64 = 1e-6;

/// The command line's words.
pub struct Args(pub Vec<String>);

impl Args {
    pub fn has(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }

    /// The value after `name`, if it is there and parses.
    pub fn value<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.0.iter().position(|a| a == name).and_then(|p| self.0.get(p + 1)).and_then(|v| v.parse().ok())
    }

    /// The value after `name`, or `default`.
    pub fn get<T: std::str::FromStr>(&self, name: &str, default: T) -> T {
        self.value(name).unwrap_or(default)
    }

    /// The air blocks of `--fine X,Y,W,H` (none if it is not there or does not parse).
    pub fn fine(&self) -> [usize; 4] {
        let v: Option<Vec<usize>> = self.value::<String>("--fine").and_then(|f| f.split(',').map(|x| x.parse().ok()).collect());
        v.and_then(|v| v.try_into().ok()).unwrap_or_default()
    }

    /// The settings of a new island: those of the band of `--band S,N` (`worldgen::terrain::banded`; the default's if
    /// it is not there) with the weather of `--weather NAME`; an error if one does not parse or the band is not one
    /// (`Config::validate`).
    pub fn island(&self) -> Result<Config, String> {
        let mut cfg = Config::default();
        if let Some(text) = self.value::<String>("--band") {
            let v: Result<Vec<f64>, _> = text.split(',').map(str::parse).collect();
            let [south, north] = v.ok().and_then(|v| v.try_into().ok()).ok_or(format!("--band {text}: want S,N in degrees"))?;
            cfg = worldgen::terrain::banded((south, north));
        }
        match self.value::<String>("--weather").as_deref() {
            None | Some("strong") => {}
            Some("soft") => cfg = worldgen::terrain::soft_weather(cfg),
            Some(other) => return Err(format!("--weather {other}: want strong or soft")),
        }
        cfg.validate().map(|_| cfg)
    }

    /// The large shape of a new island of `--shape NAME` (`Shape::Drawn` if it is not there); an error if it is not one.
    pub fn shape(&self) -> Result<Shape, String> {
        match self.value::<String>("--shape").as_deref() {
            None | Some("drawn") => Ok(Shape::Drawn),
            Some("one") => Ok(Shape::One),
            Some(other) => Err(format!("--shape {other}: want one or drawn")),
        }
    }

    /// The window of `--window X,Y,W,H` in squares, or `whole` if it is not there; an error if it does not parse.
    pub fn window(&self, whole: [f64; 4]) -> Result<[f64; 4], String> {
        let Some(text) = self.value::<String>("--window") else { return Ok(whole) };
        let v: Result<Vec<f64>, _> = text.split(',').map(str::parse).collect();
        v.ok().and_then(|v| v.try_into().ok()).filter(|v: &[f64; 4]| v[2] > 0.0 && v[3] > 0.0).ok_or(format!("--window {text}: want X,Y,W,H with W and H over 0"))
    }

    /// The dates of `--snapshots-at Y,Y,...` as months since the start, `per_year` months a year (none if it is not
    /// there); an error if one does not parse or is not a whole month.
    pub fn snapshot_months(&self, per_year: u64) -> Result<Vec<u64>, String> {
        let Some(text) = self.value::<String>("--snapshots-at") else { return Ok(Vec::new()) };
        let month = |y: &str| {
            let m = y.parse::<f64>().ok().filter(|y| *y > 0.0).map(|y| y * per_year as f64);
            m.filter(|m| (m - m.round()).abs() < WHOLE_MONTH).map(|m| m.round() as u64).ok_or(format!("--snapshots-at {text}: {y} is not a date after the start in whole months"))
        };
        text.split(',').map(month).collect()
    }

    /// The command (the first word after the program's name).
    pub fn command(&self) -> &str {
        self.0.get(1).map_or("help", String::as_str)
    }

    /// Whether to print the usage: no command, `help`, or `--help` anywhere.
    pub fn wants_help(&self) -> bool {
        ["help", "--help", "-h"].contains(&self.command()) || self.has("--help")
    }

    /// Check the words after the command against its part of `USAGE`: the words it names first (`OLD NEW`), then only
    /// its flags, each with its value where the usage writes one in capitals (`N`, `DIR`). Returns the first word that
    /// does not fit.
    pub fn check(&self) -> Result<(), String> {
        let cmd = self.command();
        let part = USAGE.split("\n  ew ").skip(1).find(|p| p.split_whitespace().next() == Some(cmd)).ok_or(format!("unknown command {cmd}"))?;
        let words: Vec<&str> = part.split_whitespace().map(|w| w.trim_end_matches(',')).collect();
        let value = |w: &&str| w.starts_with(|c: char| c.is_ascii_uppercase());
        let takes = |flag: &str| words.windows(2).any(|p| p[0] == flag && value(&p[1]));
        let mut rest = self.0.iter().skip(2 + words[1..].iter().take_while(|w| value(w)).count());
        while let Some(a) = rest.next() {
            if !a.starts_with("--") || !words.contains(&a.as_str()) {
                return Err(format!("ew {cmd}: unknown word {a}"));
            }
            if takes(a) && rest.next().is_none() {
                return Err(format!("ew {cmd}: {a} needs a value"));
            }
        }
        Ok(())
    }

    /// Set what the flags of `ew run` change in a new world: the sun, the heat-absorbing gases and the kinds of cover a
    /// cell may hold (`Config::variety`, checked with the rest by `Config::validate`).
    pub fn apply(&self, w: &mut World) {
        w.cfg.sun *= self.get("--sun", 1.0);
        w.cfg.variety = self.get("--variety", w.cfg.variety);
        worldgen::terrain::scale_absorbers(w, self.get("--absorber", 1.0));
    }

    /// How the cover is placed (`--spray`, `--fixers`).
    pub fn seeding(&self) -> Seeding {
        Seeding { rounds: self.get("--spray", 20), fixers: self.get("--fixers", FIXERS) }
    }

    /// The run's plan: what is placed when (`--life-year`, `--roster`, and the seeding flags).
    pub fn schedule(&self) -> Schedule {
        let year = |name: &str, default: i64| u64::try_from(self.get(name, default)).ok();
        let rows: &[usize] = if self.has("--roster") { &worldgen::life::ROSTER } else { &worldgen::life::ONE };
        let life = year("--life-year", 2);
        Schedule { takers: year("--fauna-year", -1), ..Schedule::new(life, rows, self.seeding()) }
    }
}
