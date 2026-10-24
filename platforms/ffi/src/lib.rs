//! C interface of the input method, for the macOS and Linux text services. Same contract as the
//! core bridge (`tacit_core::session::bridge`): a key, optional text before the caret, and its
//! stable UTF-16 coordinate origin go in; a string comes out — `1` if the key was consumed (else
//! `0`), then each command preceded by
//! `\u{1}`: `M<text>` set the composing text, `I<text>` commit it (replacing the composing
//! text), `B<n>` delete n UTF-16 units before the caret. Free a returned string with
//! `ts_string_free`. Nothing here unwinds into the caller.

use std::ffi::{c_char, c_int, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Mutex;

use tacit_core::session::bridge::Ime;
use tacit_core::store::policy::Override;
use tacit_core::{make_engine, AppPolicies, InputSession, LearningEngine, Settings};
#[cfg(feature = "japanese")]
use tacit_core::{DictionaryConverter, Language};

/// The instance behind a handle from `ts_new`.
pub struct TacitHandle(Mutex<State>);

struct State {
    ime: Ime,
    dir: PathBuf,
    stamps: [Option<std::time::SystemTime>; 3],
}

fn stamps(dir: &std::path::Path) -> [Option<std::time::SystemTime>; 3] {
    ["settings.json", "apps.json", "lexicon.json"].map(|name| {
        std::fs::metadata(dir.join(name))
            .and_then(|m| m.modified())
            .ok()
    })
}

impl State {
    fn activate(&mut self, app: &str) {
        let fresh = stamps(&self.dir);
        if fresh != self.stamps {
            self.ime.reset();
            self.ime = Ime::with_session(session_in(&self.dir));
            self.stamps = fresh;
        }
        self.ime.activate(app);
    }
}

fn text(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    // SAFETY: the caller passes a NUL-terminated string that stays valid for this call.
    unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
}

fn optional_text(s: *const c_char) -> Option<String> {
    (!s.is_null()).then(|| text(s))
}

fn with<R>(h: *mut TacitHandle, fallback: R, f: impl FnOnce(&mut State) -> R) -> R {
    if h.is_null() {
        return fallback;
    }
    // SAFETY: `h` comes from `ts_new` and is used until `ts_free`.
    let handle = unsafe { &*h };
    catch_unwind(AssertUnwindSafe(|| {
        f(&mut handle.0.lock().unwrap_or_else(|e| e.into_inner()))
    }))
    .unwrap_or(fallback)
}

fn reply(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "")).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// The typing session the way the Windows text service builds it: the languages from
/// settings.json whose packs are installed, the app rules, the kana→kanji converter. With no
/// language available every key goes through as typed.
fn session_in(dir: &std::path::Path) -> InputSession {
    let settings = Settings::load(Some(&dir.join("settings.json")));
    tacit_core::pack::refresh_for(tacit_core::pack_dirs(), &settings.languages);
    let mut policies = AppPolicies::load(Some(&dir.join("apps.json")));
    let lexicon = Some(dir.join("lexicon.json"));
    let (_languages, engine) = match settings.languages.available() {
        Some(languages) => {
            let engine: std::sync::Arc<LearningEngine> = make_engine(lexicon, &languages);
            (languages, engine)
        }
        None => {
            policies = AppPolicies::new([(
                AppPolicies::DEFAULT_KEY.to_string(),
                Override {
                    enabled: Some(false),
                    ..Override::default()
                },
            )]);
            let languages = tacit_core::Languages::legacy(false);
            (languages.clone(), make_engine(None, &languages))
        }
    };
    #[allow(unused_mut)]
    let mut session = InputSession::new(engine, policies, settings.return_confirms_only);
    #[cfg(feature = "japanese")]
    if _languages.contains(Language::Japanese) && tacit_core::pack::available(Language::Japanese) {
        let learning = Some(dir.join("kanji_learning.json"));
        session = session.with_japanese(
            std::sync::Arc::new(DictionaryConverter::new(learning)),
            settings.live_conversion,
        );
    }
    #[cfg(feature = "japanese")]
    session.set_full_width_digits(settings.full_width_digits);
    #[cfg(feature = "japanese")]
    session.set_full_width_space(settings.full_width_space);
    session
}

