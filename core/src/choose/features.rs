//! The features of the cells (laws.md 5.2), the same for every brain: each cell's own (`own`), then smoothed over its
//! neighbours once to `SIGHTS - 1` times (`Regions`). At a month's start every cell's (`cells`); at a sub-step those the
//! bodies that decide then read, worked out as `cells` would (`Near`).

use crate::brain::ln;
use crate::cover::{Parts, NP, SLOTS};
use crate::lanes::thickness;
use crate::store::Class;
use crate::World;

/// ln(1 + the cover's capture area per m² of its cell, `World::canopy`) under a dense cover (10 m² per m²).
const LN_CANOPY: f32 = 2.4;
/// The ground temperature, from `T_MID` over `T_SPAN`, K.
const T_MID: f32 = 285.0;
const T_SPAN: f32 = 15.0;
/// The surface height, m.
const DZ_SCALE: f32 = 200.0;
/// ln(1 + the members of a cell per square of the map).
const LN_MEMBERS: f32 = 10.0;
/// The members of the three classes of animals of a cell's features weigh below `SMALL_KG`, up to `BIG_KG`, and more
/// (`docs/numbers.md`).
const SMALL_KG: f32 = 1.0;
const BIG_KG: f32 = 100.0;

/// The features of a cell (`own`): its ground temperature, the depth of its standing liquid (ln(1 + depth / m)), its
/// surface height, the shade of its cover, the mass of each slot of its cover and of the cover's fuel and body matter
/// (what most diets eat), the fuel and body matter loose on its ground, the mass of its animals in three classes of
/// member mass, their members and mean strength, how hard it is to push through, its parts, and the members that moved
/// in per month; masses per m² as ln(1 + kg/m²), counts per square of the map (`Square::squares`), the members that
/// moved in per square's side of its border.
pub const T: usize = 0;
pub const WET: usize = 1;
pub const HEIGHT: usize = 2;
const SHADE: usize = 3;
const COVER: usize = 4;
pub const EDIBLE: usize = COVER + SLOTS;
pub const DEAD: usize = EDIBLE + 1;
const ANIMALS: usize = DEAD + 1;
const MEMBERS: usize = ANIMALS + 3;
const STRENGTH: usize = MEMBERS + 1;
const RESIST: usize = STRENGTH + 1;
const PARTS_HERE: usize = RESIST + 1;
const CROSSED: usize = PARTS_HERE + 1;
pub const NCELL: usize = CROSSED + 1;
/// The levels of the cells' features (`Life::feat`): the cells' own, then smoothed once to `SIGHTS - 1` times over
/// their neighbours (a cell finer than a square as the square that holds it, `regions`), what a body of sight 1 to
/// `SIGHTS` sees of the cell it would step into.
pub const SIGHTS: usize = 4;

/// The features of one cell.
type Feat = [f32; NCELL];

/// After the survey at a month's start: the features of every cell (`own`), then the levels of `SIGHTS` past the first
/// (`Regions::sights`, `Life::feat`).
pub fn cells(w: &mut World) {
    let look = Look::of(w);
    let mut feat: Vec<Feat> = (0..w.mesh.cells().n()).map(|i| own(w, &look, i)).collect();
    Regions::of(w).sights(w, &mut feat);
    w.life.feat = feat;
}

/// What the features of every cell read alike: the cover's parts and their molar masses, and how many months have
/// passed since the members that moved in were last counted (`Cell::crossings`).
struct Look {
    parts: Option<(Parts, [f64; NP])>,
    monthly: f64,
}

impl Look {
    fn of(w: &World) -> Look {
        let parts = Parts::of(&w.chem).map(|p| {
            let mm = p.mm(&w.chem);
            (p, mm)
        });
        // The members that moved in since the last month's survey, per month: they came in the last month at a month's
        // start, and since the month's start inside it (`survey::substeps`).
        let month = w.cfg.calendar().month;
        Look { parts, monthly: match w.time % month { 0 => 1.0, t => month as f64 / t as f64 } }
    }
}

