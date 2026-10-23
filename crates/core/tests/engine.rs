//! Port of DecisionEngineTests + MixedWordTests + the learning tests of the Swift suite.
//! Keys → expected output strings are the original's, unchanged.

mod common;

use std::path::PathBuf;

use tacit_core::EnglishModel;
use tacit_core::{
    make_engine, DecisionContext, DecisionEngine, HeuristicEngine, Language, Languages,
    LearningEngine, SplittingEngine, TextConverter,
};

fn temp_lexicon(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tacit-{name}-{}.json", std::process::id()))
}

fn heuristic() -> HeuristicEngine {
    HeuristicEngine::new()
}

fn splitting() -> SplittingEngine {
    SplittingEngine::new(Box::new(heuristic()), EnglishModel::shared())
}

fn ctx() -> DecisionContext {
    DecisionContext::default()
}

#[test]
fn english_words_stay_english() {
    let e = heuristic();
    for word in [
        "hello", "world", "the", "keyboard", "language", "switch", "input", "method", "files",
        "running", "commit", "github", "please", "thanks", "Apple", "API", "is", "a", "computer",
        "people", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
    ] {
        let d = e.decide(word, &ctx());
        assert_eq!(
            d.language,
            Language::English,
            "{word} → {} {}",
            d.output,
            d.explanation
        );
    }
}

#[test]
fn korean_keystrokes_become_hangul() {
    let e = heuristic();
    for (keys, expected) in [
        ("dkssudgktpdy", "안녕하세요"),
        ("gksrmf", "한글"),
        ("rkatkgkqslek", "감사합니다"),
        ("dhsmf", "오늘"),
        ("skfTl", "날씨"),
        ("whgdk", "좋아"),
        ("dlTek", "있다"),
        ("tkfkd", "사랑"),
        ("zjavbxj", "컴퓨터"),
        ("vmfhrmfoa", "프로그램"),
        ("wjdakf", "정말"),
        ("rhakdnj", "고마워"),
        ("wkf", "잘"),
        ("ajrdj", "먹어"),
        ("zzz", "ㅋㅋㅋ"),
        ("dlrjs", "이건"),
        ("djEjg", "어떻"),
        ("tlrks", "시간"),
        ("rmfjsep", "그런데"),
    ] {
        let d = e.decide(keys, &ctx());
        assert_eq!(
            d.language,
            Language::Korean,
            "{keys} → {} {}",
            d.output,
            d.explanation
        );
        assert_eq!(d.output, expected, "{keys}");
    }
}

#[test]
fn shift_overlap_uses_korean_scores_without_losing_english_case() {
    let e = heuristic();
    for partial in [true, false] {
        let context = DecisionContext { partial, ..ctx() };
        for keys in ["akfgkfRk", "akfgkfRK", "gkfRK", "dlfRk"] {
            let d = e.decide(keys, &context);
            assert_eq!(d.language, Language::Korean, "{keys}: {d:?}");
            assert_eq!(
                d.output,
                tacit_core::lang::hangul::HangulComposer::compose(keys)
                    .unwrap()
                    .text()
            );
            assert_eq!(e.render(keys, Language::English), keys);
        }
        for keys in [
            "Hello", "API", "WORK", "TYPE", "Park", "fooBar", "getURL", "aRK", "RK", "TY",
        ] {
            let d = e.decide(keys, &context);
            assert_eq!(d.language, Language::English, "{keys}: {d:?}");
            assert_eq!(d.output, keys);
        }
    }
}

