//! A slot's season in its month (laws.md 3): whether its leaves pay what they cost to keep, its upkeep, and the
//! leaves it sheds, in a season it would cost more to keep them through than to build them anew and in its own shade
//! (`pay`). Each reads the slot's crown in the canopy (`Slot::layer`), whatever slots share it.

use super::budget::SHED;
use super::grow::{Books, Slot};
use super::upkeep::{keep_per_kg, upkeep};
use super::{Cover, Parts, FUEL, MATTER, NP, SLOTS, WALL};
use crate::life::SPENT;
use crate::{Counters, World};

/// Each slot of cover `c` in `order` over `dt` at `t` K, its fine roots leaving it 1 − `Slot::roots` of what a leaf gains: it is in
/// season while what its top leaves gain, less its fine roots, pays what a kg of leaf and the walls that feed it cost to
/// keep for the month (`keep_per_kg`). Out of season it drops first, before it pays the month's upkeep, the leaves it
/// would cost more to keep through the months it expects without gain (`Cover::idle`) than to build anew, and those its
/// fuel could not keep through them (`shed`). It pays its upkeep (`upkeep`), what it lost counted in `counts`; in season
/// a layer whose one more kg would not pay its keep at the bottom of its layer (of its slot `s`) lies in its own shade
/// and is shed; its balance over about a year moves by what it stored (`stored`) less its upkeep. Its top and its depth
/// are its crown's in the canopy (`Slot::layer`), whatever slots share that crown, so a stand cut into slots keeps its
/// season and sheds as the whole. Its parts `p` hold `e` J a mol. Returns whether each pays and whether each is in season.
pub(super) fn pay(w: &World, c: &mut Cover, (p, e): (&Parts, &[f64; NP]), (s, stored): (&mut [Slot; SLOTS], &[f64; SLOTS]), order: &[usize], (t, dt): (f64, f64), (b, counts): (&mut Books, &mut Counters)) -> ([bool; SLOTS], [bool; SLOTS]) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (mm, rho) = (p.mm(chem), chem.subs[p.sub[WALL]].density[0]);
    let (year, rebuild) = (w.cfg.months_per_year as f32, p.living_cost(chem, ph));
    // The months its kinds expect without gain: those of the last year its ground held nothing for roots (`Cover::idle`).
    let months = c.idle as f64 * w.cfg.months_per_year as f64;
    let (mut pays, mut season) = ([false; SLOTS], [false; SLOTS]);
    for &x in order {
        let keep = keep_per_kg(w, c, p, x, (t, dt));
        let net = (1.0 - s[x].roots).max(f64::MIN_POSITIVE);
        season[x] = s[x].gain * net * s[x].layer.exp() > keep;
        if !season[x] {
            // What its fuel can pay: keeping a kg through the months it expects, this one at least.
            let fuel = c.mol[x][FUEL].max(0.0) * e[FUEL] / (s[x].living * keep * months.max(1.0)).max(f64::MIN_POSITIVE);
            let shed = shed(c, x, &s[x], (floor(keep, (keep, rebuild), Some(months)) / net, fuel));
            book_shed(b, counts, (p, &mm, e), shed);
            s[x] = after_shed(ph, c, (&mm, rho), x, s[x]);
        }
        let (lost, cost) = upkeep(w, c, p, x, &s[x], (t, dt), b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        pays[x] = s[x].gain * net > keep;
        c.full[x] = !pays[x];
        if !pays[x] && season[x] {
            let shed = shed(c, x, &s[x], (floor(keep, (keep, rebuild), None) / net, 1.0));
            book_shed(b, counts, (p, &mm, e), shed);
        }
        // Its balance over about a year: what it stored less its upkeep, per kg.
        c.net[x] += (((stored[x] - cost) / s[x].built) as f32 - c.net[x]) / year;
    }
    (pays, season)
}

/// What slot `x` shed, `shed` mol of body matter, goes to its remains, counted as spent (`SPENT`) and booked (`SHED`).
fn book_shed(b: &mut Books, counts: &mut Counters, (p, mm, e): (&Parts, &[f64; NP], &[f64; NP]), shed: f64) {
    counts.cover_lost[SPENT as usize] += shed * mm[MATTER];
    b.dead[p.sub[MATTER]] += shed;
    b.budget[SHED] -= shed * e[MATTER];
}

/// What slot `x` of cover `c` is after it shed some of its living cover: its built and living cover, capture area, what
/// of that absorbs and its conducting walls (`stand::sapwood`) by what it holds now, its walls of density `rho`.
fn after_shed(ph: &crate::Physics, c: &Cover, (mm, rho): (&[f64; NP], f64), x: usize, s: Slot) -> Slot {
    let k = if s.living > 0.0 { c.living(mm, x) / s.living } else { 0.0 };
    let (leaf, height) = (s.leaf * k, c.lift(ph, mm, rho, x));
    Slot { built: c.built(mm, x), living: c.living(mm, x), leaf, abs: s.abs * k, sapwood: super::stand::sapwood(ph, (leaf, height), c.walls(mm, x), rho), ..s }
}

/// The least a layer of a slot's leaves must gain a month, J per kg of living cover, to be kept (Kikuzawa 1991): a leaf
/// is kept while what it loses until it pays again costs less than building it anew, `rebuild` J a kg
/// (`Parts::living_cost`). In season (`months` None: its top leaves pay) a layer that does not pay lies in its own
/// canopy's shade and pays no more, so it must pay its month's upkeep, `per_kg` J a kg. Out of season none of its
/// leaves pay until the season comes back, after the months it expects without gain (`Cover::idle`), over which a kg
/// costs `keep` J a month to keep: it keeps those whose loss over them is less than `rebuild`.
fn floor(per_kg: f64, (keep, rebuild): (f64, f64), months: Option<f64>) -> f64 {
    match months {
        None => per_kg,
        Some(m) => keep - rebuild / m.max(f64::MIN_POSITIVE),
    }
}

/// Slot `x` of cover `c`, whose lowest leaves do not pay for themselves (`Cover::full`), sheds the living cover of the
/// layers of its canopy that gain less than `floor` J a kg (`floor`), as a canopy drops its leaves below its
/// compensation point (Monsi and Saeki 1953; Hirose 2005) and drops them all for a season it would cost more to keep
/// them through than to build them anew (Kikuzawa 1991): its light falls as e^-z with the capture area z above a leaf
/// in its layer (`Slot::layer`, whatever slots hold it, each leaf as deep in it as any), so what one more kg would gain
/// at depth z is its gain at its bottom (`Slot::gain`) times e^(y - z), y the layer's whole depth, and the share of its
/// leaves deeper than where that falls to `floor` goes; of the rest it keeps no more than the share `most` (out of
/// season, those its fuel can keep through the months it expects: what it cannot pay for it drops as leaves rather than
/// die of it as whole plants). A slot that gains nothing keeps all its leaves or none by `floor`, as its layers are
/// alike. Returns the mol of body matter shed (to its remains); its walls, its fuel and its plants stay.
fn shed(c: &mut Cover, x: usize, s: &Slot, (floor, most): (f64, f64)) -> f64 {
    let y = s.layer;
    if y <= 0.0 || (floor <= 0.0 && most >= 1.0) {
        return 0.0;
    }
    let paid = match floor {
        f if f <= 0.0 => 1.0,
        _ if s.gain <= 0.0 => 0.0,
        f => ((y + (s.gain / f).ln()) / y).clamp(0.0, 1.0),
    };
    let keep = paid.min(most);
    let m = c.mol[x][MATTER].max(0.0) * (1.0 - keep);
    c.mol[x][MATTER] -= m;
    m
}
