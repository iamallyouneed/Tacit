#![cfg(feature = "embedded-ja")]

//! Port of JapaneseSessionTests + JapaneseDetectionTests + the Japanese cases of
//! EndToEndTests and LiveConversionTests, driven through a fake text field and a deterministic
//! converter. Where the original relied on AzooKey's candidate order, the same invariants are
//! checked against `FakeConverter`; tests that need a real kanji dictionary are adapted.
//! The original's TextReplacementTests are not ported: Windows has no system text-replacement
//! dictionary to feed `replacements`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tacit_core::session::testing::FakeTextField;
use tacit_core::session::NOT_FOUND;
use tacit_core::store::policy::Override;
use tacit_core::{
    make_engine, AppPolicies, ConversionCandidate, DecisionContext, DecisionEngine, EnglishModel,
    HeuristicEngine, InputSession, JapaneseConverter, JapaneseModel, Key, Language, Languages,
    LearningEngine, TextClient,
};

/// Deterministic converter: a few readings with fixed candidates, including a
/// first-clause candidate to exercise partial commits.
struct FakeConverter {
    learned: Mutex<Vec<String>>,
    learned_details: Mutex<Vec<(ConversionCandidate, String)>>,
    continuations: Mutex<Vec<(String, String)>>,
}

impl FakeConverter {
    fn new() -> Self {
        Self {
            learned: Mutex::new(Vec::new()),
            learned_details: Mutex::new(Vec::new()),
            continuations: Mutex::new(Vec::new()),
        }
    }

    fn learned(&self) -> Vec<String> {
        self.learned.lock().unwrap().clone()
    }
}

impl JapaneseConverter for FakeConverter {
    fn candidates_after(
        &self,
        kana: &str,
        previous: &ConversionCandidate,
    ) -> Vec<ConversionCandidate> {
        self.continuations
            .lock()
            .unwrap()
            .push((kana.to_string(), previous.text.clone()));
        self.candidates(kana)
    }

    fn candidates(&self, kana: &str) -> Vec<ConversionCandidate> {
        let count = kana.chars().count();
        match kana {
            "きょうはいいてんきですね" => vec![
                ConversionCandidate::new("今日はいい天気ですね", 12),
                ConversionCandidate::new("今日は", 4),
                ConversionCandidate::new(kana, 12),
            ],
            "いいてんきですね" => vec![
                ConversionCandidate::new("いい天気ですね", 8),
                ConversionCandidate::new(kana, 8),
            ],
            // For live conversion while the phrase is still being typed.
            "きょうはいいてんきで" => {
                vec![ConversionCandidate::new("今日はいい天気で", 10)]
            }
            "へんかん" => vec![
                ConversionCandidate::new("変換", 4),
                ConversionCandidate::new("返還", 4),
                ConversionCandidate::new("編纂", 4),
            ],
            "３がつ" => vec![
                ConversionCandidate::new("三月", 3),
                ConversionCandidate::new("３月", 3),
                ConversionCandidate::new("3月", 3),
            ],
            "１２３" => vec![
                ConversionCandidate::new("一二三", 3),
                ConversionCandidate::new("123", 3),
                ConversionCandidate::new(kana, 3),
            ],
            // More candidates than fit on one page.
            "くかく" => (1..=12)
                .map(|i| ConversionCandidate::new(format!("候補{i}"), 3))
                .collect(),
            _ => vec![ConversionCandidate::new(kana, count)],
        }
    }

    fn learn(&self, candidate: &ConversionCandidate, kana: &str) {
        self.learned.lock().unwrap().push(candidate.text.clone());
        self.learned_details
            .lock()
            .unwrap()
            .push((candidate.clone(), kana.to_string()));
    }

    fn save(&self) {}
}

struct JaRig {
    session: InputSession,
    field: FakeTextField,
    converter: Arc<FakeConverter>,
}

impl JaRig {
    fn new(name: &str) -> Self {
        Self::build(name, false, AppPolicies::default(), None)
    }

    fn live(name: &str) -> Self {
        Self::build(name, true, AppPolicies::default(), None)
    }

    fn with_policies(name: &str, policies: AppPolicies, app: &str) -> Self {
        Self::build(name, false, policies, Some(app))
    }

    fn build(name: &str, live: bool, policies: AppPolicies, app: Option<&str>) -> Self {
        let path = lexicon_path(name);
        let engine = make_engine(Some(path), &Languages::legacy(true));
        let converter = Arc::new(FakeConverter::new());
        let converter_dyn: Arc<dyn JapaneseConverter> = converter.clone();
        let mut session =
            InputSession::new(engine, policies, true).with_japanese(converter_dyn, live);
        if let Some(app) = app {
            session.activate(Some(app));
        }
        Self {
            session,
            field: FakeTextField::new(),
            converter,
        }
    }

    fn key(&mut self, key: Key) -> bool {
        self.session.handle(key, &mut self.field)
    }

    /// Letters and printable keys as themselves; keys the session passes on are typed by the
    /// field, as in a real text field. "⏎" Return, "⎋" Esc, "⇧" Shift+Esc, "⌫" Delete,
    /// "←/→/↑/↓" arrows, "␠" Shift+Space.
    fn type_(&mut self, script: &str) {
        for ch in script.chars() {
            match ch {
                '⏎' => {
                    if !self.key(Key::Enter) {
                        self.field.insert_text("\n", None);
                    }
                }
                '⎋' => {
                    self.key(Key::Escape);
                }
                '⇧' => {
                    self.key(Key::Cycle);
                }
                '⌫' => {
                    self.key(Key::Delete);
                }
                '←' => {
                    self.key(Key::Left);
                }
                '→' => {
                    self.key(Key::Right);
                }
                '↑' => {
                    self.key(Key::Up);
                }
                '↓' => {
                    self.key(Key::Down);
                }
                '␠' => {
                    self.key(Key::ShiftSpace);
                }
                c => {
                    let k = if c.is_ascii_alphabetic() {
                        Key::Letter(c)
                    } else {
                        Key::Printable(c)
                    };
                    if !self.key(k) {
                        self.field.insert_text(&c.to_string(), None);
                    }
                }
            }
        }
    }

    fn text(&self) -> String {
        self.field.text()
    }

    fn composing(&self) -> bool {
        self.field.marked_range().location != NOT_FOUND
    }
}

fn lexicon_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tacit-ja-{name}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

