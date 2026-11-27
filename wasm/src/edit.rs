//! The live world's edits (god mode, handover.md 11): what the page's tools read of a place, the brush and its
//! strokes, their settling and undo (the preview of the ground they moved: `shown`).

use crate::shown::Area;
use crate::{give, handed, with, Live};
use pcore::edit::{Act, Brush, Edit};
use pcore::World;
use serde_json::json;

/// A stroke of the brush under way: where it plants, and the starting kind (planted when it ends, `Live::settle`); the
/// make of the loose ground it lays where it raises the ground (`World::draw_make`), the world as it was before it
/// began, whether one of its edits was made (then that world is the one an undo puts back), and the samples its paints
/// may have moved.
pub(crate) struct Stroke {
    planting: Vec<Brush>,
    row: usize,
    make: u8,
    before: World,
    changed: bool,
    moved: Area,
}

impl Live {
    /// End the stroke under way: its planting made (`plant`), the world's edits settled (`World::settle`); the world
    /// before it kept for an undo where it changed the world (the copy for the next stroke is made after, `hold`), else
    /// for the next stroke (`ready`: it stands as the world does), so no copy is made here. Returns what settling did,
    /// why its planting placed nothing ("" where it placed some, or there was none), and whether the stroke can be
    /// undone.
    pub(crate) fn settle(&mut self) -> (pcore::edit::Settled, String, bool) {
        let (mut unplanted, mut changed) = (String::new(), false);
        if let Some(mut st) = self.stroke.take() {
            if !st.planting.is_empty() {
                let (made, why) = self.plant(&st.planting, st.row);
                (unplanted, st.changed) = (why, st.changed || made);
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
    /// (`Act::Grown`; where none stands yet, as seedlings). Whether any of these edits was made (a clearing changes the
    /// world even where nothing is planted after it, so the stroke's undo is kept), and, where no cell was planted, the
    /// reason most cells give ("NoLand" or "NoRoom").
    fn plant(&mut self, planting: &[Brush], row: usize) -> (bool, String) {
        let under = self.world.under(planting);
        let edits: Vec<Edit> = under.iter().flat_map(|u| [Act::Clear(u.share), worldgen::schedule::grown(row)].map(|act| Edit { place: u.place, act })).collect();
        let done = self.world.edit(&edits);
        let made = done.iter().any(|&d| d);
        self.unframed |= made;
        let planted = done.chunks(2).any(|d| d.iter().all(|&ok| ok));
        if planted || under.is_empty() {
            return (made, String::new());
        }
        let wet = under.iter().filter(|u| pcore::contract::land(&self.world, u.cell) <= 0.0).count();
        (made, String::from(if 2 * wet >= under.len() { "NoLand" } else { "NoRoom" }))
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

/// The brush of radius `radius` m with the tool `tool` at the places the page handed in (`input`: of each its x and y in
/// squares and its amount, three f64 little-endian), between two months: by its amount (`shape`, `lake`, `burn`), or
/// of the starting kind of row `row` (`keep_planting`). A paint of the ground makes all its places in one call
/// (`shape`); another tool makes them in turn and answers for the last. The first call of a stroke begins it
/// (`Stroke`), with the world as it was (`ready`, else a copy) for its undo. Returns, as JSON, what the tool says
/// (`cells`, whether it changed the world), and what the page is to be sent: the preview of the ground moved
/// (`preview`: "paint", the part the paint may have moved; "whole"; or none, "") and a frame of the world (`frame`); or,
/// for a tool the shell has not or a paint of no places, why it is refused (`refused`), and nothing is done.
#[no_mangle]
pub extern "C" fn brush(tool: u32, radius: f64, row: u32) -> *const u8 {
    let f64_at = |c: &[u8], i: usize| f64::from_le_bytes(c[8 * i..8 * (i + 1)].try_into().expect("eight bytes"));
    let stamps: Vec<([f64; 2], f64)> = handed().chunks_exact(24).map(|c| ([f64_at(c, 0), f64_at(c, 1)], f64_at(c, 2))).collect();
    if !matches!(tool, SHAPE | LAKE | BURN | PLANT) || stamps.is_empty() {
        return give(json!({"refused": format!("The core has no brush tool {tool}, or the paint has no place.")}).to_string().into_bytes());
    }
    let out = with(|l| {
        if l.stroke.is_none() {
            let before = l.ready.take().unwrap_or_else(|| l.world.clone());
            l.strokes += 1;
            l.stroke = Some(Stroke { planting: Vec::new(), row: 0, make: l.world.draw_make(l.strokes), before, changed: false, moved: Area::NONE });
        }
        l.painted = Area::NONE;
        if tool == SHAPE {
            return shape(l, &stamps, radius);
        }
        let mut out = serde_json::Value::Null;
        for &(centre, amount) in &stamps {
            let b = Brush { centre, radius_m: radius };
            out = match tool {
                LAKE => lake(l, b, amount),
                BURN => burn(l, b, amount),
                // (`PLANT`: a tool the shell has not is refused above.)
                _ => keep_planting(l, b, row),
            };
        }
        out
    });
    give(out.to_string().into_bytes())
}

/// The edits `edits` in the stroke under way, in one call (`World::edit`): whether one of them was made.
fn make(l: &mut Live, edits: &[Edit]) -> bool {
    let done = l.world.edit(edits).iter().any(|&d| d);
    l.stroke.as_mut().expect("a stroke under way").changed |= done;
    l.unframed |= done;
    done
}

/// The box `area` of samples edits of the stroke under way may have moved the ground of kept, in the stroke and in the
/// newest version (`Built::moved`), for `preview`.
fn moved_ground(l: &mut Live, area: Area) {
    let st = l.stroke.as_mut().expect("a stroke under way");
    st.moved = st.moved.or(area);
    let last = l.since.last_mut().expect("a version");
    last.moved = last.moved.or(area);
}

/// The box of the samples the brushes about the places `centres` of radius `radius` m may move the ground of
/// (`Area::of`).
fn reach(l: &Live, centres: &[[f64; 2]], radius: f64) -> Area {
    let s = l.world.terrain_samples();
    let per = (s.across / l.world.map.w) as f64;
    centres.iter().fold(Area::NONE, |a, &centre| a.or(Area::of(Brush { centre, radius_m: radius }, (s.across, s.down, per), l.world.map.side)))
}

/// The ground under a brush of radius `radius` m at each place of `stamps` raised by its height at its centre, m, less
/// toward its edge (lowered where under none; `Act::Ground`), all in one call; each lays its loose top of the stroke's
/// make on what the stroke raised so far at the sample under its centre. The page draws it from the preview of the part
/// it moved (`Live::painted`: the samples the call moved and one about them, whose slope moved) until the stroke
/// settles. It names no cells (the page reads whether it changed the world).
fn shape(l: &mut Live, stamps: &[([f64; 2], f64)], radius: f64) -> serde_json::Value {
    let st = l.stroke.as_ref().expect("a stroke under way");
    let s = l.world.terrain_samples();
    let (across, down, per) = (s.across, s.down, (s.across / l.world.map.w) as f64);
    let at = |v: f64, n: usize| ((v * per).floor().max(0.0) as usize).min(n - 1);
    let raised = |[x, y]: [f64; 2]| {
        let k = at(y, down) * across + at(x, across);
        l.world.relief.shaped_at(k) - st.before.relief.shaped_at(k)
    };
    let edits: Vec<Edit> = stamps.iter().map(|&(place, height_m)| Edit { place, act: Act::Ground { radius_m: radius, height_m, stroke_m: raised(place).max(0.0), make: st.make } }).collect();
    drop(s);
    let may = reach(l, &stamps.iter().map(|p| p.0).collect::<Vec<_>>(), radius);
    let was: Vec<f64> = may.samples(across).map(|k| l.world.relief.shaped_at(k)).collect();
    let done = make(l, &edits);
    let moved = may.samples(across).zip(was).filter(|&(k, m)| l.world.relief.shaped_at(k) != m).fold(Area::NONE, |a, (k, _)| a.or(Area::sample(k, across)));
    l.painted = moved.grown(1, (across, down));
    moved_ground(l, may);
    json!({"cells": [], "changed": done, "preview": "paint", "frame": false})
}

/// A lake under the brush `b` at once, its floor `depth` m under its rim, full to its pass (`Act::Basin`: the samples
/// are built again and the liquid settles; the page asks for the new relief): a frame, and the whole preview until the
/// page's ground shows it. It names no cells, as `shape`.
fn lake(l: &mut Live, b: Brush, depth: f64) -> serde_json::Value {
    let done = make(l, &[Edit { place: b.centre, act: Act::Basin { radius_m: b.radius_m, depth_m: depth } }]);
    l.follow();
    let area = reach(l, &[b.centre], b.radius_m);
    moved_ground(l, area);
    json!({"cells": [], "changed": done, "preview": "whole", "frame": done})
}

/// `joules_m2` J a m² of heat added under the brush `b` to the remains lying there (`Act::Heat`), which dries and lights
/// them when the stroke settles (the page calls it again and again while the brush is held, each time with what the
/// time held since adds): each cell under it, the share it covers, how much of the heat that lights its remains it
/// holds (`progress`, 0 to 1: they light at 1; `flame::needed`) and why none would light (`why`, the flames' `Unlit`;
/// "" where some would).
fn burn(l: &mut Live, b: Brush, joules_m2: f64) -> serde_json::Value {
    let done = make(l, &[Edit { place: b.centre, act: Act::Heat { radius_m: b.radius_m, joules_m2 } }]);
    let w = &l.world;
    let cells: Vec<_> = w.under(&[b]).iter().map(|u| {
        let [e, over] = w.heated.iter().find(|h| h.0 == u.cell as u32).map_or([0.0, u.share], |h| h.1);
        let (progress, why) = match pcore::flame::needed(w, u.cell, over) {
            Ok(need) => ((e / need.max(f64::MIN_POSITIVE)).min(1.0), String::new()),
            Err(why) => (0.0, format!("{why:?}")),
        };
        json!({"cell": u.cell, "share": u.share, "progress": progress, "why": why})
    }).collect();
    json!({"cells": cells, "changed": done, "preview": "", "frame": false})
}

/// The brush `b` kept for the planting of the stroke, of the starting kind of row `row`, made when it ends
/// (`Live::plant`): each cell under it, the share it covers, whether it holds land (`done`) and "NoLand" where not.
fn keep_planting(l: &mut Live, b: Brush, row: u32) -> serde_json::Value {
    let st = l.stroke.as_mut().expect("a stroke under way");
    st.planting.push(b);
    st.row = (row as usize).min(worldgen::life::KINDS.len() - 1);
    let w = &l.world;
    let cells: Vec<_> = w.under(&[b]).iter().map(|u| {
        let land = pcore::contract::land(w, u.cell) > 0.0;
        json!({"cell": u.cell, "share": u.share, "done": [land], "why": if land { "" } else { "NoLand" }})
    }).collect();
    json!({"cells": cells, "changed": false, "preview": "", "frame": false})
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

/// The world waits for edits (the page paused it, undid a stroke, or a stroke settled): a copy of it is kept for the
/// next stroke (`Live::ready`), so that stroke's first call need not copy it; none while a stroke is under way. So up to
/// three worlds stand between strokes: the world, the copy, and the world before the last stroke for its undo.
#[no_mangle]
pub extern "C" fn hold() {
    with(|l| {
        if l.ready.is_none() && l.stroke.is_none() {
            l.ready = Some(l.world.clone());
        }
    })
}
