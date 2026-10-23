# 영·한 성능 리포트

프로필: `enko` (영·한). [영·한·일 성능 리포트](performance-enkoja.md)는 별도로 측정합니다.

**이 리포트의 점수·정확도·속도는 영어·한국어만 켠 상태의 결과입니다.** 설치 zip은 영·한판(`tacit-windows.zip`)이고, 활성 언어 팩 크기에는 영어·한국어만 포함합니다. 일본어를 켰을 때의 영·한 회귀와 일본어 변환 성능은 영·한·일 보고서에서 확인합니다.

릴리즈(부버전 이상)마다 크기·정확도·속도·메모리를 같은 방법으로 재서 위에서부터 쌓습니다. 맨 위 `## 한눈에`(직전 → 최신 비교)는 갈아끼우고, 그 아래에 새 버전 절을 둡니다.

<!-- score:begin -->
## 릴리즈 점수

정확도·용량·속도·메모리를 한 숫자로 합친 점수입니다(10000점 만점, 높을수록 좋음, 규격 v2, 계산기 `lab/score.py`).

![릴리즈 점수 꺾은선 그래프: 총점과 영역별 점수](score.svg)

그래프는 `.0` 릴리즈와 0.8.7, 0.8.8, 0.9.1, 0.10.0, 0.10.2, 그리고 아직 릴리즈하지 않은 최신 작업 트리 0.10.3(5252점)을 보여 줍니다(0.1.0은 태그가 없어 0.1.1로 대신). 0.9.0부터는 lab 0.2.4로 다시 쟀습니다. 계산 방법은 [score.md](score.md).

**점수가 크게 오른 곳**(같은 lab으로 잰 값, 영역 점수는 아래 표)

- **0.3.0 → 0.4.0 (270 → 1410)**: 언어 팩을 나눠서입니다. 0.3.0은 x86 DLL이 3254 KiB(사전이 DLL 안에 들어 있었음)라 용량 예산(900 KiB)을 크게 넘었고, 0.4.0은 이를 703 KiB로 줄이고 사전을 팩(합 841 KiB)으로 뺐습니다. 정확도·속도는 같아서 오른 것은 전부 L예산 감점(2.14 → 0.49)이 사라진 몫입니다.
- **0.8.5 → 0.8.6 (1398 → 2333)**: 메모리와 용량입니다. 영어 팩에서 쓰지 않는 빈도 열을 빼고 한국어 음절 표를 엔진끼리 공유해서, 팩을 올린 직후 힙이 1006 → 67 KB, 로드 시간이 5.6 → 1.8 ms가 됐고(메모리 영역 1098 → 3668), `opt-level = "s"`로 x64 DLL이 12% 작아졌습니다. 일상 문장 오류율이 예산(0.5%) 안으로 들어와 L예산도 0.49 → 0.17로 줄었습니다.
- **0.8.6 → 0.8.7 (2333 → 3871)**: 정확도와 속도입니다. 영어+한국어 분할을 경우별 규칙에서 **점수 하나**로 바꿔 조사 문장이 78.83 → 97.31%, held-out 용어+조사가 84.91 → 97.66%, 시나리오 문장이 92.86 → 98.12%가 됐습니다(정확도 영역 921 → 1919, held-out 오류율이 예산 10% 안으로 들어와 L예산 0). commit p99는 52 → 29 µs로 절반이 됐습니다(속도 영역 4009 → 5980).
- **0.8.8 → 0.9.0 (3858 → 4575)**: 용량과 정확도입니다. 패키지에서 개발용 CLI를 빼서 zip이 5235 → 2469 KiB, x64 DLL이 3457 → 927 KiB가 됐고(용량 영역 6160 → 8282), 단어 격자 탐색(Viterbi)으로 조사·숫자·긴 문장 정확도가 올랐으며(정확도 영역 1919 → 2287) 키 입력 지연이 약 15% 줄었습니다(속도 영역 5893 → 6685). 메모리 영역은 거름망 때문에 4400 → 3894로 조금 내려갔습니다.
- **0.9.0 → 0.9.1 (4688 → 5151)**: 정확도입니다. 문맥 편향을 단어 길이와 무관하게 같은 총량으로 바꿔 제목(영어 뒤) 98.03 → 99.37%, 짧은 문장 98.83 → 99.13%, 조사 문장 98.60 → 99.06%, 긴 문장 97.80 → 98.40%가 됐습니다(정확도 영역 2473 → 3156). 크기·메모리는 그대로입니다.
- **0.9.1 → 0.10.2 (5151 → 5178)**: 정확도·크기·메모리는 같고 지연만 오갑니다. 0.10.0의 속도 영역 4701은 p95·p99가 여섯 릴리즈 중 가장 느렸기 때문이고(원인은 조사하지 않았습니다), 0.10.1·0.10.2에서 5082·5424로 돌아왔습니다. 이 차이는 잡음 범위로 봅니다.
- **0.10.2 → 0.10.3, 미출시 (5178 → 5252)**: 용량입니다. 영·한만 켠 빌드에서 일본어 지원 코드가 빠져 x64 DLL이 934 → 733 KiB, x86이 756 → 605 KiB, zip이 2479 → 2241 KiB가 됐습니다(용량 영역 8269 → 8646). 정확도와 시나리오는 그대로이고, 메모리 영역은 load 시간 차이로 3876 → 4025입니다. 속도 영역은 5424 → 5057이지만 지연 변화는 모두 25% 안(최대 p99 +18%)이라 잡음으로 봅니다. 0.10.3 크기는 같은 컴퓨터에서 0.10.2와 0.10.3을 같은 설정으로 빌드한 비율을 0.10.2 CI 에셋에 곱한 값입니다.
- 0.4.0~0.8.5의 선이 평평한 것은 이 lab에서 잰 정확도(영역 753~755)·크기·메모리가 거의 같았기 때문입니다. 그 사이 릴리즈의 변경은 이 lab이 재는 항목 밖이었습니다.

