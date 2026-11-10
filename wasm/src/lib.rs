//! The live viewer's shell: a world that the page makes and steps, and the ground it draws, as plain `extern "C"`
//! exports (no wasm-bindgen). The page calls these from two Web Workers, each with its own instance, and reads the
//! results from the instance's memory. One steps the world: `meta` is JSON in the form of a tape's header, `frame` is
//! the bytes of one tape frame and `table` the cells' table made with it (`tape::Table`), `things` the list of what lies
//! in the cells of a window (`tape::things`). The other makes
//! the ground's tiles (`ground`, `tile`: `tape::Ground`), which need no world.

use pcore::{Config, World};
use serde_json::json;
use std::cell::RefCell;
use tape::LAYERS;
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;

/// The year life starts: the default of `ew run`.
const LIFE_YEAR: u64 = 2;
/// Rounds of mutation given to the founders: the default of `ew run`.
const SPRAY_ROUNDS: u32 = 20;

/// The world, what is placed when, what the shell keeps of its cells (`tape::Table`) and the table made with the last
/// frame.
struct Live {
    world: World,
    plan: Schedule,
    cells: tape::Table,
    table: Vec<f32>,
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
    static GROUND: RefCell<Option<tape::Ground>> = const { RefCell::new(None) };
    /// The bytes last handed to the page (it reads them until the next call that returns bytes).
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` on the live world; panics if `new_world` has not been called.
fn with<T>(f: impl FnOnce(&mut Live) -> T) -> T {
    LIVE.with(|c| f(c.borrow_mut().as_mut().expect("no world: call new_world first")))
}

/// Hand `bytes` to the page: their address in memory, valid until the next call that returns bytes; `out_len` is their length.
fn give(bytes: Vec<u8>) -> *const u8 {
    OUT.with(|o| {
        *o.borrow_mut() = bytes;
        o.borrow().as_ptr()
    })
}

/// Make an island of `size` x `size` cells from `seed`, with the schedule of `ew run --roster --plants`: the starting
/// kinds of cover, no animals, so the page runs fast while the work is on the ground, the liquid and the cover. Its
/// cells stay those as built: the map draws every zoom from the ground's tiles and the cells' table. Returns 0, or 1 if
/// the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32) -> u32 {
    let world = worldgen::new_world(seed as u64, size as usize, size as usize);
    if world.cfg.validate().is_err() {
        return 1;
    }
    let plan = Schedule::new(Some(LIFE_YEAR), &worldgen::life::plants(&worldgen::life::ROSTER), Seeding { hand: true, rounds: SPRAY_ROUNDS, fixers: FIXERS });
    let cells = tape::Table::new(&world);
    LIVE.with(|c| *c.borrow_mut() = Some(Live { world, plan, cells, table: Vec::new() }));
    0
}

/// Step one month: at a year's start, first place what the schedule says is due.
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

/// The cells now.
#[no_mangle]
pub extern "C" fn cells() -> u32 {
    with(|l| l.world.mesh.cells().n() as u32)
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

/// The header of a tape for this world, as JSON (no frames, series or events yet), with what a shell draws the ground
/// by: the tiles (`tape::Ground`: texels a side, the gutter, numbers a texel, the depths from the coarsest to the
/// finest), the numbers of a row of the table (`tape::Table`), and the numbers of the laws the shell applies inside a
/// cell (`Physics`).
#[no_mangle]
pub extern "C" fn meta() -> *const u8 {
    let head = with(|l| {
        let (w, g, ph) = (&l.world, &l.world.map, &l.world.cfg.phys);
        let layers: Vec<_> = LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect();
        let depths = tape::depths(g);
        json!({
            "w": g.w, "h": g.h, "cell_m": g.side, "square_depth": g.square_depth(), "globe": g.wrap, "lat_north": w.cfg.lat_north, "lat_south": w.cfg.lat_south,
            "seed": w.seed, "days_per_year": w.days_per_year(), "layers": layers,
            "tiles": {"size": tape::TILE, "gutter": tape::GUTTER, "texel": tape::TEXEL, "depths": [depths.start(), depths.end()], "row": tape::ROW},
            "laws": {"soil_m": ph.soil_m, "wet_depth": ph.wet_depth, "dormancy": ph.dormancy, "channel_width": ph.channel_width, "channel_width_exp": ph.channel_width_exp, "channel_depth": ph.channel_depth, "channel_depth_exp": ph.channel_depth_exp},
            "frames": [], "series": {}, "series_units": {}, "events": [], "months": [],
        })
    });
    give(head.to_string().into_bytes())
}

/// The present state as one tape frame, and the cells' table made with it (`tape::Table::frame`, read with `table`).
#[no_mangle]
pub extern "C" fn frame() -> *const u8 {
    let bytes = with(|l| {
        let (bytes, table) = l.cells.frame(&l.world);
        l.table = table;
        bytes
    });
    give(bytes)
}

/// The cells' table made with the last frame, f32 little-endian (`tape::Table::frame`).
#[no_mangle]
pub extern "C" fn table() -> *const u8 {
    let bytes = with(|l| l.table.iter().flat_map(|v| v.to_le_bytes()).collect());
    give(bytes)
}

/// What lies in each cell of the window from column `x` and row `y`, `w` x `h` squares: its bodies, cover and parts
/// (`tape::things`), for the page to draw one by one.
#[no_mangle]
pub extern "C" fn things(x: f64, y: f64, w: f64, h: f64) -> *const u8 {
    let bytes = with(|l| tape::things(&l.world, [x, y, w, h]));
    give(bytes)
}

/// Make the ground of the island `new_world` makes from `seed` and `size` (`tape::Ground`): its terrain and its map's
/// squares (`worldgen::new_world`; how its cells are cut does not change the ground), with the default laws.
#[no_mangle]
pub extern "C" fn ground(seed: u32, size: u32) {
    let (seed, cfg) = (seed as u64, Config::default());
    let (map, terrain) = worldgen::terrain::island_ground(seed, size as usize, size as usize);
    let g = tape::Ground::new(seed, map, terrain, (cfg.lat_south, cfg.lat_north), cfg.phys.channel_width_exp);
    GROUND.with(|c| *c.borrow_mut() = Some(g));
}

/// The tile of `depth` at column `x` and row `y` of tiles of the ground made by `ground`, f32 little-endian
/// (`tape::Ground::tile`).
#[no_mangle]
pub extern "C" fn tile(depth: u32, x: i32, y: i32) -> *const u8 {
    let texels = GROUND.with(|c| c.borrow().as_ref().expect("no ground: call ground first").tile(depth.min(u8::MAX as u32) as u8, x as i64, y as i64));
    give(texels.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// The length in bytes of what `meta`, `frame`, `table`, `things` or `tile` last returned.
#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}
