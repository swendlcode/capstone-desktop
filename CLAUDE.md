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
    ui/                   Primitives — Button, Input, Checkbox, Radio, CapstoneMark, icons.ts
    layout/               TitleBar (drag region + traffic-light inset), TabBar
    process/              QueueTable (virtualized), QueueRow, MetricCell, StatusCell,
                          QueueToolbar, ProcessFooter, DropTarget, columns.ts, format.ts
    template/             TemplatePicker, the four setting cards, MultibandEditor,
                          SliderField / NumberField, useTemplateEditor
    personalize/          PersonalizeRail + the three panes
  pages/                  ProcessPage, TemplatePage, PersonalizePage
  hooks/                  useQueue (+ useQueueSync), useTemplates, useSettings, useTheme
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
    templates.rs          Stock templates (code, not data) + templates.json
    commands/             Thin Tauri handlers, one file per domain
    jobs/                 mod.rs (queue runner), analysis.rs (background), output.rs
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
  `delete_template`
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

## Design system

CSS custom properties in `src/index.css`; Tailwind extends `capstone.*` and
`gray.*`.

- `--capstone-black` `#0C0C0C` (page), `--capstone-sky` `#3FC5F2` (accent),
  `--capstone-white` `#F7F7F7` (foreground), plus a nine-step gray ramp
- Fonts: **Syne** for headings and labels, **DM Mono** for dB, cents,
  percentages and file paths

Two rules that break things quietly if ignored:

- **Tokens are space-separated RGB triplets** (`63 197 242`), with no `rgb()`
  wrapper and no commas, so Tailwind's `<alpha-value>` placeholder can compose
  opacity: `bg-capstone-sky/10` → `rgb(var(--capstone-sky) / 0.1)`. Wrapping the
  value in `rgb()` in the stylesheet silently breaks every `/NN` class in the app.
- **The light theme flips the values under the same names.** `--capstone-black`
  means "page background" and `--capstone-white` means "primary foreground", not
  literal colours, which is what lets every component class stay
  theme-agnostic. It is selected by `<html data-theme="light">` rather than
  `prefers-color-scheme`, because the app offers the choice. The light accent is
  several steps darker (`#1288BE`): `#3FC5F2` on white is far below a readable
  contrast ratio.

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
