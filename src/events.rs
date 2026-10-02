use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub timestamp: SystemTime,
    pub pid: u32,
    pub ppid: u32,
    pub process_name: String,
    pub category: ActivityCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityCategory {
    IncomingDataStream {
        path: String,
        bytes_per_sec: u64,
        progress_pct: Option<u8>,
    },
    ModelTrainingCheckpoint {
        path: String,
        size_bytes: u64,
    },
    SourceCodeMutation {
        path: String,
        lines_added: usize,
        lines_removed: usize,
    },
    WorkspaceExpansion {
        path: String,
    },
    FileMutation {
        path: String,
        op: FileOp,
    },
    SystemIdle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOp {
    Created,
    Modified,
    Deleted,
    Renamed,
}
