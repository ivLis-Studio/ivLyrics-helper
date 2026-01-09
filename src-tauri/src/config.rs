use std::fs;
use std::path::PathBuf;

/// 앱 설정 (JavaScript와 호환을 위해 camelCase 사용)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct AppConfig {
    #[serde(default)]
    pub setupComplete: bool,
    #[serde(default)]
    pub videoFolder: String,
    #[serde(default = "default_max_cache")]
    pub maxCacheGB: u32,
    #[serde(default)]
    pub startMinimized: bool,
    #[serde(default)]
    pub startOnBoot: bool,
    #[serde(default = "default_language")]
    pub language: String,
    /// cookies.txt 파일 경로 (YouTube 성인인증 영상에 필요)
    #[serde(default)]
    pub cookiesFile: String,
}

fn default_max_cache() -> u32 {
    10
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            setupComplete: false,
            videoFolder: String::new(),
            maxCacheGB: 10,
            startMinimized: false,
            startOnBoot: false,
            language: "en".to_string(),
            cookiesFile: String::new(),
        }
    }
}

/// 설정 관리자
pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ivLyrics-helper");

        let config_path = data_dir.join("config.json");

        // 디렉토리 생성
        let _ = fs::create_dir_all(&data_dir);

        // 설정 로드 또는 기본값 사용
        let config = if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            }
        } else {
            let mut default_config = AppConfig::default();
            default_config.videoFolder = data_dir.join("videos").to_string_lossy().to_string();
            default_config
        };

        Self {
            config_path,
            config,
        }
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    pub fn get_video_folder(&self) -> PathBuf {
        if self.config.videoFolder.is_empty() {
            self.get_default_video_folder_path()
        } else {
            PathBuf::from(&self.config.videoFolder)
        }
    }

    pub fn get_default_video_folder(&self) -> String {
        self.get_default_video_folder_path()
            .to_string_lossy()
            .to_string()
    }

    fn get_default_video_folder_path(&self) -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ivLyrics-helper")
            .join("videos")
    }

    pub fn save_config(&mut self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.config = config.clone();

        // 디렉토리 생성
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 비디오 폴더 생성
        if !self.config.videoFolder.is_empty() {
            let _ = fs::create_dir_all(&self.config.videoFolder);
        }

        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content)?;

        Ok(())
    }
}
