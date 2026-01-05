use regex::Regex;
use reqwest::Client;
use std::path::PathBuf;
use std::process::Stdio;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// yt-dlp 다운로드 진행 상황
#[derive(Clone, Debug, serde::Serialize)]
pub struct DownloadProgress {
    pub video_id: String,
    pub status: DownloadStatus,
    pub percent: Option<f32>,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Checking,
    Downloading,
    Processing,
    Completed,
    Error,
    AlreadyExists,
}

/// yt-dlp 관리자
#[derive(Clone)]
pub struct YtDlpManager {
    client: Client,
    data_dir: PathBuf,
    videos_dir: PathBuf,
}

impl YtDlpManager {
    pub fn new(videos_dir: PathBuf) -> Self {
        // macOS: ~/Library/Application Support, Windows: %LOCALAPPDATA%
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ivLyrics-helper");

        Self {
            client: Client::new(),
            data_dir,
            videos_dir,
        }
    }

    /// yt-dlp 실행 파일 경로 (플랫폼별)
    pub fn ytdlp_path(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            self.data_dir.join("yt-dlp.exe")
        } else {
            // macOS, Linux
            self.data_dir.join("yt-dlp")
        }
    }

    /// 현재 플랫폼에 맞는 yt-dlp 바이너리 이름 반환
    fn get_ytdlp_binary_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "yt-dlp_macos"  // ARM Mac (Apple Silicon)
            } else {
                "yt-dlp_macos"  // Intel Mac (same binary, universal)
            }
        } else {
            "yt-dlp"  // Linux
        }
    }

    /// 비디오 저장 디렉토리
    pub fn videos_dir(&self) -> PathBuf {
        self.videos_dir.clone()
    }

    /// 특정 비디오 파일 경로
    pub fn video_path(&self, video_id: &str) -> PathBuf {
        self.videos_dir().join(format!("{}.webm", video_id))
    }

    /// yt-dlp가 존재하는지 확인하고, 없으면 다운로드
    pub async fn ensure_ytdlp(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 디렉토리 생성
        tokio::fs::create_dir_all(&self.data_dir).await?;
        tokio::fs::create_dir_all(self.videos_dir()).await?;

        let ytdlp_path = self.ytdlp_path();

        if ytdlp_path.exists() {
            tracing::info!("yt-dlp already exists at {:?}", ytdlp_path);
            // 업데이트 체크는 나중에 추가 가능
            return Ok(());
        }

        tracing::info!("Downloading yt-dlp...");

        // GitHub API에서 최신 릴리즈 정보 가져오기
        let release_info: serde_json::Value = self
            .client
            .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
            .header("User-Agent", "ivLyrics-helper")
            .send()
            .await?
            .json()
            .await?;

        // 플랫폼에 맞는 실행 파일 URL 찾기
        let assets = release_info["assets"].as_array().ok_or("No assets found")?;
        let binary_name = Self::get_ytdlp_binary_name();

        let download_url = assets
            .iter()
            .find(|asset| {
                asset["name"]
                    .as_str()
                    .map(|n| n == binary_name)
                    .unwrap_or(false)
            })
            .and_then(|asset| asset["browser_download_url"].as_str())
            .ok_or_else(|| format!("{} not found in release", binary_name))?;

        tracing::info!("Downloading from: {}", download_url);

        // 다운로드
        let response = self.client.get(download_url).send().await?;
        let bytes = response.bytes().await?;

        // 파일 저장
        tokio::fs::write(&ytdlp_path, bytes).await?;

        // macOS/Linux에서는 실행 권한 부여
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&ytdlp_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&ytdlp_path, perms).await?;
        }

        tracing::info!("yt-dlp downloaded successfully to {:?}", ytdlp_path);

        Ok(())
    }

    /// 비디오가 이미 존재하는지 확인
    pub fn video_exists(&self, video_id: &str) -> bool {
        self.video_path(video_id).exists()
    }

    /// 비디오 다운로드 (진행 상황을 broadcast 채널로 전송)
    pub async fn download_video(
        &self,
        video_id: &str,
        progress_tx: broadcast::Sender<DownloadProgress>,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let video_path = self.video_path(video_id);
        let video_id_owned = video_id.to_string();

        // 이미 존재하면 바로 반환
        if video_path.exists() {
            let _ = progress_tx.send(DownloadProgress {
                video_id: video_id_owned,
                status: DownloadStatus::AlreadyExists,
                percent: Some(100.0),
                speed: None,
                eta: None,
                message: Some("Video already downloaded".to_string()),
            });
            return Ok(video_path);
        }

        // 다운로드 상태 전송
        let _ = progress_tx.send(DownloadProgress {
            video_id: video_id_owned.clone(),
            status: DownloadStatus::Checking,
            percent: Some(0.0),
            speed: None,
            eta: None,
            message: Some("Checking video availability...".to_string()),
        });

        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        let output_template = self.videos_dir().join("%(id)s.%(ext)s");

        // yt-dlp 실행
        // -f: 1080p webm 비디오만, 오디오 없음. 없으면 차선책
        // --no-audio: 오디오 포함 안 함 (--no-audio 대신 format 선택으로 처리)
        let mut cmd = Command::new(self.ytdlp_path());
        cmd.args([
            "-f", "bestvideo[height<=1080][ext=webm]/bestvideo[height<=1080]/bestvideo[ext=webm]/bestvideo",
            "--no-playlist",
            "--progress",
            "--newline",
            "-o", output_template.to_str().unwrap(),
            &url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            // Prevent opening a console window when running yt-dlp
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

        let video_id_for_stdout = video_id_owned.clone();
        let progress_tx_clone = progress_tx.clone();

        // stdout에서 진행률 파싱
        let stdout_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            // 진행률 파싱용 정규식: [download] 12.3% of 100.00MiB at 5.00MiB/s ETA 00:15
            let progress_regex = Regex::new(
                r"\[download\]\s+(\d+\.?\d*)%\s+of\s+[\d.]+\w*\s+at\s+([\d.]+\w*/s)\s+ETA\s+(\S+)",
            )
            .ok();

            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!("yt-dlp stdout: {}", line);

                if let Some(ref regex) = progress_regex {
                    if let Some(caps) = regex.captures(&line) {
                        let percent: f32 = caps
                            .get(1)
                            .and_then(|m| m.as_str().parse().ok())
                            .unwrap_or(0.0);
                        let speed = caps.get(2).map(|m| m.as_str().to_string());
                        let eta = caps.get(3).map(|m| m.as_str().to_string());

                        let _ = progress_tx_clone.send(DownloadProgress {
                            video_id: video_id_for_stdout.clone(),
                            status: DownloadStatus::Downloading,
                            percent: Some(percent),
                            speed,
                            eta,
                            message: Some(format!("Downloading: {:.1}%", percent)),
                        });
                    }
                }

                // Merging 또는 post-processing 메시지 감지
                if line.contains("[Merger]")
                    || line.contains("[ExtractAudio]")
                    || line.contains("Deleting")
                {
                    let _ = progress_tx_clone.send(DownloadProgress {
                        video_id: video_id_for_stdout.clone(),
                        status: DownloadStatus::Processing,
                        percent: Some(99.0),
                        speed: None,
                        eta: None,
                        message: Some("Processing...".to_string()),
                    });
                }
            }
        });

        let video_id_for_stderr = video_id_owned.clone();
        let progress_tx_for_stderr = progress_tx.clone();

        // stderr도 읽기
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut last_error = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("yt-dlp stderr: {}", line);
                last_error = line;
            }

            if !last_error.is_empty() {
                let _ = progress_tx_for_stderr.send(DownloadProgress {
                    video_id: video_id_for_stderr.clone(),
                    status: DownloadStatus::Error,
                    percent: None,
                    speed: None,
                    eta: None,
                    message: Some(last_error),
                });
            }
        });

        // 프로세스 종료 대기
        let status = child.wait().await?;

        // stdout/stderr 핸들러 종료 대기
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        if status.success() {
            // 다운로드된 파일 찾기 (확장자가 다를 수 있음)
            let videos_dir = self.videos_dir();
            let mut found_path = None;

            if let Ok(mut entries) = tokio::fs::read_dir(&videos_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    if file_name_str.starts_with(video_id) {
                        found_path = Some(entry.path());
                        break;
                    }
                }
            }

            if let Some(path) = found_path {
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
                    message: Some(format!(
                        "http://localhost:15123/video/files/{}",
                        file_name
                    )),
                });
                Ok(path)
            } else {
                Err("Downloaded file not found".into())
            }
        } else {
            Err(format!("yt-dlp exited with status: {}", status).into())
        }
    }
}
