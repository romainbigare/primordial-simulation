//! Link fields: what a link of the cell level remembers between its two cells (`Level::a`, `Level::b`), one value per
//! link in link order. A link holds almost nothing; only a link that something used a lot remembers it.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Links {
    /// Ground cleared of bodies along each link, m² (`lanes::clear`, `lanes::regrow`).
    pub cleared: Vec<f32>,
}

impl Links {
    /// Links with nothing remembered, for a cell level of `links` links.
    pub fn new(links: usize) -> Links {
        Links { cleared: vec![0.0; links] }
    }
    /// What each link remembers follows it to its new number (`World::remap`): a new link remembers nothing.
    pub(crate) fn remap(&mut self, r: &crate::remap::Renumber) {
        let Links { cleared } = self;
        *cleared = r.apply(std::mem::take(cleared), || 0.0);
    }
}
