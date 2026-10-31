//! End-to-end tests for Capstone's audio engine, run against real files.
//!
//! Every fixture is generated here and written to disk, so the suite depends on
//! nothing on the machine it runs on and can go in CI as it stands. From there
//! each test goes through the production path - symphonia decodes the file, the
//! chain runs on the decoded buffer, an encoder writes the output through
//! `jobs::output`, and the result is decoded back - and every number an
//! assertion reads is measured by `support::measure`, which uses `ebur128` and
//! `rustfft` directly and never Capstone's own analysis. A bug in the limiter's
//! sidechain or in the Clips column therefore cannot hide behind a test that
//! measured with it.
//!
//! The one layer not exercised here is the Tauri bridge itself: the queue
//! runner needs a live `AppHandle`, so `support::pipeline` reassembles the same
//! sequence `jobs::process_one` runs out of the same public calls.

mod e2e;
mod support;