<details><summary>모든 릴리즈의 점수와 세부 항</summary>

| 릴리즈 | 점수 | | 정확도 | 용량 | 속도 | 메모리 | L환율 | L예산 | L회귀 |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0.10.3 | 5252 | `█████████████░░░░░░░░░░░` | 3156 | 8646 | 5057 | 4025 | 0.644 | 0.000 | 0.000 |
| 0.10.2 | 5178 | `████████████░░░░░░░░░░░░` | 3156 | 8269 | 5424 | 3876 | 0.658 | 0.000 | 0.000 |
| 0.10.1 | 5156 | `████████████░░░░░░░░░░░░` | 3156 | 8269 | 5082 | 3930 | 0.662 | 0.000 | 0.000 |
| 0.10.0 | 5123 | `████████████░░░░░░░░░░░░` | 3156 | 8260 | 4701 | 3937 | 0.669 | 0.000 | 0.000 |
| 0.9.1 | 5151 | `████████████░░░░░░░░░░░░` | 3156 | 8280 | 5109 | 3838 | 0.663 | 0.000 | 0.000 |
| 0.9.0 | 4688 | `███████████░░░░░░░░░░░░░` | 2473 | 8282 | 5242 | 3896 | 0.758 | 0.000 | 0.000 |
| 0.8.8 | 3858 | `████████████████████░░░░` | 1919 | 6160 | 5893 | 4400 | 0.952 | 0.000 | 0.000 |
| 0.8.7 | 3871 | `████████████████████░░░░` | 1919 | 6185 | 5980 | 4437 | 0.949 | 0.000 | 0.000 |
| 0.8.6 | 2333 | `████████████░░░░░░░░░░░░` | 921 | 6250 | 4009 | 3668 | 1.286 | 0.169 | 0.000 |
| 0.8.5 | 1398 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5938 | 4339 | 1098 | 1.476 | 0.491 | 0.000 |
| 0.8.4 | 1388 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5941 | 3951 | 1094 | 1.484 | 0.491 | 0.000 |
| 0.8.3 | 1395 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5943 | 4355 | 1054 | 1.479 | 0.491 | 0.000 |
| 0.8.2 | 1400 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5948 | 4391 | 1090 | 1.475 | 0.491 | 0.000 |
| 0.8.1 | 1383 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5948 | 3781 | 1091 | 1.487 | 0.491 | 0.000 |
| 0.8.0 | 1398 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5949 | 4368 | 1077 | 1.476 | 0.491 | 0.000 |
| 0.7.0 | 1394 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5891 | 4348 | 1092 | 1.479 | 0.491 | 0.000 |
| 0.6.5 | 1393 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5893 | 4391 | 1078 | 1.479 | 0.491 | 0.000 |
| 0.6.4 | 1339 | `███████░░░░░░░░░░░░░░░░░` | 753 | 5961 | 4319 | 1083 | 1.476 | 0.491 | 0.044 |
| 0.6.3 | 1404 | `███████░░░░░░░░░░░░░░░░░` | 755 | 5968 | 4413 | 1092 | 1.472 | 0.491 | 0.000 |
| 0.6.2 | 1405 | `███████░░░░░░░░░░░░░░░░░` | 755 | 5969 | 4461 | 1094 | 1.471 | 0.491 | 0.000 |
| 0.6.1 | 1408 | `███████░░░░░░░░░░░░░░░░░` | 755 | 5994 | 4447 | 1101 | 1.469 | 0.491 | 0.000 |
| 0.6.0 | 1410 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6027 | 4439 | 1095 | 1.468 | 0.491 | 0.000 |
| 0.5.2 | 1414 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6090 | 4405 | 1086 | 1.465 | 0.491 | 0.000 |
| 0.5.1 | 1416 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6093 | 4457 | 1088 | 1.464 | 0.491 | 0.000 |
| 0.5.0 | 1420 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6154 | 4461 | 1084 | 1.460 | 0.491 | 0.000 |
| 0.4.1 | 1369 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6157 | 4295 | 1063 | 1.465 | 0.491 | 0.032 |
| 0.4.0 | 1410 | `███████░░░░░░░░░░░░░░░░░` | 755 | 6690 | 4289 | 1080 | 1.432 | 0.491 | 0.035 |
| 0.3.0 | 270 | `█░░░░░░░░░░░░░░░░░░░░░░░` | 755 | 5983 | 4508 | 1144 | 1.466 | 2.143 | 0.002 |
| 0.2.0 | 303 | `██░░░░░░░░░░░░░░░░░░░░░░` | 755 | 6197 | 5309 | 1145 | 1.439 | 2.058 | 0.000 |
| 0.1.1 | 303 | `██░░░░░░░░░░░░░░░░░░░░░░` | 755 | 6197 | 5297 | 1147 | 1.439 | 2.057 | 0.000 |

