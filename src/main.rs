use std::env;
use std::path::PathBuf;

use agent_radar::acquire_single_instance_lock;
use agent_radar::app_runner::AppRunner;
use agent_radar::install_panic_hook;
use agent_radar::restore_terminal;

fn print_usage(prog: &str) {
    eprintln!(
        "Usage: {} [OPTIONS] [PATH]\n\n\
        Agent-Radar: Ultra-lightweight terminal HUD for AI coding agents\n\n\
        Arguments:\n  \
          [PATH]        Project directory to watch (default: current directory)\n\n\
        Options:\n  \
          -d, --debug   Enable debug log output\n  \
          -h, --help    Print help information\n  \
          -v, --version Print version",
        prog
    );
}

fn close_stray_fds() {
    use std::os::fd::FromRawFd;
    if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
        let fds: Vec<i32> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_string_lossy().parse::<i32>().ok())
            .filter(|&fd| fd > 2)
            .collect();
        for fd in fds {
            unsafe {
                let _ = std::fs::File::from_raw_fd(fd);
            }
        }
    }
}

fn main() {
    close_stray_fds();
    install_panic_hook();

    let args: Vec<String> = env::args().collect();
    let prog = &args[0];
    let mut watch_dir = PathBuf::from(".");
    let mut debug_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage(prog);
                return;
            }
            "-v" | "--version" => {
                println!("agent-radar 0.1.0");
                return;
            }
            "-d" | "--debug" => {
                debug_mode = true;
            }
            arg if !arg.starts_with('-') => {
                watch_dir = PathBuf::from(arg);
            }
            unknown => {
                eprintln!("Unknown option: {}", unknown);
                print_usage(prog);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let _lock = match acquire_single_instance_lock() {
        Ok(lock) => lock,
        Err(e) => {
            if e == "already running" {
                eprintln!("already running");
                std::process::exit(1);
            } else {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
    };

    let canonical_path = match watch_dir.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] Cannot access path {:?}: {}", watch_dir, e);
            std::process::exit(1);
        }
    };

    let mut runner = match AppRunner::new(&canonical_path, debug_mode) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[ERROR] Failed to initialize AppRunner: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = runner.run_loop() {
        eprintln!("[ERROR] Runtime error: {}", e);
        restore_terminal();
        std::process::exit(1);
    }

    restore_terminal();
}