// Unlike JaRig::build, reopening this rig never removes the persisted store.
fn persistent_ja_rig(engine: Arc<LearningEngine>) -> JaRig {
    let converter = Arc::new(FakeConverter::new());
    JaRig {
        session: InputSession::new(engine, AppPolicies::default(), true)
            .with_japanese(converter.clone(), false),
        field: FakeTextField::new(),
        converter,
    }
}

fn correction_store(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "tacit-ja-origin-{name}-{}-{serial}.json",
        std::process::id()
    ));
    assert!(!path.exists(), "test store must be fresh: {path:?}");
    path
}

fn cycle_to(r: &mut JaRig, text: &str) {
    for _ in 0..3 {
        r.key(Key::Escape);
        if r.text() == text {
            return;
        }
    }
    panic!("language cycle did not reach {text:?}: {:?}", r.text());
}

#[test]
fn japanese_explicit_english_confirmation_persists_displayed_origin() {
    for (keys, kana) in [("siasia", "しあしあ"), ("ozzy", "おっzy")] {
        let path = correction_store(keys);
        let engine = make_engine(Some(path.clone()), &Languages::legacy(true));
        for round in 1..=2 {
            let mut r = persistent_ja_rig(engine.clone());
            r.type_(keys);
            assert_eq!(r.text(), kana);
            r.key(Key::Escape);
            assert_eq!(r.text(), keys);
            r.key(Key::Enter);
            assert_eq!(r.text(), keys);
            assert!(!r.composing());
            if round == 1 {
                assert_eq!(engine.pending().get(keys), Some(&Language::English));
                assert!(engine.learned().is_empty());
            } else {
                assert_eq!(engine.learned().get(keys), Some(&Language::English));
                assert!(engine.pending().is_empty());
            }
        }
        drop(engine);
        let reopened = make_engine(Some(path), &Languages::legacy(true));
        assert_eq!(reopened.learned().get(keys), Some(&Language::English));
        let mut r = persistent_ja_rig(reopened);
        r.type_(keys);
        assert_eq!(r.text(), keys);
    }
}

#[test]
fn japanese_cycle_back_to_displayed_origin_does_not_learn() {
    for (keys, kana) in [("siasia", "しあしあ"), ("ozzy", "おっzy")] {
        let engine = make_engine(Some(correction_store("cycle")), &Languages::legacy(true));
        let mut r = persistent_ja_rig(engine.clone());
        r.type_(keys);
        assert_eq!(r.text(), kana);
        // Multiple unchanged full cycles remain one choice relative to the first display.
        for _ in 0..2 {
            cycle_to(&mut r, kana);
        }
        r.key(Key::Enter);
        assert_eq!(r.text(), kana);
        assert!(engine.pending().is_empty());
        assert!(engine.learned().is_empty());
    }
}

#[test]
fn japanese_post_boundary_english_correction_is_a_positive_control() {
    for (keys, kana) in [("siasia", "しあしあ"), ("ozzy", "おっzy")] {
        let engine = make_engine(Some(correction_store("boundary")), &Languages::legacy(true));
        for round in 1..=2 {
            let mut r = persistent_ja_rig(engine.clone());
            r.type_(keys);
            assert_eq!(r.text(), kana);
            r.type_("🙂");
            assert!(
                !r.composing(),
                "the delimiter must commit the Japanese word"
            );
            assert_eq!(r.text(), format!("{kana}🙂"));
            assert_eq!(r.field.cursor, kana.encode_utf16().count() + 2);
            r.key(Key::Escape);
            assert_eq!(r.text(), format!("{keys}🙂"));
            assert_eq!(r.field.cursor, keys.encode_utf16().count() + 2);
            r.key(Key::Other);
            assert_eq!(r.text(), format!("{keys}🙂"));
            assert_eq!(r.field.cursor, keys.encode_utf16().count() + 2);
            if round == 1 {
                assert_eq!(engine.pending().get(keys), Some(&Language::English));
                assert!(engine.learned().is_empty());
            } else {
                assert_eq!(engine.learned().get(keys), Some(&Language::English));
            }
        }
    }
}

#[test]
fn japanese_space_uses_origin_before_conversion_releases_the_word() {
    let engine = make_engine(Some(correction_store("space")), &Languages::legacy(true));
    let mut r = persistent_ja_rig(engine.clone());
    r.type_("siasia");
    assert_eq!(r.text(), "しあしあ");
    cycle_to(&mut r, "しあしあ");
    r.type_(" ");
    assert!(r.session.conversion().is_some());
    assert_eq!(r.text(), "しあしあ");
    // Space records before moving keys into Conversion; the original display is Japanese.
    assert!(engine.pending().is_empty());
    assert!(engine.learned().is_empty());
    r.key(Key::Enter);
    assert_eq!(r.converter.learned(), ["しあしあ"]);
    assert!(engine.pending().is_empty());
    assert!(engine.learned().is_empty());
}

#[test]
fn japanese_conversion_cancel_restores_originless_fallback_and_deduplicates() {
    let engine = make_engine(Some(correction_store("restore")), &Languages::legacy(true));
    let mut r = persistent_ja_rig(engine.clone());
    r.type_("siasia");
    cycle_to(&mut r, "しあしあ");
    r.type_(" ");
    assert!(engine.pending().is_empty());
    let trace = r.session.last_trace().unwrap();
    assert_eq!(trace.keys, "siasia");
    assert!(
        trace.partial,
        "valid origin must avoid hypothetical final scoring"
    );
    r.key(Key::Escape);
    assert!(r.session.conversion().is_none());
    assert_eq!(r.text(), "しあしあ");
    // Restoring keys invalidates origin and requests current-token final scoring.
    // Delivery still deduplicates the Japanese choice recorded by the first Space,
    // even though that first choice equalled its baseline and taught nothing.
    r.type_(" ");
    let trace = r.session.last_trace().unwrap();
    assert_eq!(trace.keys, "siasia");
    assert!(
        !trace.partial,
        "restored keys must use current-token final fallback"
    );
    assert!(engine.pending().is_empty());
    for _ in 0..2 {
        r.type_("⎋ ");
        let trace = r.session.last_trace().unwrap();
        assert_eq!(trace.keys, "siasia");
        assert!(!trace.partial);
        assert!(engine.pending().is_empty());
        assert!(engine.learned().is_empty());
    }
    r.key(Key::Enter);
    assert_eq!(r.converter.learned(), ["しあしあ"]);
    assert!(engine.pending().is_empty());
    assert!(engine.learned().is_empty());
}

