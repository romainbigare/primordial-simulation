/// Place on each cell the kinds of the classes nearest its climate (`features`, `year::distance`): each class's stand
/// (`Class::set`) at the mass per m² it holds there times the class's weight by inverse distance, 1/d² over its sum
/// (Shepard 1968; d the climate distance to the class's representative, which takes its own class alone), so the
/// cover changes as smoothly as the climate does. Kinds within `APART` × `alike` of each other (`cover::distance`)
/// share a starting kind, numbered from `FIRST_ROOT` in the order of the classes and their kinds, and join as arriving
/// seeds of one kind do (`cover::lodge`); of the starting kinds the heaviest that fit the cell's `SLOTS` are placed,
/// each with a generation's scatter of its genes (`cover::mutate`, keyed by the cell).
fn place(w: &mut World, p: &Parts, (classes, cells, features): (&[Vec<usize>], &[usize], &[year::Features]), gardens: &[Class]) -> Placed {
    let alike = APART * w.cfg.phys.alike;
    let sets: Vec<Vec<(Entrant, f64)>> = gardens.iter().map(|g| g.set(w, p)).collect();
    let mut roots: Vec<[f32; NG]> = Vec::new();
    let mut root_of = |g: &[f32; NG]| match roots.iter().position(|r| cover::distance(r, g) <= alike) {
        Some(r) => FIRST_ROOT + r as u32,
        None => {
            roots.push(*g);
            FIRST_ROOT + roots.len() as u32 - 1
        }
    };
    let labelled: Vec<Vec<([f32; NG], u32, f64)>> = sets.iter().map(|s| s.iter().map(|(e, kg)| (e.genes, root_of(&e.genes), *kg)).collect()).collect();
    let (mut out, cover_mut) = (Placed { cells: 0, kg: 0.0, roots: roots.len() }, w.cfg.phys.cover_mut);
    let spread = year::spread(features);
    for members in classes {
        for &m in members {
            let i = cells[m];
            let near: Vec<f64> = classes.iter().map(|c| 1.0 / year::distance(&features[m], &features[c[0]], &spread)).collect();
            let weight: Vec<f64> = match near.iter().position(|d| d.is_infinite()) {
                Some(own) => (0..classes.len()).map(|c| (c == own) as u8 as f64).collect(),
                None => near.iter().map(|d| d / near.iter().sum::<f64>()).collect(),
            };
            let mut kinds: Vec<([f32; NG], u32, f64)> = labelled.iter().zip(&weight).flat_map(|(set, &wt)| set.iter().map(move |&(g, r, kg)| (g, r, kg * wt))).collect();
            let mass = |r: u32| kinds.iter().filter(|e| e.1 == r).map(|e| e.2).sum::<f64>();
            let mut held: Vec<(u32, f64)> = kinds.iter().map(|e| (e.1, mass(e.1))).collect();
            held.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            held.dedup_by_key(|e| e.0);
            held.truncate(cover::SLOTS);
            kinds.retain(|e| e.2 > 0.0 && held.iter().any(|h| h.0 == e.1));
            kinds.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.1.cmp(&b.1)));
            let mut r = Rng::new(w.seed, i as u64, 0, Purpose::Nursery);
            let mut any = false;
            for (genes, root, kg) in kinds {
                if cover::place(w, i, cover::mutate(&genes, cover_mut, &mut r), root, kg) {
                    (any, out.kg) = (true, out.kg + kg * w.mesh.cells().area()[i]);
                }
            }
            out.cells += any as usize;
        }
    }
    out
}

