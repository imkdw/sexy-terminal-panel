# INSTALL - STP 설치 지침 (AI 실행용)

이 문서는 AI 에이전트가 macOS에서 STP(Sexy Terminal Panel)를 처음부터 설치하도록 작성됐다. 위에서 아래로 순서대로 실행한다. 각 단계는 "실패 시 대처"까지 포함한다.

## 배경 (먼저 이해할 것)

STP는 macOS 일반 터미널을 가로채지 않는다. **Cursor 통합 터미널**의 기본 프로필을 `STP`로 바꾸는 확장이 핵심이다.

- 확장 `extensions/cursor`가 `terminal.profiles`에 `STP` 프로필을 기여하고, `configurationDefaults`로 `terminal.integrated.defaultProfile.osx = "STP"`를 설정한다.
- 즉 **바이너리 설치만으로는 터미널이 STP로 안 열린다.** 확장까지 Cursor에 설치돼야 한다.
- 사용자가 user settings에 `terminal.integrated.defaultProfile.osx`를 이미 지정해뒀다면 확장 기본값이 무시된다 - 설치 전 반드시 확인한다.

설치가 끝나면 산출물은 3개다: `stp` 바이너리, Cursor 확장, `stp.binaryPath` 설정.

## 0. 전제조건 점검

```sh
command -v cursor   # 없으면 중단 - Cursor CLI 필수
command -v cargo     # stp 빌드에 필요
command -v bun       # VSIX 패키징에 필요
command -v tmux      # 런타임 필수 - STP는 세션을 tmux로 관리
```

- `cargo`/`bun`은 `~/.cargo/bin`, `~/.bun/bin`에 있어도 PATH에 없을 수 있다. 없으면 아래 1-a/1-b로 설치.
- `cursor`가 없으면 진행 불가. 사용자에게 Cursor 설치를 요청한다.
- `tmux`가 없으면 빌드/설치는 되지만 **터미널 실행 시 `failed to spawn tmux ... No such file or directory`로 실패**한다. 아래 1-c로 설치.

### 1-c. tmux 설치 (tmux 없을 때)

```sh
brew install tmux
```

### 1-a. Rust 설치 (cargo 없을 때)

`rust-toolchain.toml`이 rustup 관리 채널(`stable`)을 요구하므로 **rustup으로 설치**한다 (brew rust는 toolchain 파일을 존중하지 않음).

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
. "$HOME/.cargo/env"   # 현재 셸에 PATH 반영
```

### 1-b. bun 설치 (bun 없을 때)

```sh
curl -fsSL https://bun.sh/install | bash
export PATH="$HOME/.bun/bin:$PATH"   # 현재 셸에 PATH 반영
```

## 2. 바이너리 빌드

```sh
cd <repo-root>
. "$HOME/.cargo/env"
cargo build --release -p stp
```

- **컴파일 에러가 나면** 소스 문제이므로 실제 원인을 고친다 (예: 타입 추론 실패 `E0689` → 바인딩에 타입 명시). 우회하지 말 것.

## 3. 바이너리 설치

`/usr/local/bin`은 보통 쓰기 권한이 없다(sudo 필요). sudo 프롬프트를 피하려면 `~/.local/bin`에 설치하고 4단계에서 절대경로를 등록한다.

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/stp "$HOME/.local/bin/stp"
"$HOME/.local/bin/stp" --help   # 동작 확인
```

- 시스템 전역(`/usr/local/bin`)을 원하면: `sudo install -m 755 target/release/stp /usr/local/bin/stp`.

## 4. 확장 패키징 & 설치

```sh
cd extensions/cursor
export PATH="$HOME/.bun/bin:$PATH"
bun install           # vsce 등 devDependencies 설치 (재패키징에 필수)
bun run compile       # src -> dist/extension.js 최신화
bun run package-vsix  # sexy-terminal-panel-cursor-<version>.vsix 생성
cursor --install-extension "$PWD/sexy-terminal-panel-cursor-0.1.0.vsix" --force
```

- `vsce: command not found` → `bun install`을 건너뛴 것. devDependencies가 있어야 한다.
- `bun: command not found`가 prepublish에서 나면 → 현재 셸 PATH에 bun이 없는 것. 1-b의 `export PATH` 재실행.
- 급하면 리포에 커밋된 기존 `.vsix`로 바로 설치해도 되지만, 최신 소스 반영을 위해 재패키징을 권장.

## 5. Cursor 설정 (GUI PATH 문제 대응)

macOS의 Cursor GUI는 로그인 셸 PATH를 상속하지 않아 `~/.local/bin`의 `stp`를 못 찾을 수 있다. **바이너리 절대경로를 user settings에 등록**한다.

파일: `~/Library/Application Support/Cursor/User/settings.json` (JSONC - 주석/트레일링 콤마 허용)

```json
"stp.binaryPath": "/Users/<user>/.local/bin/stp"
```

동시에 다음 키가 **이미 존재하면 삭제하거나 STP로 바꾼다** (없으면 확장 기본값이 적용되므로 그대로 둔다):

```json
"terminal.integrated.defaultProfile.osx": "STP"
```

## 6. 검증

1. Cursor 재시작 또는 `Cmd+Shift+P` → **Reload Window** (확장 활성화 + configurationDefaults 반영에 필요).
2. 새 통합 터미널 열기 → 프로필 이름이 **STP**로 뜨는지 확인.
3. Explorer에 **STP Terminals** 뷰가 보이는지 확인.
4. 안 뜨면: 터미널 패널 `+` 옆 `⌄` 드롭다운에 "STP"가 있는지 확인 → 있으면 5단계의 `defaultProfile.osx` 오버라이드 점검, 프로필 선택 시 에러가 나면 `stp.binaryPath` 경로 점검.

## 요약 체크리스트

- [ ] cursor CLI 존재
- [ ] cargo / bun 존재 (없으면 1-a / 1-b)
- [ ] `cargo build --release -p stp` 성공
- [ ] `~/.local/bin/stp` 설치 및 `--help` 동작
- [ ] `bun install && bun run compile && bun run package-vsix` 성공
- [ ] `cursor --install-extension ... --force` 성공 메시지
- [ ] settings.json에 `stp.binaryPath` 등록, `defaultProfile.osx` 오버라이드 없음
- [ ] Cursor 재시작 후 새 터미널이 STP로 열림
