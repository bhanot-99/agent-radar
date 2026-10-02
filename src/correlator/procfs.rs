use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
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
        if let Some(stripped) = line.strip_prefix("Name:") {
            name = stripped.trim().to_string();
        } else if let Some(stripped) = line.strip_prefix("PPid:") {
            ppid = stripped.trim().parse().unwrap_or(0);
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
    let after_paren = stat[close_paren + 1..].trim_start();
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
    let mut socket_inodes = Vec::new();

    if let Ok(entries) = fs::read_dir(fd_dir) {
        for entry in entries.flatten() {
            if let Ok(target) = fs::read_link(entry.path()) {
                let target_str = target.to_string_lossy();
                if let Some(rest) = target_str.strip_prefix("socket:[") {
                    if let Some(inode_str) = rest.strip_suffix(']') {
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            socket_inodes.push(inode);
                        }
                    }
                }
            }
        }
    }

    if socket_inodes.is_empty() {
        return false;
    }

    // Cross-reference this PID's own socket inodes against network connection tables.
    // Only return true if one of THIS PID's own sockets has state 01 (TCP_ESTABLISHED).
    check_socket_inodes_established("/proc/net/tcp", &socket_inodes)
        || check_socket_inodes_established("/proc/net/tcp6", &socket_inodes)
        || check_socket_inodes_established("/proc/net/udp", &socket_inodes)
        || check_socket_inodes_established("/proc/net/udp6", &socket_inodes)
}

fn check_socket_inodes_established(path: &str, inodes: &[u64]) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // In /proc/net/{tcp,tcp6,udp,udp6}:
            // field 4 (index 3) is connection state: 01 = TCP_ESTABLISHED
            // field 10 (index 9) is the socket inode
            if fields.len() > 9 && fields[3] == "01" {
                if let Ok(entry_inode) = fields[9].parse::<u64>() {
                    if inodes.contains(&entry_inode) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn check_pid_accessing_path(pid: u32, target_path: &Path, canonical: &Path) -> bool {
    let fd_dir = format!("/proc/{}/fd", pid);
    if let Ok(fds) = fs::read_dir(fd_dir) {
        for fd_entry in fds.flatten() {
            if let Ok(link) = fs::read_link(fd_entry.path()) {
                if link == canonical || link == target_path {
                    return true;
                }
            }
        }
    }
    false
}

pub fn find_pid_accessing_path_with_candidates(
    target_path: &Path,
    candidates: &[u32],
    own_uid: u32,
) -> Option<u32> {
    let canonical = target_path.canonicalize().unwrap_or_else(|_| target_path.to_path_buf());

    // 1. Fast path: check candidate PIDs directly (O(candidates * fds) ~ 10-50 microseconds)
    for &pid in candidates {
        if check_pid_accessing_path(pid, target_path, &canonical) {
            return Some(pid);
        }
    }

    // 2. Slow path: scan own user's processes in /proc
    let proc_entries = fs::read_dir("/proc").ok()?;
    for entry in proc_entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            if candidates.contains(&pid) {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if meta.uid() != own_uid {
                    continue;
                }
            }
            if check_pid_accessing_path(pid, target_path, &canonical) {
                return Some(pid);
            }
        }
    }

    None
}

pub fn find_pid_accessing_path(target_path: &Path) -> Option<u32> {
    let own_uid = rustix::process::getuid().as_raw();
    find_pid_accessing_path_with_candidates(target_path, &[], own_uid)
}
