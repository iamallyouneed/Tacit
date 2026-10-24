//! One state machine per text client: the word being composed, the language of the previous
//! word, and the last committed word (so Esc right after a commit can change it).
//! Port of InputSession.swift.
//!
//! While a word is being composed:
//!   letters        extend it; the underlined preview shows the likely language
//!   Esc            cycle it: english → english+particle (github에) → korean → japanese
//!                  (where the app's Esc is set to go to the app, Esc commits and goes to the
//!                  app; Shift+Esc cycles)
//!   Shift+Esc      cycle it, in every app
//!   Delete         in Korean the last jamo of the syllable being typed, then whole syllables
//!                  (as the system's 2-set does); in Japanese the last kana; else the last keystroke
//!   Space          English/Korean: commit, then the app's space. Japanese: convert to kanji
//!                  (with `full_width_space`, a space after Japanese text is full-width; ⇧Space
//!                  types the other width)
//!   0–9            with `full_width_digits`, after Japanese text or inside a phrase they are
//!                  part of it, full-width (１２３, ０３－１２３４); Space offers the half-width form
//!   , . ? ! - '    inside a Japanese phrase they are part of it (、。？！ー); otherwise
//!                  they commit the word, then type the key
//!   Tab            complete the word to the user's most typed (else a built-in) word that
//!                  continues it; with no completion it commits and the app gets the Tab
//!   Return         commit the word as shown and nothing else (with `return_confirms_only`);
//!                  a Japanese phrase is only confirmed either way
//!   ⌃K/⌃J, F7/F6   show the Japanese reading as katakana / hiragana
//!   anything else  commit and let the app handle the key
//! While converting to kanji (candidate window open):
//!   Space / ↓      next candidate          ↑      previous candidate
//!   1–9            commit that candidate   Return commit the selected candidate
//!   Esc / Delete   back to kana            ←/→    shorten or lengthen the first clause
//!   letters/punctuation  commit, then keep typing
//! Right after a word was committed with space/punctuation, Esc / Shift+Esc cycle that
//! committed word the same way. The same correction made twice is learned for good.

pub mod bridge;
mod candidates;
mod client;
mod flip;
#[cfg(feature = "japanese")]
mod japanese;
mod key;
mod preview;
pub mod testing;
mod word;

pub use candidates::{CandidatePage, Reading};
pub use client::{NoopClient, Range, TextClient, NOT_FOUND};
use flip::Committed;
#[cfg(feature = "japanese")]
pub use japanese::Conversion;
pub use key::Key;
pub use preview::Trace;
use word::WordState;

use std::sync::Arc;

#[cfg(feature = "japanese")]
use crate::decide::convert::{JapaneseConverter, KanaConverter};
use crate::decide::engine::{DecisionEngine, Language};
use crate::decide::learning::LearningEngine;
#[cfg(feature = "japanese")]
use crate::decide::segmenter::JapaneseSegmenter;
#[cfg(feature = "japanese")]
use crate::lang::english::EnglishModel;
#[cfg(feature = "japanese")]
use crate::lang::japanese::JapaneseModel;
#[cfg(feature = "japanese")]
use crate::lang::romaji::RomajiComposer;
use crate::lang::set::Languages;
use crate::store::history::{self, History};
use crate::store::policy::{AppPolicies, AppPolicy};

struct Word {
    keys: String,
    text: String,
    language: Language,
    decided: Language,
}

/// Japanese: the conversion in progress and what feeds it.
#[cfg(feature = "japanese")]
struct JapaneseState {
    /// With Japanese on: show (and commit) the top kanji candidate while typing, like
    /// macOS Live Conversion.
    live_conversion: bool,
    /// The kana→kanji engine; none with Japanese off (the macOS engines have no Windows
    /// counterpart yet, so this is `KanaConverter` until one exists).
    converter: Option<Arc<dyn JapaneseConverter>>,
    /// The segmenter for English words inside a phrase (kyouhagoogle…); built with Japanese.
    segmenter: Option<JapaneseSegmenter>,
    /// A kana→kanji conversion in progress.
    conversion: Option<Conversion>,
    /// Esc showed the current phrase's kana instead (the next Esc switches language as usual).
    live_suppressed: bool,
    live_cache: Option<(
        String,
        Option<u16>,
        Option<crate::decide::convert::ConversionCandidate>,
    )>,
    /// "henkan" came back from conversion as "henkan'" (the final n stays ん); dropped again
    /// if the word leaves Japanese, so English doesn't get an apostrophe nobody typed.
    added_n_marker: bool,
}