**정확도·용량·속도·메모리** 열은 영역별 점수입니다(10000점 만점). 각 영역의 평균 로그비 r(지표마다 목표보다 몇 배 나쁜가의 로그를 계수로 가중평균)로 `10000 × exp(−r)`이고, 영역의 모든 지표가 목표에 닿으면 10000점입니다. 총점은 이 네 영역의 r을 영역 계수(정확도 0.40, 용량 0.38, 속도 0.08, 메모리 0.08)로 합쳐 L환율을 만들고 L예산·L회귀를 더한 것이라, 네 값의 평균이 아닙니다.

0.8.8 이전 릴리즈는 **같은 lab**(0.9.0 시점 커밋의 세트·데이터, 특히 held-out 용어 목록은 현재 팩 기준으로 고정)에 그 태그의 코어를 붙여, 릴리즈들을 번갈아 가며(한 바퀴에 릴리즈마다 한 번씩) 5바퀴 돌린 중앙값으로 쟀습니다. 0.9.0 이후(0.9.0, 0.9.1, 0.10.0, 0.10.1, 0.10.2, 0.10.3)는 lab 0.2.4와 현재 데이터로 이 여섯 개를 번갈아 9바퀴 다시 쟀고, 이 표의 0.9.0 점수(4688)는 이전 표의 4575와 다릅니다. 이 컴퓨터가 이전보다 느려 속도 영역의 절대값(0.9.0은 6685 → 5242)도 낮아졌으니 0.8.8 이하와 0.9.0 이상을 이은 선의 속도 영역은 직접 비교하지 않습니다. 크기는 각 릴리즈의 GitHub 에셋(CI 빌드, 로컬 빌드보다 약 3% 큼)입니다. lab의 케이스와 시나리오가 늘어서 이전 그래프의 점수와 다릅니다(예: 0.8.7은 4121점이었고 지금 기준 3871점). 아래쪽 0.8.7 이전 절은 그때의 lab과 로컬 빌드로 잰 값이라 이 표와 다릅니다. 0.3.0 이하는 팩 구조 이전이라 `make_engine` 호출 모양만 맞춰 같은 lab에 붙였고 `size.packs`는 0입니다. 0.2.1은 뺐습니다. 점수 ±2% 이내 차이는 잡음입니다.

</details>
<!-- score:end -->

## 측정 방법

```sh
cargo build --release -p tacit-tip -p tacit-config   # tacit-cli와 묶지 않는다: 내장 팩이 TIP·config에도 들어간다
cargo build --release -p tacit-tip --target i686-pc-windows-msvc
powershell -File scripts/package.ps1          # dist/ 에 릴리즈와 같은 zip
python lab/fetch_data.py                      # 처음 한 번 (큰 세트용 데이터, git에 안 들어감)
sh lab/report.sh --profile enko > section-enko.md     # 이 문서의 한눈에 교체 + 새 버전 절
sh lab/report.sh --profile enkoja > section-enkoja.md # 별도 영·한·일 문서
```

- 크기는 릴리즈 워크플로(`release.yml`)와 같은 빌드의 산출물입니다. 0.8.7까지는 `tacit-cli`와 같이 빌드해서 영어·한국어 팩이 x64 TIP과 `tacit-config`에 내장되었고(기능 통합), 이후로는 CLI를 묶지 않고 패키지에도 넣지 않습니다.
- 정확도는 `lab/`의 세트입니다. `acc.*`는 단어 단위(직접 고른 목록 포함), `big.*`는 실제 문장·제목에서 만든 큰 세트입니다. `sentence`는 문장 전체가 맞아야 맞고, `token`은 단어 하나씩 셉니다. 자세한 건 `lab/src/main.rs`와 `lab/src/corpus.rs`에 있습니다.
- 속도는 결정 한 번(`commit`)과 키 입력 한 번(`keystroke`)의 마이크로초이고, 5회 중앙값입니다. 릴리즈끼리 비교할 때는 한 바퀴에 릴리즈마다 한 번씩 번갈아 돌려, 기계 상태의 흔들림이 모든 릴리즈에 같이 걸리게 합니다. 같은 컴퓨터에서도 백그라운드 작업에 따라 10~20% 흔들리므로 릴리즈 사이의 작은 차이는 잡음으로 봅니다.
- 메모리는 팩을 올린 뒤 힙(`heap.loaded_kb`)과 측정 중 최대 힙(`heap.peak_kb`)입니다.
- 기준선 저장도 프로필별입니다: `cargo run --release --manifest-path lab/Cargo.toml -- --profile enko --save`는 `lab/baseline-enko.tsv`, `--profile enkoja --save`는 `lab/baseline-enkoja.tsv`만 갱신합니다. 기본값은 `enko`입니다.
- **이전 버전과 비교할 때는 같은 lab으로 잽니다.** 세트와 데이터가 달라지면 같은 코드도 수치가 달라집니다. 새 worktree 둘을 만들어 한쪽에만 `git restore --source=<이전 태그> --worktree --staged crates/core`를 하고(코어에 없는 API를 쓰는 `lab/src/tune.rs` 같은 파일은 그 worktree에서만 비활성으로 바꿉니다), 각각 `lab/data`를 **복사**해 두고 5회씩 돌려 중앙값을 냅니다. `lab/data`를 junction이나 링크로 연결한 worktree를 `git worktree remove --force` 하면 링크 너머의 실제 데이터까지 지워집니다.

## 한눈에: 0.10.9 → 0.10.10

▲ 좋아짐 · ▼ 나빠짐 · = 동점/잡음(정확도 0.05%p, 지연 25% 안은 잡음으로 봄). 정확도는 높을수록, 크기·지연·메모리는 낮을수록 좋음.

