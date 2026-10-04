// Atlas OS — Calendar integration module (RFC 28 Section G).
//
// Provides two independent surfaces that share a single SQLite-backed
// `calendar_busy_windows` table (M18):
//
// 1. **WRITE path** (`calendar-ics` feature) — `CalendarWriter` builds
//    a RFC 5545 iCalendar text feed from the missions in the Journal so
//    a user can subscribe `webcal://127.0.0.1:{port}/opencode-
//    calendar.ics?token=...` from Outlook / Apple Calendar / Google
//    Calendar. The feed is read-only (`METHOD:PUBLISH`) and emitted on
//    demand via the axum HUD server (`calendar/ics_route.rs`).
//
// 2. **READ path** — two sources feed `calendar_busy_windows`:
//    (a) `calendar-graph` pulls Microsoft Graph `/me/calendarView` via
//    OAuth 2.0 authorization-code + PKCE (loopback redirect; `auth.rs`)
//    and refreshes silently against the persisted AES-256-GCM refresh
//    token; (b) `calendar-ics` fetches operator-subscribed `.ics` feeds
//    (`ics_reader.rs`) with a conditional GET. Both persist into
//    `calendar_busy_windows` so the Planning engine (RFC 12 §3) can
//    consult `next_free_slot(turn_eta)` before enqueuing a proactive
//    turn. Subscribed feeds are registered durably in
//    `calendar_subscriptions` (M35) so one feed can be re-synced or
//    dropped without touching the others (ids are namespaced
//    `{subscription}:{uid}`).
//
// On a default build (no `calendar-*` feature on) the M18 tables are
// still created — that keeps the schema idempotent across feature
// combos, same as M17 for Toast. The writers/readers are feature-gated;
// the pure SQLite CRUD (`queue.rs`) compiles regardless so a non-
// calendar build can still expose the `opencode calendar busy list`
// debug surface (handy for operators that want to see manual / ics_local
// rows without enabling the Graph poller).

pub mod error;
pub mod payload;
pub mod queue;
pub mod token;

#[cfg(feature = "calendar-ics")]
pub mod ics_writer;

#[cfg(feature = "calendar-ics")]
pub mod ics_route;

#[cfg(feature = "calendar-ics")]
pub mod ics_reader;

#[cfg(feature = "calendar-ics")]
pub mod poller;

#[cfg(feature = "calendar-graph")]
pub mod auth;
#[cfg(feature = "calendar-graph")]
pub mod graph_reader;

pub use error::CalendarError;
pub use payload::{BusySource, BusyWindow, IcsMission};
pub use queue::{BusyWindowInput, BusyWindowQueue, BusyWindowRow};

#[cfg(feature = "calendar-ics")]
pub use ics_writer::CalendarWriter;

// The `calendar-graph` READ path is deferred to §G items 5-8 (post-MVP,
// Phase 2): the placeholder `graph_reader` module compiles empty until
// its real `CalendarReader` is written, so no re-export exists yet.