#[test]
fn shift_overlap_preserves_matching_latin_identifiers() {
    let mut failures = Vec::new();
    for japanese in [false, true] {
        let languages = Languages::legacy(japanese);
        let e = HeuristicEngine::for_languages(&languages);
        for partial in [true, false] {
            let context = DecisionContext { partial, ..ctx() };
            // These intended identifiers enter the same Shift-overlap gate as Korean input.
            for keys in [
                "cmdWL", "rmWL", "dmlWL", "turnsRI", "cmdWLr", "rmWLr", "dmlWLr", "cmdWLfoo",
            ] {
                let d = e.decide(keys, &context);
                if d.language != Language::English || d.output != keys {
                    failures.push(format!(
                        "languages={:?}, partial={partial}, {keys}: {d:?}",
                        languages.ids()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn shift_overlap_retains_korean_positive_cases() {
    for japanese in [false, true] {
        let e = HeuristicEngine::for_languages(&Languages::legacy(japanese));
        for partial in [true, false] {
            let context = DecisionContext { partial, ..ctx() };
            for (keys, text) in [("akfgkfRK", "말할까"), ("smfWlr", "늘찍")] {
                let d = e.decide(keys, &context);
                assert_eq!(d.language, Language::Korean, "{keys}: {d:?}");
                assert_eq!(d.output, text, "{keys}: {d:?}");
                assert_eq!(e.render(keys, Language::English), keys);
            }
        }
    }
}

#[test]
fn shift_overlap_without_lexical_evidence_preserves_case() {
    for japanese in [false, true] {
        let e = HeuristicEngine::for_languages(&Languages::legacy(japanese));
        for partial in [true, false] {
            // An unlisted Korean spelling is ambiguous too; it needs canonical keys or explicit correction.
            for keys in ["smfWLr", "dlfRK", "yukiRI"] {
                let d = e.decide(keys, &DecisionContext { partial, ..ctx() });
                assert_eq!(d.language, Language::English, "{keys}: {d:?}");
                assert_eq!(d.output, keys);
            }
        }
    }
}

#[cfg(feature = "japanese")]
#[test]
fn korean_shift_evidence_does_not_admit_japanese() {
    let mut e = HeuristicEngine::for_languages(&Languages::legacy(true));
    // Even an overwhelming Japanese score cannot use a Korean-specific exception.
    e.japanese_offset = 100.0;
    for partial in [true, false] {
        let d = e.decide("akfgkfRK", &DecisionContext { partial, ..ctx() });
        assert_eq!(d.language, Language::Korean, "{d:?}");
        assert_eq!(d.output, "말할까");
    }
    let e = HeuristicEngine::for_languages(&Languages::from_ids(&["en", "ja"]).unwrap());
    for keys in ["cmdWL", "rmWL", "dmlWL", "turnsRI", "yukiRI"] {
        let d = e.decide(keys, &ctx());
        assert_eq!(d.language, Language::English, "{keys}: {d:?}");
        assert_eq!(d.output, keys);
    }
}

#[test]
fn mixed_sentence() {
    let e = heuristic();
    let c = TextConverter::new(&e);
    assert_eq!(
        c.convert("dkssudgktpdy, hello world!").0,
        "안녕하세요, hello world!"
    );
    assert_eq!(
        c.convert("dlrjs github dp commit gkf rjdi").0,
        "이건 github 에 commit 할 거야"
    );
    assert_eq!(
        c.convert("tmvmfls API qkfvy wnsql").0,
        "스프린 API 발표 준비"
    );
}

#[test]
fn context_resolves_short_collisions() {
    let e = heuristic();
    let c = TextConverter::new(&e);
    assert_eq!(c.convert("let's go").0, "let's go");
    assert_eq!(c.convert("dlrj go").0, "이거 해");
}

#[test]
fn a_coin_flip_word_follows_the_sure_word_after_it() {
    let e = heuristic();
    // Alone, `so` is English; with Korean after it on the line, it is 내.
    assert_eq!(
        TextConverter::new(&e).convert("so dlrltlarhk").0,
        "내 이기심과"
    );
    assert_eq!(
        TextConverter::new(&e)
            .without_look_ahead()
            .convert("so dlrltlarhk")
            .0,
        "so 이기심과"
    );
    // A line break ends the look: the next line says nothing about this one.
    assert_eq!(
        TextConverter::new(&e)
            .convert(
                "so
dlrltlarhk"
            )
            .0,
        "so
이기심과"
    );
}

#[test]
fn partial_preview_does_not_punish_pending_consonant() {
    let d = heuristic().decide(
        "dkss",
        &DecisionContext {
            previous_language: None,
            partial: true,
            ..Default::default()
        },
    );
    assert_eq!(d.language, Language::Korean, "{}", d.explanation);
}

fn typed(keys: &str, previous: Option<Language>, partial: bool) -> tacit_core::Decision {
    heuristic().decide(
        keys,
        &DecisionContext {
            previous_language: previous,
            partial,
            ..Default::default()
        },
    )
}

#[test]
fn informal_one_syllable_words_win_over_an_english_word_with_the_same_keys() {
    // 걍 and 캬 are said in every other movie review and in no Tatoeba sentence; rid is English too.
    for (keys, word) in [("rid", "걍"), ("zi", "캬")] {
        let d = typed(keys, Some(Language::Korean), false);
        assert_eq!(d.output, word, "{keys}: {}", d.explanation);
    }
}

#[test]
fn a_common_english_word_is_not_given_up_for_a_rare_korean_one() {
    // 애 is said a few times in a review set; do is the English word everyone types.
    let d = typed("do", None, false);
    assert_eq!(d.language, Language::English, "{}", d.explanation);
}

#[test]
fn the_start_of_an_english_word_after_english_is_no_korean_word_while_it_is_typed() {
    // wo is 재 and wor is 잭, both words: the preview must not flash them in "hello world".
    for keys in ["wo", "wor"] {
        let d = typed(keys, Some(Language::English), true);
        assert_eq!(d.language, Language::English, "{keys}: {}", d.explanation);
    }
}

#[test]
fn a_one_syllable_word_typed_after_korean_shows_as_korean_at_once() {
    for (keys, word) in [("go", "해"), ("so", "내"), ("to", "새")] {
        let d = typed(keys, Some(Language::Korean), true);
        assert_eq!(d.output, word, "{keys}: {}", d.explanation);
    }
}

#[test]
fn a_long_unlisted_korean_name_after_english_is_korean() {
    // The context lifts a word by the same amount in total, long or short: 나카바루초 and 호즈미촌
    // are no word of any list, and the English before them does not make them English.
    for (keys, word) in [("skzkqkfnch", "나카바루초"), ("ghwmalchs", "호즈미촌")] {
        let d = typed(keys, Some(Language::English), false);
        assert_eq!(d.output, word, "{keys}: {}", d.explanation);
    }
}

#[test]
fn a_short_english_token_after_english_stays_english() {
    // chmod +x run.sh: sh is 노 on a Korean layout.
    let d = typed("sh", Some(Language::English), false);
    assert_eq!(d.language, Language::English, "{}", d.explanation);
}

#[test]
fn a_long_english_word_after_korean_stays_english() {
    for word in ["aortosclerosis", "fleur", "rossi"] {
        let d = typed(word, Some(Language::Korean), false);
        assert_eq!(d.language, Language::English, "{word}: {}", d.explanation);
    }
}

#[test]
fn learning_overrides_base() {
    let path = temp_lexicon("learn");
    let _ = std::fs::remove_file(&path);
    let learning = LearningEngine::new(Box::new(heuristic()), Some(path.clone()));
    assert_eq!(learning.decide("hello", &ctx()).language, Language::English);
    learning.learn("hello", Language::Korean, None);
    assert_eq!(learning.decide("hello", &ctx()).language, Language::English); // once: not yet
    learning.learn("hello", Language::Korean, None);
    assert_eq!(learning.decide("hello", &ctx()).output, "ㅗ디ㅣㅐ");
    // Persisted and reloaded.
    let reloaded = LearningEngine::new(Box::new(heuristic()), Some(path.clone()));
    assert_eq!(reloaded.decide("hello", &ctx()).language, Language::Korean);
    // Learning the base choice removes the override.
    reloaded.learn("hello", Language::English, None);
    assert_eq!(reloaded.decide("hello", &ctx()).language, Language::English);
    let _ = std::fs::remove_file(&path);
}

/// One correction teaches the kind of word, not only the word: after `ss` is corrected to
/// ㄴㄴ, the other consonant-only keys the engine was unsure of (`ec`) follow, while the ones it
/// was sure about (`tv`, `weekend`) stay as they were. Enough corrections move a surer one.
#[test]
fn a_correction_teaches_the_kind_of_word() {
    let path = temp_lexicon("shape");
    let _ = std::fs::remove_file(&path);
    let learning = LearningEngine::new(Box::new(splitting()), Some(path.clone()));
    let language = |keys: &str| learning.decide(keys, &ctx()).language;
    assert_eq!(language("ec"), Language::English);
    learning.learn("ss", Language::Korean, Some(Language::English));
    assert_eq!(learning.decide("ec", &ctx()).output, "ㄷㅊ");
    assert_eq!(language("ac"), Language::English, "sure enough to stay");
    assert_eq!(language("tv"), Language::Korean);
    assert_eq!(language("weekend"), Language::English);
    // The same keys again are one word, not more evidence.
    learning.learn("ss", Language::Korean, Some(Language::English));
    assert_eq!(language("ac"), Language::English);
    // Two more different keys: ac is no longer sure enough.
    learning.learn("ec", Language::Korean, Some(Language::English));
    learning.learn("fq", Language::Korean, Some(Language::English));
    assert_eq!(learning.decide("ac", &ctx()).output, "ㅁㅊ");
    // It survives a restart.
    let reloaded = LearningEngine::new(Box::new(splitting()), Some(path.clone()));
    assert_eq!(reloaded.decide("ac", &ctx()).output, "ㅁㅊ");
    let _ = std::fs::remove_file(&path);
}

#[cfg(feature = "embedded-ja")]
#[test]
fn shape_learning_requires_matching_runner_up() {
    let path = temp_lexicon("shape-runner-up");
    let languages = Languages::from_ids(&["ko", "en", "ja"]).unwrap();
    for count in [0, 3, 4] {
        std::fs::write(
            &path,
            serde_json::json!({
                "learned": {},
                "once": {},
                "shapes": {"medium:korean": count},
            })
            .to_string(),
        )
        .unwrap();
        let engine = make_engine(Some(path.clone()), &languages);
        for partial in [true, false] {
            let context = DecisionContext { partial, ..ctx() };
            let base = engine.base().decide("korean", &context);
            assert_eq!(base.language, Language::English);
            assert_eq!(base.runner_up, Some(Language::Japanese));
            assert_eq!(base.output, "korean");
            let learned = engine.decide("korean", &context);
            assert_eq!(
                (
                    learned.language,
                    learned.runner_up,
                    learned.output,
                    learned.confidence
                ),
                (base.language, base.runner_up, base.output, base.confidence),
                "count={count}, partial={partial}",
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}

/// A correction made twice teaches the stem too: every other particle on it follows.
#[test]
fn learning_a_word_with_an_ending_teaches_its_stem() {
    let path = temp_lexicon("stem");
    let _ = std::fs::remove_file(&path);
    let learning = LearningEngine::new(Box::new(splitting()), Some(path.clone()));
    // zorbitctl is no word the lists know, and the search already cuts a name before a case
    // particle; the correction is kept as the stem, so every other ending follows it too.
    for _ in 0..2 {
        learning.learn("zorbitctldmf", Language::Mixed, Some(Language::English));
    }
    assert_eq!(
        learning.learned().get("zorbitctl"),
        Some(&Language::English)
    );
    let d = learning.decide("zorbitctldp", &ctx());
    assert_eq!(
        (d.output.as_str(), d.language),
        ("zorbitctl에", Language::Mixed)
    );
    // Korean: 리센느 taught through 리센느가 also reads 리센느는.
    for _ in 0..2 {
        learning.learn("fltpssmrk", Language::Korean, Some(Language::English));
    }
    assert_eq!(learning.decide("fltpssmsms", &ctx()).output, "리센느는");
    // Short stems stay word by word.
    for _ in 0..2 {
        learning.learn("godp", Language::Mixed, Some(Language::Korean));
    }
    assert!(!learning.learned().contains_key("go"));
    let _ = std::fs::remove_file(&path);
}

/// A lexicon.json it can't read (hand-edited, another version) is set aside, not overwritten.
#[test]
fn unreadable_lexicon_is_kept_aside() {
    let path = temp_lexicon("aside");
    let aside = PathBuf::from(format!("{}.unreadable", path.display()));
    std::fs::write(&path, r#"{"de": "japanese"}"#).unwrap();
    let learning = LearningEngine::new(Box::new(heuristic()), Some(path.clone()));
    assert!(learning.learned().is_empty() && learning.pending().is_empty());
    assert!(aside.exists());
    // A different correction than before starts over.
    learning.learn("rjtm", Language::English, Some(Language::Korean));
    learning.learn("rjtm", Language::Mixed, Some(Language::Korean));
    assert!(learning.learned().is_empty());
    assert_eq!(learning.pending().get("rjtm"), Some(&Language::Mixed));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&aside);
}

#[test]
fn splits_english_plus_particle() {
    let e = splitting();
    for (keys, expected) in [
        ("githubdp", "github에"),
        ("APIfmf", "API를"),
        ("commitgotj", "commit해서"),
        ("serverdptj", "server에서"),
        ("pythondmfh", "python으로"),
        ("deploygoTdj", "deploy했어"),
        ("TextEditdptj", "TextEdit에서"),
        ("Slackdptj", "Slack에서"),
        ("VSCodedptj", "VSCode에서"),
        ("GitHubdp", "GitHub에"),
        ("iPhonedptj", "iPhone에서"),
    ] {
        let d = e.decide(keys, &ctx());
        assert_eq!(d.language, Language::Mixed, "{keys}: {}", d.explanation);
        assert_eq!(d.output, expected, "{keys}");
    }
}

#[test]
fn splits_english_plus_korean_word() {
    let e = splitting();
    for (keys, expected) in [
        ("hellodnjfem", "hello월드"),
        ("githubtjqj", "github서버"),
        ("loginghkaus", "login화면"),
        ("designvhsxm", "design폰트"),
        ("Reactzjavhsjsxm", "React컴포넌트"),
        ("VSCodetjfwjd", "VSCode설정"),
    ] {
        let d = e.decide(keys, &ctx());
        assert_eq!(d.language, Language::Mixed, "{keys}: {}", d.explanation);
        assert_eq!(d.output, expected, "{keys}");
    }
}

/// Korean typed with shift isn't a capitalized name; names that spell like English
/// (cartman ≠ car스무) stay whole. `hasansowo`/`nonakawo` (clean romaji) are Japanese-only: dropped.
#[test]
fn leaves_whole_words_alone() {
    let e = splitting();
    for keys in [
        "work",
        "park",
        "hello",
        "thanks",
        "dkssudgktpdy",
        "akadp",
        "ruxdp",
        "wkfrk",
        "TkdRjvnfdms",
        "dlTdj",
        "Rmxdptj",
        "ThkTdjdy",
        "RoRmtgkgfusk",
        "cartman",
        "Cartman",
        "stuntman",
        "comstock",
        "wolfsbane",
    ] {
        let d = e.decide(keys, &ctx());
        assert_ne!(d.language, Language::Mixed, "{keys}: {}", d.explanation);
    }
}

/// A single letter and a whole Korean word (n제, x축): the letter alone would be a stray jamo.
#[test]
fn splits_single_letter_and_korean_word() {
    let e = splitting();
    for (keys, want) in [
        ("nwp", "n제"),
        ("xcnr", "x축"),
        ("Nwp", "N제"),
        ("Brmq", "B급"),
        ("Axla", "A팀"),
    ] {
        let d = e.decide(keys, &ctx());
        assert_eq!(d.output, want, "{keys}: {}", d.explanation);
    }
}

/// Capitals that are all shifted jamo keys (PR = ㅖㄲ) still make an acronym before a particle.
#[test]
fn splits_shifted_acronym_and_particle() {
    let e = splitting();
    for (keys, want) in [("PRdms", "PR은"), ("QRzhem", "QR코드")] {
        let d = e.decide(keys, &ctx());
        assert_eq!(d.output, want, "{keys}: {}", d.explanation);
    }
}

/// Words that spell like English stay whole, also while typing (edi → editor).
#[test]
fn single_letter_split_leaves_english_spelling_alone() {
    let e = splitting();
    for keys in ["bro", "ceo", "mak", "yah", "edi", "edit"] {
        let d = e.decide(keys, &ctx());
        assert_ne!(d.language, Language::Mixed, "{keys}: {}", d.explanation);
    }
}

#[test]
fn esc_cycles_through_readings() {
    assert_eq!(
        Language::English.next("githubdp", &Languages::legacy(false)),
        Language::Mixed
    );
    assert_eq!(
        Language::Mixed.next("githubdp", &Languages::legacy(false)),
        Language::English
    ); // 햐소ㅕㅠ에 is garbled: skipped
    assert_eq!(
        Language::Korean.next("githubdp", &Languages::legacy(false)),
        Language::English
    );
    assert_eq!(
        Language::English.next("dkssud", &Languages::legacy(false)),
        Language::Korean
    );
    // No split available: english ↔ korean.
    assert_eq!(
        Language::English.next("hello", &Languages::legacy(false)),
        Language::Korean
    );
    // With Japanese on it joins the cycle where the keys read cleanly.
    #[cfg(feature = "japanese")]
    assert_eq!(
        Language::Korean.next("sushi", &Languages::legacy(true)),
        Language::Japanese
    );
    assert_eq!(
        Language::Korean.next("sushi", &Languages::legacy(false)),
        Language::English
    );
    assert_eq!(
        Language::Japanese.next("sushi", &Languages::legacy(true)),
        Language::English
    );
    // No Korean reading for "ra-men".
    #[cfg(feature = "japanese")]
    assert_eq!(
        Language::English.next("ra-men", &Languages::legacy(true)),
        Language::Japanese
    );
}

#[test]
fn mixed_sentence_with_splitting() {
    let e = splitting();
    let c = TextConverter::new(&e);
    assert_eq!(
        c.convert("dlrjs githubdp commitgkf rjdi").0,
        "이건 github에 commit할 거야"
    );
}

#[test]
fn full_chain_matches_the_parts() {
    let engine = make_engine(None, &Languages::legacy(false));
    let c = TextConverter::new(engine.as_ref());
    assert_eq!(
        c.convert("dlrjs githubdp commitgkf rjdi").0,
        "이건 github에 commit할 거야"
    );
}

/// Whole stories typed with no space at all (lab/cases/codeswitch.tsv, tag `story`): the cuts
/// between the languages come from the scores alone. A few wrong letters in a thousand are fine.
#[test]
fn stories_typed_without_a_space_come_out_right() {
    let engine = make_engine(None, &Languages::legacy(false));
    let converter = TextConverter::new(engine.as_ref());
    let cases = include_str!("../../../lab/cases/codeswitch.tsv");
    let mut stories = 0;
    for line in cases.lines().filter(|l| l.trim_end().ends_with("\tstory")) {
        let text = line.trim_end().trim_end_matches("\tstory");
        let got = converter.convert(&common::keys_of_text(text)).0;
        let wrong = common::edit_distance(text, &got);
        let allowed = text.chars().count() / 100;
        assert!(
            wrong <= allowed,
            "{wrong} wrong of {}: {got}",
            text.chars().count()
        );
        stories += 1;
    }
    assert!(stories >= 3, "{stories} stories");
}

/// Consonant-only chat words (ㄹㅇ, ㅇㅈ, ㅈㅅ…) read as Korean; English words made of the same
/// keys (as, we, tea, set, cat) don't.
#[test]
fn consonant_only_chat_words_are_korean() {
    let e = heuristic();
    for (keys, expected) in [
        ("fd", "ㄹㅇ"),
        ("dw", "ㅇㅈ"),
        ("wt", "ㅈㅅ"),
        ("rc", "ㄱㅊ"),
        ("cz", "ㅊㅋ"),
    ] {
        let d = e.decide(keys, &ctx());
        assert_eq!(
            (d.language, d.output.as_str()),
            (Language::Korean, expected),
            "{keys}: {}",
            d.explanation
        );
    }
    for word in ["as", "at", "we", "re", "tea", "set", "cat", "eat", "fast"] {
        assert_eq!(e.decide(word, &ctx()).language, Language::English, "{word}");
    }
}

/// Right after a digit a counter is Korean (sus is also a word) and a unit English (kg is no
/// syllable), each with or without an ending.
#[test]
fn after_a_digit_counters_are_korean_and_units_english() {
    let e = splitting();
    let after_digit = DecisionContext {
        after_digit: true,
        ..ctx()
    };
    for (keys, expected) in [
        ("sus", "년"),
        ("dnjfdp", "월에"),
        ("qns", "분"),
        ("kg", "kg"),
        ("kgdlek", "kg이다"),
        ("gb", "gb"),
        ("mbdp", "mb에"),
        // Units whose letters also spell a syllable (cm 츠, gb 휴): still the unit.
        ("cmsms", "cm는"),
        ("cmdlek", "cm이다"),
        ("gbrk", "gb가"),
        ("cmsmswjdakfWkfqdms", "cm는정말짧은"),
    ] {
        let d = e.decide(keys, &after_digit);
        assert_eq!(d.output, expected, "{keys}: {}", d.explanation);
    }
    // Without the digit nothing changes: sus stays the English word.
    assert_eq!(e.decide("sus", &ctx()).output, "sus");
}

/// A name typed as an identifier (a capital no shifted jamo explains) takes its particle.
#[test]
fn camel_case_identifier_takes_a_particle() {
    let e = splitting();
    for (keys, expected) in [
        ("isValidrk", "isValid가"),
        ("getUserNamedmf", "getUserName을"),
    ] {
        assert_eq!(e.decide(keys, &ctx()).output, expected, "{keys}");
    }
}

/// Tool names the SCOWL lists miss, with a particle.
#[test]
fn developer_tool_names_take_particles() {
    let e = splitting();
    for (keys, expected) in [
        ("mongodbfmf", "mongodb를"),
        ("kubectldmf", "kubectl을"),
        ("graphqldmf", "graphql을"),
        ("timeoutdl", "timeout이"),
        ("postgresfh", "postgres로"),
        ("sqldmf", "sql을"),
    ] {
        assert_eq!(e.decide(keys, &ctx()).output, expected, "{keys}");
    }
    // A Korean word that the keys also spell stays Korean.
    assert_eq!(e.decide("dns", &ctx()).output, "운");
}
