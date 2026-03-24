use crate::config::AppConfig;
use regex::Regex;
use reqwest::Client;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::SystemTime;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, Mutex};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, serde::Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubReleaseAsset>,
}

struct YtDlpReleaseInfo {
    version: String,
    download_url: String,
}

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
    sync_lock: Arc<Mutex<()>>,
    sync_completed: Arc<AtomicBool>,
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
            sync_lock: Arc::new(Mutex::new(())),
            sync_completed: Arc::new(AtomicBool::new(false)),
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

    /// Deno 런타임 설치 (Windows)
    #[cfg(windows)]
    async fn ensure_deno(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deno_path = self.data_dir.join("deno.exe");
        if deno_path.exists() {
            return Ok(());
        }

        tracing::info!("Downloading Deno runtime...");
        let url = "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip";

        let response = self
            .client
            .get(url)
            .header("User-Agent", "ivLyrics-helper")
            .send()
            .await?;

        let bytes = response.bytes().await?;
        let data_dir = self.data_dir.clone();

        // Blocking 작업 (압축 해제)
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                use std::io::Cursor;

                let reader = Cursor::new(bytes);
                let mut archive = zip::ZipArchive::new(reader)?;

                // deno.exe 추출
                let mut file = archive.by_name("deno.exe")?;
                let mut out_file = std::fs::File::create(data_dir.join("deno.exe"))?;
                std::io::copy(&mut file, &mut out_file)?;

                Ok(())
            },
        )
        .await??;

        tracing::info!("Deno downloaded successfully to {:?}", deno_path);
        Ok(())
    }

    /// 앱 세션당 한 번만 yt-dlp 설치/업데이트를 동기화
    pub async fn ensure_ytdlp(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.sync_completed.load(Ordering::Acquire) && self.ytdlp_path().exists() {
            return Ok(());
        }

        let _guard = self.sync_lock.lock().await;

        if self.sync_completed.load(Ordering::Acquire) && self.ytdlp_path().exists() {
            return Ok(());
        }

        self.sync_ytdlp().await?;
        self.sync_completed.store(true, Ordering::Release);
        Ok(())
    }

    async fn sync_ytdlp(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 디렉토리 생성
        tokio::fs::create_dir_all(&self.data_dir).await?;
        tokio::fs::create_dir_all(self.videos_dir()).await?;

        let ytdlp_path = self.ytdlp_path();
        let had_existing_binary = ytdlp_path.exists();

        #[cfg(windows)]
        {
            if let Err(e) = self.ensure_deno().await {
                tracing::warn!("Failed to ensure Deno: {}", e);
            }
        }

        let latest_release = match self.fetch_latest_release().await {
            Ok(release) => release,
            Err(e) if had_existing_binary => {
                tracing::warn!(
                    "Failed to check latest yt-dlp release, using installed binary: {}",
                    e
                );
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        match self.installed_ytdlp_version().await {
            Ok(Some(installed_version)) if installed_version == latest_release.version => {
                tracing::info!("yt-dlp is up to date ({})", installed_version);
                return Ok(());
            }
            Ok(Some(installed_version)) => {
                tracing::info!(
                    "Updating yt-dlp from {} to {}",
                    installed_version,
                    latest_release.version
                );
            }
            Ok(None) => {
                tracing::info!("yt-dlp is missing, downloading {}", latest_release.version);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to determine installed yt-dlp version, reinstalling latest: {}",
                    e
                );
            }
        }

        if let Err(e) = self
            .download_ytdlp_binary(&latest_release.download_url, &ytdlp_path)
            .await
        {
            if had_existing_binary {
                tracing::warn!(
                    "Failed to update yt-dlp to {}, keeping installed binary: {}",
                    latest_release.version,
                    e
                );
                return Ok(());
            }

            return Err(e);
        }

        tracing::info!(
            "yt-dlp synchronized successfully to {} at {:?}",
            latest_release.version,
            ytdlp_path
        );

        Ok(())
    }

    async fn fetch_latest_release(
        &self,
    ) -> Result<YtDlpReleaseInfo, Box<dyn std::error::Error + Send + Sync>> {
        let release_info: GitHubRelease = self
            .client
            .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
            .header("User-Agent", "ivLyrics-helper")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let binary_name = Self::get_ytdlp_binary_name();
        let download_url = release_info
            .assets
            .iter()
            .find(|asset| asset.name == binary_name)
            .map(|asset| asset.browser_download_url.clone())
            .ok_or_else(|| format!("{} not found in latest yt-dlp release", binary_name))?;

        Ok(YtDlpReleaseInfo {
            version: release_info.tag_name.trim_start_matches('v').to_string(),
            download_url,
        })
    }

    async fn installed_ytdlp_version(
        &self,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let ytdlp_path = self.ytdlp_path();
        if !ytdlp_path.exists() {
            return Ok(None);
        }

        let mut cmd = Command::new(&ytdlp_path);
        cmd.arg("--version");

        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(format!("yt-dlp --version exited with status {}", output.status).into());
        }

        let version = String::from_utf8(output.stdout)?
            .lines()
            .next()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string());

        Ok(version)
    }

    async fn download_ytdlp_binary(
        &self,
        download_url: &str,
        ytdlp_path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Downloading yt-dlp from {}", download_url);

        let response = self
            .client
            .get(download_url)
            .header("User-Agent", "ivLyrics-helper")
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;

        let file_name = ytdlp_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid yt-dlp path")?;
        let temp_path = ytdlp_path.with_file_name(format!("{}.download", file_name));
        let backup_path = ytdlp_path.with_file_name(format!("{}.bak", file_name));

        tokio::fs::write(&temp_path, bytes).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&temp_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&temp_path, perms).await?;
        }

        let had_existing_binary = ytdlp_path.exists();
        if had_existing_binary {
            if backup_path.exists() {
                tokio::fs::remove_file(&backup_path).await?;
            }
            tokio::fs::rename(ytdlp_path, &backup_path).await?;
        }

        if let Err(e) = tokio::fs::rename(&temp_path, ytdlp_path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            if had_existing_binary && backup_path.exists() {
                let _ = tokio::fs::rename(&backup_path, ytdlp_path).await;
            }
            return Err(e.into());
        }

        if had_existing_binary && backup_path.exists() {
            let _ = tokio::fs::remove_file(&backup_path).await;
        }

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

        self.ensure_ytdlp().await?;

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

        // 설정에서 화질 가져오기
        let video_quality = self.get_video_quality().await;
        let format_string = self.get_format_string(&video_quality);

        // yt-dlp 명령 구성
        let mut cmd = Command::new(self.ytdlp_path());

        let mut args = vec![
            "-f".to_string(),
            format_string,  // 동적으로 생성된 포맷 문자열 사용
            "--no-playlist".to_string(),
            "--progress".to_string(),
            "--newline".to_string(),
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
    // 설정에서 화질 가져오기
    async fn get_video_quality(&self) -> String {
        let config_path = self.data_dir.join("config.json");
        if let Ok(content) = tokio::fs::read(&config_path).await {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&content) {
                if !cfg.videoQuality.is_empty() {
                    return cfg.videoQuality;
                }
            }
        }

        // 기본값 1080p
        "1080p".to_string()
    }

    // 화질에 따른 포맷 문자열 생성
fn get_format_string(&self, quality: &str) -> String {
    match quality {
        "2160p" => {
            // 4K: 단일 스트림 우선 (병합 불필요)
            "bestvideo[height<=2160][ext=webm]/bestvideo[height<=2160]/best[height<=2160]/best"
        }
        "1440p" => {
            "bestvideo[height<=1440][ext=webm]/bestvideo[height<=1440]/best[height<=1440]/best"
        }
        "1080p" => {
            "bestvideo[height<=1080][ext=webm]/bestvideo[height<=1080]/best[height<=1080]/best"
        }
        "720p" => {
            "bestvideo[height<=720][ext=webm]/bestvideo[height<=720]/best[height<=720]/best"
        }
        "480p" => {
            "bestvideo[height<=480][ext=webm]/bestvideo[height<=480]/best[height<=480]/best"
        }
        _ => {
            "bestvideo[height<=1080][ext=webm]/bestvideo[height<=1080]/best[height<=1080]/best"
        }
        }.to_string()
    }
}
