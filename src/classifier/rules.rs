use std::fs;
use std::path::Path;
use crate::events::FileOp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleVerdict {
    // Group A — Code & Docs
    RustEdit,
    PythonEdit,
    WebEdit,
    StyleEdit,
    MarkupEdit,
    ConfigEdit,
    DocsEdit,
    ShellScriptEdit,

    // Group B — Data & Media
    ImageAsset,
    AudioAsset,
    VideoAsset,
    FontAsset,
    NotebookActivity,

    // Group C — AI/ML
    ModelTrainingCheckpoint { size_bytes: u64 },
    ModelConfigEdit,

    // Group D — Network / Transfer
    IncomingDataStream,
    ArchiveWrite,

    // Group E — Process / Action-based
    GitOperation,
    DependencyLockUpdate,
    TestFileActivity,
    EnvSecretChange,
    CiPipelineEdit,
    ContainerConfigEdit,

    // Group F — Structural
    WorkspaceExpansion,
    FileMutation(FileOp),
}

pub fn is_checkpoint_path_context(path: &Path) -> bool {
    // 1. Check directory path components for checkpoint / weights / models
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp {
            let s = c.to_string_lossy().to_lowercase();
            if s.contains("checkpoint") || s.contains("weight") || s.contains("model") {
                return true;
            }
        }
    }

    // 2. Check if a sibling *.index.json exists in the same directory
    if let Some(parent) = path.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".index.json") {
                    return true;
                }
            }
        }
    }

    false
}

