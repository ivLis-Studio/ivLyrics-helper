use std::fs;
use std::path::PathBuf;

pub const DEFAULT_VIDEO_QUALITY: &str = "1080p";
pub const VIDEO_QUALITY_OPTIONS: [&str; 6] = ["best", "2160p", "1440p", "1080p", "720p", "480p"];

pub fn normalize_video_quality(quality: &str) -> Option<&'static str> {
    let quality = quality.trim();
    VIDEO_QUALITY_OPTIONS
        .iter()
        .copied()
        .find(|candidate| *candidate == quality)
}

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
    /// 비디오 최대 화질 설정 (best, 2160p, 1440p, 1080p, 720p, 480p)
    #[serde(default = "default_video_quality")]
    pub videoQuality: String,
}

fn default_max_cache() -> u32 {
    10
}

fn default_language() -> String {
    "en".to_string()
}

fn default_video_quality() -> String {
    DEFAULT_VIDEO_QUALITY.to_string()
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
            videoQuality: DEFAULT_VIDEO_QUALITY.to_string(),
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
        let mut config = if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            }
        } else {
            let mut default_config = AppConfig::default();
            default_config.videoFolder = data_dir.join("videos").to_string_lossy().to_string();
            default_config
        };
        config.videoQuality = normalize_video_quality(&config.videoQuality)
            .unwrap_or(DEFAULT_VIDEO_QUALITY)
            .to_string();

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

    pub fn get_video_quality(&self) -> String {
        normalize_video_quality(&self.config.videoQuality)
            .unwrap_or(DEFAULT_VIDEO_QUALITY)
            .to_string()
    }

    pub fn save_config(&mut self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = config.clone();
        config.videoQuality = normalize_video_quality(&config.videoQuality)
            .unwrap_or(DEFAULT_VIDEO_QUALITY)
            .to_string();
        self.config = config;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_supported_video_qualities() {
        for quality in VIDEO_QUALITY_OPTIONS {
            assert_eq!(normalize_video_quality(quality), Some(quality));
        }

        assert_eq!(normalize_video_quality(" 720p "), Some("720p"));
        assert_eq!(normalize_video_quality("4320p"), None);
        assert_eq!(normalize_video_quality(""), None);
    }
}
