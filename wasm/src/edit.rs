//! The live world's edits (god mode, handover.md 11): what the page's tools read of a place, the brush and its
//! strokes, their settling and undo, and the preview of the ground they moved, for the page's ground to draw until it
//! is made again.

use crate::{give, with, Live};
use pcore::World;
use serde_json::json;

/// A stroke of the brush under way: where it plants, and the starting kind (planted when it ends, `Live::settle`);
/// where it moved the ground, each paint's place, radius, m, and height at its centre, m (`Act::Ground`); the make of
/// the loose ground it lays (`World::draw_make`), the world as it was before it began, whether one of its edits was
/// made (then that world is the one an undo puts back), and the samples its paints may have moved.
pub(crate) struct Stroke {
    planting: Vec<pcore::edit::Brush>,
    row: usize,
    raises: Vec<([f64; 2], f64, f64)>,
    make: u8,
    before: World,
    changed: bool,
    moved: Area,
}

/// The most samples past its brush's radius an edit moves the ground: a basin's rim, 1.5 samples (`pcore::edit`'s
/// basin), and the sample a brush's span rounds out to.
const EDIT_REACH_SAMPLES: f64 = 3.0;

/// A box of the terrain's samples: its west and north columns and rows, and its east and south ones past it (none:
/// west not before east).
#[derive(Clone, Copy)]
pub(crate) struct Area([usize; 4]);

impl Area {
    pub(crate) const NONE: Area = Area([usize::MAX, usize::MAX, 0, 0]);
    /// The samples (of `across` x `down`, `per` a square's side) the brush `b` may move the ground of, on squares of
    /// `side_m` m: those within its radius and `EDIT_REACH_SAMPLES`.
    fn of(b: pcore::edit::Brush, (across, down, per): (usize, usize, f64), side_m: f64) -> Area {
        let r = b.radius_m / side_m * per + EDIT_REACH_SAMPLES;
        let [cx, cy] = b.centre.map(|v| v * per);
        let span = |c: f64, n: usize| ((c - r).floor().max(0.0) as usize, ((c + r).ceil().max(0.0) as usize + 1).min(n));
        let ((x0, x1), (y0, y1)) = (span(cx, across), span(cy, down));
        Area([x0, y0, x1, y1])
    }
    /// The box that holds both.
    fn or(self, o: Area) -> Area {
        let (a, b) = (self.0, o.0);
        Area([a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
    }
    /// Its samples, row by row, as indices of a grid `across` wide.
    fn samples(self, across: usize) -> impl Iterator<Item = usize> {
        let [x0, y0, x1, y1] = self.0;
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| y * across + x))
    }
}

impl Stroke {
    /// How far its paints so far moved the ground at `place` (squares from the map's north-west corner, on squares of
    /// `side` m), m: each by its height at its centre and `edit::falloff` of its distance from it.
    fn raised_at(&self, place: [f64; 2], side: f64) -> f64 {
        self.raises.iter().map(|&(at, r, h)| h * pcore::edit::falloff((place[0] - at[0]).hypot(place[1] - at[1]) * side / r)).sum()
    }
}

impl Live {
    /// End the stroke under way: its planting made (`plant`), the world's edits settled (`World::settle`); the world
    /// before it kept for an undo where it changed the world, else for the next stroke (`ready`: it stands as the
    /// world does), so no copy is made here. Returns what settling did, why its planting placed nothing ("" where it
    /// placed some, or there was none), and whether the stroke can be undone.
    pub(crate) fn settle(&mut self) -> (pcore::edit::Settled, String, bool) {
        let (mut unplanted, mut changed) = (String::new(), false);
        if let Some(mut st) = self.stroke.take() {
            if !st.planting.is_empty() {
                let (planted, why) = self.plant(&st.planting, st.row);
                (unplanted, st.changed) = (why, st.changed || planted);
            }
            if st.changed {
                self.before = Some((st.before, st.moved));
            } else {
                self.ready = Some(st.before);
            }
            changed = st.changed;
        }
        let done = self.world.settle();
        self.follow();
        (done, unplanted, changed)
    }