/// The features of cell `i` (`NCELL`) as the last survey left its numbers (`Life::sum`).
fn own(w: &World, look: &Look, i: usize) -> Feat {
    let (n, life) = (w.mesh.cells().n(), &w.life);
    let ln_kg = |kg: f64, area: f64| ln(1.0 + (kg / area) as f32);
    let ground = w.store.amounts(Class::Ground);
    let (c, e, area, cell) = (life.sum(i), &life.env[i], w.mesh.cells().area()[i], life.cell_at(i));
    let mut f = [0f32; NCELL];
    (f[T], f[WET], f[HEIGHT]) = ((e[0] as f32 - T_MID) / T_SPAN, ln(1.0 + e[1] as f32), e[2] as f32 / DZ_SCALE);
    f[SHADE] = ln(1.0 + w.canopy[i]) / LN_CANOPY;
    if let Some((p, mm)) = &look.parts {
        for x in 0..SLOTS {
            f[COVER + x] = ln_kg(cell.cover.kg(mm, x), area);
        }
        let soft: f64 = (0..SLOTS).map(|x| cell.cover.mol[x][0] * mm[0] + cell.cover.mol[x][1] * mm[1]).sum();
        f[EDIBLE] = ln_kg(soft, area);
        f[DEAD] = ln_kg(p.sub[..2].iter().zip(mm).map(|(&k, m)| ground[k * n + i] * m).sum(), area);
    }
    for o in cell.bodies.iter().filter(|o| o.alive) {
        let class = if o.vis[0] < SMALL_KG { 0 } else if o.vis[0] < BIG_KG { 1 } else { 2 };
        f[ANIMALS + class] += o.vis[0] * o.n() as f32;
    }
    for class in 0..3 {
        f[ANIMALS + class] = ln(1.0 + f[ANIMALS + class] / area as f32);
    }
    // (Counts per square of the map's area; the members that moved in per square's side of its border, for what
    // crosses a border goes with its length.)
    let (per, across) = (1.0 / w.map.squares(i), 1.0 / w.map.squares(i).sqrt());
    f[MEMBERS] = ln(1.0 + (c[0] * per) as f32) / LN_MEMBERS;
    f[STRENGTH] = (c[3] / c[0].max(1.0)) as f32;
    f[RESIST] = ln(1.0 + thickness(w, c[1], area) as f32);
    (f[PARTS_HERE], f[CROSSED]) = (ln(1.0 + (cell.parts.len() as f64 * per) as f32), ln(1.0 + (cell.crossings as f64 * look.monthly * across) as f32));
    f
}

/// The cells a body of sight 2 and more sees as one, its regions: each cell alone, but a cell finer than a square of the
/// map with the others of the square that holds it, so what a body sees past the cell it would step into reaches as far
/// in metres from a fine cell as from a square. The region of each cell, the cells of each region in order (the
/// regions in the order of their first cells), the area of each region, and the regions around each, each once, in the
/// order of its cells' links (those across the corners of its cells too, `across_corners`).
pub(crate) struct Regions {
    of: Vec<usize>,
    list: Vec<Vec<usize>>,
    area: Vec<f64>,
    around: Vec<Vec<usize>>,
}

