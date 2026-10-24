//! The Japanese side of the session: the kana reading and preview, in-phrase symbols and
//! full-width digits/spaces, and the kana→kanji conversion flow. Port of the Japanese parts
//! of InputSession.swift.

use std::collections::HashSet;

use super::candidates::{page_of, PAGE_SIZE};
use super::{char_count, InputSession, Key, TextClient};
use crate::decide::convert::ConversionCandidate;
use crate::decide::engine::{DecisionContext, DecisionEngine, Language};
use crate::decide::segmenter::{JapaneseSegmenter, Segment};
use crate::lang::english::EnglishModel;
use crate::lang::hangul::{DubeolsikLayout, HangulComposer};
use crate::lang::korean::UNKNOWN_SYLLABLE_LOGP;
use crate::lang::particles::is_ending;
use crate::lang::romaji::RomajiComposer;

/// A kana→kanji conversion in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversion {
    /// The reading still being converted.
    pub kana: String,
    /// The romaji that produced `kana`, so Esc can go back to editing it.
    pub keys: String,
    pub candidates: Vec<ConversionCandidate>,
    pub selected: usize,
    /// The user chose Japanese for these keys with Esc (going back to kana keeps that).
    pub(crate) user_forced: bool,
    /// The clause committed immediately before this remaining reading.
    pub(crate) previous: Option<ConversionCandidate>,
}

impl Conversion {
    pub fn current(&self) -> &ConversionCandidate {
        &self.candidates[self.selected]
    }

    /// What the user sees: the selected candidate, then any reading it doesn't cover.
    pub fn display(&self) -> String {
        let covered = self.current().kana_count;
        let rest: String = self.kana.chars().skip(covered).collect();
        format!("{}{}", self.current().text, rest)
    }
}

impl InputSession {
    /// Digits after Japanese text, and inside a Japanese phrase, are part of it and full-width
    /// (`Settings.full_width_digits`), as addresses in Japan often need.
    pub(super) fn digits_full_width(&self) -> bool {
        self.japanese_enabled() && self.width.full_width_digits && self.policy.full_width
    }

    /// A symbol typed as its own Japanese word, shown full-width (or as the 「」・ a Japanese IME types).
    pub(super) fn start_symbol(&mut self, ch: char, client: &mut dyn TextClient) {
        self.last_recorded = None;
        self.ja.live_suppressed = false;
        *self.word.keys_mut() = ch.to_string();
        self.word_after_digit = false;
        self.word.force(Language::Japanese, false);
        self.update_preview(client);
    }

    /// Full-width openers not closed yet in this sentence, so ) ] } close them full-width
    /// after an English word too.
    pub(super) fn closes_full_width(&self, ch: char) -> bool {
        let opener = match ch {
            ')' => '（',
            ']' => '「',
            '}' => '｛',
            _ => return false,
        };
        self.width.open_brackets.last() == Some(&opener)
    }

    /// Keeps `open_brackets` up to date with text going in.
    pub(super) fn note_brackets(&mut self, text: &str) {
        for ch in text.chars() {
            if matches!(ch, '（' | '「' | '｛' | '『' | '【') {
                self.width.open_brackets.push(ch);
            } else if let Some(opener) = match ch {
                '）' => Some('（'),
                '」' => Some('「'),
                '｝' => Some('｛'),
                '』' => Some('『'),
                '】' => Some('【'),
                _ => None,
            } {
                if self.width.open_brackets.last() == Some(&opener) {
                    self.width.open_brackets.pop();
                }
            }
        }
    }

