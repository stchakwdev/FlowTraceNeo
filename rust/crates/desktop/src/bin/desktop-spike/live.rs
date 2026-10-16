//! The SpacetimeDB side of the spike: connect, subscribe to `spike_row`, and forward
//! every row event and connection change to the window over one channel.
//!
//! SDK calls follow <https://spacetimedb.com/docs/clients/rust>: the builder, the row
//! callbacks (which must be `Send + 'static`, so they only send on the channel), and the
//! two drive modes the spike compares.

use std::sync::Arc;

use async_channel::{Receiver, Sender};
use spacetimedb_sdk::{DbContext as _, Table as _, TableWithPrimaryKey as _};

use crate::module_bindings::{DbConnection, SpikeRow, SpikeRowTableAccess as _};
use crate::rows::{Keyed, RowEvent};

impl Keyed for SpikeRow {
    type Key = u32;

    fn key(&self) -> u32 {
        self.id
    }
}

/// How the SDK connection is advanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// `run_threaded`: the SDK's own thread blocks on the socket and runs the callbacks;
    /// the window wakes only when a message arrives on the channel.
    Threaded,
    /// `frame_tick` from a GPUI timer on the main thread, every [`TICK`].
    Tick,
}

/// The `frame_tick` period in [`Drive::Tick`] mode, one 60 Hz frame.
pub const TICK: std::time::Duration = std::time::Duration::from_millis(16);

/// What the connection tells the window, in delivery order.
#[derive(Debug)]
pub enum Live {
    /// The socket is open.
    Connected,
    /// A row callback fired.
    Row(RowEvent<SpikeRow>),
    /// The initial subscription is in the client cache; later rows are live changes.
    Applied,
    /// A connect or subscription error.
    Failed(String),
    /// The socket closed, with the error if it was not a clean disconnect.
    Disconnected(Option<String>),
}

/// The query the spike subscribes to.
pub const QUERY: &str = "SELECT * FROM spike_row";

/// Opens the connection and registers the subscription and row callbacks. Nothing is
/// delivered until the connection is driven, by [`DbConnection::run_threaded`] or by
/// calling [`DbConnection::frame_tick`].
pub fn connect(uri: &str, database: &str) -> anyhow::Result<(Arc<DbConnection>, Receiver<Live>)> {
    let (tx, rx) = async_channel::unbounded();
    let on_connect = tx.clone();
    let on_connect_error = tx.clone();
    let on_disconnect = tx.clone();
    let conn = DbConnection::builder()
        .with_uri(uri)
        .with_database_name(database)
        .on_connect(move |conn, _identity, _token| {
            send(&on_connect, Live::Connected);
            let applied = on_connect.clone();
            let failed = on_connect.clone();
            conn.subscription_builder()
                .on_applied(move |_| send(&applied, Live::Applied))
                .on_error(move |_, err| send(&failed, Live::Failed(err.to_string())))
                .subscribe(QUERY);
        })
        .on_connect_error(move |_, err| send(&on_connect_error, Live::Failed(err.to_string())))
        .on_disconnect(move |_, err| {
            send(
                &on_disconnect,
                Live::Disconnected(err.map(|e| e.to_string())),
            );
        })
        .build()?;

    let rows = conn.db.spike_row();
    let insert = tx.clone();
    rows.on_insert(move |_, row| send(&insert, Live::Row(RowEvent::Insert(row.clone()))));
    let update = tx.clone();
    rows.on_update(move |_, old, new| {
        send(
            &update,
            Live::Row(RowEvent::Update {
                old: old.clone(),
                new: new.clone(),
            }),
        );
    });
    rows.on_delete(move |_, row| send(&tx, Live::Row(RowEvent::Delete(row.clone()))));

    Ok((Arc::new(conn), rx))
}

/// The receiver goes away only when the window closes, and then nothing is left to
/// tell, so a failed send is dropped.
fn send(tx: &Sender<Live>, message: Live) {
    let _ = tx.try_send(message);
}
