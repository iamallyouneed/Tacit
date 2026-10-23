# Changelog

What changed in each release. Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/). When a release is made, that version's entry becomes
the GitHub release notes and is shown as-is in the dashboard's update tab. How to write entries:
[CONTRIBUTING.md](CONTRIBUTING.md#changelog-and-releases).

Korean version: [CHANGELOG.md](CHANGELOG.md).

## [Unreleased]

## [0.10.9] - 2026-10-23

### Added

- Android keyboard tools for pasting the current clipboard, moving the cursor, choosing an input mode, and controlling haptics and key previews. Clipboard history is not stored, and sensitive content is hidden from previews.

### Changed

- Refined Android keyboard light/dark surfaces and press feedback with the Tacit logo and Lucide icons. The selected reading appears first with a clear highlight; double-tapping Shift locks capitals, and Enter displays its current confirmation, search, send, or other action.
- Plain confirmation now keeps unfinished readings in the language already shown, including Enter, shortcuts and session flushes. Standalone letters are classified before display, so English suffixes no longer depend on confirmation to fix a Korean preview. Japanese confirmation keeps the displayed segmentation and live candidate, completing only a pending Japanese `n` to `ん`; explicit language changes, Space conversion, Tab completion and newly typed digit context remain separate actions.

### Fixed

- Windows release updates and removal no longer fail before the administrator prompt because the encoded installer exceeds the Windows command-line limit. The elevated script is encoded once, preserving release verification and the existing local installation path.
- Overlapping Shift input such as `akfgkfRK` now stays Korean (`말할까`) when supported by existing vocabulary or inflections; otherwise, case-sensitive Latin identifiers are preserved. The Korean exception no longer admits Japanese candidates. Unlisted Korean spellings with an overlapping Shift key now keep their Latin keys until explicitly corrected; normal Korean key spelling remains available.
- Generalized correction preferences now apply only to the actual runner-up language. This prevents a Korean preference from turning English into Japanese or broken Hangul when Japanese is enabled, while preserving explicitly learned words.
- Confirmation with Space or Enter keeps the language of a completed composition. This fixes `연습햇` becoming `연습got` and Japanese previews becoming English or Korean, while preserving explicit language cycling and Japanese candidate conversion.

## [0.10.8] - 2026-10-22

### Added

- The dashboard can copy a diagnostic report containing the version, OS, input method, Windows default and per-window switching settings, foreground app rules, and language-pack status. Typed text, learned words, user paths and authentication tokens are excluded. A selectable report is available when clipboard access is blocked.

### Changed

- CI and releases share Android emulator checks for JNI and cursor movement. Releases also require Windows installer, update and recovery checks, and macOS composition tests before publication.
- Changelogs are checked for matching Korean/English versions and dates, valid section titles, nonempty entries and broken characters before publication. Missing release notes stop publication. The historical English 0.5.1 date was corrected.

### Fixed

- Correction learning and autocomplete history hold a file lock from reading through saving, so simultaneous app processes no longer overwrite each other's changes. Dashboard learning deletion uses the same locks.
- Windows installation prepares all new files inside the administrator-owned install folder before replacement. A failed file replacement or DLL registration restores the previous files and input-method registration.
- Windows installation now sets Tacit as the default input method so new apps and sign-ins start with Tacit instead of the system input method. Updates preserve an existing Tacit default profile, including its autocomplete choice. Uninstallation clears the default only if it points to Tacit. The default can be changed in Windows Advanced keyboard settings.

## [0.10.7] - 2026-10-21

### Fixed

- Fixed Android input continuing at the old composing position after tapping to move the cursor or selecting text. Existing text is preserved and typing starts at the new cursor position.

## [0.10.6] - 2026-10-21

### Fixed

- Mending Hangul after Backspace no longer flips it to Latin while typing (`사랑`, delete, `ㅏ`). After the app deletes committed text, the character left before the caret sets the next word's language context.
- The Android keyboard no longer redraws every key and rebuilds the readings strip on each keystroke when nothing changed.

## [0.10.5] - 2026-10-21

### Added

- The Android keyboard's mode key now cycles `tacit` → `한` → `영`. `한` reads every word as Korean; `영` steps the input method aside and types plain English. The key faces follow the mode (jamo or letters), and Shift in `한` gives the double consonants. The mode starts as `tacit` in each text field.
- A readings strip above the Android keyboard shows how the word being typed can be read (for example `hello` / `ㅗ디ㅣㅐ`); tap one to switch. It is learned the same way as Esc. The strip replaces the `가/A` key, which is gone.

### Changed

- The Android keyboard has a new look: rounded keys, light and dark palettes, 56dp key height, and the jamo hints drawn inside the key corner. The hint positioning is adapted from HeliBoard (Apache-2.0 AND GPL-3.0-only); the source and licenses are in `platforms/android/THIRD_PARTY_NOTICES.md`.

### Fixed

- Updates from pre-rename versions find the new release assets and install both the program and language packs on Windows through the pinned release installer. Official update URLs also use the canonical repository case.
- macOS input-source lists and switching UI display `Tacit` instead of the internal identifier. Input-source identifiers remain unchanged.
- Fix macOS composition being reset by unavailable or inconsistent editor context, finalize marked text on session termination, and retain validated Esc correction after context recovery without changing surrounding text.

## [0.10.4] - 2026-10-19

### Changed

- The English/Korean edition now lists Japanese in the first-run page and the dashboard Languages tab. Choosing it opens the release's Japanese-edition installer (Windows, one elevation prompt), which installs English, Korean and Japanese together.

## [0.10.3] - 2026-10-19

### Changed

- Releases are split into an English/Korean edition and an edition with Japanese. The default `tacit-<os>.zip` supports English and Korean only; the Japanese implementation and its settings UI ship only in `tacit-<os>-ja.zip`. The English/Korean edition never enables Japanese, even if an old setting or a Japanese pack is left behind. Local builds on the same PC show the x64 DLL about 21% and the installer zip about 10% smaller. The language pack format and compatibility range are unchanged, so 0.10.x packs keep working. An installation that already had Japanese enabled stays on the Japanese edition through the install script; to choose it up front use `-Japanese` / `-Languages en,ko,ja` on Windows or `--languages en,ko,ja` on Unix.
- English trigram scoring no longer allocates. Decisions are identical.
- Japanese clause commits preserve the chosen dictionary path's right context for the remaining conversion, including clause resizing and editing after Esc. This reuses dictionary connection costs without word-specific exceptions or new weights.
- Japanese search reuses equivalent part-of-speech connection calculations. Whole-reading candidates and their order remain identical on 506 external readings; seven interleaved runs on the same PC reduce mean latency by about 5% and p95 by about 12%.
- Live Japanese conversion stops after the first complete A* path and skips unused clause calculations. Across 7,998 external reading prefixes, seven alternating full-list/preview runs retain identical top-1 output and reduce mean and p95 search latency by about 58%. Learned choices retain their path metadata; numeric reordering still uses the full list.
- Performance reports and lab baselines are separate for English/Korean (`enko`) and English/Korean/Japanese (`enkoja`). Common, scenario and learning evaluations use the selected languages; Japanese metrics are not averaged into English/Korean accuracy. `lab/README.md` documents commands and measurement scope.

### Fixed

- Japanese learning locks the read/merge/save transaction across processes and reloads on focus, preserving choices from concurrent apps. Read-only profiles can still read saved choices. Building requires Rust 1.89 or later.
- macOS/Linux input sessions reload changed settings, app rules and learned word files on activation.
- Performance reports calculate the median correctly for an even number of runs, with both report profiles and all lab tests checked in CI.

## [0.10.2] - 2026-10-18

### Changed

- Renamed **Typescript** to **Tacit**. Executables, release packages, settings paths, environment variables and GitHub URLs now use `tacit`; the tutorial, dashboard, input method names and documentation use the same display name.
- Settings and learned data are not migrated automatically. Uninstall with the original installer, install Tacit, then copy the contents of each old `Typescript` user folder into the sibling `tacit` folder. Android installs as a separate app because its package ID changed; select the new keyboard in system settings. The language pack format is unchanged and remains compatible with 0.10.x packs.

## [0.10.1] - 2026-10-18

### Changed

- Japanese conversion is faster. Interleaved runs of the same input against 0.10.0 give a mean latency of 0.54x and a p99 of 0.51x, with byte-identical output on 506 readings. The candidate search no longer builds and hashes a string per path (it continues an incremental hash), and lattice nodes share one text buffer instead of allocating a string each.
- Loading the Japanese dictionary no longer walks all 15 MB again: installed packs are already checked once per file. First initialisation drops from about 45 ms to about 4 ms and the heap after it from 5.8 MB to 3.0 MB. The dictionary format is unchanged, so 0.10.0 packs keep working.

## [0.10.0] - 2026-10-18

### Changed

- Japanese kanji conversion absorbs Mozc OSS vocabulary, IME connection costs, and official auxiliary dictionary/filter preprocessing into the existing Rust lattice. No Mozc process or C++ runtime is required. The Japanese dictionary grows from about 9.7 MB to 38.1 MB.
- The Japanese pack format now supports 12-bit context IDs, more homophones and supplementary-plane kanji. This is why the minor version is now 0.10.0; update the program and language packs together.

### Added

- Japanese conversion, language detection and English preservation checks in the lab, plus revision/hash-pinned AJIMEE (200 sentences) and Mozc regression evaluations. CI enforces Japanese quality floors.

## [0.9.1] - 2026-10-18

### Changed

- The welcome guide's Esc screen no longer depends on the engine getting a word wrong. If `깨짐` (`Rowla`) comes out right, Esc shows the other reading and one more press brings it back; if it stayed English, Esc fixes it as before. The three rounds are now one.
- 142 one-syllable words that informal Korean says (`걍`, `캬`, `잭` ...) are known words now. `korean_words.txt` comes from translated sentences and had no spoken Korean, so `rid` stayed English instead of becoming `걍`. They come from the test half of NSMC (CC0), `tools/colloquial_words.py` makes them again, and a word is left out when an English word on the same keys is much more common (`do` does not become `애`). About 130-180 points on the movie-review sets.
- The bias that follows the word before is now the same in total whatever the length of the word. The score is an average per key, so a longer word got a bigger total: a long Korean name that no list knows (`나카바루초`, `호즈미촌`) stayed English after English, and a long English word (`aortosclerosis`) was read as Korean after Korean. Korean titles after English go from 98.1% to 99.4% (about 220-260 points); short tokens like the `sh` of `chmod +x run.sh` do not change. In return, the English words that flash Korean while typed after Korean go up by 1.4 points.
- The preview flashes Korean less while an English word is typed. After a word that is not Korean, one syllable is not a word yet (the `wo` of `world` turning into `재`): 12.7% → 5.4% of English words flicker after English, 24.7% → 20.1% with no text before. In return, with no Korean before it, the preview of a one-syllable Korean word can show English until the word is committed.
- The welcome guide's language screen no longer has a "skip the tutorial" button: pick a language and press Start to go on.

## [0.9.0] - 2026-10-17

### Changed
- **The pack–program compatibility range moves to 0.9.** The Korean pack gains two files (`korean_syllable_counts.txt`, `korean_word_filter.bin`). This program rejects 0.8 packs and 0.8 programs reject 0.9 packs. Update the packs together in the settings tool.
- English and Korean typed with no space are split **over the whole word lattice at once**. Instead of comparing every possible cut with one score, a Viterbi search carries the best path forward at each letter position. Long sentences with English and Korean run together split correctly. The sentence test (`chars.sentences`) went from 39.21% to 99.82%, everyday sentences `acc.ALL` from 99.68% to 99.79%, sentences with particles from 97.31% to 98.60%, number sentences from 98.81% to 99.17%, long Korean sentences from 97.00% to 97.80%. No metric dropped.
- A keystroke is faster: the average is about 0.89× the previous release and the slowest 1% (p99) about 0.84× (relative values, measured against the previous version interleaved on the same computer). A stretch of letters that is no word is ruled out by a Bloom filter of the words carried in the pack and never searched.
- Korean syllable frequencies come from a large corpus, and a syllable not in the dictionary shares only the mass of the syllables seen once.

### Added
- The endings `므로` and `으므로` are recognised.

### Fixed
- `B급의` and `3D방식` (a label such as one letter or digits+letters, followed by a noun and a particle) split correctly.
- A word that is in no dictionary is no longer read as the noun of a label (`brb` → `b규`).
- A one-letter English word (`a`) is less often a bridge between Korean syllables (`만들a고`).
- A function word (`The`) is no host for a case particle: `The는` no longer turns into `쏟는`.

### Packaging
- The release package no longer holds `tacit.exe` (the development CLI). Built together with the CLI, the packs it embeds ended up in the other binaries too, 2.5 MB each.

## [0.8.8] - 2026-10-16

### Added
- Converting Japanese to kanji now shows a **candidate list window**: the selected line is highlighted, 1–9 pick from the current page, lists past nine candidates page as `1/2`, and the list flips above the word when it would run off the bottom of the screen.
- While the candidate list is open, Space, the arrow keys and the digits act on it: before, those keys leaked to the app and typed a space or a digit instead of moving through or confirming.

## [0.8.7] - 2026-10-16

### Added
- English and Korean typed with no space are split: `reportwha` → `report좀`, `kakaotalkaptlswj` → `kakaotalk메신저`, `KakaoTalk메신저`. The English word need not be in the dictionary and the Korean may be one syllable.
- Words right after a digit are read better: counters such as `2024년`, `3개`, `5분` come out Korean, units such as `1.5kg이다`, `10gb`, `100ms` come out English.
- A camelCase identifier followed by a particle (`isValid가`, `getUserName을`) is split as English.
- About 60 developer tool names the dictionary lacked are recognised as English with a particle: `mongodb를`, `kubectl을`, `graphql을`, `timeout이`, `postgres로`, `sql을`. `ssh` is English with Japanese on too.
- A whole sentence typed with no space is split too. The limit went from 32 to 96 keys, so a sentence that mixes English and Korean several times, like `확인dkssudhello바라…`, is read piece by piece.
- Esc also teaches the **kind** of word you corrected. After you correct a few different short consonant abbreviations, such as `ss` to `ㄴㄴ`, similar ones like `ac` → `ㅁㅊ` come out right without correcting each. Only English/Korean calls that were close to a coin flip change; English it was sure of stays.

### Changed
- `heh`, `meh`, `yeh`, `tsk` are listed as English words. They also read as a letter + `도`/`나`, and with splitting now scored, being in the dictionary is what keeps them English.
- English + Korean splitting is now **one score** instead of several rules for particular cases. Every possible cut is compared on one scale and the best wins; the weights are fitted to data by `lab --tune`. Developer terms the dictionary lacks followed by a particle (`vite한테`, `rebase한테`) go from 95.95% to 98.03% right, and English words followed by a particle in sentences from 96.76% to 98.27%.
- The live preview jitters less: `pre야` no longer flashes while you type `predisposable`.
- Deciding one word is faster (`commit` p99 165 → about 60 µs, keystroke p99 120 → about 90 µs), and the input method uses about 26 KiB less memory.
- When a whole text is converted at once, a word that was close to a coin flip is decided again in the language of a sure word after it on the same line. Chat-style sentences go from 82.35% to 88.24% right and all sentences from 97.74% to 98.12%, with nothing lower. The preview while you type is not affected.

### Fixed
- Two very rare English words are read as mixed with Korean: `armscye` becomes `a근쵿`, and `nihongo` becomes `nihon해` without the Japanese pack. Esc turns them back to English, and correcting twice teaches them.
- `nodejs의` no longer comes out as `nod던의`: a two-syllable tail that is neither a word nor an ending now costs a penalty. English + particle sentences go from 96.73% to 97.31% right.

## [0.8.6] - 2026-10-16

### Added
- An English word you teach once also teaches the other forms with a particle or ending: correct `kubectl을` once and `kubectl하고`, `kubectl로` come out right without correcting each. Korean words work the same way.
- Better results for a single letter (or capital acronym) followed by a Korean word (`n제`, `x축`, `PR은`, `QR코드`) and for an English word followed by a particle (`slack에서`).
- `cd` now comes out as English instead of the Hangul letters `ㅊㅇ`. File extensions and commands such as `rs`, `rb`, `gz`, `cmd`, `env`, `htm`, `cjs` are treated as English too.

### Changed
- The English pack drops an unused frequency column (about 9% smaller) and the engines share one Korean syllable table. Memory right after loading the packs goes from 1006 KiB to 67 KiB, and peak memory from 9.3 to 8.0 MiB.
- The core engine is built for size (`opt-level = "s"`): the input method DLL is about 12% smaller, and deciding one word is about 3% slower (about 20 microseconds on average).
- New `docs/report.md`: every release measures size, accuracy, speed and memory the same way and adds the result there.

### Fixed
- A lone jamo at the edge of a long key sequence (`cmdr`) being read as Korean.
- English with capitals such as `GDP` being broken by the single-letter rule.

## [0.8.5] - 2026-10-15

### Fixed
- Typing over selected text replaces it from the first key: in Win+R or File Explorer's F2 rename (text preselected), the word used to be appended after the selection and replaced it only on commit.

## [0.8.4] - 2026-10-15

### Fixed
- The dashboard chat test did not use learned words. It now reads the same `lexicon.json` as the input method, so what works in other apps works in the chat test too.

### Changed
- The taught-words list on the language tab shows what each word turns into (`Rowla → 깨짐`) instead of the language name, sorted a–z regardless of case.

## [0.8.3] - 2026-10-15

### Added
- The dashboard's update tab has **Delete**. It removes the input method and the program, and everything that was learned, the settings and the language packs, from this computer. One press never deletes: it asks once more and then the button has to be **held down for 1.5 seconds** (it fills while you hold it, and letting go or moving off it starts over). An installed Windows program is removed by the installer's `-Uninstall`, so an administrator prompt appears once, and the settings and learned words are removed **only after that worked** (declining the prompt deletes nothing). Opened from somewhere that is not an installed program (a build folder, a test folder) it leaves the program alone and removes only the data.

## [0.8.2] - 2026-10-15

### Changed
- Welcome guide (tutorial): the first two screens are one. Under "안녕하세요", in small grey text, it asks you to type `안녕 Tacit`, and when you have, the same screen moves on to "Is the Han/Eng key really needed?".
- The welcome guide's Esc screen is now three rounds. Type `깨짐` (`Rowla` on an English layout), fix it with Esc, then "Could you do it once more?" and "One last time". The program learns a word the second time it is corrected the same way, so the last round comes out as `깨짐` without Esc, and then it tells you "words you fix twice like this are remembered". If it has not been learned yet it does not say so and simply moves on. A correction is recorded only once the word is confirmed (Space), so each round tells you to press Space.
- The welcome guide's last screen wording was polished, and the progress bar segments are lit or off, never half.

### Fixed
- Choosing "switch the word's language" or "do nothing" on the Keys tab showed the text `null` under the cards.
- In the welcome guide a long sentence in the title place (for example "words you fix twice like this are remembered") was broken across two lines.

## [0.8.1] - 2026-10-15

### Changed
- Keys tab: the shortcut the Han/Eng key sends is designated in two slots. The left slot takes the Ctrl, Alt, Shift and Win buttons; the right slot is filled by pressing one key or picking a common shortcut. A combination that locks the screen when pressed together, such as Win+L, is now designated safely as the Win button plus a single L, and nothing is saved until you press Designate. Tab navigation and typing in the chat keep working while the panel is open, and leaving the tab closes the panel. Ctrl+S and Alt+Tab were added to the common shortcuts.
- The dashboard's right-hand panel is now called "Test" instead of "Try it".
- Update tab: the description paragraph in front of a version (for example why 0.8.0 raised the minor version) is shown with its records.

### Fixed
- The update tab showing no records when the request for them finished while another tab was in front.

## [0.8.0] - 2026-10-15

This version is **0.8.0**, not 0.7.1, because the pack file format changed. Packs and the program are only compatible within the same minor version (0.7.x packs are for 0.7.x programs), and the new Japanese pack is in a format the older program (0.7.x) cannot read, so the compatibility range is split here. A 0.7.x program sees this release's pack list as incompatible, does not download it, and says "update Tacit first". Updating installs the program and the Japanese pack together.

### Changed
- The Japanese pack got smaller: 17.2 MB → 10.1 MB installed, and the download about 5.9 MB → 4.9 MB. The tables are stored again at their natural width, so detection and conversion results are the same as before. (This is the change that alters the pack format: `kana_trigrams.txt` became `kana_trigrams.bin` and the dictionary went from `TSIPDIC2` to `TSIPDIC3`.) Thanks to Ddongddi for the contribution.
- Release binaries are smaller. `tacit-config` is about 7% smaller (6.6 → 6.1 MB) and `tacit` about 2%; the input method DLL is unchanged.

### Added
- An English changelog ([CHANGELOG.en.md](CHANGELOG.en.md)).

### Fixed
- Files that could not be deleted on uninstall or reinstall because a running app held them (the DLLs in `C:\Program Files\tacit`) are now scheduled for deletion at the next restart. Before, the folder stayed behind after an uninstall and only a message said otherwise.

## [0.7.0] - 2026-10-14

### Added
- First-run tutorial (welcome page, `tacit-config welcome`): starts with "안녕하세요", shows the hassle of typing while hitting the Han/Eng key, then lets you try `dkssud Tacit` without it. Picking languages downloads and enables them (per-language progress and completion are shown), and practices switching input methods, Korean, and Esc one screen at a time. The Esc screen shows an intentionally wrong example (typing `깨짐` on an English layout leaves `Rowla`) and explains that undecided text can be changed with Esc. A progress bar sits on top, practice fields are free-typing with only the result checked. It opens only on a fresh install (never on update) and can be reopened from the dashboard's "Tutorial" link.
- `tacit-config setup --if-configured`: reinstalls the already-enabled languages (for updates). It fails on a fresh install so the install scripts can tell the two cases apart.
- Dashboard update tab: the changelog for every version in one place (since 0.1.0). Expand/collapse all, and records for versions newer than the current one are fetched from GitHub and shown on top.
- Dashboard: a "taught words" section on the language tab (delete one, clear all) and a "word history" section on the input-settings tab (remembered word count, clear). Both need two presses to clear.
- Dashboard: also detects the currently selected input method on macOS and Linux, shows a one-time setup checklist, and adapts the switch-key guidance to the system.
- Dashboard: opening an old address (expired token) shows a "please reopen" notice. Added `GET /api/health` (version, uptime, whether busy).

### Changed
- The language prompt during installation (checklist and ordering) moved to the welcome page's first screen. Passing `-Languages` or having already-configured languages (an update) still installs quietly. Dropped the `dialoguer` dependency for the terminal prompt, and a bare `tacit-config setup` opens the welcome page instead of asking in the terminal.
- Dashboard: server error responses carry a `code` (invalid, upstream, busy, …). The connectivity check slows to every 30 seconds while the window is hidden and backs off while it keeps failing.

### Fixed
- Language-pack downloads and the dashboard taking 8–25 seconds every time on some networks. GitHub's file servers have several addresses and one of them is unresponsive on those networks, so connecting to that address first meant waiting for the timeout. Connections now move to the next address after 3 seconds (one more try at 15 seconds if all fail), and the fetched pack list (`packs.json`) is remembered for 5 minutes instead of being fetched on every request. An English+Korean install dropped from about 57 seconds to about 5.
- Changelog entries without a subheading such as `## 0.1.0` and their continuation lines being dropped from the records.

## [0.6.5] - 2026-10-14

### Changed
- Security: password fields are passed straight to the app — nothing there is read, counted, or learned. Detected via input scope (password) on Windows, secure-input mode on macOS, and input purpose (password/PIN) on Linux (IBus). (Previously Android only.)
- Security: language packs and updates are fetched over https only. An `http://` address in `TACIT_RELEASE` is refused unless it is this machine (localhost). Folders can still be used as-is.
- Security: opening `tacit-config` from the terminal no longer always prints the address carrying the session token; it is shown only when the browser fails to open.
- Built-in rules for terminals, editors, and messengers. Terminals (Windows Terminal, cmd, PowerShell, …) use a strict conversion threshold, editors (VS Code, Cursor, JetBrains, …) pass Esc to the app, and messengers (Slack, Discord, Teams, Telegram, LINE, KakaoTalk) send on a single Enter. An entry in `apps.json` for that app wins. App rules gain `returnConfirmsOnly` (per-app Enter behavior).
- Security: `install.sh` refuses to install archives containing paths outside their folder (`../`, absolute paths).
- Damaged word-history and kanji-learning files are moved aside to `.unreadable` instead of being silently overwritten (same as the correction-learning file). All saves are atomic, including the update-check cache.
- Dashboard: fetching languages or updating no longer blocks settings changes, and asking for the same job twice is answered with 429. Update failures respond with 502 like checks do, and 429/503 carry `Retry-After`.
- Word history (`user_history.json`) is capped at 20,000 words. Past that, the least-typed words are dropped first.

### Fixed
- English+Korean readings only the engine finds (`xdp`→`x에`, `hellodnjfem`→`hello월드`) missing from the Esc cycle, so once you left them you could never get back.

## [0.6.4] - 2026-10-14

### Added
- Dashboard: a "changelog" view on the update tab showing this build's CHANGELOG, folded per version.
- Dashboard: when Enter only confirms in the chat, a "to send at once, change the setting" hint appears and takes you to the "Enter only confirms" switch in input settings.

### Changed
- Dashboard: pressing the physical Esc key also presses the on-screen Esc keycap on the language tab.
- Dashboard: fixed the chat flickering on every keystroke; typing on a blank part of the page outside the inputs now goes to the chat. Underlines ride more softly when messages are posted.

### Fixed
- Typing a lone consonant jamo after Korean text (`d`→ㅇ, `z`→ㅋ) turning into the English letter `d`. A lone consonant right after Korean now stays the jamo. It can still be switched to the English letter with Esc.
- From that lone consonant, Esc cycled Korean → Japanese → English and never came back to Korean.

## [0.6.3] - 2026-10-14

### Added
- Dashboard: a banner when the currently selected input method is not Tacit, naming it and telling you to switch with Win+Space. (Windows)

### Changed
- Dashboard: trimmed the "connected" indicator at the top down to a small dot and text.

## [0.6.2] - 2026-10-14

### Changed
- Dashboard "Keys" tab: when the Han/Eng key is set to send a shortcut, it says it works "everywhere". Leaves room below and links to KeyZen for more key mappings.

### Fixed
- Windows: the elevated half of the install ran a file from a user-writable folder. The elevated side now fetches the GitHub release itself, verifies its SHA-256, and installs straight from memory. Running `install.ps1` from an unpacked folder installs the latest release (or `-Version`) instead of that folder.
- Damaged or half-copied Japanese packs (dictionary, trigram tables) are refused at registration. They used to panic on first conversion. A file that passes once leaves a marker so other apps don't re-scan the 16 MB dictionary.

## [0.6.1] - 2026-10-14

### Added
- Han/Eng key sends a shortcut: set it from the dashboard "Keys" tab via `Han/Eng → ?`. Switch the word's language (default), do nothing, or send a shortcut such as `Ctrl+C` or `Win+L`. Pick shortcuts from the list or by pressing the key, with Win chosen by button. (Windows, `hangulAction`)
- Hovering a chat message shows each word's read language and confidence as an underline (color is the language, length is the confidence). Hovering a word shows how it was read and the runner-up.

### Changed
- Dashboard: removed the try-out tab; every tab has the chat on its right so anything can be tried immediately. "Enter only confirms" is changed in input settings.
- Dashboard "Keys" tab is a single `Han/Eng → ?` line. The language-switch keys, pause keys, and which key is the Han/Eng key are changed only in `settings.json`.
- Removed the `English → 한국어` order chips from the language tab. The order is changed with each language's ↑↓.

### Fixed
- Dashboard: typing digits in the input-settings tab's chat input switched tabs.
- Windows: shortcuts assigned to the Han/Eng key (`Win+D` and the like) worked only inside text fields. They now work everywhere, including the desktop, the taskbar, and already-running apps. Same for laptops where the Han/Eng key arrives as right Alt. Installing starts a key watcher at sign-in (`rundll32 tacit_tip.dll,KeyHook`). It does not act in front of windows run as administrator.

## [0.6.0] - 2026-10-14

### Added
- **Keys tab**: the dashboard draws a keyboard to pick keys with. It sets which key is the Han/Eng key (`hangulKey`, default `Han/Eng`), which keys switch the word's language (`flipKeys`, default `Esc` and the Han/Eng key), and which pause (`pauseKeys`, default `Alt`+Han/Eng). Changing the Han/Eng key follows in the other two's "Han/Eng key". `Ctrl+Space` combinations and lone `Ctrl` keys work, lists grow to 8 entries. Win combinations, common shortcuts like `Ctrl+C`, and keys claimed by both actions are refused. (Windows)

### Changed
- The default pause key changed from `Han/Eng` to `Alt+Han/Eng`, and a lone `Han/Eng` switches the word's language like `Esc`. An old `pauseKey` setting is migrated to `pauseKeys` (`"off"` stays off).

### Fixed
- Windows: in input fields that talk IMM32, such as File Explorer's rename box, the space or period after a word landed before the word. Keys that confirm a word are now delivered to the app after confirming.
- Windows: in fields with selected text, such as File Explorer's rename box, the old name stayed until the word was fully typed and only changed at the end. The selection is now cleared on the first keystroke.

## [0.5.2] - 2026-10-14

### Fixed
- Windows: typing over selected text changed nothing. In fields with a selection, such as File Explorer's rename box, the old text stayed and the typing was appended. The selection is now deleted first and typed in its place.
- Windows: with "Enter only confirms" off, after sending a message in KakaoTalk the next characters went to the placeholder ("Message") instead of the input. In IMM32 apps one Enter confirms the word, then Enter is sent after confirming.
- Running `scripts/install.ps1` from the source folder failed at the elevated step.

## [0.5.1] - 2026-10-14

### Added
- Docs: Korean and English docs under `docs/` (installation, configuration, architecture, language packs, troubleshooting) and the English README.
- Dashboard input settings gain **try in chat**: type on an English layout and check it the same way the real input method does.
  With "Enter only confirms" on it takes two Enters to send, with it off one — the bubble's `Enter ×N`
  shows it, and buttons compare the two behaviors. Tab completion and Backspace work too.
- When an update replaces the program, the settings page tells you to close it and reopen `tacit-config`.

### Fixed
- "Unusable pack: …needs core …" staying on the dashboard after a program update. When a newer version of the same pack already works, old version folders are no longer reported as problems, and installing a pack cleans up that pack's old version folders.
- The dashboard's try-out erroring when an enabled language's pack was missing or mismatched with this build. Like the input method, it runs without that language and shows a notice when no usable pack exists.
- A big outline wrapping the whole panel after keyboard navigation on the dashboard.

## [0.5.0] - 2026-10-14

### Added
- **macOS**: InputMethodKit input method (`tacit.app`, Apple silicon + Intel).
- **Linux**: IBus engine (`ibus-engine-tacit`).
- `install.sh`: one-line install/uninstall for macOS and Linux (current user only, no administrator rights, SHA-256 verified).
- `platforms/ffi`: the input method wrapped as C functions (`tacit.h`). The macOS and Linux input methods share it.
- Dashboard: clicking a word on the try-out tab shows and switches to how it reads in other languages.
- Dashboard: the app-rule conversion threshold becomes a slider with a live preview of how a sample sentence reads while it moves.
- Dashboard: drag the Esc order chips to reorder and press Esc to preview the cycle.
- Dashboard: short demos of the "Enter only confirms" and "Tab completion" settings.
- Dashboard: undo a change, shortcuts (`1`–`5` switch tabs, `/` finds an app).
- New logo and icons.
- `LICENSE` changed to AGPL-3.0 with `NOTICE`, `CONTRIBUTING.md`, and this changelog added.

### Changed
- Settings and language-pack folders are per system. Windows is unchanged; macOS uses `~/Library/Application Support/tacit`,
  Linux uses `~/.config/tacit` (packs under `~/.local/share/tacit/packs`).
- License: MIT → AGPL-3.0-only (releases up to v0.4.1 stay MIT).
- Dashboard sources split into markup, style, and script files under `crates/config/src/web/`.
- CI and releases build, test, and package on Linux and macOS in addition to Windows and Android.

### Fixed
- Accessibility attributes (`aria-pressed` and the like) going out as empty values.

## [0.4.1] - 2026-10-13

### Added
- Dashboard try-out tab: see right away how what you type on an English layout reads word by word.
- Per-setting examples with a "not the default" mark and a "reset to default".
- Per-app rule cards, presets (terminal, game, vim, KakaoTalk), running-app list (search, pages, drag and drop).
- Custom dropdowns, accessibility (keyboard, screen reader, high contrast) and narrow-screen support.

### Changed
- Dashboard restyled into a minimal flat design.

## [0.4.0] - 2026-10-13

### Added
- **Language packs**: language data ships as packs so only what is needed is fetched. Structure for adding languages as modules.
- `tacit-config`: terminal setup screen, command line, web dashboard.
- Update checks for new versions and language packs from releases.
- Pick languages at install time and fetch their packs.

### Changed
- The engine decides over any combination of enabled languages. The Esc cycle follows the enabled languages.

## [0.3.0] - 2026-10-13

### Added
- English/Korean and Japanese builds distributed separately (`install.ps1 -Japanese`).

### Fixed
- Katakana candidates for words missing from IPADIC, and `ー` after English words.

## [0.2.1] - 2026-10-13

### Added
- **Japanese**: romaji → kana decision, conversion flow, full-width handling, IPADIC-based kanji conversion.

### Changed
- First APK signed with the fixed release key.

## [0.2.0] - 2026-10-13

### Added
- **Android**: `InputMethodService` app and on-screen keyboard.

### Changed
- The core is shared by `platforms/windows` and `platforms/android`.

## [0.1.1] - 2026-10-12

### Fixed
- The install script failing to find the repository path at the elevated step.

## 0.1.0

- First release: English/Korean auto-decision input method for Windows (TSF), Tab completion, per-app rules, KakaoTalk compatibility.

[Unreleased]: https://github.com/iamallyouneed/tacit/compare/v0.10.9...HEAD
[0.10.9]: https://github.com/iamallyouneed/tacit/compare/v0.10.8...v0.10.9
[0.10.8]: https://github.com/iamallyouneed/tacit/compare/v0.10.7...v0.10.8
[0.10.7]: https://github.com/iamallyouneed/tacit/compare/v0.10.6...v0.10.7
[0.10.6]: https://github.com/iamallyouneed/tacit/compare/v0.10.5...v0.10.6
[0.10.5]: https://github.com/iamallyouneed/tacit/compare/v0.10.4...v0.10.5
[0.10.4]: https://github.com/iamallyouneed/tacit/compare/v0.10.3...v0.10.4
[0.10.3]: https://github.com/iamallyouneed/tacit/compare/v0.10.2...v0.10.3
[0.10.2]: https://github.com/iamallyouneed/tacit/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/iamallyouneed/tacit/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/iamallyouneed/tacit/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/iamallyouneed/tacit/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/iamallyouneed/tacit/compare/v0.8.8...v0.9.0
[0.8.8]: https://github.com/iamallyouneed/tacit/compare/v0.8.7...v0.8.8
[0.8.7]: https://github.com/iamallyouneed/tacit/compare/v0.8.6...v0.8.7
[0.8.6]: https://github.com/iamallyouneed/tacit/compare/v0.8.5...v0.8.6
[0.8.5]: https://github.com/iamallyouneed/tacit/compare/v0.8.4...v0.8.5
[0.8.4]: https://github.com/iamallyouneed/tacit/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/iamallyouneed/tacit/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/iamallyouneed/tacit/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/iamallyouneed/tacit/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/iamallyouneed/tacit/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/iamallyouneed/tacit/compare/v0.6.5...v0.7.0
[0.6.5]: https://github.com/iamallyouneed/tacit/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/iamallyouneed/tacit/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/iamallyouneed/tacit/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/iamallyouneed/tacit/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/iamallyouneed/tacit/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/iamallyouneed/tacit/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/iamallyouneed/tacit/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/iamallyouneed/tacit/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/iamallyouneed/tacit/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/iamallyouneed/tacit/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/iamallyouneed/tacit/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/iamallyouneed/tacit/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/iamallyouneed/tacit/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/iamallyouneed/tacit/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/iamallyouneed/tacit/releases/tag/v0.1.1
