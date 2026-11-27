//! Edits by a brush (handover.md 11): the cells under a round brush and the share of each it covers; the ground under
//! it raised and lowered, booked from the outside node, and the samples built again over it when the edits settle.

use pcore::chem::Phase;
use pcore::edit::{Act, Brush, Edit};
use pcore::World;
use worldgen::testkit::{inland, small_world, Books};

/// The area of a disc of radius `r`, in squares a side.
fn disc(r: f64) -> f64 {
    std::f64::consts::PI * r * r
}

/// A brush covers the cells under it by the terrain's samples whose centres lie within its radius: the squares it
/// covers, summed over its cells, are its disc's area to within the samples along its edge, and each cell's place lies
/// in that cell; a brush finer than a sample covers the sample under its centre alone.
#[test]
fn a_brush_covers_its_disc_of_the_cells_under_it() {
    let w = small_world();
    let centre = [w.map.w as f64 / 2.0, w.map.h as f64 / 2.0];
    let side = w.map.side;
    let under = w.under(&[Brush { centre, radius_m: 1.5 * side }]);
    let covered: f64 = under.iter().map(|u| u.share * w.map.squares(u.cell)).sum();
    assert!((covered / disc(1.5) - 1.0).abs() < 0.02, "covered {covered} squares of a disc of {}", disc(1.5));
    for u in &under {
        let [x0, y0, x1, y1] = w.map.bounds(u.cell);
        assert!(u.share > 0.0 && u.share <= 1.0 && (x0..x1).contains(&u.place[0]) && (y0..y1).contains(&u.place[1]), "{u:?} in {:?}", w.map.bounds(u.cell));
    }
    assert!(under.windows(2).all(|p| p[0].cell < p[1].cell), "the cells in their order");
    let fine = w.under(&[Brush { centre, radius_m: 1.0 }]);
    let samples = (side / w.map.length(pcore::space::terrain::SAMPLE_DEPTH)).powi(2);
    assert_eq!(fine.len(), 1, "{fine:?}");
    assert!((fine[0].share * w.map.squares(fine[0].cell) * samples - 1.0).abs() < 1e-12, "one sample: {fine:?}");
}

/// The volume of the solids that hold the ground up (`World::holds_up`) in cell `i`'s loose ground and in its bed, m³.
fn ground_volumes(w: &World, i: usize) -> [f64; 2] {
    let v = |at: &dyn Fn(usize) -> f64| (0..w.chem.ns()).filter(|&k| w.holds_up(k)).map(|k| at(k) * w.chem.molar_volume(k, Phase::Solid)).sum::<f64>();
    [v(&|k| w.g(k, i)), v(&|k| w.b(k, i))]
}

/// An edit of the ground under a brush inside cell `i` (radius 0.4 of a square about its middle), raising it by
/// `height` m at its centre.
fn shape(w: &mut World, i: usize, height: f64) -> bool {
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let act = Act::Ground { radius_m: 0.4 * w.map.side, height_m: height, stroke_m: 0.0, make: 0 };
    w.edit(&[Edit { place: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], act }])[0]
}

/// The volume the edits shaped on the terrain's samples, m³ (`Relief::shaped` by each sample's area).
fn shaped(w: &World) -> f64 {
    let s = w.terrain_samples();
    w.relief.shaped.iter().enumerate().map(|(k, &m)| m * s.area(k)).sum()
}

/// A raise lays the cell's own rock and, at the top of a stroke, `edit_top_m` of loose ground: the cell's ground rises by
/// the volume its samples rose by, the books are kept. When the edits settle, the samples are built again over it (a
/// new version of the relief, no build of the years): against the same world's samples built again with no edit, its
/// sample nearest its centre stands higher by what the relief holds it raised (the raise, worn as it settled:
/// `World::wear_edits`) and a sample far from it stands where it does.
#[test]
fn a_raised_brush_raises_its_disc_and_keeps_the_books() {
    let mut w = small_world();
    let i = inland(&w);
    let (books, z0, [loose0, rock0]) = (Books::now(&w), w.ground_z(i), ground_volumes(&w, i));
    let (years, version) = (w.built_years(), w.relief.version);
    let mut control = w.clone();
    control.relief.due = true;
    control.settle();
    let before = control.terrain_samples();
    let height = 20.0;
    assert!(shape(&mut w, i, height));
    books.assert_kept(&w, 1e-9);
    let (v, area) = (shaped(&w), w.mesh.cells().area()[i]);
    let [loose, rock] = ground_volumes(&w, i);
    let top = w.cfg.edit_top_m / height;
    assert!(((w.ground_z(i) - z0) * area / v - 1.0).abs() < 1e-9, "the ground rose {} m for {v} m3", w.ground_z(i) - z0);
    assert!(((loose - loose0) / (top * v) - 1.0).abs() < 1e-9 && ((rock - rock0) / ((1.0 - top) * v) - 1.0).abs() < 1e-9, "loose {} and rock {} of {v} m3", loose - loose0, rock - rock0);
    let centre = (0..w.relief.shaped.len()).max_by(|&a, &b| w.relief.shaped[a].total_cmp(&w.relief.shaped[b])).unwrap();
    assert!(w.relief.shaped[centre] > 0.9 * height, "the brush's centre rose {} m", w.relief.shaped[centre]);
    let settled = w.settle();
    books.assert_kept(&w, 1e-9);
    assert!(settled.ground && w.relief.version == version + 1 && w.built_years() == years && !w.relief.due, "{settled:?}, version {}", w.relief.version);
    let after = w.terrain_samples();
    let far = (centre + after.h.len() / 2) % after.h.len();
    let rose = w.relief.shaped[centre];
    assert!(rose > 0.0 && ((after.h[centre] - before.h[centre]) / rose - 1.0).abs() < 1e-6, "the centre rose {} m for {rose}", after.h[centre] - before.h[centre]);
    assert!((after.h[far] - before.h[far]).abs() < 1e-3, "a sample far away moved {} m", after.h[far] - before.h[far]);
}

