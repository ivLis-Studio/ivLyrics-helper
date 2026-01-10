mod autostart;
mod config;
mod lyrics_server;
mod video_server;
mod ytdlp;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use lyrics_server::LyricsData;
use lyrics_server::ProgressData;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_updater::UpdaterExt;
use tokio::process::Command;
use tokio::sync::RwLock;

const GITHUB_OWNER: &str = "ivLis-Studio";
const GITHUB_REPO: &str = "ivLyrics-helper";
const UPDATER_USER_AGENT: &str = "ivLyrics-helper-updater";

pub use config::{AppConfig, ConfigManager};
pub use lyrics_server::LyricsServer;
pub use video_server::VideoServer;
pub use ytdlp::YtDlpManager;

/// 앱 전역 상태
pub struct AppState {
    pub ytdlp: YtDlpManager,
    pub config: Arc<RwLock<ConfigManager>>,
    pub lyrics: Arc<Mutex<Option<LyricsData>>>,
    pub progress: Arc<Mutex<Option<ProgressData>>>,
}

impl AppState {
    pub fn new() -> Self {
        let config_manager = ConfigManager::new();
        let ytdlp = YtDlpManager::new(config_manager.get_video_folder());
        let lyrics = Arc::new(Mutex::new(None));
        let progress = Arc::new(Mutex::new(None));

        Self {
            ytdlp,
            config: Arc::new(RwLock::new(config_manager)),
            lyrics,
            progress,
        }
    }
}

// Tauri Commands

#[tauri::command]
async fn get_config(state: tauri::State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.get_config().clone())
}

