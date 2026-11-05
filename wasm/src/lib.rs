//! The live viewer's shell: a world that the page makes and steps, as plain `extern "C"` exports (no wasm-bindgen).
//! The page calls these from a Web Worker and reads the results from the instance's memory: `meta` is JSON in the
//! form of a tape's header, `frame` is the bytes of one tape frame (`tape::frame_now`).

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
    let world = worldgen::new_world(seed as u64, size as usize, size as usize);
    if world.cfg.validate().is_err() {
        return 1;
    }
    let plan = Schedule::new(Some(LIFE_YEAR), false, Seeding { hand: true, rounds: SPRAY_ROUNDS, fixers: FIXERS });
    LIVE.with(|c| *c.borrow_mut() = Some(Live { world, plan, out: vec![] }));
    0
}

/// Step one month; at a year's start first place what the schedule says is due.
#[no_mangle]
pub extern "C" fn step_month() {
    with(|l| {
        if l.world.month() % l.world.cfg.months_per_year as u64 == 0 {
            l.plan.place(&mut l.world);
        }
        pcore::step(&mut l.world);
        l.world.drain_events();
    })
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
            "w": g.w, "h": g.h, "cell_m": g.side, "globe": g.wrap, "lat_north": w.cfg.lat_north, "lat_south": w.cfg.lat_south,
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

/// The length in bytes of what `meta` or `frame` last returned.
#[no_mangle]
pub extern "C" fn out_len() -> usize {
    with(|l| l.out.len())
}