/// What full-width text needs to remember about the sentence so far.
struct WidthState {
    /// Full-width openers not closed yet in this sentence, so ) ] } close them full-width
    /// after an English word too.
    #[cfg(feature = "japanese")]
    open_brackets: Vec<char>,
    /// The last key passed to the app was a digit (so "." there is a decimal point).
    after_digit: bool,
    /// Digits after Japanese text, and inside a Japanese phrase, are part of it and full-width
    /// (`Settings.full_width_digits`), as addresses in Japan often need.
    #[cfg(feature = "japanese")]
    full_width_digits: bool,
    /// A space after Japanese text is full-width (`Settings.full_width_space`).
    #[cfg(feature = "japanese")]
    full_width_space: bool,
}

/// What a text field's words are read as: the engine decides (tacit), every word is Korean, or
/// tacit steps aside and the keys reach the app as typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Auto,
    Korean,
    English,
}

pub struct InputSession {
    engine: Arc<LearningEngine>,
    policies: AppPolicies,
    return_confirms_only: bool,
    word: WordState,
    #[cfg(feature = "japanese")]
    ja: JapaneseState,
    width: WidthState,
    app: Option<String>,
    policy: AppPolicy,
    /// The last `decide` kept a short token (3 keys or fewer, like a command) English only
    /// because the app is set to Strict.
    kept_english_by_strict: bool,
    previous_language: Option<Language>,
    /// The app deleted text before the caret since the last word: the next word's context is
    /// what is left there (`language_before_caret`).
    edited_back: bool,
    last_commit: Option<Committed>,
    paused: bool,
    /// Every word starts out read as Korean (`InputMode::Korean`).
    korean_only: bool,
    /// The languages that are on, in Esc order (the engine's).
    languages: Languages,
    /// The last correction handed to the learning engine: going back and forth is one correction.
    last_recorded: Option<(String, Language)>,
    /// The word being typed started right after a digit (2024년).
    word_after_digit: bool,
    /// The key that commits the word is a digit (p95): the word is judged with that known.
    word_before_digit: bool,
    last_trace: Option<Trace>,
    /// Counts committed words and completes with them; None: no autocomplete.
    history: Option<Arc<History>>,
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

impl InputSession {
    pub fn new(
        engine: Arc<LearningEngine>,
        policies: AppPolicies,
        return_confirms_only: bool,
    ) -> Self {
        let languages = engine.languages().clone();
        #[cfg(feature = "japanese")]
        let japanese = languages.contains(Language::Japanese);
        Self {
            engine,
            policies,
            return_confirms_only,
            word: WordState::default(),
            app: None,
            policy: AppPolicy::STANDARD,
            kept_english_by_strict: false,
            previous_language: None,
            edited_back: false,
            last_commit: None,
            paused: false,
            korean_only: false,
            languages,
            #[cfg(feature = "japanese")]
            ja: JapaneseState {
                live_conversion: false,
                converter: japanese.then(|| Arc::new(KanaConverter) as Arc<dyn JapaneseConverter>),
                segmenter: japanese.then(|| {
                    JapaneseSegmenter::new(EnglishModel::shared(), JapaneseModel::shared())
                }),
                conversion: None,
                live_suppressed: false,
                live_cache: None,
                added_n_marker: false,
            },
            width: WidthState {
                #[cfg(feature = "japanese")]
                open_brackets: Vec::new(),
                after_digit: false,
                #[cfg(feature = "japanese")]
                full_width_digits: false,
                #[cfg(feature = "japanese")]
                full_width_space: false,
            },
            last_recorded: None,
            word_after_digit: false,
            word_before_digit: false,
            last_trace: None,
            history: None,
        }
    }

    /// Turns on Tab completion, learning from the words committed.
    pub fn with_history(mut self, history: Arc<History>) -> Self {
        self.history = Some(history);
        self
    }

