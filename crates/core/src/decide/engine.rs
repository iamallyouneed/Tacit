//! Language decision (DecisionEngine.swift, with the Japanese model this port adds).

use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[cfg(feature = "japanese")]
use crate::decide::segmenter::{JapaneseSegmenter, Segment};
use crate::lang::english::{EnglishModel, EXTRA_LEVEL};
use crate::lang::hangul::{is_vowel, DubeolsikLayout, HangulComposer, Unit};
#[cfg(feature = "japanese")]
use crate::lang::japanese::JapaneseModel;
use crate::lang::korean::{KoreanModel, KoreanScore};
use crate::lang::registry;
use crate::lang::set::Languages;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    English,
    Korean,
    /// English word + Korean particle/ending, e.g. `github에` (see `MixedWord`).
    Mixed,
    /// Romaji typed as Japanese (rendered as hiragana; kanji conversion happens on Space).
    Japanese,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Korean => "korean",
            Language::Mixed => "mixed",
            Language::Japanese => "japanese",
        }
    }

    pub fn supported(self) -> bool {
        registry::module(self).is_some()
    }

    /// The short id used in settings.json and pack names.
    pub fn id(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Korean => "ko",
            Language::Mixed => "mixed",
            Language::Japanese => "ja",
        }
    }

    /// A language the user can turn on (not `Mixed`, which comes from English + Korean).
    pub fn from_id(id: &str) -> Option<Language> {
        match id {
            "en" => Some(Language::English),
            "ko" => Some(Language::Korean),
            "ja" => Some(Language::Japanese),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Option<Language> {
        match s {
            "english" => Some(Language::English),
            "korean" => Some(Language::Korean),
            "mixed" => Some(Language::Mixed),
            "japanese" => Some(Language::Japanese),
            _ => None,
        }
    }

    /// The language a sentence continues in after this word.
    pub fn sentence_language(self) -> Language {
        match self {
            Language::English => Language::English,
            Language::Korean | Language::Mixed => Language::Korean,
            Language::Japanese => Language::Japanese,
        }
    }

    /// What Esc switches to: the enabled languages in the user's order, the English + Korean
    /// reading right after English. Readings that can't exist for these keys are skipped
    /// (no Korean for "ra-men"), and so are garbled ones (dkssud as Japanese: dkっすd) as long
    /// as a clean alternative exists — a typo can still be forced into its language when
    /// nothing reads cleanly.
    pub fn next(self, keys: &str, languages: &Languages) -> Language {
        self.next_with(keys, languages, false)
    }

    /// `next`, where `engine_mixed` says the engine reads these keys as English + Korean even
    /// though the plain particle split finds nothing (`xdp` → `x에`, `hellodnjfem` →
    /// `hello월드`): that reading is then part of the cycle too.
    pub fn next_with(self, keys: &str, languages: &Languages, engine_mixed: bool) -> Language {
        let available: Vec<Language> = registry::cycle_order(languages)
            .into_iter()
            .filter(|l| {
                registry::reading(*l, keys).is_some() || (*l == Language::Mixed && engine_mixed)
            })
            .collect();
        let clean = |l: &Language| registry::is_clean(*l, keys);
        let clean_count = available.iter().filter(|l| clean(l)).count();
        let cycle: Vec<Language> = if clean_count >= 2 {
            available
                .iter()
                .copied()
                .filter(|l| clean(l) || *l == self)
                .collect()
        } else {
            available
        };
        match cycle.iter().position(|l| *l == self) {
            Some(i) => cycle[(i + 1) % cycle.len()],
            None => cycle.first().copied().unwrap_or(languages.first()),
        }
    }
}

/// What the engine knows besides the token itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct DecisionContext {
    /// Language of the previously committed word, if it is recent enough to matter.
    pub previous_language: Option<Language>,
    /// The token is still being typed (used for the live preview).
    pub partial: bool,
    /// The token starts right after a digit (2024년, 1.5kg), where a counter or a unit is
    /// the likely word.
    pub after_digit: bool,
    /// A digit follows the token right away (p95, v0.9.7, k8s): the letters next to it are
    /// more likely a name than Korean.
    pub before_digit: bool,
}