    /// Whether a symbol typed after the current word continues a Japanese phrase.
    /// Sentence punctuation (、。？！) never changes the word's language: it joins only a
    /// word already read as Japanese ("hello." and "dho?" = 왜? stay what they were).
    /// The in-word marks ー and ' may also turn an English-looking start into Japanese
    /// ("ra" → "ra-" → らー), but never take a word away from Korean.
    pub(super) fn belongs_to_japanese_phrase(&self, ch: char) -> bool {
        if ch.is_ascii() && ch.is_ascii_digit() {
            if !self.digits_full_width() {
                return false;
            }
            return self.word.forced() == Some(Language::Japanese)
                || (self.word.forced().is_none()
                    && self.word.preview_language() == Some(Language::Japanese)
                    && matches!(
                        RomajiComposer::compose(self.word.keys(), true)
                            .pending
                            .as_str(),
                        "" | "n"
                    ));
        }
        if !RomajiComposer::is_in_phrase_symbol(ch) {
            return false;
        }
        if self.word.forced() == Some(Language::Japanese) {
            return true;
        }
        // A letter still waiting for its vowel says the word may not be Japanese after all
        // (baz) is baz), not ばz）); a final n is ん (nihon. is にほん。).
        if self.word.preview_language() == Some(Language::Japanese) {
            return ch == '-'
                || ch == '\''
                || matches!(
                    RomajiComposer::compose(self.word.keys(), true)
                        .pending
                        .as_str(),
                    "" | "n"
                );
        }
        // An English word before the mark keeps it English (to-do, co-op); ra-men isn't one.
        // A hyphen always re-decides, even after a word the list knows: めーる, よそろー and
        // どーなつ start with me, yo, do, and the engine (the hyphen weighs against English)
        // is what keeps them typable. An English compound whose reading is clean kana
        // (to-do → とーど) leans Japanese now; Esc reads it back. (macOS Seamtype keeps the
        // word English here; this port re-decides instead.)
        if self.word.forced().is_none()
            && (ch == '-' || ch == '\'')
            && self.word.preview_language() == Some(Language::English)
            && (ch == '-' || !EnglishModel::shared().is_known_word(self.word.keys()))
            && RomajiComposer::compose(self.word.keys(), true).stray_letters == 0
        {
            let context = DecisionContext {
                previous_language: self.previous_language,
                partial: true,
                ..Default::default()
            };
            let mut keys = self.word.keys().to_owned();
            keys.push(ch);
            return self.engine.decide(&keys, &context).language == Language::Japanese;
        }
        false
    }

    /// Kana for the keys, with any English words inside the phrase kept as typed.
    pub(super) fn japanese_reading(&self, partial: bool) -> String {
        if !partial {
            if let Some(segments) = self
                .ja
                .segmenter
                .as_ref()
                .and_then(|s| s.segment(self.word.keys()))
            {
                return JapaneseSegmenter::text(&segments);
            }
            return RomajiComposer::compose(self.word.keys(), false).text;
        }
        self.japanese_partial_reading().0
    }

    /// The visible reading and only the pending letters belonging to its Japanese tail.
    fn japanese_partial_reading(&self) -> (String, String) {
        let composed = RomajiComposer::compose(self.word.keys(), true);
        // While typing, a consonant at the end is just waiting for its vowel (watashihag → わたしはg),
        // not a stray letter to split an English word ("hag") out of.
        if composed.stray_letters > 0 {
            if let Some(segments) = self
                .ja
                .segmenter
                .as_ref()
                .and_then(|s| s.segment(self.word.keys()))
            {
                // While typing, the last segment may still be growing; keep its pending letters visible.
                if let Some((last, head)) = segments.split_last() {
                    let (tail, pending) = match last {
                        Segment::Japanese { keys, .. } => {
                            let tail = RomajiComposer::compose(keys, true);
                            (tail.text, tail.pending)
                        }
                        Segment::English(word) => (word.clone(), String::new()),
                    };
                    return (
                        format!("{}{}", JapaneseSegmenter::text(head), tail),
                        pending,
                    );
                }
            }
        }
        (composed.text, composed.pending)
    }

    // MARK: - Live conversion

    pub(super) fn is_live(&self) -> bool {
        self.ja.live_conversion && !self.ja.live_suppressed && self.ja.converter.is_some()
    }

