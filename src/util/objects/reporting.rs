#[derive(Clone)]
pub enum ReportEvent {
    Operation(OperationStatus),
    Status(StatusReport)
}

#[derive(Clone)]
pub enum OperationStatus {
    START(String),
    DONE
}

#[derive(Clone)]
pub struct StatusReport {
    pub module: Option<String>,
    pub status: Status,
    pub run_type: RunType
}

#[derive(Clone)]
pub enum Status {
    START,
    DONE,
    ERROR,
    SKIP,
    DISABLED
}

#[derive(Clone)]
pub enum RunType {
    RUN,
    BACKUP,
    SYNC
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Status::START => write!(f, "start"),
            Status::DONE => write!(f, "done"),
            Status::ERROR => write!(f, "failure"),
            Status::SKIP => write!(f, "skip"),
            Status::DISABLED => write!(f, "disabled")
        }
    }
}

impl std::fmt::Display for RunType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunType::RUN => write!(f, "run"),
            RunType::BACKUP => write!(f, "backup"),
            RunType::SYNC => write!(f, "sync")
        }
    }
}