#[test]
fn japanese_kana_delete_retype_invalidates_origin_without_recapturing() {
    let engine = make_engine(Some(correction_store("respell")), &Languages::legacy(true));
    let mut r = persistent_ja_rig(engine.clone());
    r.type_("siasia");
    assert_eq!(r.text(), "しあしあ");
    cycle_to(&mut r, "しあしあ");
    r.key(Key::Delete);
    assert_eq!(r.text(), "しあし");
    r.type_("a");
    assert_eq!(r.text(), "しあしあ");
    cycle_to(&mut r, "siasia");
    assert_eq!(
        engine
            .decide("siasia", &DecisionContext::default())
            .language,
        Language::English
    );
    r.key(Key::Enter);
    assert_eq!(r.text(), "siasia");
    // Identical final bytes do not revive the first Japanese origin.
    assert!(engine.pending().is_empty());
    assert!(engine.learned().is_empty());
}

#[test]
fn japanese_conversion_n_marker_normalization_uses_current_final_fallback() {
    let engine = make_engine(Some(correction_store("n-marker")), &Languages::legacy(true));
    let mut r = persistent_ja_rig(engine.clone());
    r.type_("henkan");
    assert_eq!(r.text(), "へんかn");
    cycle_to(&mut r, "へんかn");
    r.type_(" ");
    assert_eq!(r.text(), "変換");
    r.key(Key::Escape);
    assert_eq!(r.text(), "へんかん");
    assert!(r.session.conversion().is_none());
    assert!(engine.pending().is_empty());
    cycle_to(&mut r, "henkan");
    // Leaving restored Japanese removes the synthetic apostrophe, never learning
    // henkan' or reviving provenance captured before conversion.
    let baseline = engine
        .decide("henkan", &DecisionContext::default())
        .language;
    r.key(Key::Enter);
    assert_eq!(r.text(), "henkan");
    assert!(!engine.pending().contains_key("henkan'"));
    assert!(engine.learned().is_empty());
    if baseline == Language::English {
        assert!(engine.pending().is_empty());
    } else {
        assert_eq!(engine.pending().len(), 1);
        assert_eq!(engine.pending().get("henkan"), Some(&Language::English));
    }
}

fn trilingual() -> HeuristicEngine {
    HeuristicEngine::with_japanese(EnglishModel::shared(), JapaneseModel::shared())
}

// ---- JapaneseDetectionTests ----

#[test]
fn japanese_phrases_are_detected() {
    let engine = trilingual();
    for (keys, kana) in [
        ("konnichiha", "こんにちは"),
        ("watashihagakuseidesu", "わたしはがくせいです"),
        ("arigatougozaimasu", "ありがとうございます"),
        ("kyouhaiitenkidesune", "きょうはいいてんきですね"),
        ("ra-mentabetai", "らーめんたべたい"),
        ("sumimasen", "すみません"),
        ("yoroshikuonegaishimasu", "よろしくおねがいします"),
        ("daijoubu", "だいじょうぶ"),
    ] {
        let d = engine.decide(keys, &DecisionContext::default());
        assert_eq!(d.language, Language::Japanese, "{keys}: {}", d.explanation);
        assert_eq!(d.output, kana, "{keys}");
    }
}

#[test]
fn english_stays_english_with_japanese_on() {
    let engine = trilingual();
    for word in [
        "hello", "world", "keyboard", "wanna", "gotta", "the", "people", "thanks", "commit", "same",
    ] {
        let d = engine.decide(word, &DecisionContext::default());
        assert_eq!(d.language, Language::English, "{word}: {}", d.explanation);
    }
}

#[test]
fn korean_stays_korean_with_japanese_on() {
    let engine = trilingual();
    for (keys, hangul) in [
        ("dkssudgktpdy", "안녕하세요"),
        ("gksrmf", "한글"),
        ("rkatkgkqslek", "감사합니다"),
        ("tkfkd", "사랑"),
    ] {
        let d = engine.decide(keys, &DecisionContext::default());
        assert_eq!(d.language, Language::Korean, "{keys}: {}", d.explanation);
        assert_eq!(d.output, hangul, "{keys}");
    }
}

#[test]
fn japanese_context_helps_ambiguous_words() {
    // "kami" (紙/髪/神) after Japanese reads as Japanese.
    let engine = trilingual();
    let d = engine.decide(
        "kamiwo",
        &DecisionContext {
            previous_language: Some(Language::Japanese),
            partial: false,
            ..Default::default()
        },
    );
    assert_eq!(d.language, Language::Japanese, "{}", d.explanation);
}

#[test]
fn japanese_is_off_by_default() {
    let engine = make_engine(None, &Languages::legacy(false));
    assert_ne!(
        engine
            .decide("konnichiha", &DecisionContext::default())
            .language,
        Language::Japanese
    );
}

/// SegmenterTests.decisionUsesSegmentation: English inside a phrase is kept as typed.
#[test]
fn decision_uses_segmentation() {
    let d = trilingual().decide("kyouhagoogledekensaku", &DecisionContext::default());
    assert_eq!(d.language, Language::Japanese, "{}", d.explanation);
    assert_eq!(d.output, "きょうはgoogleでけんさく");
}

// ---- JapaneseSessionTests ----

#[test]
fn preview_shows_kana_while_typing() {
    let mut r = JaRig::new("ja-preview");
    r.type_("kyouhaiitenk");
    assert_eq!(r.text(), "きょうはいいてんk");
}

#[test]
fn space_converts_and_return_commits() {
    let mut r = JaRig::new("ja-space");
    r.type_("kyouhaiitenkidesune ");
    assert!(r.session.conversion().is_some());
    assert_eq!(r.text(), "今日はいい天気ですね");
    assert!(r.key(Key::Enter));
    assert_eq!(r.text(), "今日はいい天気ですね");
    assert!(r.session.conversion().is_none());
    assert_eq!(r.converter.learned(), ["今日はいい天気ですね"]);
}

#[test]
fn space_cycles_candidates_and_numbers_pick() {
    let mut r = JaRig::new("ja-cycle");
    r.type_("henkan ");
    assert_eq!(r.text(), "変換");
    r.type_(" ");
    assert_eq!(r.text(), "返還");
    r.key(Key::Up);
    assert_eq!(r.text(), "変換");
    r.type_("3");
    assert_eq!(r.text(), "編纂");
    assert!(r.session.conversion().is_none());
}

