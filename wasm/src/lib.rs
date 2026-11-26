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

mod edit;
mod ground;
use edit::Stroke;

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

/// The world; the version of its samples what the shell keeps of its cells is of (`Relief::version`); what is placed
/// when, and what a recorder keeps of it (`tape::Kept`, saved with it, `save_world`); what the shell keeps of its cells
/// (`tape::Table`); the table made with the last frame, till `table` hands it to the page; the samples' discharge, m³/s, and load, g/m³, last sent to the
/// page (`tape::Table::discharge`, `tape::Table::load`; none before the first); the brush's stroke under way (`Stroke`;
/// none between strokes: a stroke ends at `settle`) and the strokes begun; whether an edit changed the world since the
/// last frame (`frame`), so a stroke's end sends one only then (`settle`); a copy of the world as it stands now, kept
/// while it waits for edits (`hold`, and after a stroke that changed nothing), which the next stroke takes as the world
/// before it, so its first call copies nothing; the world as it was before the last stroke that changed it began, till
/// the world steps on (`undo`), with the samples that stroke moved (`edit::Area`); and of the version of the ground the
/// page shows (`ground_shown`) and of each built since, oldest first (`since`, `Built`), what edits had shaped, from
/// which `preview` tells the page what edits moved since: the last is the version the cells' table is of.
struct Live {
    world: World,
    built: u32,
    plan: Schedule,
    kept: tape::Kept,
    cells: tape::Table,
    table: Vec<f32>,
    sent: Vec<[f32; 2]>,
    stroke: Option<Stroke>,
    strokes: u64,
    unframed: bool,
    ready: Option<World>,
    before: Option<(World, edit::Area)>,
    since: Vec<Built>,
}

/// A version of the world's samples (`Relief::version`): what edits had shaped of each of the terrain's samples when
/// they were built (`Relief::shaped`: the samples that hold some, with how far, in their order), and the samples edits
/// moved while it was the newest version (`edit::Area`).
struct Built {
    version: u32,
    shaped: Vec<(usize, f64)>,
    moved: edit::Area,
}

impl Built {
    /// The version of the world `w`'s samples now.
    fn of(w: &World) -> Built {
        let shaped = w.relief.shaped.iter().enumerate().filter(|p| *p.1 != 0.0).map(|(k, &m)| (k, m)).collect();
        Built { version: w.relief.version, shaped, moved: edit::Area::NONE }
    }
    /// What edits had shaped of sample `k` at this version, m.
    fn shaped_at(&self, k: usize) -> f64 {
        self.shaped.binary_search_by_key(&k, |p| p.0).map_or(0.0, |i| self.shaped[i].1)
    }
}