이전 열은 같은 프로필의 과거 보고서 기록입니다. 같은 lab·데이터로 교차 재측정하지 않은 지연 차이는 코드 개선의 증거로 해석하지 않습니다.

### 정확도 핵심 (%)

막대 범위: 영·한 80~100%, 일본어(ja.*) 0~100%.

| 지표 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `acc.ALL` | 99.69 `████████████` | 99.69 `████████████` | +0.00 = |
| `acc.en.freq/none` | 99.50 `████████████` | 99.50 `████████████` | +0.00 = |
| `acc.en.freq/after-ko` | 99.45 `████████████` | 99.45 `████████████` | +0.00 = |
| `acc.ko.words/none` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `acc.ko.vocab/none` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `big.ko.sent.sentence` | 99.13 `███████████░` | 99.13 `███████████░` | +0.00 = |
| `big.ko.sent.token` | 99.83 `████████████` | 99.83 `████████████` | +0.00 = |
| `big.ko.phrase.sentence` | 99.37 `████████████` | 99.37 `████████████` | +0.00 = |
| `big.ko.long.sentence` | 98.40 `███████████░` | 98.40 `███████████░` | +0.00 = |
| `big.en.sent.sentence` | 99.93 `████████████` | 99.93 `████████████` | +0.00 = |
| `big.cs.particle.sentence` | 99.06 `███████████░` | 99.06 `███████████░` | +0.00 = |
| `big.cs.particle.token` | 99.85 `████████████` | 99.85 `████████████` | +0.00 = |
| `big.cs_heldout.particle.sentence` | 98.95 `███████████░` | 98.95 `███████████░` | +0.00 = |
| `big.cs.bare.sentence` | 97.77 `███████████░` | 97.77 `███████████░` | +0.00 = |
| `big.cs.number.sentence` | 99.29 `████████████` | 99.29 `████████████` | +0.00 = |
| `big.ko.title.none` | 99.87 `████████████` | 99.87 `████████████` | +0.00 = |
| `big.ko.title.after-en` | 99.37 `████████████` | 99.37 `████████████` | +0.00 = |
| `big.learn.other_forms_before` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `big.learn.other_forms_after` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |

### 유형별 시나리오 문장 정답률 (%, 높을수록 좋음, 막대 80~100 구간)

| 유형 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| chat | 96.51 `██████████░░` | 96.51 `██████████░░` | +0.00 = |
| dev | 98.98 `███████████░` | 98.98 `███████████░` | +0.00 = |
| writing | 98.78 `███████████░` | 98.78 `███████████░` | +0.00 = |
| 전체 | 98.12 `███████████░` | 98.12 `███████████░` | +0.00 = |

### 크기 비교 (KiB, 낮을수록 좋음, 막대 표 안 최대 기준)

| 항목 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `tacit_tip.dll` (x64) | 756 `███░░░░░░░░░` | 756 `███░░░░░░░░░` | 0 = |
| `tacit_tip.dll` (x86) | 620 `██░░░░░░░░░░` | 620 `██░░░░░░░░░░` | 0 = |
| `tacit-config.exe` | 3158 `████████████` | 3158 `████████████` | 0 = |
| `tacit-windows.zip` | 2182 `████████░░░░` | 2182 `████████░░░░` | 0 = |
| `tacit-pack-en-0.10.10.zip` | 446 `██░░░░░░░░░░` | 446 `██░░░░░░░░░░` | 0 = |
| `tacit-pack-ko-0.10.10.zip` | 451 `██░░░░░░░░░░` | 450 `██░░░░░░░░░░` | -1 (-0.2%) ▲ |

### 지연 비교 (µs 중앙값, 낮을수록 좋음, 막대 표 안 최대 기준)

| 항목 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `commit.avg_us` | 15.84 `██░░░░░░░░░░` | 19.04 `██░░░░░░░░░░` | +3.20 (+20.2%) = |
| `commit.p50_us` | 12.80 `██░░░░░░░░░░` | 13.70 `██░░░░░░░░░░` | +0.90 (+7.0%) = |
| `commit.p95_us` | 26.30 `███░░░░░░░░░` | 31.40 `████░░░░░░░░` | +5.10 (+19.4%) = |
| `commit.p99_us` | 56.50 `███████░░░░░` | 75.00 `█████████░░░` | +18.50 (+32.7%) ▼ |
| `keystroke.avg_us` | 15.18 `██░░░░░░░░░░` | 17.06 `██░░░░░░░░░░` | +1.88 (+12.4%) = |
| `keystroke.p50_us` | 10.70 `█░░░░░░░░░░░` | 11.50 `█░░░░░░░░░░░` | +0.80 (+7.5%) = |
| `keystroke.p95_us` | 33.70 `████░░░░░░░░` | 37.10 `█████░░░░░░░` | +3.40 (+10.1%) = |
| `keystroke.p99_us` | 83.60 `██████████░░` | 97.70 `████████████` | +14.10 (+16.9%) = |

### 시작·메모리 비교 (낮을수록 좋음, 막대 행별 비교)

| 항목 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `load.ms` | 3.19 `███████████░` | 3.47 `████████████` | +0.28 (+8.8%) = |
| `heap.loaded_kb` | 52.67 `████████████` | 52.67 `████████████` | +0.00 (+0.0%) = |
| `heap.peak_kb` | 11628.82 `████████████` | 11628.82 `████████████` | +0.00 (+0.0%) = |

## 0.10.10