#[derive(Clone, Debug)]
pub struct Decision {
    pub language: Language,
    /// 0.5 = coin flip, 1.0 = certain.
    pub confidence: f64,
    /// What should be committed for this decision.
    pub output: String,
    /// Human-readable reasoning, for the CLI and debugging.
    pub explanation: String,
    pub runner_up: Option<Language>,
}

pub trait DecisionEngine: Send + Sync {
    fn decide(&self, keys: &str, context: &DecisionContext) -> Decision;

    /// Renders `keys` in a given language regardless of what the engine would pick.
    fn render(&self, keys: &str, language: Language) -> String {
        if language == Language::Mixed {
            // The engine's own split (hello월드, github에); the particle split for engines without one.
            let decision = self.decide(keys, &DecisionContext::default());
            if decision.language == Language::Mixed {
                return decision.output;
            }
        }
        registry::reading(language, keys).unwrap_or_else(|| keys.to_string())
    }
}

/// Score floors for a dictionary English word by length: short ones collide with real Korean.
const ENGLISH_FLOOR_LONG: f64 = -1.4;
const ENGLISH_FLOOR_MEDIUM: f64 = -2.0;
const ENGLISH_FLOOR_SHORT: f64 = -2.6;
/// Taken off the English score when the token has non-letters.
const NON_LETTER_PENALTY: f64 = 3.0;
/// Confidence when capitalization identifies English rather than shifted Korean keys.
const CAPITALISED_ENGLISH_CONFIDENCE: f64 = 0.99;
/// A single consonant key typed inside Korean text (ㅇ for d): it is the jamo, not an English letter.
const LONE_CONSONANT_CONFIDENCE: f64 = 0.95;
/// A common two-letter English word needs to beat a one-syllable Korean word's floor by this.
const COMMON_SHORT_WORD_MARGIN: f64 = 0.1;
/// With no second reading to compare with, the best one is taken to lead by this much.
const NO_RUNNER_UP_MARGIN: f64 = 5.0;
/// Steepness of the margin → confidence curve (a logistic: margin 0 is a coin flip).
pub(crate) const CONFIDENCE_SLOPE: f64 = 2.0;

/// Scores the same keystrokes as English (character trigrams + word list), Korean
/// (2-set syllables) and, when enabled, Japanese (romaji → kana trigrams), each as
/// average log-probability per keystroke, nudges the previous word's language, and
/// picks the best. Confidence comes from the gap to the runner-up.
pub struct HeuristicEngine {
    pub english: &'static EnglishModel,
    pub korean: KoreanModel,
    /// None when Japanese is off or its model isn't available.
    #[cfg(feature = "japanese")]
    pub japanese: Option<&'static JapaneseModel>,
    /// Added to the previous word's language score: people mostly stay in one language.
    pub context_bias: f64,
    /// The length in keys at which `context_bias` is the average added per key. The score is an
    /// average per key, so a bias of that size adds more in total to a long word than to a
    /// short one; with this set, the total is the same for every length (a long word that no
    /// list knows is lifted less, a short one more). 0: per key, whatever the length.
    pub context_bias_keys: f64,
    /// Bias after Japanese text. Raising it to 1.2 recovered 1 of 2,590 held-out phrases
    /// but tripled common English words turning into kana right after Japanese (0.5% → 1.6%).
    #[cfg(feature = "japanese")]
    pub japanese_context_bias: f64,
    /// Shifts Japanese scores to the English/Korean scale (kana trigrams over ~80 symbols
    /// per ~2 keys read differently from letter trigrams). Tuned on held-out data so that
    /// English words almost never turn into kana.
    #[cfg(feature = "japanese")]
    pub japanese_offset: f64,
    /// Shifts Korean scores (used when the syllable model changes scale).
    pub korean_offset: f64,
    /// Taken off the dictionary floor for rarer English words (SCOWL levels 40–50: dork,
    /// ahem), so a common Korean word with the same keys (애가, 모든) wins the tie.
    pub rare_word_floor_penalty: f64,
    /// The previous word's pull is scaled by this when the keys are a known word of another
    /// language and not of the previous one: that's someone switching language.
    pub switch_bias_factor: f64,
    /// Score floors for a Korean word seen in real text (two or more syllables, one).
    pub korean_known_floor_long: f64,
    pub korean_known_floor_short: f64,
    /// How far Japanese must lead before a known English word is turned into kana
    /// outside Japanese text.
    #[cfg(feature = "japanese")]
    pub english_word_protection: f64,
    /// With Japanese on: English words inside a phrase (`googlede…`) score as segmented
    /// Japanese, not as kana with stray letters.
    #[cfg(feature = "japanese")]
    pub segmenter: Option<JapaneseSegmenter>,
    /// English competes (off: the keys stay as typed only when no language reads them).
    pub english_on: bool,
    /// Korean competes.
    pub korean_on: bool,
}

