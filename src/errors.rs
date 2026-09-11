#[derive(Debug)]
pub struct TimedOut {
    pub label: &'static str,
    pub timeout_secs: u64,
}

impl std::fmt::Display for TimedOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} timed out after {}s — UTM may be unresponsive",
            self.label, self.timeout_secs
        )
    }
}

impl std::error::Error for TimedOut {}

#[derive(Copy, Clone, Debug)]
pub enum ExitCode {
    Success,
    InvalidUsage,
    DependencyMissing,
    NotFound,
    Conflict,
    PartialFailure,
    ExternalCommandFailed,
    CommandTimedOut,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::InvalidUsage => 64,
            Self::DependencyMissing => 124,
            Self::NotFound => 66,
            Self::Conflict => 65,
            Self::PartialFailure => 1,
            Self::ExternalCommandFailed => 1,
            Self::CommandTimedOut => 1,
        }
    }
}