commit cfd3096, 2026-10-23, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enko` (영·한), lab 0.2.12.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 756 |
| `tacit_tip.dll` (x86) | 620 |
| `tacit-config.exe` | 3158 |
| `tacit-windows.zip` | 2182 |
| `tacit-pack-en-0.10.10.zip` | 446 |
| `tacit-pack-ko-0.10.10.zip` | 450 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.69 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.45 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.13 |
| `big.ko.sent.token` | 99.83 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 98.40 |
| `big.en.sent.sentence` | 99.93 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 97.77 |
| `big.cs.number.sentence` | 99.29 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 4.33 |
| dev | 98.98 | 99.69 | 99.37 | 4.68 |
| writing | 98.78 | 99.77 | 99.75 | 3.37 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 19.04 |
| `commit.p50_us` | 13.70 |
| `commit.p95_us` | 31.40 |
| `commit.p99_us` | 75.00 |
| `keystroke.avg_us` | 17.06 |
| `keystroke.p50_us` | 11.50 |
| `keystroke.p95_us` | 37.10 |
| `keystroke.p99_us` | 97.70 |
| `load.ms` | 3.47 |
| `heap.loaded_kb` | 52.67 |
| `heap.peak_kb` | 11628.82 |

## 0.10.9

commit b58cc6d, 2026-10-23, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enko` (영·한), lab 0.2.12.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 756 |
| `tacit_tip.dll` (x86) | 620 |
| `tacit-config.exe` | 3158 |
| `tacit-windows.zip` | 2182 |
| `tacit-pack-en-0.10.9.zip` | 446 |
| `tacit-pack-ko-0.10.9.zip` | 451 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.69 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.45 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.13 |
| `big.ko.sent.token` | 99.83 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 98.40 |
| `big.en.sent.sentence` | 99.93 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 97.77 |
| `big.cs.number.sentence` | 99.29 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 3.04 |
| dev | 98.98 | 99.69 | 99.37 | 3.50 |
| writing | 98.78 | 99.77 | 99.75 | 2.23 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 15.84 |
| `commit.p50_us` | 12.80 |
| `commit.p95_us` | 26.30 |
| `commit.p99_us` | 56.50 |
| `keystroke.avg_us` | 15.18 |
| `keystroke.p50_us` | 10.70 |
| `keystroke.p95_us` | 33.70 |
| `keystroke.p99_us` | 83.60 |
| `load.ms` | 3.19 |
| `heap.loaded_kb` | 52.67 |
| `heap.peak_kb` | 11628.82 |

## 0.10.3

commit 5816b5d (작업 트리 변경 포함), 2026-10-19, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enko` (영·한), lab 0.2.4.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 747 |
| `tacit_tip.dll` (x86) | 613 |
| `tacit-config.exe` | 3100 |
| `tacit-windows.zip` | 2150 |
| `tacit-pack-en-0.10.3.zip` | 446 |
| `tacit-pack-ko-0.10.3.zip` | 451 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.69 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.45 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.13 |
| `big.ko.sent.token` | 99.83 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 98.40 |
| `big.en.sent.sentence` | 99.93 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 97.77 |
| `big.cs.number.sentence` | 99.29 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 1.76 |
| dev | 98.98 | 99.69 | 99.37 | 2.36 |
| writing | 98.78 | 99.77 | 99.75 | 1.74 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 9회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 10.48 |
| `commit.p50_us` | 9.00 |
| `commit.p95_us` | 19.80 |
| `commit.p99_us` | 37.00 |
| `keystroke.avg_us` | 9.70 |
| `keystroke.p50_us` | 7.60 |
| `keystroke.p95_us` | 22.80 |
| `keystroke.p99_us` | 50.30 |
| `load.ms` | 1.57 |
| `heap.loaded_kb` | 52.67 |
| `heap.peak_kb` | 11628.73 |

## 0.10.2

commit 53325c6 (작업 트리 변경 포함), 2026-10-18, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enko` (영·한), lab 0.2.0.

2026-10-19에 0.10.3과 같은 lab 0.2.4·데이터로 번갈아 9바퀴 다시 쟀습니다. 아래 지연·메모리는 그 중앙값이고, 크기는 같은 컴퓨터의 0.10.2 로컬 빌드(`53325c6` 태그 `v0.10.2`)입니다. 정확도는 처음 기록과 같았습니다.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 설치 zip은 공통이며 팩 목록만 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 951 |
| `tacit_tip.dll` (x86) | 766 |
| `tacit-config.exe` | 3276 |
| `tacit-windows.zip` | 2379 |
| `tacit-pack-en-0.10.2.zip` | 446 |
| `tacit-pack-ko-0.10.2.zip` | 451 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.69 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.45 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.13 |
| `big.ko.sent.token` | 99.83 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 98.40 |
| `big.en.sent.sentence` | 99.93 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 97.77 |
| `big.cs.number.sentence` | 99.29 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 1.42 |
| dev | 98.98 | 99.69 | 99.37 | 1.68 |
| writing | 98.78 | 99.77 | 99.75 | 1.16 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 10.32 |
| `commit.p50_us` | 9.20 |
| `commit.p95_us` | 18.20 |
| `commit.p99_us` | 31.30 |
| `keystroke.avg_us` | 9.48 |
| `keystroke.p50_us` | 7.80 |
| `keystroke.p95_us` | 20.60 |
| `keystroke.p99_us` | 44.90 |
| `load.ms` | 1.78 |
| `heap.loaded_kb` | 52.66 |
| `heap.peak_kb` | 11628.71 |

