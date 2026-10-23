# 영·한·일 성능 리포트

프로필: `enkoja` (영·한·일). [영·한 성능 리포트](performance.md)와 기준선·이력을 분리합니다.

영어·한국어·일본어를 모두 켜고 공통 단어·문장·시나리오·사용자 학습 평가를 실행합니다.
`acc.ALL`, `big.*`, `sc.*`는 영·한 공통 세트의 결과이므로, 일본어 활성화가 기존 입력에
미치는 영향을 볼 수 있습니다. 일본어 판별·가나→한자 변환은 `ja.*`로 따로 표시하며
서로 다른 세트의 정확도를 하나로 평균내지 않습니다. chat/dev/writing 시나리오는
영·한 시나리오이며 일본어 문장 평가는 자체 64건을 사용합니다. AJIMEE·Mozc 외부
평가와 한계는 [일본어 개선 보고서](japanese-improvement.md)를 참고합니다.

설치 zip은 일본어 포함판(`tacit-windows-ja.zip`)이고 영·한판보다 크며, 설치하는 언어 팩에는 일본어가 추가됩니다.
`load.ms`, `heap.*`는 판별기 시작 및 공통 단어 평가 기준이며, `ja.load.ms`,
`ja.heap.loaded_kb`는 그 뒤 별도로 초기화하는 가나→한자 변환기의 시간과 힙 증가입니다.
이 값은 실제 앱의 전체 working set이나 키 입력부터 화면 반영까지의 지연이 아닙니다.

```sh
sh lab/report.sh --profile enkoja > section-enkoja.md
sh lab/report.sh --profile enkoja --selftest
cargo run --release --manifest-path lab/Cargo.toml -- --profile enkoja --save
```

출력을 아래 최신 버전 절로 넣습니다. 이전 영·한·일 측정이 있을 때만 같은 프로필의
직전 버전과 비교합니다. 기준선은 `lab/baseline-enkoja.tsv`이며, 영·한 기준선을 대신
사용하지 않습니다. 아래 수치는 동일 프로필 5회 실행의 중앙값입니다.

## 한눈에: 0.10.9 → 0.10.10

▲ 좋아짐 · ▼ 나빠짐 · = 동점/잡음(정확도 0.05%p, 지연 25% 안은 잡음으로 봄). 정확도는 높을수록, 크기·지연·메모리는 낮을수록 좋음.

이전 열은 같은 프로필의 과거 보고서 기록입니다. 같은 lab·데이터로 교차 재측정하지 않은 지연 차이는 코드 개선의 증거로 해석하지 않습니다.

### 정확도 핵심 (%)

막대 범위: 영·한 80~100%, 일본어(ja.*) 0~100%.

