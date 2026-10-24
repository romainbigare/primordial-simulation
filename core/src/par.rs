//! Optional threads (feature `threads`; none in a wasm build), with the same results: independent work is made in
//! parallel into rows, then applied in the fixed order of the cells, so no sum depends on which thread was first.

/// Make the calling thread one of the pool's threads. Called first, on the thread that runs the world, so that its
/// parallel loops start at once on that thread, which works on them with the others, instead of waiting for them.
pub fn adopt_thread() {
    #[cfg(feature = "threads")]
    {
        // The pool may already exist (a thread of a test made it): then the loops start from outside it.
        let _ = rayon::ThreadPoolBuilder::new().use_current_thread().build_global();
    }
}

/// Fill `out` in rows of `width` numbers, row `i` by `f(state, i, row)` with a scratch `state` from `init` per task.
/// The rows are cut into tasks only as far as other threads are idle (work stealing): a row may cost a few
/// multiplications or a balance solved by iteration, and a fixed size of task suited only one of them.
pub fn rows<S>(out: &mut [f64], width: usize, init: impl Fn() -> S + Sync + Send, f: impl Fn(&mut S, usize, &mut [f64]) + Sync + Send) {
    if width == 0 || out.is_empty() {
        return;
    }
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        out.par_chunks_mut(width).enumerate().for_each_init(init, |st, (i, row)| f(st, i, row));
    }
    #[cfg(not(feature = "threads"))]
    {
        let mut st = init();
        for (i, row) in out.chunks_mut(width).enumerate() {
            f(&mut st, i, row);
        }
    }
}

/// Run `f` on each item, each idle thread taking the next item in their order (so the first items start first), or in
/// order without threads. Each item owns all it writes (a domain of bodies, `survey::on_domains`), so the order and the
/// threads change nothing.
pub fn each<T: Send>(items: &mut [T], f: impl Fn(&mut T) + Sync + Send) {
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        items.iter_mut().par_bridge().for_each(f);
    }
    #[cfg(not(feature = "threads"))]
    items.iter_mut().for_each(f);
}