/// The samples' loose ground follows an edit (`mantle::edited`): a hill raised as steep as loose ground rests at (it
/// slides no steeper, `World::rest`) keeps the less of it the steeper it stands, its steep flanks (over three quarters
/// of `REST`, an eighth of its samples or more) less than half the share of their own its gentle ground keeps, its
/// gentle top keeps loose ground (no more than lay there), and a sample no edit shaped keeps its own; what a shell
/// reads of it before the samples are built again (`World::loose_to_build`) is what the build gives.
#[test]
fn a_steep_raised_hill_thins_its_flanks() {
    let mut w = small_world();
    let i = inland(&w);
    let mut control = w.clone();
    control.relief.due = true;
    control.settle();
    let before = control.terrain_samples();
    let height = 0.5 * w.map.side;
    assert!(shape(&mut w, i, height));
    let ks: Vec<usize> = (0..w.relief.shaped.len()).filter(|&k| w.relief.shaped[k] != 0.0).collect();
    w.settle();
    let after = w.terrain_samples();
    let read = w.loose_to_build(&ks, &|j| after.h[j]);
    assert!(ks.iter().zip(&read).all(|(&k, &m)| m == after.loose[k]), "the shell reads the build's loose ground");
    // The share of its own loose ground each sample keeps, by its slope across it (as the mantle reads it).
    let dy = w.map.length(pcore::space::terrain::SAMPLE_DEPTH);
    let slope = |k: usize| ((after.h[k + 1] - after.h[k - 1]) / (2.0 * after.rows[k / after.across] / dy)).hypot((after.h[k + after.across] - after.h[k - after.across]) / (2.0 * dy));
    let kept = |steep: &dyn Fn(f64) -> bool| {
        let shares: Vec<f64> = ks.iter().filter(|&&k| before.loose[k] > 0.0 && steep(slope(k))).map(|&k| after.loose[k] / before.loose[k]).collect();
        (shares.iter().sum::<f64>() / shares.len() as f64, shares.len())
    };
    let ((flanks, n_flanks), (gentle, n_gentle)) = (kept(&|s| s > 0.75 * pcore::space::REST), kept(&|s| s < 0.25 * pcore::space::REST));
    assert!(n_flanks > ks.len() / 8 && n_gentle > 0 && flanks < 0.5 * gentle, "{n_flanks} flanks keep {flanks}, {n_gentle} gentle samples {gentle}");
    assert!(ks.iter().all(|&k| after.settled[k] <= after.loose[k]), "more settled than lies loose");
    let top = *ks.iter().max_by(|&&a, &&b| w.relief.shaped[a].total_cmp(&w.relief.shaped[b])).unwrap();
    assert!(after.loose[top] > 0.0 && after.loose[top] <= before.loose[top], "the top keeps {} of {} m", after.loose[top], before.loose[top]);
    let far = (0..after.h.len()).find(|&k| w.relief.shaped_at(k) == 0.0 && before.loose[k] > 0.0).unwrap();
    assert_eq!(after.loose[far], before.loose[far], "a sample far away keeps its loose ground");
}

