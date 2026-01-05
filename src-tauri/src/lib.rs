mod config;
mod video_server;
mod ytdlp;

use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, AppHandle,
};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::RwLock;

pub use config::{AppConfig, ConfigManager};
pub use video_server::VideoServer;
pub use ytdlp::YtDlpManager;

/// 앱 전역 상태
pub struct AppState {
    pub ytdlp: YtDlpManager,
    pub config: Arc<RwLock<ConfigManager>>,
}

impl AppState {
    pub fn new() -> Self {
        let config_manager = ConfigManager::new();
        let ytdlp = YtDlpManager::new(config_manager.get_video_folder());

        Self {
            ytdlp,
            config: Arc::new(RwLock::new(config_manager)),
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
async fn check_ytdlp_exists(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.ytdlp.ytdlp_path().exists())
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
    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => Ok(Some(update.version)),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 앱 시작 시 백그라운드에서 업데이트 체크
async fn check_update_on_startup(app: AppHandle) {
    // 앱 시작 후 3초 뒤에 업데이트 체크
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("Failed to get updater: {}", e);
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!("Update available: {}", update.version);
            // 업데이트가 있으면 프론트엔드에 알림
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("update-available", update.version);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let app_state = Arc::new(AppState::new());
    let app_state_for_server = app_state.clone();
    let start_minimized = {
        let config = futures::executor::block_on(app_state.config.read());
        config.get_config().startMinimized && config.get_config().setupComplete
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
            check_ytdlp_exists,
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

            // API 서버를 별도 스레드에서 시작
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async {
                    // 비디오 API 서버 시작
                    let server = VideoServer::new(app_state.ytdlp.clone());
                    if let Err(e) = server.start(15123).await {
                        tracing::error!("Failed to start video server: {}", e);
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
