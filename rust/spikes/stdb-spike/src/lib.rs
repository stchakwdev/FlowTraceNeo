//! Throwaway SpacetimeDB module for the F3 spike (rust/docs/adr/0002-gpui-spacetime.md):
//! one public table of rows and one reducer that inserts or updates them, so the GPUI
//! spike window has live rows to subscribe to. Not product code; the ledger module is
//! `crates/ledger-module`.
//!
//! Table and reducer syntax: <https://spacetimedb.com/docs/quickstarts/rust/>.
#![expect(
    missing_docs,
    reason = "the spacetimedb 2.12 table and reducer macros emit undocumented public items"
)]

use spacetimedb::{ReducerContext, Table};

/// One row of the spike's `DataTable`.
#[spacetimedb::table(accessor = spike_row, public)]
pub struct SpikeRow {
    /// Row key; the client keeps its rows sorted by it.
    #[primary_key]
    pub id: u32,
    /// Free text shown in the label column.
    pub label: String,
    /// Amount in minor units (cents), shown as major units with two decimals.
    pub amount_minor: i64,
    /// Zero on insert, plus one on every update.
    pub revision: u32,
}

/// Inserts or updates rows `start_id .. start_id + count`.
///
/// Offset `k` in the range gets amount `amount_minor * (k + 1)`. A range of one row
/// takes `label` as given; a longer range suffixes each label with the row id, so
/// `upsert_rows(1, 10000, "Row", 1250)` seeds rows "Row 1" to "Row 10000".
///
/// # Errors
///
/// When the id range passes `u32::MAX` or an amount passes `i64`; the reducer then
/// rolls back and writes nothing.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn upsert_rows(
    ctx: &ReducerContext,
    start_id: u32,
    count: u32,
    label: String,
    amount_minor: i64,
) -> Result<(), String> {
    let end = start_id
        .checked_add(count)
        .ok_or_else(|| format!("id range {start_id} + {count} overflows u32"))?;
    for (offset, id) in (start_id..end).enumerate() {
        let factor = i64::try_from(offset + 1).map_err(|e| e.to_string())?;
        let amount_minor = amount_minor
            .checked_mul(factor)
            .ok_or_else(|| format!("amount for row {id} overflows i64"))?;
        let label = if count == 1 {
            label.clone()
        } else {
            format!("{label} {id}")
        };
        let rows = ctx.db.spike_row();
        match rows.id().find(id) {
            Some(existing) => {
                rows.id().update(SpikeRow {
                    id,
                    label,
                    amount_minor,
                    revision: existing.revision.saturating_add(1),
                });
            }
            None => {
                rows.insert(SpikeRow {
                    id,
                    label,
                    amount_minor,
                    revision: 0,
                });
            }
        }
    }
    Ok(())
}
