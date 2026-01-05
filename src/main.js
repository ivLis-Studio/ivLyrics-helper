const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const API_BASE = 'http://localhost:15123';

// 다국어 지원
const i18n = {
  en: {
    'setup.subtitle': 'Complete the initial setup before using.',
    'setup.language': 'Language',
    'setup.ytdlpDownload': 'yt-dlp Download',
    'setup.ytdlpDesc': 'Install yt-dlp required for YouTube video download.',
    'setup.waiting': 'Waiting...',
    'setup.checking': 'Checking yt-dlp...',
    'setup.downloading': 'Downloading yt-dlp...',
    'setup.alreadyInstalled': 'yt-dlp is already installed.',
    'setup.downloadComplete': 'Download complete!',
    'setup.downloadFailed': 'Download failed: ',
    'setup.downloadBtn': 'Download yt-dlp',
    'setup.storageSettings': 'Storage Settings',
    'setup.complete': 'Complete Setup',

    'settings.videoFolder': 'Video Save Folder',
    'settings.browse': 'Browse',
    'settings.change': 'Change',
    'settings.maxCache': 'Max Cache Size (GB)',
    'settings.cacheHint': 'Old videos will be auto-deleted when cache exceeds limit.',
    'settings.save': 'Save',
    'settings.currentCache': 'Current Cache Usage',
    'settings.calculating': 'Calculating...',
    'settings.clearCache': 'Clear Cache',
    'settings.language': 'Language',
    'settings.startOnBoot': 'Launch on system startup',
    'settings.startOnBootHint': 'Automatically start after signing in to Windows.',
    'settings.startMinimized': 'Start minimized to tray',
    'settings.startMinimizedHint': 'When enabled, the program starts without a window, only in the tray.',
    'settings.saved': 'Settings saved',
    'settings.saveFailed': 'Failed to save settings',
    'settings.cacheCleared': 'Cache cleared',
    'settings.storageSettings': 'Storage Settings',

    'status.checking': 'Checking server status...',
    'status.online': 'Server running',
    'status.offline': 'Server connection failed',
    'status.apiServer': 'API Server',

    'nav.settings': 'Settings',

    'api.videoRequest': 'Download YouTube video and return streaming URL',
    'api.videoRequestDetail1': 'Returns URL immediately if file exists',
    'api.videoRequestDetail2': 'Streams download progress via SSE if not',
    'api.videoRequestDetail3': 'Auto-selects 1080p WebM (no audio)',
    'api.videoStatus': 'Check video download status',
    'api.videoFiles': 'Serve downloaded video files',

    'confirm.clearCache': 'Delete all cached videos?',
    'unknown': 'Unknown'
  },
  ko: {
    'setup.subtitle': '처음 사용하기 전 기본 설정을 완료해주세요.',
    'setup.language': '언어',
    'setup.ytdlpDownload': 'yt-dlp 다운로드',
    'setup.ytdlpDesc': 'YouTube 영상 다운로드에 필요한 yt-dlp를 설치합니다.',
    'setup.waiting': '대기중...',
    'setup.checking': 'yt-dlp 확인중...',
    'setup.downloading': 'yt-dlp 다운로드중...',
    'setup.alreadyInstalled': 'yt-dlp가 이미 설치되어 있습니다.',
    'setup.downloadComplete': '다운로드 완료!',
    'setup.downloadFailed': '다운로드 실패: ',
    'setup.downloadBtn': 'yt-dlp 다운로드',
    'setup.storageSettings': '저장 설정',
    'setup.complete': '설정 완료',

    'settings.videoFolder': '영상 저장 폴더',
    'settings.browse': '찾아보기',
    'settings.change': '변경',
    'settings.maxCache': '최대 캐시 용량 (GB)',
    'settings.cacheHint': '캐시 용량 초과시 오래된 영상부터 자동 삭제됩니다.',
    'settings.save': '저장',
    'settings.currentCache': '현재 캐시 사용량',
    'settings.calculating': '계산중...',
    'settings.clearCache': '캐시 비우기',
    'settings.language': '언어',
    'settings.startOnBoot': '컴퓨터 시작 시 자동 실행',
    'settings.startOnBootHint': 'Windows에 로그인하면 ivLyrics Helper가 자동으로 실행됩니다.',
    'settings.startMinimized': '시작 시 트레이 아이콘으로 실행',
    'settings.startMinimizedHint': '활성화하면 프로그램이 시작될 때 창 없이 트레이 아이콘으로만 실행됩니다.',
    'settings.saved': '설정이 저장되었습니다',
    'settings.saveFailed': '설정을 저장하지 못했습니다',
    'settings.cacheCleared': '캐시를 비웠습니다',
    'settings.storageSettings': '저장 설정',

    'status.checking': '서버 상태 확인중...',
    'status.online': '서버 실행중',
    'status.offline': '서버 연결 실패',
    'status.apiServer': 'API 서버',

    'nav.settings': '설정',

    'api.videoRequest': 'YouTube 영상 다운로드 및 스트리밍 URL 반환',
    'api.videoRequestDetail1': '기존 파일이 있으면 즉시 URL 반환',
    'api.videoRequestDetail2': '없으면 SSE로 다운로드 진행상황 스트리밍',
    'api.videoRequestDetail3': '1080p WebM (무음) 자동 선택',
    'api.videoStatus': '영상 다운로드 상태 확인',
    'api.videoFiles': '다운로드된 영상 파일 서빙',

    'confirm.clearCache': '모든 캐시된 영상을 삭제하시겠습니까?',
    'unknown': '알 수 없음'
  }
};