impl Live {
    /// The live world `world`, its plan and what a recorder keeps of it, the cells' table made for it; the page shows
    /// its ground as it is.
    fn new(world: World, (plan, kept): (Schedule, tape::Kept)) -> Live {
        let cells = tape::Table::new(&world);
        let since = vec![Built::of(&world)];
        Live { built: world.relief.version, world, plan, kept, cells, table: Vec::new(), sent: Vec::new(), stroke: None, strokes: 0, unframed: false, ready: None, before: None, since }
    }
    /// Where the world built its samples anew (a build, a reshape, an undo): the cells' table found again, as the cells'
    /// ground and the shore moved (`tape::Table::update`), and what edits had shaped kept as the new version's.
    fn follow(&mut self) {
        if self.world.relief.version != self.built {
            self.cells.update(&self.world);
            self.built = self.world.relief.version;
            self.since.push(Built::of(&self.world));
        }
    }
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
    static GROUND: RefCell<Option<ground::Drawn>> = const { RefCell::new(None) };
    /// What was last handed to the page (it reads it until the next call that returns bytes).
    static OUT: RefCell<Out> = const { RefCell::new(Out::Bytes(Vec::new())) };
    /// The bytes the page last handed in (`input`), until a call reads them.
    static IN: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// What a snapshot of `ew run` holds: the world, its run's plan and what its recorder keeps (`tape::Kept`, kept for the
/// page's save).
type Saved = (World, (Schedule, tape::Kept));

/// Run `f` on the live world; panics if `new_world` has not been called.
fn with<T>(f: impl FnOnce(&mut Live) -> T) -> T {
    LIVE.with(|c| f(c.borrow_mut().as_mut().expect("no world: call new_world first")))
}

/// What the shell hands the page: bytes, or numbers of 16 or 32 bits, handed as they lie in memory: little-endian, as
/// the target is (wasm32), so none is copied to be encoded.
enum Out {
    Bytes(Vec<u8>),
    Halves(Vec<u16>),
    Floats(Vec<f32>),
}

impl Out {
    /// Its address in memory and its length in bytes.
    fn place(&self) -> (*const u8, usize) {
        match self {
            Out::Bytes(v) => (v.as_ptr(), v.len()),
            Out::Halves(v) => (v.as_ptr().cast(), 2 * v.len()),
            Out::Floats(v) => (v.as_ptr().cast(), 4 * v.len()),
        }
    }
}

impl From<Vec<u8>> for Out {
    fn from(v: Vec<u8>) -> Out {
        Out::Bytes(v)
    }
}
impl From<Vec<u16>> for Out {
    fn from(v: Vec<u16>) -> Out {
        Out::Halves(v)
    }
}
impl From<Vec<f32>> for Out {
    fn from(v: Vec<f32>) -> Out {
        Out::Floats(v)
    }
}

/// Hand `out` to the page: its address in memory, valid until the next call that returns bytes; `out_len` is its
/// length in bytes.
fn give(out: impl Into<Out>) -> *const u8 {
    OUT.with(|o| {
        *o.borrow_mut() = out.into();
        o.borrow().place().0
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
/// the planet's air where they are given (`worldgen::planet::stand`, the table the build holds), its weather soft where `soft`
/// is not 0 (`worldgen::terrain::soft_weather`, `ew run --weather soft`), with the schedule of `ew run --roster`: the
/// starting kinds of cover. Its cells stay those as built: the map draws every zoom from the ground's
/// tiles and the cells' table. Returns 0, or 1 if the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32, south: f64, north: f64, drawn: u32, variety: u32, soft: u32) -> u32 {
    let band = island((south, north));
    let cfg = Config { variety: variety as usize, ..if soft != 0 { worldgen::terrain::soft_weather(band) } else { band } };
    let mut world = worldgen::terrain::new_island(seed as u64, (size as usize, size as usize), shape_of(drawn), cfg);
    if south < north {
        worldgen::planet::stand(&mut world);
    }
    if world.cfg.validate().is_err() {
        return 1;
    }
    // Its ground gets Flow's state from its heat first, as `ew run` gives a new world's, so the same seed makes the
    // same world in the page as on the command line.
    pcore::flow::state(&mut world, true);
    let plan = Schedule { takers: Some(TAKERS_YEAR), ..Schedule::new(Some(LIFE_YEAR), &worldgen::life::ROSTER, Seeding { rounds: SPRAY_ROUNDS, fixers: FIXERS }) };
    let kept = tape::Kept::new(&world);
    LIVE.with(|c| *c.borrow_mut() = Some(Live::new(world, (plan, kept))));
    0
}

/// Room for `len` bytes the page hands in (the gzip of a snapshot, for `open_world` or `ground_of_world`; a relief, for
/// `ground_built`): their
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
        Ok((mut world, shell)) => {
            // A snapshot holds no cache: Flow's state of its ground (its ponds' levels, what its soils hold) and the
            // place of each cell (its liquid, by which the samples' discharge is drawn) are found again as a month's
            // start finds them, so the page draws the world at its date as one it played into, its rivers too.
            pcore::contract::surroundings(&mut world);
            LIVE.with(|c| *c.borrow_mut() = Some(Live::new(world, shell)));
            Vec::new()
        }
        Err(e) => e.into_bytes(),
    })
}

/// The live world as a snapshot file of `ew run` (`tape::snapshot_gz`: the world, its plan and what a recorder keeps),
/// which the page saves and `open_world` or `ew resume` opens. The page saves between strokes.
#[no_mangle]
pub extern "C" fn save_world() -> *const u8 {
    let bytes = with(|l| tape::snapshot_gz(&l.world, &(&l.plan, &l.kept)).expect("a world saves"));
    give(bytes)
}

/// Step one month: first a stroke of the brush still under way ends (`Live::settle`); the last stroke can no longer be
/// undone (the one timeline goes on from it); at a year's start, the schedule places what is due.
#[no_mangle]
pub extern "C" fn step_month() {
    with(|l| {
        if l.stroke.is_some() {
            l.settle();
        }
        (l.ready, l.before) = (None, None);
        if l.world.month() % l.world.cfg.months_per_year as u64 == 0 {
            l.plan.place(&mut l.world);
        }
        pcore::step(&mut l.world);
        l.world.drain_events();
        l.follow();
    })
}

