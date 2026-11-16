pub mod btree_map;
pub mod binary_heap;
pub mod btree_set;
pub mod vec_deque;

pub use binary_heap::BinaryHeap;
pub use btree_map::BTreeMap;
pub use btree_set::BTreeSet;
pub use vec_deque::VecDeque;

/// The error of a fallible reservation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TryReserveError {
    kind: TryReserveErrorKind,
}
#[derive(Clone, PartialEq, Eq, Debug)]
enum TryReserveErrorKind {
    CapacityOverflow,
}
impl TryReserveError {
    pub(crate) fn capacity_overflow() -> Self { TryReserveError { kind: TryReserveErrorKind::CapacityOverflow } }
}
impl core::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("memory allocation failed")?;
        match self.kind {
            TryReserveErrorKind::CapacityOverflow => f.write_str(" because the computed capacity exceeded the collection's maximum"),
        }
    }
}
