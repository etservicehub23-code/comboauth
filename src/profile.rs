/// A saved combo profile: name, token sequence, recorded timing gaps, and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboProfile {
    pub name: String,
    pub sequence: String,
    pub status: String,
    pub timing_window_ms: u32,
    /// Recorded inter-keypress gaps (ms) from the original recording session.
    /// Empty means no timing constraint is enforced at test time.
    pub gaps_ms: Vec<u64>,
}