/// The ground a brush shapes stands no steeper than loose ground rests at (`space::REST`; `World::rest`): after a long
/// hold at one place, raising far past where its flanks would stand steeper, no link along a row or a column and no
/// sample (its slope across it, as the samples' loose ground reads it) stands steeper than that, where it stood no
/// steeper before the edits (steep ground no edit made keeps its slope); the ground slid out from under the brush, no
/// sample stands lower than before, and the books are kept.
#[test]
fn a_held_brush_stands_no_steeper_than_loose_ground_rests() {
    use pcore::space::terrain::SAMPLE_DEPTH;
    let mut w = small_world();
    let i = inland(&w);
    let books = Books::now(&w);
    let before = w.terrain_samples();
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let (centre, r) = ([(x0 + x1) / 2.0, (y0 + y1) / 2.0], 0.4 * w.map.side);
    let height = 0.01 * w.map.side;
    for _ in 0..60 {
        assert!(shape(&mut w, i, height));
    }
    books.assert_kept(&w, 1e-9);
    let s = w.terrain_samples();
    let (per, dy) = ((s.across / w.map.w) as f64, w.map.length(SAMPLE_DEPTH));
    let near = |k: usize| (((k % s.across) as f64 + 0.5) / per - centre[0]).hypot(((k / s.across) as f64 + 0.5) / per - centre[1]) * w.map.side < 3.0 * r;
    let now = |k: usize| s.height_now(&w.relief, k);
    // The slope across sample `k` of heights `h`, from its neighbours east and west and north and south.
    let across = |h: &dyn Fn(usize) -> f64, k: usize| {
        let dx = s.rows[k / s.across] / dy;
        ((h(k + 1) - h(k - 1)) / (2.0 * dx)).hypot((h(k + s.across) - h(k - s.across)) / (2.0 * dy))
    };
    let (mut links, mut slid) = (0, false);
    for k in (0..s.h.len()).filter(|&k| near(k)) {
        for (j, len) in [(k + 1, s.rows[k / s.across] / dy), (k + s.across, dy)] {
            let (now, was) = ((now(k) - now(j)).abs() / len, (before.h[k] - before.h[j]).abs() / len);
            assert!(now <= pcore::space::REST.max(was) + 1e-9, "the link from {k} to {j} stands at {now} (before {was})");
            links += 1;
        }
        let (now, was) = (across(&now, k), across(&|j| before.h[j], k));
        assert!(now < pcore::space::REST.max(was), "sample {k} stands at {now} (before {was})");
        assert!(w.relief.shaped_at(k) > -1e-9, "sample {k} stands {} m lower", -w.relief.shaped_at(k));
        slid |= w.relief.shaped_at(k) > 0.0 && ((k % s.across) as f64 + 0.5) / per > centre[0] + 1.5 * r / w.map.side;
    }
    assert!(links > 20 && slid, "{links} links about the brush; ground slid past it: {slid}");
}

/// When the edits settle, the ground a stroke shaped is laid as the start's wear would leave it worn
/// (`World::wear_edits`, `Config::edit_wear_years`): a round hill raised by one brush is round no longer, the samples
/// the brush raised alike (at one distance from its centre, about a corner of samples) standing apart by more than a
/// twentieth of its height, for most of them; no sample stands lower than before the stroke, the ground the stroke did
/// not shape is where it stood, and the books are kept.
#[test]
fn a_raised_hill_settles_worn() {
    let mut w = small_world();
    let i = inland(&w);
    let books = Books::now(&w);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let centre = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    let (radius_m, height_m) = (0.75 * w.map.side, 0.15 * w.map.side);
    assert!(w.edit(&[Edit { place: centre, act: Act::Ground { radius_m, height_m, stroke_m: 0.0, make: 0 } }])[0]);
    let raised = w.relief.shaped.clone();
    w.settle();
    books.assert_kept(&w, 1e-9);
    let s = w.terrain_samples();
    let per = (s.across / w.map.w) as f64;
    assert!((0..s.h.len()).all(|k| raised.get(k).copied().unwrap_or(0.0) != 0.0 || w.relief.shaped_at(k) == 0.0), "the ground no edit shaped stays");
    let low = w.relief.shaped.iter().cloned().fold(0.0, f64::min);
    assert!(low > -1e-9, "no sample stands under where it stood, to within the rounding: {low}");
    // The samples the brush raised alike: the same distances east and south of its centre, either way and either axis.
    let mut alike: std::collections::BTreeMap<(i64, i64), Vec<f64>> = Default::default();
    for k in (0..s.h.len()).filter(|&k| raised[k] > 0.2 * height_m) {
        let [u, v] = [(k % s.across) as f64 + 0.5 - centre[0] * per, (k / s.across) as f64 + 0.5 - centre[1] * per].map(|d| (2.0 * d.abs()).round() as i64);
        alike.entry((u.min(v), u.max(v))).or_default().push(w.relief.shaped_at(k));
    }
    let apart = alike.values().filter(|h| h.len() > 1 && h.iter().cloned().fold(f64::MIN, f64::max) - h.iter().cloned().fold(f64::MAX, f64::min) > height_m / 20.0).count();
    assert!(2 * apart > alike.len(), "{apart} of {} sets of samples raised alike stand apart", alike.len());
}

/// A lowering takes the loose ground first: one shallower than the cell's loose ground leaves its rock as it was; the
/// books are kept.
#[test]
fn a_lowering_takes_the_loose_ground_first() {
    let mut w = small_world();
    let i = inland(&w);
    let (books, [loose0, rock0]) = (Books::now(&w), ground_volumes(&w, i));
    assert!(shape(&mut w, i, -0.5));
    let v = -shaped(&w);
    let [loose, rock] = ground_volumes(&w, i);
    assert!(v > 0.0 && v < loose0 && rock == rock0 && ((loose0 - loose) / v - 1.0).abs() < 1e-9, "took {} loose of {v} m3, rock {rock0} -> {rock}", loose0 - loose);
    books.assert_kept(&w, 1e-9);
}