#[tauri::command]
async fn save_config(
    state: tauri::State<'_, Arc<AppState>>,
    config: AppConfig,
) -> Result<(), String> {
    let mut config_manager = state.config.write().await;
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_default_video_folder(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state.config.read().await;
    Ok(config.get_default_video_folder())
}

#[tauri::command]
async fn update_video_folder(
    state: tauri::State<'_, Arc<AppState>>,
    folder: String,
) -> Result<(), String> {
    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.videoFolder = folder;
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_max_cache(
    state: tauri::State<'_, Arc<AppState>>,
    max_cache_gb: u32,
) -> Result<(), String> {
    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.maxCacheGB = max_cache_gb;
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_start_minimized(
    state: tauri::State<'_, Arc<AppState>>,
    start_minimized: bool,
) -> Result<(), String> {
    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.startMinimized = start_minimized;
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_start_on_boot(
    state: tauri::State<'_, Arc<AppState>>,
    start_on_boot: bool,
) -> Result<(), String> {
    autostart::set_autostart(start_on_boot).map_err(|e| e.to_string())?;

    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.startOnBoot = start_on_boot;
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_ytdlp_exists(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.ytdlp.ytdlp_path().exists())
}

/// 쿠키 파일을 앱 데이터 폴더에 youtube_cookie.txt로 복사
#[tauri::command]
async fn update_cookies_file(
    state: tauri::State<'_, Arc<AppState>>,
    cookies_file: String,
) -> Result<(), String> {
    // 원본 파일 읽기
    let content = tokio::fs::read(&cookies_file)
        .await
        .map_err(|e| format!("Failed to read cookies file: {}", e))?;

    // 앱 데이터 폴더에 저장
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ivLyrics-helper");

    // 디렉토리 생성
    tokio::fs::create_dir_all(&data_dir)
        .await
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    let target_path = data_dir.join("youtube_cookie.txt");

    // 파일 복사
    tokio::fs::write(&target_path, content)
        .await
        .map_err(|e| format!("Failed to save cookies file: {}", e))?;

    tracing::info!("Cookies file saved to {:?}", target_path);

    // config에는 파일이 등록되었음을 표시 (고정 경로 사용)
    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.cookiesFile = target_path.to_string_lossy().to_string();
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

/// 쿠키 파일이 등록되어 있는지 확인
#[tauri::command]
async fn has_cookies_file() -> Result<bool, String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ivLyrics-helper");
    let cookie_path = data_dir.join("youtube_cookie.txt");
    Ok(cookie_path.exists())
}

/// 등록된 쿠키 파일 삭제
#[tauri::command]
async fn clear_cookies_file(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ivLyrics-helper");
    let cookie_path = data_dir.join("youtube_cookie.txt");

    // 파일 삭제
    if cookie_path.exists() {
        tokio::fs::remove_file(&cookie_path)
            .await
            .map_err(|e| format!("Failed to delete cookies file: {}", e))?;
        tracing::info!("Cookies file deleted");
    }

    // config 업데이트
    let mut config_manager = state.config.write().await;
    let mut config = config_manager.get_config().clone();
    config.cookiesFile = String::new();
    config_manager
        .save_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_ytdlp(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.ytdlp.ensure_ytdlp().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_cache_usage(state: tauri::State<'_, Arc<AppState>>) -> Result<u64, String> {
    let config = state.config.read().await;
    let videos_dir = std::path::PathBuf::from(&config.get_config().videoFolder);

    if !videos_dir.exists() {
        return Ok(0);
    }

    let mut total_size: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(&videos_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
    }

    Ok(total_size)
}

#[tauri::command]
async fn clear_cache(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let config = state.config.read().await;
    let videos_dir = std::path::PathBuf::from(&config.get_config().videoFolder);

    if !videos_dir.exists() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(&videos_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    find_available_update(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    match try_tauri_update(&app).await {
        Ok(updated) if updated => return Ok(()),
        Ok(_) => return Ok(()),
        Err(e) => {
            tracing::warn!("Tauri updater failed, attempting GitHub fallback: {}", e);
        }
    }

    perform_github_update(&app).await.map(|_| ())
}

/// 앱 시작 시 백그라운드에서 업데이트 체크
async fn check_update_on_startup(app: AppHandle) {
    // 앱 시작 후 3초 뒤에 업데이트 체크
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    match find_available_update(&app).await {
        Ok(Some(version)) => {
            tracing::info!("Update available: {}", version);
            // 업데이트가 있으면 프론트엔드에 알림
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("update-available", version);
            }
        }
        Ok(None) => {
            tracing::info!("No updates available");
        }
        Err(e) => {
            tracing::warn!("Failed to check for updates: {}", e);
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize, Clone)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone)]
struct ReleaseInfo {
    version: String,
    asset: GitHubAsset,
}

/// Tauri updater가 실패했을 때 GitHub API로 최신 버전을 확인
async fn find_available_update(app: &AppHandle) -> Result<Option<String>, String> {
    let current_version = app.package_info().version.to_string();

    if let Ok(updater) = app.updater() {
        match updater.check().await {
            Ok(Some(update)) => return Ok(Some(update.version)),
            Ok(None) => return Ok(None),
            Err(e) => tracing::warn!("Tauri updater check failed: {}", e),
        }
    } else {
        tracing::warn!("Tauri updater is not available");
    }

    match fetch_latest_release_info().await {
        Ok(Some(release)) => {
            if is_version_newer(&current_version, &release.version) {
                Ok(Some(release.version))
            } else {
                Ok(None)
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 내장 업데이트 설치 시도 (성공 여부 반환)
async fn try_tauri_update(app: &AppHandle) -> Result<bool, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            update
                .download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// GitHub 릴리스에서 최신 인스톨러를 내려받아 실행
async fn perform_github_update(app: &AppHandle) -> Result<bool, String> {
    let current_version = app.package_info().version.to_string();
    let Some(release) = fetch_latest_release_info().await? else {
        return Ok(false);
    };

    if !is_version_newer(&current_version, &release.version) {
        return Ok(false);
    }

    let installer_path = download_asset(&release.asset).await?;
    tracing::info!("Downloaded fallback installer to {:?}", installer_path);
    let install_result = install_downloaded(&installer_path).await;
    let _ = tokio::fs::remove_file(&installer_path).await;
    install_result.map(|_| true)
}

async fn fetch_latest_release_info() -> Result<Option<ReleaseInfo>, String> {
    let client = Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let response = client
        .get(url)
        .header("User-Agent", UPDATER_USER_AGENT)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API responded with status {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response.json().await.map_err(|e| e.to_string())?;
    let version = normalize_version(&release.tag_name);
    let asset = select_asset(&release.assets);

    Ok(asset.map(|asset| ReleaseInfo { version, asset }))
}

fn normalize_version(tag: &str) -> String {
    tag.trim_start_matches('v').to_string()
}

fn is_version_newer(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => latest != current,
    }
}

fn select_asset(assets: &[GitHubAsset]) -> Option<GitHubAsset> {
    #[cfg(target_os = "windows")]
    let preferred = [".exe", ".msi"];
    #[cfg(target_os = "macos")]
    let preferred = [".dmg", ".app.tar.gz", ".zip"];
    #[cfg(target_os = "linux")]
    let preferred = [".AppImage", ".appimage", ".tar.gz"];
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let preferred: [&str; 0] = [];

    preferred
        .iter()
        .find_map(|ext| {
            assets
                .iter()
                .find(|asset| asset.name.ends_with(ext))
                .cloned()
        })
        .or_else(|| assets.first().cloned())
}

async fn download_asset(asset: &GitHubAsset) -> Result<PathBuf, String> {
    let client = Client::new();
    let response = client
        .get(&asset.browser_download_url)
        .header("User-Agent", UPDATER_USER_AGENT)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download installer: {}",
            response.status()
        ));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let mut path = std::env::temp_dir();
    path.push(&asset.name);

    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(path)
}

async fn install_downloaded(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let path_str = path
            .to_str()
            .ok_or_else(|| "Invalid installer path".to_string())?;

        let status = if ext == "msi" {
            Command::new("msiexec")
                .args(["/i", path_str, "/passive"])
                .spawn()
                .map_err(|e| e.to_string())?
                .wait()
                .await
                .map_err(|e| e.to_string())?
        } else {
            Command::new(path)
                .spawn()
                .map_err(|e| e.to_string())?
                .wait()
                .await
                .map_err(|e| e.to_string())?
        };

        if status.success() {
            Ok(())
        } else {
            Err(format!("Installer exited with status {}", status))
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?
            .wait()
            .await
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Installer exited with status {}", status))
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new(path)
            .spawn()
            .map_err(|e| e.to_string())?
            .wait()
            .await
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Installer exited with status {}", status))
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported platform for fallback update".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let app_state = Arc::new(AppState::new());
    let app_state_for_server = app_state.clone();
    let (start_minimized, start_on_boot) = {
        let config = futures::executor::block_on(app_state.config.read());
        let config = config.get_config();
        (
            config.startMinimized && config.setupComplete,
            config.startOnBoot && config.setupComplete,
        )
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_default_video_folder,
            update_video_folder,
            update_max_cache,
            update_start_minimized,
            update_start_on_boot,
            check_ytdlp_exists,
            update_cookies_file,
            has_cookies_file,
            clear_cookies_file,
            download_ytdlp,
            get_cache_usage,
            clear_cache,
            check_for_updates,
            install_update,
        ])
        .setup(move |app| {
            let app_state = app_state_for_server.clone();

            // 트레이 아이콘 메뉴 생성
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // 트레이 아이콘 생성
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("ivLyrics Helper")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // startMinimized가 true면 창 숨기기
            if start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Ensure autostart entry matches config
            if let Err(e) = autostart::set_autostart(start_on_boot) {
                tracing::warn!("Failed to update autostart entry: {}", e);
            }

            // API 서버를 별도 스레드에서 시작
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async {
                    // 비디오, 가사 API 시작 및 병합
                    let video_router = VideoServer::new(app_state.ytdlp.clone()).get_router();
                    let lyrics_router =
                        LyricsServer::new(app_state.progress.clone(), app_state.lyrics.clone())
                            .get_router();

                    let app = axum::Router::new()
                        .merge(video_router)
                        .merge(lyrics_router)
                        .layer(
                            tower_http::cors::CorsLayer::new()
                                .allow_origin(tower_http::cors::Any)
                                .allow_methods(tower_http::cors::Any)
                                .allow_headers(tower_http::cors::Any),
                        );

                    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 15123));
                    tracing::info!("Server listening on http://{}", addr);

                    if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
                        if let Err(e) = axum::serve(listener, app).await {
                            tracing::error!("Server error: {}", e);
                        }
                    } else {
                        tracing::error!("Failed to bind port 15123");
                    }
                });
            });

            // 백그라운드에서 업데이트 체크
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_update_on_startup(app_handle).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 창 닫기 버튼을 누르면 프로그램 종료 대신 창 숨기기
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
