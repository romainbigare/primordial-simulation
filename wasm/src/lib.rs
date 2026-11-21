//! The live viewer's shell: a world that the page makes and steps, and the ground it draws, as plain `extern "C"`
//! exports (no wasm-bindgen). The page calls these from two Web Workers, each with its own instance, and reads the
//! results from the instance's memory. One steps the world: `meta` is JSON in the form of a tape's header, `frame` is
//! the bytes of one tape frame and `table` the cells' table made with it (`tape::Table`), `discharge` what runs through
//! each of the terrain's samples when it changed enough to show, `things` the list of what lies in the cells of a
//! window (`tape::things`). The other makes
//! the ground's tiles (`ground`, `tile`: `tape::Ground`), which need no world. Either may instead stand on a saved world
//! the page hands in (`input`): a snapshot of `ew run` (`open_world`, `ground_of_world`).

use pcore::space::terrain::Shape;
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

/// The year the live island's starting kinds of takers are placed: 50 years after the first life, when the cover
/// stands over most of the land it will hold (a start's choice, as `ew run --fauna-year`).
const TAKERS_YEAR: u64 = LIFE_YEAR + 50;

/// A channel's width goes as its discharge to the power `channel_width_exp` (about 1/2): the page is sent the samples'
/// discharge again once one has changed by more than this share since it was last sent, a change of its width of
/// about 5%, a pixel of a river 20 pixels wide.
const DISCHARGE_SHOWS: f64 = 0.1;
/// A channel's water is coloured by the logarithm of what it carries (its attenuation and its colour grow with it): the
/// page is sent the samples' load again once one has changed by more than this share since it was last sent.
const LOAD_SHOWS: f64 = 0.25;

/// The world, what is placed when, what the shell keeps of its cells (`tape::Table`), the table made with the last
/// frame, and the samples' discharge, m³/s, and load, g/m³, last sent to the page (`tape::Table::discharge`,
/// `tape::Table::load`; none before the first).
struct Live {
    world: World,
    plan: Schedule,
    cells: tape::Table,
    table: Vec<f32>,
    sent: Vec<[f32; 2]>,
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
    static GROUND: RefCell<Option<tape::Ground>> = const { RefCell::new(None) };
    /// The planet's air of the first banded world of the session (`worldgen::planet::planet`): the same for every island
    /// of the one chemistry, so it is made once.
    static PLANET: RefCell<Option<pcore::flow::Planet>> = const { RefCell::new(None) };
    /// The bytes last handed to the page (it reads them until the next call that returns bytes).
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// The bytes the page last handed in (`input`), until a call reads them.
    static IN: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// What a snapshot of `ew run` holds: the world, its run's plan and what its recorder keeps (`tape::Kept`, which the
/// page does not use).
type Saved = (World, (Schedule, tape::Kept));

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

/// The settings of an island between the latitudes `south` and `north`, degrees
/// (`worldgen::terrain::banded`); the default's where `south` is not below `north` (the page asks for no
/// band).
fn island((south, north): (f64, f64)) -> Config {
    if south < north { worldgen::terrain::banded((south, north)) } else { Config::default() }
}

/// The large shape of an island (`pcore::space::terrain::Shape`): drawn from the seed where `drawn` is not 0, else one
/// rise in the middle.
fn shape_of(drawn: u32) -> Shape {
    if drawn != 0 { Shape::Drawn } else { Shape::One }
}

/// Make an island of `size` x `size` cells and the large shape of `drawn` (`shape_of`) from `seed` between the latitudes
/// `south` and `north` (`island`), its cells holding at most `variety` kinds of cover (`Config::variety`), standing in
/// the planet's air where they are given (`worldgen::planet::planet`, made once, `PLANET`), its weather soft where `soft`
/// is not 0 (`worldgen::terrain::soft_weather`, `ew run --weather soft`), with the schedule of `ew run --roster`: the
/// starting kinds of cover. Its cells stay those as built: the map draws every zoom from the ground's
/// tiles and the cells' table. Returns 0, or 1 if the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32, south: f64, north: f64, drawn: u32, variety: u32, soft: u32) -> u32 {
    let band = island((south, north));
    let cfg = Config { variety: variety as usize, ..if soft != 0 { worldgen::terrain::soft_weather(band) } else { band } };
    let mut world = worldgen::terrain::new_island(seed as u64, (size as usize, size as usize), shape_of(drawn), cfg);
    if south < north {
        world.planet = Some(PLANET.with(|p| p.borrow_mut().get_or_insert_with(worldgen::planet::planet).clone()));
    }
    if world.cfg.validate().is_err() {
        return 1;
    }
    // Its ground gets Flow's state from its heat first, as `ew run` gives a new world's, so the same seed makes the
    // same world in the page as on the command line.
    pcore::flow::state(&mut world, true);
    let plan = Schedule { takers: Some(TAKERS_YEAR), ..Schedule::new(Some(LIFE_YEAR), &worldgen::life::ROSTER, Seeding { rounds: SPRAY_ROUNDS, fixers: FIXERS }) };
    let cells = tape::Table::new(&world);
    LIVE.with(|c| *c.borrow_mut() = Some(Live { world, plan, cells, table: Vec::new(), sent: Vec::new() }));
    0
}

