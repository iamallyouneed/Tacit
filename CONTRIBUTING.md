# 기여하기 · Contributing

[한국어](#한국어) · [English](#english)

---

## 한국어

Tacit은 **Necessity**의 프로젝트로 운영되는 오픈소스입니다. 라이선스는 [AGPL-3.0-only](LICENSE)입니다.
버그 제보, 문서, 코드, 언어 데이터 모두 환영합니다. 큰 변경은 먼저 이슈로 이야기해 주세요.

### 기여 조건 (DCO)

별도의 계약서는 없고 [Developer Certificate of Origin 1.1](https://developercertificate.org/)로 확인합니다.
"이 코드는 내가 썼거나 낼 권리가 있고, 프로젝트의 라이선스로 배포되는 데 동의한다"는 뜻입니다. 모든 커밋에
서명 줄을 붙여 주세요.

```sh
git commit -s -m "fix: ..."     # Signed-off-by: 이름 <이메일> 이 붙습니다
```

- 기여한 코드는 AGPL-3.0-only로 배포됩니다. 라이선스를 다시 바꾸려면 기여자 동의가 필요합니다.
- AGPL은 수정한 프로그램을 네트워크로 사용하게 할 때도 소스를 제공하라고 요구합니다. 대시보드는 이 PC에서만
  열리지만, 이 코드를 서비스로 돌린다면 해당 조항이 적용됩니다.

### 개발 환경과 빌드·테스트

Rust stable 1.89 이상이 필요합니다. 모든 시스템에서 공통으로 이 셋이 통과해야 합니다.

Android APK를 빌드한 뒤 실행 중인 에뮬레이터·기기에서 JNI를 검사하려면
`./platforms/android/smoke.ps1 -Serial emulator-5554`를 실행합니다. 생성·활성화,
영·한 입력, 일시 정지·재개와 해제를 검사하며 시스템 입력기 선택은 바꾸지 않습니다.

키보드 UI는 격리된 에뮬레이터에서 `./platforms/android/ui-smoke.ps1 -Serial emulator-5560`으로
검사합니다. 테스트 APK를 설치하고 Tacit을 잠시 선택한 뒤 기존 입력기를 복원하며, 실제
조합·클립보드·선택 영역·대문자 고정·Enter·커서 전달을 검증합니다. 테스트용 편집 화면은
`io.github.iamallyouneed.tacit.tests/io.github.iamallyouneed.tacit.KeyboardTestActivity`입니다.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Linux와 macOS에서는 Windows 전용 텍스트 서비스를 빼고 돌립니다: `--workspace --exclude tacit-tip`.

| 시스템 | 빌드 |
| --- | --- |
| Windows | `cargo build --release -p tacit-tip -p tacit-config` (`tacit-cli`는 개발용이라 같이 빌드하지 않습니다: 내장 팩이 다른 바이너리에도 들어가 커집니다) (x86은 `rustup target add i686-pc-windows-msvc`), 설치는 `.\scripts\install.ps1` (관리자 권한 한 번) |
| Android | `.\platforms\android\build.ps1` (Android SDK·NDK, JDK 17, `cargo install cargo-ndk`) |
| macOS | `sh platforms/macos/build.sh` (Xcode 명령줄 도구, `rustup target add aarch64-apple-darwin x86_64-apple-darwin`) |
| Linux | `sudo apt install libibus-1.0-dev pkg-config` 후 `cargo build --release -p tacit-ffi && make -C platforms/linux` |

- **웹 대시보드**는 `crates/config/src/web/`(마크업, 스타일, 스크립트)에 있고 컴파일 때 한 페이지로 합쳐집니다.
  내 설정을 건드리지 않고 시험하려면 다음처럼 실행하세요.
  `TACIT_HOME=/tmp/dash TACIT_NO_BROWSER=1 cargo run -p tacit-config`
- **골든 테스트**(`crates/core/tests/golden/`)는 영어·한국어·일본어 판별 결과의 스냅샷입니다. 동작을 바꾸려는 게
  아니라면 이 파일이 달라져서는 안 되고, 바꿔야 한다면 PR에서 이유를 설명해 주세요. Linux·macOS에서는 부동소수점
  표현 차이로 숫자만 1e-6까지 허용하고, Windows에서는 바이트 단위로 같아야 합니다.
- en/ko/ja의 정확도는 떨어지면 안 됩니다. `crates/core/tests/accuracy.rs`가 이를 지킵니다.

### PR과 커밋 규칙

- 브랜치는 `feat/…`, `fix/…`처럼 짓고, PR은 작게 나눠 주세요. 하나의 PR은 하나의 주제입니다.
- **테스트를 먼저** 쓰고 실패하는 것을 확인한 뒤 구현해 주세요. 버그 수정에는 재현 테스트가 들어갑니다.
- 커밋 메시지는 영어로 `type: 설명`입니다. type은 `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci` 중
  하나이고, 본문에는 *왜* 바꿨는지를 적습니다.
- 사용자에게 보이는 변경은 [CHANGELOG.md](CHANGELOG.md)의 `[Unreleased]`에 한 줄 추가해 주세요.
- 푸시하기 전에 위 세 명령이 통과해야 하고, CI(Windows, Android, Linux, macOS)도 통과해야 머지합니다.

### 변경 기록

입력 위치 오류·글자 유실·입력 불능·크래시 등 사용자 작업을 방해하는 버그의 긴급 배포는 [퀵패치 절차](docs/quick-patch.md)를 따릅니다. 플랫폼 한정 수정의 로컬 검증·성능 측정 예외도 이 절차에 명시되어 있습니다.

**버전 기준.** 팩과 프로그램의 호환은 마이너 번호로 판단합니다. 팩 파일 형식이나 팩–프로그램 호환 범위처럼 이전 프로그램이 읽지 못하게 되는 변경만 마이너(`0.x.0`)를 올리고, 형식을 바꾸지 않는 기능·수정은 모두 패치(`0.8.x`)입니다. 1.0 이전에는 메이저를 쓰지 않습니다. 커밋은 `main`에 쌓아 두고, 릴리스는 필요할 때 한 번에 합니다.

`CHANGELOG.md`는 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/) 형식입니다. 항목은 `추가`, `변경`,
`수정`(필요하면 `제거`, `보안`) 아래에 사용자 관점으로 한 줄씩 씁니다. 릴리스 담당자는 이 순서로 합니다.

1. `[Unreleased]` 아래 내용을 `## [X.Y.Z] - 날짜`로 옮기고 아래쪽 비교 링크를 갱신합니다.
2. 버전을 올립니다: 각 `Cargo.toml`(7곳), `Cargo.lock`, README의 예시 버전.
   패키징한 뒤(`scripts/package.ps1`) `sh lab/report.sh --profile enko`와 `sh lab/report.sh --profile enkoja`로 각각 측정합니다.
   출력을 [영·한](docs/report/performance.md)과 [영·한·일](docs/report/performance-enkoja.md) 문서 맨 위에 따로 붙입니다. 출력은 같은 프로필의 `## 한눈에`(직전 → 이번 비교, 이전 블록을 갈아끼움)와 새 버전 절이며, 각 명령에 `--selftest`를 붙여 숫자를 확인합니다.
3. `v` 태그(`git tag vX.Y.Z && git push origin vX.Y.Z`)를 푸시하면 릴리스 워크플로가 해당 버전의 항목을
   릴리스 노트로 올립니다. `python scripts/validate-changelog.py --selftest --tag vX.Y.Z`로 영·한 버전·날짜·문자·항목을 검사합니다. 검사 실패나 빈 노트는 공개를 중단합니다.
   (`sh scripts/release-notes.sh vX.Y.Z`로 미리 볼 수 있습니다.)

### 언어와 사전 데이터 기여

- **새 언어**는 `crates/core/src/lang/<id>/` 모듈과 `registry.rs`의 한 줄, 그리고 `crates/packgen`의 팩 정의로
  들어갑니다. 팩은 `cargo run -p tacit-packgen --bin build-packs -- crates/core/data target/packs`로 만듭니다.
- **데이터의 출처와 라이선스**를 [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt)에 적어 주세요. 라이선스를
  모르는 데이터, 재배포가 막힌 데이터는 받을 수 없습니다.
- CC BY-SA 같은 데이터는 코드와 별개로 **원래 라이선스를 유지**합니다. 데이터는 코드와 섞지 말고 `crates/core/data`
  의 별도 파일로 두세요.
- 기존 언어의 판별이 달라지는 변경은 `accuracy`·골든 테스트로 나아지는 점과 나빠지는 점을 함께 보여 주세요.

### 버그와 보안 제보

- **버그**는 [이슈](https://github.com/iamallyouneed/tacit/issues)로 알려 주세요. 입력기는 환경에 따라
  달라서 이것들이 있으면 재현이 빨라집니다: 시스템과 버전(Windows·Android·macOS·Linux), 입력기 종류와 Tacit
  버전, 입력하던 **앱 이름**, 친 **키 순서**, 기대한 결과와 실제 결과, 가능하면 대시보드 체험 탭의 결과.
- **보안 취약점**은 공개 이슈에 올리지 말고 GitHub의
  [비공개 보안 권고](https://github.com/iamallyouneed/tacit/security/advisories/new)로 보내 주세요.

### 기여자

- [<img src="https://github.com/noth3dev.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="noth3dev"> **@noth3dev**](https://github.com/noth3dev) (Necessity) — 프로젝트 운영과 핵심 개발
- [<img src="https://github.com/Ddongddi.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="Ddongddi"> **@Ddongddi**](https://github.com/Ddongddi) — 일본어 지원 추가
- [<img src="https://github.com/snowykr.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="snowykr"> **@snowykr**](https://github.com/snowykr) — macOS 조합 상태와 입력 소스 이름 수정, 회귀 테스트 추가
- [<img src="https://github.com/Seungchan0325.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="Seungchan0325"> **@Seungchan0325**](https://github.com/Seungchan0325) — GeekNews에 소개

이 프로젝트는 [@aimdevlee](https://github.com/aimdevlee)의 [Seamtype](https://github.com/aimdevlee/seamtype) (MIT) Rust 포트로 시작되었으며, Seamtype의 모델은 [@yksr-melt](https://github.com/yksr-melt)의 [Meltype](https://github.com/yksr-melt/Meltype)에서 영감을 받았습니다. 자세한 고지는 [NOTICE](NOTICE)에 있습니다.

---

## English

Tacit is open source, run as a project of **Necessity**, under the
[AGPL-3.0-only](LICENSE). Bug reports, docs, code and language data are all welcome. Please discuss big
changes in an issue first.

### Terms of contribution (DCO)

There is no contract; we use the [Developer Certificate of Origin 1.1](https://developercertificate.org/):
you wrote the code or have the right to submit it, and you agree it is distributed under the project's
license. Sign off every commit:

```sh
git commit -s -m "fix: ..."     # adds Signed-off-by: Name <email>
```

- Contributed code is distributed under AGPL-3.0-only. Relicensing needs the contributors' consent.
- The AGPL asks for the source of a modified program even when users only reach it over a network. The
  dashboard is local to the machine, but if you run this code as a service, that clause applies.

### Development, build and test

You need Rust stable 1.89 or later. On every system these three must pass:

After building the Android APK, run `./platforms/android/smoke.ps1 -Serial emulator-5554`
against a running emulator/device to check JNI creation, activation, English/Korean input,
pause/resume, reset and destruction. This does not change the system's selected input method.

Run `./platforms/android/ui-smoke.ps1 -Serial emulator-5560` on an isolated emulator for keyboard UI
checks. It installs the test APKs, temporarily selects Tacit, restores the previous IME, and checks
real composition, clipboard/selection behavior, caps lock, Enter and cursor events. The separate
test editor is `io.github.iamallyouneed.tacit.tests/io.github.iamallyouneed.tacit.KeyboardTestActivity`.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

On Linux and macOS leave out the Windows-only text service: `--workspace --exclude tacit-tip`.

| System | Build |
| --- | --- |
| Windows | `cargo build --release -p tacit-tip -p tacit-config` (not with `tacit-cli`, a development tool: its embedded packs would end up in the other binaries too) (for x86: `rustup target add i686-pc-windows-msvc`); install with `.\scripts\install.ps1` (one administrator prompt) |
| Android | `.\platforms\android\build.ps1` (Android SDK and NDK, JDK 17, `cargo install cargo-ndk`) |
| macOS | `sh platforms/macos/build.sh` (Xcode command line tools, `rustup target add aarch64-apple-darwin x86_64-apple-darwin`) |
| Linux | `sudo apt install libibus-1.0-dev pkg-config`, then `cargo build --release -p tacit-ffi && make -C platforms/linux` |

- The **web dashboard** lives in `crates/config/src/web/` (markup, styles, scripts) and is put together into one
  page at compile time. To try it without touching your own settings:
  `TACIT_HOME=/tmp/dash TACIT_NO_BROWSER=1 cargo run -p tacit-config`
- The **golden tests** (`crates/core/tests/golden/`) are snapshots of what is decided for English, Korean and
  Japanese. They must not change unless you mean to change behavior; if they must, explain why in the PR. On
  Linux and macOS numbers may differ by 1e-6 (floating-point libraries); on Windows they match byte for byte.
- Accuracy for en/ko/ja must not drop; `crates/core/tests/accuracy.rs` guards it.

### Pull requests and commits

- Name branches `feat/…`, `fix/…`; keep PRs small, one topic each.
- Write the **test first** and watch it fail before the fix; a bug fix comes with a test that reproduces it.
- Commit messages are English, `type: description`, with type one of `feat`, `fix`, `refactor`, `docs`, `test`,
  `chore`, `perf`, `ci`; the body says *why*.
- Add a line under `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md) for anything a user can see.
- The three commands above and CI (Windows, Android, Linux, macOS) must pass before a merge.

### Changelog and releases

For urgent fixes affecting user work (misplaced input, lost text, blocked input, or crashes), follow the [quick-patch procedure](docs/quick-patch.md) (Korean), including its exceptions for local validation and benchmarks on platform-only fixes.

**Versioning.** Packs and the program are compatible by minor version. Only a change that older programs cannot read (the pack file format, the pack–program compatibility range) bumps the minor (`0.x.0`); features and fixes that keep the formats are patches (`0.8.x`). No major before 1.0. Commits pile up on `main`; release when needed, not per fix.

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/): one user-facing line per
change under `추가` (Added), `변경` (Changed), `수정` (Fixed), and `제거`/`보안` when needed. To release:

1. Move the `[Unreleased]` entries under `## [X.Y.Z] - date` and update the compare links at the bottom.
2. Bump the version: each `Cargo.toml` (7), `Cargo.lock`, and the example version in the README.
   After packaging (`scripts/package.ps1`), run `sh lab/report.sh --profile enko` and
   `sh lab/report.sh --profile enkoja` separately. Put each result in its own report:
   [EN/KO](docs/report/performance.md) or [EN/KO/JA](docs/report/performance-enkoja.md) (Korean).
    The output is a `## 한눈에` block (previous → current comparison, replacing the old block) plus the new
    version section. Add `--selftest` to each profile command to check its numbers.
3. Push a `v` tag (`git tag vX.Y.Z && git push origin vX.Y.Z`). The release workflow publishes that version's
   entry as the release notes; with no entry GitHub generates them, so fill it in. Preview with
   `sh scripts/release-notes.sh vX.Y.Z`.

### Language and dictionary data

- A **new language** is a module `crates/core/src/lang/<id>/`, one line in `registry.rs`, and a pack definition
  in `crates/packgen`. Build packs with `cargo run -p tacit-packgen --bin build-packs -- crates/core/data target/packs`.
- State the **source and license** of the data in [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt). Data of
  unknown license, or that cannot be redistributed, cannot be accepted.
- Data such as CC BY-SA **keeps its own license**, separate from the code: keep it in its own files under
  `crates/core/data`, never mixed into code.
- If a change alters what an existing language decides, show with the `accuracy` and golden tests what gets
  better and what gets worse.

### Bugs and security

- Report **bugs** as [issues](https://github.com/iamallyouneed/tacit/issues). An input method depends on its
  environment, so please include: system and version (Windows, Android, macOS, Linux), which input method and
  Tacit version, the **app** you were typing in, the **keys** you pressed, what you expected and what you
  got, and the dashboard's 체험 (try) result if you can.
- Report **security vulnerabilities** through GitHub's
  [private security advisory](https://github.com/iamallyouneed/tacit/security/advisories/new), not in a
  public issue.

### Contributors

- [<img src="https://github.com/noth3dev.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="noth3dev"> **@noth3dev**](https://github.com/noth3dev) (Necessity) — project lead and core development
- [<img src="https://github.com/Ddongddi.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="Ddongddi"> **@Ddongddi**](https://github.com/Ddongddi) — added Japanese support
- [<img src="https://github.com/snowykr.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="snowykr"> **@snowykr**](https://github.com/snowykr) — fixed macOS composition state and input-source naming, added regression tests
- [<img src="https://github.com/Seungchan0325.png?size=40" width="20" height="20" style="border-radius: 50%; vertical-align: -3px;" alt="Seungchan0325"> **@Seungchan0325**](https://github.com/Seungchan0325) — shared the project on GeekNews

The project began as a Rust port of [Seamtype](https://github.com/aimdevlee/seamtype) by [@aimdevlee](https://github.com/aimdevlee) (MIT), which was inspired by [Meltype](https://github.com/yksr-melt/Meltype) by [@yksr-melt](https://github.com/yksr-melt); see [NOTICE](NOTICE) for the notices.
