//! The rows the spike's `DataTable` renders: one row per key, in ascending key order,
//! changed only by batches of table events from the SpacetimeDB subscription.
//!
//! Pure logic with no GPUI or SDK types, so it is the part of the spike under
//! `cargo test`. The event shapes mirror the SDK row callbacks `on_insert`,
//! `on_update` and `on_delete` (<https://spacetimedb.com/docs/clients/rust>).

use std::collections::BTreeMap;

/// Batches at least this long are applied through a sorted map and collected back,
/// `O(n log n)` for the whole batch; shorter ones edit the vector in place by binary
/// search, `O(log n)` per update and one shift per insert or delete. The initial
/// subscription arrives as one batch of inserts in no promised order, which would
/// cost a shift per row on the in-place path.
const BULK_MIN: usize = 32;

/// A row with a primary key.
pub trait Keyed {
    /// The key type; rows are kept in ascending key order.
    type Key: Ord + Copy;

    /// This row's key.
    fn key(&self) -> Self::Key;
}

/// One table event, in the order the SDK delivered it.
#[derive(Debug, Clone, PartialEq)]
pub enum RowEvent<R> {
    /// A row entered the subscription (initial rows or a later insert).
    Insert(R),
    /// A row with a primary key changed.
    Update {
        /// The row before the change.
        old: R,
        /// The row after the change.
        new: R,
    },
    /// A row left the subscription.
    Delete(R),
}

/// One row per key, ascending by key.
#[derive(Debug, Clone, PartialEq)]
pub struct RowStore<R> {
    rows: Vec<R>,
}

impl<R> Default for RowStore<R> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<R: Keyed> RowStore<R> {
    /// The number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// The row at `index` in key order.
    pub fn get(&self, index: usize) -> Option<&R> {
        self.rows.get(index)
    }

    /// All rows in key order.
    pub fn rows(&self) -> &[R] {
        &self.rows
    }

    /// Applies `batch` in order. An insert of a present key replaces the row, an
    /// update of an absent row inserts it, and a delete of an absent key is ignored,
    /// so a replayed or reordered event cannot duplicate a row.
    pub fn apply(&mut self, batch: Vec<RowEvent<R>>) {
        if batch.len() >= BULK_MIN {
            self.apply_bulk(batch);
        } else {
            self.apply_in_place(batch);
        }
    }

    fn apply_in_place(&mut self, batch: Vec<RowEvent<R>>) {
        for event in batch {
            match event {
                RowEvent::Insert(row) => self.put(row),
                RowEvent::Update { old, new } => {
                    if old.key() != new.key() {
                        self.remove(old.key());
                    }
                    self.put(new);
                }
                RowEvent::Delete(row) => self.remove(row.key()),
            }
        }
    }

    fn apply_bulk(&mut self, batch: Vec<RowEvent<R>>) {
        let mut map: BTreeMap<R::Key, R> = self.rows.drain(..).map(|r| (r.key(), r)).collect();
        for event in batch {
            match event {
                RowEvent::Insert(row) => {
                    map.insert(row.key(), row);
                }
                RowEvent::Update { old, new } => {
                    map.remove(&old.key());
                    map.insert(new.key(), new);
                }
                RowEvent::Delete(row) => {
                    map.remove(&row.key());
                }
            }
        }
        self.rows = map.into_values().collect();
    }

    fn put(&mut self, row: R) {
        let key = row.key();
        match self.rows.binary_search_by(|r| r.key().cmp(&key)) {
            Ok(index) => self.rows[index] = row,
            Err(index) => self.rows.insert(index, row),
        }
    }

    fn remove(&mut self, key: R::Key) {
        if let Ok(index) = self.rows.binary_search_by(|r| r.key().cmp(&key)) {
            self.rows.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::{BULK_MIN, Keyed, RowEvent, RowStore};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq)]
    struct Row {
        id: u32,
        label: String,
    }

    impl Keyed for Row {
        type Key = u32;

        fn key(&self) -> u32 {
            self.id
        }
    }

    fn row(id: u32, label: &str) -> Row {
        Row {
            id,
            label: label.to_owned(),
        }
    }

    fn ids(store: &RowStore<Row>) -> Vec<u32> {
        store.rows().iter().map(|r| r.id).collect()
    }

    fn store_of(rows: &[Row]) -> RowStore<Row> {
        let mut store = RowStore::default();
        store.apply(rows.iter().cloned().map(RowEvent::Insert).collect());
        store
    }