impl Regions {
    pub(crate) fn of(w: &World) -> Regions {
        let (cells, square, mut first, mut list) = (w.mesh.cells(), w.map.square_depth(), std::collections::BTreeMap::new(), Vec::<Vec<usize>>::new());
        let of: Vec<usize> = (w.map.leaves().iter().enumerate()).map(|(i, l)| {
            let mut up = *l;
            while up.depth > square {
                up = up.parent().expect("a cell finer than a square lies in one");
            }
            let r = *first.entry(up).or_insert_with(|| {
                list.push(Vec::new());
                list.len() - 1
            });
            list[r].push(i);
            r
        }).collect();
        let area = list.iter().map(|c| if c.len() == 1 { cells.area()[c[0]] } else { c.iter().map(|&i| cells.area()[i]).sum() }).collect();
        let around = (list.iter().enumerate()).map(|(r, c)| {
            let mut near = Vec::new();
            for q in c.iter().flat_map(|&i| cells.around(i).1.iter().map(|&j| j as usize).chain(across_corners(w, i))).map(|j| of[j]) {
                if q != r && !near.contains(&q) {
                    near.push(q);
                }
            }
            near
        }).collect();
        Regions { of, list, area, around }
    }
    /// The levels of `SIGHTS` past the first, after the cells' own features `feat`: each level the last smoothed over
    /// each region (`first`, `next`); a cell sees its region's (`Life::feat`).
    fn sights(&self, w: &World, feat: &mut Vec<Feat>) {
        let mut last: Vec<Feat> = (0..self.list.len()).map(|r| self.first(w, r, feat)).collect();
        for _ in 1..SIGHTS {
            let next: Vec<Feat> = (0..self.list.len()).map(|r| self.next(r, &last)).collect();
            feat.extend(self.of.iter().map(|&r| next[r]));
            last = next;
        }
    }
    /// The first level of region `r`: its cells' own features `feat`, their mean by area.
    fn first(&self, w: &World, r: usize, feat: &[Feat]) -> Feat {
        let (c, a) = (&self.list[r], self.area[r]);
        if c.len() == 1 {
            return feat[c[0]];
        }
        let mut f = [0f32; NCELL];
        for &i in c {
            f.iter_mut().zip(&feat[i]).for_each(|(x, y)| *x += (w.mesh.cells().area()[i] / a) as f32 * y);
        }
        f
    }
    /// The next level of region `r` from the last, `last`: its own and those of the regions around it, each by its area
    /// over the region's own.
    fn next(&self, r: usize, last: &[Feat]) -> Feat {
        let (mut f, mut sum) = (last[r], 1.0);
        for &q in &self.around[r] {
            let by = (self.area[q] / self.area[r]) as f32;
            f.iter_mut().zip(&last[q]).for_each(|(a, b)| *a += by * b);
            sum += by;
        }
        f.map(|v| v / sum)
    }
}

/// The cells across the four corners of cell `i`, if it is finer than a square or a square beside one, one depth apart
/// from it at most, a cell finer than a square counted as its square (as `Regions` sees it): those a corner of the
/// squares would link it to (`Square::mesh`), where the links of fine cells, at most 8, leave some out; so a region of
/// fine cells has the squares across its corners around it as a square does.
fn across_corners(w: &World, i: usize) -> impl Iterator<Item = usize> + '_ {
    let (leaves, square) = (w.map.leaves(), w.map.square_depth());
    let (l, finer) = (leaves[i], |j: &u32| leaves[*j as usize].depth > square);
    let (x, y, s) = (l.x as i64, l.y as i64, l.side() as i64);
    let near = l.depth > square || (l.depth == square && w.mesh.cells().around(i).1.iter().any(finer));
    let corners = [(x - 1, y - 1), (x + s, y - 1), (x - 1, y + s), (x + s, y + s)].into_iter().filter(move |_| near);
    corners.filter_map(|(a, b)| w.mesh.tree().at(a, b)).filter(move |&k| w.map.leaves()[k].depth.min(square).abs_diff(square) <= 1)
}

/// The features inside a month's sub-steps (`survey::substeps`), where only the bodies of the cells that step decide: a
/// body reads its own cell's own features and those of the cells around it at each level (`choose::options`, `plan`),
/// and a level of a cell reads the level before of its region and those around it, so a sub-step works out only those,
/// each as `cells` would. The regions of the month; per cell the pass (`Touched::pass`) its own features were last
/// worked out at; per region its first level and whether that holds its cells' own features now; per level past the
/// first the values of the regions it needs; and a mark per region of the levels (`at`), so each is listed once.
pub(crate) struct Near {
    regions: Regions,
    own_at: Vec<u64>,
    first_ok: Vec<bool>,
    levels: Vec<Vec<Feat>>,
    mark: Vec<usize>,
}

