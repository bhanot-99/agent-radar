use std::fs;
use std::path::Path;
use crate::events::FileOp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleVerdict {
    WorkspaceExpansion,
    ModelTrainingCheckpoint { size_bytes: u64 },
    IncomingDataStream,
    SourceCodeMutation,
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
) -> RuleVerdict {
    // 1. Directory creation -> WorkspaceExpansion
    if is_dir && op == FileOp::Created {
        return RuleVerdict::WorkspaceExpansion;
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 2. Checkpoint extensions (.pt, .safetensors, .ckpt, .onnx)
    if file_name.ends_with(".pt")
        || file_name.ends_with(".safetensors")
        || file_name.ends_with(".ckpt")
        || file_name.ends_with(".onnx")
    {
        let size_bytes = get_file_size(path);
        return RuleVerdict::ModelTrainingCheckpoint { size_bytes };
    }

    // 3. .bin disambiguation via path context predicates
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

    // 4. Archive / dataset extensions gated on ACTIVE_NETWORK_STREAM
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
            return RuleVerdict::FileMutation(op);
        }
    }

    // 5. Source code extensions
    if file_name.ends_with(".rs")
        || file_name.ends_with(".py")
        || file_name.ends_with(".ts")
        || file_name.ends_with(".tsx")
        || file_name.ends_with(".js")
        || file_name.ends_with(".jsx")
        || file_name.ends_with(".cpp")
        || file_name.ends_with(".c")
        || file_name.ends_with(".h")
        || file_name.ends_with(".hpp")
        || file_name.ends_with(".go")
        || file_name.ends_with(".java")
        || file_name.ends_with(".toml")
        || file_name.ends_with(".json")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml")
        || file_name.ends_with(".md")
        || file_name.ends_with(".sh")
        || file_name.ends_with(".css")
        || file_name.ends_with(".html")
        || file_name.ends_with(".sql")
    {
        return RuleVerdict::SourceCodeMutation;
    }

    // 6. Generic File Mutation catch-all
    RuleVerdict::FileMutation(op)
}
