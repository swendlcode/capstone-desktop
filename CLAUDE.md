# Capstone — Project Context

Desktop batch audio corrector. Offline, local, no account. A DJ or producer points
Capstone at a folder of finished tracks and gets them back at one consistent
loudness, with clipped peaks repaired, optional analogue-style warmth, optional
pitch correction, and the tags still on them.

The input is a **finished file**, not a session. Capstone is not a mastering suite
and not an editor: there is no transport, no waveform, nothing to audition. The
product is a table of files, a template that says what to do to them, and a Start
button.

## Stack

| Layer | Tech |
|---|---|
| Desktop shell | **Tauri 2** (`src-tauri/`) |
| UI | **React 18 + Vite + TypeScript (strict)** (`src/`) |
| Styling | **Tailwind CSS** + CSS vars for the design system |
| Icons | **iconsax-reactjs** 0.0.8 — *only* re-exported through `src/components/ui/icons.ts` |
| State | **Zustand** (client UI state) + **@tanstack/react-query** v5 (backend state) |
| Backend lang | **Rust 2021**, MSRV 1.87 (pinned in `Cargo.toml`) |
| Persistence | **Two JSON files** in the app data dir. No database |
| Decoding | **symphonia** (wav, aiff, flac, mp3, m4a/aac, alac, caf, ogg/vorbis/opus) |
| Encoding | **hound** (wav), **aifc** (aiff), **flacenc** (flac), **alac-encoder** + own CAF muxer, **rusty_mp3** (mp3, behind a feature) |
| Loudness | **ebur128** (R128 integrated + true peak) |
| Resampling | **rubato** 5 |
| Pitch | **signalsmith-stretch** (keep-tempo), rubato (change-tempo) |
| FFT | **rustfft** (tuning estimator, test measurement) |
| Parallelism | **rayon**, over files — never inside one file |
| Logging | **tracing** + `tracing-subscriber` with `EnvFilter` |

Every dependency is MIT, Apache-2.0 or BSD. That is a hard constraint, not a
preference — see **Licensing is load-bearing** below.

## Dev commands

```bash
. "$HOME/.cargo/env"        # one-time per shell if cargo isn't on PATH
npm install                 # first time only
npm run tauri dev           # dev with HMR + Rust hot rebuild

cd src-tauri && cargo test --all-features   # 124 unit + 25 end-to-end
npm run build                               # tsc -b && vite build
```

`cargo test` takes about half a minute because the end-to-end suite synthesises
real audio files and runs the whole pipeline over them. If it ever takes ten
minutes, the dev profile's `opt-level = 2` has been lost — see the comment on
`[profile.dev]` in `Cargo.toml`, which explains why optimisation is on in a debug
build.

## Repository layout

```
src/                      React frontend
  App.tsx                 Shell: title bar, tab router, one queue listener
  main.tsx                Theme-before-paint, QueryClient, StrictMode entry
  index.css               Design tokens (dark + light) + Tailwind layers
  components/
    ui/                   Primitives — Button, Input, Checkbox, Radio, Modal,
                          ContextMenu, CapstoneMark, icons.ts
    layout/               TitleBar (drag region + traffic-light inset), TabBar,
                          UpdateBadge, StatusBar (the mark lives here, not in the
                          title bar: beside the traffic lights it crowded the tabs
                          and read as chrome rather than as a byline)
    process/              QueueTable (virtualized), QueueRow, CoverCell, MetricCell,
                          StatusCell, QueueToolbar, ProcessFooter, DropTarget,
                          rowMenu + useRowActions (right-click), columns.ts, format.ts
    template/             TemplatePicker + TemplateMenu, TemplateShare, the four
                          setting cards, MultibandEditor, SliderField / NumberField,
                          useTemplateEditor, useTemplateShare
    personalize/          PersonalizeRail + the three panes
    onboarding/           First-run tour (four steps in the shared Modal) and the
                          title-bar action that reopens it
  pages/                  ProcessPage, TemplatePage, PersonalizePage
  hooks/                  useQueue (+ useQueueSync), useTemplates, useSettings,
                          useTheme, useCoverArt, useOnboarding, useAppUpdate
  services/               capstoneService — THE ONLY module that calls invoke()
  stores/                 queueStore, uiStore, settingsStore (Zustand)
  types/                  The TS half of the IPC contract; re-exported from index.ts
  lib/                    tauri-core / tauri-event transport shims (Vite-aliased)
  theme/                  color.ts — hex → RGB triplet helpers for the accent
  data/changelog.json     Release notes, keyed by version

src-tauri/
  Cargo.toml              Dev profile is optimised on purpose; `mp3` is a feature
  tauri.conf.json         Window chrome, asset protocol, bundle targets
  capabilities/           default.json — core/dialog/fs/opener/process
  src/
    main.rs               bin stub, calls capstone_lib::run()
    lib.rs                Module list + builder chain + the flat command list
    state.rs              AppState: queue, templates, settings, cancel/running flags
    error.rs              CapstoneError, serialized to a plain string for IPC
    models/               THE IPC CONTRACT. Every struct is camelCase
    settings.rs           settings.json load/save + defaults
    templates.rs          Load/save; templates/catalog.rs is the stock set (code,
                          not data), templates/share.rs the shared-file format
    cover.rs              Embedded artwork → one small PNG data URI, memoised per
                          path; the queue is the cache's only ceiling, so
                          `clear_queue` is what brings it down
    commands/             Thin Tauri handlers, one file per domain
    jobs/                 mod.rs (queue runner), analysis.rs (background), output.rs
                          (+ output_tests.rs: tests live beside, not inside, any
                          module close to the 300-line cap)
    audio/
      mod.rs              DecodedAudio (planar) + dB helpers
      decode.rs           symphonia probe/decode → planar f32, plus tag extraction
      analyze.rs          The report: LUFS, true peak, clips, crest, tuning
      analysis/           spectrum.rs (framing/FFT), tuning.rs (cents estimator)
      process/            THE CHAIN — see below
      encode.rs           Format dispatch + rate conversion + tag write
      encode/             wav, aiff, flac, mp3, alac, caf, dither, resample, tags
  tests/
    audio_e2e.rs          Entry point
    e2e/                  Eight claims, one module each
    support/              Fixture synthesis, independent measurement, pipeline harness
```

