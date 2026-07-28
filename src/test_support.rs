use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    pub fn new() -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "codex-context-window-{}-{counter}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temporary directory");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn token_count(used: u64, limit: u64) -> String {
    json!({
        "type": "event_msg",
        "payload": {
            "type": "token_count",
            "info": {
                "last_token_usage": { "total_tokens": used },
                "model_context_window": limit
            }
        }
    })
    .to_string()
}

pub fn task_started(limit: u64) -> String {
    json!({
        "type": "event_msg",
        "payload": {
            "type": "task_started",
            "model_context_window": limit
        }
    })
    .to_string()
}
