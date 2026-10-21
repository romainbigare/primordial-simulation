//! Optional threads (feature `threads`; none in a wasm build), with the same results: independent work is made in
//! parallel into rows, then applied in the fixed order of the cells, so no sum depends on which thread was first.

/// Output size (numbers) below which a task is not split: a smaller job costs more to hand out than to do.
const CHUNK: usize = 4096;

/// Fill `out` in rows of `width` numbers, row `i` by `f(state, i, row)` with a scratch `state` from `init` per task.
pub fn rows<S>(out: &mut [f64], width: usize, init: impl Fn() -> S + Sync + Send, f: impl Fn(&mut S, usize, &mut [f64]) + Sync + Send) {
    if width == 0 || out.is_empty() {
        return;
    }
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        out.par_chunks_mut(width).enumerate().with_min_len((CHUNK / width).max(1)).for_each_init(init, |st, (i, row)| f(st, i, row));
    }
    #[cfg(not(feature = "threads"))]
    {
        let _ = CHUNK;
        let mut st = init();
        for (i, row) in out.chunks_mut(width).enumerate() {
            f(&mut st, i, row);
        }
    }
}