    /// The top candidate covering all of `kana` (cached: the preview asks on every key).
    fn live_candidate(&mut self, kana: &str) -> Option<ConversionCandidate> {
        let previous_rc = self
            .word
            .previous_clause
            .as_ref()
            .and_then(|c| c.right_context);
        if let Some((cached_kana, cached_rc, cached)) = &self.ja.live_cache {
            if cached_kana == kana && *cached_rc == previous_rc {
                return cached.clone();
            }
        }
        let mut candidate = if Self::digits_of(kana).is_empty() {
            self.ja
                .converter
                .as_ref()
                .and_then(|c| c.best_candidate(kana, self.word.previous_clause.as_ref()))
        } else {
            // Number-preserving reordering needs the complete candidate list.
            self.conversion_candidates(kana)
                .into_iter()
                .find(|c| c.kana_count == char_count(kana) && !c.is_prediction)
        };
        // A short reading only turns into kanji (私, 今日), not into katakana mid-word
        // (と → ト, こんに → コンに): those flicker while the phrase is still being typed.
        if char_count(kana) <= 3 {
            if let Some(found) = &candidate {
                if !found
                    .text
                    .chars()
                    .any(|ch| (0x4E00..=0x9FFF).contains(&(ch as u32)))
                {
                    candidate = None;
                }
            }
        }
        self.ja.live_cache = Some((kana.to_string(), previous_rc, candidate.clone()));
        candidate
    }

    /// The Japanese phrase as shown while typing: kana, or with live conversion the top
    /// candidate followed by letters still waiting for their vowel (今日はいい天気でs).
    pub(super) fn japanese_display(&mut self) -> String {
        self.render_japanese(false)
    }

    /// Plain confirmation keeps the visible segmentation and live candidate. Only a genuinely
    /// pending Japanese n is completed; an English segment's final n is already literal text.
    pub(super) fn committed_japanese(&mut self) -> String {
        self.render_japanese(true)
    }

    fn render_japanese(&mut self, confirming: bool) -> String {
        let (mut reading, pending) = self.japanese_partial_reading();
        let complete_n = confirming && pending == "n";
        // The composer appends pending letters verbatim; English tails have no pending suffix.
        if self.is_live() && reading.len() > pending.len() {
            let kana = &reading[..reading.len() - pending.len()];
            if let Some(candidate) = self.live_candidate(kana) {
                if confirming {
                    if let Some(converter) = &self.ja.converter {
                        converter.learn(&candidate, kana);
                    }
                }
                let tail = if complete_n { "ん" } else { &pending };
                return format!("{}{}", candidate.text, tail);
            }
        }
        if complete_n {
            reading.pop();
            reading.push('ん');
        }
        reading
    }

    // MARK: - Conversion

    pub(super) fn start_conversion(&mut self, client: &mut dyn TextClient) {
        let kana = self.japanese_reading(false);
        if self.ja.converter.is_none() || kana.is_empty() {
            return;
        }
        let user_forced =
            self.word.forced() == Some(Language::Japanese) && self.word.forced_by_user();
        if user_forced {
            let keys = self.word.keys().to_owned();
            let engine_choice = self.committed_language();
            self.record(&keys, Language::Japanese, engine_choice);
        }
        let candidates = self.conversion_candidates(&kana);
        let live = if self.is_live() {
            self.live_candidate(&kana)
        } else {
            None
        };
        if candidates.is_empty() {
            self.commit(client);
            return;
        }
        // With live conversion the top candidate is already on screen: Space shows the next one.
        let mut selected = 0;
        if let Some(live) = &live {
            if let Some(i) = candidates.iter().position(|c| c == live) {
                if candidates.len() > 1 {
                    selected = (i + 1) % candidates.len();
                }
            }
        }
        self.ja.live_suppressed = false;
        self.ja.conversion = Some(Conversion {
            kana: kana.clone(),
            keys: std::mem::take(self.word.keys_mut()),
            candidates,
            selected,
            user_forced,
            previous: self.word.previous_clause.take(),
        });
        self.word.release();
        self.word.syllable_start = None;
        self.previous_language = Some(Language::Japanese);
        let display = self
            .ja
            .conversion
            .as_ref()
            .map(|c| c.display())
            .unwrap_or_default();
        client.set_marked_text(&display);
    }