## Architectural rules (enforce in reviews)

- **Commands are thin.** No DSP and no business logic inside `#[tauri::command]`.
  They validate input, call `jobs` / `audio` / `templates` / `settings`, return.
- **All DSP lives in Rust, under `audio/`.** Nothing in `commands` or `jobs`
  reaches into a sample buffer. The frontend never computes a figure it could
  have been told.
- **React components never call `invoke()`.** Flow: component → hook → service →
  `invoke`. `services/capstoneService.ts` is the only file that imports `invoke`.
- **Zustand = client state, react-query = backend state.** The queue is backend
  state and lives in react-query; `queueStore` holds only the run's own UI state
  (progress, selection, last-run summary) and duplicates no `QueuedFile`.
- **Every IPC struct carries `#[serde(rename_all = "camelCase")]`.** Nothing
  checks this at compile time on either side: a missed rename surfaces as an
  `undefined` column in the table or a setting that silently forgets itself. It
  is spelled out on each type in `models/mod.rs` rather than assumed.
- **File size caps.** React components ≤200 lines, Rust modules ≤300 lines,
  functions ≤50 lines. Extract a submodule instead of letting a file grow —
  `limiter_envelope.rs` exists for exactly this reason, and the brickwall proofs
  moved with the functions they are about.
- **Icons only from `components/ui/icons.ts`.** A name that does not exist in
  iconsax-reactjs 0.0.8 imports as `undefined` with no TypeScript error and no
  build error; React then throws on render and the window is white. Verify any
  addition against `node_modules/iconsax-reactjs/dist/esm/index.js`. Known traps:
  `Music2`, `Grid`, `Terminal`, `Ruler2`, `Voice` are not in the package.
- **Async spawn in `setup`: `tauri::async_runtime::spawn`, not `tokio::spawn`.**
  The Tokio runtime is not live during `setup`. CPU-bound work then goes to
  `tokio::task::spawn_blocking` — both the analysis batch and the processing run
  do, because neither may stall the IPC bridge.
- **Comments explain why.** No comment restates the line under it.

### Licensing is load-bearing

Capstone ships as a permissive open-source project, so **no GPL or AGPL
dependency may be added**, however convenient. Three places in the codebase are
shaped by this and will look like the wrong choice without it:

- **MP3 sits behind a Cargo feature.** Every mature MP3 encoder binding is
  copyleft (`mp3lame-encoder` and `mp3lame-sys` are LGPL-3.0, `shine-rs` is
  LGPL-2.0). `rusty_mp3` is the only permissive option and it is young, so it is
  isolated behind `--features mp3` and `audio/encode/mp3.rs` is the only file
  that would change if it has to be swapped out.
- **ALAC has a hand-written container.** No permissive crate muxes ALAC frames
  into `.m4a` or `.caf`, so `audio/encode/caf.rs` is ours — a `desc` chunk, the
  magic cookie, a `pakt` table and the data. CoreAudio and FFmpeg both accept it.
- **Pitch shifting uses signalsmith-stretch, not rubberband.** The rubberband
  bindings are GPL. Do not reach for them.

## The IPC contract

`src-tauri/src/models/mod.rs` and `src/types/` are two halves of one thing. The
Rust side is the source of truth; the field names are frozen because the frontend
is built against exactly them.

| Type | Crosses as | Notes |
|---|---|---|
| `Analysis` | `analysis` on a row | What was measured **before** anything changed |
| `QueuedFile` | the table's row | `id`, `path`, `filename`, `analysis`, `status`, `result`, `error` |
| `FileStatus` | lowercase string | `pending`, `analyzing`, `analyzed`, `processing`, `done`, `error` |
| `ProcessResult` | `result` on a row | What processing actually **did**, which is not what was asked for |
| `Template` | the Configure tab | `serde(default)` at container level |
| `BandSetting` | one multiband band | Crossovers meet exactly; `high_hz` *is* the next band's `low_hz` |
| `Settings` | the Personalize tab | `serde(default)` at container level |
| `Warmth` | `none` / `gentle` / `tube` | lowercase |
| `PitchMode` | `keep_tempo` / `change_tempo` | **snake_case**, not camel |
| `OutputLocation` | `specific` / `same_as_source` | **snake_case**, not camel |

`Template` and `Settings` both carry a container-level `serde(default)`. That is
what lets a `templates.json` or `settings.json` written by an older build still
deserialize after a release adds a field, instead of the whole file failing to
parse and the user losing every template they made. Add fields; never rename or
repurpose one.

Errors cross as a plain string: `CapstoneError` has a hand-written `Serialize`
that emits `to_string()`, so `invoke` rejects with a sentence a human can read
rather than an enum shape the frontend would have to switch on.

### Commands

Registered in one flat `generate_handler!` in `lib.rs`. A missing line there is
the usual cause of a "command not found" at runtime, and a flat list is what
keeps that checkable at a glance.

- **queue**: `add_paths` (expands folders, filters to audio, starts analysis,
  returns only the new rows), `analyze_file`, `get_queue`, `remove_from_queue`,
  `clear_queue`