impl HeuristicEngine {
    pub fn new() -> Self {
        Self::with(EnglishModel::shared(), KoreanModel::default())
    }

    pub fn with(english: &'static EnglishModel, korean: KoreanModel) -> Self {
        Self {
            english,
            korean,
            #[cfg(feature = "japanese")]
            japanese: None,
            context_bias: 0.8,
            context_bias_keys: 3.0,
            #[cfg(feature = "japanese")]
            japanese_context_bias: 0.8,
            #[cfg(feature = "japanese")]
            japanese_offset: 0.5,
            korean_offset: 0.0,
            rare_word_floor_penalty: 0.3,
            switch_bias_factor: 0.25,
            korean_known_floor_long: -1.4,
            korean_known_floor_short: -2.0,
            #[cfg(feature = "japanese")]
            english_word_protection: 1.0,
            #[cfg(feature = "japanese")]
            segmenter: None,
            english_on: true,
            korean_on: true,
        }
    }

    /// The heuristic over the languages that are on (Japanese only in builds that carry it).
    pub fn for_languages(languages: &Languages) -> Self {
        let mut engine = Self::new();
        #[cfg(feature = "japanese")]
        if languages.contains(Language::Japanese) {
            engine = Self::with_japanese(EnglishModel::shared(), JapaneseModel::shared());
        }
        engine.english_on = languages.contains(Language::English);
        engine.korean_on = languages.contains(Language::Korean);
        engine
    }

    /// The heuristic with Japanese scoring and segmentation on (the app's `makeEngine`).
    #[cfg(feature = "japanese")]
    pub fn with_japanese(english: &'static EnglishModel, japanese: &'static JapaneseModel) -> Self {
        let mut engine = Self::with(english, KoreanModel::default());
        engine.japanese = Some(japanese);
        engine.segmenter = Some(JapaneseSegmenter::new(english, japanese));
        engine
    }
}

impl HeuristicEngine {
    /// Average log-probability of `keys` read as English: character trigrams, lifted to a floor
    /// for dictionary words.
    fn english_score(&self, keys: &str, known: bool) -> f64 {
        let count = keys.chars().count();
        let mut e = if keys.is_ascii() {
            self.english.average_log_probability(keys)
        } else {
            let letters: String = keys.chars().filter(|c| c.is_alphabetic()).collect();
            self.english.average_log_probability(&letters)
        };
        // Also while typing: the preview must show what Space/Return will commit.
        if known {
            // Short dictionary words collide with real Korean (go=해, so=내, do=애),
            // so they get a weaker boost and leave room for context to decide.
            let mut floor = match count {
                4.. => ENGLISH_FLOOR_LONG,
                3 => ENGLISH_FLOOR_MEDIUM,
                _ => ENGLISH_FLOOR_SHORT,
            };
            // to, so, go, do: everyday English just above a one-syllable Korean word (새, 내,
            // 해, 애), so they stay English unless the previous word was Korean.
            if count == 2 && self.english.is_common_word(keys) {
                floor = self.korean_known_floor_short + COMMON_SHORT_WORD_MARGIN;
            }
            if self.english.level(keys).unwrap_or(0) > EXTRA_LEVEL {
                floor -= self.rare_word_floor_penalty;
            }
            e = e.max(floor);
        }
        if keys.chars().any(|c| !c.is_alphabetic()) {
            e -= NON_LETTER_PENALTY; // "ra-men", "kan'i" aren't English
        }
        e
    }