    /// Shows the reading as katakana (or hiragana) as the selected "candidate": Return
    /// commits it, Space flips to the other form, typing on commits it and continues.
    pub(super) fn show_kana_forms(
        &mut self,
        katakana_first: bool,
        kana: Option<String>,
        keys: Option<String>,
        client: &mut dyn TextClient,
    ) {
        let kana = kana.unwrap_or_else(|| self.japanese_reading(false));
        let keys = keys.unwrap_or_else(|| self.word.keys().to_owned());
        let forms = vec![
            ConversionCandidate::new(RomajiComposer::katakana(&kana), char_count(&kana)),
            ConversionCandidate::new(kana.clone(), char_count(&kana)),
        ];
        let candidates: Vec<ConversionCandidate> = if katakana_first {
            forms
        } else {
            forms.into_iter().rev().collect()
        };
        self.ja.conversion = Some(Conversion {
            kana,
            keys,
            candidates,
            selected: 0,
            user_forced: false,
            previous: self.word.previous_clause.take(),
        });
        self.word.keys_mut().clear();
        self.word.release();
        self.word.syllable_start = None;
        self.previous_language = Some(Language::Japanese);
        let display = self
            .ja
            .conversion
            .as_ref()
            .map(|c| c.display())
            .unwrap_or_default();
        client.set_marked_text(&display);
    }

    /// The conversion with its first clause `step` kana shorter or longer (今日は|いいてんき):
    /// candidates for exactly that much of the reading, the rest shown as kana. A boundary
    /// never falls inside an English word (google). None when it can't move further.
    fn resize_first_clause(&mut self, conversion: &Conversion, step: i32) -> Option<Conversion> {
        self.ja.converter.as_ref()?;
        let reading: Vec<char> = conversion.kana.chars().collect();
        let mut length = conversion.current().kana_count as i64 + step as i64;
        while length >= 1
            && length < reading.len() as i64
            && reading[(length - 1) as usize].is_ascii()
            && reading[(length - 1) as usize].is_alphabetic()
            && reading[length as usize].is_ascii()
            && reading[length as usize].is_alphabetic()
        {
            length += step as i64;
        }
        if length < 1
            || length > reading.len() as i64
            || length == conversion.current().kana_count as i64
        {
            return None;
        }
        let clause: String = reading[..length as usize].iter().collect();
        let candidates: Vec<ConversionCandidate> = self
            .conversion_candidates_after(&clause, conversion.previous.as_ref())
            .into_iter()
            .filter(|c| c.kana_count == char_count(&clause))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        Some(Conversion {
            kana: conversion.kana.clone(),
            keys: conversion.keys.clone(),
            candidates,
            selected: 0,
            user_forced: conversion.user_forced,
            previous: conversion.previous.clone(),
        })
    }

