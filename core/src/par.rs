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

/// The least work, in instructions, that the other threads could take from a phase for it to go to the pool (`pays`):
/// a pool call costs 30 to 55 µs, some 10^5 instructions, and four threads save three quarters of what they share.
pub const POOL_WORK: usize = 200_000;

/// The work of a phase whose cost is not counted, for `rows` and `map_n`: it goes to the pool.
pub const UNCOUNTED: usize = usize::MAX;

/// Does a phase pay for a pool call (`POOL_WORK`), where the other threads could take `work` instructions of it
/// (`shareable`)?
pub fn pays(work: usize) -> bool {
    cfg!(feature = "threads") && work >= POOL_WORK
}

/// What the other threads could take of a phase whose items cost `work` instructions each: all but the largest, which
/// one thread does alone.
pub fn shareable(work: impl IntoIterator<Item = usize>) -> usize {
    let (all, most) = work.into_iter().fold((0, 0), |(all, most): (usize, usize), x| (all.saturating_add(x), most.max(x)));
    all - most
}

/// Fill `out` in rows of `width` numbers, row `i` by `f(state, i, row)` with a scratch `state` from `init` per task: on
/// the pool where the other threads could take `work` instructions of it (`pays`), else in order on the calling thread.
/// The rows are cut into tasks only as far as other threads are idle (work stealing): a row may cost a few
/// multiplications or a balance solved by iteration, and a fixed size of task suited only one of them.
pub fn rows<S>(out: &mut [f64], width: usize, work: usize, init: impl Fn() -> S + Sync + Send, f: impl Fn(&mut S, usize, &mut [f64]) + Sync + Send) {
    if width == 0 || out.is_empty() {
        return;
    }
    #[cfg(feature = "threads")]
    if pays(work) {
        use rayon::prelude::*;
        return out.par_chunks_mut(width).enumerate().for_each_init(init, |st, (i, row)| f(st, i, row));
    }
    #[cfg(not(feature = "threads"))]
    let _ = work;
    let mut st = init();
    out.chunks_mut(width).enumerate().for_each(|(i, row)| f(&mut st, i, row));
}

/// The least work of a phase that goes to the pool, counted in its items of work (updates, cells surveyed, views
/// booked): a pool call costs 30 to 55 µs, more than a few updates, so a smaller phase runs on
/// the calling thread.
pub const SERIAL_BELOW: usize = 8;

/// Run `f` on each item with a scratch `state` made by `init` once per thread, not per item; each idle thread takes the
/// next item in their order (so the first items start first). In order on the calling thread without threads, or when
/// the phase's `work` is under `SERIAL_BELOW`. Each item owns all it writes (a chunk of cells, `life::run`; a block of
/// Change's nodes, `longest_first`; an array of the store, `store::book_by_array`), so the order and the threads change
/// nothing.
pub fn each_init<T: Send, S>(items: &mut [T], work: usize, init: impl Fn() -> S + Sync + Send, f: impl Fn(&mut S, &mut T) + Sync + Send) {
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        if work >= SERIAL_BELOW {
            return items.iter_mut().par_bridge().for_each_init(init, f);
        }
    }
    #[cfg(not(feature = "threads"))]
    let _ = work;
    let mut st = init();
    items.iter_mut().for_each(|x| f(&mut st, x));
}

/// `f` on what each block of `work` is given (its index, its count of work, and what it is given), with a scratch made
/// by `init` once per thread (`each_init`), the most work first so the last to end is a small one (longest processing
/// time first, Graham 1969; ties by index): the blocks of Change's reactors under one air node (`change/rows.rs`).
pub fn longest_first<T: Send, S>(mut work: Vec<(usize, usize, T)>, init: impl Fn() -> S + Sync + Send, f: impl Fn(&mut S, &mut T) + Sync + Send) {
    work.sort_by_key(|x| (std::cmp::Reverse(x.1), x.0));
    let total = work.iter().map(|x| x.1).sum();
    each_init(&mut work, total, init, |s, x| f(s, &mut x.2));
}

#[cfg(feature = "threads")]
/// Fewest items in a task of `map` and `map_n`: an item there costs about a microsecond, and a task costs more to start
/// than a few of them.
const MIN_TASK: usize = 32;

/// `f` of each of `items`, in parallel, in the order of `items` (the tape adds them up in that order).
pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        items.par_iter().with_min_len(MIN_TASK).map(f).collect()
    }
    #[cfg(not(feature = "threads"))]
    items.iter().map(f).collect()
}

/// `f(i)` for `i` in `0..n`, in order: on the pool where the other threads could take `work` instructions of it
/// (`pays`), else on the calling thread.
pub fn map_n<R: Send>(n: usize, work: usize, f: impl Fn(usize) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "threads")]
    if pays(work) {
        use rayon::prelude::*;
        return (0..n).into_par_iter().with_min_len(MIN_TASK).map(f).collect();
    }
    #[cfg(not(feature = "threads"))]
    let _ = work;
    (0..n).map(f).collect()
}

/// `a()` and `b()`, side by side on the pool (tasks that write different things); both results, in this order.
pub fn join<A: Send, B: Send>(a: impl FnOnce() -> A + Send, b: impl FnOnce() -> B + Send) -> (A, B) {
    #[cfg(feature = "threads")]
    {
        rayon::join(a, b)
    }
    #[cfg(not(feature = "threads"))]
    (a(), b())
}