impl Near {
    /// The features of the month as its survey left them (`cells`), on the regions of its cells.
    pub(crate) fn new(w: &World) -> Near {
        let regions = Regions::of(w);
        let (n, r) = (w.mesh.cells().n(), regions.list.len());
        Near { regions, own_at: vec![0; n], first_ok: vec![false; r], levels: vec![vec![[0.0; NCELL]; r]; SIGHTS], mark: vec![SIGHTS; r] }
    }
    /// The cells whose numbers (`Life::sum`) and own features the bodies that decide in the cells `deciding` read: those
    /// cells and the cells around them, and every cell of the regions their levels read (`levels`), in order, each once.
    pub(crate) fn reads(&mut self, w: &World, deciding: &[u32]) -> (Vec<u32>, Vec<Vec<usize>>) {
        let cells = w.mesh.cells();
        let mut seen: Vec<u32> = deciding.iter().flat_map(|&i| std::iter::once(i).chain(cells.around(i as usize).1.iter().copied())).collect();
        seen.sort_unstable();
        seen.dedup();
        let levels = self.levels_of(&seen);
        let mut reads: Vec<u32> = levels[0].iter().flat_map(|&r| self.regions.list[r].iter().map(|&i| i as u32)).chain(seen.iter().copied()).collect();
        reads.sort_unstable();
        reads.dedup();
        (reads, vec![seen.iter().map(|&i| i as usize).collect()].into_iter().chain(levels).collect())
    }
    /// The regions each level reads, from the last down to the first: the last those of the cells `seen`, each level
    /// before them those and the regions around them.
    fn levels_of(&mut self, seen: &[u32]) -> Vec<Vec<usize>> {
        let mut at: Vec<usize> = Vec::new();
        for &i in seen {
            let r = self.regions.of[i as usize];
            if self.mark[r] != SIGHTS - 1 {
                self.mark[r] = SIGHTS - 1;
                at.push(r);
            }
        }
        let mut levels = vec![at];
        for s in (0..SIGHTS - 1).rev() {
            let mut at = levels[0].clone();
            at.iter().for_each(|&r| self.mark[r] = s);
            for &r in &levels[0] {
                for &q in &self.regions.around[r] {
                    if self.mark[q] != s {
                        self.mark[q] = s;
                        at.push(q);
                    }
                }
            }
            levels.insert(0, at);
        }
        levels.iter().flatten().for_each(|&r| self.mark[r] = SIGHTS);
        levels
    }
    /// The features the bodies that decide read (`reads`: the cells, and the regions of each level), as `cells` would
    /// work them out now: the own features of a cell held since they were last worked out (`Touched::at`), or whose
    /// members that moved in count per month by the time (`Look::monthly`), worked out again; the first level of a
    /// region whose cells' own features changed, or that does not hold them, again; the other levels of the regions
    /// read, from the level before.
    pub(crate) fn features(&mut self, w: &mut World, (cells, levels): (&[u32], &[Vec<usize>])) {
        let (look, n, pass) = (Look::of(w), w.mesh.cells().n(), w.life.touched.pass);
        let mut feat = std::mem::take(&mut w.life.feat);
        for &i in cells {
            let i = i as usize;
            let held = w.life.touched.at.get(i).is_none_or(|&at| at > self.own_at[i]);
            if held || w.life.cell_at(i).crossings > 0.0 {
                feat[i] = own(w, &look, i);
                (self.own_at[i], self.first_ok[self.regions.of[i]]) = (pass, false);
            }
        }
        for &r in &levels[1] {
            if !self.first_ok[r] {
                (self.levels[0][r], self.first_ok[r]) = (self.regions.first(w, r, &feat), true);
            }
        }
        for s in 1..SIGHTS {
            let (done, ahead) = self.levels.split_at_mut(s);
            for &r in &levels[s + 1] {
                ahead[0][r] = self.regions.next(r, &done[s - 1]);
            }
            for &i in &levels[0] {
                feat[s * n + i] = ahead[0][self.regions.of[i]];
            }
        }
        w.life.feat = feat;
    }
}