/// The version of the world's samples (`Relief::version`: one more at each build and each reshape after edits, and at
/// an undo): the page asks its ground for the samples of a new version (`relief`) when it moved.
#[no_mangle]
pub extern "C" fn ground_version() -> u32 {
    with(|l| l.world.relief.version)
}

/// The relief the world last built its samples from, the years since its start then and the world's cells
/// (`tape::Ground::relief_bytes`), for the ground's worker (`ground_built`).
#[no_mangle]
pub extern "C" fn relief() -> *const u8 {
    let bytes = with(|l| tape::Ground::relief_bytes(&l.world));
    give(bytes)
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
/// of a row of the table, `tape::Table`, of a kind of cover of a square, of the places of its cell's patches and of the
/// dead walls standing on them;
/// the reflectance of the walls the cover builds), the terrain's samples (across,
/// down, their depth), the numbers of the laws the shell applies inside a cell (`Physics`), the discharge per metre a
/// pixel takes past which a channel shows as a line, m³/s per m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`), that line's
/// law (`tape::LINE_PX`, `LINE_GROW`, `LINE_MOST`), how many times the mean flow the flood that shapes a channel's
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
            "tiles": {"size": tape::TILE, "gutter": tape::GUTTER, "depths": [depths.start(), depths.end()], "step_most": tape::STEP_MOST, "along_wrap": tape::ALONG_WRAP_M, "row": tape::ROW, "kind_row": tape::KIND_ROW, "place_row": tape::PLACE_ROW, "dead_row": tape::DEAD_ROW, "walls": walls},
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
        (l.table, l.unframed) = (table, false);
        bytes
    });
    give(bytes)
}

/// The cells' table made with the last frame, f32 little-endian (`tape::Table::frame`), handed over once.
#[no_mangle]
pub extern "C" fn table() -> *const u8 {
    let table = with(|l| std::mem::take(&mut l.table));
    give(table)
}

/// What runs through each of the terrain's samples, m³/s, and what it carries, g/m³, row by row, two 16-bit floats
/// little-endian a sample (`tape::Table::discharge`, `tape::Table::load`): when the page has none yet, or one has
/// changed by more than `DISCHARGE_SHOWS` or `LOAD_SHOWS` since it was last sent (a sample that ran dry or began to run
/// or to carry counts); else nothing (`out_len` 0).
#[no_mangle]
pub extern "C" fn discharge() -> *const u8 {
    let halves = with(|l| {
        let q = l.cells.discharge(&l.world);
        let now: Vec<[f32; 2]> = q.iter().zip(l.cells.load(&l.world, &q)).map(|(&q, c)| [q, c]).collect();
        let off = |a: f32, b: f32, share: f64| (a > 0.0) != (b > 0.0) || (a as f64 - b as f64).abs() > share * b.max(a) as f64;
        let moved = |(a, b): (&[f32; 2], &[f32; 2])| off(a[0], b[0], DISCHARGE_SHOWS) || off(a[1], b[1], LOAD_SHOWS);
        if l.sent.len() == now.len() && !l.sent.iter().zip(&now).any(moved) {
            return Vec::new();
        }
        let halves: Vec<u16> = now.iter().flat_map(|v| v.map(tape::half)).collect();
        l.sent = now;
        halves
    });
    give(halves)
}

/// Where load came into a body of standing liquid in the last month (`Counters::plumes`): each the terrain's sample
/// (u32) and how much, kg/s (f32), little-endian.
#[no_mangle]
pub extern "C" fn plumes() -> *const u8 {
    let bytes: Vec<u8> = with(|l| l.world.counts.plumes.iter().flat_map(|&(k, kg)| k.to_le_bytes().into_iter().chain(kg.to_le_bytes())).collect());
    give(bytes)
}

/// What lies in each cell of the window from column `x` and row `y`, `w` x `h` squares: its slots of cover
/// (`tape::things`), for the page to show.
#[no_mangle]
pub extern "C" fn things(x: f64, y: f64, w: f64, h: f64) -> *const u8 {
    let bytes = with(|l| tape::things(&l.world, [x, y, w, h]));
    give(bytes)
}

/// The length in bytes of what the last call that returns bytes returned (`meta`, `frame`, `table`, `tile`, `rock`,
/// `rock_rows`, `preview` and the others).
#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().place().1)
}
