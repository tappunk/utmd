use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VmInfo {
    pub name: String,
    pub state: Option<String>,
    pub os: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub ok: bool,
    pub action: String,
    pub target: Option<String>,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteSummary {
    pub matched: usize,
    pub skipped: usize,
    pub deleted: usize,
    pub failed: usize,
    pub results: Vec<OperationResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandResponse<T>
where
    T: Serialize,
{
    pub command: String,
    pub ok: bool,
    pub data: Option<T>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T>
where
    T: Serialize,
{
    pub fn success(command: &str, data: T) -> Self {
        Self {
            command: command.to_string(),
            ok: true,
            data: Some(data),
            warnings: Vec::new(),
            error: None,
        }
    }

    pub fn failure(command: &str, error: impl Into<String>) -> Self {
        Self {
            command: command.to_string(),
            ok: false,
            data: None,
            warnings: Vec::new(),
            error: Some(error.into()),
        }
    }
}