/// Trees planted show as the world grows them: an edit that plants a kind places the world's grown stand of it
/// (`Act::Grown`, `cover::grown`), the same built cover per m² of the ground it stands on and the same plants by size,
/// its matter from the outside node, so the books are kept; a kind no cell holds yet has no grown stand.
#[test]
fn a_planted_kind_stands_as_the_worlds_grown_stand_of_it() {
    use pcore::cover::{self, Parts};
    use worldgen::life::LIGHT_ROW;
    let mut w = small_world();
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let lands = worldgen::testkit::dry_lands(&w);
    let (from, to) = (lands[0], lands[lands.len() / 2]);
    assert!(cover::grown(&w, LIGHT_ROW as u32).is_none(), "no stand yet");
    worldgen::testkit::cover(&mut w, from, 2.0);
    let mm = Parts::of(&w.chem).unwrap().mm(&w.chem);
    let source = cover::grown(&w, LIGHT_ROW as u32).expect("the stand placed");
    let books = Books::now(&w);
    let [x0, y0, x1, y1] = w.map.bounds(to);
    let place = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    assert_eq!(w.edit(&[Edit { place, act: Act::Clear(0.5) }, Edit { place, act: worldgen::schedule::grown(LIGHT_ROW) }]), [true, true]);
    books.assert_kept(&w, 1e-9);
    let c = w.life.cover(to);
    let x = c.held().find(|&x| c.root[x] == LIGHT_ROW as u32).expect("a slot of the kind");
    let area = cover::ground(&w, to) * c.patch_share(x) * c.stands[x] as f64;
    let per: f64 = source.groups.iter().map(|g| g.kg).sum();
    assert!((c.built(&mm, x) / area / per - 1.0).abs() < 1e-9, "{} kg/m2 for {per}", c.built(&mm, x) / area);
    let mass = |n: f64, share: f64| share * c.built(&mm, x) / n;
    for (k, g) in source.groups.iter().enumerate() {
        assert!((mass(c.plants[x][k], c.share[x][k]) / (g.kg / g.n) - 1.0).abs() < 1e-9, "group {k}: {:?} {:?} for {g:?}", c.plants[x], c.share[x]);
    }
}

/// The terrain's samples whose centres lie within `r` m of `place` (squares), as a brush covers them (`World::under`).
fn disc_samples(w: &World, place: [f64; 2], r: f64) -> Vec<usize> {
    let s = w.terrain_samples();
    let per = (s.across / w.map.w) as f64;
    let m = w.map.length(pcore::space::terrain::SAMPLE_DEPTH);
    (0..s.h.len()).filter(|&k| (((k % s.across) as f64 + 0.5 - place[0] * per).hypot((k / s.across) as f64 + 0.5 - place[1] * per)) * m <= r).collect()
}

/// A basin of liquid (`Act::Basin`): the ground under a brush is cut to a bowl `DEPTH` under the lowest dry sample of
/// its rim at its centre, none of it raised, and the key liquid poured in to its pass with what the pores under it
/// lack, from the outside node, so the books are kept. The floor is a closed basin, a pond that deep to its pass, whose
/// body holds the liquid: on dry ground it covers the brush's middle and stands the asked depth over the floor at once,
/// and still after a month.
#[test]
fn a_basin_holds_its_liquid() {
    const DEPTH: f64 = 5.0;
    // A basin pours until it lacks less than a hundredth of its room (`edit::basin`).
    const FULL_SHARE: f64 = 0.01;
    let mut w = small_world();
    // An inland cell whose samples and its neighbours' lie in no pond: a basin of its own.
    let s0 = w.terrain_samples();
    let dry = |i: usize| w.map.leaves()[i].split(pcore::space::terrain::SAMPLE_DEPTH).all(|f| s0.pond[s0.index(&f)] == pcore::space::network::NONE);
    let i = (0..w.mesh.cells().n()).find(|&i| w.is_land(i) && dry(i) && w.mesh.cells().around(i).1.iter().all(|&j| dry(j as usize))).expect("dry ground");
    drop(s0);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let books = Books::now(&w);
    let (place, r) = ([(x0 + x1) / 2.0, (y0 + y1) / 2.0], 0.4 * w.map.side);
    assert_eq!(w.edit(&[Edit { place, act: Act::Basin { radius_m: r, depth_m: DEPTH } }]), [true]);
    books.assert_kept(&w, 1e-9);
    let s = w.terrain_samples();
    let k = (0..w.relief.shaped.len()).filter(|&k| w.relief.shaped[k] != 0.0).min_by(|&a, &b| s.h[a].total_cmp(&s.h[b])).expect("the floor");
    let pond = s.pond[k];
    assert!(pond != pcore::space::network::NONE, "the floor is no pond");
    let under = disc_samples(&w, place, r);
    assert!(under.iter().all(|&j| w.relief.shaped[j] <= 0.0), "ground under the brush raised");
    let middle = disc_samples(&w, place, 0.5 * r);
    assert!(middle.iter().all(|&j| s.pond[j] == pond), "the liquid covers {} of the brush's middle's {} samples", middle.iter().filter(|&&j| s.pond[j] == pond).count(), middle.len());
    let depth = |w: &World| w.pond_levels()[pond as usize] - s.h[k];
    let (now, full) = (depth(&w), s.ponds[pond as usize].level - s.h[k]);
    assert!(full >= DEPTH - 1e-6 && now >= (1.0 - FULL_SHARE) * DEPTH && now <= full + 1e-6, "the liquid stands {now} m deep, {full} m to its pass");
    pcore::step(&mut w);
    let later = depth(&w);
    assert!((later - now).abs() < 0.2 * full, "after a month it stands {later} m deep, from {now}");
    books.assert_kept(&w, 1e-9);
}

