//! The recorder's work on the pool (feature `threads`; none without). A walk over bodies or cells makes a number or a
//! small record for each item, in parallel; they come back in the order of the items, and the recorder adds them up
//! in that order. Tasks that write different things run side by side. So no sum and no file depends on the threads.

#[cfg(feature = "threads")]
/// Fewest items in a task: an item costs about a microsecond, and a task costs more to start than a few of them.
const MIN_TASK: usize = 32;

/// `f` of each of `items`, in the order of `items`.
pub(crate) fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        items.par_iter().with_min_len(MIN_TASK).map(f).collect()
    }
    #[cfg(not(feature = "threads"))]
    items.iter().map(f).collect()
}

/// `f(i)` for `i` in `0..n`, in order.
pub(crate) fn map_n<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        (0..n).into_par_iter().with_min_len(MIN_TASK).map(f).collect()
    }
    #[cfg(not(feature = "threads"))]
    (0..n).map(f).collect()
}

/// `a()` and `b()`, side by side on the pool; both results, in this order.
pub(crate) fn join<A: Send, B: Send>(a: impl FnOnce() -> A + Send, b: impl FnOnce() -> B + Send) -> (A, B) {
    #[cfg(feature = "threads")]
    {
        rayon::join(a, b)
    }
    #[cfg(not(feature = "threads"))]
    (a(), b())
}