## 0.9.1

commit 646dec9, 2026-10-18, rustc 1.93.1 (01f6ddf75 2026-02-11)

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 943 |
| `tacit_tip.dll` (x86) | 760 |
| `tacit-config.exe` | 3265 |
| `tacit-windows.zip` | 2369 |
| `tacit-pack-en-0.9.1.zip` | 446 |
| `tacit-pack-ko-0.9.1.zip` | 450 |
| `tacit-pack-ja-0.9.1.zip` | 4763 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.69 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.45 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.13 |
| `big.ko.sent.token` | 99.83 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 98.40 |
| `big.en.sent.sentence` | 99.93 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 97.77 |
| `big.cs.number.sentence` | 99.29 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 2.61 |
| dev | 98.98 | 99.69 | 99.37 | 2.75 |
| writing | 98.78 | 99.77 | 99.75 | 2.30 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 8.22 |
| `commit.p50_us` | 7.40 |
| `commit.p95_us` | 13.40 |
| `commit.p99_us` | 24.70 |
| `keystroke.avg_us` | 7.46 |
| `keystroke.p50_us` | 6.30 |
| `keystroke.p95_us` | 14.70 |
| `keystroke.p99_us` | 33.40 |
| `load.ms` | 1.87 |
| `heap.loaded_kb` | 52.24 |
| `heap.peak_kb` | 11628.28 |

## 0.9.0

commit d249a85, 2026-10-17, rustc 1.93.1 (01f6ddf75 2026-02-11). 크기는 릴리스 에셋(CI 빌드), 정확도·속도·메모리는 로컬에서 릴리스들을 번갈아 돌려 잰 값입니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 927 |
| `tacit_tip.dll` (x86) | 751 |
| `tacit-config.exe` | 3350 |
| `tacit-windows.zip` | 2469 |
| `tacit-pack-en-0.9.0.zip` | 446 |
| `tacit-pack-ko-0.9.0.zip` | 450 |
| `tacit-pack-ja-0.9.0.zip` | 4763 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.79 |
| `acc.en.freq/none` | 99.55 |
| `acc.en.freq/after-ko` | 99.40 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 98.83 |
| `big.ko.sent.token` | 99.78 |
| `big.ko.phrase.sentence` | 98.90 |
| `big.ko.long.sentence` | 97.80 |
| `big.en.sent.sentence` | 99.97 |
| `big.cs.particle.sentence` | 98.60 |
| `big.cs.particle.token` | 99.78 |
| `big.cs_heldout.particle.sentence` | 98.48 |
| `big.cs.bare.sentence` | 97.18 |
| `big.cs.number.sentence` | 99.17 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 98.03 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 1.68 |
| dev | 98.98 | 99.69 | 99.68 | 2.08 |
| writing | 98.78 | 99.77 | 99.75 | 1.43 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 7.72 |
| `commit.p50_us` | 6.90 |
| `commit.p95_us` | 12.40 |
| `commit.p99_us` | 23.00 |
| `keystroke.avg_us` | 7.07 |
| `keystroke.p50_us` | 5.70 |
| `keystroke.p95_us` | 14.20 |
| `keystroke.p99_us` | 32.80 |
| `load.ms` | 1.78 |
| `heap.loaded_kb` | 52.23 |
| `heap.peak_kb` | 11569.83 |

## 0.8.8

릴리스 에셋(CI 빌드)의 크기와, 0.9.0 시점의 lab에 0.8.8 코어를 붙여 0.9.0과 번갈아 잰 정확도·속도·메모리입니다(0.8.8 당시의 lab이 아닙니다), 2026-10-17, rustc 1.93.1 (01f6ddf75 2026-02-11).

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 3457 |
| `tacit_tip.dll` (x86) | 724 |
| `tacit.exe` | 3103 |
| `tacit-config.exe` | 5877 |
| `tacit-windows.zip` | 5235 |
| `tacit-pack-en-0.8.8.zip` | 446 |
| `tacit-pack-ko-0.8.8.zip` | 387 |
| `tacit-pack-ja-0.8.8.zip` | 4763 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.68 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.40 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 98.43 |
| `big.ko.sent.token` | 99.70 |
| `big.ko.phrase.sentence` | 98.87 |
| `big.ko.long.sentence` | 97.00 |
| `big.en.sent.sentence` | 99.97 |
| `big.cs.particle.sentence` | 97.31 |
| `big.cs.particle.token` | 99.60 |
| `big.cs_heldout.particle.sentence` | 97.66 |
| `big.cs.bare.sentence` | 96.71 |
| `big.cs.number.sentence` | 98.81 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 98.03 |
| `big.learn.other_forms_before` | 99.84 |
| `big.learn.other_forms_after` | 99.84 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 1.64 |
| dev | 98.98 | 99.69 | 99.68 | 1.75 |
| writing | 98.78 | 99.77 | 99.75 | 1.77 |
| 전체 | 98.12 | 99.50 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 92.31 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 88.24 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 96.00 |
| `dev.tool` | 100.00 |
| `writing.academic` | 92.86 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 8.51 |
| `commit.p50_us` | 7.70 |
| `commit.p95_us` | 14.80 |
| `commit.p99_us` | 28.70 |
| `keystroke.avg_us` | 8.34 |
| `keystroke.p50_us` | 6.50 |
| `keystroke.p95_us` | 18.00 |
| `keystroke.p99_us` | 40.00 |
| `load.ms` | 1.80 |
| `heap.loaded_kb` | 35.67 |
| `heap.peak_kb` | 11249.28 |