let currentLang = 'en';

// 번역 가져오기
function t(key) {
  return i18n[currentLang]?.[key] || i18n['en'][key] || key;
}

// 페이지 전체 번역 적용
function applyTranslations() {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    el.textContent = t(key);
  });
}

function showSaveStatus(message, type = 'success') {
  const statusEl = document.getElementById('save-status');
  if (!statusEl) return;

  statusEl.textContent = message;
  statusEl.classList.remove('hidden', 'success', 'error');
  statusEl.classList.add(type === 'error' ? 'error' : 'success');

  if (saveStatusTimer) {
    clearTimeout(saveStatusTimer);
  }
  saveStatusTimer = setTimeout(() => {
    statusEl.classList.add('hidden');
  }, 2000);
}

// 언어 변경
async function setLanguage(lang, save = true) {
  currentLang = lang;

  // 버튼 활성화 상태 업데이트
  document.querySelectorAll('.lang-btn, .lang-btn-small').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.lang === lang);
  });

  applyTranslations();

  if (save) {
    try {
      await invoke('save_config', {
        config: { ...appState.config, language: lang }
      });
      appState.config.language = lang;
    } catch (error) {
      console.error('Failed to save language:', error);
    }
  }
}

// 앱 상태
let appState = {
  isSetupComplete: false,
  config: {
    videoFolder: '',
    maxCacheGB: 10,
    startMinimized: false,
    startOnBoot: false,
    language: 'en'
  }
};
let saveStatusTimer = null;

// 초기화
document.addEventListener('DOMContentLoaded', async () => {
  await checkSetupStatus();
});

// Setup 상태 확인
async function checkSetupStatus() {
  try {
    const config = await invoke('get_config');
    if (config) {
      appState.config = config;
      currentLang = config.language || 'en';
    }

    if (config && config.setupComplete) {
      appState.isSetupComplete = true;
      showMainApp();
    } else {
      showSetupWizard();
    }
  } catch (error) {
    console.error('Failed to get config:', error);
    showSetupWizard();
  }
}

// Setup Wizard 표시
function showSetupWizard() {
  document.getElementById('setup-wizard').classList.remove('hidden');
  document.getElementById('main-app').classList.add('hidden');
  initSetupWizard();
}

// Main App 표시
function showMainApp() {
  document.getElementById('setup-wizard').classList.add('hidden');
  document.getElementById('main-app').classList.remove('hidden');
  initMainApp();
}