/// Room for `len` bytes the page hands in (the gzip of a snapshot, for `open_world` or `ground_of_world`): their
/// address in memory, valid until the next call.
#[no_mangle]
pub extern "C" fn input(len: usize) -> *mut u8 {
    IN.with(|i| {
        *i.borrow_mut() = vec![0; len];
        i.borrow_mut().as_mut_ptr()
    })
}

/// The saved world handed in (`input`; read once), or why it is refused: not a snapshot of this build's format
/// (`tape::open_snapshot`), a globe (the page draws islands), or settings `Config::validate` refuses.
fn saved() -> Result<Saved, String> {
    let gz = IN.with(|i| std::mem::take(&mut *i.borrow_mut()));
    let (world, rest): Saved = tape::open_snapshot(&gz[..]).map_err(|e| format!("Not a saved world of this build: {e}."))?;
    if world.map.wrap {
        return Err(String::from("A saved globe: the page draws islands only."));
    }
    world.cfg.validate().map_err(|e| format!("A saved world with refused settings: {e}."))?;
    Ok((world, rest))
}

/// Stand the live world on the saved world handed in (`input`): the world, its plan, and the cells' table made anew;
/// it steps on as a new world does. Returns nothing (`out_len` 0), or why it is refused (`saved`), as text.
#[no_mangle]
pub extern "C" fn open_world() -> *const u8 {
    give(match saved() {
        Ok((mut world, (plan, _))) => {
            // A snapshot holds no cache: Flow's state of its ground (its ponds' levels, what its soils hold) and the
            // place of each cell (its liquid, by which the samples' discharge is drawn) are found again as a month's
            // start finds them, so the page draws the world at its date as one it played into, its rivers too.
            pcore::contract::surroundings(&mut world);
            let cells = tape::Table::new(&world);
            LIVE.with(|c| *c.borrow_mut() = Some(Live { world, plan, cells, table: Vec::new(), sent: Vec::new() }));
            Vec::new()
        }
        Err(e) => e.into_bytes(),
    })
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
/// by: the tiles (`tape::Ground`: texels a side, the gutter, the depths from the coarsest to the finest, the most steps
/// by which a texel names its channel's sample, and the length its place along its channel is kept modulo; the numbers
/// of a row of the table, `tape::Table`, the kinds of cover of a square, `Config::variety`, and the numbers of each;
/// the reflectance of the walls the cover builds), the terrain's samples (across,
/// down, their depth), the numbers of the laws the shell applies inside a cell (`Physics`), the discharge per metre a
/// pixel takes past which a channel shows as a line, m³/s per m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`), that line's
/// law (`tape::LINE_PX`, `LINE_GROW`, `LINE_MOST`), and how many times the mean flow the flood that shapes a channel's
/// bed passes (`tape::FLOOD`) with the power a channel's water widens by at one place (`tape::STATION_EXP`).
#[no_mangle]
pub extern "C" fn meta() -> *const u8 {
    let head = with(|l| {
        let (w, g, ph) = (&l.world, &l.world.map, &l.world.cfg.phys);
        let layers: Vec<_> = LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect();
        let depths = tape::depths(g);
        let s = w.terrain_samples();
        // The reflectance of the walls the cover builds, by which a shell colours its stems.
        let walls = pcore::cover::Parts::of(&w.chem).map_or([0.0; 3], |p| w.chem.subs[p.sub[pcore::cover::WALL]].refl[0]);
        json!({
            "w": g.w, "h": g.h, "cell_m": g.side, "square_depth": g.square_depth(), "globe": g.wrap, "lat_north": w.cfg.lat_north, "lat_south": w.cfg.lat_south,
            "seed": w.seed, "shape": w.terrain.shape, "days_per_year": w.days_per_year(), "layers": layers,
            "tiles": {"size": tape::TILE, "gutter": tape::GUTTER, "depths": [depths.start(), depths.end()], "step_most": tape::STEP_MOST, "along_wrap": tape::ALONG_WRAP_M, "row": tape::ROW, "kinds": w.cfg.variety, "kind_row": tape::KIND_ROW, "walls": walls},
            "samples": {"across": s.across, "down": s.down, "depth": pcore::space::terrain::SAMPLE_DEPTH},
            "laws": {"soil_m": ph.soil_m, "wet_depth": ph.wet_depth, "rooted_from": ph.rooted_from, "channel_width": ph.channel_width, "channel_width_exp": ph.channel_width_exp, "channel_depth": ph.channel_depth, "channel_depth_exp": ph.channel_depth_exp, "line": tape::TYPICAL_RUNOFF * tape::SHOW_M, "line_law": [tape::LINE_PX, tape::LINE_GROW, tape::LINE_MOST], "flood": tape::FLOOD, "station": tape::STATION_EXP},
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

/// What runs through each of the terrain's samples, m³/s, and what it carries, g/m³, row by row, two 16-bit floats
/// little-endian a sample (`tape::Table::discharge`, `tape::Table::load`): when the page has none yet, or one has
/// changed by more than `DISCHARGE_SHOWS` or `LOAD_SHOWS` since it was last sent (a sample that ran dry or began to run
/// or to carry counts); else nothing (`out_len` 0).
#[no_mangle]
pub extern "C" fn discharge() -> *const u8 {
    let bytes = with(|l| {
        let q = l.cells.discharge(&l.world);
        let now: Vec<[f32; 2]> = q.iter().zip(l.cells.load(&l.world, &q)).map(|(&q, c)| [q, c]).collect();
        let off = |a: f32, b: f32, share: f64| (a > 0.0) != (b > 0.0) || (a as f64 - b as f64).abs() > share * b.max(a) as f64;
        let moved = |(a, b): (&[f32; 2], &[f32; 2])| off(a[0], b[0], DISCHARGE_SHOWS) || off(a[1], b[1], LOAD_SHOWS);
        if l.sent.len() == now.len() && !l.sent.iter().zip(&now).any(moved) {
            return Vec::new();
        }
        let bytes = now.iter().flat_map(|v| v.iter().flat_map(|&x| tape::half(x).to_le_bytes())).collect();
        l.sent = now;
        bytes
    });
    give(bytes)
}

/// Where load came into a body of standing liquid in the last month (`Counters::plumes`): each the terrain's sample
/// (u32) and how much, kg/s (f32), little-endian.
#[no_mangle]
pub extern "C" fn plumes() -> *const u8 {
    let bytes = with(|l| l.world.counts.plumes.iter().flat_map(|&(k, kg)| k.to_le_bytes().into_iter().chain(kg.to_le_bytes())).collect());
    give(bytes)
}

/// What lies in each cell of the window from column `x` and row `y`, `w` x `h` squares: its slots of cover
/// (`tape::things`), for the page to show.
#[no_mangle]
pub extern "C" fn things(x: f64, y: f64, w: f64, h: f64) -> *const u8 {
    let bytes = with(|l| tape::things(&l.world, [x, y, w, h]));
    give(bytes)
}

/// Make the ground of the island `new_world` makes from `seed`, `size`, the latitudes `south` and `north` and `drawn`
/// (`tape::Ground`): its terrain and its map's squares (`worldgen::terrain::island_ground`; how its cells are cut does
/// not change the ground), with the default laws.
#[no_mangle]
pub extern "C" fn ground(seed: u32, size: u32, south: f64, north: f64, drawn: u32) {
    let (seed, cfg) = (seed as u64, island((south, north)));
    let (map, terrain) = worldgen::terrain::island_ground(seed, (size as usize, size as usize), shape_of(drawn), &cfg.phys);
    let g = tape::Ground::new(seed, map, terrain, (cfg.lat_south, cfg.lat_north), &cfg.phys, cfg.gravity);
    GROUND.with(|c| *c.borrow_mut() = Some(g));
}

/// Make the ground of the saved world handed in (`input`; `tape::Ground::of`). Returns nothing (`out_len` 0), or why it
/// is refused (`saved`), as text.
#[no_mangle]
pub extern "C" fn ground_of_world() -> *const u8 {
    give(match saved() {
        Ok((world, _)) => {
            GROUND.with(|c| *c.borrow_mut() = Some(tape::Ground::of(&world)));
            Vec::new()
        }
        Err(e) => e.into_bytes(),
    })
}

/// The tile of `depth` at column `x` and row `y` of tiles of the ground made by `ground`, 16-bit floats little-endian
/// (`tape::Ground::tile`).
#[no_mangle]
pub extern "C" fn tile(depth: u32, x: i32, y: i32) -> *const u8 {
    let texels = GROUND.with(|c| c.borrow().as_ref().expect("no ground: call ground first").tile(depth.min(u8::MAX as u32) as u8, x as i64, y as i64));
    give(texels.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// The rock of each of the terrain's samples of the ground made by `ground`, 16-bit floats little-endian
/// (`tape::Ground::rock`).
#[no_mangle]
pub extern "C" fn rock() -> *const u8 {
    // The reflectance of each make of the bed, its substances' by volume (as `tape::layers::bed_colour` reads a cell's).
    let (chem, init) = worldgen::chemistry::chemistry0();
    let colours: Vec<[f64; 3]> = init
        .makes
        .iter()
        .map(|make| {
            let v: f64 = make.iter().map(|p| p.1).sum();
            std::array::from_fn(|c| make.iter().map(|&(k, f)| f * chem.subs[k].refl[0][c] as f64).sum::<f64>() / v.max(1e-12))
        })
        .collect();
    let texels = GROUND.with(|c| c.borrow().as_ref().expect("no ground: call ground first").rock(&colours));
    give(texels.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// The length in bytes of what `meta`, `frame`, `table`, `discharge`, `things`, `tile`, `rock`, `open_world` or
/// `ground_of_world` last returned.
#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}