    #[test]
    fn inserts_are_kept_in_key_order() {
        let store = store_of(&[row(3, "c"), row(1, "a"), row(2, "b")]);
        assert_eq!(ids(&store), [1, 2, 3]);
        assert_eq!(store.get(0).unwrap().label, "a");
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn update_replaces_the_row_in_place() {
        let mut store = store_of(&[row(1, "a"), row(2, "b"), row(3, "c")]);
        store.apply(vec![RowEvent::Update {
            old: row(2, "b"),
            new: row(2, "edited"),
        }]);
        assert_eq!(ids(&store), [1, 2, 3]);
        assert_eq!(store.get(1).unwrap().label, "edited");
    }

    #[test]
    fn delete_removes_the_row_and_ignores_absent_keys() {
        let mut store = store_of(&[row(1, "a"), row(2, "b")]);
        store.apply(vec![
            RowEvent::Delete(row(1, "a")),
            RowEvent::Delete(row(9, "x")),
        ]);
        assert_eq!(ids(&store), [2]);
    }

    #[test]
    fn insert_of_a_present_key_replaces_instead_of_duplicating() {
        let mut store = store_of(&[row(1, "a")]);
        store.apply(vec![RowEvent::Insert(row(1, "again"))]);
        assert_eq!(ids(&store), [1]);
        assert_eq!(store.get(0).unwrap().label, "again");
    }

    #[test]
    fn update_of_an_absent_row_inserts_it() {
        let mut store = store_of(&[row(1, "a"), row(3, "c")]);
        store.apply(vec![RowEvent::Update {
            old: row(2, "b"),
            new: row(2, "b2"),
        }]);
        assert_eq!(ids(&store), [1, 2, 3]);
    }

    #[test]
    fn update_that_changes_the_key_moves_the_row() {
        let mut store = store_of(&[row(1, "a"), row(2, "b")]);
        store.apply(vec![RowEvent::Update {
            old: row(1, "a"),
            new: row(5, "a"),
        }]);
        assert_eq!(ids(&store), [2, 5]);
    }

    #[test]
    fn events_apply_in_batch_order() {
        let mut store = RowStore::default();
        store.apply(vec![
            RowEvent::Insert(row(5, "a")),
            RowEvent::Delete(row(5, "a")),
            RowEvent::Insert(row(6, "b")),
            RowEvent::Delete(row(7, "c")),
            RowEvent::Insert(row(7, "c")),
        ]);
        assert_eq!(ids(&store), [6, 7]);
    }

    #[test]
    fn ten_thousand_rows_load_in_one_batch_then_take_live_updates() {
        let mut store = RowStore::default();
        store.apply(
            (1..=10_000)
                .rev()
                .map(|id| RowEvent::Insert(row(id, "seed")))
                .collect(),
        );
        assert_eq!(store.len(), 10_000);
        assert_eq!(store.get(0).unwrap().id, 1);
        assert_eq!(store.get(9_999).unwrap().id, 10_000);
        assert!(store.rows().windows(2).all(|w| w[0].id < w[1].id));

        store.apply(vec![RowEvent::Update {
            old: row(42, "seed"),
            new: row(42, "live"),
        }]);
        assert_eq!(store.get(41).unwrap().label, "live");
        assert_eq!(store.rows().iter().filter(|r| r.label == "live").count(), 1);
    }

    /// A deterministic mix of inserts, updates and deletes over a small key space,
    /// so the same key is hit many times in one batch.
    fn mixed_events(count: usize) -> Vec<RowEvent<Row>> {
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        (0..count)
            .map(|i| {
                let id = u32::try_from(next() % 300).unwrap();
                let label = format!("v{i}");
                match next() % 3 {
                    0 => RowEvent::Insert(row(id, &label)),
                    1 => RowEvent::Update {
                        old: row(id, "old"),
                        new: row(id, &label),
                    },
                    _ => RowEvent::Delete(row(id, "old")),
                }
            })
            .collect()
    }

    /// The model the store must match: last write per key wins, deletes remove.
    fn model(events: &[RowEvent<Row>]) -> Vec<Row> {
        let mut map = BTreeMap::new();
        for event in events {
            match event {
                RowEvent::Insert(r) => {
                    map.insert(r.id, r.clone());
                }
                RowEvent::Update { old, new } => {
                    map.remove(&old.id);
                    map.insert(new.id, new.clone());
                }
                RowEvent::Delete(r) => {
                    map.remove(&r.id);
                }
            }
        }
        map.into_values().collect()
    }

    #[test]
    fn in_place_and_bulk_paths_match_the_model() {
        let events = mixed_events(5_000);
        let expected = model(&events);

        let mut one_at_a_time = RowStore::default();
        for event in events.iter().cloned() {
            one_at_a_time.apply(vec![event]);
        }
        assert_eq!(one_at_a_time.rows(), expected.as_slice());

        let mut in_place = RowStore::default();
        in_place.apply_in_place(events.clone());
        assert_eq!(in_place.rows(), expected.as_slice());

        let mut bulk = RowStore::default();
        bulk.apply_bulk(events.clone());
        assert_eq!(bulk.rows(), expected.as_slice());

        let mut chunked = RowStore::default();
        for chunk in events.chunks(BULK_MIN + 7) {
            chunked.apply(chunk.to_vec());
        }
        assert_eq!(chunked.rows(), expected.as_slice());
    }
}
