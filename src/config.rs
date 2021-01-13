/// Configuration tuning parameters for GhostActors
#[non_exhaustive]
pub struct GhostConfig {
    /// Channel bound for communicating with actor.
    /// Default: 32.
    pub channel_bound: usize,
}

impl Default for GhostConfig {
    fn default() -> Self {
        Self { channel_bound: 32 }
    }
}