## 0.8.7

commit 9527580(정확도·속도는 커밋된 lab으로 재측정), 2026-10-16, rustc 1.93.1 (01f6ddf75 2026-02-11)

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 3460 |
| `tacit_tip.dll` (x86) | 724 |
| `tacit.exe` | 3110 |
| `tacit-config.exe` | 5793 |
| `tacit-windows.zip` | 5069 |
| `tacit-pack-en-0.8.7.zip` | 446 |
| `tacit-pack-ko-0.8.7.zip` | 387 |
| `tacit-pack-ja-0.8.7.zip` | 4763 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.81 |
| `acc.en.freq/none` | 99.50 |
| `acc.en.freq/after-ko` | 99.40 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 98.43 |
| `big.ko.sent.token` | 99.70 |
| `big.ko.phrase.sentence` | 98.87 |
| `big.ko.long.sentence` | 97.00 |
| `big.en.sent.sentence` | 99.97 |
| `big.cs.particle.sentence` | 98.01 |
| `big.cs.particle.token` | 99.70 |
| `big.cs_heldout.particle.sentence` | 97.78 |
| `big.cs.bare.sentence` | 97.18 |
| `big.cs.number.sentence` | 98.81 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 98.03 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 98.44 | 99.47 | 99.47 | 3.65 |
| dev | 98.48 | 99.54 | 99.53 | 3.71 |
| writing | 100.00 | 100.00 | 100.00 | 3.51 |
| 전체 | 98.92 | 99.71 | | |

| 시나리오 | 문장 정답률 |
| --- | ---: |
| `chat.brand` | 100.00 |
| `chat.casual` | 100.00 |
| `chat.english` | 100.00 |
| `chat.loan` | 100.00 |
| `chat.number` | 100.00 |
| `chat.punct` | 100.00 |
| `chat.slang` | 90.91 |
| `dev.commit` | 100.00 |
| `dev.error` | 100.00 |
| `dev.identifier` | 100.00 |
| `dev.issue` | 100.00 |
| `dev.review` | 100.00 |
| `dev.terminal` | 94.12 |
| `dev.tool` | 100.00 |
| `writing.academic` | 100.00 |
| `writing.email` | 100.00 |
| `writing.english` | 100.00 |
| `writing.essay` | 100.00 |
| `writing.formal` | 100.00 |
| `writing.long` | 100.00 |
| `writing.numbers` | 100.00 |
| `writing.quote` | 100.00 |

### 속도·메모리 (중앙값, 5회)

| 항목 | 값 |
| --- | ---: |
| `commit.avg_us` | 11.99 |
| `commit.p50_us` | 10.50 |
| `commit.p95_us` | 22.30 |
| `commit.p99_us` | 44.20 |
| `keystroke.avg_us` | 11.74 |
| `keystroke.p50_us` | 9.40 |
| `keystroke.p95_us` | 25.90 |
| `keystroke.p99_us` | 60.70 |
| `load.ms` | 2.90 |
| `heap.loaded_kb` | 35.67 |
| `heap.peak_kb` | 7968.90 |

이번 릴리즈에서 시도하고 접은 것(연쇄 재채점, 한국어 언어모델, 기술 용어 trigram 등)과 수치는 [trial.md](trial.md)에 있습니다.

### 0.8.6 → 0.8.7: 유형별 시나리오 측정과 첫 고도화

측정에 유형별 시나리오를 더했습니다(`lab/scenarios/{chat,dev,writing}.tsv`, 대화 64·개발 66·글쓰기 55문장, 시나리오 태그별). 문장 하나씩(`sentence`, `token`)과 같은 유형 4문장을 이어 입력한 경우(`chain.token`, 앞 단어의 언어가 다음 단어로 이어지는 상황)를 잽니다. 직접 쓴 문장이라 편향이 있고, 틀린 것을 일감으로 쓰는 세트입니다. 아래는 같은 세트를 0.8.6 코어와 0.8.7 코어에 돌린 값입니다.

| 유형 | 문장 (0.8.6 → 후) | token (0.8.6 → 후) | chain.token (0.8.6 → 후) |
| ---- | ----------------: | -----------------: | -----------------------: |
| 대화 (chat)     | 96.88 → 98.44 | 98.95 → 99.47 | 99.47 → 99.47 |
| 개발 (dev)      | 96.97 → 98.48 | 99.08 → 99.54 | 99.06 → 99.53 |
| 글쓰기 (writing) | 94.55 → 100.00 | 98.98 → 100.00 | 99.61 → 100.00 |
| 전체           | 96.22 → 98.92 | 99.00 → 99.71 | |

시나리오별로 크게 달라진 것: 개발 `identifier` 85.7 → 100(`isValid가`), 글쓰기 `numbers` 62.5 → 100(`2024년`, `1.5kg이다`), 대화 `slang` 81.8 → 90.9.

고친 것:
- 숫자 바로 뒤의 단어: `년월일시분초개명원번살층대권`은 한국어(`2024sus` → `2024년`), `kg`, `gb`, `ms` 같은 단위는 영어(`1.5kgdlek` → `1.5kg이다`).
- camelCase 식별자 뒤 조사(`isValidrk` → `isValid가`).
- 사전에 없던 개발 도구 이름 약 60개(`graphql`, `mongodb`, `kubectl`, `timeout`, `sql`, `ssh` …). 한글 단어로도 읽히는 `dns`(운), `tls`(신)는 일부러 뺐습니다.