| 지표 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `acc.ALL` | 99.52 `████████████` | 99.52 `████████████` | +0.00 = |
| `acc.en.freq/none` | 98.85 `███████████░` | 98.85 `███████████░` | +0.00 = |
| `acc.en.freq/after-ko` | 98.80 `███████████░` | 98.80 `███████████░` | +0.00 = |
| `acc.ko.words/none` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `acc.ko.vocab/none` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `big.ko.sent.sentence` | 99.03 `███████████░` | 99.03 `███████████░` | +0.00 = |
| `big.ko.sent.token` | 99.81 `████████████` | 99.81 `████████████` | +0.00 = |
| `big.ko.phrase.sentence` | 99.37 `████████████` | 99.37 `████████████` | +0.00 = |
| `big.ko.long.sentence` | 97.70 `███████████░` | 97.70 `███████████░` | +0.00 = |
| `big.en.sent.sentence` | 99.90 `████████████` | 99.90 `████████████` | +0.00 = |
| `big.cs.particle.sentence` | 99.06 `███████████░` | 99.06 `███████████░` | +0.00 = |
| `big.cs.particle.token` | 99.85 `████████████` | 99.85 `████████████` | +0.00 = |
| `big.cs_heldout.particle.sentence` | 98.95 `███████████░` | 98.95 `███████████░` | +0.00 = |
| `big.cs.bare.sentence` | 96.94 `██████████░░` | 96.94 `██████████░░` | +0.00 = |
| `big.cs.number.sentence` | 99.05 `███████████░` | 99.05 `███████████░` | +0.00 = |
| `big.ko.title.none` | 99.87 `████████████` | 99.87 `████████████` | +0.00 = |
| `big.ko.title.after-en` | 99.37 `████████████` | 99.37 `████████████` | +0.00 = |
| `big.learn.other_forms_before` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `big.learn.other_forms_after` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `ja.convert.ALL.top1` | 85.94 `██████████░░` | 85.94 `██████████░░` | +0.00 = |
| `ja.convert.ALL.top5` | 96.88 `████████████` | 96.88 `████████████` | +0.00 = |
| `ja.detect.none.top1` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `ja.detect.after-ja.top1` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |
| `ja.guard.english.top1` | 100.00 `████████████` | 100.00 `████████████` | +0.00 = |

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
| `tacit_tip.dll` (x64) | 968 `█░░░░░░░░░░░` | 968 `█░░░░░░░░░░░` | 0 = |
| `tacit_tip.dll` (x86) | 778 `█░░░░░░░░░░░` | 778 `█░░░░░░░░░░░` | 0 = |
| `tacit-config.exe` | 3339 `███░░░░░░░░░` | 3339 `███░░░░░░░░░` | 0 = |
| `tacit-windows-ja.zip` | 2418 `██░░░░░░░░░░` | 2418 `██░░░░░░░░░░` | 0 = |
| `tacit-pack-en-0.10.10.zip` | 446 `█░░░░░░░░░░░` | 446 `█░░░░░░░░░░░` | 0 = |
| `tacit-pack-ko-0.10.10.zip` | 451 `█░░░░░░░░░░░` | 450 `█░░░░░░░░░░░` | -1 (-0.2%) ▲ |
| `tacit-pack-ja-0.10.10.zip` | 14628 `████████████` | 14628 `████████████` | 0 = |

### 지연 비교 (µs 중앙값, 낮을수록 좋음, 막대 표 안 최대 기준)

| 항목 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `commit.avg_us` | 78.49 `█░░░░░░░░░░░` | 75.77 `█░░░░░░░░░░░` | -2.72 (-3.5%) = |
| `commit.p50_us` | 68.40 `█░░░░░░░░░░░` | 63.20 `█░░░░░░░░░░░` | -5.20 (-7.6%) = |
| `commit.p95_us` | 172.30 `██░░░░░░░░░░` | 170.40 `██░░░░░░░░░░` | -1.90 (-1.1%) = |
| `commit.p99_us` | 257.60 `████░░░░░░░░` | 238.70 `███░░░░░░░░░` | -18.90 (-7.3%) = |
| `keystroke.avg_us` | 37.14 `█░░░░░░░░░░░` | 34.50 `█░░░░░░░░░░░` | -2.64 (-7.1%) = |
| `keystroke.p50_us` | 22.70 `█░░░░░░░░░░░` | 20.80 `█░░░░░░░░░░░` | -1.90 (-8.4%) = |
| `keystroke.p95_us` | 111.00 `██░░░░░░░░░░` | 104.00 `█░░░░░░░░░░░` | -7.00 (-6.3%) = |
| `keystroke.p99_us` | 181.80 `███░░░░░░░░░` | 164.00 `██░░░░░░░░░░` | -17.80 (-9.8%) = |
| `ja.convert.avg_us` | 360.45 `█████░░░░░░░` | 339.06 `█████░░░░░░░` | -21.39 (-5.9%) = |
| `ja.convert.p50_us` | 317.00 `████░░░░░░░░` | 311.00 `████░░░░░░░░` | -6.00 (-1.9%) = |
| `ja.convert.p95_us` | 592.00 `████████░░░░` | 571.60 `████████░░░░` | -20.40 (-3.4%) = |
| `ja.convert.p99_us` | 860.00 `████████████` | 666.60 `█████████░░░` | -193.40 (-22.5%) = |