- **process**: `start_processing` (spawns and returns at once), `cancel_processing`
- **template**: `get_templates`, `save_template`, `duplicate_template`,
  `delete_template`, `export_template`, `import_template` (one template as a
  `.json` file anybody can post: no server, no account, the file *is* the
  feature). Both drive a native dialog from Rust and never hand the bytes to
  the webview. Import is the only door a stranger's values come through, so
  everything about trusting them lives in `templates::share::from_json` — fresh
  id, `isStock: false`, a name suffixed rather than overwriting one already
  there, and every field clamped to the range the Configure tab enforces.
- **settings**: `get_settings`, `update_settings`
- **dialog**: `open_output_folder`, `pick_files`, `pick_folder`

`save_template` and `update_settings` both return **what was stored, not what was
sent**, because the two differ: a zero core count is corrected, and a stock
template is refused. The frontend reseeds its cache from the reply rather than
from the edit, or its slider and the next run would disagree.

### Events (backend → frontend)

Listened once in `useQueueSync`, mounted in `App.tsx` rather than per page — the
queue keeps running while the user reads the other two tabs, and a listener that
unmounted with the table would miss every event in between.

- `capstone://file-updated` → one `QueuedFile`, on every status change
- `capstone://queue-progress` → `{ done, total }`
- `capstone://queue-finished` → `{ processed, failed }`

Nothing polls. A failed emit is logged and swallowed everywhere in `jobs`: the
window may simply have closed, and that is not a reason to abandon a run that is
still writing files.

## The DSP chain, and the order it runs in

`audio/process/mod.rs::process` runs the whole chain on a planar buffer and
returns a `ProcessResult`. **It never touches the filesystem** — `jobs` fills in
`output_path` once it has encoded the buffer. That separation is what lets the
entire chain be tested on a buffer with no disk involved.

The order is not a style choice:

1. **Measure** integrated LUFS, before anything.
2. **Clip repair** — first, while the damage is still where the encoder left it.
3. **Pitch correction** — before anything that measures or reshapes level.
4. **Multiband dynamics** — four bands, serial bank with allpass compensation.
5. **Warmth** — oversampled waveshaper, level-normalised in and out.
6. **Gain + limiter** — last, so the file lands on target after every stage that
   could have moved it.
7. **Measure again**, for the Volume column's "before → after".

### Why clip repair runs before limiting

What was clipped is gone. The job is to put a plausible curve back where the
encoder left a flat top, and the **flat top is the only evidence** the file was
ever clipped. Every stage after repair either filters or scales the signal, and
each one destroys that evidence: once the multiband has been through it, there is
nothing left to detect.

`clip_repair` identifies a run by flatness, not by length — the time a waveform
naturally spends above a ceiling depends on its frequency, so a full-scale 60 Hz
sine dwells near full scale for three or four samples at 44.1 kHz and a
length-only detector would call it clipped. Runs longer than 20 ms are left alone:
that is a square-wave synth or a sub-bass drop, and a cubic across a gap that long
is fiction.

Two things it is not. It does not remove the harmonic distortion already baked
into the file; it prevents *new* distortion downstream. And on a lossy source it
will correctly find nothing, because MP3 and AAC coding smears the flat top away
— such a file still clips, and gain and limiting handle it alone.

### Why reconstruction needs headroom

The repair deliberately reaches **above** full scale. A run that was 2 dB into the
clip comes back about 2 dB over, capped at +6 dB, past which the cubic is
extrapolating nonsense. That overshoot is the point: capping the repair at the
ceiling instead would flatten the transient again and defeat the whole stage.

`template.headroom_db` (1.5 dB by default) is what makes the overshoot safe, and
the chain order **repair → measure → gain → limit** is what spends it: the
limiter is the last stage and the only guarantee that nothing leaves above the
ceiling, so an interim buffer sitting over full scale is fine as long as nothing
writes it to disk in that state. Nothing does.

### Why there is oversampling around the saturator

A memoryless waveshaper generates harmonics at `N × f`, and at the native rate
those fold back around Nyquist. The audible failure is specific: the second
harmonic of a 15 kHz tone at 44.1 kHz lands at 30 kHz and aliases to 14.1 kHz, a
descending tone that moves the *wrong way* with the input. That is what makes an
un-oversampled "warmth" control sound like a broken codec rather than like tape.

`process/oversample.rs` is one polyphase stage per direction, not a cascade of
halfbands: at these factors a single stage is cheaper per output sample and far
easier to reason about, and this is an offline batch tool. The kernel is symmetric
and odd-length, so **the round trip delays by exactly `TAPS_PER_PHASE` base-rate
samples whatever the factor**. That matters: clip positions and loudness gating
blocks both assume the buffer stays sample-aligned, so the caller compensates with
an integer and never with a fraction.

The other half of warmth sounding like warmth is level normalisation. A fixed
shaper's distortion depends on how hard it is driven, so without it the same
setting does visibly different things to a −20 LUFS vinyl rip and a −6 LUFS
master. For a tool whose whole purpose is batch consistency that is a defect, so
the signal is scaled to sit at −18 dBFS going in and the scaling is undone coming
out.

### Why the limiter has a true-peak sidechain of its own

The dBTP figure the table shows comes from `ebur128`, which is conformance-tested
and has no business being hand-rolled. `process/truepeak.rs` exists for the one
job that crate cannot do: a streaming, sample-accurate detector the limiter can
read per sample. A sample-peak sidechain instead leaves 0.3 to 1 dB of
inter-sample overshoot in the written file, which becomes audible distortion after
any lossy transcode or on a cheap DAC's reconstruction filter.