    pub(super) fn handle_converting(&mut self, key: Key, client: &mut dyn TextClient) -> bool {
        let Some(conversion) = self.ja.conversion.take() else {
            return false;
        };
        match key {
            Key::Printable(' ') | Key::Down | Key::ShiftSpace => {
                let mut conversion = conversion;
                conversion.selected = (conversion.selected + 1) % conversion.candidates.len();
                let display = conversion.display();
                self.ja.conversion = Some(conversion);
                client.set_marked_text(&display);
                true
            }

            Key::Up => {
                let mut conversion = conversion;
                let count = conversion.candidates.len();
                conversion.selected = (conversion.selected + count - 1) % count;
                let display = conversion.display();
                self.ja.conversion = Some(conversion);
                client.set_marked_text(&display);
                true
            }

            Key::Left | Key::Right => {
                let step = if key == Key::Left { -1 } else { 1 };
                match self.resize_first_clause(&conversion, step) {
                    Some(resized) => {
                        let display = resized.display();
                        self.ja.conversion = Some(resized);
                        client.set_marked_text(&display);
                    }
                    None => self.ja.conversion = Some(conversion),
                }
                true
            }

            Key::Printable(ch) if ch.is_numeric() => {
                let n = ch.to_digit(10).or_else(|| {
                    let v = ch as u32;
                    (0xFF10..=0xFF19).contains(&v).then(|| v - 0xFF10)
                });
                self.ja.conversion = Some(conversion);
                if let Some(n) = n {
                    // 1-9 pick the current page's lines; a number the page doesn't reach
                    // confirms the selection and types on, as it did for numbers past 9.
                    if n >= 1 && (n as usize) <= PAGE_SIZE {
                        let target = {
                            let conversion = self.ja.conversion.as_ref().unwrap();
                            let (page, _, _) =
                                page_of(conversion.selected, conversion.candidates.len());
                            page * PAGE_SIZE + n as usize - 1
                        };
                        if target < self.ja.conversion.as_ref().unwrap().candidates.len() {
                            self.choose(target, client);
                            return true;
                        }
                    }
                }
                self.commit(client);
                self.handle(key, client)
            }

            Key::Enter => {
                self.ja.conversion = Some(conversion);
                self.commit_candidate(client);
                true
            }

            Key::Katakana | Key::Hiragana => {
                self.word.previous_clause = conversion.previous;
                self.show_kana_forms(
                    key == Key::Katakana,
                    Some(conversion.kana),
                    Some(conversion.keys),
                    client,
                );
                true
            }

            Key::Escape | Key::Cycle | Key::Delete => {
                self.ja.live_suppressed = true; // "back to kana" means kana, also with live conversion
                                                // Back to editing the romaji — when it still spells the reading. After part of a
                                                // phrase with English in it was committed it may not; keep the kana rather than lose it.
                if JapaneseSegmenter::reading_of(&conversion.keys, self.ja.segmenter.as_ref())
                    != conversion.kana
                {
                    client.insert_text(&conversion.kana, None);
                    self.previous_language = Some(Language::Japanese);
                    return true;
                }
                // A final "n" was shown as ん; keep it ("henkan" → "henkan'") instead of
                // letting it wait for a vowel again.
                let mut keys = conversion.keys.clone();
                self.word.syllable_start = None;
                if keys.ends_with('n') && !keys.ends_with("nn") {
                    keys.push('\'');
                    self.ja.added_n_marker = true;
                }
                *self.word.keys_mut() = keys;
                self.word.previous_clause = conversion.previous;
                self.ja.live_cache = None;
                self.word.force(Language::Japanese, conversion.user_forced);
                self.update_preview(client);
                true
            }

            Key::Letter(_) | Key::Printable(_) => {
                self.ja.conversion = Some(conversion);
                self.commit(client);
                self.handle(key, client)
            }

            _ => {
                self.ja.conversion = Some(conversion);
                self.commit(client);
                false
            }
        }
    }

    /// Commits the selected candidate; if it covers only the start of the reading,
    /// keeps converting the rest.
    fn commit_candidate(&mut self, client: &mut dyn TextClient) {
        if self.ja.converter.is_none() {
            return;
        }
        let Some(conversion) = self.ja.conversion.take() else {
            return;
        };
        let candidate = conversion.current().clone();
        if let Some(converter) = &self.ja.converter {
            converter.learn(&candidate, &conversion.kana);
        }
        client.insert_text(&candidate.text, None);
        self.word_after_digit = false;
        self.note_brackets(&candidate.text);
        let rest: String = conversion.kana.chars().skip(candidate.kana_count).collect();
        if rest.is_empty() {
            return;
        }
        let rest_keys = Self::keys_after(&conversion.keys, candidate.kana_count);
        let candidates = self.conversion_candidates_after(&rest, Some(&candidate));
        if candidates.is_empty() {
            client.insert_text(&rest, None);
        } else {
            // The rest is the engine's Japanese, not something the user corrected.
            let rest_conversion = Conversion {
                kana: rest,
                keys: rest_keys,
                candidates,
                selected: 0,
                user_forced: false,
                previous: Some(candidate),
            };
            let display = rest_conversion.display();
            self.ja.conversion = Some(rest_conversion);
            client.set_marked_text(&display);
        }
    }

    /// Commits candidate `index` of the open conversion (e.g. clicked in the candidate window).
    pub fn choose(&mut self, index: usize, client: &mut dyn TextClient) {
        match self.ja.conversion.as_mut() {
            Some(conversion) if index < conversion.candidates.len() => {
                conversion.selected = index;
            }
            _ => return,
        }
        self.commit_candidate(client);
    }