    /// Of each cell under the brushes `planting` (`World::under`), the share of it they covered cleared about its
    /// sample nearest one's centre, and the starting kind of row `row` placed in it as the world's grown stand of it
    /// (`Act::Grown`; where none stands yet, as seedlings). Whether any cell was planted, and where none was the reason
    /// most cells give ("NoLand" or "NoRoom").
    fn plant(&mut self, planting: &[pcore::edit::Brush], row: usize) -> (bool, String) {
        use pcore::edit::{Act, Edit};
        let under = self.world.under(planting);
        let edits: Vec<Edit> = under.iter().flat_map(|u| [Act::Clear(u.share), worldgen::schedule::grown(row)].map(|act| Edit { place: u.place, act })).collect();
        let done = self.world.edit(&edits);
        self.unframed |= done.iter().any(|&d| d);
        let planted = done.chunks(2).any(|d| d.iter().all(|&ok| ok));
        if planted || under.is_empty() {
            return (planted, String::new());
        }
        let wet = under.iter().filter(|u| pcore::contract::land(&self.world, u.cell) <= 0.0).count();
        (false, String::from(if 2 * wet >= under.len() { "NoLand" } else { "NoRoom" }))
    }
}

/// What the page's edits read of the cell under the place `x`, `y` (squares from the map's north-west corner), as JSON:
/// the cell, its edges in squares [west, north, east, south], its land, m², the patches of its cover, and whether heat
/// added there lights it (`flame::needed`: "yes", or the reason it does not, `flame::Unlit`).
#[no_mangle]
pub extern "C" fn probe(x: f64, y: f64) -> *const u8 {
    let out = with(|l| {
        let w = &l.world;
        let i = w.cell_under([x, y]);
        let fire = pcore::flame::needed(w, i, 1.0).map_or_else(|why| format!("{why:?}"), |_| String::from("yes"));
        json!({"cell": i, "bounds": w.map.bounds(i), "land": pcore::contract::land(w, i), "patches": w.life.cover(i).patch_count(), "fire": fire})
    });
    give(out.to_string().into_bytes())
}

/// The starting kinds the world's plan places, as JSON: each its row of `worldgen::life::KINDS` and its name.
#[no_mangle]
pub extern "C" fn kinds() -> *const u8 {
    let out = with(|l| {
        let rows: Vec<_> = l.plan.kinds.iter().map(|&(row, _)| json!({"row": row, "name": worldgen::life::KINDS[row].name})).collect();
        serde_json::Value::Array(rows)
    });
    give(out.to_string().into_bytes())
}

/// The brush's tools (`brush`): raise or lower the ground, make a lake, plant (a clearing about the place nearest the
/// brush's centre in each cell, planted with a grown stand of a starting kind) and burn (heat added to the remains under
/// it).
const SHAPE: u32 = 0;
const LAKE: u32 = 1;
const PLANT: u32 = 2;
const BURN: u32 = 3;

