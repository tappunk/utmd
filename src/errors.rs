#[derive(Copy, Clone, Debug)]
pub enum ExitCode {
    Success,
    InvalidUsage,
    DependencyMissing,
    NotFound,
    Conflict,
    PartialFailure,
    ExternalCommandFailed,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::InvalidUsage => 2,
            Self::DependencyMissing => 3,
            Self::NotFound => 4,
            Self::Conflict => 5,
            Self::PartialFailure => 6,
            Self::ExternalCommandFailed => 10,
        }
    }
}