/// A new input method over the settings, apps and lexicon in `dir` (NULL or "": the user's
/// usual folder). NULL on failure.
#[no_mangle]
pub extern "C" fn ts_new(dir: *const c_char) -> *mut TacitHandle {
    let dir = Some(text(dir))
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(tacit_core::support_dir);
    let Some(dir) = dir else {
        return std::ptr::null_mut();
    };
    catch_unwind(|| {
        let ime = Ime::with_session(session_in(&dir));
        let state = State {
            ime,
            stamps: stamps(&dir),
            dir,
        };
        Box::into_raw(Box::new(TacitHandle(Mutex::new(state))))
    })
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
/// # Safety
/// `h` is NULL or from `ts_new`, and is not used afterwards.
pub unsafe extern "C" fn ts_free(h: *mut TacitHandle) {
    if !h.is_null() {
        // SAFETY: the handle is not used after this call.
        drop(unsafe { Box::from_raw(h) });
    }
}

#[no_mangle]
/// # Safety
/// `s` is NULL or a string this library returned, and is not used afterwards.
pub unsafe extern "C" fn ts_string_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: `s` came from this library's `reply`.
        drop(unsafe { CString::from_raw(s) });
    }
}

/// A new text field in the program `app` (picks its rule; NULL or "" for unknown).
#[no_mangle]
pub extern "C" fn ts_activate(h: *mut TacitHandle, app: *const c_char) {
    let app = text(app);
    with(h, (), |state| state.activate(&app));
}

/// `kind`: 0 letter, 1 printable, 2 delete, 3 escape, 4 cycle, 5 enter, 6 tab, else other.
#[no_mangle]
pub extern "C" fn ts_key(
    h: *mut TacitHandle,
    kind: c_int,
    ch: u32,
    before: *const c_char,
    before_start: i64,
) -> *mut c_char {
    let (ch, before) = (char::from_u32(ch).unwrap_or('\0'), optional_text(before));
    let before_start = usize::try_from(before_start).ok();
    reply(with(h, "0".to_string(), |state| {
        state.ime.key(kind, ch, before.as_deref(), before_start)
    }))
}

#[no_mangle]
pub extern "C" fn ts_reset(h: *mut TacitHandle) {
    with(h, (), |state| state.ime.reset());
}

#[no_mangle]
pub extern "C" fn ts_toggle_pause(
    h: *mut TacitHandle,
    before: *const c_char,
    before_start: i64,
) -> *mut c_char {
    let before = optional_text(before);
    let before_start = usize::try_from(before_start).ok();
    reply(with(h, "0".to_string(), |state| {
        state.ime.toggle_pause(before.as_deref(), before_start)
    }))
}

