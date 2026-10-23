//! Port of InputSessionTests + MixedWordSessionTests + PauseTests + the English/Korean parts of
//! EndToEndTests, SessionPolicyTests and CorrectionTests, driven through a fake text field.

use std::path::PathBuf;
use std::sync::Arc;

use tacit_core::session::testing::FakeTextField;
use tacit_core::session::{InputMode, Range, NOT_FOUND};
use tacit_core::store::policy::Override;
use tacit_core::History;
use tacit_core::{
    AppPolicies, AppPolicy, EnglishModel, HeuristicEngine, InputSession, Key, LearningEngine,
    SplittingEngine, TextClient,
};

fn engine(name: &str) -> Arc<LearningEngine> {
    let path = std::env::temp_dir().join(format!("tacit-sess-{name}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let base = SplittingEngine::new(Box::new(HeuristicEngine::new()), EnglishModel::shared());
    Arc::new(LearningEngine::new(Box::new(base), Some(path)))
}

fn lexicon_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tacit-sess-{name}-{}.json", std::process::id()))
}

struct Rig {
    session: InputSession,
    field: FakeTextField,
}

impl Rig {
    fn new(name: &str) -> Self {
        Self {
            session: InputSession::new(engine(name), AppPolicies::default(), true),
            field: FakeTextField::new(),
        }
    }

    fn with_policies(name: &str, policies: AppPolicies, app: &str) -> Self {
        let mut rig = Self {
            session: InputSession::new(engine(name), policies, true),
            field: FakeTextField::new(),
        };
        rig.session.activate(Some(app));
        rig
    }

    fn key(&mut self, key: Key) -> bool {
        self.session.handle(key, &mut self.field)
    }

    /// Letters and printable keys as themselves; keys the session passes on are typed by the
    /// field, as in a real text field. "⏎" Return, "⎋" Esc, "⇧" Shift+Esc, "⌫" Delete.
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

    /// Delete as the app sees it: the session takes it while a word is being typed, otherwise
    /// the app erases the character before the caret.
    fn erase(&mut self, times: usize) {
        for _ in 0..times {
            if !self.key(Key::Delete) && self.field.cursor > 0 {
                let at = self.field.cursor;
                self.field.insert_text(
                    "",
                    Some(Range {
                        location: at - 1,
                        length: 1,
                    }),
                );
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

#[test]
fn types_mixed_sentence() {
    let mut r = Rig::new("mixed");
    r.type_("dkssudgktpdy hello ");
    assert_eq!(r.text(), "안녕하세요 hello ");
}

#[test]
fn esc_after_commit_flips_the_word_and_learns_it() {
    let mut r = Rig::new("flip");
    r.type_("hello ");
    assert_eq!(r.text(), "hello ");
    assert!(r.key(Key::Escape));
    assert_eq!(r.text(), "ㅗ디ㅣㅐ ");
    assert_eq!(
        r.field.selected_range().location,
        "ㅗ디ㅣㅐ ".encode_utf16().count()
    );
    // Esc again flips back.
    assert!(r.key(Key::Escape));
    assert_eq!(r.text(), "hello ");
    // Flipping back to the engine's own choice clears the learned override.
    r.type_("hello ");
    assert_eq!(r.text(), "hello hello ");
}

/// One Esc changes only that word (a habitual Esc shouldn't stick); the same correction a second
/// time is remembered.
#[test]
fn flipped_choice_is_remembered_the_second_time() {
    let mut r = Rig::new("remember");
    r.type_("rjtm ");
    let first = r.text();
    r.key(Key::Escape);
    let flipped = r.text();
    assert_ne!(flipped, first);
    r.type_("rjtm ");
    assert_eq!(r.text(), format!("{flipped}{first}"));
    r.key(Key::Escape);
    assert_eq!(r.text(), format!("{flipped}{flipped}"));
    r.type_("rjtm ");
    assert_eq!(r.text(), format!("{flipped}{flipped}{flipped}"));
}

/// Only where the word ends up is a correction: an Esc loop that comes back to the engine's own
/// choice, however many times it is done, teaches nothing.
#[test]
fn esc_loops_back_to_the_start_teach_nothing() {
    let mut r = Rig::new("loop");
    for _ in 0..3 {
        r.type_("rjtm ");
        let shown = r.text();
        r.key(Key::Escape);
        r.key(Key::Escape); // two readings: back where it started
        assert_eq!(r.text(), shown);
        r.type_("x "); // closes the undo window
    }
    assert!(r.session.engine().learned().is_empty());
    assert!(r.session.engine().pending().is_empty());
}

#[test]
fn flip_only_applies_to_the_word_before_the_caret() {
    let mut r = Rig::new("caret");
    r.type_("dkssud ");
    r.field.cursor = 0; // user clicked elsewhere
    assert!(!r.key(Key::Escape));
    assert_eq!(r.text(), "안녕 ");
}

#[test]
fn esc_passes_through_when_text_cannot_be_verified() {
    let mut r = Rig::new("unreadable");
    r.field.readable = false;
    r.type_("dkssud ");
    assert!(!r.key(Key::Escape));
    assert_eq!(r.text(), "안녕 ");
}

#[test]
fn any_other_key_ends_the_flip_window() {
    let mut r = Rig::new("window");
    r.type_("dkssud ");
    assert!(!r.key(Key::Other));
    assert!(!r.key(Key::Escape));
    assert_eq!(r.text(), "안녕 ");
}

#[test]
fn esc_while_composing_flips_the_preview() {
    let mut r = Rig::new("compose");
    r.type_("dkssud");
    assert_eq!(r.text(), "안녕");
    r.key(Key::Escape);
    assert_eq!(r.text(), "dkssud");
    r.type_(" ");
    assert_eq!(r.text(), "dkssud ");
}

/// Readings the engine finds but the plain particle split does not (a single letter + particle,
/// a name + a Korean word) must still be reachable with Esc, not only shown once.
#[test]
fn esc_reaches_an_engine_found_mixed_reading() {
    let mut r = Rig::new("mixedcycle");
    r.type_("xdp");
    assert_eq!(r.text(), "x에");
    let mut seen = vec![r.text()];
    for _ in 0..3 {
        r.key(Key::Escape);
        seen.push(r.text());
    }
    // Mixed -> (next reading) -> ... -> Mixed again: it is part of the cycle.
    assert_eq!(seen.iter().filter(|t| *t == "x에").count(), 2, "{seen:?}");
    assert!(seen.contains(&"xdp".to_string()), "{seen:?}");
}

#[test]
fn delete_removes_keystrokes() {
    let mut r = Rig::new("del");
    r.type_("dkssud");
    r.key(Key::Delete);
    assert_eq!(r.text(), "안녀");
}

/// As the system's 2-set input: the syllable being typed goes jamo by jamo, earlier ones whole.
#[test]
fn delete_korean_like_the_system() {
    let mut r = Rig::new("delko");
    r.type_("gksrnr");
    assert_eq!(r.text(), "한국");
    for expected in ["한구", "한ㄱ", "한", ""] {
        r.key(Key::Delete);
        assert_eq!(r.text(), expected);
    }
    r.type_("dkssudgktpdy");
    for expected in ["안녕하세ㅇ", "안녕하세", "안녕하", "안녕"] {
        r.key(Key::Delete);
        assert_eq!(r.text(), expected);
    }
    r.type_("gk");
    assert_eq!(r.text(), "안녕하");
    r.key(Key::Delete);
    assert_eq!(r.text(), "안녕ㅎ");
}

/// After a Delete the syllable being typed stays apart from the ones before: 세요 → 세ㅇ → 세유.
#[test]
fn retyping_after_delete_keeps_syllables_apart() {
    let mut r = Rig::new("retype");
    r.type_("dkssudgktpdy");
    r.key(Key::Delete);
    r.type_("b ");
    assert_eq!(r.text(), "안녕하세유 ");
}

#[test]
fn delete_keeps_the_language_shown() {
    let mut r = Rig::new("keeplang");
    r.type_("gksrnr");
    r.key(Key::Delete);
    r.key(Key::Delete);
    assert_eq!(r.text(), "한ㄱ");
    r.type_(" ");
    assert_eq!(r.text(), "한ㄱ ");
}

#[test]
fn delete_english_key_by_key() {
    let mut r = Rig::new("delen");
    r.type_("hello");
    r.key(Key::Delete);
    assert_eq!(r.text(), "hell");
}

#[test]
fn types_english_then_korean_word() {
    let mut r = Rig::new("enko");
    r.type_("hellodnjfem githubtjqj ");
    assert_eq!(r.text(), "hello월드 github서버 ");
}

/// Return on an underlined word only confirms it; with nothing underlined it is the app's.
#[test]
fn return_confirms_then_goes_to_the_app() {
    let mut r = Rig::new("ret");
    r.type_("dkssud");
    assert!(r.key(Key::Enter));
    assert_eq!(r.text(), "안녕");
    assert!(!r.composing());
    assert!(!r.key(Key::Enter));
}

#[test]
fn confirmation_keeps_the_visible_korean_reading() {
    for boundary in [Key::Printable(' '), Key::Enter, Key::Tab, Key::Other] {
        let mut r = Rig::new("confirm-visible");
        r.field.insert_text("앞😀  뒤", None);
        r.field.cursor = "앞😀 ".encode_utf16().count();
        r.type_("dustmqgot");
        assert_eq!(r.text(), "앞😀 연습햇 뒤");
        r.key(boundary);
        assert_eq!(r.text(), "앞😀 연습햇 뒤", "{boundary:?}");
        assert!(!r.composing());
        assert_eq!(
            r.field.selected_range().location,
            "앞😀 연습햇".encode_utf16().count()
        );
    }
}

#[test]
fn shift_overlap_identifiers_keep_case_and_allow_explicit_korean() {
    for keys in ["cmdWL", "rmWL", "dmlWL", "turnsRI", "smfWLr", "dlfRK"] {
        for correct in [false, true] {
            let mut r = Rig::new("shift-overlap-identifiers");
            r.field.insert_text("앞😀  뒤", None);
            r.field.cursor = "앞😀 ".encode_utf16().count();
            r.type_(keys);
            assert_eq!(r.text(), format!("앞😀 {keys} 뒤"));
            let expected = if correct {
                r.key(Key::Escape);
                tacit_core::lang::hangul::HangulComposer::compose(keys)
                    .unwrap()
                    .text()
            } else {
                keys.to_string()
            };
            assert_eq!(r.text(), format!("앞😀 {expected} 뒤"));
            assert!(r.key(Key::Enter));
            assert_eq!(r.text(), format!("앞😀 {expected} 뒤"));
            assert!(!r.composing());
            assert_eq!(
                r.field.selected_range().location,
                format!("앞😀 {expected}").encode_utf16().count()
            );
        }
    }
}

#[test]
fn confirmation_keeps_incomplete_previews() {
    let mut changes = Vec::new();
    for script in [
        "s", "d", "z", "r", "k", "t", "n", "g", "dkssud d", "Tom's", "I'm", "don't", "2024sus",
        "1s", "hello t",
    ] {
        for boundary in [
            Key::Enter,
            Key::Printable(' '),
            Key::Printable('.'),
            Key::Tab,
            Key::Other,
        ] {
            let mut r = Rig::new("confirm-incomplete");
            r.type_(script);
            let shown = r.text();
            r.key(boundary);
            if r.text() != shown {
                changes.push(format!(
                    "{script:?} {boundary:?}: {shown:?} -> {:?}",
                    r.text()
                ));
            }
            assert!(!r.composing());
        }
        for new_context in [false, true] {
            let mut r = Rig::new("confirm-flush");
            r.type_(script);
            let shown = r.text();
            if new_context {
                r.session.commit_for_new_context(&mut r.field);
            } else {
                r.session.commit(&mut r.field);
            }
            if r.text() != shown {
                changes.push(format!(
                    "{script:?} flush({new_context}): {shown:?} -> {:?}",
                    r.text()
                ));
            }
            assert!(!r.composing());
        }
    }
    assert!(changes.is_empty(), "{}", changes.join("\n"));
}

/// Final confidence is not permission to replace an unfinished visible reading.
#[test]
fn confirmation_does_not_reclassify_incomplete_jamo() {
    use tacit_core::{Decision, DecisionContext, DecisionEngine, Language};
    struct Unfinished;
    impl DecisionEngine for Unfinished {
        fn decide(&self, keys: &str, context: &DecisionContext) -> Decision {
            Decision {
                language: if context.partial {
                    Language::Korean
                } else {
                    Language::English
                },
                confidence: 1.0,
                output: keys.to_string(),
                explanation: String::new(),
                runner_up: None,
            }
        }
    }
    let rig = || {
        let engine = Arc::new(LearningEngine::new(Box::new(Unfinished), None));
        (
            Rig {
                session: InputSession::new(engine.clone(), AppPolicies::default(), true),
                field: FakeTextField::new(),
            },
            engine,
        )
    };
    for boundary in [
        Key::Enter,
        Key::Printable(' '),
        Key::Printable('.'),
        Key::Tab,
        Key::Other,
    ] {
        let (mut r, engine) = rig();
        r.type_("rr");
        assert_eq!(r.text(), "ㄱㄱ");
        r.key(boundary);
        assert_eq!(r.text(), "ㄱㄱ", "{boundary:?}");
        assert!(!r.composing());
        assert!(engine.pending().is_empty());
        assert!(engine.learned().is_empty());
    }
    // An explicit override still uses the final engine decision for learning.
    let (mut r, engine) = rig();
    r.type_("rr⎋");
    assert_eq!(r.text(), "rr");
    r.key(Key::Enter);
    assert_eq!(r.text(), "rr");
    assert!(engine.pending().is_empty());
    assert!(engine.learned().is_empty());
}

#[test]
fn typed_digits_remain_new_evidence_not_plain_confirmation() {
    let mut r = Rig::new("confirm-digit-evidence");
    r.type_("p95 k8s v0.9.7 ");
    assert_eq!(r.text(), "p95 k8s v0.9.7 ");
    // A standalone first-key guess must not pin the growing Korean counter to English.
    let mut counter = Rig::new("confirm-digit-counter");
    counter.type_("10sus");
    assert_eq!(counter.text(), "10년");
    assert!(counter.key(Key::Enter));
    assert_eq!(counter.text(), "10년");
    // A preceding digit already affected the preview; Enter adds no new evidence.
    r.type_("2024sus");
    let shown = r.text();
    assert!(r.key(Key::Enter));
    assert_eq!(r.text(), shown);
}

#[test]
fn standalone_letter_scoring_does_not_change_growing_words() {
    for (keys, expected) in [("thwo", "소재"), ("dkssud", "안녕")] {
        let mut r = Rig::new("confirm-growing-word");
        r.type_(keys);
        assert_eq!(r.text(), expected, "{keys}");
        r.key(Key::Enter);
        assert_eq!(r.text(), expected, "{keys}");
    }
    let mut deleted = Rig::new("confirm-seed-delete");
    deleted.type_("t⌫thwo");
    assert_eq!(deleted.text(), "소재");
    let mut pinned = Rig::new("confirm-seed-pin");
    pinned.type_("th⌫");
    assert_eq!(pinned.text(), "ㅅ");
    pinned.key(Key::Enter);
    assert_eq!(pinned.text(), "ㅅ");
    let mut forced = Rig::new("confirm-seed-force");
    forced.type_("t⎋hwo⏎");
    assert_eq!(forced.text(), "소재");
    let mut reset = Rig::new("confirm-seed-reset");
    reset.type_("t");
    reset.key(Key::Other);
    reset.type_("thwo");
    assert_eq!(reset.text(), "t소재");
}

#[test]
fn english_single_letter_suffixes_finish_as_english() {
    let mut r = Rig::new("english-suffixes");
    r.type_("Tom's I'm don't ");
    assert_eq!(r.text(), "Tom's I'm don't ");
}

#[test]
fn a_completed_english_korean_ending_is_visible_before_confirmation() {
    let mut r = Rig::new("mixed-preview");
    r.type_("isValidrk gkdtkd truedi");
    assert_eq!(r.text(), "isValid가 항상 true야");
    r.type_(" ");
    assert_eq!(r.text(), "isValid가 항상 true야 ");
}

#[test]
fn shifted_vowel_after_a_doubled_consonant_stays_korean() {
    for boundary in [Key::Printable(' '), Key::Enter] {
        let mut r = Rig::new("shift-overlap");
        r.type_("akfgkfRK");
        assert_eq!(r.text(), "말할까");
        assert_eq!(r.session.keys(), "akfgkfRK");
        r.key(Key::Escape);
        assert_eq!(r.text(), "akfgkfRK");
        r.key(Key::Escape);
        assert_eq!(r.text(), "말할까");
        r.key(boundary);
        assert_eq!(r.text(), "말할까");
        assert!(!r.composing());
    }
}

/// With the setting off, Return confirms the word and the app gets it too.
#[test]
fn return_goes_on_when_it_does_not_only_confirm() {
    let mut session = InputSession::new(engine("ret2"), AppPolicies::default(), false);
    let mut field = FakeTextField::new();
    for c in "dkssud".chars() {
        session.handle(Key::Letter(c), &mut field);
    }
    assert!(!session.handle(Key::Enter, &mut field));
    assert_eq!(field.text(), "안녕");
    assert!(field.marked_range().location == NOT_FOUND);
}

#[test]
fn types_single_letter_with_particle() {
    let mut r = Rig::new("single");
    r.type_("xdp ndl Xfmf Xeh ");
    assert_eq!(r.text(), "x에 n이 X를 X도 ");
}

/// A lone consonant key after Korean text is the jamo (ㅇ, ㅋ), not an English letter; Esc still
/// turns it into the letter, and after English it stays the letter.
#[test]
fn a_lone_consonant_after_korean_is_a_jamo() {
    let mut r = Rig::new("lone-jamo");
    r.type_("dkssud d z ");
    assert_eq!(r.text(), "안녕 ㅇ ㅋ ");
    let mut r = Rig::new("lone-jamo-esc");
    r.type_("dkssud d⎋ ");
    assert_eq!(r.text(), "안녕 d ");
    let mut r = Rig::new("lone-jamo-english");
    r.type_("hello d ");
    assert_eq!(r.text(), "hello d ");
}

#[test]
fn keeps_short_english_ending_in_eh_or_sk() {
    let mut r = Rig::new("ehsk");
    r.type_("heh meh tsk ");
    assert_eq!(r.text(), "heh meh tsk ");
}

/// Korean then English with no space: 오늘은 then google에서.
#[test]
fn types_korean_then_english_word() {
    let mut r = Rig::new("koen");
    r.type_("dhsmfdmsgoogledptj dlrjtdmsgithubdp ");
    assert_eq!(r.text(), "오늘은google에서 이것은github에 ");
}

#[test]
fn plain_korean_stays_whole() {
    let mut r = Rig::new("whole");
    r.type_("dkssudgktpdy qusruddlfur rhkswjadmfh ");
    assert_eq!(r.text(), "안녕하세요 변경이력 관점으로 ");
    // A consonant just typed for the next syllable isn't broken Hangul (하느님ㅇ, then lad…).
    let mut r = Rig::new("whole2");
    r.type_("gksmsladml tlsqnsladms godirpTspdy ");
    assert_eq!(r.text(), "하느님의 신부님은 해야겠네요 ");
}

// ---- pause ----

#[test]
fn paused_keys_go_through_as_typed() {
    let mut r = Rig::with_policies("pause1", AppPolicies::default(), "test.app");
    r.session.toggle_pause(&mut r.field);
    r.type_("jjkkhl dkssud");
    assert_eq!(r.text(), "jjkkhl dkssud");
    assert!(!r.composing());
}

#[test]
fn pausing_commits_the_word_in_progress() {
    let mut r = Rig::with_policies("pause2", AppPolicies::default(), "test.app");
    r.type_("dkssud");
    r.session.toggle_pause(&mut r.field);
    assert_eq!(r.text(), "안녕");
    assert!(!r.composing());
}

#[test]
fn second_tap_resumes() {
    let mut r = Rig::with_policies("pause3", AppPolicies::default(), "test.app");
    r.session.toggle_pause(&mut r.field);
    r.type_("jk ");
    r.session.toggle_pause(&mut r.field);
    r.type_("dkssud ");
    assert_eq!(r.text(), "jk 안녕 ");
}

#[test]
fn another_text_field_resumes() {
    let mut r = Rig::with_policies("pause4", AppPolicies::default(), "test.app");
    r.session.toggle_pause(&mut r.field);
    r.session.activate(Some("test.other"));
    assert!(!r.session.paused());
}

// ---- policy ----

fn strict_policies() -> AppPolicies {
    AppPolicies::new([
        (
            "test.strict".to_string(),
            Override {
                conversion_threshold: Some(0.9999),
                ..Default::default()
            },
        ),
        (
            "test.off".to_string(),
            Override {
                enabled: Some(false),
                ..Default::default()
            },
        ),
        (
            "test.terminal".to_string(),
            Override {
                conversion_threshold: Some(AppPolicy::STRICT_THRESHOLD),
                escape_to_app: Some(true),
                ..Default::default()
            },
        ),
    ])
}

#[test]
fn strict_app_keeps_uncertain_words_english() {
    let mut r = Rig::with_policies("strict", strict_policies(), "test.strict");
    r.type_("dhsmf ");
    assert_eq!(r.text(), "dhsmf ");
}

#[test]
fn disabled_app_passes_keys_through() {
    let mut r = Rig::with_policies("off", strict_policies(), "test.off");
    assert!(!r.key(Key::Letter('d')));
    assert!(r.text().is_empty());
}

#[test]
fn strict_still_splits_mixed_words() {
    let mut r = Rig::with_policies("strictsplit", strict_policies(), "test.terminal");
    r.type_("ciw gst githubdp xdp hellodnjfem ");
    assert_eq!(r.text(), "ciw gst github에 x에 hello월드 ");
}

#[test]
fn esc_cycles_mixed_word_after_commit() {
    let mut r = Rig::new("cyclemixed");
    r.type_("githubdp ");
    assert_eq!(r.text(), "github에 ");
    // The garbled Korean reading (햐소ㅕㅠ에) is skipped: github에 ↔ githubdp.
    r.key(Key::Escape);
    assert_eq!(r.text(), "githubdp ");
    r.key(Key::Escape);
    assert_eq!(r.text(), "github에 ");
}

#[test]
fn correction_counts_once_with_the_final_choice() {
    let mut r = Rig::new("corr1");
    r.session.activate(Some("com.example.chat"));
    r.type_("hello ");
    r.key(Key::Escape); // hello → ㅗ디ㅣㅐ
    assert!(r.session.engine().pending().is_empty()); // window still open
    r.type_("x"); // any other key closes it
    assert_eq!(
        r.session.engine().pending().get("hello"),
        Some(&tacit_core::Language::Korean)
    );
}

#[test]
fn nothing_counts_when_the_word_is_flipped_back() {
    let mut r = Rig::new("corr2");
    r.type_("hello ");
    r.key(Key::Escape);
    r.key(Key::Escape);
    r.type_("x");
    assert!(r.session.engine().pending().is_empty());
}

/// The engine is unsure about "so" (English, 0.55): an Esc to 내 must still count.
#[test]
fn esc_while_typing_is_learned_the_second_time() {
    let mut r = Rig::new("corr3");
    r.session.activate(Some("com.apple.TextEdit"));
    r.type_("so⎋ ");
    assert_eq!(r.text(), "내 ");
    r.type_("the so⎋ ");
    assert_eq!(
        r.session.engine().learned().get("so"),
        Some(&tacit_core::Language::Korean)
    );
    r.type_("the so ");
    assert!(r.text().ends_with("내 "), "{}", r.text());
}

// ---- terminal (Esc belongs to the app) ----

#[test]
fn space_commits_what_the_preview_shows() {
    let mut r = Rig::with_policies("e2e1", strict_policies(), "test.terminal");
    r.type_("esc djEjgrp");
    assert_eq!(r.text(), "esc 어떻게"); // still composing (marked)
    r.type_(" todrkrgo?");
    assert_eq!(r.text(), "esc 어떻게 생각해?");
}

#[test]
fn terminal_escape_goes_to_the_app() {
    let mut r = Rig::with_policies("e2e2", strict_policies(), "test.terminal");
    r.type_("dkssud");
    assert!(!r.key(Key::Escape)); // passed to the app
    assert_eq!(r.text(), "안녕");
    r.type_(" hello ");
    assert!(!r.key(Key::Escape)); // no undo after commit either
    assert_eq!(r.text(), "안녕 hello ");
}

#[test]
fn terminal_shift_escape_switches() {
    let mut r = Rig::with_policies("e2e3", strict_policies(), "test.terminal");
    r.type_("hello⇧");
    assert_eq!(r.text(), "ㅗ디ㅣㅐ");
    r.type_("⇧ "); // back to English, then commit
    assert_eq!(r.text(), "hello ");
    r.type_("⇧");
    assert_eq!(r.text(), "ㅗ디ㅣㅐ ");
}

#[test]
fn escape_switches_in_other_apps() {
    let mut r = Rig::with_policies("e2e4", strict_policies(), "com.apple.TextEdit");
    r.type_("hello⎋");
    assert_eq!(r.text(), "ㅗ디ㅣㅐ");
}

#[test]
fn acronyms_and_longer_endings() {
    for (script, expected) in [
        ("UIrkxdmsrjs ", "UI같은건 "),
        ("UIdp ", "UI에 "),
        ("AIrk ", "AI가 "),
        ("githubrkxdmsep ", "github같은데 "),
    ] {
        let mut r = Rig::with_policies("e2e5", strict_policies(), "test.terminal");
        r.type_(script);
        assert_eq!(r.text(), expected);
    }
}

#[test]
fn korean_with_english_and_particles() {
    let mut r = Rig::new("e2e6");
    r.type_("dlrjs githubdp commitgkf rjdi.");
    assert_eq!(r.text(), "이건 github에 commit할 거야.");
}

#[test]
fn punctuation_after_words() {
    for (script, expected) in [
        ("dho?", "왜?"),
        ("rjdi.", "거야."),
        ("hello.", "hello."),
        ("hello, world!", "hello, world!"),
    ] {
        let mut r = Rig::new("punct");
        r.type_(script);
        assert_eq!(r.text(), expected, "{script}");
    }
}

#[test]
fn lexicon_file_is_created_under_the_given_path() {
    let path = lexicon_path("persist");
    let _ = std::fs::remove_file(&path);
    let engine = Arc::new(LearningEngine::new(
        Box::new(SplittingEngine::new(
            Box::new(HeuristicEngine::new()),
            EnglishModel::shared(),
        )),
        Some(path.clone()),
    ));
    let mut session = InputSession::new(engine, AppPolicies::default(), true);
    let mut field = FakeTextField::new();
    for c in "hello".chars() {
        session.handle(Key::Letter(c), &mut field);
    }
    session.handle(Key::Printable(' '), &mut field);
    field.insert_text(" ", None);
    session.handle(Key::Escape, &mut field);
    session.handle(Key::Other, &mut field); // closes the window → one correction recorded
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

fn rig_with_history(name: &str, words: &[&str]) -> Rig {
    let history = Arc::new(History::new(None));
    for w in words {
        history.record(w);
    }
    let mut rig = Rig::new(name);
    rig.session =
        InputSession::new(engine(name), AppPolicies::default(), true).with_history(history);
    rig
}

/// Tab replaces the underlined word with the word typed most often that continues it.
#[test]
fn tab_completes_english_from_the_users_own_words() {
    let mut r = rig_with_history("tab-en", &["zebrafish", "zebrafish"]);
    r.type_("zebraf");
    assert!(r.key(Key::Tab));
    assert_eq!(r.text(), "zebrafish");
    assert!(!r.composing());
    // The word that was completed counts for next time, and the sentence goes on.
    r.type_(" ");
    assert_eq!(r.text(), "zebrafish ");
}

#[test]
fn tab_completes_korean_from_the_users_own_words() {
    let mut r = rig_with_history("tab-ko", &["안녕하세요", "안녕하세요"]);
    r.type_("dkssud");
    assert!(r.composing());
    assert!(r.key(Key::Tab));
    assert_eq!(r.text(), "안녕하세요");
    assert!(!r.composing());
}

/// Nothing to complete: the word goes in as shown and the app gets the Tab.
#[test]
fn tab_without_a_completion_commits_and_passes_through() {
    let mut r = rig_with_history("tab-none", &[]);
    r.type_("the");
    assert!(!r.key(Key::Tab));
    assert_eq!(r.text(), "the");
    assert!(!r.composing());
}

#[test]
fn tab_without_history_never_completes() {
    let mut r = Rig::new("tab-off");
    r.type_("zebraf");
    assert!(!r.key(Key::Tab));
    assert_eq!(r.text(), "zebraf");
}

/// A file name stays whole: `rs` after `main.` is the extension, not the jamo ㄱㄴ.
#[test]
fn file_names_with_an_extension_stay_english() {
    let mut r = Rig::new("file-ext");
    r.type_("main.rs tnwjdgoTdj ");
    assert_eq!(r.text(), "main.rs 수정했어 ");
}

#[test]
fn mode_cycles_auto_korean_english() {
    let mut r = Rig::new("mode-cycle");
    assert_eq!(r.session.mode(), InputMode::Auto);
    r.session.cycle_mode(&mut r.field);
    assert_eq!(r.session.mode(), InputMode::Korean);
    r.session.cycle_mode(&mut r.field);
    assert_eq!(r.session.mode(), InputMode::English);
    r.session.cycle_mode(&mut r.field);
    assert_eq!(r.session.mode(), InputMode::Auto);
}

/// 한: every word is Korean, also one the engine would keep English.
#[test]
fn korean_mode_types_hangul_for_every_word() {
    let mut r = Rig::new("mode-korean");
    r.session.cycle_mode(&mut r.field);
    r.type_("hello dkssud ");
    assert_eq!(r.text(), "ㅗ디ㅣㅐ 안녕 ");
}

/// 영: the session steps aside and the keys reach the app as typed.
#[test]
fn english_mode_leaves_keys_to_the_app() {
    let mut r = Rig::new("mode-english");
    r.session.cycle_mode(&mut r.field);
    r.session.cycle_mode(&mut r.field);
    r.type_("dkssud ");
    assert_eq!(r.text(), "dkssud ");
}

#[test]
fn switching_mode_commits_the_word_in_progress() {
    let mut r = Rig::new("mode-commit");
    r.type_("dkssud");
    r.session.cycle_mode(&mut r.field);
    assert_eq!(r.text(), "안녕");
    assert!(!r.composing());
}

#[test]
fn readings_list_each_distinct_reading_and_mark_the_shown_one() {
    let mut r = Rig::new("readings");
    assert!(r.session.readings().is_empty());
    r.type_("hello");
    let readings = r.session.readings();
    let texts: Vec<&str> = readings.iter().map(|x| x.text.as_str()).collect();
    assert_eq!(texts, ["hello", "ㅗ디ㅣㅐ"]);
    assert_eq!(
        readings.iter().map(|x| x.shown).collect::<Vec<_>>(),
        [true, false]
    );
}

#[test]
fn choosing_a_reading_changes_the_word_like_esc() {
    let mut r = Rig::new("choose");
    r.type_("hello");
    assert!(r.session.choose_reading(1, &mut r.field));
    assert!(r.session.readings()[1].shown);
    r.type_(" ");
    assert_eq!(r.text(), "ㅗ디ㅣㅐ ");
}

#[test]
fn choosing_a_missing_reading_does_nothing() {
    let mut r = Rig::new("choose-none");
    assert!(!r.session.choose_reading(0, &mut r.field));
    r.type_("hello");
    assert!(!r.session.choose_reading(9, &mut r.field));
}

#[test]
fn mending_hangul_after_delete_stays_hangul() {
    let mut r = Rig::new("mend");
    r.type_("tkfkd");
    r.erase(3);
    r.type_("k");
    assert_eq!(r.text(), "사ㅏ");
}

#[test]
fn text_left_before_the_caret_is_the_context_of_the_next_word() {
    let mut r = Rig::new("caret-context");
    r.type_("dkssud hello ");
    r.erase(7);
    r.type_("en ");
    assert_eq!(r.text(), "안녕두 ");
}