// Setup Wizard 초기화
async function initSetupWizard() {
  const completeBtn = document.getElementById('setup-complete');
  const maxCacheInput = document.getElementById('max-cache');
  const videoFolderInput = document.getElementById('video-folder');
  const browseFolderBtn = document.getElementById('browse-folder');

  // 언어 선택 초기화
  setLanguage(currentLang, false);

  // 언어 버튼 이벤트
  document.querySelectorAll('#step-language .lang-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      setLanguage(btn.dataset.lang, false);
      appState.config.language = btn.dataset.lang;
    });
  });

  // 기본 폴더 경로 가져오기
  try {
    const defaultFolder = await invoke('get_default_video_folder');
    videoFolderInput.value = defaultFolder;
    appState.config.videoFolder = defaultFolder;
  } catch (error) {
    console.error('Failed to get default folder:', error);
  }

  // 폴더 선택
  browseFolderBtn.addEventListener('click', async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('settings.videoFolder')
      });
      if (selected) {
        videoFolderInput.value = selected;
        appState.config.videoFolder = selected;
      }
    } catch (error) {
      console.error('Failed to select folder:', error);
    }
  });

  // 캐시 용량 변경
  maxCacheInput.addEventListener('change', () => {
    appState.config.maxCacheGB = parseInt(maxCacheInput.value) || 10;
  });

  // yt-dlp 다운로드 버튼 초기화
  await initYtDlpDownload();

  // 완료 버튼
  completeBtn.addEventListener('click', async () => {
    try {
      await invoke('save_config', {
        config: {
          setupComplete: true,
          videoFolder: appState.config.videoFolder,
          maxCacheGB: appState.config.maxCacheGB,
          startMinimized: false,
          startOnBoot: false,
          language: appState.config.language
        }
      });
      appState.isSetupComplete = true;
      showMainApp();
    } catch (error) {
      console.error('Failed to save config:', error);
    }
  });
}

// yt-dlp 다운로드 버튼 초기화
async function initYtDlpDownload() {
  const downloadBtn = document.getElementById('download-ytdlp-btn');
  const progressContainer = document.getElementById('ytdlp-progress-container');
  const progressFill = document.getElementById('ytdlp-progress');
  const statusText = document.getElementById('ytdlp-status-text');
  const completeBtn = document.getElementById('setup-complete');

  // yt-dlp가 이미 있는지 확인
  try {
    const exists = await invoke('check_ytdlp_exists');

    if (exists) {
      downloadBtn.textContent = t('setup.alreadyInstalled');
      downloadBtn.disabled = true;
      downloadBtn.classList.add('success');
      completeBtn.disabled = false;
      return;
    }
  } catch (error) {
    console.error('Failed to check yt-dlp:', error);
  }

  // 다운로드 버튼 클릭 이벤트
  downloadBtn.addEventListener('click', async () => {
    downloadBtn.disabled = true;
    downloadBtn.classList.add('hidden');
    progressContainer.classList.remove('hidden');

    statusText.textContent = t('setup.downloading');
    progressFill.style.width = '30%';

    try {
      await invoke('download_ytdlp');

      progressFill.style.width = '100%';
      statusText.textContent = t('setup.downloadComplete');
      completeBtn.disabled = false;

    } catch (error) {
      statusText.textContent = t('setup.downloadFailed') + error;
      progressFill.style.width = '0%';
      downloadBtn.classList.remove('hidden');
      downloadBtn.disabled = false;
    }
  });
}

// Main App 초기화
function initMainApp() {
  setLanguage(currentLang, false);
  checkServerStatus();
  initCollapsibleSections();
  initGlobalSettings();
  initVideoServiceSettings();

  // 주기적으로 상태 확인
  setInterval(checkServerStatus, 5000);
}

// 서버 상태 확인
async function checkServerStatus() {
  const indicator = document.getElementById('status-indicator');
  const statusText = indicator.querySelector('.status-text');

  try {
    const response = await fetch(`${API_BASE}/health`);
    if (response.ok) {
      indicator.className = 'status-indicator online';
      statusText.textContent = t('status.online');
    } else {
      throw new Error('Server not responding');
    }
  } catch (error) {
    indicator.className = 'status-indicator offline';
    statusText.textContent = t('status.offline');
  }
}

// 접을 수 있는 섹션 초기화
function initCollapsibleSections() {
  // Global Settings Section
  initCollapsible('global-settings-toggle', 'global-settings-content', 'global-settings-icon');

  // Video Service Section
  initCollapsible('video-service-toggle', 'video-service-content', 'video-toggle-icon');
}

function initCollapsible(toggleId, contentId, iconId) {
  const toggle = document.getElementById(toggleId);
  const content = document.getElementById(contentId);
  const icon = document.getElementById(iconId);

  toggle.addEventListener('click', () => {
    const isHidden = content.classList.contains('hidden');
    content.classList.toggle('hidden');
    content.classList.toggle('visible', isHidden);
    icon.textContent = isHidden ? '-' : '+';
  });
}