/// A basin that reaches a pond grows it (`World::basin`): its level is the pond's, so the pond's liquid still covers
/// all its samples, at its own level, and the basin's own centre lies in it, with the brushes overlapping by most of
/// one or by a little; a basin beside it, its rim on the pond's bank, leaves the pond as it was. The books are kept.
#[test]
fn a_basin_that_reaches_a_pond_grows_it() {
    use pcore::space::network::NONE;
    let mut w = small_world();
    let i = inland(&w);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let (place, r) = ([(x0 + x1) / 2.0, (y0 + y1) / 2.0], 0.3 * w.map.side);
    let act = Act::Basin { radius_m: r, depth_m: 5.0 };
    assert_eq!(w.edit(&[Edit { place, act }]), [true]);
    let s = w.terrain_samples();
    let centre = |s: &pcore::space::samples::Samples, p: [f64; 2]| {
        let per = (s.across / w.map.w) as f64;
        (p[1] * per) as usize * s.across + (p[0] * per) as usize
    };
    let pond = s.pond[centre(&s, place)];
    assert!(pond != NONE, "the first pond");
    let level = w.pond_levels()[pond as usize];
    let members: Vec<usize> = (0..s.h.len()).filter(|&k| s.pond[k] == pond && s.h[k] < level).collect();
    for apart in [1.0, 1.6, 2.4] {
        let mut v = w.clone();
        let books = Books::now(&v);
        let at = [place[0] + apart * r / v.map.side, place[1]];
        assert_eq!(v.edit(&[Edit { place: at, act }]), [true]);
        books.assert_kept(&v, 1e-9);
        let t = v.terrain_samples();
        let grown = t.pond[centre(&t, at)];
        assert!(grown != NONE, "{apart} radii apart: no pond at the second brush");
        let first = t.pond[members[0]];
        let kept = members.iter().all(|&k| t.pond[k] == first) && (v.pond_levels()[first as usize] - level).abs() < 1e-6;
        assert!(kept && (apart > 2.0 || first == grown), "{apart} radii apart: the first pond not whole at its level in the grown pond: {} from {level}", v.pond_levels()[first as usize]);
    }
}

/// After a stroke Flow finds each cell's ground by height again only where its samples moved (`flow::curve::update`):
/// every cell's land, its ground under the liquid and its levels are those of the same world whose Flow finds them all
/// anew (its saved state kept, its caches dropped, as a snapshot read again holds it), to the last bit.
#[test]
fn the_ground_by_height_found_again_is_the_ground_found_anew() {
    let mut w = small_world();
    let i = inland(&w);
    let stroke = |w: &mut World, height: f64| {
        assert!(shape(w, i, height));
        w.settle();
    };
    stroke(&mut w, 20.0);
    stroke(&mut w, -10.0);
    let mut anew = w.clone();
    anew.scratch = bincode::deserialize(&bincode::serialize(&w.scratch).expect("saved")).expect("read");
    pcore::contract::surroundings(&mut w);
    pcore::contract::surroundings(&mut anew);
    let cells: Vec<usize> = (0..w.mesh.cells().n()).collect();
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
    let shores = |w: &World| pcore::flow::shores(w, &cells).expect("Flow's state").into_iter().flat_map(|(land, parts)| std::iter::once(land).chain(parts)).collect::<Vec<f64>>();
    let levels = |w: &World| pcore::flow::levels(w, &cells).expect("Flow's state").into_iter().flatten().collect::<Vec<f64>>();
    assert!(bits(&shores(&w)) == bits(&shores(&anew)) && bits(&levels(&w)) == bits(&levels(&anew)), "the cells' ground by height");
}

