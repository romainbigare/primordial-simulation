//! The starting terrain drains: a priority flood fills each closed basin to its outlet (`space::drain`).

use pcore::space::Level;

/// Give every cell of `cells` a way down to the low ground (`space::drain::drain`, from it). Noise has closed basins that
/// no liquid ever cut; real wet land drains. On a globe (`wrap`) the low ground is every cell below 0 m; on an island,
/// the cells below 0 m joined to its lowest cell, so ground below 0 m inland is a closed basin too.
pub(crate) fn drain(cells: &Level, z: &mut [f64], wrap: bool) {
    let mut reached: Vec<bool> = z.iter().map(|&h| h < 0.0 && wrap).collect();
    let lowest = (0..z.len()).min_by(|&a, &b| z[a].total_cmp(&z[b])).filter(|&i| !wrap && z[i] < 0.0);
    let mut low: Vec<usize> = lowest.into_iter().collect();
    low.iter().for_each(|&i| reached[i] = true);
    while let Some(i) = low.pop() {
        for &j in cells.around(i).1 {
            if z[j as usize] < 0.0 && !reached[j as usize] {
                reached[j as usize] = true;
                low.push(j as usize);
            }
        }
    }
    let around = |i: usize| {
        let (links, near) = cells.around(i);
        links.iter().zip(near).map(|(&l, &j)| (j as usize, cells.dist()[l as usize]))
    };
    pcore::space::drain::drain(z, reached, around);
}