The envelope is in `limiter_envelope.rs` and is **provably** overshoot-free: a
sliding-window minimum of the required gain over the look-ahead, then a triangular
smoothing window shorter than the look-ahead. Any non-negative unity-sum average
of values each at most `r` is itself at most `r`. The proof lives next to the four
functions it is about, which is why they are their own module.

With a −12.5 LUFS target and a −1.5 dBTP ceiling the limiter only engages once the
source's peak-to-loudness ratio passes 11 dB. Modern dance masters sit at 6 to
9 dB: they get turned down and the limiter never touches them, which is most of a
DJ library and has to come out clean. Vinyl rips, 80s and 90s CDs and classical sit
at 13 to 18 dB: they get turned up by 5 to 10 dB and need 2 to 7 dB of peak
limiting, which is where **Limiter Character** becomes audible.

Character is one dial, 0 to 10, over four correlated parameters, held in an
eleven-row table in `process/character.rs` rather than a fitted curve. The control
the UI sends is an integer, so eleven entries are exact, auditable, and re-tunable
by ear by editing eleven numbers. Low is fast and transparent and flattens
transients; high rides the level, glues, and pumps on sparse material.

### Why the gain stage iterates

The limiter measures loudness for itself, because the multiband and warmth stages
above it just moved the level by one to three decibels. After the first gain move
it re-measures and corrects up to three times, each pass giving back about three
quarters of what the one before it was short by — the gain a pass adds is itself
partly limited away. Tolerance is 0.05 dB, an order of magnitude under anything
audible and under the half-unit the product claims, and comfortably above the
meter's own repeatability so refinement cannot chase noise.

A file that lands on target at the first attempt — most of a DJ library — pays for
one extra reading and no extra passes.

### The silence floor

`SILENCE_LUFS = -70.0` is the loudness standard's own absolute gate, not an
invented number. Without the floor a file 80 dB down would ask for 67 dB of gain
and get **its own noise floor** normalised to dance-floor level. Claim 8 in the
end-to-end suite exists because that is a real bug with a real shape.

### Why the multiband sums flat

A serial bank where band *k* carries the allpass of every crossover above it. The
derivation is written out literally at the top of `process/multiband.rs`, because
a tree topology with hand-guessed compensation is the classic source of a notch at
every band edge, and a notch is something no single band's threshold or ratio can
explain to a user. The bands are not time-aligned with each other — each carries a
different number of phase rotations — and that is fine for dynamics: it is the sum
that has to be flat, and a test asserts it to 0.1 dB off a real impulse before
anything gets tuned by ear.

Biquad state is f64 throughout and that is not a preference. At a 120 Hz corner
and 44.1 kHz the coefficients cluster around 1 and −2, so the feedback path
subtracts two nearly equal numbers every sample; f32 state turns the residue into
quantisation noise sitting in the low band, where it is audible on a club system.

### Why pitch has two modes and a ±50 cent bound

Tuning is measured by folding every strong partial onto the equal-tempered grid
and taking the weighted centre of a one-cent histogram (`analysis/tuning.rs`). One
bent note cannot move it; a tape running fast moves all of it. The fold carries an
ambiguity no cleverness removes: a record 60 cents sharp is indistinguishable from
one 40 cents flat. So the answer is bounded to ±50 cents and always points the
shorter way, which is also the musically right call — 60 cents is a different key
rather than a mistuning.

- `keep_tempo` shifts pitch and holds the running time, through
  signalsmith-stretch. **Do not hand-roll a phase vocoder**: identity phase
  locking and transient phase reset are mandatory rather than optional, and
  without both a kick turns into a thud. That library has them.
- `change_tempo` resamples, the way nudging a turntable does. Pitch and tempo move
  together and the file gets longer or shorter — up to 2.9% at 50 cents, ten
  seconds on a six-minute track. A DJ with saved cue points will care, so the UI
  has to say so.

### Why 16-bit output is dithered and 24-bit is not

Rounding to 16 bits without dither is an audible mistake, and it is audible in
exactly the material a DJ library is full of: a long reverb tail or a quiet intro
turns into quantisation distortion that *tracks the signal*, which the ear hears
as a gritty edge rather than as noise. TPDF dither trades that for a steady,
signal-independent noise floor about 4.8 dB above the undithered one, which is the
right trade every time.

24-bit is not dithered. Its own noise floor sits near −144 dBFS, far below the
noise already on any real recording, so dither there would add noise to solve
nothing.

The generator is seeded with a constant, so the same input produces the same
output bytes on every run. That is what makes the round-trip and idempotence tests
meaningful, and nobody can hear the difference between one dither seed and
another. The full-scale constants are the positive maxima (32767, not 32768):
scaling by 32768 and rounding lets a sample at exactly ±1.0 land one step outside
the range, and a wrap there is a loud click rather than a rounding error.

## Template and settings model

Two JSON files in the app data dir. No database: there is exactly one `Settings`
and a handful of templates, and both are small enough that a torn write is cheaper
to recover from than a schema is to maintain.

**Neither load function can fail.** A missing, unreadable or malformed file means
the defaults, logged at `warn`. Refusing to start because `settings.json` has a
stray comma would be worse than losing a preference.

### Templates

A template is the whole processing recipe: target loudness, limiter character,
clip repair and its threshold, headroom, warmth, pitch mode and tolerance, and the
band list. One ships — **Festival**, `-12.5 LUFS`, character 4, repair on above
−14 LUFS, 1.5 dB headroom, gentle warmth, pitch off, four bands.

**Stock templates are code, not data.** They are rebuilt on every launch and
merged over whatever is on disk, and `save` writes only what the user owns:

- A shipped tweak to Festival reaches existing installs. A stale copy of a stock
  id sitting on disk would shadow it, so `load` drops any stored template wearing
  a stock id.
