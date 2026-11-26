//! The ground's tiles of the live viewer (the second worker's instance, `tape::Ground`), which need no world: the
//! ground of a new island or of a saved world, made again on the relief the world built, its tiles and its rock.

use crate::{give, island, saved, shape_of, GROUND, IN};

/// The ground made by `ground` made again on the samples the relief handed in (`input`, from `relief`) builds
/// (`tape::Ground::rebuild_from`): returns a byte for each square of the map, row by row, 1 where its tiles changed;
/// or, past them, why the relief was refused, as text.
#[no_mangle]
pub extern "C" fn ground_built() -> *const u8 {
    let bytes = IN.with(|i| std::mem::take(&mut *i.borrow_mut()));
    let out = GROUND.with(|c| c.borrow_mut().as_mut().expect("no ground: call ground first").rebuild_from(&bytes));
    give(match out {
        Ok(squares) => squares.iter().map(|&c| c as u8).collect(),
        Err(e) => e.into_bytes(),
    })
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
/// (`tape::Ground::tile`); nothing (`out_len` 0) for a depth the ground is not drawn at.
#[no_mangle]
pub extern "C" fn tile(depth: u32, x: i32, y: i32) -> *const u8 {
    let texels = GROUND.with(|c| {
        let g = c.borrow();
        let g = g.as_ref().expect("no ground: call ground first");
        let depth = u8::try_from(depth).ok().filter(|d| g.depths().contains(d));
        depth.map_or_else(Vec::new, |d| g.tile(d, x as i64, y as i64))
    });
    give(texels)
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
    give(texels)
}