// 전역 설정 초기화
async function initGlobalSettings() {
  const startupTrayToggle = document.getElementById('startup-tray-toggle');
  const autostartToggle = document.getElementById('startup-autostart-toggle');

  // 현재 설정 로드
  if (appState.config.startMinimized) {
    startupTrayToggle.classList.add('active');
  }
  if (appState.config.startOnBoot) {
    autostartToggle.classList.add('active');
  }

  // 토글 클릭 이벤트
  startupTrayToggle.addEventListener('click', async () => {
    const isActive = startupTrayToggle.classList.toggle('active');
    appState.config.startMinimized = isActive;

    try {
      await invoke('update_start_minimized', {
        start_minimized: isActive,
        startMinimized: isActive
      });
      showSaveStatus(t('settings.saved'));
    } catch (error) {
      console.error('Failed to update start minimized setting:', error);
      startupTrayToggle.classList.toggle('active');
      showSaveStatus(t('settings.saveFailed'), 'error');
    }
  });

  autostartToggle.addEventListener('click', async () => {
    const isActive = autostartToggle.classList.toggle('active');
    appState.config.startOnBoot = isActive;

    try {
      await invoke('update_start_on_boot', {
        start_on_boot: isActive,
        startOnBoot: isActive
      });
      showSaveStatus(t('settings.saved'));
    } catch (error) {
      console.error('Failed to update start on boot setting:', error);
      autostartToggle.classList.toggle('active');
      showSaveStatus(t('settings.saveFailed'), 'error');
    }
  });

  // 언어 버튼 이벤트 (메인 앱)
  document.querySelectorAll('#global-settings-content .lang-btn-small').forEach(btn => {
    btn.addEventListener('click', () => {
      setLanguage(btn.dataset.lang);
    });
  });
}

// Video Service 설정 초기화
async function initVideoServiceSettings() {
  const folderInput = document.getElementById('settings-video-folder');
  const browseFolderBtn = document.getElementById('settings-browse-folder');
  const cacheInput = document.getElementById('settings-max-cache');
  const saveCacheBtn = document.getElementById('save-cache-setting');
  const clearCacheBtn = document.getElementById('clear-cache');

  // 현재 설정 표시
  folderInput.value = appState.config.videoFolder || '';
  cacheInput.value = appState.config.maxCacheGB || 10;

  // 캐시 사용량 로드
  await updateCacheUsage();

  // 폴더 변경
  browseFolderBtn.addEventListener('click', async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('settings.videoFolder')
      });
      if (selected) {
        folderInput.value = selected;
        await invoke('update_video_folder', { folder: selected });
        appState.config.videoFolder = selected;
        showSaveStatus(t('settings.saved'));
      }
    } catch (error) {
      console.error('Failed to update folder:', error);
      showSaveStatus(t('settings.saveFailed'), 'error');
    }
  });

  // 캐시 용량 저장
  saveCacheBtn.addEventListener('click', async () => {
    try {
      const maxCache = parseInt(cacheInput.value) || 10;
      await invoke('update_max_cache', { maxCacheGb: maxCache });
      appState.config.maxCacheGB = maxCache;
      showSaveStatus(t('settings.saved'));
    } catch (error) {
      console.error('Failed to save cache setting:', error);
      showSaveStatus(t('settings.saveFailed'), 'error');
    }
  });

  // 캐시 비우기
  clearCacheBtn.addEventListener('click', async () => {
    if (confirm(t('confirm.clearCache'))) {
      try {
        await invoke('clear_cache');
        await updateCacheUsage();
        showSaveStatus(t('settings.cacheCleared'));
      } catch (error) {
        console.error('Failed to clear cache:', error);
        showSaveStatus(t('settings.saveFailed'), 'error');
      }
    }
  });
}

// 캐시 사용량 업데이트
async function updateCacheUsage() {
  const cacheUsage = document.getElementById('current-cache-usage');
  try {
    const usage = await invoke('get_cache_usage');
    cacheUsage.textContent = formatBytes(usage);
  } catch (error) {
    cacheUsage.textContent = t('unknown');
  }
}

// 바이트를 읽기 쉬운 형식으로 변환
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}
