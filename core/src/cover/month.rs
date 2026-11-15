//! The cover's passes of a month: after its month on the chunks of `life` (`life::month`) its seeds land and what it
//! built is counted (`landed`), and before each air step the light set aside in each cell's aside node that its cover
//! does not claim goes back to the ground (`claims`, `release`; laws.md 1.4).

use crate::store::{apply, Batch, Class};
use crate::World;

/// After the month of every chunk (`life::month`): the cover's seeds land (`cover::land`), and what each cell's cover
/// built is counted into its year (`grown`).
pub fn landed(w: &mut World) {
    super::land(w);
    grown(w);
}

/// What each cell's cover built in the month is added to its year (`Counters::grown_year`); after the year's last
/// month that year is the last whole one (`Counters::grown_last`), for the tape.
fn grown(w: &mut World) {
    let (n, year) = (w.mesh.cells().n(), w.cfg.months_per_year as u64);
    let last = w.month() % year == year - 1;
    let c = &mut w.counts;
    c.grown_year.resize(n, 0.0);
    for (i, kg) in std::mem::take(&mut c.grown) {
        c.grown_year[i as usize] += kg;
    }
    if last {
        c.grown_last = std::mem::take(&mut c.grown_year);
    }
}

/// Per cell, what its cover claims of its aside node, J: the light set aside for it since its last month
/// (`cover::step`), its `Cover::keep` of what its capture area absorbed since.
fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    let Some(p) = super::Parts::of(&w.chem) else { return claim };
    let mm = p.mm(&w.chem);
    for (i, c) in w.life.cells.iter().map(|x| &x.cover).enumerate().filter(|e| e.1.keep > 0.0) {
        let abs: f64 = c.held().map(|x| c.leaf(&mm, x) * (1.0 - c.reflectance(&w.chem, &p, x))).sum();
        claim[i] += c.keep as f64 * abs * (w.light[i] - c.mark);
    }
    claim
}

/// Before an air step: the heat each aside node holds that its cover does not claim (`claims`) goes back to its
/// ground, so the step starts from it.
pub fn release(w: &mut World) {
    let mut back = Batch::new([Class::Aside, Class::Ground], &[]);
    for (i, c) in claims(w).into_iter().enumerate() {
        let free = w.store.heat(Class::Aside)[i] - c;
        if free > 0.0 {
            back.link((i, i), free);
        }
    }
    apply(w, &[back.flux()], &[]);
}
