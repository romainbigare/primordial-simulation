//! The features of the cells (laws.md 5.2), the same for every brain: each cell's own (`own`), then smoothed over its
//! neighbours once to `SIGHTS - 1` times (`Regions`), at a month's start (`cells`).

use crate::brain::ln;
use crate::cover::{Parts, NP, SLOTS};
use crate::contract::{self, Layer};
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

/// What the features of every cell read alike: the cover's parts and their molar masses.
struct Look {
    parts: Option<(Parts, [f64; NP])>,
}

impl Look {
    fn of(w: &World) -> Look {
        let parts = Parts::of(&w.chem).map(|p| {
            let mm = p.mm(&w.chem);
            (p, mm)
        });
        Look { parts }
    }
}

/// The features of cell `i` (`NCELL`) as the last survey left its numbers (`Life::sum`).
fn own(w: &World, look: &Look, i: usize) -> Feat {
    let (n, life) = (w.mesh.cells().n(), &w.life);
    let ln_kg = |kg: f64, area: f64| ln(1.0 + (kg / area) as f32);
    let ground = w.store.amounts(Class::Ground);
    let (c, e, area, cell) = (life.sum(i), contract::place(w, i), w.mesh.cells().area()[i], life.cell_at(i));
    let mut f = [0f32; NCELL];
    (f[T], f[WET], f[HEIGHT]) = ((e.t as f32 - T_MID) / T_SPAN, ln(1.0 + e.liquid as f32), e.z as f32 / DZ_SCALE);
    f[SHADE] = ln(1.0 + contract::hides(w, i)) / LN_CANOPY;
    if let Some((p, mm)) = &look.parts {
        let mut soft = 0.0;
        for x in 0..SLOTS {
            let s = contract::shown(w, cell, Layer::Slot(x)).expect("the cover's parts");
            f[COVER + x] = ln_kg(s.kg, area);
            soft += s.mol[p.sub[0]] * mm[0] + s.mol[p.sub[1]] * mm[1];
        }
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
    f[STRENGTH] = (c[1] / c[0].max(1.0)) as f32;
    f[RESIST] = ln(1.0 + contract::thickness(w, contract::stand(w, i).mass, area) as f32);
    (f[PARTS_HERE], f[CROSSED]) = (ln(1.0 + (cell.parts.iter().map(|p| p.count).sum::<f64>() * per) as f32), ln(1.0 + (cell.crossings as f64 * across) as f32));
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
