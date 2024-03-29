use std::{sync::Arc, thread};

use audio::AudioEvent;
use colored::Colorize;
use db::Database;
use events::{Bus, Channel, Emitter};
use ingest::IngestionEvent;
use log::{error, info};
use queue::QueueEvent;
use rooms::RoomEvent;
use server::sse::SseManager;
use store::Store;
use thiserror::Error;
use tokio::runtime::{self, Runtime};

use crate::logging::{EventLogger, LogColor};

mod audio;
mod auth;
mod db;
mod events;
mod http;
mod ingest;
mod logging;
mod queue;
mod rooms;
mod server;
mod store;
mod track;
mod util;

pub struct Vinyl {
    db: Arc<Database>,
    store: Arc<Store>,
    event_bus: Arc<EventBus>,
    sse: Arc<SseManager>,
    runtime: Runtime,
}

#[derive(Debug, Clone)]
pub enum VinylEvent {
    Room(RoomEvent),
    Audio(AudioEvent),
    Queue(QueueEvent),
    Ingestion(IngestionEvent),
}

pub type EventEmitter = Emitter<Channel<VinylEvent>, VinylEvent>;
pub type EventBus = Bus<Channel<VinylEvent>, VinylEvent>;

#[derive(Clone)]
pub struct VinylContext {
    pub db: Arc<Database>,
    pub store: Arc<Store>,
    pub sse: Arc<SseManager>,
}

#[derive(Debug, Error)]
enum VinylError {
    #[error("Could not initialize database: {0}")]
    Database(#[from] surrealdb::Error),

    #[error("Fatal error: {0}")]
    Fatal(String),
}

impl Vinyl {
    fn new() -> Result<Self, VinylError> {
        info!("Building async runtime...");
        let main_runtime = runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("vinyl-async")
            .build()
            .map_err(|e| VinylError::Fatal(e.to_string()))?;

        info!("Connecting to database...");

        let channel = Channel::new();
        let event_bus = EventBus::new(channel);

        let store = Store::new(event_bus.emitter());
        let sse = SseManager::new(Arc::downgrade(&store));

        let database = main_runtime.block_on(db::connect())?;

        event_bus.register(EventLogger);
        event_bus.register(store.queue_store.handler());
        event_bus.register(sse.handler());

        main_runtime
            .block_on(store.room_store.init(&database))
            .map_err(|e| VinylError::Fatal(e.to_string()))?;

        Ok(Self {
            sse,
            store,
            event_bus,
            db: database.into(),
            runtime: main_runtime,
        })
    }

    fn run(&self) {
        audio::run_playback(self.store.playback.clone());
        ingest::run_ingestion(self.store.ingestion.clone());

        let event_bus = self.event_bus.clone();
        thread::spawn(move || loop {
            event_bus.tick()
        });

        self.runtime
            .block_on(async move { server::run_server(self.context()).await });
    }

    fn context(&self) -> VinylContext {
        VinylContext {
            db: self.db.clone(),
            sse: self.sse.clone(),
            store: self.store.clone(),
        }
    }
}

impl VinylError {
    fn hint(&self) -> String {
        match self {
            VinylError::Database(_) => "This is a database error. Make sure the SurrealDB instance is properly installed and running, then try again.".to_string(),
            VinylError::Fatal(_) => "This error is fatal, and should not happen.".to_string(),
        }
    }
}

fn main() {
    logging::init_logger();

    match Vinyl::new() {
        Ok(vinyl) => {
            info!("Initialized successfully.");
            vinyl.run();
        }
        Err(error) => {
            error!("{} Read the error below to troubleshoot the issue. If you think this might be a bug, please report it by making a GitHub issue.", "Vinyl failed to start!".bold().color(LogColor::Red));
            error!("{}", error);
            error!(
                "{}",
                format!("Hint: {}", error.hint())
                    .color(LogColor::Dimmed)
                    .italic()
            );
        }
    }
}
