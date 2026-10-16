<img width="2208" height="512" alt="ivLyrics 한국어 배너" src="https://github.com/ivLis-Studio/ivLyrics/blob/main/preview_readme_ko.png?raw=true" /><br>
PLATFORM : <a href="https://github.com/ivLis-Studio/ivLyrics">PC</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-Android">Android</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-IOS">iOS</a><br>
MODULE : <a href="https://github.com/ivLis-Studio/ivLyrics-overlay">Overlay</a> | <a href="https://github.com/ivLis-Studio/ivLyrics-helper">Helper</a>

# ivLyrics Helper

[한국어](README.md) | [English](README_EN.md)

[PC용 ivLyrics](https://github.com/ivLis-Studio/ivLyrics)의 동영상 배경을 컴퓨터에 다운로드하여 재생하거나, 현재 곡의 가사와 재생 정보를 다른 프로그램에 전달할 때 사용하는 데스크탑 보조 앱입니다.

두 기능 중 필요한 것이 있을 때 선택해서 설치하세요. Spotify 안에서 가사를 보는 데 필수는 아닙니다. 별도 가사 창을 띄우고 싶다면 [ivLyrics Overlay](https://github.com/ivLis-Studio/ivLyrics-overlay)를 사용하세요. Overlay는 Helper 없이 사용할 수 있습니다.

## 어떤 경우에 사용하나요?

- **동영상 배경:** PC용 ivLyrics가 요청한 YouTube 영상을 다운로드하고, 저장한 파일을 로컬에서 재생하도록 제공합니다. 같은 영상과 화질의 캐시가 있으면 다시 사용합니다. Helper 자체에서 영상을 검색하거나 URL을 붙여 넣는 방식은 아닙니다.
- **다른 프로그램과 가사 연동:** PC용 ivLyrics에서 받은 곡 정보·전체 가사·현재 가사·재생 위치를 로컬 API로 제공합니다. 이 정보를 표시하려면 해당 API를 사용하는 별도 프로그램이 필요합니다. Helper 자체는 가사 표시 창이 아닙니다.

두 기능은 PC용 ivLyrics에서 각각 켤 수 있습니다. Helper를 실행하는 것만으로 두 기능이 모두 활성화되지는 않습니다.

## 다운로드와 준비 사항

[최신 버전 다운로드](https://github.com/ivLis-Studio/ivLyrics-helper/releases/latest) · [릴리스 기록](https://github.com/ivLis-Studio/ivLyrics-helper/releases)

릴리스의 **Assets**에서 컴퓨터에 맞는 설치 파일 하나를 받으세요.

| 컴퓨터 | 설치 파일 | 설치 방법 |
| --- | --- | --- |
| Windows x64 | `ivLyrics.Helper_…_x64-setup.exe` 또는 `…_x64_en-US.msi` | 설치 파일을 실행하고 안내를 따르세요. |
| macOS Apple Silicon | `ivLyrics.Helper_…_aarch64.dmg` | DMG를 열어 앱을 응용 프로그램 폴더로 복사한 뒤 실행하세요. |

최신 공개 릴리스에는 Intel Mac·Linux·모바일용 설치 파일이 없습니다. `latest.json`, `.sig`, `.app.tar.gz`는 업데이트용 파일이므로 일반 설치에는 위 설치 파일을 사용하세요.

Spotify 데스크탑 앱과 PC용 ivLyrics를 같은 컴퓨터에 설치하세요. ivLyrics 설치는 [PC 안내](https://github.com/ivLis-Studio/ivLyrics#readme)를 참고하세요. 초기 설정과 새 영상 다운로드에는 인터넷 연결이 필요합니다. Windows 설치 프로그램은 필요한 경우 WebView2를 다운로드합니다. 배포된 Helper를 사용하는 데 Node.js나 Rust는 필요하지 않습니다.

## 처음 실행하기

1. **ivLyrics Helper**를 실행하고 **한국어** 또는 **English**를 선택하세요.
2. **yt-dlp 다운로드**가 완료될 때까지 기다리세요. 처음에는 자동으로 시작하며, 실패하면 인터넷 연결을 확인한 뒤 같은 버튼으로 다시 시도하세요. 가사 연동만 사용할 때도 현재 초기 설정에서는 이 확인을 완료해야 합니다.
3. **저장 설정**에서 **영상 저장 폴더**와 **최대 캐시 용량 (GB)**을 정하세요. 기본 캐시 한도는 10GB입니다. 캐시 정리 시 폴더 안의 파일이 삭제될 수 있으므로 Helper 전용 폴더를 사용하세요.
4. **설정 완료**를 누르고 상단에 **서버 실행중**이 표시되는지 확인하세요. 연결 주소는 `http://localhost:15123`입니다.
5. 초기 설정에서 영상 저장 폴더를 바꿨다면, 트레이 메뉴의 **Quit**로 Helper를 완전히 종료한 뒤 다시 실행하세요. 다운로드와 재생에 사용할 폴더는 앱을 시작할 때 적용됩니다.

트레이 메뉴는 Windows 알림 영역이나 macOS 메뉴 막대의 앱 아이콘에서 엽니다. Helper의 트레이 항목 **Show Window**, **Quit**와 화면의 **Video Background Service**, **Lyrics Service** 제목은 한국어 설정에서도 영어로 표시됩니다.

## PC용 ivLyrics와 연결하기

Helper와 Spotify를 같은 컴퓨터에서 실행한 상태로 필요한 기능을 켜세요. 휴대폰이나 다른 컴퓨터에서 이 Helper로 연결하는 설정은 제공하지 않습니다.

### 동영상 배경을 사용할 때

1. Spotify의 ivLyrics에서 **설정 → 외관**을 여세요.
2. 배경으로 **동영상 배경**을 선택하고 **헬퍼 프로그램 사용**을 켜세요.
3. 해당 항목에 **✓ 연결됨**이 표시되는지 확인한 뒤, 동영상이 등록된 곡을 재생하세요. 처음 요청하는 영상은 다운로드가 끝나야 재생할 수 있습니다.

다운로드한 영상에는 소리가 포함되지 않습니다. 음악은 Spotify에서 재생합니다. 영상이 없거나 사용할 수 없는 곡에는 PC용 ivLyrics에서 지정한 대체 배경이 표시됩니다. Helper를 사용해도 모든 YouTube 영상의 다운로드나 재생이 보장되지는 않습니다.

### 다른 프로그램에서 가사를 사용할 때

1. PC용 ivLyrics의 **설정 → 동작 → 헬퍼 연동**을 여세요.
2. **가사 헬퍼 사용**을 켜고 **✓ 연결됨**을 확인하세요.
3. 노래를 재생하고 ivLyrics에서 가사를 불러오세요. 연동할 프로그램이 같은 컴퓨터의 Helper API를 읽도록 설정하세요.

동영상용 **헬퍼 프로그램 사용**과 가사용 **가사 헬퍼 사용**은 별도 설정입니다. Overlay의 **데스크탑 오버레이 사용**과도 다릅니다. Helper의 연결 포트는 `15123`으로 고정되어 있으며, 화면에서 바꾸는 항목은 없습니다.

<details>
<summary>다른 프로그램과 연동하기: 로컬 가사 API</summary>

기본 주소는 `http://localhost:15123`입니다. 다음 주소는 데이터를 읽는 용도입니다.

| 요청 | 제공하는 정보 |
| --- | --- |
| `GET /lyrics/getfull` | 현재 곡 정보, 전체 가사, 동기화 여부 |
| `GET /lyrics/getnow` | 전달받은 재생 위치에 해당하는 가사 한 줄 |
| `GET /lyrics/progress` | 재생 위치와 재생 여부 등 재생 상태 |

재생 위치(`position`)와 가사 시각(`startTime`/`endTime`)은 밀리초 단위입니다. 데이터가 아직 없으면 `null`이 반환됩니다. 재생 위치와 가사는 PC용 ivLyrics에서 계속 전달되어야 하며, Helper 종료 후에는 다시 받아야 합니다. 요청·응답 형식과 수신용 주소는 [가사 API 소스](src-tauri/src/lyrics_server.rs)를 참고하세요. 영상 연동 형식은 [영상 API 소스](src-tauri/src/video_server.rs)에서 확인할 수 있습니다.

</details>

## 저장 공간과 실행 설정

Helper에서 **Video Background Service**를 펼치면 다음 설정을 바꿀 수 있습니다.

- **영상 저장 폴더:** **변경**으로 새 폴더를 선택하세요. 선택 후에는 트레이의 **Quit**로 종료하고 다시 실행해야 다운로드·재생에 반영됩니다. 기존 캐시 파일은 자동으로 옮겨지지 않습니다.
- **최대 영상 화질:** 기본값은 1080p이며, 사용 가능한 최고 화질·2160p·1440p·1080p·720p·480p 중에서 선택할 수 있습니다. 새 다운로드의 해상도 상한이며 실제 화질은 제공되는 영상에 따라 다릅니다. 화질별로 캐시를 따로 사용하므로 설정을 바꾸면 같은 영상도 다시 다운로드할 수 있습니다.
- **최대 캐시 용량 (GB):** 값을 바꾼 뒤 **저장**을 누르세요. 다운로드 후 용량이 한도를 넘으면 오래된 파일부터 정리합니다.
- **캐시 비우기:** 확인 후 현재 영상 저장 폴더 안의 파일을 삭제합니다. 필요한 파일은 미리 다른 곳에 보관하세요. 이후 영상을 다시 요청하면 재다운로드합니다.

**설정**을 펼치면 언어와 **시작 시 트레이 아이콘으로 실행**을 바꿀 수 있습니다. **컴퓨터 시작 시 자동 실행**은 현재 Windows에서만 지원합니다. 창의 닫기 버튼은 앱을 종료하지 않고 숨깁니다. 다시 열려면 트레이의 **Show Window**, 완전히 종료하려면 **Quit**를 선택하세요.

## 업데이트

Helper 앱은 [최신 릴리스](https://github.com/ivLis-Studio/ivLyrics-helper/releases/latest)에서 새 설치 파일을 받아 업데이트하세요. 트레이의 **Quit**로 종료하고 새 버전을 설치한 뒤 다시 실행하세요. 현재 화면에는 앱 업데이트를 실행하는 버튼이 없습니다.

영상 다운로드에 사용하는 yt-dlp는 Helper가 시작될 때 최신 버전을 확인하고 필요한 경우 다운로드합니다. Windows에서는 보조 런타임인 Deno도 자동으로 다운로드할 수 있습니다.

## 문제가 있을 때

| 증상 | 확인할 내용 |
| --- | --- |
| 초기 설정을 완료할 수 없습니다. | yt-dlp 다운로드 상태와 GitHub에 접근할 수 있는 인터넷 연결을 확인하고 다운로드를 다시 시도하세요. |
| 서버 연결 실패 또는 PC에서 연결 안됨으로 표시됩니다. | Helper가 같은 컴퓨터에서 실행 중인지 확인하세요. `15123` 포트를 다른 프로그램이나 Helper의 다른 실행 창이 사용 중인지 확인한 뒤 다시 실행하세요. PC에서 사용하려는 기능의 스위치도 각각 켜야 합니다. |
| 연결은 되었지만 영상이 나오지 않습니다. | 해당 곡에 동영상이 있는지, 다운로드가 완료되었는지, 인터넷과 디스크 공간이 충분한지 확인하세요. 저장 폴더를 바꿨다면 Helper를 완전히 종료하고 다시 실행하세요. |
| 특정 영상에서 로그인·성인 인증 오류가 납니다. | 아래 Cookie 안내를 참고하세요. 영상의 공개 범위·지역·계정 제한이나 YouTube의 변경으로 계속 실패할 수 있습니다. |
| 가사 API에 데이터가 없거나 이전 곡이 남아 있습니다. | **가사 헬퍼 사용**과 연결 상태를 확인하고 PC용 ivLyrics에서 현재 곡의 가사를 다시 불러오세요. Helper는 받은 데이터를 보관하므로 실행 상태만으로 최신 가사 수신을 확인할 수는 없습니다. |
| macOS에서 자동 실행 설정이 저장되지 않습니다. | 현재 **컴퓨터 시작 시 자동 실행**은 Windows 전용입니다. macOS에서는 Helper를 직접 실행하세요. |

## 데이터, Cookie와 제한 사항

Helper는 같은 컴퓨터의 `127.0.0.1:15123`에서 동작합니다. 전달받은 가사와 재생 상태는 실행 중 메모리에 보관하며, 설정·영상 캐시·다운로드 도구는 컴퓨터에 저장합니다. 로컬 API에는 별도 인증이 없고 요청 출처를 제한하지 않으므로 포트를 외부에 공개하지 마세요.

새 영상 다운로드는 YouTube에, 도구와 앱 업데이트 확인은 GitHub에 연결합니다. 저장한 영상을 다시 사용할 수 있어도 Spotify와 PC용 ivLyrics까지 오프라인으로 동작한다는 의미는 아닙니다.

로그인이 필요한 영상에서 오류가 나면 등록한 Cookie 파일이나 설치된 브라우저의 Cookie로 재시도할 수 있습니다. Cookie 파일을 직접 등록하려면 Helper의 **Video Background Service → 성인 인증 영상 → 쿠키 파일 (cookies.txt) → 등록**에서 내보낸 `.txt` 파일을 선택하세요. 화면에 내보내기 도움말 링크가 있습니다. 로그인 정보가 담길 수 있으므로 Cookie 파일을 다른 사람에게 공유하거나 문의에 첨부하지 마세요.

등록한 파일은 앱 데이터 폴더의 `youtube_cookie.txt`로 복사됩니다. 등록 옆 **✕**를 누르면 이 복사본과 등록 설정을 삭제하며, 직접 내보낸 원본 파일은 그대로 남습니다. Cookie 등록이 영상 접근이나 다운로드 성공을 보장하지는 않습니다.

## 문의와 라이선스

[Discord](https://discord.gg/2fu36fUzdE) 또는 [GitHub Issues](https://github.com/ivLis-Studio/ivLyrics-helper/issues)로 문의하세요. 운영체제와 CPU 종류, Helper·PC용 ivLyrics 버전, 사용한 기능, 곡 또는 영상, 오류 메시지와 재현 절차를 알려주세요. Cookie 파일은 첨부하지 마세요.

[PC용 ivLyrics](https://github.com/ivLis-Studio/ivLyrics) · [Android](https://github.com/ivLis-Studio/ivLyrics-Android) · [iOS](https://github.com/ivLis-Studio/ivLyrics-IOS) · [ivLyrics Overlay](https://github.com/ivLis-Studio/ivLyrics-overlay)

이 저장소의 라이선스는 [GNU LGPL v2.1](LICENSE)입니다. 앱은 [Tauri](https://tauri.app/)를 사용하며, 영상 다운로드에는 [yt-dlp](https://github.com/yt-dlp/yt-dlp)와 Windows용 [Deno](https://github.com/denoland/deno)를 사용합니다. 각 프로젝트의 라이선스와 고지는 해당 프로젝트에서 확인하세요.