#[test]
fn candidate_pages_follow_the_selection() {
    let mut r = JaRig::new("ja-pages");
    r.type_("kukaku ");
    let page = r.session.candidate_page().unwrap();
    assert_eq!((page.page, page.selected, page.pages), (0, 0, 2));
    assert_eq!(page.texts.len(), 9);
    assert_eq!(page.texts[0], "候補1");
    assert_eq!(page.texts[8], "候補9");
    // Space nine times lands on the tenth candidate: the second page's first line.
    for _ in 0..9 {
        r.key(Key::Printable(' '));
    }
    assert_eq!(r.text(), "候補10");
    let page = r.session.candidate_page().unwrap();
    assert_eq!((page.page, page.selected, page.pages), (1, 0, 2));
    assert_eq!(page.texts, ["候補10", "候補11", "候補12"]);
    // Space wraps back to the start.
    for _ in 0..3 {
        r.key(Key::Printable(' '));
    }
    assert_eq!(r.text(), "候補1");
    let page = r.session.candidate_page().unwrap();
    assert_eq!((page.page, page.selected), (0, 0));
}

#[test]
fn numbers_pick_from_the_current_page() {
    let mut r = JaRig::new("ja-numpages");
    r.type_("kukaku ");
    // On the second page, "1" picks its first line.
    for _ in 0..9 {
        r.key(Key::Printable(' '));
    }
    r.type_("1");
    assert_eq!(r.text(), "候補10");
    assert!(r.session.conversion().is_none());
    assert_eq!(r.converter.learned(), ["候補10"]);
}

#[test]
fn a_number_past_the_page_confirms_and_types_on() {
    let mut r = JaRig::new("ja-numpast");
    r.type_("kukaku ");
    for _ in 0..9 {
        r.key(Key::Printable(' '));
    }
    r.type_("4");
    assert_eq!(r.text(), "候補104");
    assert!(r.session.conversion().is_none());
}

#[test]
fn candidate_page_needs_a_list() {
    let mut r = JaRig::new("ja-pagelist");
    assert!(r.session.candidate_page().is_none());
    // A reading with a single candidate (the fake converter's default) shows no window.
    r.type_("takusan ");
    assert_eq!(r.text(), "たくさん");
    assert!(r.session.conversion().is_some());
    assert!(r.session.candidate_page().is_none());
}

#[test]
fn first_clause_candidate_keeps_converting_the_rest() {
    let mut r = JaRig::new("ja-clause");
    r.type_("kyouhaiitenkidesune ");
    r.key(Key::Down); // 今日は (covers 4 kana)
    assert_eq!(r.text(), "今日はいいてんきですね");
    r.key(Key::Enter);
    assert_eq!(r.text(), "今日はいい天気ですね");
    assert_eq!(r.session.conversion().unwrap().kana, "いいてんきですね");
    r.key(Key::Enter);
    assert!(r.session.conversion().is_none());
    assert_eq!(r.text(), "今日はいい天気ですね");
}

#[test]
fn escape_returns_to_kana() {
    let mut r = JaRig::new("ja-esc");
    r.type_("henkan ");
    r.key(Key::Escape);
    assert!(r.session.conversion().is_none());
    assert_eq!(r.text(), "へんかん");
    r.key(Key::Delete);
    assert_eq!(r.text(), "へんか");
}

#[test]
fn delete_takes_off_a_whole_kana() {
    let mut r = JaRig::new("ja-del");
    r.type_("kannkoku");
    assert_eq!(r.text(), "かんこく");
    r.key(Key::Delete);
    assert_eq!(r.text(), "かんこ");
    r.key(Key::Delete);
    assert_eq!(r.text(), "かん");
    r.type_("ji ");
    assert!(r.converter.learned().is_empty());
    assert_eq!(r.session.conversion().unwrap().kana, "かんじ");
}

#[test]
fn delete_keeps_a_pending_letter_separate() {
    let mut r = JaRig::new("ja-delpend");
    r.type_("kannkok");
    assert_eq!(r.text(), "かんこk");
    r.key(Key::Delete);
    assert_eq!(r.text(), "かんこ");
}

#[test]
fn delete_splits_contracted_and_doubled_kana() {
    let mut r = JaRig::new("ja-del2");
    r.type_("kyouha");
    r.key(Key::Delete);
    r.key(Key::Delete);
    assert_eq!(r.text(), "きょ");
    r.key(Key::Delete);
    assert_eq!(r.text(), "き");
    r.type_("tte");
    assert_eq!(r.text(), "きって");
    r.key(Key::Delete);
    assert_eq!(r.text(), "きっ");
    r.type_("ta");
    assert_eq!(r.text(), "きった");
}

#[test]
fn return_without_converting_commits_kana_only() {
    let mut r = JaRig::new("ja-ret");
    r.type_("arigatou");
    assert_eq!(r.text(), "ありがとう");
    let consumed = r.key(Key::Enter);
    assert!(consumed); // a Japanese IME's Return only confirms the text
    assert_eq!(r.text(), "ありがとう");
}

#[test]
fn preview_matches_what_gets_committed() {
    // "kanji" is an English dictionary word: outside Japanese text it stays English,
    // and the preview says so before Return.
    let mut r = JaRig::new("ja-dict");
    r.type_("kanji");
    assert_eq!(r.text(), "kanji");
    let consumed = r.key(Key::Enter);
    assert!(consumed); // Return only confirms, whatever the language
    assert_eq!(r.text(), "kanji");
    assert!(!r.composing());
}

#[test]
fn confirmation_keeps_the_visible_language_with_japanese_enabled() {
    for count in [0, 4] {
        for boundary in [Key::Printable(' '), Key::Enter] {
            for (keys, shown) in [
                ("dustmqgot", "연습햇"),
                ("siasia", "しあしあ"),
                ("githubdp", "github에"),
                ("akfgkfRK", "말할까"),
            ] {
                let mut r = JaRig::new("confirm-visible");
                let path = lexicon_path("confirm-visible");
                std::fs::write(
                    &path,
                    serde_json::json!({
                        "learned": {}, "once": {}, "shapes": {"medium:korean": count}
                    })
                    .to_string(),
                )
                .unwrap();
                r.session.engine().reload();
                r.field.insert_text("앞😀  뒤", None);
                r.field.cursor = "앞😀 ".encode_utf16().count();
                r.type_(keys);
                let expected = format!("앞😀 {shown} 뒤");
                assert_eq!(r.text(), expected, "{keys}, count={count}");
                r.key(boundary);
                assert_eq!(r.text(), expected, "{keys}, count={count}, {boundary:?}");
                if r.session.is_converting() {
                    r.key(Key::Enter);
                }
                assert_eq!(r.text(), expected);
                assert!(!r.composing());
                assert_eq!(
                    r.field.selected_range().location,
                    format!("앞😀 {shown}").encode_utf16().count()
                );
                drop(r);
                std::fs::remove_file(path).unwrap();
            }
        }
    }
}

