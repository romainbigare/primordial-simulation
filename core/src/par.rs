//! Optional threads (feature `threads`; none in a wasm build), with the same results: independent work is made in
//! parallel into rows, then applied in the fixed order of the cells, so no sum depends on which thread was first.

#[cfg(feature = "threads")]
use std::sync::Mutex;

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

/// Run `f` on each task with one of `worlds` and its index, at once on the threads, each taking the next task not yet
/// taken (give the larger tasks first); returns the results in the order of the tasks.
#[cfg(feature = "threads")]
pub(crate) fn apart<T, R, W>(tasks: Vec<T>, worlds: &[Mutex<W>], f: impl Fn(&mut W, usize, T) -> R + Sync) -> Vec<R>
where
    T: Send,
    R: Send,
    W: Send,
{
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let (n, next) = (tasks.len(), AtomicUsize::new(0));
    let tasks: Vec<Mutex<Option<T>>> = tasks.into_iter().map(|t| Mutex::new(Some(t))).collect();
    let out: Vec<Mutex<Option<R>>> = (0..n).map(|_| Mutex::new(None)).collect();
    (0..worlds.len().min(n)).into_par_iter().with_max_len(1).for_each(|i| {
        let w = &mut *worlds[i].lock().expect("a world of the threads");
        while let Some(k) = Some(next.fetch_add(1, Ordering::Relaxed)).filter(|&k| k < n) {
            let t = tasks[k].lock().expect("a task").take().expect("each task once");
            *out[k].lock().expect("a result") = Some(f(w, i, t));
        }
    });
    out.into_iter().map(|r| r.into_inner().expect("a result").expect("each task ran")).collect()
}