- Storing stock templates as well would be a second source of truth, to drift.
- `save_template` refuses a stock template, and refuses an `is_stock: false`
  template wearing a stock id. The refusal lives in the command and not only in
  the UI, because the gear menu is not the only way the command can be reached and
  accepting it would look like a save that worked and then lose the user's
  settings on the next launch.

The UI makes a stock template editable by duplicating it. There is **no Save
button** anywhere on the Configure tab: every control commits through
`save_template`, and a stock template disables its inputs rather than taking edits
it would have to throw away.

**A card's hint is derived from the template, never typed as a constant.** The
Output Volume card carried the fixed line "-12.5 dB keeps the contrast between
loud and quiet parts." It was true of Festival and of nothing else shipped —
Club is -11, Stream -14, Vinyl Rip and Archive -16, and a user template is
whatever was typed — so for four of the five it stated a figure the field
directly beneath it contradicted. `volumeHint()` in `OutputVolumeCard.tsx`
reads `template.targetLufs`; anything else that quotes a setting does the same.

### Sharing a template

A template exports as one `.json` file and imports the same way: no server, no
account, no gallery. The file **is** the feature, and `templates/share.rs` owns
both halves of it.

Both commands drive a native dialog from Rust and write or read the file there.
The webview never sees the bytes: a template is the user's own settings and
there is nothing for the frontend to do with them but hand them straight back.

`formatVersion` is checked exactly, not compared as a range. A new field is
additive and arrives with a serde default, the same way `Template` tolerates
one; the version only moves for a change an older build genuinely could not
read, and then the refusal says which format the file is and which this build
reads.

Import is the only door a stranger's numbers come through, so everything about
trusting them lives in `share::from_json`:

- A fresh uuid and `isStock: false`, so an imported file can never wear a stock
  id and shadow a shipped template.
- A name already taken gets a numeric suffix rather than overwriting. Two
  people's "Warehouse" are two templates; an import that quietly replaced the
  one the user spent an evening tuning is the worst kind of data loss, the
  invisible kind.
- **Every value is clamped to the range the Configure tab itself enforces.**
  That is not politeness. `headroomDb` becomes the limiter's ceiling and
  `makeupDb` is raw gain, so a hand-edited or hostile file could otherwise ask
  for a ceiling above full scale or 400 dB of makeup, and the first the user
  would know of it is the damage. Clamping to the edge lands them where a
  slider would have stopped them anyway.
- Bands are **re-tiled**, not validated: the submitted crossovers are sorted
  and laid end to end from 20 Hz to 20 kHz with a minimum width reserved for
  every band above. `multiband` reads each `highHz` as the next band's `lowHz`
  without checking, so a file with a gap would come back with a notch at every
  edge and nothing in the UI to explain it.

Three things are refused rather than clamped, because they are not a template
being optimistic — they are not a template:

- A file over `MAX_FILE_BYTES` (64 KB). The import command reads through
  `read_capped`, one byte past that ceiling, so an oversized file is refused by
  its size without having been pulled into memory whole first.
- A band count outside 1..=8.
- A non-finite number. NaN came from a broken writer, not from a user, and
  `f64::clamp` panics on it.

The same bounds run over a template coming **off disk** (`share::clamped`), so a
hand-edited `templates.json` is as safe as an imported file. A template that
cannot be bounded at all keeps its name and takes Festival's values, logged at
`warn`: dropping something the user made without telling them is worse.

### Settings

Output location (specific folder or beside the source), output folder, mirror the
input folder structure, format, MP3 rate, worker count, active template id.

- `output_dir: None` means "not chosen yet". The runner falls back to
  `~/Music/Capstone`, so a first run never has to stop and ask before it can write
  anything.
- **AIFF 16/44 is the default** because every DJ program on both platforms opens
  it and nothing is thrown away.
- `max_cores` defaults to all of them and is floored on read as well as on write.
  A stored zero would size the rayon pool to nothing.
- Theme and accent are **not** in `Settings`. They are client-only, live in
  `localStorage`, and are applied to the DOM before React mounts so the first
  paint is not the default theme flashing and then swapping.

Personalize has no Save button either. Every control writes the whole `Settings`
object through `update_settings` on change, and each pane renders from what came
back rather than from a local draft, so a rejected write shows the stored value
instead of the attempted one.

## The run

`jobs/mod.rs` is the queue runner: one bounded rayon pool over the queued files,
three events out, and a cancel flag checked at **file boundaries**.

- The pool is built explicitly and never rayon's global one. The global pool sizes
  itself to the machine and ignores `max_cores`, which would make the Processor
  Load setting a decoration. Parallelism is across files only — `flacenc`'s own
  `par` feature is switched off for the same reason.
- `running` is held for the length of a run, so a second Start is a no-op rather
  than an error. Two pools over one queue would process some files twice and race
  on their output paths. It is released on every path, including a failure to
  build the pool; leaving it raised would make Start dead for the rest of the
  session.
- Cancel is checked per file, never mid-file, so a cancel cannot leave a
  half-written output on disk.
- Output is written through `write_atomically`: a file appears at its destination
  only once it is complete, so a failed encode cannot leave a truncated track for
  the user to find in a set weeks later.
- A failure lands on the **row's own Status cell**. The user is looking at the
  table, so an error that only went to the log would read as a file that silently
  did nothing.
- A `done` row is skipped on the next Start; an `error` row is retried, since the
  usual reason to press Start again is that the user fixed whatever was wrong.
  `analyzing` rows are **included** — analysis begins the moment files are added,
  so a user who drops a folder and presses Start a second later is looking at a
  table of rows mid-decode, and leaving those out meant the files they had just
  added were exactly the ones silently skipped.

