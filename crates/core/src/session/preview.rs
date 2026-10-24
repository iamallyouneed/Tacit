//! What the word on screen reads as: the engine's decision, the underlined preview, and the
//! hysteresis that keeps the preview from flickering between languages.

use super::{char_count, InputSession, TextClient};
use crate::decide::engine::{Decision, DecisionContext, DecisionEngine, Language};
use crate::decide::mixed::MixedWord;
use crate::lang::english::EnglishModel;
use crate::lang::hangul::HangulComposer;
use crate::lang::korean::{KoreanModel, UNKNOWN_SYLLABLE_LOGP};
use crate::lang::pack::PackLazy;
#[cfg(feature = "japanese")]
use crate::lang::romaji::RomajiComposer;
use crate::store::policy::AppPolicy;

/// The preview only switches language mid-word when the new reading is at least this likely.
const PREVIEW_FLIP_CONFIDENCE: f64 = 0.7;
/// Romaji is shown as kana while typing only when the engine is this sure, and the word is at
/// least this many keys long.
#[cfg(feature = "japanese")]
const LIVE_KANA_CONFIDENCE: f64 = 0.9;
#[cfg(feature = "japanese")]
const LIVE_KANA_MIN_KEYS: usize = 4;

pub(super) static KOREAN: PackLazy<KoreanModel> =
    PackLazy::new(Language::Korean, KoreanModel::default);

/// The last decision and what it was made from, for debugging.
#[derive(Clone, Debug)]
pub struct Trace {
    pub keys: String,
    pub previous: Option<Language>,
    pub decision: Decision,
    /// Partial-input scoring requested; standalone previews can request final scoring.
    pub partial: bool,
}

impl InputSession {
    pub(super) fn decide(&mut self, partial: bool) -> Decision {
        let ctx = DecisionContext {
            previous_language: self.previous_language,
            partial,
            after_digit: self.word_after_digit,
            before_digit: self.word_before_digit,
        };
        let mut d = self.engine.decide(self.word.keys(), &ctx);
        // A completed Korean ending on a known English stem must be visible before confirmation.
        if partial
            && d.language == Language::English
            && !EnglishModel::shared().is_known_word(self.word.keys())
            && MixedWord::split(self.word.keys())
                .is_some_and(|mixed| EnglishModel::shared().is_known_word(&mixed.english))
        {
            let finished = self.engine.decide(
                self.word.keys(),
                &DecisionContext {
                    partial: false,
                    ..ctx
                },
            );
            if finished.language == Language::Mixed {
                d = finished;
            }
        }
        let threshold = self.policy.conversion_threshold;
        let kept = d.language != Language::English && d.confidence < threshold;
        self.kept_english_by_strict =
            kept && threshold >= AppPolicy::STRICT_THRESHOLD && char_count(self.word.keys()) <= 3;
        let decision = if kept {
            Decision {
                language: Language::English,
                confidence: 1.0 - d.confidence,
                output: self.word.keys().to_owned(),
                explanation: format!(
                    "kept English ({:.2} < {:.2}); {}",
                    d.confidence, threshold, d.explanation
                ),
                runner_up: d.runner_up,
            }
        } else {
            d
        };
        self.last_trace = Some(Trace {
            keys: self.word.keys().to_owned(),
            previous: self.previous_language,
            decision: decision.clone(),
            partial,
        });
        decision
    }

    /// Confirmation accepts the engine reading already shown, even when unfinished.
    /// A newly typed digit is different: it supplies identifier context (p95, k8s),
    /// so that boundary retains the ordinary confidence rule. Overrides are applied
    /// by the caller. Unchanged explicit corrections use their first visible baseline;
    /// edited or restored forced tokens use the final decision for the current keys.
    pub(super) fn committed_language(&mut self) -> Language {
        let shown = if self.word.forced_by_user() {
            let Some(origin) = self.word.correction_origin() else {
                return self.decide(false).language;
            };
            Some(origin)
        } else {
            self.word.engine_preview()
        };
        self.ordinary_confirmation_language(shown)
    }

    fn ordinary_confirmation_language(&mut self, shown: Option<Language>) -> Language {
        let Some(shown) = shown else {
            return self.decide(false).language;
        };
        if !self.word_before_digit {
            return shown;
        }
        let d = self.decide(false);
        if shown != d.language && !self.is_weak_flip_allowed(&d, shown) {
            shown
        } else {
            d.language
        }
    }

    /// Whether `decision` may replace the reading on screen: when it's confident, when it's a
    /// dictionary English word (which is what Space/Return will commit), or when a Strict app
    /// kept a short token English.
    pub(super) fn is_weak_flip_allowed(&self, decision: &Decision, shown: Language) -> bool {
        decision.confidence >= PREVIEW_FLIP_CONFIDENCE
            || (decision.language == Language::English
                && (self.kept_english_by_strict
                    || EnglishModel::shared().is_known_word(self.word.keys())))
            || self.preview(shown).is_none()
    }

