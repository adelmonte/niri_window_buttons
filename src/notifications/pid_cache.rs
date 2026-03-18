use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, SystemTime},
};
use async_channel::{Receiver, Sender};
use futures::{FutureExt, StreamExt, TryStreamExt, channel::oneshot};
use waybar_cffi::gtk::glib;
use zbus::{
    Connection, MatchRule, MessageStream,
    fdo::{DBusProxy, MonitoringProxy, NameOwnerChanged},
    message::Type,
    names::UniqueName,
};

#[derive(Debug, Clone)]
pub struct PidCache {
    request_tx: Sender<CacheRequest>,
}

impl PidCache {
    pub fn create(ttl: Duration) -> Self {
        let (tx, rx) = async_channel::unbounded();
        glib::spawn_future_local(async move {
            if let Err(e) = cache_worker(rx, ttl).await {
                tracing::error!(%e, "PID cache worker failed");
            }
        });

        Self { request_tx: tx }
    }

    #[tracing::instrument(level = "TRACE", skip(self))]
    pub async fn query(&self, connection: impl ToString + Debug) -> Option<u32> {
        let (result_tx, result_rx) = oneshot::channel();
        if let Err(e) = self
            .request_tx
            .send(CacheRequest::Query {
                connection: connection.to_string(),
                response: result_tx,
            })
            .await
        {
            tracing::error!(%e, "cache request send failed");
            return None;
        }

        result_rx.await.unwrap_or(None)
    }
}

#[derive(Debug)]
enum CacheRequest {
    Query {
        connection: String,
        response: oneshot::Sender<Option<u32>>,
    },
}

#[derive(Debug)]
struct CacheEntry {
    pid: Option<u32>,
    expires_at: SystemTime,
}

const DBUS_SYSTEM_INTERFACE: &str = "org.freedesktop.DBus";

async fn cache_worker(rx: Receiver<CacheRequest>, ttl: Duration) -> anyhow::Result<()> {
    let mut storage = CacheStorage::new(ttl);

    let dbus_connection = Connection::session().await?;
    let dbus_api = DBusProxy::new(&dbus_connection).await?;

    let monitor_connection = Connection::session().await?;
    let monitor_api = MonitoringProxy::new(&monitor_connection).await?;
    monitor_api
        .become_monitor(
            &[MatchRule::builder()
                .msg_type(Type::Signal)
                .interface(DBUS_SYSTEM_INTERFACE)?
                .member("NameOwnerChanged")?
                .build()],
            0,
        )
        .await?;

    let mut cleanup_timer = glib::interval_stream(Duration::from_secs(60)).fuse();
    let mut event_stream = MessageStream::from(monitor_connection);

    loop {
        futures::select! {
            result = event_stream.try_next() => {
                match result {
                    Ok(Some(msg)) => {
                        process_dbus_event(&mut storage, &dbus_api, msg).await;
                    }
                    Ok(None) => {
                        tracing::error!("D-Bus event stream closed");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(%e, "D-Bus event stream error");
                        anyhow::bail!(e);
                    }
                }
            }
            result = rx.recv().fuse() => {
                match result {
                    Ok(request) => {
                        handle_cache_request(&mut storage, &dbus_api, request).await;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            _ = cleanup_timer.next() => {
                storage.remove_expired(SystemTime::now());
            }
        }
    }

    Ok(())
}

async fn process_dbus_event(
    storage: &mut CacheStorage,
    dbus_api: &DBusProxy<'_>,
    message: zbus::Message,
) {
    if let Some(change_event) = NameOwnerChanged::from_message(message) {
        if let Ok(args) = change_event.args() {
            if let Some(new_connection) = args.new_owner().as_ref() {
                if let Ok(pid) = dbus_api.get_connection_unix_process_id(new_connection.clone().into()).await {
                    storage.store(new_connection, Some(pid));
                }
            } else if let Some(old_connection) = args.old_owner.as_ref() {
                storage.evict(old_connection);
            }
        }
    }
}

async fn handle_cache_request(
    storage: &mut CacheStorage,
    dbus_api: &DBusProxy<'_>,
    request: CacheRequest,
) {
    match request {
        CacheRequest::Query { connection, response } => {
            if let Some(cached_pid) = storage.retrieve(&connection) {
                let _ = response.send(cached_pid);
            } else if let Ok(unique_name) = UniqueName::try_from(connection.as_str()) {
                if let Ok(pid) = dbus_api.get_connection_unix_process_id(unique_name.into()).await {
                    storage.store(connection, Some(pid));
                    let _ = response.send(Some(pid));
                }
            }
        }
    }
}

#[derive(Debug)]
struct CacheStorage {
    entries: HashMap<String, CacheEntry>,
    ttl: Duration,
}

impl CacheStorage {
    fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    fn remove_expired(&mut self, current_time: SystemTime) {
        self.entries.retain(|_, entry| entry.expires_at > current_time);
    }

    fn retrieve(&mut self, connection: &str) -> Option<Option<u32>> {
        self.entries.get_mut(connection).map(|entry| {
            entry.expires_at = SystemTime::now() + self.ttl;
            entry.pid
        })
    }

    fn store(&mut self, connection: impl ToString, pid: Option<u32>) {
        self.entries.insert(
            connection.to_string(),
            CacheEntry {
                pid,
                expires_at: SystemTime::now() + self.ttl,
            },
        );
    }

    fn evict(&mut self, connection: &str) {
        self.entries.remove(connection);
    }
}
