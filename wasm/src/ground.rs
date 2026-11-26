//! The ground's tiles of the live viewer (the second worker's instance, `tape::Ground`), which need no world: the
//! ground of a new island or of a saved world, made again on the relief the world built, its tiles and its rock.

use crate::{give, island, saved, shape_of, GROUND, IN};

/// The ground a worker draws (`tape::Ground`), and the reflectance of each make of its bed, which its rock is coloured
/// by (`colours`).
pub(crate) struct Drawn {
    ground: tape::Ground,
    colours: Vec<[f64; 3]>,
}

impl Drawn {
    fn new(ground: tape::Ground) -> Drawn {
        Drawn { ground, colours: colours() }
    }
}

/// The reflectance of each make of the bed, its substances' by volume (as `tape::layers::bed_colour` reads a cell's).
fn colours() -> Vec<[f64; 3]> {
    let (chem, init) = worldgen::chemistry::chemistry0();
    init.makes
        .iter()
        .map(|make| {
            let v: f64 = make.iter().map(|p| p.1).sum();
            std::array::from_fn(|c| make.iter().map(|&(k, f)| f * chem.subs[k].refl[0][c] as f64).sum::<f64>() / v.max(1e-12))
        })
        .collect()
}

/// Run `f` on the ground; panics if `ground` or `ground_of_world` has not been called.
fn with_ground<T>(f: impl FnOnce(&mut Drawn) -> T) -> T {
    GROUND.with(|c| f(c.borrow_mut().as_mut().expect("no ground: call ground first")))
}

/// The ground made by `ground` made again on the samples the relief handed in (`input`, from `relief`) builds
/// (`tape::Ground::rebuild_from`): returns a byte for each square of the map, row by row, 1 where its tiles changed;
/// or, past them, why the relief was refused, as text. The rock it changed is read with `rock_rows`.
#[no_mangle]
pub extern "C" fn ground_built() -> *const u8 {
    let bytes = IN.with(|i| std::mem::take(&mut *i.borrow_mut()));
    let out = with_ground(|d| d.ground.rebuild_from(&bytes));
    give(match out {
        Ok(squares) => squares.iter().map(|&c| c as u8).collect(),
        Err(e) => e.into_bytes(),
    })
}

/// Keep the ground `g`; nothing (`out_len` 0), or why its tiles cannot be drawn (`tape::Ground::drawable`), as text.
fn keep(g: tape::Ground) -> Vec<u8> {
    let why = g.drawable().err().unwrap_or_default();
    GROUND.with(|c| *c.borrow_mut() = Some(Drawn::new(g)));
    why.into_bytes()
}

/// Make the ground of the island `new_world` makes from `seed`, `size`, the latitudes `south` and `north` and `drawn`
/// (`tape::Ground`): its terrain and its map's squares (`worldgen::terrain::island_ground`; how its cells are cut does
/// not change the ground), with the default laws. Returns nothing (`out_len` 0), or why it cannot be drawn (`keep`).
#[no_mangle]
pub extern "C" fn ground(seed: u32, size: u32, south: f64, north: f64, drawn: u32) -> *const u8 {
    let (seed, cfg) = (seed as u64, island((south, north)));
    let (map, terrain) = worldgen::terrain::island_ground(seed, (size as usize, size as usize), shape_of(drawn), &cfg.phys);
    give(keep(tape::Ground::new(seed, map, terrain, (cfg.lat_south, cfg.lat_north), &cfg.phys, cfg.gravity)))
}

/// Make the ground of the saved world handed in (`input`; `tape::Ground::of`). Returns nothing (`out_len` 0), or why it
/// is refused (`saved`) or cannot be drawn (`keep`), as text.
#[no_mangle]
pub extern "C" fn ground_of_world() -> *const u8 {
    give(match saved() {
        Ok((world, _)) => keep(tape::Ground::of(&world)),
        Err(e) => e.into_bytes(),
    })
}

/// The tile of `depth` at column `x` and row `y` of tiles of the ground made by `ground`, 16-bit floats little-endian
/// (`tape::Ground::tile`); nothing (`out_len` 0) for a depth the ground is not drawn at.
#[no_mangle]
pub extern "C" fn tile(depth: u32, x: i32, y: i32) -> *const u8 {
    let texels = with_ground(|d| {
        let depth = u8::try_from(depth).ok().filter(|t| d.ground.depths().contains(t));
        depth.map_or_else(Vec::new, |t| d.ground.tile(t, x as i64, y as i64))
    });
    give(texels)
}

/// The rock of each of the terrain's samples of the ground made by `ground`, 16-bit floats little-endian
/// (`tape::Ground::rock`).
#[no_mangle]
pub extern "C" fn rock() -> *const u8 {
    let texels = with_ground(|d| d.ground.rock(&d.colours));
    give(texels)
}

/// The rock the last `ground_built` may have changed (`tape::Ground::rock_rows`), 16-bit floats little-endian: first
/// the band of rows of samples it holds, its first row and the row past its last, as two u32 (each two 16-bit words,
/// the low first); then of each of the rock's four planes those rows (`rock`).
#[no_mangle]
pub extern "C" fn rock_rows() -> *const u8 {
    let out = with_ground(|d| {
        let ((from, to), band) = d.ground.rock_rows(&d.colours);
        let word = |v: usize, high: bool| (if high { v >> 16 } else { v & 0xFFFF }) as u16;
        let mut out = Vec::with_capacity(4 + band.len());
        out.extend([word(from, false), word(from, true), word(to, false), word(to, true)]);
        out.extend(band);
        out
    });
    give(out)
}
