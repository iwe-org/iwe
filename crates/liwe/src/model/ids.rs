use std::sync::atomic::{AtomicI64, Ordering};

use super::{LineId, NodeId};

static COUNTER: AtomicI64 = AtomicI64::new(1);

pub fn alloc_node_id() -> NodeId {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn alloc_line_id() -> LineId {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ids;

impl Ids {
    pub fn new() -> Self {
        Ids
    }

    pub fn alloc_node_id(&self) -> NodeId {
        alloc_node_id()
    }

    pub fn alloc_line_id(&self) -> LineId {
        alloc_line_id()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rayon::prelude::*;

    use super::*;

    #[test]
    fn ids_are_positive() {
        assert!(alloc_node_id() > 0);
        assert!(alloc_line_id() > 0);
    }

    #[test]
    fn concurrent_allocation_never_repeats() {
        let ids: Vec<NodeId> = (0..10_000)
            .into_par_iter()
            .map(|_| alloc_node_id())
            .collect();
        let unique: HashSet<NodeId> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }
}