    /// The kana→kanji engine for Space and, with `live_conversion`, the top candidate while
    /// typing. Japanese itself is on when the engine's languages include it.
    #[cfg(feature = "japanese")]
    pub fn with_japanese(
        mut self,
        converter: Arc<dyn JapaneseConverter>,
        live_conversion: bool,
    ) -> Self {
        self.ja.converter = Some(converter);
        self.ja.live_conversion = live_conversion;
        self
    }

    /// Romaji is detected and converted (Japanese is among the languages).
    #[cfg(feature = "japanese")]
    fn japanese_enabled(&self) -> bool {
        self.languages.contains(Language::Japanese)
    }

    /// Digits after Japanese text, and inside a Japanese phrase, are part of it and full-width.
    #[cfg(feature = "japanese")]
    pub fn set_full_width_digits(&mut self, on: bool) {
        self.width.full_width_digits = on;
    }

    /// A space after Japanese text is full-width.
    #[cfg(feature = "japanese")]
    pub fn set_full_width_space(&mut self, on: bool) {
        self.width.full_width_space = on;
    }

    /// The kana→kanji conversion in progress, if any.
    #[cfg(feature = "japanese")]
    pub fn conversion(&self) -> Option<&Conversion> {
        self.ja.conversion.as_ref()
    }

