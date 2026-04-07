use std::sync::Arc;
use async_channel::Sender;
use futures::StreamExt;
use niri_ipc::Workspace;
use waybar_cffi::gtk::glib;
use crate::{
    audio,
    compositor::{self, CompositorClient, WindowSnapshot},
    icons::IconResolver,
    notifications::{self, NotificationData},
    settings::Settings,
};

#[derive(Debug, Clone)]
pub struct SharedState(Arc<StateInner>);

#[derive(Debug)]
struct StateInner {
    settings: Settings,
    icon_resolver: IconResolver,
    compositor: CompositorClient,
}

impl SharedState {
    pub fn create(settings: Settings) -> Self {
        Self(Arc::new(StateInner {
            compositor: CompositorClient::create(settings.clone()),
            icon_resolver: IconResolver::new(),
            settings,
        }))
    }

    pub fn settings(&self) -> &Settings {
        &self.0.settings
    }

    pub fn icon_resolver(&self) -> &IconResolver {
        &self.0.icon_resolver
    }

    pub fn compositor(&self) -> &CompositorClient {
        &self.0.compositor
    }

    pub fn create_event_stream(&self) -> async_channel::Receiver<EventMessage> {
        let (tx, rx) = async_channel::unbounded();

        if self.settings().notifications_enabled() {
            glib::spawn_future_local(forward_notifications(tx.clone()));
        }

        if self.settings().audio_indicator().enabled {
            glib::spawn_future_local(forward_audio_updates(tx.clone()));
        }

        let window_tx = tx.clone();
        let (glib_win_tx, glib_win_rx) = glib::MainContext::channel(glib::Priority::DEFAULT);
        glib_win_rx.attach(None, move |snapshot: WindowSnapshot| {
            if let Err(e) = window_tx.try_send(EventMessage::WindowUpdate(snapshot)) {
                tracing::error!(%e, "failed to forward window update");
            }
            glib::ControlFlow::Continue
        });
        compositor::start_window_stream(glib_win_tx, self.compositor().only_current_workspace());

        let workspace_tx = tx;
        let (glib_ws_tx, glib_ws_rx) = glib::MainContext::channel(glib::Priority::DEFAULT);
        glib_ws_rx.attach(None, move |_: Vec<Workspace>| {
            if let Err(e) = workspace_tx.try_send(EventMessage::Workspaces) {
                tracing::error!(%e, "failed to forward workspace change");
            }
            glib::ControlFlow::Continue
        });
        compositor::start_workspace_stream(glib_ws_tx);

        rx
    }
}

pub enum EventMessage {
    Notification(Box<NotificationData>),
    WindowUpdate(WindowSnapshot),
    Workspaces,
    AudioUpdate(audio::AudioState),
}

async fn forward_audio_updates(tx: Sender<EventMessage>) {
    let mut stream = Box::pin(audio::create_stream());
    while let Some(state) = stream.next().await {
        if let Err(e) = tx.send(EventMessage::AudioUpdate(state)).await {
            tracing::error!(%e, "failed to forward audio update");
        }
    }
}

async fn forward_notifications(tx: Sender<EventMessage>) {
    let mut notification_stream = Box::pin(notifications::create_stream());
    while let Some(notification) = notification_stream.next().await {
        if let Err(e) = tx.send(EventMessage::Notification(Box::new(notification))).await {
            tracing::error!(%e, "failed to forward notification");
        }
    }
}
