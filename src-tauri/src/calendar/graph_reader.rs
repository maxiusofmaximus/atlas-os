// Atlas OS — Calendar MS Graph reader (RFC 28 Section G — READ path).
//
// Placeholder — full implementation lands in §G items 5-8 (post-MVP),
// backed by `graph-rs-sdk` / `aes-gcm` / `ring` dependencies that are
// already declared (optional) in `Cargo.toml`. This stub exists so
// `cargo fmt` / `cargo clippy` over `--features calendar-graph` can
// locate the module file before its real content is written.
//
// Projected public surface (RFC 28 §G.3 + §G.4):
//
//     pub struct CalendarReader { ... }
//     impl CalendarReader {
//         pub fn new(profile_root: &Path) -> Result<Self, CalendarError>;
//         pub fn authenticate(&self) -> Result<(), CalendarError>;
//         pub fn poll_once(&self, queue: &BusyWindowQueue<'_>)
//             -> Result<usize, CalendarError>;
//         pub fn spawn(self, queue: Arc<Mutex<...>>, cancel: CancellationToken)
//             -> JoinHandle;
//     }
