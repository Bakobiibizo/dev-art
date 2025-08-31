//! Background poller for a local static directory.
//!
//! Intended as a scaffold for reacting to newly created files under
//! `STATIC_DRIVE_PATH`. Currently it iterates directory entries on an
//! interval but performs no side effects.
use tokio::time::{self, Duration};
use std::path::Path;
use tokio::fs;

pub struct StaticDrivePoller {
    path: String,
}

impl StaticDrivePoller {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub async fn start_polling(&self) {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            self.poll_drive().await;
        }
    }

    async fn poll_drive(&self) {
        let path = Path::new(&self.path);
        if let Ok(entries) = fs::read_dir(path).await {
            // Process new files here
            // You might want to move processed files to a different directory
            // or update a database with the new file information
        }
    }
}
