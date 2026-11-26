//! The ground the page shows and what edits moved since (god mode, handover.md 11): the page's ground is made of the
//! samples of one version (`ground_shown`), and the preview tells it how far edits moved each sample since, for it to
//! draw over that ground until it is made again (`preview`).

use crate::{give, with};

/// The most samples past its brush's radius an edit moves the ground: a basin's rim, 1.5 samples (`pcore::edit`'s
/// basin), and the sample a brush's span rounds out to.
const EDIT_REACH_SAMPLES: f64 = 3.0;

/// A box of the terrain's samples: its west and north columns and rows, and its east and south ones past it (none:
/// west not before east).
#[derive(Clone, Copy)]
pub(crate) struct Area([usize; 4]);

impl Area {
    pub(crate) const NONE: Area = Area([usize::MAX, usize::MAX, 0, 0]);
    /// The samples (of `across` x `down`, `per` a square's side) the brush `b` may move the ground of, on squares of
    /// `side_m` m: those within its radius and `EDIT_REACH_SAMPLES`.
    pub(crate) fn of(b: pcore::edit::Brush, (across, down, per): (usize, usize, f64), side_m: f64) -> Area {
        let r = b.radius_m / side_m * per + EDIT_REACH_SAMPLES;
        let [cx, cy] = b.centre.map(|v| v * per);
        let span = |c: f64, n: usize| ((c - r).floor().max(0.0) as usize, ((c + r).ceil().max(0.0) as usize + 1).min(n));
        let ((x0, x1), (y0, y1)) = (span(cx, across), span(cy, down));
        Area([x0, y0, x1, y1])
    }
    /// The box that holds both.
    pub(crate) fn or(self, o: Area) -> Area {
        let (a, b) = (self.0, o.0);
        Area([a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
    }
    /// Its samples, row by row, as indices of a grid `across` wide.
    fn samples(self, across: usize) -> impl Iterator<Item = usize> {
        let [x0, y0, x1, y1] = self.0;
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| y * across + x))
    }
    /// Its west column, north row, width and height (all none where it holds no sample).
    fn window(self) -> [usize; 4] {
        let [x0, y0, x1, y1] = self.0;
        if x0 >= x1 || y0 >= y1 { [0; 4] } else { [x0, y0, x1 - x0, y1 - y0] }
    }
}

/// The page's ground now shows the samples of version `version` (`ground_version`; its tiles made again from that
/// version's relief): `preview` tells it what edits moved since. Those of the versions before it are forgotten; a
/// version not built since the one shown before is ignored.
#[no_mangle]
pub extern "C" fn ground_shown(version: u32) {
    with(|l| {
        let Some(at) = l.since.iter().position(|v| v.version == version) else { return };
        l.since.drain(..at);
    })
}

/// What edits moved the ground since the samples of the ground the page shows (`ground_shown`), for the page to add to
/// the heights it draws until its ground is made again, over the tiles made of the samples of the version it is of: the
/// window of the terrain's samples it tells, its west column, north row, width and height in samples, that version
/// (`ground_version` as `ground_shown` last took it) and whether it is a part (six u32), then of each sample of the
/// window, row by row, how far its ground moved, m (f32), then the depth of its loose ground once the samples are built
/// again over the ground as it stands (`World::loose_to_build`, m, f32), little-endian. Whole (`part` 0): the window
/// holds every sample that moved, and none moved outside it (an empty window, 0, 0, 0, 0, where none moved); only the
/// samples edits may have moved since (`Built::moved`) are looked at. A part (`part` not 0): the window is the box of
/// samples the last paint may have moved (`Live::painted`), what the page holds outside it stays.
#[no_mangle]
pub extern "C" fn preview(part: u32) -> *const u8 {
    let bytes: Vec<u8> = with(|l| {
        let (shown, newest) = (&l.since[0], l.since.last().expect("a version"));
        let now = |k: usize| l.world.relief.shaped_at(k);
        let across = l.world.terrain_samples().across;
        let window = if part != 0 {
            l.painted.window()
        } else {
            let moved = |k: usize| now(k) != shown.shaped_at(k);
            let may = l.since.iter().fold(Area::NONE, |a, v| a.or(v.moved));
            let held = may.samples(across).filter(|&k| moved(k)).fold(Area::NONE, |a, k| a.or(Area([k % across, k / across, k % across + 1, k / across + 1])));
            held.window()
        };
        let [x0, y0, w, h] = window;
        let ks: Vec<usize> = (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| y * across + x)).collect();
        let dh = ks.iter().map(|&k| (now(k) - shown.shaped_at(k)) as f32);
        // The heights the samples will be built at: those last built with what edits shaped since.
        let built = l.world.terrain_samples();
        let loose = l.world.loose_to_build(&ks, &|j| built.h[j] + now(j) - newest.shaped_at(j));
        let tail = dh.chain(loose.iter().map(|&m| m as f32));
        let head = window.map(|v| v as u32).into_iter().chain([shown.version, (part != 0) as u32]);
        head.flat_map(|v| v.to_le_bytes()).chain(tail.flat_map(|v| v.to_le_bytes())).collect()
    });
    give(bytes)
}