### 시작·메모리 비교 (낮을수록 좋음, 막대 행별 비교)

| 항목 | 0.10.9 | 0.10.10 | 변화 |
| --- | ---: | ---: | ---: |
| `load.ms` | 30.86 `████████████` | 19.83 `████████░░░░` | -11.03 (-35.7%) ▲ |
| `heap.loaded_kb` | 2468.14 `████████████` | 2468.14 `████████████` | +0.00 (+0.0%) = |
| `heap.peak_kb` | 14047.65 `████████████` | 14047.65 `████████████` | +0.00 (+0.0%) = |
| `ja.load.ms` | 5.60 `████████████` | 4.23 `█████████░░░` | -1.37 (-24.5%) = |
| `ja.heap.loaded_kb` | 2998.60 `████████████` | 2998.60 `████████████` | +0.00 (+0.0%) = |

## 0.10.10

commit cfd3096, 2026-10-23, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enkoja` (영·한·일), lab 0.2.12.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 968 |
| `tacit_tip.dll` (x86) | 778 |
| `tacit-config.exe` | 3339 |
| `tacit-windows-ja.zip` | 2418 |
| `tacit-pack-en-0.10.10.zip` | 446 |
| `tacit-pack-ko-0.10.10.zip` | 450 |
| `tacit-pack-ja-0.10.10.zip` | 14628 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.52 |
| `acc.en.freq/none` | 98.85 |
| `acc.en.freq/after-ko` | 98.80 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.03 |
| `big.ko.sent.token` | 99.81 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 97.70 |
| `big.en.sent.sentence` | 99.90 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 96.94 |
| `big.cs.number.sentence` | 99.05 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |
| `ja.convert.ALL.top1` | 85.94 |
| `ja.convert.ALL.top5` | 96.88 |
| `ja.detect.none.top1` | 100.00 |
| `ja.detect.after-ja.top1` | 100.00 |
| `ja.guard.english.top1` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 7.08 |
| dev | 98.98 | 99.69 | 99.37 | 7.24 |
| writing | 98.78 | 99.77 | 99.75 | 8.34 |
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
| `commit.avg_us` | 75.77 |
| `commit.p50_us` | 63.20 |
| `commit.p95_us` | 170.40 |
| `commit.p99_us` | 238.70 |
| `keystroke.avg_us` | 34.50 |
| `keystroke.p50_us` | 20.80 |
| `keystroke.p95_us` | 104.00 |
| `keystroke.p99_us` | 164.00 |
| `ja.convert.avg_us` | 339.06 |
| `ja.convert.p50_us` | 311.00 |
| `ja.convert.p95_us` | 571.60 |
| `ja.convert.p99_us` | 666.60 |
| `load.ms` | 19.83 |
| `heap.loaded_kb` | 2468.14 |
| `heap.peak_kb` | 14047.65 |
| `ja.load.ms` | 4.23 |
| `ja.heap.loaded_kb` | 2998.60 |

## 0.10.9

commit b58cc6d, 2026-10-23, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enkoja` (영·한·일), lab 0.2.12.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 968 |
| `tacit_tip.dll` (x86) | 778 |
| `tacit-config.exe` | 3339 |
| `tacit-windows-ja.zip` | 2418 |
| `tacit-pack-en-0.10.9.zip` | 446 |
| `tacit-pack-ko-0.10.9.zip` | 451 |
| `tacit-pack-ja-0.10.9.zip` | 14628 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.52 |
| `acc.en.freq/none` | 98.85 |
| `acc.en.freq/after-ko` | 98.80 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.03 |
| `big.ko.sent.token` | 99.81 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 97.70 |
| `big.en.sent.sentence` | 99.90 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 96.94 |
| `big.cs.number.sentence` | 99.05 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |
| `ja.convert.ALL.top1` | 85.94 |
| `ja.convert.ALL.top5` | 96.88 |
| `ja.detect.none.top1` | 100.00 |
| `ja.detect.after-ja.top1` | 100.00 |
| `ja.guard.english.top1` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 8.42 |
| dev | 98.98 | 99.69 | 99.37 | 9.05 |
| writing | 98.78 | 99.77 | 99.75 | 10.60 |
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
| `commit.avg_us` | 78.49 |
| `commit.p50_us` | 68.40 |
| `commit.p95_us` | 172.30 |
| `commit.p99_us` | 257.60 |
| `keystroke.avg_us` | 37.14 |
| `keystroke.p50_us` | 22.70 |
| `keystroke.p95_us` | 111.00 |
| `keystroke.p99_us` | 181.80 |
| `ja.convert.avg_us` | 360.45 |
| `ja.convert.p50_us` | 317.00 |
| `ja.convert.p95_us` | 592.00 |
| `ja.convert.p99_us` | 860.00 |
| `load.ms` | 30.86 |
| `heap.loaded_kb` | 2468.14 |
| `heap.peak_kb` | 14047.65 |
| `ja.load.ms` | 5.60 |
| `ja.heap.loaded_kb` | 2998.60 |