Analysis (`jobs/analysis.rs`) is deliberately **not** part of the run. A dropped
folder should start reporting what is in it straight away, so that by the time the
user reaches Start the table is already the thing they are deciding from. The run
measures loudness again on the post-chain signal, so nothing in analysis is a
prerequisite for processing a file: it is the report, not a stage.

Tags ride through the pipeline and are written back in whatever form the output
format accepts. These are DJ libraries — a track whose artist and title vanished
is worse than a track that was never processed, because the owner has to find it
again by ear. ALAC is the one exception, stated in the UI rather than discovered:
the CAF writer has no `info` chunk.

## Window chrome

Three strips, and which one a thing belongs in is settled:

- **Title bar** — tabs on the left (`pl-[68px]`, clear of the traffic lights at
  x:16), then the update badge, the onboarding help action and the theme
  toggle on the right. The bar is a `flex` row with a full-bleed drag layer
  behind it; wrappers stay `pointer-events-none` so the empty width still
  reaches that layer, and only the controls take their clicks back.
- **Status bar** — the signature strip along the bottom: mark, name, version,
  copyright, and the support link. The version is read from the bundle with
  `getVersion()`, never written as a literal, or it drifts the moment a release
  ships and then disagrees with the version the updater reports one strip up.
  The support link goes through the opener plugin; a bare `<a href>` navigates
  the webview itself and replaces the app with a web page, with no way back.
- **The pages** — everything else.

**The mark lives in the status bar, not the title bar.** Beside the traffic
lights it crowded the tabs and read as chrome rather than as a byline, and the
title bar was three competing groups. With the left side free the tabs take it.
The status bar is one short muted row on purpose: a second toolbar down there
costs the queue table the height it needs.

## First run

A four-step tour in the shared `Modal`, mounted at the **root** rather than on
the Process tab: it is portalled over the whole window and has to survive a tab
switch while it is open. It renders nothing once the user has seen it.

The "seen" flag is `localStorage['capstone:onboardingDone']`, read
**synchronously at module load** in `hooks/useOnboarding.ts`. Deciding it in an
effect mounts the dialog for one frame on every relaunch, which is exactly the
flash an existing user must not see. Blocked site data reads as *already seen*:
a tour that cannot remember being dismissed would come back every launch, which
is worse than never showing it.

Every way out counts as having done it — Skip, Escape, the backdrop, the close
button and the last step all land on `finish()` — so none of them leaves the
user to be interrupted again tomorrow. The help action in the title bar
reopens it from step one.

Copy is two or three words of title and **one line** of body per step. A step
that needs a paragraph means the screen behind it is unclear, and the fix for
that is the screen.

## Cover art

`cover.rs` turns a file's embedded picture into a small PNG `data:` URI the
table can put straight in an `<img src>`. Two rules, both about size:

- **Nothing full-size ever leaves that module.** The picture is decoded, scaled
  to 64px on its long edge (roughly 2x the 28px the table draws, so it still
  looks right on a retina panel) and re-encoded before it is base64'd. A real
  cover is commonly 1400x1400 and a megabyte or two; fifty of those, plus a
  third again for base64, held in React state is hundreds of megabytes of
  renderer heap for a thumbnail. PNG rather than JPEG because the input may
  carry alpha and there is no quality decision to make at this size.
- **The result is memoised per path**, and `None` is cached as well as `Some`:
  most files in a real library carry no art, and re-probing them on every
  scroll is the same wasted work as re-decoding. The cache is unbounded on
  purpose — an entry is a few kilobytes and the keys are the paths the user
  added, so the queue is the ceiling, which makes `clear_queue` the thing that
  brings it down. Keyed on path alone, so a file re-tagged underneath us keeps
  its old thumbnail until a restart.

The lock is **not** held across the decode. Two rows asking for the same path at
once both do the work, which is cheap and rare; blocking every other row behind
one slow file is neither. `get_cover_art` is `spawn_blocking` for the same
reason: the first call for a path opens a file and decodes a full-size image,
and on the IPC thread that stalls every other command behind it.

Artwork never fails. No picture, an unreadable file and a format we do not
decode all answer `null`, and the UI draws the placeholder: a tagger that wrote
something strange must not become an error in the UI. Every tag is searched,
not just the primary one — a converted file often keeps its art in an ID3v2
block while its primary tag is something else, and reading only the primary tag
reports no artwork on a file that visibly has some.

On the frontend `useCoverArt` keys on the **path**, matching the backend cache,
so the same file queued twice hits one entry on both sides. It is
`staleTime: Infinity` with no refetch on mount or focus: artwork is baked into
the file, so it is the rare thing that genuinely cannot go stale while the app
is open, and scrolling the queue would otherwise refire the query for every row
that leaves and re-enters the viewport.

## The row menu

Right-click on a queue row: Play file, Reveal in Finder, Copy path, Re-analyze,
Open output, Remove from queue. `ContextMenu` is the shared primitive;
`rowMenu.tsx` is only the entry list, and `useRowActions` holds the shell calls.

- **Every row shows the same entries in the same order.** What a row cannot do
  right now is greyed out, never dropped: a menu that changes shape per row has
  to be re-read every time it opens, and the user loses where Remove sits.
  `analyzing` disables Re-analyze (it would race the read in flight) and
  `processing` disables Re-analyze and Remove (the runner is holding that row).
- **Re-analyze is the one entry with no other affordance.** Before it, an
  errored row could only be re-read by removing it and adding it back.
- The menu is **portalled to `document.body`** so no ancestor's `overflow` or
  `transform` can clip it, and clamped to the viewport in a layout effect after
  measuring, so it paints once at the clamped position instead of flashing at
  the raw one. Listeners are registered in the **capture** phase: scroll does
  not bubble, and a row that stops propagation would otherwise trap the menu
  open. Scroll closes it, because the row it belongs to is in a virtualized
  list and scrolling moves that row out from under a menu positioned once in
  viewport coordinates.