/// A basin of liquid whose floor opens to the open low ground (on a coast, its rim under that ground's liquid) pours
/// nothing: the open low ground's liquid stands there already, and its level is no pass. The books are kept, and
/// the liquid's levels are found (no level of liquid poured without end).
#[test]
fn a_basin_open_to_the_low_ground_pours_nothing() {
    let mut w = small_world();
    let cells = w.mesh.cells();
    let coast = (0..cells.n()).find(|&i| w.is_land(i) && cells.around(i).1.iter().any(|&j| !w.is_land(j as usize))).expect("a coast");
    let [x0, y0, x1, y1] = w.map.bounds(coast);
    let books = Books::now(&w);
    let act = Act::Basin { radius_m: 0.6 * w.map.side, depth_m: 5.0 };
    w.edit(&[Edit { place: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], act }]);
    books.assert_kept(&w, 1e-9);
    assert!(w.pond_levels().iter().all(|l| !l.is_infinite()), "{:?}", w.pond_levels());
    pcore::step(&mut w);
    books.assert_kept(&w, 1e-9);
}

/// What runs into the open low ground of samples `s` of world `w`, mol/s: the yearly flows of the samples in no pond
/// whose liquid runs into it.
fn to_low(w: &World, s: &pcore::space::samples::Samples) -> f64 {
    use pcore::space::network::NONE;
    let open = |k: usize| s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE;
    (0..s.h.len()).filter(|&k| s.pond[k] == NONE && s.to[k] != NONE && open(s.to[k] as usize)).map(|k| w.discharge[k] as f64).sum()
}

/// A world `w` with a hill raised at sample `k` (radius `r` m, 60 m high) and settled.
fn hill_at(w: &World, k: usize, r: f64) -> World {
    let s = w.terrain_samples();
    let per = (s.across / w.map.w) as f64;
    let place = [((k % s.across) as f64 + 0.5) / per, ((k / s.across) as f64 + 0.5) / per];
    let mut v = w.clone();
    v.edit(&[Edit { place, act: Act::Ground { radius_m: r, height_m: HILL_M, stroke_m: 0.0, make: 0 } }]);
    v.settle();
    v
}

/// The height of the hills the tests of the flows raise, m.
const HILL_M: f64 = 60.0;
/// How far what runs into the open low ground may move when a hill moves its channels, a share: the rounding of the
/// yearly flows (32-bit).
const TO_LOW_SHARE: f64 = 1e-4;

/// A hill raised across a channel moves the channel, never drops it: the yearly flows run down the samples' new ways,
/// and a pond the hill dams passes on what comes into it from its pass (`flow::build::reroute`), so what runs into the
/// open low ground is what ran into it before, to the rounding of the flows. Of the test's channels, the first a hill
/// dams into a new pond.
#[test]
fn a_hill_that_dams_a_channel_keeps_what_runs_to_the_low_ground() {
    use pcore::space::network::NONE;
    let w = small_world();
    let s = w.terrain_samples();
    let before = to_low(&w, &s);
    let mut channels: Vec<usize> = (0..s.h.len()).filter(|&k| s.pond[k] == NONE && s.to[k] != NONE && s.h[k] > 0.0 && s.up[k] > 1e7).collect();
    channels.sort_by(|&a, &b| s.up[b].total_cmp(&s.up[a]));
    let passes: Vec<u32> = s.ponds.iter().map(|p| p.pass).collect();
    let dammed = channels.iter().take(DAM_TRIES).find_map(|&k| {
        let v = hill_at(&w, k, 0.3 * w.map.side);
        let t = v.terrain_samples();
        t.ponds.iter().any(|p| p.pass != NONE && !passes.contains(&p.pass)).then(|| to_low(&v, &t))
    });
    let after = dammed.expect("a hill that dams a channel");
    assert!(before > 0.0 && ((after - before) / before).abs() < TO_LOW_SHARE, "ran to the low ground {before} mol/s, now {after}");
}

/// A hill raised on a pond's pass moves its pass: the pond passes on what comes into it from its new pass, and its old
/// pass no longer lets it over (`flow::build::reroute`), so what runs into the open low ground is what ran into it
/// before, to the rounding of the flows, and no flow is counted twice. Of the closed ponds, those that most comes into
/// first, the first whose pass a hill moves to a new sample out of it while it stays a pond.
#[test]
fn a_hill_on_a_pass_keeps_what_runs_to_the_low_ground() {
    use pcore::space::network::NONE;
    let w = small_world();
    let s = w.terrain_samples();
    let before = to_low(&w, &s);
    // The closed ponds, by their keys and passes, those that most comes into first.
    let came = |p: usize| (0..s.h.len()).filter(|&k| s.pond[k] == p as u32).map(|k| w.discharge[k] as f64).sum::<f64>();
    let mut closed: Vec<(usize, u32, f64)> = s.ponds.iter().enumerate().filter(|p| p.1.pass != NONE).filter_map(|(p, pond)| (0..s.h.len()).find(|&k| s.pond[k] == p as u32).map(|k| (k, pond.pass, came(p)))).collect();
    closed.sort_by(|a, b| b.2.total_cmp(&a.2));
    let passes: Vec<u32> = s.ponds.iter().map(|p| p.pass).collect();
    let moved = closed.iter().find_map(|&(key, pass, _)| {
        let v = hill_at(&w, pass as usize, PASS_HILL_SHARE * w.map.side);
        let t = v.terrain_samples();
        let p = t.pond[key];
        (p != NONE && t.ponds[p as usize].pass != NONE && !passes.contains(&t.ponds[p as usize].pass) && t.pond[pass as usize] == NONE).then(|| to_low(&v, &t))
    });
    let after = moved.expect("a hill that moves a pass");
    assert!(before > 0.0 && ((after - before) / before).abs() < TO_LOW_SHARE, "ran to the low ground {before} mol/s, now {after}");
}