    /// The converter's candidates, with a number kept as typed: a reading that is only a number
    /// offers itself (１２３) and its half-width form (123) first; otherwise the candidates that keep
    /// the digits come before those that spell them out (３月, 3月, then 三月; 東京都１, not 東京都一).
    fn conversion_candidates(&self, kana: &str) -> Vec<ConversionCandidate> {
        self.conversion_candidates_after(kana, self.word.previous_clause.as_ref())
    }

    fn conversion_candidates_after(
        &self,
        kana: &str,
        previous: Option<&ConversionCandidate>,
    ) -> Vec<ConversionCandidate> {
        let candidates = self
            .ja
            .converter
            .as_ref()
            .map(|c| match previous {
                Some(previous) => c.candidates_after(kana, previous),
                None => c.candidates(kana),
            })
            .unwrap_or_default();
        let digits = Self::digits_of(kana);
        if digits.is_empty() {
            return candidates;
        }
        let count = char_count(kana);
        let whole = |only_matching: bool| -> Vec<ConversionCandidate> {
            candidates
                .iter()
                .filter(|c| c.kana_count == count)
                .filter(|c| (Self::digits_of(&c.text) == digits) == only_matching)
                .cloned()
                .collect()
        };
        let mut ordered: Vec<ConversionCandidate> = whole(true);
        ordered.extend(whole(false));
        if kana.chars().all(is_number_character) {
            let mut head = vec![
                ConversionCandidate::new(kana, count),
                ConversionCandidate::new(Self::half_width(kana), count),
            ];
            head.extend(ordered);
            ordered = head;
        }
        let mut shown: HashSet<String> = HashSet::new();
        ordered
            .into_iter()
            .chain(candidates.into_iter().filter(|c| c.kana_count != count))
            .filter(|c| shown.insert(c.text.clone()))
            .collect()
    }

    /// The romaji left after the part that produced the first `kana_count` kana: the
    /// shortest tail that composes to exactly the remaining kana ("kitte" after きっ is
    /// "te"; "konnichiha" after こん is "nichiha").
    pub fn keys_after(keys: &str, kana_count: usize) -> String {
        let chars: Vec<char> = keys.chars().collect();
        let rest: String = RomajiComposer::compose(keys, false)
            .text
            .chars()
            .skip(kana_count)
            .collect();
        for i in (0..=chars.len()).rev() {
            let tail: String = chars[i..].iter().collect();
            if RomajiComposer::compose(&tail, false).text == rest {
                return tail;
            }
        }
        String::new()
    }

    /// The digits of `text` in order, as ASCII ("東京都１－２" → "12").
    pub fn digits_of(text: &str) -> String {
        Self::half_width(text)
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect()
    }

    /// Full-width ASCII (！ to ～) as ASCII.
    pub fn half_width(text: &str) -> String {
        text.chars()
            .map(|c| {
                let v = c as u32;
                if (0xFF01..=0xFF5E).contains(&v) {
                    char::from_u32(v - 0xFEE0).unwrap_or(c)
                } else {
                    c
                }
            })
            .collect()
    }

    /// Takes the last kana off a Japanese phrase, as a Japanese IME does (かんこく → かんこ, not
    /// かんこk): the keys become a spelling of the shorter reading, keeping as much of what was
    /// typed as composes to it. A letter still waiting for its vowel goes alone (かんこk → かんこ),
    /// and so does anything in a phrase with English in it.
    pub(super) fn delete_kana(&mut self) {
        let composed = RomajiComposer::compose(self.word.keys(), true);
        if composed.pending.is_empty() && composed.stray_letters == 0 && !composed.text.is_empty() {
            let shorter: String = composed
                .text
                .chars()
                .take(char_count(&composed.text) - 1)
                .collect();
            if let Some(keys) = Self::keys_spelling(&shorter, self.word.keys()) {
                *self.word.keys_mut() = keys;
                return;
            }
        }
        self.word.keys_mut().pop();
    }

