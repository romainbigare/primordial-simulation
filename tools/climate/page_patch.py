"""The page's band: apply in the worktree (run from its root)."""


def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a[:70])
        s = s.replace(a, b)
    open(p, 'w').write(s)


edit('wasm/src/lib.rs', [
("""/// Make an island of `size` x `size` cells from `seed`, with the schedule of `ew run --roster --plants`: the starting
/// kinds of cover, no animals, so the page runs fast while the work is on the ground, the liquid and the cover. Its
/// cells stay those as built: the map draws every zoom from the ground's tiles and the cells' table. Returns 0, or 1 if
/// the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32) -> u32 {
    let world = worldgen::new_world(seed as u64, size as usize, size as usize);""",
"""/// The settings of an island of `size` rows between the latitudes `south` and `north`, degrees, with the air's stretch
/// of that band (`worldgen::terrain::banded`); the default's where `south` is not below `north` (the page asks for no
/// band).
fn island(size: u32, (south, north): (f64, f64)) -> Config {
    if south < north { worldgen::terrain::banded(size as usize, (south, north)) } else { Config::default() }
}

/// Make an island of `size` x `size` cells from `seed` between the latitudes `south` and `north` (`island`), with the
/// schedule of `ew run --roster --plants`: the starting kinds of cover, no animals, so the page runs fast while the work
/// is on the ground, the liquid and the cover. Its cells stay those as built: the map draws every zoom from the
/// ground's tiles and the cells' table. Returns 0, or 1 if the world is refused.
#[no_mangle]
pub extern "C" fn new_world(seed: u32, size: u32, south: f64, north: f64) -> u32 {
    let world = worldgen::terrain::new_island(seed as u64, size as usize, size as usize, island(size, (south, north)));"""),
("""/// Make the ground of the island `new_world` makes from `seed` and `size` (`tape::Ground`): its terrain and its map's
/// squares (`worldgen::new_world`; how its cells are cut does not change the ground), with the default laws.
#[no_mangle]
pub extern "C" fn ground(seed: u32, size: u32) {
    let (seed, cfg) = (seed as u64, Config::default());""",
"""/// Make the ground of the island `new_world` makes from `seed`, `size` and the latitudes `south` and `north`
/// (`tape::Ground`): its terrain and its map's squares (`worldgen::new_world`; how its cells are cut does not change the
/// ground), with the default laws.
#[no_mangle]
pub extern "C" fn ground(seed: u32, size: u32, south: f64, north: f64) {
    let (seed, cfg) = (seed as u64, island(size, (south, north)));"""),
])

edit('web/src/worker.ts', [
("  new_world(seed: number, size: number): number;", "  new_world(seed: number, size: number, south: number, north: number): number;"),
("  | { cmd: 'new'; seed: number; size: number }", "  | { cmd: 'new'; seed: number; size: number; band: [number, number] }"),
("core.new_world(m.seed, m.size) !== 0", "core.new_world(m.seed, m.size, m.band[0], m.band[1]) !== 0"),
])

edit('web/src/tiles-worker.ts', [
("  ground(seed: number, size: number): void;", "  ground(seed: number, size: number, south: number, north: number): void;"),
("  | { cmd: 'init'; wasm: ArrayBuffer; seed: number; size: number }", "  | { cmd: 'init'; wasm: ArrayBuffer; seed: number; size: number; band: [number, number] }"),
("      core.ground(m.seed, m.size);", "      core.ground(m.seed, m.size, m.band[0], m.band[1]);"),
])

edit('web/src/ground.ts', [
("  constructor(source: string, wasm: ArrayBuffer, seed: number, size: number, readonly map: MapOf,",
 "  constructor(source: string, wasm: ArrayBuffer, seed: number, size: number, band: [number, number], readonly map: MapOf,"),
("      w.worker.postMessage({ cmd: 'init', wasm: copy, seed, size }, [copy]);", "      w.worker.postMessage({ cmd: 'init', wasm: copy, seed, size, band }, [copy]);"),
])

edit('web/src/live.ts', [
("  newWorld(seed: number, size: number): void { this.worker.postMessage({ cmd: 'new', seed, size }); }",
 "  /** A new island of `seed` and `size` between the latitudes `band` (south, north; NaN: the core's default band). */\n  newWorld(seed: number, size: number, band: [number, number]): void { this.worker.postMessage({ cmd: 'new', seed, size, band }); }"),
])

edit('web/src/main.ts', [
("coreWasm.slice(0), m.seed, m.w, { w: m.w,", "coreWasm.slice(0), m.seed, m.w, [m.lat_south, m.lat_north], { w: m.w,"),
("""function newWorld(): void {
  const seed = Math.max(0, Math.floor(+$<HTMLInputElement>('seed').value) || 0);
  const size = Math.min(64, Math.max(8, Math.floor(+$<HTMLInputElement>('size').value) || LIVE_SIZE));
  playing = false;
  say('Making the world…');
  live?.newWorld(seed, size);
}""",
"""/** The band of latitudes of the URL's `band=S,N`, degrees, south first; [NaN, NaN] (the core's default band) without
 * one, or where S is not below N. */
function bandOf(text: string | null): [number, number] {
  const v = (text ?? '').split(',').map(Number);
  return v.length === 2 && v[0] < v[1] ? [v[0], v[1]] : [NaN, NaN];
}
function newWorld(): void {
  const seed = Math.max(0, Math.floor(+$<HTMLInputElement>('seed').value) || 0);
  const size = Math.min(64, Math.max(8, Math.floor(+$<HTMLInputElement>('size').value) || LIVE_SIZE));
  playing = false;
  say('Making the world…');
  live?.newWorld(seed, size, bandOf(new URLSearchParams(location.search).get('band')));
}"""),
("""['size', `${m.w}×${m.h}`],""", """['size', `${m.w}×${m.h}`], ...(m.globe ? [] : [['band', `${fmt(m.lat_south)}° to ${fmt(m.lat_north)}°`]]),"""),
])
print("ok")