사전에 단어를 더하면 `terms.txt` 기반 세트가 부풀려지므로, 사전에 올리지 않은 단어만으로 따로 잰 값(`big.cs_heldout.particle.sentence`)을 같이 봅니다.

| 세트 | 0.8.6 | 후 |
| ---- | ----: | --: |
| `big.cs.particle.sentence` (전체 용어) | 96.02 | 98.01 |
| `big.cs_heldout.particle.sentence` (사전에 없는 용어만) | 95.32 | 97.78 |
| `big.ko.sent.sentence` | 97.10 | 97.23 |
| `acc.ALL` | 99.81 | 99.82 |

아직 못 고친 것(다음 일감):
- 문장 맨 앞의 두 자음 슬랭 `ㄴㄴ`(`ss`), `ㅁㅊ`(`ac`): 영어 약어와 구분할 근거가 없습니다(대화 `slang` 81.8%).
- 하이픈 뒤 옵션 `grep -rn` → `-구`: 직전 글자(`-`)를 엔진에 알려야 하고 세션 쪽 변경이 큽니다(개발 `terminal` 94.1%).
- 문장 시작의 `rm`(→그), `cp`(→체), 한글로 읽히는 `dns`, `tls`.

## 0.8.6

기준 커밋 d556020(+릴리즈 준비), 2026-10-16, rustc 1.93.1. 영어 팩의 쓰지 않던 빈도 열 제거, 한국어 음절 표 공유, 코어 엔진 `opt-level = "s"` 빌드, `cd` 영어 처리가 들어간 뒤의 첫 측정입니다.

### 크기 (KiB)

| 항목                            |  KiB |
| ------------------------------- | ---: |
| `tacit_tip.dll` (x64)      | 3435 |
| `tacit_tip.dll` (x86)      |  704 |
| `tacit.exe`                | 3081 |
| `tacit-config.exe`         | 5765 |
| `tacit-windows.zip` (설치) | 5036 |
| `tacit-pack-en-0.8.6.zip`  |  446 |
| `tacit-pack-ko-0.8.6.zip`  |  387 |
| `tacit-pack-ja-0.8.6.zip`  | 4763 |

### 정확도 (%)

| 세트                           | 정확도 |
| ------------------------------ | -----: |
| `acc.ALL` (직접 고른 전체)     |  99.81 |
| `acc.en.freq/none`             |  99.60 |
| `acc.en.freq/after-ko`         |  99.45 |
| `acc.ko.words/none`            |  99.95 |
| `acc.ko.vocab/none`            | 100.00 |
| `big.ko.sent.sentence`         |  97.10 |
| `big.ko.sent.token`            |  99.45 |
| `big.en.sent.sentence`         |  99.97 |
| `big.cs.particle.sentence`     |  86.82 |
| `big.cs.particle.token`        |  98.08 |
| `big.cs.bare.sentence`         |  95.66 |
| `big.cs.number.sentence`       |  96.95 |
| `big.ko.title.none`            |  99.87 |
| `big.ko.title.after-en`        |  98.57 |
| `big.learn.other_forms_before` |  87.50 |
| `big.learn.other_forms_after`  |  98.21 |

### 속도·메모리 (중앙값, 5회)

| 항목               |       값 |
| ------------------ | -------: |
| `commit.avg_us`    |    23.85 |
| `commit.p50_us`    |    20.10 |
| `commit.p95_us`    |    51.00 |
| `commit.p99_us`    |   101.70 |
| `keystroke.avg_us` |    13.90 |
| `keystroke.p50_us` |     8.90 |
| `keystroke.p95_us` |    34.10 |
| `keystroke.p99_us` |    63.40 |
| `load.ms`          |     1.90 |
| `heap.loaded_kb`   |    66.70 |
| `heap.peak_kb`     |  7995.82 |

### 이전과 비교 (이번 작업 전 `6c5ba4d`, 같은 lab 세트)

릴리즈 전에 따로 잰 값입니다. 다음 릴리즈부터는 위와 같은 표끼리 비교합니다.

| 항목                                    |    이전 |  0.8.6 |
| --------------------------------------- | ------: | -----: |
| `big.ko.sent.sentence` (%)              |   96.83 |  97.10 |
| `big.cs.particle.sentence` (%)          |   85.78 |  86.82 |
| `big.cs.bare.sentence` (%)              |   95.20 |  95.66 |
| `big.learn.other_forms_after` (%)       |   87.50 |  98.21 |
| 로드 직후 힙 (KiB)                      |    1006 |     67 |
| 최대 힙 (MiB)                           |     9.3 |    8.0 |
| 영어 팩 (MiB)                           |    1.58 |   1.44 |
| `commit` 평균 (µs, 같은 날 같은 조건)   |    ≈19 |    ≈20 |

- 정확도는 이미 97% 안팎이던 실데이터에서 소폭 올랐고, 직접 고른 목록(letter-noun 0 → 95%)에서 크게 올랐습니다. 학습 시뮬레이션은 한 형태를 가르친 뒤 다른 활용형이 87.5%에서 98.2%로 올랐습니다.
- 메모리와 크기는 줄었고, 결정 속도는 같거나 약간 느립니다(코어 `opt-level = "s"`로 약 3%).
- 알려진 약점: `graphql을`, `mongodb를`, `kubectl하고`처럼 사전에 없는 IT 이름 뒤에 조사가 붙는 문장(`big.cs.particle.sentence`의 나머지 13%).