/// The brush of radius `radius` m about the place `x`, `y` in squares (`World::under`), with the tool `tool`, between
/// two months, by `amount` (`shape`, `lake`, `burn`) or of the starting kind of row `row` (`keep_planting`). The first
/// call of a stroke begins it (`Stroke`), with the world as it was (`ready`, else a copy) for its undo. Returns, as
/// JSON, what the tool says (`cells`, whether it changed the world), and what the page is to be sent: the preview of
/// the ground moved (`preview`) and a frame of the world (`frame`); or, for a tool the shell has not, why it is refused
/// (`refused`), and nothing is done.
#[no_mangle]
pub extern "C" fn brush(tool: u32, x: f64, y: f64, radius: f64, amount: f64, row: u32) -> *const u8 {
    if !matches!(tool, SHAPE | LAKE | BURN | PLANT) {
        return give(json!({"refused": format!("The core has no brush tool {tool}.")}).to_string().into_bytes());
    }
    let out = with(|l| {
        if l.stroke.is_none() {
            let before = l.ready.take().unwrap_or_else(|| l.world.clone());
            l.strokes += 1;
            l.stroke = Some(Stroke { planting: Vec::new(), row: 0, raises: Vec::new(), make: l.world.draw_make(l.strokes), before, changed: false, moved: Area::NONE });
        }
        let b = pcore::edit::Brush { centre: [x, y], radius_m: radius };
        match tool {
            SHAPE => shape(l, b, amount),
            LAKE => lake(l, b, amount),
            BURN => burn(l, b, amount),
            // (`PLANT`: a tool the shell has not is refused above.)
            _ => keep_planting(l, b, row),
        }
    });
    give(out.to_string().into_bytes())
}

/// The edit `act` at the brush `b`'s centre in the stroke under way: whether it was made.
fn make(l: &mut Live, b: pcore::edit::Brush, act: pcore::edit::Act) -> bool {
    let done = l.world.edit(&[pcore::edit::Edit { place: b.centre, act }])[0];
    l.stroke.as_mut().expect("a stroke under way").changed |= done;
    l.unframed |= done;
    done
}

/// The samples the brush `b` may have moved the ground of kept, in the stroke under way and in the newest version
/// (`Built::moved`), for `preview`.
fn moved_ground(l: &mut Live, b: pcore::edit::Brush) {
    let s = l.world.terrain_samples();
    let per = (s.across / l.world.map.w) as f64;
    let area = Area::of(b, (s.across, s.down, per), l.world.map.side);
    let st = l.stroke.as_mut().expect("a stroke under way");
    st.moved = st.moved.or(area);
    let last = l.since.last_mut().expect("a version");
    last.moved = last.moved.or(area);
}

/// The ground under the brush `b` raised by `height` m at its centre, less toward its edge (lowered where under none;
/// `Act::Ground`), its loose top of the stroke's make on what the stroke raised where the brush is now. The page draws
/// it from the preview until the stroke settles. It names no cells (the page reads whether it changed the world).
fn shape(l: &mut Live, b: pcore::edit::Brush, height: f64) -> serde_json::Value {
    let st = l.stroke.as_mut().expect("a stroke under way");
    let act = pcore::edit::Act::Ground { radius_m: b.radius_m, height_m: height, stroke_m: st.raised_at(b.centre, l.world.map.side).max(0.0), make: st.make };
    st.raises.push((b.centre, b.radius_m, height));
    let done = make(l, b, act);
    moved_ground(l, b);
    json!({"cells": [], "changed": done, "preview": true, "frame": false})
}

/// A lake under the brush `b` at once, its floor `depth` m under its rim, full to its pass (`Act::Basin`: the samples
/// are built again and the liquid settles; the page asks for the new relief): a frame, and the preview until the
/// page's ground shows it. It names no cells, as `shape`.
fn lake(l: &mut Live, b: pcore::edit::Brush, depth: f64) -> serde_json::Value {
    let done = make(l, b, pcore::edit::Act::Basin { radius_m: b.radius_m, depth_m: depth });
    l.follow();
    moved_ground(l, b);
    json!({"cells": [], "changed": done, "preview": true, "frame": done})
}