- The shell calls go through a hook, not the row, for the same reason `invoke`
  lives behind the service layer, and they are **mutations** so a failure — a
  file the user moved since dropping it is the everyday one — is something
  react-query owns rather than an unhandled rejection in the console. Clipboard
  writes fall back to a hidden textarea: `navigator.clipboard` is exposed only
  in a secure context, which the webview's custom origin is not on every
  platform.

## Design system

CSS custom properties in `src/index.css`; Tailwind extends `capstone.*` and
`gray.*`.

- `--capstone-black` `#0C0C0C` (page), `--capstone-sky` `#3FC5F2` (accent),
  `--capstone-white` `#F7F7F7` (foreground), plus a nine-step gray ramp
- `--capstone-red` `#FF6B6B` is the one hue outside the accent and the ramp. It
  exists because a failed file has to separate from a queued one at a glance
  and the ramp has only greys to say it with. Use `text-capstone-red`, never a
  literal and never Tailwind's stock red: both were in the tree at once, in
  five places, and neither followed the light theme.
- Fonts: **Syne** for headings and labels, **DM Mono** for dB, cents,
  percentages and file paths

Four rules that break things quietly if ignored:

- **Tokens are space-separated RGB triplets** (`63 197 242`), with no `rgb()`
  wrapper and no commas, so Tailwind's `<alpha-value>` placeholder can compose
  opacity: `bg-capstone-sky/10` → `rgb(var(--capstone-sky) / 0.1)`. Wrapping the
  value in `rgb()` in the stylesheet silently breaks every `/NN` class in the app.
- **The light theme flips the values under the same names.** `--capstone-black`
  means "page background" and `--capstone-white` means "primary foreground", not
  literal colours, which is what lets every component class stay
  theme-agnostic. It is selected by `<html data-theme="light">` rather than
  `prefers-color-scheme`, because the app offers the choice. The light accent is
  several steps darker (`#0D7099`): `#3FC5F2` on white is far below a readable
  contrast ratio.
- **The ramp has two halves, and mixing them is what broke the dark theme.**
  `900/800/700` are **planes** — the page sits below them and each step is a
  surface you can land an element on; none of them is legible as text.
  `600/500/400/300/200/100` are **ink**, each clearing 4.5:1 on the darkest
  plane it is used over (600 is the floor at 3:1, for hairlines, status dots
  and 9-10px unit labels). Reaching into the plane half for text, the way
  `text-gray-600` did for the drop-zone caption, buys 1.5:1 and an invisible
  control; reaching into the ink half for a fill buys a grey slab. The dark
  values are **not** a numeric mirror of the light ones — planes are spaced by
  CIE L*, ink is solved against the worst plane it actually sits on. Mirroring
  is what the old ramp did, and it put four planes inside a 1.26:1 band. The
  light theme had the matching bug from the other side: `--gray-900` was pure
  white, the same value as the page, so the shell had no plane of its own and
  the queue's `hover:bg-gray-900` was a literal no-op.
- **Disabled is a step down the ink ramp, never an `opacity`.** A disabled
  control paints in `gray-500` on a card (4.0:1 dark, 4.5:1 light) or
  `gray-400` inside a well such as an `Input` (4.6:1 dark, 5.1:1 light), at
  full alpha. `opacity` multiplies down the tree: `FieldRow` used to dim a
  dependent row at `opacity-40` while the `NumberField` in it dimmed itself at
  `opacity-50`, so the pitch card's "When off key by at least" and its cents
  field rendered at an effective 0.2 — 1.38:1 in dark, in the state that card
  ships in. Controls own their disabled look (`Input`, `Checkbox`, `Radio`);
  a container dims its own label and nothing else. The one place `opacity`
  still disables is a solid control that fades as a whole and holds no nested
  dimming of its own — `Button`, and the slider track — which is why those are
  safe and a text row is not.


**One theme for the whole app, held outside React.** `useTheme` publishes
through `useSyncExternalStore`; it used to be `useState` inside the hook, which
hands every caller its own copy. The title bar's toggle repainted `<html>` for
everyone, but a second consumer — a status bar, an onboarding step, a settings
pane — went on rendering the icon and label of whichever theme the app was in
when it mounted, and nothing recovered it short of a remount. The accent
override is a single value for both themes, which is a limit of the accent
contract rather than an oversight: there is nowhere to keep a per-theme pair.

## Tests

`cargo test --all-features` runs 124 unit tests in the library and 25 end-to-end
tests in `tests/`.

The end-to-end suite is the one to understand. **Every fixture is synthesised and
written to disk**, so the suite depends on nothing on the machine it runs on and
goes into CI as it stands. Each test then goes through the production path —
symphonia decodes the file, the chain runs, an encoder writes the output through
`jobs::output`, and the result is decoded back — and **every number an assertion
reads is measured by `support::measure`**, which uses `ebur128` and `rustfft`
directly and never Capstone's own analysis. A bug in the limiter's sidechain or in
the Clips column therefore cannot hide behind a test that measured with it.

The eight claims, one module each:

1. **loudness** — a programme-like file lands within 0.5 LU of −12.5 LUFS.
2. **clips** — the clip count drops to zero, the true peak stays under the
   ceiling, and the regions that were flat are not flat any more. The last part is
   the one that matters: a repair that only turned the file down would satisfy a
   count-only test.
3. **pitch** — +30 cents comes back within 2 cents; `keep_tempo` holds duration to
   0.5%, `change_tempo` moves it as the arithmetic predicts.
