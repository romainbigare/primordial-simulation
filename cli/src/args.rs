//! The command line: `ew COMMAND --name value ...`, the usage text, and what the flags set.

use pcore::{Config, World};
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;

pub const USAGE: &str = "usage:
  ew run      simulate a new world, write pictures and the tape to --out
    --seed N              world seed (1)
    --size N              cells along a side (128)
    --years N             years to run (10)
    --every N             print and draw a picture every N years (10)
    --out DIR             where the tape, pictures and snapshots go (out)
    --frames-every N      a viewer frame every N years (10)
    --monthly-years N     a viewer frame every month in the first N years (2)
    --life-year N         the year life starts: soil, then the kinds (2; -1: no life)
    --roster              the ten kinds of worldgen::life::ROSTER, each at its year after the first life, not the two
    --clever-year N       the big-brain test at year N: copies of living eaters with a big brain and without (-1: none)
    --clever-n N          eaters copied by the big-brain test (300)
    --spray N             rounds of mutation of every founder (20)
    --spray-size X        step of one round on the row biases of a neutral start (0.2)
    --spray-genes X       the genes' spray, times the step of one generation (3)
    --fixers X            share of the starting light-eaters that carry the fixing gene (0.1)
    --plant-bias          light-eaters start with hand-set brain biases, not neutral brains
    --neutral-eaters      animals start with neutral brains, not hand-set biases
    --own-brains          each body learns alone (no shared brain rows in a group)
    --individuals         animals are never nodes (light-eaters stay nodes): the detailed form
    --no-mix              children have one parent, not two
    --null                learning and imitation off, brain rows frozen: the run to compare a seed with
    --sun X               the sun's strength, times the default (1)
    --absorber X          the heat-absorbing gases of the air at the start, times chemistry 0's (1)
    --flow-days N         days between moves of liquid and creep (15; a multiple of the air's)
    --air-days N          days between moves of the air (3; a multiple of the physics step's, 3)
    --exchange-days N     days between evaporation and condensation (3; a multiple of the air's)
    --regrow X            Config::regrow
    --crowd N             Config::crowd
  ew resume   go on from a snapshot, into the tape in --out, placing what its run's plan still has due
    --from FILE           the snapshot (snap_NNNNN.bin.gz)
    --years N, --every N, --out DIR, --frames-every N, --monthly-years N   as for run
  ew chem     print chemistry 0 as a table
    --seed N              a generated chemistry instead
  ew numbers OLD NEW      the fields of Physics and Config that differ between two runs (directories or physics.json)";

/// Check that the time steps fit together as the laws take them (`pcore::step_marked`, `flow::step`): physics runs
/// every `phys_days`, and the wind, evaporation and liquid each at the first physics step `air_days`, `exchange_days`
/// and `flow_days` after their last. Each must be a whole multiple of `phys_days`, or it runs at another interval than
/// the one asked.
pub fn check_steps(c: &Config) -> Result<(), String> {
    if c.phys_days == 0 {
        return Err("phys_days must be at least 1".into());
    }
    for (name, x, of, y) in [("air_days", c.air_days, "phys_days", c.phys_days), ("exchange_days", c.exchange_days, "phys_days", c.phys_days), ("flow_days", c.flow_days, "phys_days", c.phys_days)] {
        if x == 0 || x % y != 0 {
            return Err(format!("{name} ({x}) must be a whole multiple of {of} ({y})"));
        }
    }
    Ok(())
}

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

    /// The command (the first word after the program's name).
    pub fn command(&self) -> &str {
        self.0.get(1).map_or("help", String::as_str)
    }

    /// Set what the flags of `ew run` change in a new world: the sun, the switches, the time steps, the
    /// heat-absorbing gases.
    pub fn apply(&self, w: &mut World) {
        let c = &mut w.cfg;
        c.sun *= self.get("--sun", 1.0);
        c.share_brains = !self.has("--own-brains");
        c.flow_days = self.get("--flow-days", c.flow_days);
        c.air_days = self.get("--air-days", c.air_days);
        c.exchange_days = self.get("--exchange-days", c.exchange_days);
        c.regrow = self.get("--regrow", c.regrow);
        c.crowd = self.get("--crowd", c.crowd);
        c.null = self.has("--null");
        c.nodes = !self.has("--individuals");
        c.two_parents = !self.has("--no-mix");
        worldgen::terrain::scale_absorbers(w, self.get("--absorber", 1.0));
    }

    /// How the founders are placed: the animals with hand-set brain biases unless `--neutral-eaters`, the
    /// light-eaters with neutral brains unless `--plant-bias`.
    pub fn seeding(&self) -> Seeding {
        let hand = [self.has("--plant-bias"), !self.has("--neutral-eaters")];
        Seeding { hand, rounds: self.get("--spray", 20), size: self.get("--spray-size", 0.2), genes: self.get("--spray-genes", 3.0), fixers: self.get("--fixers", FIXERS) }
    }

    /// What is placed when (`--life-year`, `--roster`, `--clever-year`, `--clever-n`, and the seeding flags).
    pub fn schedule(&self) -> Schedule {
        let year = |name: &str, default: i64| u64::try_from(self.get(name, default)).ok();
        Schedule::new(year("--life-year", 2), self.has("--roster"), year("--clever-year", -1).map(|y| (y, self.get("--clever-n", 300))), self.seeding())
    }
}

#[cfg(test)]
mod tests {
    use super::check_steps;
    use pcore::Config;

    #[test]
    fn the_default_steps_fit_and_a_step_off_its_base_is_refused() {
        assert_eq!(check_steps(&Config::default()), Ok(()));
        for c in [Config { flow_days: 4, ..Default::default() }, Config { exchange_days: 4, ..Default::default() }, Config { air_days: 2, ..Default::default() }, Config { phys_days: 0, ..Default::default() }] {
            assert!(check_steps(&c).is_err(), "{} {} {} {}", c.phys_days, c.air_days, c.exchange_days, c.flow_days);
        }
    }
}