    /// The longest start of `keys` that composes to a start of `kana`, followed by a spelling of
    /// the rest ("kya" for き is "ki"; "kitte" for きっ is "kiltu").
    fn keys_spelling(kana: &str, from: &str) -> Option<String> {
        let chars: Vec<char> = from.chars().collect();
        for cut in (0..chars.len()).rev() {
            let front: String = chars[..cut].iter().collect();
            let composed = RomajiComposer::compose(&front, true);
            if !composed.pending.is_empty()
                || composed.stray_letters != 0
                || !kana.starts_with(&composed.text)
            {
                continue;
            }
            let rest_kana: String = kana.chars().skip(char_count(&composed.text)).collect();
            let Some(rest) = RomajiComposer::romaji(&rest_kana) else {
                continue;
            };
            let respelled = RomajiComposer::compose(&format!("{front}{rest}"), true);
            if respelled.pending.is_empty() && respelled.text == kana {
                return Some(format!("{front}{rest}"));
            }
        }
        None
    }

    pub(super) fn drop_n_marker(&mut self) {
        if self.ja.added_n_marker && self.word.keys().ends_with('\'') {
            self.word.keys_mut().pop();
        }
        self.ja.added_n_marker = false;
    }

    /// Japanese with Korean typed straight after it reads as broken Hangul: a particle
    /// (rirekifmf, 履歴를, not 갹다ㅑ를) or a word (kyouhaehtl…, 今日は도시락은). Once the Korean
    /// part is a particle or two ordinary syllables, the Japanese goes in (kanji with live
    /// conversion) and the rest carries on as Korean, so 를요, 에서 or 도시락은 can follow.
    pub(super) fn split_off_japanese_before_korean(&mut self, client: &mut dyn TextClient) {
        if !self.japanese_enabled() || char_count(self.word.keys()) < 5 {
            return;
        }
        if !HangulComposer::compose(self.word.keys()).is_none_or(|c| c.is_broken()) {
            return;
        }
        // Keys that still spell romaji are more Japanese (kyouhago is きょうはご, not きょうは해).
        if RomajiComposer::compose(self.word.keys(), true).stray_letters == 0 {
            return;
        }
        let chars: Vec<char> = self.word.keys().chars().collect();
        // The longest Japanese front first: きょうは|도시, not きょう|하ㄷ….
        for cut in (3..chars.len()).rev() {
            let front: String = chars[..cut].iter().collect();
            let back: String = chars[cut..].iter().collect();
            if !DubeolsikLayout::starts_syllable(chars[cut]) {
                continue;
            }
            let Some(korean) = HangulComposer::compose(&back) else {
                continue;
            };
            if !korean.stray_jamo().is_empty() {
                continue;
            }
            let korean_text = korean.text();
            if !(is_ending(&korean_text)
                || (korean.syllables().len() >= 2
                    && korean.syllables().iter().all(|s| {
                        super::preview::KOREAN.get().syllable_log_probability(*s)
                            > UNKNOWN_SYLLABLE_LOGP
                    })))
            {
                continue;
            }
            if !RomajiComposer::compose(&front, false).is_clean() {
                continue;
            }
            let decision = self.engine.decide(
                &front,
                &DecisionContext {
                    previous_language: self.previous_language,
                    partial: false,
                    ..Default::default()
                },
            );
            if decision.language != Language::Japanese || decision.confidence < 0.9 {
                continue;
            }
            *self.word.keys_mut() = front;
            let committed = self.committed_japanese();
            client.insert_text(&committed, None);
            *self.word.keys_mut() = back;
            self.word_after_digit = false;
            self.word.syllable_start = None;
            self.previous_language = Some(Language::Japanese);
            self.word.force(Language::Korean, false);
            self.word.forget_preview();
            return;
        }
    }
}

/// What a number reading is made of: digits and the punctuation inside numbers, either width.
fn is_number_character(c: char) -> bool {
    c.is_ascii_digit()
        || ('０'..='９').contains(&c)
        || matches!(
            c,
            '－' | '．' | '，' | '／' | '：' | '-' | '.' | ',' | '/' | ':'
        )
}