    pub fn keys(&self) -> &str {
        self.word.keys()
    }
    pub fn app(&self) -> Option<&str> {
        self.app.as_deref()
    }
    pub fn policy(&self) -> AppPolicy {
        self.policy
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn mode(&self) -> InputMode {
        match (self.paused, self.korean_only) {
            (true, _) => InputMode::English,
            (false, true) => InputMode::Korean,
            (false, false) => InputMode::Auto,
        }
    }
    pub fn last_trace(&self) -> Option<&Trace> {
        self.last_trace.as_ref()
    }
    pub fn engine(&self) -> &Arc<LearningEngine> {
        &self.engine
    }
    /// A word is being composed.
    pub fn is_composing(&self) -> bool {
        !self.word.is_empty()
    }
    /// A Japanese conversion (the candidate list) is open: Space, the arrows and the
    /// digits act on it, not on the app.
    #[cfg(feature = "japanese")]
    pub fn is_converting(&self) -> bool {
        self.ja.conversion.is_some()
    }
    /// A just-committed word can still be flipped with Esc.
    pub fn has_flip_window(&self) -> bool {
        self.last_commit.is_some()
    }

    /// A new text field became active: pick its app's policy and forget the sentence context.
    pub fn activate(&mut self, exe: Option<&str>) {
        self.width.after_digit = false;
        self.word_after_digit = false;
        self.word.invalidate_correction_origin();
        #[cfg(feature = "japanese")]
        if let Some(converter) = &self.ja.converter {
            converter.reload();
        }
        #[cfg(feature = "japanese")]
        {
            self.ja.live_cache = None;
        }
        self.close_flip_window();
        self.app = exe.map(str::to_lowercase);
        self.policy = self.policies.policy(exe);
        self.previous_language = None;
        self.paused = false;
        self.korean_only = false;
    }

    /// Next mode in this text field: tacit → Korean → English → tacit. What was being typed
    /// goes in first.
    pub fn cycle_mode(&mut self, client: &mut dyn TextClient) {
        match self.mode() {
            InputMode::Auto => {
                self.commit(client);
                self.korean_only = true;
                self.previous_language = None;
            }
            InputMode::Korean => {
                self.korean_only = false;
                self.toggle_pause(client);
            }
            InputMode::English => self.toggle_pause(client),
        }
    }

    /// Pauses or resumes in this text field; what was being typed goes in first.
    pub fn toggle_pause(&mut self, client: &mut dyn TextClient) {
        if !self.paused {
            self.commit(client);
        }
        self.word.invalidate_correction_origin();
        self.paused = !self.paused;
        self.previous_language = None;
    }

    /// What Esc switches `current` to for these keys. The engine is asked whether it reads them
    /// as English + Korean, because only it knows (the plain split misses `xdp` → `x에`).
    pub(super) fn next_language(&self, current: Language, keys: &str) -> Language {
        let engine_mixed = self.engine.render(keys, Language::Mixed) != keys;
        current.next_with(keys, &self.languages, engine_mixed)
    }

    /// Whether the session would consume `key` right now without a word being composed:
    /// full-width symbols, digits and spaces after Japanese text. The TIP asks before letting
    /// such a key go to the app.
    #[cfg(feature = "japanese")]
    pub fn claims_key(&self, key: Key) -> bool {
        if !self.policy.enabled
            || self.paused
            || !self.japanese_enabled()
            || !self.word.keys().is_empty()
            || self.ja.conversion.is_some()
        {
            return false;
        }
        match key {
            Key::ShiftSpace => {
                self.policy.full_width
                    && !(self.width.full_width_space
                        && self.previous_language == Some(Language::Japanese))
            }
            Key::Printable(' ') => {
                self.policy.full_width
                    && self.width.full_width_space
                    && self.previous_language == Some(Language::Japanese)
            }
            Key::Printable(ch) if ch.is_ascii_digit() => {
                self.digits_full_width() && self.previous_language == Some(Language::Japanese)
            }
            Key::Printable(ch) => {
                RomajiComposer::is_symbol_key(ch)
                    && ((self.previous_language == Some(Language::Japanese)
                        && !self.width.after_digit)
                        || self.closes_full_width(ch))
            }
            _ => false,
        }
    }

    /// Returns true when the key was consumed.
    pub fn handle(&mut self, key: Key, client: &mut dyn TextClient) -> bool {
        if !self.policy.enabled || self.paused {
            return false;
        }
        #[cfg(feature = "japanese")]
        if key == Key::ShiftSpace {
            // With nothing being typed, the other width: full-width unless Space already types that.
            let eligible = self.word.keys().is_empty()
                && self.ja.conversion.is_none()
                && self.japanese_enabled()
                && self.policy.full_width;
            if !eligible {
                return self.handle(Key::Printable(' '), client);
            }
            self.width.after_digit = false;
            self.close_flip_window();
            if self.width.full_width_space && self.previous_language == Some(Language::Japanese) {
                return false; // the app types the half-width " "
            }
            client.insert_text("　", None);
            return true;
        }
        #[cfg(feature = "japanese")]
        if self.ja.conversion.is_some() {
            return self.handle_converting(key, client);
        }
        #[cfg(not(feature = "japanese"))]
        if key == Key::ShiftSpace {
            return self.handle(Key::Printable(' '), client);
        }
        if !(matches!(key, Key::Letter(_))
            || (matches!(key, Key::Printable(_)) && self.word.keys().is_empty()))
        {
            self.width.after_digit = false;
        }
        let pending = self.last_commit.take();
        // Esc switches languages unless the app owns it; Shift+Esc always does. With a single
        // language there is nothing to switch to, and the app gets the key.
        let switches = (key == Key::Cycle || (key == Key::Escape && !self.policy.escape_to_app))
            && self.languages.len() > 1;
        if switches && self.word.is_empty() {
            if let Some(changed) = pending.as_ref().and_then(|p| self.cycle(p, client)) {
                self.last_commit = Some(changed);
                return true;
            }
        }
        if let Some(p) = &pending {
            self.finalize(p);
        }

        match key {
            Key::Letter(ch) => self.on_letter(ch, client),
            Key::Delete => self.on_delete(client),
            Key::Escape | Key::Cycle => self.on_cycle(switches, client),
            Key::Printable(ch) => self.on_printable(ch, client),
            Key::Enter => self.on_enter(client),
            Key::Tab => self.on_tab(client),
            #[cfg(feature = "japanese")]
            Key::Katakana => self.on_kana_form(true, client),
            #[cfg(feature = "japanese")]
            Key::Hiragana => self.on_kana_form(false, client),
            // Handled at the top of `handle`, before a word exists.
            Key::ShiftSpace => false,
            #[cfg(not(feature = "japanese"))]
            Key::Katakana | Key::Hiragana => self.on_other(client),
            Key::Up | Key::Down | Key::Left | Key::Right | Key::Other => self.on_other(client),
        }
    }

    fn on_letter(&mut self, ch: char, client: &mut dyn TextClient) -> bool {
        let after_digit = std::mem::take(&mut self.width.after_digit);
        // A symbol on its own is confirmed by the next word, which is judged by itself.
        #[cfg(feature = "japanese")]
        if !self.word.keys().is_empty()
            && self.word.keys().chars().all(RomajiComposer::is_symbol_key)
        {
            self.commit(client);
        }
        if self.word.keys().is_empty() {
            self.last_recorded = None; // a new word
            if std::mem::take(&mut self.edited_back) {
                self.previous_language = self
                    .language_before_caret(client)
                    .or(self.previous_language);
            }
            if self.korean_only {
                self.word.force(Language::Korean, false);
            }
            self.word_after_digit = after_digit;
            #[cfg(feature = "japanese")]
            {
                self.ja.live_suppressed = false;
            }
        }
        #[cfg(feature = "japanese")]
        {
            self.ja.added_n_marker = false;
        }
        self.word.keys_mut().push(ch);
        // Mending the Hangul just typed (한국 → 한ㄱ, then `r`) stays Korean; the engine judging
        // the whole key string would flip it to Latin. Once the edited part no longer reads as
        // Hangul, the engine decides again (한 + hello).
        if !(self.word.pinned() == Some(Language::Korean) && self.word.edit_reads_as_hangul()) {
            self.word.unpin();
        }
        self.update_preview(client);
        true
    }

    /// The app is about to delete committed text: what the next word follows is whatever is left
    /// before the caret, not the sentence the session remembers.
    fn language_before_caret(&self, client: &dyn TextClient) -> Option<Language> {
        let caret = client.selected_range().location;
        if caret == NOT_FOUND || caret == 0 {
            return None;
        }
        let ch = client
            .text_in(Range {
                location: caret - 1,
                length: 1,
            })?
            .chars()
            .next()?;
        match ch {
            '\u{AC00}'..='\u{D7A3}' | '\u{3131}'..='\u{318E}' => Some(Language::Korean),
            c if c.is_ascii_alphabetic() => Some(Language::English),
            _ => None,
        }
    }

    fn on_delete(&mut self, client: &mut dyn TextClient) -> bool {
        if self.word.is_empty() {
            self.edited_back = true;
            return false;
        }
        let shown = self.word.shown_language();
        if self.word.forced().is_none() {
            self.word.pin(shown);
        }
        match shown {
            #[cfg(feature = "japanese")]
            Some(Language::Japanese) => self.delete_kana(),
            Some(Language::Korean) => self.word.delete_jamo_or_syllable(),
            _ => {
                self.word.keys_mut().pop();
            }
        }
        #[cfg(feature = "japanese")]
        {
            self.ja.added_n_marker = false;
        }
        if self.word.is_empty() {
            // The owned suffix is gone, but its preceding digit is still in the app.
            self.width.after_digit = std::mem::take(&mut self.word_after_digit);
            self.word.reset();
        }
        self.update_preview(client);
        true
    }

    /// Esc / Shift+Esc on a word being typed. `switches`: false where the app owns Esc.
    fn on_cycle(&mut self, switches: bool, client: &mut dyn TextClient) -> bool {
        if self.word.is_empty() {
            return false;
        }
        if !switches {
            // The app's Esc: the word goes in as shown, then the app gets the key.
            self.commit(client);
            return false;
        }
        // A live-converted phrase goes back to its kana first.
        #[cfg(feature = "japanese")]
        if self.is_live()
            && self.word.forced().or(self.word.preview_language()) == Some(Language::Japanese)
        {
            self.ja.live_suppressed = true;
            self.update_preview(client);
            return true;
        }
        // Cycle from what the user sees.
        let shown = match self.word.shown_language() {
            Some(l) => l,
            None => self.decide(true).language,
        };
        self.read_as(self.next_language(shown, self.word.keys()), client);
        true
    }

    /// The word being typed is read as `language`, by the user's choice.
    fn read_as(&mut self, language: Language, client: &mut dyn TextClient) {
        self.word.choose_reading(language);
        #[cfg(feature = "japanese")]
        if language != Language::Japanese {
            self.drop_n_marker();
        }
        self.update_preview(client);
    }

    /// Space, digit or punctuation: the word goes in, the app types the key itself.
    /// Japanese: symbols and digits inside a phrase join it, Space opens the conversion.
    fn on_printable(&mut self, ch: char, client: &mut dyn TextClient) -> bool {
        if self.word.keys().is_empty() {
            #[cfg(feature = "japanese")]
            let handled = self.on_printable_empty(ch, client);
            #[cfg(not(feature = "japanese"))]
            let handled = false;
            self.width.after_digit = ch.is_numeric();
            return handled;
        }
        #[cfg(feature = "japanese")]
        if self.japanese_enabled() && self.belongs_to_japanese_phrase(ch) {
            self.word.keys_mut().push(ch);
            // Digits don't read as anything: the phrase stays Japanese, as it was shown.
            if ch.is_numeric() && self.word.forced().is_none() {
                self.word.force(Language::Japanese, false);
            }
            self.update_preview(client);
            return true;
        }
        #[cfg(feature = "japanese")]
        if ch == ' '
            && self.current_language() == Some(Language::Japanese)
            && self.ja.converter.is_some()
        {
            self.start_conversion(client);
            return true;
        }
        #[cfg(feature = "japanese")]
        if self.closes_full_width(ch) {
            // 日本語（English）
            self.commit(client);
            self.start_symbol(ch, client);
            return true;
        }
        let marked = client.marked_range();
        let location = if marked.location != NOT_FOUND {
            marked.location
        } else {
            client.selected_range().location
        };
        self.word_before_digit = ch.is_ascii_digit();
        let word = self.finish_word();
        self.word_before_digit = false;
        self.width.after_digit = ch.is_numeric();
        // Only the word goes in; the app types the space or punctuation itself, as with
        // the system's Korean input.
        client.insert_text(&word.text, None);
        self.last_commit = Some(Committed {
            keys: word.keys,
            decided: word.decided,
            committed_as: word.language,
            language: word.language,
            text: format!("{}{}", word.text, ch),
            boundary: ch.to_string(),
            location,
        });
        false
    }

    /// A printable key with nothing being typed: after Japanese text a space is full-width
    /// (`full_width_space`), a digit starts a Japanese number word (`full_width_digits`), and
    /// a symbol becomes a Japanese symbol word of its own, underlined until Return or the next
    /// word confirms it, as in a Japanese IME; Space shows its other forms.
    #[cfg(feature = "japanese")]
    fn on_printable_empty(&mut self, ch: char, client: &mut dyn TextClient) -> bool {
        if self.previous_language == Some(Language::Japanese) {
            #[cfg(feature = "japanese")]
            if ch == ' '
                && self.width.full_width_space
                && self.japanese_enabled()
                && self.policy.full_width
            {
                client.insert_text("　", None);
                return true;
            }
            // A number after Japanese text starts a Japanese word: １２３, ３がつ → ３月.
            if ch.is_ascii_digit() && self.digits_full_width() {
                self.start_symbol(ch, client);
                return true;
            }
        }
        // Not inside a number (3.5, 10:30), but a bracket closes the way it was opened.
        if RomajiComposer::is_symbol_key(ch)
            && ((self.previous_language == Some(Language::Japanese) && !self.width.after_digit)
                || self.closes_full_width(ch))
        {
            self.start_symbol(ch, client);
            return true;
        }
        false
    }

    fn on_enter(&mut self, client: &mut dyn TextClient) -> bool {
        if self.word.is_empty() {
            return false;
        }
        // Like a Japanese IME, Return on an underlined word only confirms it; the next
        // Return is the app's. With the setting off the app gets this one too — except after
        // a Japanese phrase, which is only confirmed either way.
        #[cfg(feature = "japanese")]
        let japanese = self.current_language() == Some(Language::Japanese);
        #[cfg(not(feature = "japanese"))]
        let japanese = false;
        self.commit(client);
        self.policy
            .return_confirms_only
            .unwrap_or(self.return_confirms_only)
            || japanese
    }

    fn on_tab(&mut self, client: &mut dyn TextClient) -> bool {
        let Some(word) = self.suggestion() else {
            return self.on_other(client);
        };
        let language = self.word.shown_language().unwrap_or(Language::English);
        self.word.reset();
        self.word_after_digit = false;
        client.insert_text(&word, None);
        if let Some(h) = &self.history {
            h.record(&word);
        }
        self.previous_language = Some(language.sentence_language());
        true
    }

    /// ⌃K/⌃J and F7/F6: the Japanese reading as katakana / hiragana. Only for a Japanese
    /// phrase; otherwise the keys stay the app's shortcuts.
    #[cfg(feature = "japanese")]
    fn on_kana_form(&mut self, katakana: bool, client: &mut dyn TextClient) -> bool {
        if self.word.keys().is_empty()
            || !self.japanese_enabled()
            || !(self.word.forced() == Some(Language::Japanese)
                || self.word.preview_language() == Some(Language::Japanese))
        {
            self.commit(client);
            return false;
        }
        self.show_kana_forms(katakana, None, None, client);
        true
    }

    /// Arrows, shortcuts…: what is typed goes in and the app handles the key.
    fn on_other(&mut self, client: &mut dyn TextClient) -> bool {
        self.commit(client);
        self.previous_language = None;
        false
    }

    /// Commits whatever is in progress as-is (focus change, shortcut, deactivation).
    pub fn commit(&mut self, client: &mut dyn TextClient) {
        self.width.after_digit = false;
        self.close_flip_window();
        #[cfg(feature = "japanese")]
        if let Some(conversion) = self.ja.conversion.take() {
            let display = conversion.display();
            client.insert_text(&display, None);
            self.note_brackets(&display);
            #[cfg(feature = "japanese")]
            if let Some(converter) = &self.ja.converter {
                converter.learn(conversion.current(), &conversion.kana);
            }
            self.word.keys_mut().clear();
            self.word_after_digit = false;
            self.previous_language = Some(Language::Japanese);
            return;
        }
        if self.word.is_empty() {
            self.word_after_digit = false;
            return;
        }
        let word = self.finish_word();
        client.insert_text(&word.text, None);
    }

    /// Commits what's in progress after the caret was moved elsewhere (a click): the next
    /// word doesn't continue the previous sentence.
    pub fn commit_for_new_context(&mut self, client: &mut dyn TextClient) {
        self.commit(client);
        if let Some(h) = &self.history {
            h.flush();
            h.reload();
        }
        self.previous_language = None;
    }

    // MARK: - Private

    /// The word Tab would complete the one being typed to (None: Tab passes through, and
    /// always without autocomplete, i.e. without a history).
    pub fn suggestion(&self) -> Option<String> {
        let history = self.history.as_deref()?;
        let language = self.word.shown_language()?;
        let prefix = self.preview(language)?;
        history::suggest(Some(history), language, &prefix)
    }

    /// Route Enter and explicit Space conversion by the reading actually on screen.
    #[cfg(feature = "japanese")]
    fn current_language(&self) -> Option<Language> {
        self.word.shown_language()
    }

    /// Decides the current word, learns an Esc override, and resets the buffer.
    fn finish_word(&mut self) -> Word {
        let engine_choice = self.committed_language();
        let language = self.word.override_language().unwrap_or(engine_choice);
        #[cfg(feature = "japanese")]
        if language != Language::Japanese {
            self.drop_n_marker();
        }
        // "Back to kana" without the user's Esc, or a reading kept while deleting, isn't a
        // correction: report it as the engine's.
        let decided = if (self.word.forced().is_some() && !self.word.forced_by_user())
            || (self.word.forced().is_none() && self.word.pinned().is_some())
        {
            language
        } else {
            engine_choice
        };
        if self.word.forced().is_some() && self.word.forced_by_user() {
            let keys = self.word.keys().to_owned();
            self.record(&keys, language, decided);
        }
        let text = match language {
            #[cfg(feature = "japanese")]
            Language::Japanese => self.committed_japanese(),
            Language::Korean => self
                .word
                .hangul()
                .unwrap_or_else(|| self.engine.render(self.word.keys(), language)),
            other => self.engine.render(self.word.keys(), other),
        };
        #[cfg(feature = "japanese")]
        {
            self.ja.live_suppressed = false;
        }
        #[cfg(feature = "japanese")]
        {
            self.note_brackets(&text);
        }
        if let Some(h) = &self.history {
            h.record(&text);
        }
        let word = Word {
            keys: std::mem::take(self.word.keys_mut()),
            text,
            language,
            decided,
        };
        self.previous_language = Some(language.sentence_language());
        self.word.reset();
        self.word_after_digit = false;
        #[cfg(feature = "japanese")]
        {
            self.ja.added_n_marker = false;
        }
        word
    }
}

#[cfg(not(feature = "japanese"))]
impl InputSession {
    pub fn is_converting(&self) -> bool {
        false
    }
    pub fn claims_key(&self, _key: Key) -> bool {
        false
    }
}