#[no_mangle]
pub extern "C" fn ts_paused(h: *mut TacitHandle) -> c_int {
    with(h, false, |state| state.ime.paused()) as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            loop {
                let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
                let path =
                    std::env::temp_dir().join(format!("tacit-ffi-{}-{id}", std::process::id()));
                match std::fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create test directory: {error}"),
                }
            }
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct Fixture {
        handle: *mut TacitHandle,
        _dir: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = TempDir::new();
            std::fs::write(
                dir.path().join("settings.json"),
                br#"{"languages":["en","ko"],"returnConfirmsOnly":true}"#,
            )
            .unwrap();
            Self::from_dir(dir)
        }

        fn from_dir(dir: TempDir) -> Self {
            let dir_c = CString::new(dir.path().to_string_lossy().as_bytes()).unwrap();
            let handle = ts_new(dir_c.as_ptr());
            assert!(!handle.is_null());
            ts_activate(handle, std::ptr::null());
            Self { handle, _dir: dir }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe { ts_free(self.handle) };
                self.handle = std::ptr::null_mut();
            }
        }
    }

    fn result(p: *mut c_char) -> String {
        assert!(!p.is_null());
        // SAFETY: `p` is a string this library just returned.
        let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        unsafe { ts_string_free(p) };
        s
    }

    fn call_at(
        h: *mut TacitHandle,
        kind: c_int,
        ch: char,
        before: Option<&str>,
        before_start: i64,
    ) -> String {
        let before = before.map(|before| CString::new(before).unwrap());
        let before = before
            .as_ref()
            .map_or(std::ptr::null(), |before| before.as_ptr());
        result(ts_key(h, kind, ch as u32, before, before_start))
    }

    fn toggle_at(h: *mut TacitHandle, before: Option<&str>, before_start: i64) -> String {
        let before = before.map(|before| CString::new(before).unwrap());
        let before = before
            .as_ref()
            .map_or(std::ptr::null(), |before| before.as_ptr());
        result(ts_toggle_pause(h, before, before_start))
    }

    /// A tiny UTF-16 editor that replays the actual bridge protocol and native key passthrough.
    struct Editor {
        units: Vec<u16>,
        cursor: usize,
        marked: Option<Range<usize>>,
        document_origin: usize,
    }

    impl Editor {
        fn new(before: &str, document_origin: usize) -> Self {
            let units: Vec<u16> = before.encode_utf16().collect();
            let cursor = units.len();
            Self {
                units,
                cursor,
                marked: None,
                document_origin,
            }
        }

        fn text(&self) -> String {
            String::from_utf16(&self.units).expect("test editor text stays valid UTF-16")
        }

        fn before_window(&self) -> (String, i64) {
            let mut start = self.cursor.saturating_sub(64);
            if start > 0 && start < self.cursor && (0xdc00..=0xdfff).contains(&self.units[start]) {
                start -= 1;
            }
            let text = String::from_utf16(&self.units[start..self.cursor])
                .expect("window must not split a surrogate pair");
            let absolute = self.document_origin.checked_add(start).unwrap();
            (text, i64::try_from(absolute).unwrap())
        }

        fn composition_origin(&self) -> i64 {
            let local = self
                .marked
                .as_ref()
                .map_or(self.cursor, |range| range.start);
            i64::try_from(self.document_origin.checked_add(local).unwrap()).unwrap()
        }

        fn apply(&mut self, reply: &str) -> bool {
            let mut commands = reply.split('\u{1}');
            let consumed = match commands.next() {
                Some("1") => true,
                Some("0") => false,
                other => panic!("invalid consumed flag {other:?}"),
            };
            for command in commands {
                let Some(tag) = command.chars().next() else {
                    panic!("empty command");
                };
                let rest = &command[tag.len_utf8()..];
                match tag {
                    'M' | 'I' => {
                        let target = self.marked.take().unwrap_or(self.cursor..self.cursor);
                        assert!(target.start <= target.end && target.end <= self.units.len());
                        let inserted: Vec<u16> = rest.encode_utf16().collect();
                        self.units.splice(target.clone(), inserted.iter().copied());
                        self.cursor = target.start + inserted.len();
                        if tag == 'M' && !inserted.is_empty() {
                            self.marked = Some(target.start..self.cursor);
                        }
                    }
                    'B' => {
                        let count: usize = rest.parse().expect("B count must be UTF-16 units");
                        let old_cursor = self.cursor;
                        let start = old_cursor
                            .checked_sub(count)
                            .expect("B deletes before caret");
                        self.units.drain(start..old_cursor);
                        self.cursor = start;
                        if let Some(marked) = self.marked.take() {
                            if marked.end <= start {
                                self.marked = Some(marked);
                            } else if marked.start >= old_cursor {
                                self.marked = Some(marked.start - count..marked.end - count);
                            } else if marked.start >= start {
                                let remaining = marked.end.saturating_sub(old_cursor);
                                if remaining > 0 {
                                    self.marked = Some(start..start + remaining);
                                }
                            }
                        }
                    }
                    other => panic!("unexpected bridge command tag {other:?}"),
                }
            }
            consumed
        }

        fn native_key(&mut self, kind: c_int, ch: char) {
            match kind {
                0 | 1 => {
                    let mut buffer = [0; 2];
                    let inserted = ch.encode_utf16(&mut buffer);
                    self.units
                        .splice(self.cursor..self.cursor, inserted.iter().copied());
                    self.cursor += inserted.len();
                }
                2 if self.cursor > 0 => {
                    self.cursor -= 1;
                    self.units.remove(self.cursor);
                }
                _ => {}
            }
        }

        fn dispatch(
            &mut self,
            h: *mut TacitHandle,
            kind: c_int,
            ch: char,
            before: Option<&str>,
            before_start: i64,
        ) -> String {
            let reply = call_at(h, kind, ch, before, before_start);
            let consumed = self.apply(&reply);
            if !consumed {
                self.native_key(kind, ch);
            }
            reply
        }

        fn press_known(&mut self, h: *mut TacitHandle, kind: c_int, ch: char) -> String {
            let (before, origin) = self.before_window();
            self.dispatch(h, kind, ch, Some(&before), origin)
        }

        fn press_local(&mut self, h: *mut TacitHandle, kind: c_int, ch: char) -> String {
            let origin = self.composition_origin();
            self.dispatch(h, kind, ch, None, origin)
        }

        fn press_unknown(&mut self, h: *mut TacitHandle, kind: c_int, ch: char) -> String {
            self.dispatch(h, kind, ch, None, -1)
        }

        fn press_unlocated_context(
            &mut self,
            h: *mut TacitHandle,
            kind: c_int,
            ch: char,
        ) -> String {
            let (before, _) = self.before_window();
            self.dispatch(h, kind, ch, Some(&before), -1)
        }

        fn type_known(&mut self, h: *mut TacitHandle, text: &str) {
            for ch in text.chars() {
                let kind = if ch.is_ascii_alphabetic() { 0 } else { 1 };
                self.press_known(h, kind, ch);
            }
        }

        fn type_unknown(&mut self, h: *mut TacitHandle, text: &str) {
            for ch in text.chars() {
                let kind = if ch.is_ascii_alphabetic() { 0 } else { 1 };
                self.press_unknown(h, kind, ch);
            }
        }

        fn append_external(&mut self, text: &str) {
            let old_len = self.units.len();
            self.units
                .splice(self.cursor..self.cursor, text.encode_utf16());
            self.cursor += self.units.len() - old_len;
        }
    }

    #[test]
    fn replay_rejects_unexpected_command_tags() {
        let mut editor = Editor::new("", 0);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            editor.apply("0\u{1}Xunexpected")
        }))
        .is_err());
    }

    #[test]
    fn typing_dkssud_gives_korean_through_the_c_interface() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("", 0);
        editor.type_known(fixture.handle, "dkssud");
        let out = editor.press_known(fixture.handle, 5, '\0');
        assert!(out.starts_with('1'), "{out:?}");
        assert_eq!(editor.text(), "안녕");
        assert_eq!(ts_paused(fixture.handle), 0);
    }

    #[test]
    fn confirmation_preserves_visible_korean_and_surrounding_text() {
        for (keys, shown) in [
            ("dustmqgot", "연습햇"),
            ("akfgkfRK", "말할까"),
            ("cmdWL", "cmdWL"),
            ("rmWL", "rmWL"),
            ("dmlWL", "dmlWL"),
            ("turnsRI", "turnsRI"),
            ("cmdWLr", "cmdWLr"),
            ("smfWLr", "smfWLr"),
            ("d", "d"),
            ("Tom's", "Tom's"),
            ("dkssud d", "안녕 ㅇ"),
            ("10cm", "10cm"),
            ("10fps", "10fps"),
            ("10sus", "10년"),
            ("k8s", "k8s"),
        ] {
            for (kind, ch, boundary) in [(1, ' ', " "), (5, '\0', "")] {
                for known_context in [true, false] {
                    let fixture = Fixture::new();
                    let mut editor = Editor::new("앞😀  뒤", 0);
                    editor.cursor = "앞😀 ".encode_utf16().count();
                    if known_context {
                        editor.type_known(fixture.handle, keys);
                    } else {
                        editor.type_unknown(fixture.handle, keys);
                    }
                    assert_eq!(editor.text(), format!("앞😀 {shown} 뒤"));
                    let reply = if known_context {
                        editor.press_known(fixture.handle, kind, ch)
                    } else {
                        editor.press_unknown(fixture.handle, kind, ch)
                    };
                    assert_eq!(reply.starts_with('1'), kind == 5);
                    assert_eq!(editor.text(), format!("앞😀 {shown}{boundary} 뒤"));
                    assert!(editor.marked.is_none());
                    assert_eq!(
                        editor.cursor,
                        format!("앞😀 {shown}{boundary}").encode_utf16().count()
                    );
                }
            }
        }
    }

    #[test]
    fn null_context_retains_composition_across_keystrokes() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("", 0);
        editor.type_unknown(fixture.handle, "dkssudgktpdy");
        let out = editor.press_unknown(fixture.handle, 5, '\0');
        assert!(out.starts_with('1'), "{out:?}");
        assert_eq!(editor.text(), "안녕하세요");
    }

    #[test]
    fn losing_document_access_mid_composition_preserves_a_prefixed_word() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("prefix ", 0);
        editor.press_known(fixture.handle, 0, 'd');
        for ch in "kssudgktpdy".chars() {
            editor.press_unknown(fixture.handle, 0, ch);
        }
        assert_eq!(editor.text(), "prefix 안녕하세요");

        let out = editor.press_known(fixture.handle, 5, '\0');
        assert!(out.starts_with('1'), "{out:?}");
        assert_eq!(editor.text(), "prefix 안녕하세요");
    }

    #[test]
    fn null_context_supports_delete_escape_enter_and_pause() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("", 0);
        editor.type_unknown(fixture.handle, "dkssud");
        editor.press_unknown(fixture.handle, 2, '\0');
        assert_eq!(editor.text(), "안녀");

        editor.press_unknown(fixture.handle, 3, '\0');
        assert_eq!(editor.text(), "dkssu");
        editor.press_unknown(fixture.handle, 5, '\0');
        assert_eq!(editor.text(), "dkssu");

        editor.type_unknown(fixture.handle, "dkssud");
        let out = toggle_at(fixture.handle, None, -1);
        assert!(out.contains("\u{1}I안녕"), "{out:?}");
        editor.apply(&out);
        assert_eq!(editor.text(), "dkssu안녕");
        assert_eq!(ts_paused(fixture.handle), 1);

        let out = toggle_at(fixture.handle, None, -1);
        assert!(out.starts_with('1'), "{out:?}");
        editor.apply(&out);
        assert_eq!(ts_paused(fixture.handle), 0);
    }

    #[test]
    fn toggle_pause_uses_an_explicit_context_origin() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("prefix ", 0);
        editor.type_known(fixture.handle, "dkssud");
        let before = editor.text();
        let out = toggle_at(fixture.handle, Some(&before), 0);
        assert!(out.contains("\u{1}I안녕"), "{out:?}");
        editor.apply(&out);
        assert_eq!(editor.text(), "prefix 안녕");
        assert_eq!(ts_paused(fixture.handle), 1);
    }

    #[test]
    fn known_empty_context_resets_a_nonempty_mark() {
        let fixture = Fixture::new();
        let fresh = Fixture::new();
        let mut editor = Editor::new("", 0);
        let first = editor.press_known(fixture.handle, 0, 'd');
        assert!(first.starts_with('1'), "{first:?}");
        assert!(
            editor.marked.is_some(),
            "reset must start with a nonempty mark"
        );
        let after_reset = call_at(fixture.handle, 0, 'k', Some(""), 0);
        let fresh_key = call_at(fresh.handle, 0, 'k', Some(""), 0);
        assert_eq!(after_reset, fresh_key);
    }

    #[test]
    fn null_context_composition_does_not_leak_between_handles() {
        let first = Fixture::new();
        let second = Fixture::new();
        let (mut a, mut b) = (Editor::new("", 0), Editor::new("", 0));
        for (left, right) in "dkssud".chars().zip("gksrnr".chars()) {
            a.press_unknown(first.handle, 0, left);
            b.press_unknown(second.handle, 0, right);
        }
        assert_eq!(a.text(), "안녕");
        assert_eq!(b.text(), "한국");
    }

    #[test]
    fn a_null_handle_is_ignored() {
        assert_eq!(call_at(std::ptr::null_mut(), 0, 'a', None, -1), "0");
        unsafe { ts_free(std::ptr::null_mut()) };
    }

    fn space_then_escape(
        prefix: &str,
        origin: usize,
        hide_active_context: bool,
    ) -> (String, String, String) {
        let fixture = Fixture::new();
        let mut editor = Editor::new(prefix, origin);
        if hide_active_context {
            editor.press_known(fixture.handle, 0, 'd');
            for ch in "kssud".chars() {
                editor.press_local(fixture.handle, 0, ch);
            }
        } else {
            editor.type_known(fixture.handle, "dkssud");
        }
        let space = if hide_active_context {
            editor.press_local(fixture.handle, 1, ' ')
        } else {
            editor.press_known(fixture.handle, 1, ' ')
        };
        assert_eq!(editor.text(), format!("{prefix}안녕 "));
        let escape = editor.press_known(fixture.handle, 3, '\0');
        (space, escape, editor.text())
    }

    #[test]
    fn known_origin_escape_corrects_only_the_word_and_space() {
        let (space, escape, text) = space_then_escape("prefix ", 0, false);
        assert!(space.starts_with('0'), "{space:?}");
        assert!(escape.contains("\u{1}B3"), "{escape:?}");
        assert_eq!(text, "prefix dkssud ");
    }

    #[test]
    fn first_known_then_active_none_with_known_origin_matches_full_context() {
        let full = space_then_escape("prefix ", 0, false);
        let recovered = space_then_escape("prefix ", 0, true);
        assert_eq!(recovered, full);
        assert!(recovered.1.contains("\u{1}B3"), "{:?}", recovered.1);
        assert_eq!(recovered.2, "prefix dkssud ");
    }

    #[test]
    fn non_bmp_prefix_is_preserved_during_escape_correction() {
        let (_, escape, text) = space_then_escape("📌 prefix ", 0, false);
        assert!(escape.contains("\u{1}B3"), "{escape:?}");
        assert_eq!(text, "📌 prefix dkssud ");
    }

    #[test]
    fn changing_document_window_origins_preserves_the_large_document_anchor() {
        let origin = usize::MAX >> 3;
        let fixture = Fixture::new();
        let mut editor = Editor::new(&"x".repeat(100), origin);
        let mut starts = Vec::new();
        for ch in "dkssud".chars() {
            starts.push(editor.before_window().1);
            editor.press_known(fixture.handle, 0, ch);
        }
        assert!(starts.windows(2).any(|pair| pair[0] != pair[1]));
        let space = editor.press_known(fixture.handle, 1, ' ');
        assert!(space.starts_with('0'), "{space:?}");
        let escape = editor.press_known(fixture.handle, 3, '\0');
        assert!(escape.contains("\u{1}B3"), "{escape:?}");
        assert_eq!(editor.text(), format!("{}dkssud ", "x".repeat(100)));
    }

    #[test]
    fn identical_suffix_at_a_different_caret_does_not_correct() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("prefix ", 0);
        editor.type_known(fixture.handle, "dkssud");
        editor.press_known(fixture.handle, 1, ' ');
        editor.append_external("gap 안녕 ");
        let before = editor.text();
        let out = editor.press_known(fixture.handle, 3, '\0');
        assert!(!out.contains("\u{1}B"), "{out:?}");
        assert_eq!(editor.text(), before);
    }

    #[test]
    fn changed_commit_text_or_boundary_does_not_correct() {
        for change_boundary in [false, true] {
            let fixture = Fixture::new();
            let mut editor = Editor::new("prefix ", 0);
            editor.type_known(fixture.handle, "dkssud");
            editor.press_known(fixture.handle, 1, ' ');
            let end = editor.cursor;
            if change_boundary {
                editor.units[end - 1] = '!' as u16;
            } else {
                let previous = editor.units[end - 2];
                editor.units[end - 2] = if previous == '녕' as u16 {
                    '뇽' as u16
                } else {
                    '녕' as u16
                };
            }
            let before = editor.text();
            let out = editor.press_known(fixture.handle, 3, '\0');
            assert!(!out.contains("\u{1}B"), "{out:?}");
            assert_eq!(editor.text(), before);
        }
    }

    #[test]
    fn unknown_commit_origin_cannot_be_recovered_from_a_matching_suffix() {
        let fixture = Fixture::new();
        let mut editor = Editor::new("prefix ", 0);
        editor.press_known(fixture.handle, 0, 'd');
        for ch in "kssud".chars() {
            editor.press_unlocated_context(fixture.handle, 0, ch);
        }
        let space = editor.press_unlocated_context(fixture.handle, 1, ' ');
        assert!(space.starts_with('0'), "{space:?}");
        let committed = editor.text();
        assert_eq!(committed, "prefix 안녕 ");

        let escape = editor.press_known(fixture.handle, 3, '\0');
        assert!(!escape.contains("\u{1}B"), "{escape:?}");
        assert_eq!(editor.text(), committed);
    }

    #[test]
    fn activating_picks_up_changed_settings_with_deterministic_timestamps() {
        let dir = TempDir::new();
        let settings = dir.path().join("settings.json");
        std::fs::write(
            &settings,
            br#"{"languages":["en","ko"],"returnConfirmsOnly":true}"#,
        )
        .unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&settings)
            .unwrap();
        file.set_modified(SystemTime::UNIX_EPOCH).unwrap();
        drop(file);
        let fixture = Fixture::from_dir(dir);
        let confirms = |fixture: &Fixture| {
            ts_activate(fixture.handle, std::ptr::null());
            let mut editor = Editor::new("", 0);
            editor.type_known(fixture.handle, "hello");
            editor.press_known(fixture.handle, 5, '\0').starts_with('1')
        };
        assert!(confirms(&fixture));

        std::fs::write(
            &settings,
            br#"{"languages":["en","ko"],"returnConfirmsOnly":false}"#,
        )
        .unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&settings)
            .unwrap();
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(2))
            .unwrap();
        assert!(!confirms(&fixture));
    }
}
