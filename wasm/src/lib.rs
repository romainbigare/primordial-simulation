//! The live viewer's shell: a world that the page makes and steps, as plain `extern "C"` exports (no wasm-bindgen).
//! The page calls these from a Web Worker and reads the results from the instance's memory: `meta` is JSON in the
//! form of a tape's header, `frame` is the bytes of one tape frame (`tape::frame_now`).

use pcore::refine::Shot;
use pcore::World;
use serde_json::json;
use std::cell::RefCell;
use tape::LAYERS;
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;

/// The year life starts: the default of `ew run`.
const LIFE_YEAR: u64 = 2;
/// Rounds of mutation given to the founders: the default of `ew run`.
const SPRAY_ROUNDS: u32 = 20;

/// The most cells the live map holds while refinement splits them: about twice an island of 32 x 32 squares (about
/// 800 cells), which a worker steps in a browser at a usable pace.
const CELL_BUDGET: u32 = 1500;
/// The finest depth below its air block refinement takes a cell to in the live world: 7.8 m on a 2 km island, a
/// street, whose bodies step every 2.8 hours.
const FINEST_DEPTH: u8 = 10;

/// The world, what is placed when, and the bytes last handed to the page (it reads them until the next call).
struct Live {
    world: World,
    plan: Schedule,
    out: Vec<u8>,
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
}

/// Run `f` on the live world; panics if `new_world` has not been called.
fn with<T>(f: impl FnOnce(&mut Live) -> T) -> T {
    LIVE.with(|c| f(c.borrow_mut().as_mut().expect("no world: call new_world first")))
}

/// Hand `bytes` to the page: their address in memory, valid until the next call that returns bytes; `out_len` is their length.
fn give(bytes: Vec<u8>) -> *const u8 {
    with(|l| {
        l.out = bytes;
        l.out.as_ptr()
    })
}

/// Make an island of `size` x `size` cells from `seed`, with the schedule of `ew run`. Returns 0, or 1 if the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32) -> u32 {
    let mut world = worldgen::new_world(seed as u64, size as usize, size as usize);
    (world.cfg.cell_budget, world.cfg.finest_depth) = (CELL_BUDGET, FINEST_DEPTH);
    if world.cfg.validate().is_err() {
        return 1;
    }
    let plan = Schedule::new(Some(LIFE_YEAR), false, Seeding { hand: true, rounds: SPRAY_ROUNDS, fixers: FIXERS });
    LIVE.with(|c| *c.borrow_mut() = Some(Live { world, plan, out: vec![] }));
    0
}

/// Step one month: first refine the cells by the plan's camera path (`World::refine`), then at a year's start place
/// what the schedule says is due.
#[no_mangle]
pub extern "C" fn step_month() {
    with(|l| {
        l.world.refine(&l.plan.camera);
        if l.world.month() % l.world.cfg.months_per_year as u64 == 0 {
            l.plan.place(&mut l.world);
        }
        pcore::step(&mut l.world);
        l.world.drain_events();
    })
}

/// Send the camera to the window from column `x` and row `y`, `w` x `h` squares, asking for cells of `depth` below
/// their air block, from the next month on: a shot of the run's camera path (`ew run --camera`), so the run can be made
/// again. A window of no area sends the camera away.
#[no_mangle]
pub extern "C" fn set_camera(x: f64, y: f64, w: f64, h: f64, depth: u32) {
    with(|l| {
        let month = l.world.month();
        let shots = &mut l.plan.camera.shots;
        shots.retain(|s| s.month < month);
        shots.push(Shot { month, x, y, w, h, depth: depth.min(u8::MAX as u32) as u8 });
    })
}

/// The cells now.
#[no_mangle]
pub extern "C" fn cells() -> u32 {
    with(|l| l.world.mesh.cells().n() as u32)
}

/// The finest depth below its air block that a cell has now.
#[no_mangle]
pub extern "C" fn finest() -> u32 {
    with(|l| l.world.map.leaves().iter().map(|c| c.depth).max().unwrap_or(0) as u32)
}

/// Years since the start, with their fraction.
#[no_mangle]
pub extern "C" fn year() -> f64 {
    with(|l| l.world.year())
}

/// The world's day.
#[no_mangle]
pub extern "C" fn day() -> f64 {
    with(|l| l.world.day() as f64)
}

/// The header of a tape for this world, as JSON (no frames, series or events yet).
#[no_mangle]
pub extern "C" fn meta() -> *const u8 {
    let head = with(|l| {
        let (w, g) = (&l.world, &l.world.map);
        let layers: Vec<_> = LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect();
        json!({
            "w": g.w, "h": g.h, "cell_m": g.side, "square_depth": g.square_depth(), "finest_depth": w.cfg.finest_depth, "globe": g.wrap, "lat_north": w.cfg.lat_north, "lat_south": w.cfg.lat_south,
            "seed": w.seed, "days_per_year": w.days_per_year(), "layers": layers,
            "frames": [], "series": {}, "series_units": {}, "events": [], "months": [],
        })
    });
    give(head.to_string().into_bytes())
}

/// The present state as one tape frame (`tape::frame_now`).
#[no_mangle]
pub extern "C" fn frame() -> *const u8 {
    let bytes = with(|l| tape::frame_now(&l.world));
    give(bytes)
}

/// The layers of a window of the map as a raster of `across` x `down` pixels (`tape::frame_window`): the window from
/// column `x` and row `y`, `w` x `h` squares.
#[no_mangle]
pub extern "C" fn frame_window(x: f64, y: f64, w: f64, h: f64, across: u32, down: u32) -> *const u8 {
    let bytes = with(|l| tape::frame_window(&l.world, [x, y, w, h], across as usize, down as usize));
    give(bytes)
}

/// The length in bytes of what `meta` or `frame` last returned.
#[no_mangle]
pub extern "C" fn out_len() -> usize {
    with(|l| l.out.len())
}