#[test]
fn punctuation_stays_inside_japanese_phrase() {
    let mut r = JaRig::new("ja-punct");
    r.type_("arigatou.");
    assert_eq!(r.text(), "ありがとう。");
    r.type_("ra-men");
    assert_eq!(r.text(), "ありがとう。らーめn"); // ん waits for the next key, like any Japanese IME
}

/// After Japanese a symbol is full-width and underlined, as in a Japanese IME: Return only
/// confirms it, the next letter starts a word of its own.
#[test]
fn symbol_after_japanese_waits_for_return() {
    let mut r = JaRig::new("ja-sym");
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("!?");
    assert_eq!(r.text(), "変換！？");
    assert!(r.composing());
    assert!(r.key(Key::Enter));
    assert_eq!(r.text(), "変換！？");
    assert!(!r.composing());
    r.type_("_");
    r.key(Key::Delete);
    assert_eq!(r.text(), "変換！？");
    r.type_("/wo");
    assert_eq!(r.text(), "変換！？・を");
}

#[test]
fn symbol_after_japanese_converts_with_space() {
    let mut r = JaRig::new("ja-sym2");
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("_ ");
    assert_eq!(r.session.conversion().unwrap().kana, "＿");
}

#[test]
fn numbers_after_japanese_stay_half_width() {
    let mut r = JaRig::new("ja-num0");
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("3.5:");
    assert_eq!(r.text(), "変換3.5:");
}

#[test]
fn brackets_close_the_way_they_opened() {
    let mut r = JaRig::new("ja-brackets");
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("(hello)");
    r.key(Key::Enter);
    assert_eq!(r.text(), "変換（hello）");
}

#[test]
fn symbols_join_only_a_finished_japanese_word() {
    let mut r = JaRig::new("ja-nihon");
    r.type_("nihon.");
    assert_eq!(r.text(), "にほん。");
}

#[test]
fn symbols_in_english_stay_half_width() {
    let mut r = JaRig::new("ja-ensym");
    r.type_("foo_bar (baz) a/b ");
    assert_eq!(r.text(), "foo_bar (baz) a/b ");
}

/// Off by default: digits after Japanese stay half-width and go to the app.
#[test]
fn digits_stay_half_width_by_default() {
    let mut r = JaRig::new("ja-digits0");
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("123");
    assert_eq!(r.text(), "変換123");
}

#[test]
fn full_width_digits_after_japanese() {
    let mut r = JaRig::new("ja-digits1");
    r.session.set_full_width_digits(true);
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("123");
    assert_eq!(r.text(), "変換１２３");
    assert!(r.composing());
    r.type_(" ");
    let texts: Vec<&str> = r
        .session
        .conversion()
        .unwrap()
        .candidates
        .iter()
        .take(3)
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(texts, ["１２３", "123", "一二三"]);
    r.type_(" ");
    r.key(Key::Enter);
    assert_eq!(r.text(), "変換123");
}

/// Addresses and phone numbers: the punctuation inside a number is full-width too, and Delete
/// takes it off one character at a time.
#[test]
fn full_width_number_punctuation() {
    let mut r = JaRig::new("ja-digits2");
    r.session.set_full_width_digits(true);
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("03-1234");
    assert_eq!(r.text(), "変換０３－１２３４");
    r.key(Key::Delete);
    r.key(Key::Delete);
    assert_eq!(r.text(), "変換０３－１２");
}

/// Inside a phrase a number joins it, and candidates that keep the digits come first.
#[test]
fn number_inside_a_phrase() {
    let mut r = JaRig::new("ja-digits3");
    r.session.set_full_width_digits(true);
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("3gatsu ");
    let texts: Vec<&str> = r
        .session
        .conversion()
        .unwrap()
        .candidates
        .iter()
        .take(3)
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(texts, ["３月", "3月", "三月"]);
}

#[test]
fn digits_after_english_stay_half_width() {
    let mut r = JaRig::new("ja-digits4");
    r.session.set_full_width_digits(true);
    r.type_("hello 123 ");
    assert_eq!(r.text(), "hello 123 ");
}

#[test]
fn full_width_space_after_japanese() {
    let mut r = JaRig::new("ja-fwspace");
    r.type_("henkan ");
    r.key(Key::Enter);
    assert!(r.key(Key::ShiftSpace)); // ⇧Space: the other width
    assert_eq!(r.text(), "変換　");
    r.session.set_full_width_space(true);
    r.type_(" ");
    assert_eq!(r.text(), "変換　　");
    assert!(!r.key(Key::ShiftSpace)); // the app types the half-width one
}

#[test]
fn no_full_width_where_the_app_says_no() {
    let policies = AppPolicies::new([(
        "test.terminal".to_string(),
        Override {
            full_width: Some(false),
            ..Default::default()
        },
    )]);
    let mut r = JaRig::with_policies("ja-nofw", policies, "test.terminal");
    r.session.set_full_width_digits(true);
    r.session.set_full_width_space(true);
    r.type_("henkan ");
    r.key(Key::Enter);
    r.type_("12 ");
    assert_eq!(r.text(), "変換12 ");
}

#[test]
fn number_helpers() {
    assert_eq!(InputSession::digits_of("東京都１－２"), "12");
    assert_eq!(InputSession::half_width("０３－１２３４"), "03-1234");
    assert_eq!(
        tacit_core::RomajiComposer::compose("03-1234", false).text,
        "０３－１２３４"
    );
    assert_eq!(
        tacit_core::RomajiComposer::compose("ra-men", false).text,
        "らーめん"
    );
}

