//! Who draws "form" (for the tape; no law reads it): members by starting kind, by size class of the brain and by
//! whether "form" was the top-scored option when it was drawn.

/// Size classes of a brain: class k holds the brains of 2^k to 2^(k+1) - 1 hidden units, the last class all larger.
pub const SIZE_CLASSES: usize = 8;

/// Members that drew "form" since the shell last took the count.
#[derive(Clone, Debug, Default)]
pub struct Forms {
    /// By the starting kind of the member's lineage (`Lineage::root`)
    pub by_kind: Vec<f64>,
    /// By the size class of its brain (`SIZE_CLASSES`)
    pub by_size: [f64; SIZE_CLASSES],
    /// Whether "form" had the top score (learned), or not (chance)
    pub by_score: [f64; 2],
}

impl Forms {
    /// `members` of a lineage of starting kind `root` and `hidden` hidden units drew "form", `top`: it had the top score.
    pub(super) fn add(&mut self, (root, hidden): (u32, usize), top: bool, members: f64) {
        let kind = root as usize;
        if self.by_kind.len() <= kind {
            self.by_kind.resize(kind + 1, 0.0);
        }
        self.by_kind[kind] += members;
        self.by_size[(hidden.max(1).ilog2() as usize).min(SIZE_CLASSES - 1)] += members;
        self.by_score[!top as usize] += members;
    }

    /// The counts `o` are added.
    pub(crate) fn absorb(&mut self, o: Forms) {
        if self.by_kind.len() < o.by_kind.len() {
            self.by_kind.resize(o.by_kind.len(), 0.0);
        }
        self.by_kind.iter_mut().zip(o.by_kind).for_each(|(a, b)| *a += b);
        self.by_size.iter_mut().zip(o.by_size).for_each(|(a, b)| *a += b);
        self.by_score.iter_mut().zip(o.by_score).for_each(|(a, b)| *a += b);
    }
}