/// The radius of the hill raised on a pass, a share of a square's side.
const PASS_HILL_SHARE: f64 = 0.05;

/// The channels a hill is raised across in the test of a dam, at most, the largest first.
const DAM_TRIES: usize = 20;

/// The ponds keep their numbers when the samples are built again after an edit (`Relief::ponds`): a basin adds its
/// own pond and renumbers none it did not reach, and a number never comes to name another pond: after a second basin
/// grows the first pond, each number names none or the pond that holds the key it had (an empty number reads that
/// pond), so whatever reads a pond by its number elsewhere reads the same.
#[test]
fn a_basin_renumbers_no_pond_it_did_not_reach() {
    use pcore::space::network::NONE;
    let mut w = small_world();
    pcore::flow::build::reshape(&mut w);
    let before = w.terrain_samples();
    let i = inland(&w);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let (place, r) = ([(x0 + x1) / 2.0, (y0 + y1) / 2.0], 0.4 * w.map.side);
    let act = Act::Basin { radius_m: r, depth_m: 5.0 };
    assert_eq!(w.edit(&[Edit { place, act }]), [true]);
    let after = w.terrain_samples();
    let members = |s: &pcore::space::samples::Samples, p: u32| (0..s.h.len()).filter(|&k| s.pond[k] == p).collect::<Vec<usize>>();
    // Each pond of before whose samples are those of a pond after: the same pond, at the same number.
    let same: Vec<u32> = (0..before.ponds.len() as u32).filter(|&p| {
        let m = members(&before, p);
        m.first().is_some_and(|&k| after.pond[k] != NONE && members(&after, after.pond[k]) == m)
    }).collect();
    assert!(!same.is_empty(), "ponds away from the basin");
    for p in same {
        let k = members(&before, p)[0];
        assert_eq!(after.pond[k], p, "pond {p}");
    }
    assert!(after.ponds.len() > before.ponds.len(), "the basin's own pond");
    let keys = w.relief.ponds.clone();
    assert_eq!(w.edit(&[Edit { place: [place[0] + r / w.map.side, place[1]], act }]), [true]);
    let grown = w.terrain_samples();
    for (p, &k) in keys.iter().enumerate() {
        let empty = grown.ponds.get(p).is_some_and(|q| q.level.is_nan());
        assert!(empty || (k != NONE && grown.pond[k as usize] == p as u32), "number {p} names a pond that does not hold its key");
    }
}

/// How far under the floor of the open low ground the test's pit is dug at its centre, m.
const PIT_BELOW_M: f64 = 300.0;

/// A pit dug inland below the floor of the open low ground stays a closed basin: the open low ground is what lies under
/// its level joined to the seed's (`drain::open_from`), so the pit's samples are a pond with a pass and the seed's open
/// low ground stays open, however deep the pit. (Its brush is wide enough that its walls stand no steeper than loose
/// ground rests at: the brush's falloff is steepest at 1.54 times its depth over its radius.)
#[test]
fn a_pit_deeper_than_the_open_low_ground_stays_a_closed_pond() {
    use pcore::space::network::NONE;
    let mut w = small_world();
    let i = inland(&w);
    let open = |s: &pcore::space::samples::Samples, k: usize| s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE;
    let s0 = w.terrain_samples();
    let low: Vec<usize> = (0..s0.h.len()).filter(|&k| open(&s0, k)).collect();
    let floor = s0.h.iter().copied().fold(f64::INFINITY, f64::min);
    drop(s0);
    let height = floor - PIT_BELOW_M - w.ground_z(i);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let act = Act::Ground { radius_m: 2.0 * height.abs() / pcore::space::REST, height_m: height, stroke_m: 0.0, make: 0 };
    assert!(w.edit(&[Edit { place: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], act }])[0]);
    w.settle();
    let s = w.terrain_samples();
    let pit = (0..s.h.len()).min_by(|&a, &b| s.h[a].total_cmp(&s.h[b])).unwrap();
    assert!(s.h[pit] < floor && w.relief.shaped_at(pit) < 0.0, "the pit at {} m, the floor at {floor} m", s.h[pit]);
    assert!(s.pond[pit] != NONE && !open(&s, pit), "the pit is open low ground");
    assert!(low.iter().all(|&k| open(&s, k)), "the open low ground is closed");
}

