<img width="2208" height="512" alt="ivLyrics English banner" src="https://github.com/ivLis-Studio/ivLyrics/blob/main/preview_readme_en.png?raw=true" /><br>
PLATFORM : <a href="https://github.com/ivLis-Studio/ivLyrics">PC</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-Android">Android</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-IOS">iOS</a><br>
MODULE : <a href="https://github.com/ivLis-Studio/ivLyrics-overlay">Overlay</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-helper">Helper</a>

# ivLyrics Helper

[한국어](README.md) | [English](README_EN.md)

A desktop companion for [ivLyrics for PC](https://github.com/ivLis-Studio/ivLyrics). Use it to download video backgrounds for local playback, or to share the current song's lyrics and playback information with other programs.

Install it if you need either feature. It is optional for viewing lyrics inside Spotify. For a separate lyrics window, use [ivLyrics Overlay](https://github.com/ivLis-Studio/ivLyrics-overlay), which works without Helper.

## When should I use it?

- **Video backgrounds:** Download YouTube videos requested by ivLyrics for PC and serve the saved files for local playback. Cached copies are reused for the same video and quality. You do not search for videos or paste URLs into Helper itself.
- **Lyrics in other programs:** Provide track information, full lyrics, the current line, and playback position received from ivLyrics for PC through a local API. Displaying this information requires another program that uses the API. Helper itself is not a lyrics display window.

Enable each feature separately in ivLyrics for PC. Launching Helper alone does not enable both features.

## Download and requirements

[Download the latest version](https://github.com/ivLis-Studio/ivLyrics-helper/releases/latest) · [Release history](https://github.com/ivLis-Studio/ivLyrics-helper/releases)

Under **Assets** on the release page, download one installer for your computer.

| Computer | Installer | How to install |
| --- | --- | --- |
| Windows x64 | `ivLyrics.Helper_…_x64-setup.exe` or `…_x64_en-US.msi` | Run the installer and follow its instructions. |
| macOS Apple Silicon | `ivLyrics.Helper_…_aarch64.dmg` | Open the DMG, copy the app to Applications, and launch it. |

The latest public release does not include installers for Intel Macs, Linux, or mobile devices. `latest.json`, `.sig`, and `.app.tar.gz` files are for updates; use an installer above for a normal installation.

Install Spotify desktop and ivLyrics for PC on the same computer. See the [PC guide](https://github.com/ivLis-Studio/ivLyrics/blob/main/README_EN.md) for ivLyrics installation. Initial setup and new video downloads require an internet connection. The Windows installer downloads WebView2 if needed. You do not need Node.js or Rust to use the released Helper app.

## First launch

1. Launch **ivLyrics Helper** and choose **English** or **한국어**.
2. Wait for **yt-dlp Download** to finish. It starts automatically on first launch. If it fails, check your internet connection and retry with **Download yt-dlp**. The current setup requires this check even if you only want lyrics integration.
3. Under **Storage Settings**, choose **Video Save Folder** and **Max Cache Size (GB)**. The default cache limit is 10GB. Use a folder dedicated to Helper, since cache cleanup can delete files inside it.
4. Click **Complete Setup** and check that the top status reads **Server running**. The connection address is `http://localhost:15123`.
5. If you changed the video folder during setup, fully exit Helper with **Quit** in its tray menu and launch it again. The folder used for downloading and serving videos is applied at app startup.

Open the tray menu from the app icon in the Windows notification area or macOS menu bar. Helper's **Show Window** and **Quit** tray items, and the **Video Background Service** and **Lyrics Service** headings, remain in English even when Korean is selected.

## Connect to ivLyrics for PC

Keep Helper and Spotify running on the same computer, then enable the feature you need. There is no setting for connecting a phone or another computer to this Helper.

### For video backgrounds

1. In ivLyrics inside Spotify, open **Settings → Appearance**.
2. Select **Video Background** as the background and turn on **Use Helper Program**.
3. Check that the setting shows **✓ Connected**, then play a song with an available video. A video requested for the first time must finish downloading before it can play.

Downloaded videos have no audio. Spotify continues to play the music. For songs without an available or playable video, ivLyrics for PC uses the configured fallback background. Helper cannot guarantee that every YouTube video can be downloaded or played.

### For lyrics in other programs

1. In ivLyrics for PC, open **Settings → Behavior → Helper Integration**.
2. Turn on **Use Lyrics Helper** and check for **✓ Connected**.
3. Play a song and load its lyrics in ivLyrics. Configure the program you want to connect to read the Helper API on the same computer.

**Use Helper Program** for video and **Use Lyrics Helper** for lyrics are separate settings. They are also separate from Overlay's **Enable Desktop Overlay**. Helper uses the fixed port `15123`; there is no port setting in its interface.

<details>
<summary>Connect another program: local lyrics API</summary>

The base address is `http://localhost:15123`. These endpoints read data:

| Request | Information returned |
| --- | --- |
| `GET /lyrics/getfull` | Current track information, full lyrics, and synced status |
| `GET /lyrics/getnow` | One lyric line corresponding to the received playback position |
| `GET /lyrics/progress` | Playback state, including position and whether playback is active |

Playback position (`position`) and lyric timestamps (`startTime`/`endTime`) are in milliseconds. Responses are `null` when the required data has not arrived. ivLyrics for PC must keep sending lyrics and playback position; after Helper exits, the data must be sent again. See the [lyrics API source](src-tauri/src/lyrics_server.rs) for request and response formats and receiving endpoints. Video integration formats are in the [video API source](src-tauri/src/video_server.rs).

</details>

## Storage and startup settings

Expand **Video Background Service** in Helper to adjust these settings:

- **Video Save Folder:** Use **Change** to select a folder. Exit through **Quit** in the tray and relaunch Helper to apply it to downloads and playback. Existing cached files are not moved automatically.
- **Maximum Video Quality:** The default is 1080p. Choose best available, 2160p, 1440p, 1080p, 720p, or 480p. This caps the resolution of new downloads; actual quality depends on the available video. Each quality has its own cache, so changing quality may download the same video again.
- **Max Cache Size (GB):** Change the value and click **Save**. After a download, files are removed oldest first if the cache exceeds the limit.
- **Clear Cache:** After confirmation, deletes files in the current video folder. Move any files you need to keep elsewhere first. Requesting a video again downloads it again.

Expand **Settings** to change the language or **Start minimized to tray**. **Launch on system startup** is currently supported only on Windows. Closing the window hides it without exiting the app. Use **Show Window** in the tray to reopen it, or **Quit** to fully exit.

## Updates

Update the Helper app by downloading a new installer from the [latest release](https://github.com/ivLis-Studio/ivLyrics-helper/releases/latest). Select **Quit** in the tray, install the new version, and relaunch it. The current interface has no button for installing app updates.

Helper checks for the latest yt-dlp version at startup and downloads it when needed. On Windows, it may also automatically download the Deno runtime used alongside the video downloader.

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| Initial setup cannot finish. | Check the yt-dlp download status and that your internet connection can reach GitHub, then retry the download. |
| Server connection failed, or the PC setting shows Disconnected. | Check that Helper is running on the same computer. Check whether another program or another Helper instance is using port `15123`, then relaunch. Enable the individual feature you want in ivLyrics for PC. |
| Connected, but no video appears. | Check that the song has a video, the download has completed, and internet access and disk space are available. If you changed the video folder, fully exit and relaunch Helper. |
| A video reports a login or age verification error. | See the cookie guidance below. Video visibility, region or account restrictions, or changes to YouTube can still prevent a download. |
| The lyrics API has no data or still shows the previous song. | Check **Use Lyrics Helper** and its connection status, then reload the current song's lyrics in ivLyrics for PC. Helper retains received data, so a running server alone does not confirm that current lyrics have arrived. |
| The startup setting cannot be saved on macOS. | **Launch on system startup** is currently Windows only. Launch Helper manually on macOS. |

## Data, cookies, and limitations

Helper runs at `127.0.0.1:15123` on the same computer. Received lyrics and playback state are kept in memory while it runs; settings, cached videos, and download tools are saved on your computer. The local API has no separate authentication and does not restrict request origins, so do not expose its port outside your computer.

New video downloads contact YouTube, while tool and app update checks contact GitHub. Reusing a saved video does not mean Spotify and ivLyrics for PC also work offline.

When a video reports that login is required, Helper may retry using a registered cookie file or cookies from installed browsers. To register a file yourself, open **Video Background Service → Age-Restricted Videos → Cookies File (cookies.txt) → Register** and select an exported `.txt` file. The interface includes a link with export instructions. Cookie files can contain login information; do not share them or attach them to support requests.

The registered file is copied to `youtube_cookie.txt` in the app data folder. Clicking **✕** beside the registration removes this copy and its setting, while your original exported file remains. Registering cookies does not guarantee video access or successful downloads.

## Support and license

Contact us on [Discord](https://discord.gg/2fu36fUzdE) or [GitHub Issues](https://github.com/ivLis-Studio/ivLyrics-helper/issues). Include your operating system and CPU type, Helper and ivLyrics for PC versions, the feature used, the song or video, the error message, and steps to reproduce the issue. Do not attach cookie files.

[ivLyrics for PC](https://github.com/ivLis-Studio/ivLyrics) · [Android](https://github.com/ivLis-Studio/ivLyrics-Android) · [iOS](https://github.com/ivLis-Studio/ivLyrics-IOS) · [ivLyrics Overlay](https://github.com/ivLis-Studio/ivLyrics-overlay)

This repository is licensed under [GNU LGPL v2.1](LICENSE). The app uses [Tauri](https://tauri.app/), with [yt-dlp](https://github.com/yt-dlp/yt-dlp) and [Deno for Windows](https://github.com/denoland/deno) for video downloads. Refer to each project for its licenses and notices.