4. **limiter** — no sample peak and no inter-sample peak over the ceiling.
5. **warmth** — `tube` adds measurably more harmonic energy than `gentle`, which
   adds more than `none`.
6. **multiband** — with every compressor bypassed, the bands sum flat to 0.1 dB.
7. **formats** — a round trip through every output format, reopened by the reader
   the app itself uses.
8. **edges** — nothing panics, and silence stays silent.

The one layer not covered is the Tauri bridge itself: the queue runner needs a
live `AppHandle`, so `support::pipeline` reassembles the same sequence
`jobs::process_one` runs out of the same public calls.

## Release

`.github/workflows/release.yml` builds a macOS universal DMG and a Windows
installer on a `v*` tag and publishes the GitHub release, with notes rendered from
`src/data/changelog.json`. The version is taken from the tag and written into
`tauri.conf.json` and `Cargo.toml` by the workflow, so a tag is the only place a
version number has to be edited.

Signing secrets are referenced by name only and are not configured yet:
`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Until a
keypair exists and `bundle.createUpdaterArtifacts` is set in `tauri.conf.json`,
the build still succeeds and simply ships no update manifest. See the next section
for what that means for in-app updating.

## Known stubs / TODO

- **In-app updating is not wired.** `@tauri-apps/plugin-updater` is in
  `package.json` and `lib/tauri-core.ts` already re-exports the `Resource` symbol
  it needs, but there is no `tauri-plugin-updater` in `Cargo.toml`, no updater
  config and no UI. The release workflow is written to publish `latest.json` the
  moment a signing keypair exists, so 0.1.0 installs cannot self-update and will
  need a manual download.
- **ALAC is 16-bit and untagged.** `FormatDescription::alac()` hardcodes
  `bits_per_channel: 16` with private fields, so 24-bit would mean writing the
  magic cookie by hand, and the CAF writer has no `info` chunk. FLAC is the
  lossless format to choose when either matters.
- **MP3 output is behind a feature and the encoder is young.** `rusty_mp3` has not
  been validated against a large body of real programme material. Treat the
  format as provisional.
- **No output rate conversion is offered.** A file keeps the rate it arrived at;
  the format labels in the UI name the common case, not a conversion. The only
  resampling on the way out is the one MP3's nine legal rates force.
- **Tags are reduced to a short closed list.** Artwork, ReplayGain and MusicBrainz
  ids are dropped, because the five output formats agree on almost nothing and a
  field earns a place only if more than one of them can write it.
- **WAV has no RF64 or BWF.** `hound` cannot express a file over 4 GB. For single
  tracks that will not bite.
- **App icons are generated placeholders.** The real mark is in
  `src/assets/logo/`; the bundle icons under `src-tauri/icons/` were produced from
  it by the Tauri CLI and have had no eye on them at small sizes.
- **No preview and no undo.** Processing writes new files and never touches the
  source, which is the whole safety story — there is nothing to undo, but there is
  also no way to hear the result before the batch runs.

## Common gotchas

- **White screen after adding an icon** → the name does not exist in
  iconsax-reactjs 0.0.8. Check `node_modules/iconsax-reactjs/dist/esm/index.js`
  before importing, and add it to `components/ui/icons.ts`, never anywhere else.
- **"no reactor running" panic** → `tokio::spawn` inside `setup`. Use
  `tauri::async_runtime::spawn`.
- **A new command returns "command not found"** → it is missing from
  `generate_handler!` in `lib.rs`.
- **A column reads `undefined`, or a setting forgets itself** → a missing or
  wrong `rename_all` on the Rust type, or a snake_case key in a service call.
  Remember `pitchMode` and `outputLocation` carry **snake_case values** inside a
  camelCase field.
- **`cargo test` suddenly takes ten minutes** → `opt-level = 2` has been dropped
  from `[profile.dev]`. Unoptimised, the DSP runs roughly forty times slower. Note
  `profile.dev` matters as well as `profile.test`, because an integration test
  links the library as a *dependency* and a dependency is built with the dev
  profile however the test target itself is built.
- **Grepping for `tauri-core` finds nothing** → Vite aliases
  `@tauri-apps/api/core` and `@tauri-apps/api/event` to the shims in `src/lib/`.
  An import that reads as the real package resolves there. Anything a plugin
  reaches for and the shim does not re-export is `undefined` at runtime with no
  build error.
- **Play file or Open output does nothing, with no build error** → the opener
  plugin's `opener:default` set covers urls and reveal-in-dir but *not*
  `open_path`, and `allow-open-path` on its own enables the command with an
  empty scope, which refuses every path. `capabilities/default.json` carries the
  permission **and** a `**` path scope; both halves are needed.
- **The Processor Load slider appears to do nothing** → something is using
  rayon's global pool instead of the explicit one in `jobs::run_inner`.
- **Every band edge has a notch** → the allpass compensation in
  `process/multiband.rs` is wrong. Run the multiband claim before touching any
  band setting by ear.
- **"Warmth" sounds like a broken codec, with a tone that moves the wrong way** →
  the oversampler is bypassed or its delay compensation is off by a fraction of a
  sample.
- **A processed file opens but Capstone cannot reopen it** → FLAC. `flacenc`
  writes a streaminfo block that needs `fix_flac_streaminfo`; it also needs
  `use_fixed`, which is the difference between a 596 kB file and a 370 MB one.
- **The Open Output Folder button looks broken on a fresh install** → the folder
  is created before the reveal for exactly this reason; both platforms fail
  silently when asked to reveal a path that does not exist.
- **New chain logic does not change an already-processed file** → nothing is ever
  written twice. A `done` row is skipped on the next Start; remove it and re-add
  it, or clear the queue.