    pub(super) fn update_preview(&mut self, client: &mut dyn TextClient) {
        if self.word.is_empty() {
            self.word.forget_preview();
            client.set_marked_text("");
        } else if let Some(forced) = self.word.override_language() {
            self.word.show(forced);
            let text = match forced {
                #[cfg(feature = "japanese")]
                Language::Japanese => self.japanese_display(),
                Language::Korean => self
                    .word
                    .hangul()
                    .unwrap_or_else(|| self.word.keys().to_owned()),
                other => self.engine.render(self.word.keys(), other),
            };
            client.set_marked_text(&text);
        } else {
            self.split_off_korean_before_other(client);
            #[cfg(feature = "japanese")]
            self.split_off_japanese_before_korean(client);
            let continuation = self.word.take_continuation().filter(|_| {
                self.word.keys().len() == 2
                    && self.word.keys().bytes().all(|c| c.is_ascii_alphabetic())
            });
            let partial = self.decide(true);
            let decision = if char_count(self.word.keys()) == 1 {
                // A lone key is a complete letter/jamo on screen, but the next letter
                // must continue the original partial hypothesis, not that standalone guess.
                self.word.seed_continuation(partial.language);
                self.decide(false)
            } else {
                partial
            };
            let mut language = decision.language;
            // Hysteresis uses the first-key continuation hypothesis once, then the visible
            // reading. A dictionary English word may always replace a weak earlier guess.
            if let Some(shown) = continuation.or(self.word.preview_language()) {
                if shown != language
                    && char_count(self.word.keys()) > 1
                    && !self.is_weak_flip_allowed(&decision, shown)
                {
                    language = shown;
                }
            }
            self.word.show_engine(language);
            let text = match language {
                #[cfg(feature = "japanese")]
                Language::Japanese => self.japanese_display(),
                _ => self
                    .preview(language)
                    .unwrap_or_else(|| self.word.keys().to_owned()),
            };
            client.set_marked_text(&text);
        }
    }

    /// Korean with English typed straight after it reads as broken Hangul: 오늘은 then
    /// google에서 (dhsmfdmsgoogledptj). When the keys only make Hangul with stray jamo, but a
    /// front of two or more ordinary syllables is followed by a known English word (or one with
    /// a particle), the Korean goes in as typed and the rest carries on as its own word.
    pub(super) fn split_off_korean_before_other(&mut self, client: &mut dyn TextClient) {
        let chars: Vec<char> = self.word.keys().chars().collect();
        if chars.len() < 7 {
            return;
        }
        if HangulComposer::compose(self.word.keys()).is_none_or(|c| !c.is_broken()) {
            return;
        }
        // Only what reads as Korean on its own, judged without context.
        let alone = self.engine.decide(
            self.word.keys(),
            &DecisionContext {
                previous_language: None,
                partial: true,
                ..Default::default()
            },
        );
        if alone.language != Language::Korean {
            return;
        }
        for cut in 4..=(chars.len() - 3) {
            let front: String = chars[..cut].iter().collect();
            let back: String = chars[cut..].iter().collect();
            let Some(korean) = HangulComposer::compose(&front) else {
                continue;
            };
            if !korean.stray_jamo().is_empty()
                || korean.syllables().len() < 2
                || !korean
                    .syllables()
                    .iter()
                    .all(|s| KOREAN.get().syllable_log_probability(*s) > UNKNOWN_SYLLABLE_LOGP)
                || !self.is_other_language_word(&back)
            {
                continue;
            }
            client.insert_text(&korean.text(), None);
            *self.word.keys_mut() = back;
            self.word_after_digit = false;
            self.word.syllable_start = None;
            self.previous_language = Some(Language::Korean);
            self.word.forget_preview();
            return;
        }
    }

    /// A known English word, one with a Korean particle (google에서), or romaji the engine
    /// is sure is Japanese (no word starts with っ or ん: that cut takes a letter from the Korean).
    pub(super) fn is_other_language_word(&self, keys: &str) -> bool {
        let ctx = DecisionContext {
            previous_language: Some(Language::Korean),
            partial: true,
            ..Default::default()
        };
        let decision = self.engine.decide(keys, &ctx);
        match decision.language {
            Language::English => {
                char_count(keys) >= 3 && EnglishModel::shared().is_known_word(keys)
            }
            Language::Mixed => true,
            #[cfg(feature = "japanese")]
            Language::Japanese => {
                if !self.japanese_enabled()
                    || decision.confidence < LIVE_KANA_CONFIDENCE
                    || char_count(keys) < LIVE_KANA_MIN_KEYS
                {
                    return false;
                }
                let romaji = RomajiComposer::compose(keys, true);
                romaji.stray_letters == 0
                    && romaji
                        .text
                        .chars()
                        .next()
                        .is_some_and(|c| c != 'っ' && c != 'ん')
            }
            Language::Korean => false,
            #[cfg(not(feature = "japanese"))]
            Language::Japanese => false,
        }
    }

    /// The word so far rendered in `language`, or None when it has no such reading.
    pub(super) fn preview(&self, language: Language) -> Option<String> {
        match language {
            #[cfg(not(feature = "japanese"))]
            Language::Japanese => None,
            Language::English => Some(self.word.keys().to_owned()),
            Language::Korean => self.word.hangul(),
            Language::Mixed => {
                let text = self.engine.render(self.word.keys(), Language::Mixed);
                (text != self.word.keys()).then_some(text)
            }
            #[cfg(feature = "japanese")]
            Language::Japanese => self.japanese_enabled().then(|| self.japanese_reading(true)),
        }
    }
}