    /// The Korean reading of `keys` with its score, and whether it is a word seen in real
    /// text (which lifts the score to a floor). `after_korean`: the word before was Korean.
    fn korean_score(
        &self,
        keys: &str,
        partial: bool,
        after_korean: bool,
    ) -> (Option<KoreanScore>, bool) {
        let mut k = self.korean.score(keys, partial);
        let syllables = k.as_ref().map_or(0, |s| {
            s.composition
                .units
                .iter()
                .filter(|unit| matches!(unit, Unit::Syllable(_)))
                .count()
        });
        let known = k.as_ref().is_some_and(|s| {
            if s.composition
                .units
                .iter()
                .any(|u| matches!(u, Unit::Jamo(_)))
            {
                return false;
            }
            let word = s.composition.text();
            self.korean.is_known_word_or_inflected(&word)
                // Wider vocabulary counts too, unless the keys are everyday English
                // (a rare word like ahem may well be 모드).
                || (syllables >= 2
                    && self.english.level(keys).is_none_or(|l| l > EXTRA_LEVEL)
                    && self.korean.is_vocabulary_word(&word))
        });
        // While it is still being typed, one syllable out of English or nowhere is the start of
        // anything (wo of world, as 재 and 잭 are words): it is no word yet, and lifts neither
        // its score nor the Korean side of the context. After Korean it is a word more often
        // than not (해, 내, 새), so it keeps both.
        let lone = k.is_some() && syllables < 2;
        let known = known && !(partial && lone && !after_korean);
        if let (Some(score), true) = (k.as_mut(), known) {
            let floor = if syllables >= 2 {
                self.korean_known_floor_long
            } else {
                self.korean_known_floor_short
            };
            score.average_log_probability = score.average_log_probability.max(floor);
        }
        (k, known)
    }
}

