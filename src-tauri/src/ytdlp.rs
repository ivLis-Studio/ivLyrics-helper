use crate::config::AppConfig;
use regex::Regex;
use reqwest::Client;

use std::path::PathBuf;
use std::process::Stdio;
use std::time::SystemTime;
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
                "yt-dlp_macos" // ARM Mac (Apple Silicon)
            } else {
                "yt-dlp_macos" // Intel Mac (same binary, universal)
            }
        } else {
            "yt-dlp" // Linux
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

    /// 설치된 브라우저 감지 (Windows)
    #[cfg(windows)]
    fn detect_installed_browsers() -> Vec<&'static str> {
        let mut installed = Vec::new();

        // %LOCALAPPDATA% 환경 변수 가져오기
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

        // 브라우저별 설치 경로 확인
        // (browser_name, system_paths, user_local_path_suffix)
        let browsers: &[(&str, &[&str], Option<&str>)] = &[
            (
                "chrome",
                &[
                    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                ],
                Some(r"Google\Chrome\Application\chrome.exe"),
            ),
            (
                "edge",
                &[
                    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
                ],
                None,
            ),
            (
                "firefox",
                &[
                    r"C:\Program Files\Mozilla Firefox\firefox.exe",
                    r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
                ],
                None,
            ),
            (
                "vivaldi",
                &[r"C:\Program Files\Vivaldi\Application\vivaldi.exe"],
                Some(r"Vivaldi\Application\vivaldi.exe"),
            ),
            (
                "opera",
                &[
                    r"C:\Program Files\Opera\launcher.exe",
                    r"C:\Program Files (x86)\Opera\launcher.exe",
                ],
                Some(r"Programs\Opera\launcher.exe"),
            ),
            (
                "brave",
                &[
                    r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
                    r"C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe",
                ],
                Some(r"BraveSoftware\Brave-Browser\Application\brave.exe"),
            ),
            (
                "whale",
                &[
                    r"C:\Program Files\Naver\Naver Whale\Application\whale.exe",
                    r"C:\Program Files (x86)\Naver\Naver Whale\Application\whale.exe",
                ],
                Some(r"Naver\Naver Whale\Application\whale.exe"),
            ),
        ];

        for (browser_name, system_paths, user_local_suffix) in browsers {
            let mut found = false;

            // 시스템 경로 확인 (Program Files)
            for path in *system_paths {
                if std::path::Path::new(path).exists() {
                    found = true;
                    break;
                }
            }

            // 시스템 경로에 없으면 사용자 로컬 경로 확인 (%LOCALAPPDATA%)
            if !found {
                if let Some(suffix) = user_local_suffix {
                    if !local_app_data.is_empty() {
                        let user_path = format!("{}\\{}", local_app_data, suffix);
                        if std::path::Path::new(&user_path).exists() {
                            found = true;
                            tracing::debug!(
                                "Found {} at user-local path: {}",
                                browser_name,
                                user_path
                            );
                        }
                    }
                }
            }

            if found {
                installed.push(*browser_name);
            }
        }

        // 우선순위에 따라 정렬 (Firefox 우선 - Chrome/Edge는 Windows에서 DPAPI 문제가 있음)
        // Firefox → Whale → Chrome → Edge → Vivaldi → Opera → Brave
        let priority_order = [
            "firefox", "whale", "chrome", "edge", "vivaldi", "opera", "brave",
        ];
        installed.sort_by_key(|browser| {
            priority_order
                .iter()
                .position(|b| b == browser)
                .unwrap_or(999)
        });

        tracing::info!("Detected installed browsers (Windows): {:?}", installed);
        installed
    }

    /// 설치된 브라우저 감지 (macOS)
    #[cfg(target_os = "macos")]
    fn detect_installed_browsers() -> Vec<&'static str> {
        let mut installed = Vec::new();

        // 각 브라우저의 설치 경로를 확인
        let browser_paths: &[(&str, &[&str])] = &[
            (
                "chrome",
                &[
                    "/Applications/Google Chrome.app",
                    "~/Applications/Google Chrome.app",
                ],
            ),
            (
                "edge",
                &[
                    "/Applications/Microsoft Edge.app",
                    "~/Applications/Microsoft Edge.app",
                ],
            ),
            (
                "firefox",
                &["/Applications/Firefox.app", "~/Applications/Firefox.app"],
            ),
            (
                "vivaldi",
                &["/Applications/Vivaldi.app", "~/Applications/Vivaldi.app"],
            ),
            (
                "opera",
                &["/Applications/Opera.app", "~/Applications/Opera.app"],
            ),
            (
                "brave",
                &[
                    "/Applications/Brave Browser.app",
                    "~/Applications/Brave Browser.app",
                ],
            ),
            (
                "whale",
                &["/Applications/Whale.app", "~/Applications/Whale.app"],
            ),
            ("safari", &["/Applications/Safari.app"]),
        ];

        for (browser_name, paths) in browser_paths {
            for path in *paths {
                let expanded_path = if path.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&path[2..])
                    } else {
                        PathBuf::from(path)
                    }
                } else {
                    PathBuf::from(path)
                };

                if expanded_path.exists() {
                    installed.push(*browser_name);
                    break;
                }
            }
        }

        // 우선순위에 따라 정렬
        let priority_order = [
            "chrome", "edge", "firefox", "vivaldi", "opera", "brave", "whale", "safari",
        ];
        installed.sort_by_key(|browser| {
            priority_order
                .iter()
                .position(|b| b == browser)
                .unwrap_or(999)
        });

        tracing::info!("Detected installed browsers: {:?}", installed);
        installed
    }

    /// 설치된 브라우저 감지 (Linux)
    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn detect_installed_browsers() -> Vec<&'static str> {
        use std::process::Command as StdCommand;

        let mut installed = Vec::new();

        // which 명령어로 브라우저 실행 파일 확인
        let browser_commands: &[(&str, &[&str])] = &[
            (
                "chrome",
                &["google-chrome", "google-chrome-stable", "chrome"],
            ),
            ("chromium", &["chromium", "chromium-browser"]),
            ("edge", &["microsoft-edge", "microsoft-edge-stable"]),
            ("firefox", &["firefox"]),
            ("vivaldi", &["vivaldi", "vivaldi-stable"]),
            ("opera", &["opera"]),
            ("brave", &["brave", "brave-browser"]),
        ];

        for (browser_name, commands) in browser_commands {
            for cmd in *commands {
                let output = StdCommand::new("which").arg(cmd).output();
                if let Ok(out) = output {
                    if out.status.success() {
                        installed.push(*browser_name);
                        break;
                    }
                }
            }
        }

        // 우선순위에 따라 정렬
        let priority_order = [
            "chrome", "edge", "firefox", "vivaldi", "opera", "brave", "chromium",
        ];
        installed.sort_by_key(|browser| {
            priority_order
                .iter()
                .position(|b| b == browser)
                .unwrap_or(999)
        });

        tracing::info!("Detected installed browsers: {:?}", installed);
        installed
    }

    /// 에러 메시지가 성인인증 관련인지 확인
    fn is_age_restriction_error(error_msg: &str) -> bool {
        error_msg.contains("Sign in to confirm your age")
            || error_msg.contains("age-restricted")
            || error_msg.contains("confirm your age")
            || error_msg.contains("inappropriate for some users")
            || error_msg.contains("--cookies-from-browser")
    }

    /// 에러 메시지가 DPAPI 복호화 실패인지 확인 (Windows Chrome/Edge 쿠키 문제)
    fn is_dpapi_error(error_msg: &str) -> bool {
        error_msg.contains("Failed to decrypt with DPAPI")
            || error_msg.contains("failed to decrypt")
            || error_msg.contains("DPAPI")
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

        // 쿠키 없이 먼저 시도
        let result = self
            .try_download_video(video_id, &progress_tx, None, None)
            .await;

        match result {
            Ok(path) => Ok(path),
            Err(e) => {
                let error_msg = e.to_string();

                // 성인인증 에러인 경우 쿠키로 재시도
                if Self::is_age_restriction_error(&error_msg) {
                    tracing::info!("Age restriction detected, attempting to use cookies...");

                    // 1. 먼저 cookies.txt 파일로 시도 (설정에서 지정한 경우)
                    let cookies_file = self.get_cookies_file_path().await;
                    if let Some(ref cookies_path) = cookies_file {
                        if std::path::Path::new(cookies_path).exists() {
                            tracing::info!("Trying with cookies.txt file: {}", cookies_path);

                            let _ = progress_tx.send(DownloadProgress {
                                video_id: video_id_owned.clone(),
                                status: DownloadStatus::Checking,
                                percent: Some(0.0),
                                speed: None,
                                eta: None,
                                message: Some("Trying with cookies.txt file...".to_string()),
                            });

                            match self
                                .try_download_video(
                                    video_id,
                                    &progress_tx,
                                    None,
                                    Some(cookies_path.as_str()),
                                )
                                .await
                            {
                                Ok(path) => {
                                    tracing::info!("Successfully downloaded with cookies.txt");
                                    return Ok(path);
                                }
                                Err(cookies_err) => {
                                    tracing::warn!("Failed with cookies.txt: {}", cookies_err);
                                }
                            }
                        }
                    }

                    // 2. 브라우저 쿠키로 시도
                    let installed_browsers = Self::detect_installed_browsers();

                    if installed_browsers.is_empty() && cookies_file.is_none() {
                        tracing::warn!("No supported browsers or cookies.txt found");
                        let _ = progress_tx.send(DownloadProgress {
                            video_id: video_id_owned.clone(),
                            status: DownloadStatus::Error,
                            percent: None,
                            speed: None,
                            eta: None,
                            message: Some("Age-restricted video. No cookies.txt or supported browsers found. Please set a cookies.txt file in Settings.".to_string()),
                        });
                        return Err(e);
                    }

                    // 각 브라우저로 순차적으로 시도
                    for browser in installed_browsers {
                        tracing::info!("Trying with browser cookies: {}", browser);

                        let _ = progress_tx.send(DownloadProgress {
                            video_id: video_id_owned.clone(),
                            status: DownloadStatus::Checking,
                            percent: Some(0.0),
                            speed: None,
                            eta: None,
                            message: Some(format!("Trying with {} cookies...", browser)),
                        });

                        match self
                            .try_download_video(video_id, &progress_tx, Some(browser), None)
                            .await
                        {
                            Ok(path) => {
                                tracing::info!("Successfully downloaded with {} cookies", browser);
                                return Ok(path);
                            }
                            Err(browser_err) => {
                                let err_msg = browser_err.to_string();
                                if Self::is_dpapi_error(&err_msg)
                                    || Self::is_cookie_db_error(&err_msg)
                                {
                                    tracing::warn!("Cookie extraction failed for {} (Chromium security). Trying next browser...", browser);
                                } else {
                                    tracing::warn!(
                                        "Failed with {} cookies: {}",
                                        browser,
                                        browser_err
                                    );
                                }
                                // 다음 브라우저로 계속 시도
                            }
                        }
                    }

                    // 모든 시도 실패
                    let _ = progress_tx.send(DownloadProgress {
                        video_id: video_id_owned.clone(),
                        status: DownloadStatus::Error,
                        percent: None,
                        speed: None,
                        eta: None,
                        message: Some("Age-restricted video. Please set a valid cookies.txt file in Settings. See the help (?) for instructions.".to_string()),
                    });
                    Err(
                        "Failed to download age-restricted video. Please configure cookies.txt file."
                            .into(),
                    )
                } else {
                    Err(e)
                }
            }
        }
    }

    /// cookies.txt 파일 경로 가져오기 (설정에서)
    async fn get_cookies_file_path(&self) -> Option<String> {
        let config_path = self.data_dir.join("config.json");
        if let Ok(content) = tokio::fs::read(&config_path).await {
            if let Ok(cfg) = serde_json::from_slice::<crate::config::AppConfig>(&content) {
                if !cfg.cookiesFile.is_empty() {
                    return Some(cfg.cookiesFile);
                }
            }
        }
        None
    }

    /// 에러 메시지가 쿠키 데이터베이스 복사 실패인지 확인
    fn is_cookie_db_error(error_msg: &str) -> bool {
        error_msg.contains("Could not copy Chrome cookie database")
            || error_msg.contains("could not copy")
            || error_msg.contains("cookie database")
    }

    /// 비디오 다운로드 시도 (브라우저 쿠키 또는 cookies.txt 파일 옵션 포함)
    async fn try_download_video(
        &self,
        video_id: &str,
        progress_tx: &broadcast::Sender<DownloadProgress>,
        browser: Option<&str>,
        cookies_file: Option<&str>,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let video_id_owned = video_id.to_string();

        // 다운로드 상태 전송
        let checking_msg = if cookies_file.is_some() {
            "Checking video with cookies.txt...".to_string()
        } else if let Some(b) = browser {
            format!("Checking video with {} cookies...", b)
        } else {
            "Checking video availability...".to_string()
        };

        let _ = progress_tx.send(DownloadProgress {
            video_id: video_id_owned.clone(),
            status: DownloadStatus::Checking,
            percent: Some(0.0),
            speed: None,
            eta: None,
            message: Some(checking_msg),
        });

        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        let output_template = self.videos_dir().join("%(id)s.%(ext)s");

        // yt-dlp 명령 구성
        let mut cmd = Command::new(self.ytdlp_path());

        let mut args = vec![
            "-f".to_string(), 
            "bestvideo[height<=1080][ext=webm]/bestvideo[height<=1080]/bestvideo[ext=webm]/bestvideo".to_string(),
            "--no-playlist".to_string(),
            "--progress".to_string(),
            "--newline".to_string(),
            // Fix JavaScript runtime issue by using web player client
            "--extractor-args".to_string(),
            "youtube:player_client=web".to_string(),
            // Restrict filenames to avoid Windows invalid character issues
            "--restrict-filenames".to_string(),
        ];

        // cookies.txt 파일 옵션 (우선)
        if let Some(cookies_path) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies_path.to_string());
        }
        // 브라우저 쿠키 옵션
        else if let Some(browser_name) = browser {
            args.push("--cookies-from-browser".to_string());
            args.push(browser_name.to_string());
        }

        args.push("-o".to_string());
        args.push(output_template.to_str().unwrap().to_string());
        args.push(url.clone());

        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
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

        // stderr 캡처 (에러 확인용)
        let stderr_content = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut all_stderr = Vec::new();

            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("yt-dlp stderr: {}", line);
                all_stderr.push(line);
            }

            (video_id_for_stderr, all_stderr)
        });

        // 프로세스 종료 대기
        let status = child.wait().await?;

        // stdout 핸들러 종료 대기
        let _ = stdout_handle.await;

        // stderr 내용 가져오기
        let (_, stderr_lines) = stderr_content.await?;
        let combined_stderr = stderr_lines.join("\n");

        if status.success() {
            // 다운로드된 파일 찾기
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
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Cache pruning (best effort)
                if let Err(e) = self.prune_cache_if_needed().await {
                    tracing::warn!("Failed to prune cache: {}", e);
                }

                let _ = progress_tx.send(DownloadProgress {
                    video_id: video_id_owned,
                    status: DownloadStatus::Completed,
                    percent: Some(100.0),
                    speed: None,
                    eta: None,
                    message: Some(format!("http://localhost:15123/video/files/{}", file_name)),
                });
                Ok(path)
            } else {
                Err("Downloaded file not found".into())
            }
        } else {
            // 에러 발생 시 stderr 내용을 에러로 반환
            let error_msg = if !combined_stderr.is_empty() {
                format!("ERROR: {}", combined_stderr)
            } else {
                format!("yt-dlp exited with status: {}", status)
            };

            let _ = progress_tx.send(DownloadProgress {
                video_id: video_id_owned.clone(),
                status: DownloadStatus::Error,
                percent: None,
                speed: None,
                eta: None,
                message: Some(error_msg.clone()),
            });

            Err(error_msg.into())
        }
    }

    async fn prune_cache_if_needed(&self) -> Result<(), String> {
        let max_bytes = self.max_cache_bytes().await;
        if max_bytes == 0 {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(self.videos_dir())
            .await
            .map_err(|e| e.to_string())?;
        let mut files: Vec<(PathBuf, SystemTime, u64)> = Vec::new();
        let mut total: u64 = 0;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let metadata = entry.metadata().await.map_err(|e| e.to_string())?;
            if metadata.is_file() {
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let size = metadata.len();
                total = total.saturating_add(size);
                files.push((entry.path(), modified, size));
            }
        }

        if total <= max_bytes {
            return Ok(());
        }

        // 오래된 파일부터 삭제
        files.sort_by_key(|(_, modified, _)| *modified);
        for (path, _, size) in files {
            if total <= max_bytes {
                break;
            }
            if tokio::fs::remove_file(&path).await.is_ok() {
                total = total.saturating_sub(size);
            }
        }

        Ok(())
    }

    async fn max_cache_bytes(&self) -> u64 {
        let config_path = self.data_dir.join("config.json");
        if let Ok(content) = tokio::fs::read(&config_path).await {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&content) {
                return (cfg.maxCacheGB as u64) * 1024 * 1024 * 1024;
            }
        }

        // 기본값 10GB
        10 * 1024 * 1024 * 1024
    }
}
