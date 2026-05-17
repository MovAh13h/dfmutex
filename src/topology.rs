use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub type GroupId = usize;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn next_id() -> GroupId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

static GRAPH: OnceLock<Mutex<LockOrderGraph>> = OnceLock::new();

fn global_graph() -> &'static Mutex<LockOrderGraph> {
    GRAPH.get_or_init(|| Mutex::new(LockOrderGraph::default()))
}

#[derive(Default)]
pub(crate) struct LockOrderGraph {
    edges: HashMap<GroupId, HashSet<GroupId>>,
}

impl LockOrderGraph {
    pub(crate) fn add_edge(&mut self, from: GroupId, to: GroupId) {
        self.edges.entry(from).or_default().insert(to);
    }

    // Returns true if adding edges from each node in `held` to `acquiring` would create a cycle.
    // A cycle exists when `acquiring` can already reach any node in `held` via existing edges.
    pub(crate) fn would_create_cycle(&self, held: &[GroupId], acquiring: GroupId) -> bool {
        let reachable = self.reachable(acquiring);
        held.iter().any(|h| reachable.contains(h))
    }

    fn reachable(&self, start: GroupId) -> HashSet<GroupId> {
        let mut visited = HashSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited.insert(node) {
                if let Some(neighbors) = self.edges.get(&node) {
                    stack.extend(neighbors.iter().copied());
                }
            }
        }
        visited
    }
}

thread_local! {
    static HELD: RefCell<Vec<GroupId>> = const { RefCell::new(Vec::new()) };
    // Raw Arc pointer values for every mutex currently held by this thread.
    // Used to detect reentrant locking (same Arc acquired twice) before blocking.
    static HELD_PTRS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// Called before blocking on a lock. Checks for reentrant locking and cycles,
/// then records the acquisition. `mutex_ptr` is `Arc::as_ptr(&inner.mutex) as usize`.
pub fn check_and_register_acquire(group_id: GroupId, mutex_ptr: usize) {
    // Reentrant check first: same Arc on the same thread means same underlying
    // std::sync::Mutex, which is not reentrant and would deadlock silently.
    HELD_PTRS.with(|ptrs| {
        let ptrs = ptrs.borrow();
        assert!(
            !ptrs.contains(&mutex_ptr),
            "DFMutex: reentrant lock detected — \
            the same mutex is being acquired twice on the same thread. \
            std::sync::Mutex is not reentrant; this would deadlock.",
        );
    });

    HELD.with(|held| {
        let mut held = held.borrow_mut();

        // Locks in the same group are peers — no ordering constraint between them.
        let other_held: Vec<GroupId> = held.iter().copied().filter(|&g| g != group_id).collect();

        if !other_held.is_empty() {
            // Recover from a poisoned mutex — the graph state is still consistent because
            // check_and_register_acquire panics before adding any edges on a cycle detection.
            let mut graph = global_graph()
                .lock()
                .unwrap_or_else(|p| p.into_inner());

            assert!(
                !graph.would_create_cycle(&other_held, group_id),
                "DFMutex: potential deadlock detected — acquiring lock group {} \
                while holding {:?} would create a cycle. \
                Ensure lock acquisition order is consistent, or use LockGroup for peer locks.",
                group_id,
                other_held,
            );

            for &h in &other_held {
                graph.add_edge(h, group_id);
            }
        }

        held.push(group_id);
    });

    HELD_PTRS.with(|ptrs| ptrs.borrow_mut().push(mutex_ptr));
}

/// Called when a lock guard is dropped.
pub fn register_release(group_id: GroupId, mutex_ptr: usize) {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        // Remove the most recent occurrence (handles same-group peer locks).
        if let Some(pos) = held.iter().rposition(|&g| g == group_id) {
            held.remove(pos);
        }
    });
    HELD_PTRS.with(|ptrs| {
        let mut ptrs = ptrs.borrow_mut();
        if let Some(pos) = ptrs.iter().rposition(|&p| p == mutex_ptr) {
            ptrs.remove(pos);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn fresh_graph_with_chain(len: usize) -> LockOrderGraph {
        let mut g = LockOrderGraph::default();
        for i in 0..len.saturating_sub(1) {
            g.add_edge(i, i + 1);
        }
        g
    }

    proptest! {
        // For every n, add all forward edges (from < to) in order.
        // A topologically-ordered set of forward edges can never form a cycle.
        #[test]
        fn forward_edges_never_trigger_cycle(n in 2usize..9) {
            let mut graph = LockOrderGraph::default();
            for from in 0..n {
                for to in (from + 1)..n {
                    prop_assert!(
                        !graph.would_create_cycle(&[from], to),
                        "False positive: forward edge {}→{} flagged as cycle",
                        from, to
                    );
                    graph.add_edge(from, to);
                }
            }
        }

        // Build a chain 0→1→…→n-1. Every backwards edge must be detected as a cycle.
        #[test]
        fn back_edge_always_triggers_cycle(n in 2usize..9) {
            let graph = fresh_graph_with_chain(n);
            for a in 1..n {
                for b in 0..a {
                    prop_assert!(
                        graph.would_create_cycle(&[a], b),
                        "Missed cycle: back edge {}→{} not detected in chain of {}",
                        a, b, n
                    );
                }
            }
        }
    }

    #[test]
    fn simple_chain_no_cycle() {
        let graph = fresh_graph_with_chain(3); // 0→1→2
        assert!(!graph.would_create_cycle(&[0], 2));
        assert!(!graph.would_create_cycle(&[1], 2));
    }

    #[test]
    fn simple_back_edge_cycle() {
        let graph = fresh_graph_with_chain(3); // 0→1→2
        assert!(graph.would_create_cycle(&[2], 0));
        assert!(graph.would_create_cycle(&[1], 0));
        assert!(graph.would_create_cycle(&[2], 1));
    }

    #[test]
    fn diamond_dag_no_cycle() {
        let mut graph = LockOrderGraph::default();
        graph.add_edge(0, 1);
        graph.add_edge(0, 2);
        graph.add_edge(1, 3);
        graph.add_edge(2, 3);
        assert!(!graph.would_create_cycle(&[0], 3));
        assert!(!graph.would_create_cycle(&[1], 3));
        assert!(!graph.would_create_cycle(&[2], 3));
        assert!(graph.would_create_cycle(&[3], 0));
    }

    #[test]
    fn empty_held_never_cycles() {
        let graph = fresh_graph_with_chain(5);
        for i in 0..5 {
            assert!(!graph.would_create_cycle(&[], i));
        }
    }
}