/// `joules_m2` J a m² of heat added under the brush `b` to the remains lying there (`Act::Heat`), which dries and lights
/// them when the stroke settles (the page calls it again and again while the brush is held, each time with what the
/// time held since adds): each cell under it, the share it covers, how much of the heat that lights its remains it
/// holds (`progress`, 0 to 1: they light at 1; `flame::needed`) and why none would light (`why`, the flames' `Unlit`;
/// "" where some would).
fn burn(l: &mut Live, b: pcore::edit::Brush, joules_m2: f64) -> serde_json::Value {
    let done = make(l, b, pcore::edit::Act::Heat { radius_m: b.radius_m, joules_m2 });
    let w = &l.world;
    let cells: Vec<_> = w.under(&[b]).iter().map(|u| {
        let [e, over] = w.heated.iter().find(|h| h.0 == u.cell as u32).map_or([0.0, u.share], |h| h.1);
        let (progress, why) = match pcore::flame::needed(w, u.cell, over) {
            Ok(need) => ((e / need.max(f64::MIN_POSITIVE)).min(1.0), String::new()),
            Err(why) => (0.0, format!("{why:?}")),
        };
        json!({"cell": u.cell, "share": u.share, "progress": progress, "why": why})
    }).collect();
    json!({"cells": cells, "changed": done, "preview": false, "frame": false})
}

/// The brush `b` kept for the planting of the stroke, of the starting kind of row `row`, made when it ends
/// (`Live::plant`): each cell under it, the share it covers, whether it holds land (`done`) and "NoLand" where not.
fn keep_planting(l: &mut Live, b: pcore::edit::Brush, row: u32) -> serde_json::Value {
    let st = l.stroke.as_mut().expect("a stroke under way");
    st.planting.push(b);
    st.row = (row as usize).min(worldgen::life::KINDS.len() - 1);
    let w = &l.world;
    let cells: Vec<_> = w.under(&[b]).iter().map(|u| {
        let land = pcore::contract::land(w, u.cell) > 0.0;
        json!({"cell": u.cell, "share": u.share, "done": [land], "why": if land { "" } else { "NoLand" }})
    }).collect();
    json!({"cells": cells, "changed": false, "preview": false, "frame": false})
}

/// The relief the world's samples will be built from when its stroke settles (`tape::Ground::settled_relief_bytes`),
/// for the page's ground to be made again while the world settles; nothing (`out_len` 0) where none can be told before.
#[no_mangle]
pub extern "C" fn settled_relief() -> *const u8 {
    let bytes = with(|l| tape::Ground::settled_relief_bytes(&l.world).unwrap_or_default());
    give(bytes)
}

/// End the brush's stroke and run at once, with no time on the world's clock, what its edits start that takes no time
/// (`World::settle`): the samples built again over the ground it shaped and the liquid settled over them (`ground`
/// true: the page asks for the new version's relief), and the strikes burnt. Returns, as JSON, whether the ground was
/// built again, each cell the flames reached and when, s after its strike (`burnt`: [cell, s] pairs), the squares whose
/// centre lies in one of those cells (`squares`), why the stroke's planting placed nothing (`unplanted`: "" where it
/// placed some), whether the stroke can be undone (`undo`: one of its edits was made), and whether the page is to be
/// sent a frame (`frame`: the world changed since the last, `Live::unframed`, or as it settled).
#[no_mangle]
pub extern "C" fn settle() -> *const u8 {
    let out = with(|l| {
        let (done, unplanted, undo) = l.settle();
        let reached: std::collections::BTreeSet<usize> = done.burnt.iter().map(|b| b.0 as usize).collect();
        let frame = l.unframed || done.ground || !done.burnt.is_empty();
        let mut out = json!(done);
        (out["squares"], out["unplanted"], out["undo"], out["frame"]) = (json!(l.cells.squares_in(|i| reached.contains(&i))), json!(unplanted), json!(undo), json!(frame));
        out
    });
    give(out.to_string().into_bytes())
}

/// Settle what the edits of the stroke under way started (`World::settle`: the samples built again over the ground it
/// shaped so far and the liquid settled over them) and keep the stroke: the page settles a held stroke of the ground now
/// and then, so its water follows while it is held. Its undo is still to the world before the stroke began.
#[no_mangle]
pub extern "C" fn soften() {
    with(|l| {
        l.world.settle();
        l.follow();
    })
}