#[test]
fn typing_on_after_conversion_commits_it() {
    let mut r = JaRig::new("ja-typing");
    r.type_("henkan ");
    r.type_("wo");
    assert_eq!(r.text(), "変換を");
    assert!(r.session.conversion().is_none());
}

#[test]
fn english_and_korean_still_work() {
    let mut r = JaRig::new("ja-others");
    r.type_("hello dkssud ");
    assert_eq!(r.text(), "hello 안녕 ");
}

#[test]
fn keys_remainder_after_kana() {
    assert_eq!(InputSession::keys_after("kyouhaiitenki", 4), "iitenki");
    assert_eq!(InputSession::keys_after("kitte", 2), "te");
    assert_eq!(InputSession::keys_after("konnichiha", 2), "nichiha");
}

/// The TIP asks `claims_key` before letting a key go to the app: it must be true exactly when
/// `handle` would consume the key with nothing being composed.
#[test]
fn claims_what_it_would_consume_after_japanese() {
    let mut r = JaRig::new("ja-claims");
    r.type_("henkan ");
    r.key(Key::Enter);
    // Half-width by default: digits and spaces go to the app…
    assert!(!r.session.claims_key(Key::Printable('1')));
    assert!(!r.session.claims_key(Key::Printable(' ')));
    // …but ⇧Space still offers the full-width one, and a sentence symbol is ours.
    assert!(r.session.claims_key(Key::ShiftSpace));
    assert!(r.session.claims_key(Key::Printable(',')));
    assert!(!r.session.claims_key(Key::Letter('a')));
    r.session.set_full_width_digits(true);
    assert!(r.session.claims_key(Key::Printable('1')));
    r.session.set_full_width_space(true);
    assert!(r.session.claims_key(Key::Printable(' ')));
    assert!(!r.session.claims_key(Key::ShiftSpace)); // Space now types full-width itself
}

// ---- EndToEndTests (Japanese cases; engine output reduced to kana where FakeConverter has none) ----

#[test]
fn three_languages_in_one_line() {
    let mut r = JaRig::new("ja-e2e1");
    r.type_("dkssudgktpdy hello ");
    r.type_("kyouhaiitenkidesune ⏎");
    assert_eq!(r.text(), "안녕하세요 hello 今日はいい天気ですね");
}

/// The reading keeps `google` as typed (no kanji dictionary in the fake).
#[test]
fn english_inside_japanese() {
    let mut r = JaRig::new("ja-e2e2");
    r.type_("kyouhagoogledekensaku ⏎");
    assert_eq!(r.text(), "きょうはgoogleでけんさく");
}

#[test]
fn english_word_with_a_particle() {
    // With no kanji engine the Japanese halves stay kana (今日は → きょうは).
    for (script, expected) in [
        ("githubde ⏎", "githubで"),
        ("kyouha ⏎githubwo ⏎", "きょうはgithubを"),
        ("slackkara ⏎", "slackから"),
        ("i have 2 cats. githubde⏎", "i have 2 cats. githubで"),
        ("ok githubwo ⏎", "ok githubを"),
        ("jayne ", "jayne "),
        ("photo ", "photo "),
        ("guswo ", "현재 "),
        ("guswosms ", "현재는 "),
        ("kyouha ⏎guswo ", "きょうは현재 "), // gus + を? no: 현재
    ] {
        let mut r = JaRig::new("ja-e2e3");
        r.type_(script);
        assert_eq!(r.text(), expected, "{script}");
    }
}

/// In a terminal (0.97), 어떻게 after an English word is 0.969 sure: what was shown is committed.
#[test]
fn space_commits_what_the_preview_shows() {
    let terminal = terminal_policies();
    let mut r = JaRig::with_policies("ja-e2e4", terminal, "test.terminal");
    r.type_("esc djEjgrp");
    assert_eq!(r.text(), "esc 어떻게"); // still composing (marked)
    r.type_(" todrkrgo?");
    assert_eq!(r.text(), "esc 어떻게 생각해?");
}

#[test]
fn terminal_escape_goes_to_the_app() {
    let mut r = JaRig::with_policies("ja-e2e5", terminal_policies(), "test.terminal");
    r.type_("dkssud");
    assert!(!r.key(Key::Escape)); // passed to the app
    assert_eq!(r.text(), "안녕");
    assert!(r.session.conversion().is_none());
    r.type_(" hello ");
    assert!(!r.key(Key::Escape)); // no undo after commit either
    assert_eq!(r.text(), "안녕 hello ");
}

/// ⇧Esc switches languages there, while composing and right after a commit.
#[test]
fn terminal_shift_escape_switches() {
    let mut r = JaRig::with_policies("ja-e2e6", terminal_policies(), "test.terminal");
    // hello's clean readings are English and へっぉ.
    r.type_("hello⇧");
    assert_eq!(r.text(), "へっぉ");
    r.type_("⇧ "); // back to English, then commit
    assert_eq!(r.text(), "hello ");
    r.type_("⇧");
    assert_eq!(r.text(), "へっぉ ");
}

/// Converting, backing out and converting again is still one correction: not learned.
#[test]
fn reconverting_counts_as_one_correction() {
    let mut r = JaRig::new("ja-e2e7");
    r.type_("hello⎋ ⎋ ⏎");
    assert_ne!(r.text(), "hello");
    r.type_(" hello ");
    assert!(r.text().ends_with(" hello "), "{}", r.text());
}

#[test]
fn escape_switches_in_other_apps() {
    let mut r = JaRig::with_policies("ja-e2e8", terminal_policies(), "com.apple.TextEdit");
    r.type_("hello⎋");
    assert_eq!(r.text(), "へっぉ");
}

#[test]
fn japanese_conversions_stay_kana_without_an_engine() {
    // The original here asserted kanji (本を読む): with `FakeConverter` the reading is the
    // candidate, which is also what `KanaConverter` produces.
    for (script, expected) in [
        ("honwoyomu ⏎", "ほんをよむ"),
        ("watashihagakuseidesu ⏎", "わたしはがくせいです"),
    ] {
        let mut r = JaRig::new("ja-e2e9");
        r.type_(script);
        assert_eq!(r.text(), expected, "{script}");
    }
}