impl Default for HeuristicEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionEngine for HeuristicEngine {
    fn decide(&self, keys: &str, context: &DecisionContext) -> Decision {
        let mut previous = '\0';
        let mut has_lowercase = false;
        let mut shift_overlap = false;
        let english_capital = self.english_on
            // Keep ordinary lowercase typing on the existing fast path.
            && keys
                .chars()
                .any(|c| c.is_uppercase() && !DubeolsikLayout::is_korean_shift_key(c))
            && keys.chars().any(|c| {
                // Shift may overlap the vowel after a doubled consonant; verify its Korean evidence below.
                let overlap = self.korean_on
                    && has_lowercase
                    && matches!(previous, 'Q' | 'W' | 'E' | 'R' | 'T')
                    && DubeolsikLayout::jamo(c).is_some_and(is_vowel);
                shift_overlap |= c.is_uppercase()
                    && !DubeolsikLayout::is_korean_shift_key(c)
                    && overlap;
                previous = c;
                has_lowercase |= c.is_ascii_lowercase();
                c.is_uppercase() && !DubeolsikLayout::is_korean_shift_key(c) && !overlap
            });
        // Syllable likelihood alone also accepts identifiers; require existing Korean lexical evidence.
        let korean_overlap = !shift_overlap
            || HangulComposer::compose(keys).is_some_and(|composed| {
                if composed.is_broken() {
                    return false;
                }
                let text = composed.text();
                self.korean.is_known_word_or_inflected(&text)
                    || self.korean.is_vocabulary_word(&text)
            });
        if english_capital || !korean_overlap {
            return Decision {
                language: Language::English,
                confidence: CAPITALISED_ENGLISH_CONFIDENCE,
                output: keys.to_string(),
                explanation: if english_capital {
                    "uppercase outside QWERTOP"
                } else {
                    "Shift overlap without Korean lexical evidence"
                }
                .to_string(),
                runner_up: None,
            };
        }

        // A lone consonant key right after Korean text is a jamo (ㅇ, ㅋ, ㅎ as in ㅋㅋ): one key on
        // its own has nothing else to go on, and Korean is what is being typed.
        if self.korean_on
            && context.previous_language.map(Language::sentence_language) == Some(Language::Korean)
            && keys.chars().count() == 1
        {
            let jamo = HangulComposer::compose(keys).map(|c| c.text());
            if let Some(text) = jamo.filter(|t| t.chars().all(|c| ('ㄱ'..='ㅎ').contains(&c))) {
                return Decision {
                    language: Language::Korean,
                    confidence: LONE_CONSONANT_CONFIDENCE,
                    output: text,
                    explanation: "a lone consonant key inside Korean text".to_string(),
                    runner_up: Some(Language::English),
                };
            }
        }

        let known = self.english_on && self.english.is_known_word(keys);
        let e = self.english_score(keys, known);

        // Without Korean its tables are never touched (its pack may not be installed).
        let (k, known_korean) = if self.korean_on {
            self.korean_score(
                keys,
                context.partial,
                context.previous_language.map(Language::sentence_language)
                    == Some(Language::Korean),
            )
        } else {
            (None, false)
        };

        let count = keys.chars().count() as f64;
        // A dictionary English word may already be complete: score its Japanese reading as
        // finished too, so the preview shows what Space will commit ("to" stays to, not と).
        #[cfg(feature = "japanese")]
        let j = self
            .japanese
            // Korean Shift evidence only admits a Korean reading, not unrelated Japanese candidates.
            .filter(|_| !shift_overlap)
            .and_then(|model| model.score(keys, context.partial && !known));
        // English words inside a Japanese phrase (googlede…) score as segmented Japanese,
        // not as kana with stray letters.
        #[cfg(feature = "japanese")]
        let mut segmented: Option<(Vec<Segment>, f64)> = None;
        #[cfg(feature = "japanese")]
        if let Some(score) = &j {
            if score.composition.stray_letters > 0 {
                segmented = self
                    .segmenter
                    .as_ref()
                    .and_then(|s| s.scored_segments(keys));
            }
        }
        #[cfg(feature = "japanese")]
        let japanese_score = j.as_ref().map(|score| {
            score.average_log_probability.max(
                segmented
                    .as_ref()
                    .map_or(f64::NEG_INFINITY, |(_, s)| s / count),
            )
        });

        let mut score_storage = [(Language::English, e); 3];
        let mut score_len = usize::from(self.english_on);
        if let Some(k) = &k {
            score_storage[score_len] = (
                Language::Korean,
                k.average_log_probability + self.korean_offset,
            );
            score_len += 1;
        }
        #[cfg(feature = "japanese")]
        if let Some(japanese_score) = japanese_score {
            score_storage[score_len] = (Language::Japanese, japanese_score + self.japanese_offset);
            score_len += 1;
        }
        let scores = &mut score_storage[..score_len];
        if let Some(previous) = context.previous_language.map(Language::sentence_language) {
            if let Some(i) = scores.iter().position(|s| s.0 == previous) {
                let mut bias = match previous {
                    #[cfg(feature = "japanese")]
                    Language::Japanese => self.japanese_context_bias,
                    _ => {
                        let share = if self.context_bias_keys > 0.0 {
                            self.context_bias_keys / count.max(2.0)
                        } else {
                            1.0
                        };
                        self.context_bias * share
                    }
                };
                let known_in = |l: Language| match l {
                    Language::English => known,
                    Language::Korean => known_korean,
                    _ => false,
                };
                if !known_in(previous)
                    && [Language::English, Language::Korean]
                        .iter()
                        .any(|l| *l != previous && known_in(*l))
                {
                    bias *= self.switch_bias_factor;
                }
                scores[i].1 += bias;
            }
        }
        if scores.is_empty() {
            // No language that is on reads these keys (Korean only, and the keys aren't Hangul).
            let language = if self.korean_on {
                Language::Korean
            } else if cfg!(feature = "japanese") {
                Language::Japanese
            } else {
                Language::English
            };
            return Decision {
                language,
                confidence: 0.5,
                output: keys.to_string(),
                explanation: "no enabled language reads these keys".to_string(),
                runner_up: None,
            };
        }
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // A dictionary English word only becomes kana on a clear win, unless we're already
        // in Japanese text (where "same", "sake", "kite" are Japanese words).
        #[cfg(feature = "japanese")]
        if known
            && scores[0].0 == Language::Japanese
            && context.previous_language.map(Language::sentence_language)
                != Some(Language::Japanese)
        {
            if let Some(i) = scores.iter().position(|s| s.0 == Language::English) {
                if scores[0].1 - scores[i].1 < self.english_word_protection {
                    scores.swap(0, i);
                }
            }
        }

        let (language, best) = scores[0];
        let runner_up = scores.get(1).copied();
        let margin = best - runner_up.map_or(best - NO_RUNNER_UP_MARGIN, |r| r.1);
        let confidence = 1.0 / (1.0 + (-margin * CONFIDENCE_SLOPE).exp());
        let output = match language {
            Language::Korean => k
                .as_ref()
                .map(|k| k.composition.text())
                .unwrap_or_else(|| keys.to_string()),
            #[cfg(feature = "japanese")]
            Language::Japanese => segmented
                .map(|(segments, _)| JapaneseSegmenter::text(&segments))
                .or_else(|| j.as_ref().map(|score| score.composition.text.clone()))
                .unwrap_or_else(|| keys.to_string()),
            _ => keys.to_string(),
        };
        let mut explanation = String::new();
        for (i, (language, score)) in scores.iter().enumerate() {
            if i != 0 {
                explanation.push(' ');
            }
            write!(explanation, "{}={:.2}", &language.name()[..2], score).unwrap();
        }
        write!(
            explanation,
            "{} margin={:+.2}",
            if known { " (dict)" } else { "" },
            margin
        )
        .unwrap();
        Decision {
            language,
            confidence,
            output,
            explanation,
            runner_up: runner_up.map(|r| r.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_scores_keep_language_order_confidence_and_diagnostics() {
        let mut engine = HeuristicEngine::new();
        #[cfg(feature = "japanese")]
        {
            engine.japanese = None;
        }
        let keys = "dkssud";
        engine.korean_known_floor_long = 0.0;
        let english = engine.english_score(keys, false);
        let (korean, _) = engine.korean_score(keys, false, false);
        assert_eq!(korean.unwrap().average_log_probability, 0.0);
        engine.korean_offset = english;
        let decision = engine.decide(keys, &DecisionContext::default());
        assert_eq!(decision.language, Language::English);
        assert_eq!(decision.runner_up, Some(Language::Korean));
        assert_eq!(decision.confidence, 0.5);
        assert_eq!(decision.output, keys);
        assert_eq!(
            decision.explanation,
            format!("en={english:.2} ko={english:.2} margin=+0.00")
        );

        engine.korean_on = false;
        let decision = engine.decide(keys, &DecisionContext::default());
        assert_eq!(decision.runner_up, None);
        assert_eq!(
            decision.explanation,
            format!("en={english:.2} margin=+5.00")
        );
    }
}
