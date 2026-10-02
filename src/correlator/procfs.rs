use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub start_time: u64,
}

pub fn read_process_info(pid: u32) -> io::Result<ProcessInfo> {
    let proc_dir = format!("/proc/{}", pid);
    
    // Read status for Name and PPid
    let status_str = fs::read_to_string(format!("{}/status", proc_dir))?;
    let mut name = String::new();
    let mut ppid = 0u32;

    for line in status_str.lines() {
        if line.starts_with("Name:") {
            name = line["Name:".len()..].trim().to_string();
        } else if line.starts_with("PPid:") {
            ppid = line["PPid:".len()..].trim().parse().unwrap_or(0);
        }
    }

    // Read cmdline
    let cmdline_bytes = fs::read(format!("{}/cmdline", proc_dir)).unwrap_or_default();
    let cmdline = if !cmdline_bytes.is_empty() {
        let parts: Vec<String> = cmdline_bytes
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).to_string())
            .collect();
        parts.join(" ")
    } else {
        name.clone()
    };

    // Read stat for start_time (field 22)
    let stat_str = fs::read_to_string(format!("{}/stat", proc_dir)).unwrap_or_default();
    let start_time = parse_start_time(&stat_str).unwrap_or(0);

    Ok(ProcessInfo {
        pid,
        ppid,
        name,
        cmdline,
        start_time,
    })
}

// Parses start_time (field 22) from /proc/<pid>/stat
// Note: field 2 is (comm) which can contain spaces and parentheses,
// so we find the last ')' and parse remaining fields from there.
fn parse_start_time(stat: &str) -> Option<u64> {
    let close_paren = stat.rfind(')')?;
    let after_paren = &stat[close_paren + 1..].trim_start();
    let fields: Vec<&str> = after_paren.split_whitespace().collect();
    // After '(comm)' which is field 2:
    // field 3 (state) is index 0
    // field 22 (starttime) is index 19
    if fields.len() > 19 {
        fields[19].parse::<u64>().ok()
    } else {
        None
    }
}

pub fn is_process_alive(pid: u32, recorded_start_time: u64) -> bool {
    let stat_path = format!("/proc/{}/stat", pid);
    if let Ok(stat_str) = fs::read_to_string(stat_path) {
        if let Some(current_start) = parse_start_time(&stat_str) {
            return current_start == recorded_start_time;
        }
    }
    false
}

pub fn has_active_socket(pid: u32) -> bool {
    let fd_dir = format!("/proc/{}/fd", pid);
    if let Ok(entries) = fs::read_dir(fd_dir) {
        for entry in entries.flatten() {
            if let Ok(target) = fs::read_link(entry.path()) {
                let target_str = target.to_string_lossy();
                if target_str.starts_with("socket:[") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn find_pid_accessing_path(target_path: &Path) -> Option<u32> {
    let canonical = target_path.canonicalize().ok().unwrap_or_else(|| target_path.to_path_buf());
    
    // Read /proc entries to search own processes
    let proc_entries = fs::read_dir("/proc").ok()?;
    for entry in proc_entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            let fd_dir = format!("/proc/{}/fd", pid);
            if let Ok(fds) = fs::read_dir(fd_dir) {
                for fd_entry in fds.flatten() {
                    if let Ok(link) = fs::read_link(fd_entry.path()) {
                        if link == canonical || link == target_path {
                            return Some(pid);
                        }
                    }
                }
            }
        }
    }
    None
}