/// ←/→ move the first clause's end; Return commits it and converts the rest.
#[test]
fn clause_by_clause_with_arrows() {
    let mut r = JaRig::new("ja-e2e10");
    r.type_("kyouhaiitenkidesune ");
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 12);
    r.type_("←←←←←←←←"); // 12 → 4: きょうは
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 4);
    assert_eq!(r.text(), "きょうはいいてんきですね");
    r.key(Key::Enter);
    assert!(r.text().starts_with("きょうは"));
    assert_eq!(r.session.conversion().unwrap().kana, "いいてんきですね");
    r.key(Key::Enter);
    assert_eq!(r.text(), "きょうはいい天気ですね");
    assert!(r.session.conversion().is_none());
}

#[test]
fn clause_ends_stay_within_bounds() {
    let mut r = JaRig::new("ja-e2e11");
    r.type_("kyouha ");
    r.type_("→");
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 4); // already the whole reading
    r.type_("←←←←←←");
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 1);
    r.key(Key::Escape);
    assert!(r.session.conversion().is_none());
}

/// A clause boundary never splits an English word inside the phrase.
#[test]
fn clause_boundary_skips_english_letters() {
    let mut r = JaRig::new("ja-e2e12");
    r.type_("kyouhagoogledekensaku ");
    assert_eq!(
        r.session.conversion().unwrap().kana,
        "きょうはgoogleでけんさく"
    );
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 15);
    r.type_("←←←←←"); // 15 → 10: きょうはgoogle
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 10);
    r.type_("←"); // not 9 (googl|e) but 4: きょうは
    assert_eq!(r.session.conversion().unwrap().current().kana_count, 4);
}

#[test]
fn numbers_after_japanese_keep_their_decimal_point() {
    let mut r = JaRig::new("ja-e2e13");
    r.type_("kyouha ⏎");
    r.type_("3.5");
    assert_eq!(r.text(), "きょうは3.5");
    r.type_("desu.");
    assert!(r.text().ends_with("です。"), "{}", r.text());
}

#[test]
fn katakana_and_hiragana_shortcuts() {
    let mut r = JaRig::new("ja-e2e14");
    r.type_("ko-hi-");
    r.key(Key::Katakana);
    assert_eq!(r.text(), "コーヒー");
    r.type_(" "); // flip to the other form
    assert_eq!(r.text(), "こーひー");
    r.key(Key::Enter);
    assert_eq!(r.text(), "こーひー");
    // ⌃K on an English word is the app's shortcut, not ours.
    r.type_(" hello");
    assert!(!r.key(Key::Katakana));
    assert_eq!(r.text(), "こーひー hello");
}

/// The long-vowel mark after a start the English list knows: me, yo and do are English words,
/// but めーる, よーそろー and どーなつ must still type as Japanese. (macOS Seamtype keeps the
/// word English here, which leaves those words untypable; this port re-decides with the mark
/// in the keys instead — see belongs_to_japanese_phrase.)
#[test]
fn long_vowel_after_a_known_english_word() {
    let mut r = JaRig::new("ja-longvowel");
    r.type_("me-ru");
    assert_eq!(r.text(), "めーる");
    r.type_("⏎");
    r.type_("yo-soro-");
    assert_eq!(r.text(), "めーるよーそろー");
    r.type_("⏎");
    r.type_("do-natsu");
    assert_eq!(r.text(), "めーるよーそろーどーなつ");
}

/// An English compound whose reading is clean kana leans Japanese at the hyphen and reads back
/// with one Esc: to-do → とーど → ⎋ → to-do.
#[test]
fn clean_kana_english_compound_reads_back_with_escape() {
    let mut r = JaRig::new("ja-compound");
    r.type_("to-do");
    assert_eq!(r.text(), "とーど");
    r.type_("⎋");
    assert_eq!(r.text(), "to-do");
    r.type_("⏎");
    assert_eq!(r.text(), "to-do");
}

#[test]
fn pick_a_second_candidate() {
    // The original used a reading with several kanji candidates (かんじを); with the fake
    // converter へんかん stands in for it.
    let mut r = JaRig::new("ja-e2e15");
    r.type_("henkan ");
    let first = r.text();
    r.type_(" ");
    assert_ne!(r.text(), first);
    r.key(Key::Enter);
    assert!(r.session.conversion().is_none());
    assert_eq!(r.text(), "返還");
}

#[test]
fn fix_a_wrong_guess_with_escape() {
    let mut r = JaRig::new("ja-e2e16");
    r.type_("ozzy");
    assert_eq!(r.text(), "おっzy");
    // Correct the visible guess explicitly, rather than relying on Space to reclassify it.
    r.type_("⎋ ");
    assert_eq!(r.text(), "ozzy ");
    r.type_("⎋"); // ozzy → Korean
    assert_ne!(r.text(), "ozzy ");
    r.type_("⎋⎋"); // → Japanese → back to English
    assert_eq!(r.text(), "ozzy ");
}

// ---- LiveConversionTests (adapted; the fake's whole-reading candidate stands in for AzooKey) ----

#[test]
fn kanji_while_typing_and_return_commits_them() {
    let mut r = JaRig::live("ja-live1");
    r.type_("kyouhaiitenkidesune");
    assert_eq!(r.text(), "今日はいい天気ですね"); // still marked
    r.type_("⏎");
    assert_eq!(r.text(), "今日はいい天気ですね");
    assert!(r.session.keys().is_empty() && r.session.conversion().is_none());
}

#[test]
fn letters_waiting_for_a_vowel_stay_visible() {
    let mut r = JaRig::live("ja-live2");
    r.type_("kyouhaiitenkides");
    assert!(r.text().starts_with("今日は"), "{}", r.text());
    assert!(r.text().ends_with('s'), "{}", r.text());
}

