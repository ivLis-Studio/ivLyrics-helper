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
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::ytdlp::{DownloadProgress, DownloadStatus, YtDlpManager};

/// 비디오 API 서버
pub struct VideoServer {
    ytdlp: YtDlpManager,
}

impl VideoServer {
    pub fn new(ytdlp: YtDlpManager) -> Self {
        Self { ytdlp }
    }

    /// 서버 시작
    pub async fn start(self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let videos_dir = self.ytdlp.videos_dir();
        
        // CORS 설정 (Spicetify에서 접근 가능하도록)
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let ytdlp = Arc::new(self.ytdlp);

        let app = Router::new()
            .route("/video/request", get(handle_video_request))
            .route("/video/status", get(handle_video_status))
            .route("/health", get(health_check))
            // 정적 파일 서빙 (다운로드된 비디오)
            .nest_service("/video/files", ServeDir::new(videos_dir))
            .layer(cors)
            .with_state(ytdlp);

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        tracing::info!("Video server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
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
    State(ytdlp): State<Arc<YtDlpManager>>,
    Query(query): Query<VideoQuery>,
) -> Response {
    let video_id = query.id.trim();

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

    // SSE 스트림으로 다운로드 진행상황 전송
    let (progress_tx, progress_rx) = broadcast::channel::<DownloadProgress>(100);
    let video_id_owned = video_id.to_string();
    let ytdlp_clone = ytdlp.clone();

    // 다운로드 시작 (백그라운드)
    tokio::spawn(async move {
        match ytdlp_clone.download_video(&video_id_owned, progress_tx.clone()).await {
            Ok(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                let _ = progress_tx.send(DownloadProgress {
                    video_id: video_id_owned,
                    status: DownloadStatus::Completed,
                    percent: Some(100.0),
                    speed: None,
                    eta: None,
                    message: Some(format!("http://localhost:15123/video/files/{}", file_name)),
                });
            }
            Err(e) => {
                let _ = progress_tx.send(DownloadProgress {
                    video_id: video_id_owned,
                    status: DownloadStatus::Error,
                    percent: None,
                    speed: None,
                    eta: None,
                    message: Some(e.to_string()),
                });
            }
        }
    });

    // SSE 스트림 생성
    let stream = create_progress_stream(progress_rx);

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

/// 비디오 상태 확인 엔드포인트 (SSE 없이 단순 조회)
/// GET /video/status?id=<youtube_id>
async fn handle_video_status(
    State(ytdlp): State<Arc<YtDlpManager>>,
    Query(query): Query<VideoQuery>,
) -> axum::Json<VideoResponse> {
    let video_id = query.id.trim();

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

    stream.filter_map(|result| {
        match result {
            Ok(progress) => {
                let is_final = progress.status == DownloadStatus::Completed 
                    || progress.status == DownloadStatus::Error
                    || progress.status == DownloadStatus::AlreadyExists;

                let event_data = serde_json::to_string(&progress).unwrap_or_default();
                let event = Event::default()
                    .data(event_data)
                    .event(if is_final { "complete" } else { "progress" });

                Some(Ok(event))
            }
            Err(_) => None,
        }
    })
}