/// Put the world back as it was before the last stroke of the brush that changed it began, once (one level). Its
/// samples take a new version (`Relief::version`, above every one before, so no version names two grounds), and the
/// page asks for its relief as after a build. Returns, as JSON, whether there was a stroke to undo.
#[no_mangle]
pub extern "C" fn undo() -> *const u8 {
    let out = with(|l| {
        let Some((mut before, moved)) = l.before.take() else { return json!({"done": false}) };
        // (Its samples kept as those of its new version: none are built again.)
        let version = l.world.relief.version + 1;
        before.terrain.rename_built(before.relief.version, version);
        before.relief.version = version;
        l.world = before;
        (l.stroke, l.ready) = (None, None);
        l.follow();
        // (It moves back what the stroke moved.)
        let last = l.since.last_mut().expect("a version");
        last.moved = last.moved.or(moved);
        json!({"done": true})
    });
    give(out.to_string().into_bytes())
}

/// The world waits for edits (the page paused it, or undid a stroke): a copy of it is kept for the first stroke
/// (`Live::ready`), so that stroke's first call need not copy it; none where the last stroke's undo is kept (the next stroke copies the world
/// then, so no more than two copies of it are kept between strokes).
#[no_mangle]
pub extern "C" fn hold() {
    with(|l| {
        if l.ready.is_none() && l.before.is_none() && l.stroke.is_none() {
            l.ready = Some(l.world.clone());
        }
    })
}

/// The page's ground now shows the samples of version `version` (`ground_version`; its tiles made again from that
/// version's relief): `preview` tells it what edits moved since. Those of the versions before it are forgotten; a
/// version not built since the one shown before is ignored.
#[no_mangle]
pub extern "C" fn ground_shown(version: u32) {
    with(|l| {
        let Some(at) = l.since.iter().position(|v| v.version == version) else { return };
        l.since.drain(..at);
    })
}

/// What edits moved the ground since the samples of the ground the page shows (`ground_shown`), for the page to add to
/// the heights it draws until its ground is made again, over the tiles made of the samples of the version it is of: the
/// window of the terrain's samples that moved, its west column, north row, width and height in samples, and that
/// version (`ground_version` as `ground_shown` last took it; five u32), then of each sample of the window, row by row,
/// how far its ground moved, m (f32), then the depth of its loose ground once the samples are built again over the
/// ground as it stands (`World::loose_to_build`, m, f32), little-endian; an empty window (0, 0, 0, 0) where none
/// moved. Only the samples edits may have moved since (`Built::moved`) are looked at.
#[no_mangle]
pub extern "C" fn preview() -> *const u8 {
    let bytes = with(|l| {
        let (shown, newest) = (&l.since[0], l.since.last().expect("a version"));
        let now = |k: usize| l.world.relief.shaped_at(k);
        let across = l.world.terrain_samples().across;
        let moved = |k: usize| now(k) != shown.shaped_at(k);
        let may = l.since.iter().fold(Area::NONE, |a, v| a.or(v.moved));
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for k in may.samples(across).filter(|&k| moved(k)) {
            let (x, y) = (k % across, k / across);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
        let window: [u32; 4] = if x0 == usize::MAX { [0; 4] } else { [x0, y0, x1 - x0, y1 - y0].map(|v| v as u32) };
        let ks: Vec<usize> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| y * across + x)).collect();
        let dh = ks.iter().map(|&k| (now(k) - shown.shaped_at(k)) as f32);
        // The heights the samples will be built at: those last built with what edits shaped since.
        let built = l.world.terrain_samples();
        let loose = l.world.loose_to_build(&ks, &|j| built.h[j] + now(j) - newest.shaped_at(j));
        let tail = dh.chain(loose.iter().map(|&m| m as f32));
        window.iter().chain([&shown.version]).flat_map(|v| v.to_le_bytes()).chain(tail.flat_map(|v| v.to_le_bytes())).collect()
    });
    give(bytes)
}
