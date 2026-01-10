use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::services::ServeDir;

use crate::ytdlp::{DownloadProgress, DownloadStatus, YtDlpManager};

/// 비디오 API 서버
pub struct VideoServer {
    coordinator: DownloadCoordinator,
}

impl VideoServer {
    pub fn new(ytdlp: YtDlpManager) -> Self {
        Self {
            coordinator: DownloadCoordinator::new(ytdlp),
        }
    }

    /// Router 반환
    pub fn get_router(self) -> Router {
        let videos_dir = self.coordinator.ytdlp.videos_dir();

        let coordinator = Arc::new(self.coordinator);

        Router::new()
            .route("/video/request", get(handle_video_request))
            .route("/video/status", get(handle_video_status))
            .route("/health", get(health_check))
            // 정적 파일 서빙 (다운로드된 비디오)
            .nest_service("/video/files", ServeDir::new(videos_dir))
            .with_state(coordinator)
    }
}

/// 쿼리 파라미터
#[derive(serde::Deserialize)]
struct VideoQuery {
    id: String,
}

/// 비디오 응답
#[derive(serde::Serialize)]
struct VideoResponse {
    success: bool,
    video_id: String,
    url: Option<String>,
    message: Option<String>,
}

/// 헬스 체크
async fn health_check() -> &'static str {
    "OK"
}

/// 비디오 다운로드 및 URL 반환 엔드포인트
/// GET /video/request?id=<youtube_id>
///
/// 이미 존재하면 즉시 URL 반환
/// 없으면 다운로드 시작하고 SSE로 진행상황 스트리밍
async fn handle_video_request(
    State(coordinator): State<Arc<DownloadCoordinator>>,
    Query(query): Query<VideoQuery>,
) -> Response {
    let video_id = query.id.trim();
    let ytdlp = &coordinator.ytdlp;

    // 유효성 검사
    if video_id.is_empty() || video_id.len() > 20 {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(VideoResponse {
                success: false,
                video_id: video_id.to_string(),
                url: None,
                message: Some("Invalid video ID".to_string()),
            }),
        )
            .into_response();
    }

    // 이미 존재하는 경우 바로 응답
    if ytdlp.video_exists(video_id) {
        let video_path = ytdlp.video_path(video_id);
        let default_name = format!("{}.webm", video_id);
        let file_name = video_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or(default_name);

        return axum::Json(VideoResponse {
            success: true,
            video_id: video_id.to_string(),
            url: Some(format!("http://localhost:15123/video/files/{}", file_name)),
            message: Some("Video already available".to_string()),
        })
        .into_response();
    }

    // 진행 중 다운로드가 있으면 합류, 없으면 새 다운로드 시작
    let progress_rx = coordinator.start_or_subscribe(video_id).await;

    // SSE 스트림 생성
    let stream = create_progress_stream(progress_rx);

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

/// 비디오 상태 확인 엔드포인트 (SSE 없이 단순 조회)
/// GET /video/status?id=<youtube_id>
async fn handle_video_status(
    State(coordinator): State<Arc<DownloadCoordinator>>,
    Query(query): Query<VideoQuery>,
) -> axum::Json<VideoResponse> {
    let video_id = query.id.trim();
    let ytdlp = &coordinator.ytdlp;

    if ytdlp.video_exists(video_id) {
        let video_path = ytdlp.video_path(video_id);
        let default_name = format!("{}.webm", video_id);
        let file_name = video_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or(default_name);

        axum::Json(VideoResponse {
            success: true,
            video_id: video_id.to_string(),
            url: Some(format!("http://localhost:15123/video/files/{}", file_name)),
            message: Some("Video available".to_string()),
        })
    } else {
        axum::Json(VideoResponse {
            success: false,
            video_id: video_id.to_string(),
            url: None,
            message: Some("Video not downloaded".to_string()),
        })
    }
}

/// broadcast 수신기를 SSE 스트림으로 변환
fn create_progress_stream(
    rx: broadcast::Receiver<DownloadProgress>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let stream = BroadcastStream::new(rx);

    stream.filter_map(|result| match result {
        Ok(progress) => {
            let is_final = progress.status == DownloadStatus::Completed
                || progress.status == DownloadStatus::Error
                || progress.status == DownloadStatus::AlreadyExists;

            let event_data = serde_json::to_string(&progress).unwrap_or_default();
            let event = Event::default().data(event_data).event(if is_final {
                "complete"
            } else {
                "progress"
            });

            Some(Ok(event))
        }
        Err(_) => None,
    })
}

/// 진행 중 다운로드를 공유하기 위한 코디네이터
pub struct DownloadCoordinator {
    ytdlp: YtDlpManager,
    in_progress: Arc<Mutex<HashMap<String, broadcast::Sender<DownloadProgress>>>>,
}

impl DownloadCoordinator {
    pub fn new(ytdlp: YtDlpManager) -> Self {
        Self {
            ytdlp,
            in_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 이미 진행 중이면 기존 SSE 스트림에 합류하고, 아니면 새 다운로드를 시작
    pub async fn start_or_subscribe(
        &self,
        video_id: &str,
    ) -> broadcast::Receiver<DownloadProgress> {
        // 이미 진행 중인 다운로드가 있으면 해당 채널에 합류
        if let Some(sender) = self.in_progress.lock().await.get(video_id) {
            return sender.subscribe();
        }

        // 새 다운로드 채널 생성
        let (tx, rx) = broadcast::channel::<DownloadProgress>(100);
        self.in_progress
            .lock()
            .await
            .insert(video_id.to_string(), tx.clone());

        // 다운로드 작업 시작
        let video_id_owned = video_id.to_string();
        let ytdlp = self.ytdlp.clone();
        let in_progress = self.in_progress.clone();
        tokio::spawn(async move {
            let result = ytdlp.download_video(&video_id_owned, tx.clone()).await;

            if let Err(e) = result {
                let _ = tx.send(DownloadProgress {
                    video_id: video_id_owned.clone(),
                    status: DownloadStatus::Error,
                    percent: None,
                    speed: None,
                    eta: None,
                    message: Some(e.to_string()),
                });
            }

            // 다운로드가 끝났으니 in-progress 목록에서 제거
            in_progress.lock().await.remove(&video_id_owned);
        });

        rx
    }
}
