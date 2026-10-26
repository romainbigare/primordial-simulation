//! The command line: `ew COMMAND --name value ...`, the usage text, and what the flags set.

use pcore::World;
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;

pub const USAGE: &str = "usage:
  ew run      simulate a new world, write pictures and the tape to --out
    --seed N              world seed (1)
    --size N              cells along a side (128); of a globe, its rows from pole to pole (even)
    --globe               a globe whose rows wrap east to west, twice as many columns as rows (not an island)
    --region X,Y,W,H      the W x H cells from column X and row Y split in four (each side 2 or more; none)
    --years N             years to run (10)
    --every N             print and draw a picture every N years (10)
    --out DIR             where the tape, pictures and snapshots go (out)
    --frames-every N      a viewer frame every N years (10)
    --monthly-years N     a viewer frame every month in the first N years (2)
    --life-year N         the year life starts: soil, then the kinds (2; -1: no life)
    --roster              the ten kinds of worldgen::life::ROSTER, each at its year after the first life, not the two
    --spray N             rounds of mutation of every founder (20)
    --fixers X            share of the starting light-eaters that carry the fixing gene (0.1)
    --neutral-eaters      animals start with neutral brains, not hand-set biases
    --individuals         bodies with a brain are never nodes (those without stay nodes): the detailed form
    --no-mix              children have one parent, not two
    --null                learning and imitation off, brain rows frozen: the run to compare a seed with
    --sun X               the sun's strength, times the default (1)
    --absorber X          the heat-absorbing gases of the air at the start, times chemistry 0's (1)
    --phys-days N         days between surveys of the cells and steps of the reactions (10; a multiple of the bodies' batch, 10 days)
    --air-hours N         hours between steps of the air: radiation, evaporation, rain and wind (240; divides the physics step)
    --flow-days N         days between moves of liquid (30; a whole number of air steps)
    --crowd N             past N body nodes in a cell, the most alike merge each month (4; Config::crowd)
    --node-days N         a population node without a brain updates at most every N days (40; a multiple of the bodies' batch, 10 days; Config::node_days)
    --sample-every N      a node without a brain runs its sample member on one update in N (2; Config::sample_every)
    --think-days N        a population node with a brain decides about every N days, by habit between (20; 0: every update; Config::think_days)
  ew resume   go on from a snapshot, into the tape in --out, placing what its run's plan still has due
    --from FILE           the snapshot (snap_NNNNN.bin.gz)
    --years N, --every N, --out DIR, --frames-every N, --monthly-years N   as for run
  ew chem     print chemistry 0 as a table
    --seed N              a generated chemistry instead
  ew numbers OLD NEW      the fields of Physics and Config that differ between two runs (directories or physics.json)";

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

    /// The refined region of `--region X,Y,W,H`, if it is there and parses.
    pub fn region(&self) -> Option<[usize; 4]> {
        let v: Vec<usize> = self.value::<String>("--region")?.split(',').map(str::parse).collect::<Result<_, _>>().ok()?;
        v.try_into().ok()
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

    /// Set what the flags of `ew run` change in a new world: the sun, the switches, the time steps, the
    /// heat-absorbing gases.
    pub fn apply(&self, w: &mut World) {
        let c = &mut w.cfg;
        c.sun *= self.get("--sun", 1.0);
        c.phys_days = self.get("--phys-days", c.phys_days);
        c.flow_days = self.get("--flow-days", c.flow_days);
        c.air_hours = self.get("--air-hours", c.air_hours);
        c.crowd = self.get("--crowd", c.crowd);
        c.node_days = self.get("--node-days", c.node_days);
        c.sample_every = self.get("--sample-every", c.sample_every);
        c.think_days = self.get("--think-days", c.think_days);
        c.null = self.has("--null");
        c.nodes = !self.has("--individuals");
        c.two_parents = !self.has("--no-mix");
        worldgen::terrain::scale_absorbers(w, self.get("--absorber", 1.0));
    }

    /// How the founders are placed: the animals with hand-set brain biases unless `--neutral-eaters` (the light-eaters
    /// have no brain).
    pub fn seeding(&self) -> Seeding {
        let hand = !self.has("--neutral-eaters");
        Seeding { hand, rounds: self.get("--spray", 20), fixers: self.get("--fixers", FIXERS) }
    }

    /// What is placed when (`--life-year`, `--roster`, and the seeding flags).
    pub fn schedule(&self) -> Schedule {
        let year = |name: &str, default: i64| u64::try_from(self.get(name, default)).ok();
        Schedule::new(year("--life-year", 2), self.has("--roster"), self.seeding())
    }
}