pub fn get_file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn evaluate_rules(
    path: &Path,
    is_dir: bool,
    op: FileOp,
    active_network_stream: bool,
    is_git_process: bool,
) -> RuleVerdict {
    // 1. Directory creation -> WorkspaceExpansion (highest priority)
    if is_dir && op == FileOp::Created {
        return RuleVerdict::WorkspaceExpansion;
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let path_str_lower = path.to_string_lossy().to_lowercase();

    // 2. Filename-exact matches (checked before any extension logic)
    // EnvSecretChange: .env or starts with .env.
    if file_name == ".env" || file_name.starts_with(".env.") {
        return RuleVerdict::EnvSecretChange;
    }

    // ContainerConfigEdit: Dockerfile, Dockerfile.*, docker-compose.yml/yaml
    if file_name == "dockerfile"
        || file_name.starts_with("dockerfile.")
        || file_name == "docker-compose.yml"
        || file_name == "docker-compose.yaml"
    {
        return RuleVerdict::ContainerConfigEdit;
    }

    // DependencyLockUpdate: exact lockfile names
    if matches!(
        file_name.as_str(),
        "package-lock.json"
            | "cargo.lock"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "poetry.lock"
            | "gemfile.lock"
    ) {
        return RuleVerdict::DependencyLockUpdate;
    }

    // CiPipelineEdit: .gitlab-ci.yml or .github/workflows/*.yml|yaml
    if file_name == ".gitlab-ci.yml"
        || (path_str_lower.contains(".github/workflows/")
            && (file_name.ends_with(".yml") || file_name.ends_with(".yaml")))
    {
        return RuleVerdict::CiPipelineEdit;
    }

    // 3. Test-pattern match (checked BEFORE per-language code splits)
    let is_test_file = file_name.starts_with("test_")
        || file_name.contains("_test.")
        || file_name.contains(".spec.");
    let is_test_path = path.components().any(|c| {
        if let std::path::Component::Normal(comp) = c {
            let s = comp.to_string_lossy().to_lowercase();
            s == "tests" || s == "test"
        } else {
            false
        }
    });
    if is_test_file || is_test_path {
        return RuleVerdict::TestFileActivity;
    }

    // 4. Process-attributed: git operations (checked BEFORE per-language splits)
    if is_git_process {
        return RuleVerdict::GitOperation;
    }

    // 5. Checkpoint extensions (.pt, .safetensors, .ckpt, .onnx)
    if file_name.ends_with(".pt")
        || file_name.ends_with(".safetensors")
        || file_name.ends_with(".ckpt")
        || file_name.ends_with(".onnx")
    {
        let size_bytes = get_file_size(path);
        return RuleVerdict::ModelTrainingCheckpoint { size_bytes };
    }

    // 6. .bin disambiguation via path context predicates
    if file_name.ends_with(".bin") {
        if is_checkpoint_path_context(path) {
            let size_bytes = get_file_size(path);
            return RuleVerdict::ModelTrainingCheckpoint { size_bytes };
        } else if active_network_stream {
            return RuleVerdict::IncomingDataStream;
        } else {
            return RuleVerdict::FileMutation(op);
        }
    }

    // 7. Model-config context (.json, .yaml, .yml in checkpoint path context)
    if (file_name.ends_with(".json") || file_name.ends_with(".yaml") || file_name.ends_with(".yml"))
        && is_checkpoint_path_context(path)
    {
        return RuleVerdict::ModelConfigEdit;
    }

    // 8. Archive / dataset extensions
    if file_name.ends_with(".zip")
        || file_name.ends_with(".tar.gz")
        || file_name.ends_with(".tgz")
        || file_name.ends_with(".tar")
        || file_name.ends_with(".parquet")
        || file_name.ends_with(".arrow")
        || file_name.ends_with(".csv")
        || file_name.ends_with(".jsonl")
    {
        if active_network_stream {
            return RuleVerdict::IncomingDataStream;
        } else {
            return RuleVerdict::ArchiveWrite;
        }
    }

    // 9. Media extensions
    // Image
    if file_name.ends_with(".png")
        || file_name.ends_with(".jpg")
        || file_name.ends_with(".jpeg")
        || file_name.ends_with(".gif")
        || file_name.ends_with(".svg")
        || file_name.ends_with(".webp")
    {
        return RuleVerdict::ImageAsset;
    }
    // Audio
    if file_name.ends_with(".mp3") || file_name.ends_with(".wav") || file_name.ends_with(".flac") {
        return RuleVerdict::AudioAsset;
    }
    // Video
    if file_name.ends_with(".mp4") || file_name.ends_with(".mov") || file_name.ends_with(".webm") {
        return RuleVerdict::VideoAsset;
    }
    // Font
    if file_name.ends_with(".ttf")
        || file_name.ends_with(".otf")
        || file_name.ends_with(".woff")
        || file_name.ends_with(".woff2")
    {
        return RuleVerdict::FontAsset;
    }
    // Notebook
    if file_name.ends_with(".ipynb") {
        return RuleVerdict::NotebookActivity;
    }

    // 10. Per-language code extensions
    // Rust
    if file_name.ends_with(".rs") {
        return RuleVerdict::RustEdit;
    }
    // Python
    if file_name.ends_with(".py") {
        return RuleVerdict::PythonEdit;
    }
    // Web
    if file_name.ends_with(".ts")
        || file_name.ends_with(".tsx")
        || file_name.ends_with(".js")
        || file_name.ends_with(".jsx")
    {
        return RuleVerdict::WebEdit;
    }
    // Style
    if file_name.ends_with(".css") || file_name.ends_with(".scss") || file_name.ends_with(".less") {
        return RuleVerdict::StyleEdit;
    }
    // Markup
    if file_name.ends_with(".html") {
        return RuleVerdict::MarkupEdit;
    }
    // Shell
    if file_name.ends_with(".sh") || file_name.ends_with(".bash") || file_name.ends_with(".zsh") {
        return RuleVerdict::ShellScriptEdit;
    }
    // Docs
    if file_name.ends_with(".md") || file_name.ends_with(".rst") || file_name.ends_with(".txt") {
        return RuleVerdict::DocsEdit;
    }
    // Config
    if file_name.ends_with(".toml")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml")
        || file_name.ends_with(".json")
    {
        return RuleVerdict::ConfigEdit;
    }

    // 11. Generic File Mutation catch-all
    RuleVerdict::FileMutation(op)
}
