use axum::{extract::State, routing::get, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Track info from Spotify
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art: Option<String>,
    pub duration: u64,
}

// Single lyric line
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub text: String, // Original text
    #[serde(default)]
    pub pron_text: Option<String>, // Phonetic/romanized text
    #[serde(default)]
    pub trans_text: Option<String>, // Translation text
}

// Full lyrics data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsData {
    pub track: TrackInfo,
    pub lyrics: Vec<LyricLine>,
    pub is_synced: bool,
}

// Progress sync data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressData {
    pub position: u64,
    pub is_playing: bool,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub next_track: Option<NextTrackInfo>,
}

// Next track info for preview
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextTrackInfo {
    pub title: String,
    pub artist: String,
    pub album_art: Option<String>,
}

pub struct LyricsServer {
    coordinator: LyricsCoordinator,
}

impl LyricsServer {
    pub fn new(
        progress: Arc<Mutex<Option<ProgressData>>>,
        lyrics: Arc<Mutex<Option<LyricsData>>>,
    ) -> Self {
        Self {
            coordinator: LyricsCoordinator::new(progress, lyrics),
        }
    }

    pub fn get_router(self) -> Router {
        let coordinator = Arc::new(self.coordinator);

        Router::new()
            .route("/progress", post(handle_progress).get(handle_get_progress))
            .route("/lyrics", post(handle_lyrics).get(handle_get_lyrics))
            .route("/lyrics/now", get(handle_get_now))
            .route("/lyrics/health", get(health_check))
            .with_state(coordinator)
    }
}

// HTTP endpoint handlers
async fn handle_lyrics(
    State(coordinator): State<Arc<LyricsCoordinator>>,
    Json(lyrics_data): Json<LyricsData>,
) -> &'static str {
    // Store in state
    if let Ok(mut lock) = coordinator.lyrics.lock() {
        *lock = Some(lyrics_data.clone());
    }
    "OK"
}

async fn handle_progress(
    State(coordinator): State<Arc<LyricsCoordinator>>,
    Json(progress_data): Json<ProgressData>,
) -> &'static str {
    // Store in state
    if let Ok(mut lock) = coordinator.progress.lock() {
        *lock = Some(progress_data.clone());
    }
    "OK"
}

async fn handle_get_lyrics(
    State(coordinator): State<Arc<LyricsCoordinator>>,
) -> Json<Option<LyricsData>> {
    let lyrics_data = if let Ok(lock) = coordinator.lyrics.lock() {
        lock.clone()
    } else {
        None
    };
    Json(lyrics_data)
}

async fn handle_get_progress(
    State(coordinator): State<Arc<LyricsCoordinator>>,
) -> Json<Option<ProgressData>> {
    let progress_data = if let Ok(lock) = coordinator.progress.lock() {
        lock.clone()
    } else {
        None
    };
    Json(progress_data)
}

static mut current_lyric: LyricLine = LyricLine {
    start_time: 0,
    end_time: None,
    text: String::new(),
    pron_text: None,
    trans_text: None,
};
async fn handle_get_now(
    State(coordinator): State<Arc<LyricsCoordinator>>,
) -> Json<Option<LyricLine>> {
    let lyrics_data = if let Ok(lock) = coordinator.lyrics.lock() {
        lock.clone()
    } else {
        None
    };
    let progress_data = if let Ok(lock) = coordinator.progress.lock() {
        lock.clone()
    } else {
        None
    };

    if let Some(lyrics_data) = lyrics_data {
        if let Some(progress_data) = progress_data {
            let current_time = progress_data.position as i64;

            for lyric in lyrics_data.lyrics.iter() {
                let start_time = lyric.start_time as i64;
                let end_time = lyric.end_time.unwrap_or(lyric.start_time) as i64;
                if start_time <= current_time && current_time <= end_time {
                    unsafe {
                        current_lyric = lyric.clone();
                    }
                    break;
                }
                if start_time > current_time {
                    break;
                }
            }
        }
    }
    unsafe { Json(Some(current_lyric.clone())) }
}

pub struct LyricsCoordinator {
    lyrics: Arc<Mutex<Option<LyricsData>>>,
    progress: Arc<Mutex<Option<ProgressData>>>,
}

impl LyricsCoordinator {
    pub fn new(
        progress: Arc<Mutex<Option<ProgressData>>>,
        lyrics: Arc<Mutex<Option<LyricsData>>>,
    ) -> Self {
        Self { lyrics, progress }
    }
}

async fn health_check() -> &'static str {
    "Lyrics Server OK"
}
