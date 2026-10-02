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
    // Group A — Code & Docs (preserves per-language line-diff tracking)
    RustEdit { path: String, lines_added: usize, lines_removed: usize },
    PythonEdit { path: String, lines_added: usize, lines_removed: usize },
    WebEdit { path: String, lines_added: usize, lines_removed: usize },
    StyleEdit { path: String, lines_added: usize, lines_removed: usize },
    MarkupEdit { path: String, lines_added: usize, lines_removed: usize },
    ConfigEdit { path: String, lines_added: usize, lines_removed: usize },
    DocsEdit { path: String, lines_added: usize, lines_removed: usize },
    ShellScriptEdit { path: String, lines_added: usize, lines_removed: usize },

    // Group B — Data & Media
    ImageAsset { path: String },
    AudioAsset { path: String },
    VideoAsset { path: String },
    FontAsset { path: String },
    NotebookActivity { path: String },

    // Group C — AI/ML
    ModelTrainingCheckpoint { path: String, size_bytes: u64 },
    ModelConfigEdit { path: String },

    // Group D — Network / Transfer
    IncomingDataStream { path: String, bytes_per_sec: u64, progress_pct: Option<u8> },
    ArchiveWrite { path: String },

    // Group E — Process / Action-based
    GitOperation { path: String },
    DependencyLockUpdate { path: String },
    TestFileActivity { path: String },
    EnvSecretChange { path: String },
    CiPipelineEdit { path: String },
    ContainerConfigEdit { path: String },

    // Group F — Structural
    WorkspaceExpansion { path: String },
    MassDeletion { count: usize, sample_paths: Vec<String> },
    FileMutation { path: String, op: FileOp },

    // System state
    SystemIdle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOp {
    Created,
    Modified,
    Deleted,
    Renamed,
}