/// An edit whose numbers are not finite is refused before it touches the world (`Edit::valid`): it is told as not made,
/// and the ground's amounts, its heat and the relief stay as they were, to the last bit.
#[test]
fn an_edit_of_numbers_not_finite_is_refused() {
    use pcore::store::Class;
    let mut w = small_world();
    let i = inland(&w);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let place = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    let state = |w: &World| (w.store.amounts(Class::Ground).iter().chain(w.store.heat(Class::Ground)).map(|x| x.to_bits()).collect::<Vec<u64>>(), w.relief.clone());
    let before = state(&w);
    let acts = [
        Act::Ground { radius_m: 0.4 * w.map.side, height_m: f64::NAN, stroke_m: 0.0, make: 0 },
        Act::Basin { radius_m: f64::INFINITY, depth_m: 5.0 },
        Act::Heat { radius_m: 0.4 * w.map.side, joules_m2: f64::INFINITY },
        Act::Clear(f64::NAN),
    ];
    let edits: Vec<Edit> = acts.iter().map(|&act| Edit { place, act }).chain([Edit { place: [f64::NAN, 1.0], act: Act::Strike }]).collect();
    assert!(w.edit(&edits).iter().all(|&done| !done));
    w.settle();
    assert!(state(&w) == before, "the world moved");
}

/// A basin that cannot be made after a stroke (its centre in the open low ground) still leaves the ground the stroke
/// shaped settled: the world is the one the stroke alone and its settling give, to the last bit of the ground's amounts
/// and heat.
#[test]
fn a_basin_not_made_still_settles_the_stroke_before_it() {
    use pcore::store::Class;
    let mut w = small_world();
    let i = inland(&w);
    assert!(shape(&mut w, i, 20.0));
    let mut alone = w.clone();
    let low = (0..w.mesh.cells().n()).find(|&j| !w.is_land(j)).expect("the open low ground");
    let [x0, y0, x1, y1] = w.map.bounds(low);
    let act = Act::Basin { radius_m: 0.3 * w.map.side, depth_m: 5.0 };
    assert_eq!(w.edit(&[Edit { place: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], act }]), [false]);
    w.settle();
    alone.settle();
    let state = |w: &World| w.store.amounts(Class::Ground).iter().chain(w.store.heat(Class::Ground)).map(|x| x.to_bits()).collect::<Vec<u64>>();
    assert!(state(&w) == state(&alone) && w.relief.version == alone.relief.version, "the stroke's ground not settled");
}

/// The liquid poured into a pond (`flow::pour`, as a basin pours it) comes from the outside node with its potential
/// energy where it comes to lie, as the ground an edit raises does (`World::cross`): the cell keeps its temperature, to
/// within the rounding, and the books are kept.
#[test]
fn a_poured_pond_leaves_its_cells_temperature() {
    let mut w = small_world();
    let i = inland(&w);
    let [x0, y0, x1, y1] = w.map.bounds(i);
    let place = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    let act = Act::Basin { radius_m: 0.4 * w.map.side, depth_m: 5.0 };
    assert_eq!(w.edit(&[Edit { place, act }]), [true]);
    let s = w.terrain_samples();
    let per = (s.across / w.map.w) as f64;
    let pond = s.pond[(place[1] * per) as usize * s.across + (place[0] * per) as usize];
    drop(s);
    let k = pcore::cover::Parts::of(&w.chem).expect("the liquid").liquid;
    let m3 = POURED_M * w.mesh.cells().area()[i];
    let mol = m3 * w.chem.subs[k].density[1] / w.chem.subs[k].molar_mass;
    let (books, t0) = (Books::now(&w), w.t_ground(i));
    assert_eq!(pcore::flow::pour(&mut w, pond, k, &[(i, m3)], |w, i, _| worldgen::testkit::put(w, k, i, mol)), [true], "the cell holds no part of its pond");
    books.assert_kept(&w, 1e-9);
    let t = w.t_ground(i);
    assert!((t - t0).abs() < POURED_KELVIN, "the cell went from {t0} K to {t} K");
}

/// The depth over its cell the test pours into a pond, m.
const POURED_M: f64 = 2.0;
/// How far the temperature of a cell a pond is poured into may move, K: the rounding of its heat.
const POURED_KELVIN: f64 = 1e-9;

/// A copied world (a shell's undo keeps one) steps as the world it was copied from, to the last bit: a copy of Flow's
/// working memory leaves out only room its passes write before they read (`flow::spare::Spare`).
#[test]
fn a_copied_world_steps_as_its_original() {
    use pcore::store::Class;
    let mut w = small_world();
    let i = inland(&w);
    assert!(shape(&mut w, i, 20.0));
    w.settle();
    let mut copy = w.clone();
    pcore::step(&mut w);
    pcore::step(&mut copy);
    let bits = |w: &World| [Class::Ground, Class::Air].iter().flat_map(|&c| w.store.amounts(c).iter().chain(w.store.heat(c))).map(|x| x.to_bits()).collect::<Vec<u64>>();
    assert!(bits(&w) == bits(&copy), "the copy stepped apart");
}