## 0.10.3

commit 5816b5d (작업 트리 변경 포함), 2026-10-19, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enkoja` (영·한·일), lab 0.2.4.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 영·한판(tacit-windows.zip)과 일본어 포함판(tacit-windows-ja.zip)은 프로그램과 설치 zip이 다르고, 팩 목록도 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 962 |
| `tacit_tip.dll` (x86) | 773 |
| `tacit-config.exe` | 3285 |
| `tacit-windows-ja.zip` | 2389 |
| `tacit-pack-en-0.10.3.zip` | 446 |
| `tacit-pack-ko-0.10.3.zip` | 450 |
| `tacit-pack-ja-0.10.3.zip` | 14628 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.52 |
| `acc.en.freq/none` | 98.85 |
| `acc.en.freq/after-ko` | 98.80 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.03 |
| `big.ko.sent.token` | 99.81 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 97.70 |
| `big.en.sent.sentence` | 99.90 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 96.94 |
| `big.cs.number.sentence` | 99.05 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |
| `ja.convert.ALL.top1` | 85.94 |
| `ja.convert.ALL.top5` | 96.88 |
| `ja.detect.none.top1` | 100.00 |
| `ja.detect.after-ja.top1` | 100.00 |
| `ja.guard.english.top1` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 4.47 |
| dev | 98.98 | 99.69 | 99.37 | 4.68 |
| writing | 98.78 | 99.77 | 99.75 | 5.99 |
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
| `commit.avg_us` | 46.94 |
| `commit.p50_us` | 42.50 |
| `commit.p95_us` | 99.90 |
| `commit.p99_us` | 143.50 |
| `keystroke.avg_us` | 22.07 |
| `keystroke.p50_us` | 14.00 |
| `keystroke.p95_us` | 64.30 |
| `keystroke.p99_us` | 101.30 |
| `ja.convert.avg_us` | 191.27 |
| `ja.convert.p50_us` | 173.10 |
| `ja.convert.p95_us` | 320.90 |
| `ja.convert.p99_us` | 386.00 |
| `load.ms` | 18.74 |
| `heap.loaded_kb` | 2468.14 |
| `heap.peak_kb` | 14047.56 |
| `ja.load.ms` | 2.91 |
| `ja.heap.loaded_kb` | 2998.60 |

## 0.10.2

commit 53325c6 (작업 트리 변경 포함), 2026-10-18, rustc 1.93.1 (01f6ddf75 2026-02-11)

프로필: `enkoja` (영·한·일), lab 0.2.0.

2026-10-19에 0.10.3과 같은 lab 0.2.4·데이터로 번갈아 5바퀴 다시 쟀습니다. 아래 정확도·지연·메모리는 그 중앙값입니다. 크기는 처음 기록 그대로입니다.

공통 정확도·시나리오는 해당 언어가 켜진 판별기로 측정합니다. acc.ALL은 영·한 공통 세트이며 일본어 정답률과 합산하지 않습니다.

load.ms·heap.*는 판별기 시작/단어 평가 기준입니다. enkoja의 ja.load.ms·ja.heap.loaded_kb는 별도 가나→한자 변환기 초기화/힙 증가입니다. 설치 zip은 공통이며 팩 목록만 활성 언어에 따라 다릅니다.

### 크기 (KiB)

| 항목 | KiB |
| --- | ---: |
| `tacit_tip.dll` (x64) | 957 |
| `tacit_tip.dll` (x86) | 771 |
| `tacit-config.exe` | 3277 |
| `tacit-windows.zip` | 2386 |
| `tacit-pack-en-0.10.2.zip` | 446 |
| `tacit-pack-ko-0.10.2.zip` | 450 |
| `tacit-pack-ja-0.10.2.zip` | 14628 |

### 정확도 (%)

| 세트 | 정확도 |
| --- | ---: |
| `acc.ALL` | 99.52 |
| `acc.en.freq/none` | 98.85 |
| `acc.en.freq/after-ko` | 98.80 |
| `acc.ko.words/none` | 100.00 |
| `acc.ko.vocab/none` | 100.00 |
| `big.ko.sent.sentence` | 99.03 |
| `big.ko.sent.token` | 99.81 |
| `big.ko.phrase.sentence` | 99.37 |
| `big.ko.long.sentence` | 97.70 |
| `big.en.sent.sentence` | 99.90 |
| `big.cs.particle.sentence` | 99.06 |
| `big.cs.particle.token` | 99.85 |
| `big.cs_heldout.particle.sentence` | 98.95 |
| `big.cs.bare.sentence` | 96.94 |
| `big.cs.number.sentence` | 99.05 |
| `big.ko.title.none` | 99.87 |
| `big.ko.title.after-en` | 99.37 |
| `big.learn.other_forms_before` | 100.00 |
| `big.learn.other_forms_after` | 100.00 |
| `ja.convert.ALL.top1` | 85.94 |
| `ja.convert.ALL.top5` | 96.88 |
| `ja.detect.none.top1` | 100.00 |
| `ja.detect.after-ja.top1` | 100.00 |
| `ja.guard.english.top1` | 100.00 |

### 유형별 시나리오 (문장 정답률 %, token은 단어 단위, chain은 4문장 이어 입력)

| 유형 | 문장 | token | chain.token | µs/글자 |
| --- | ---: | ---: | ---: | ---: |
| chat | 96.51 | 98.78 | 99.17 | 4.64 |
| dev | 98.98 | 99.69 | 99.37 | 6.81 |
| writing | 98.78 | 99.77 | 99.75 | 6.89 |
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
| `commit.avg_us` | 84.15 |
| `commit.p50_us` | 54.30 |
| `commit.p95_us` | 257.30 |
| `commit.p99_us` | 387.30 |
| `keystroke.avg_us` | 37.78 |
| `keystroke.p50_us` | 22.10 |
| `keystroke.p95_us` | 136.00 |
| `keystroke.p99_us` | 245.90 |
| `ja.convert.avg_us` | 246.87 |
| `ja.convert.p50_us` | 222.70 |
| `ja.convert.p95_us` | 438.70 |
| `ja.convert.p99_us` | 509.50 |
| `load.ms` | 16.89 |
| `heap.loaded_kb` | 2468.13 |
| `heap.peak_kb` | 14047.55 |
| `ja.load.ms` | 3.22 |
| `ja.heap.loaded_kb` | 2998.60 |