#[test]
fn plain_confirmation_preserves_japanese_preview() {
    for live in [false, true] {
        for (keys, reading, converted, committed_tail) in [
            ("watashihag", "わたしはg", "わたしはg", ""),
            ("ozzy", "おっzy", "おっzy", ""),
            ("henkan", "へんかn", "へんかn", "ん"),
            (
                "kyouhaiitenkides",
                "きょうはいいてんきでs",
                "今日はいい天気でs",
                "",
            ),
            (
                "kyouhaiitenkiden",
                "きょうはいいてんきでn",
                "今日はいい天気でn",
                "ん",
            ),
            (
                "kyouhagoogledekensaku",
                "きょうはgoogleでけんさく",
                "きょうはgoogleでけんさく",
                "",
            ),
            ("kyouhapython", "きょうはpython", "きょうはpython", ""),
            (
                "kyouhagoogledehenkan",
                "きょうはgoogleでへんかn",
                "きょうはgoogleでへんかn",
                "ん",
            ),
        ] {
            for confirmation in 0..5 {
                let mut r = JaRig::build(
                    &format!("ja-confirm-{live}-{keys}"),
                    live,
                    AppPolicies::default(),
                    None,
                );
                r.type_(keys);
                let visible = if live { converted } else { reading };
                assert!(r.composing(), "{keys}, live={live}");
                assert_eq!(r.text(), visible, "{keys}, live={live}");
                match confirmation {
                    0 => {
                        r.key(Key::Enter);
                    }
                    1 => {
                        r.key(Key::Other);
                    }
                    2 => {
                        r.key(Key::Tab);
                    }
                    3 => r.session.commit(&mut r.field),
                    _ => r.session.commit_for_new_context(&mut r.field),
                }
                let expected = if committed_tail.is_empty() {
                    visible.to_string()
                } else {
                    format!("{}{committed_tail}", visible.strip_suffix('n').unwrap())
                };
                assert_eq!(
                    r.text(),
                    expected,
                    "{keys}, live={live}, confirmation={confirmation}"
                );
                assert!(!r.composing());
            }
        }
    }
}

#[test]
fn incomplete_japanese_consumes_enter_when_return_normally_passes_through() {
    for live in [false, true] {
        let policies = AppPolicies::new([(
            "test.return-through".to_string(),
            Override {
                return_confirms_only: Some(false),
                ..Default::default()
            },
        )]);
        let mut r = JaRig::build(
            &format!("ja-confirm-return-through-{live}"),
            live,
            policies,
            Some("test.return-through"),
        );
        r.type_("kyouhaiitenkides");
        let expected = if live {
            "今日はいい天気でs"
        } else {
            "きょうはいいてんきでs"
        };
        assert!(r.composing());
        assert_eq!(r.text(), expected);
        assert!(
            r.key(Key::Enter),
            "Japanese confirmation must consume Return"
        );
        assert_eq!(r.text(), expected);
        assert!(!r.composing());
        assert!(
            !r.key(Key::Enter),
            "Return passes through after confirmation"
        );
    }
}

#[test]
fn plain_confirmation_learns_the_visible_live_prefix() {
    let mut r = JaRig::live("ja-confirm-live-learning");
    r.type_("kyouhaiitenkides");
    assert!(r.composing());
    assert_eq!(r.text(), "今日はいい天気でs");
    r.key(Key::Enter);
    assert_eq!(r.text(), "今日はいい天気でs");
    assert_eq!(
        *r.converter.learned_details.lock().unwrap(),
        [(
            ConversionCandidate::new("今日はいい天気で", 10),
            "きょうはいいてんきで".to_string()
        )]
    );
}

#[test]
fn explicit_conversion_can_transform_a_pending_n() {
    for live in [false, true] {
        let mut r = JaRig::build(
            &format!("ja-confirm-explicit-{live}"),
            live,
            AppPolicies::default(),
            None,
        );
        r.type_("henkan");
        assert!(r.composing());
        assert_eq!(r.text(), "へんかn");
        r.type_(" ");
        let expected = if live { "返還" } else { "変換" };
        assert_eq!(r.text(), expected);
        assert!(r.session.conversion().is_some());
        r.key(Key::Enter);
        assert_eq!(r.text(), expected);
        assert_eq!(r.converter.learned(), [expected]);
    }
}

#[test]
fn space_shows_the_next_candidate() {
    let mut r = JaRig::live("ja-live3");
    r.type_("kyouhaiitenkidesune");
    let live = r.text();
    r.type_(" ");
    assert!(r.session.conversion().is_some());
    assert_ne!(r.text(), live);
    r.key(Key::Enter); // 今日は, a first-clause candidate
    assert!(r.text().starts_with("今日は"));
    r.key(Key::Enter);
    assert!(r.session.conversion().is_none());
    assert!(r.text().ends_with("ですね"), "{}", r.text());
}

#[test]
fn escape_shows_kana_then_switches_language() {
    let mut r = JaRig::live("ja-live4");
    r.type_("kyouhaiitenkidesune⎋");
    assert_eq!(r.text(), "きょうはいいてんきですね");
    r.type_("⎋");
    assert!(!r.text().contains("きょう"), "{}", r.text());
    r.type_("⎋⎋⎋");
}

#[test]
fn kana_commit_after_escape() {
    let mut r = JaRig::live("ja-live5");
    r.type_("kyouhaiitenkidesune⎋⏎");
    assert_eq!(r.text(), "きょうはいいてんきですね");
}

#[test]
fn live_other_languages_are_unchanged() {
    let mut r = JaRig::live("ja-live6");
    r.type_("dkssudgktpdy hello ");
    assert_eq!(r.text(), "안녕하세요 hello ");
}

#[test]
fn live_english_inside_phrase_and_punctuation() {
    let mut r = JaRig::live("ja-live7");
    r.type_("kyouhagoogledekensaku⏎");
    assert_eq!(r.text(), "きょうはgoogleでけんさく");
    r.type_(" arigatou.⏎");
    assert!(r.text().ends_with('。'), "{}", r.text());
}

#[test]
fn clause_context_survives_resizing_and_ends_with_the_conversion() {
    let mut r = JaRig::new("ja-clause-context");
    r.type_("kyouhaiitenkidesune ");
    r.session.choose(1, &mut r.field);
    assert_eq!(
        *r.converter.continuations.lock().unwrap(),
        [("いいてんきですね".to_string(), "今日は".to_string())]
    );
    r.type_("←→");
    let calls = r.converter.continuations.lock().unwrap().clone();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().all(|(_, previous)| previous == "今日は"));
    r.type_("⎋ ");
    let calls = r.converter.continuations.lock().unwrap().clone();
    assert!(calls.len() > 3);
    assert!(calls.iter().all(|(_, previous)| previous == "今日は"));
    let count = calls.len();
    r.type_("⏎henkan ⏎");
    assert_eq!(r.converter.continuations.lock().unwrap().len(), count);
    assert!(r.text().ends_with("変換"), "{}", r.text());
}

fn terminal_policies() -> AppPolicies {
    AppPolicies::new([(
        "test.terminal".to_string(),
        Override {
            conversion_threshold: Some(tacit_core::AppPolicy::STRICT_THRESHOLD),
            escape_to_app: Some(true),
            ..Default::default()
        },
    )])
}
