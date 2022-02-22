// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Computational performance stats.

use io_utils::dir_list;
use libc::{getuid, rusage, RLIMIT_NPROC};
use libc::{rlimit, setrlimit};
use std::{
    cmp::min,
    fs::File,
    io::{BufRead, BufReader},
    process::id,
    time::Instant,
};
use string_utils::TextUtils;

// Find elapsed time.  Usage example:
//    let t = Instant::now( );
//    .. do something ..
//    println!( "{} seconds used doing such and such", elapsed(&t) );

pub fn elapsed(start: &Instant) -> f64 {
    let d = start.elapsed();
    d.as_secs() as f64 + d.subsec_nanos() as f64 / 1e9
}

// Report number of threads in use.

pub fn nthreads() -> i64 {
    let procfn = "/proc/self/status";
    let prob1 = "\nWARNING: nthreads() failed to access /proc status file.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let f = std::fs::File::open(procfn);
    match f {
        Err(_) => println!("{}{}", prob1, crap),
        Ok(f) => {
            let f = std::io::BufReader::new(f);
            for line in f.lines() {
                let s = line.unwrap();
                if s.starts_with("Threads:") {
                    let mut s = s.after("Threads:").to_string();
                    s = s.replace("\t", "");
                    s = s.replace(" ", "");
                    return s.force_i64();
                }
            }
        }
    };
    -1
}

// Set the maximum number of threads.  This is intended as a debugging thing.
// If you think that at some point you're using too many threads, this gives you
// a way of finding it, by pre-capping the thread count.

pub fn set_max_threads(n: u64) {
    unsafe {
        let limit = rlimit {
            rlim_cur: n,
            rlim_max: n,
        };
        let _ = setrlimit(RLIMIT_NPROC, &limit);
    }
}

// Report peak memory usage in bytes or gigabytes.  For linux, this is determined by reading the
// proc filesystem.

#[cfg(target_os = "linux")]
pub fn peak_mem_usage_bytes() -> i64 {
    let procfn = "/proc/self/status";
    let prob1 = "\nWARNING: peak_mem_usage_bytes( ) failed to \
                 access /proc status file.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let f = std::fs::File::open(procfn);
    match f {
        Err(_) => println!("{}{}", prob1, crap),
        Ok(f) => {
            let f = std::io::BufReader::new(f);
            for line in f.lines() {
                let s = line.unwrap();
                if s.starts_with("VmHWM") {
                    let split: Vec<&str> = s.split_whitespace().collect();
                    let printfail = |msg| {
                        println!(
                            "\nWARNING:\
                         peak_mem_usage_bytes( ) encountered broken line in\
                         /proc status file.\n{}\nLine = {}\n{}",
                            msg, s, crap
                        )
                    };
                    if split.len() != 3 {
                        printfail("Field count wrong.");
                    } else if split[2] != "kB" {
                        printfail("Bad units field.");
                    } else if split[1].parse::<i64>().is_err() {
                        printfail("Bad count field.");
                    } else {
                        return split[1].force_i64() * 1024;
                    }
                    break;
                }
            }
        }
    };
    -1
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn peak_mem_usage_bytes() -> i64 {
    let maxrss_slf;
    unsafe {
        let mut rusage: libc::rusage = std::mem::zeroed();
        let retval = libc::getrusage(libc::RUSAGE_SELF, &mut rusage as *mut _);
        assert_eq!(retval, 0);
        maxrss_slf = rusage.ru_maxrss;
    }
    maxrss_slf
}

pub fn peak_mem_usage_gb() -> f64 {
    peak_mem_usage_bytes() as f64 / ((1024 * 1024 * 1024) as f64)
}

// Report available memory gigabytes.

pub fn available_mem_gb() -> Option<f64> {
    let procfn = "/proc/meminfo";
    // let mut bytes : i64 = -1;
    let f = std::fs::File::open(procfn);
    match f {
        Err(_) => None,
        Ok(f) => {
            let f = std::io::BufReader::new(f);
            for line in f.lines() {
                let s = line.unwrap();
                if s.starts_with("MemAvailable") {
                    let split: Vec<&str> = s.split_ascii_whitespace().collect();
                    if split.len() != 3 || split[2] != "kB" {
                        return None;
                    }
                    return Some(split[1].parse::<i64>().ok()? as f64 / (1024_f64 * 1024_f64));
                }
            }
            None
        }
    }
}

// Report getrusage stats.

pub fn getrusage() -> rusage {
    use std::mem::MaybeUninit;
    let usage: rusage = unsafe { MaybeUninit::zeroed().assume_init() };
    usage
}

// Return current memory usage.

pub fn mem_usage_bytes() -> i64 {
    let procfn = "/proc/self/statm".to_string();
    let prob1 = "\nWARNING: mem_usage_bytes( ) failed to access /proc/self/statm.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let f = std::fs::File::open(procfn);
    match f {
        Err(_) => println!("{}{}", prob1, crap),
        Ok(f) => {
            let f = std::io::BufReader::new(f);
            if let Some(line) = f.lines().next() {
                let s = line.unwrap();
                let fields: Vec<&str> = s.split_whitespace().collect();
                // ◼ The page size should not be hardcoded.
                let page_size = 4096_i64;
                return fields[1].force_i64() * page_size;
            }
        }
    };
    -1_i64
}

pub fn mem_usage_gb() -> f64 {
    mem_usage_bytes() as f64 / ((1024 * 1024 * 1024) as f64)
}

// Report the status of all processes having the same owner as 'this' process,
// showing the parent process id, the process id, its memory use in GB (RSS),
// and the command, which is folder.

pub fn ps_me() {
    let uid = unsafe { getuid() } as i64;
    println!(
        "\nPROCESSES HAVING THE SAME OWNER (UID={}) AS THIS PROCESS = {}\n",
        uid,
        id()
    );
    let procs = dir_list("/proc");
    println!("{:>6}  {:>6} {:>7}  CMD", "PPID", "PID", "GB");
    const RIGHT: usize = 60;
    'outer: for i in 0..procs.len() {
        if procs[i].parse::<i64>().is_err() {
            continue;
        }
        let (mut pid, mut ppid) = (-1_i64, -1_i64);
        let mut rss = -1_f64;
        let mut cmd = String::new();
        let f = format!("/proc/{}/status", procs[i]);
        let f = File::open(&f);
        if f.is_err() {
            continue;
        }
        let f = f.unwrap();
        let f = BufReader::new(f);
        for line in f.lines() {
            if line.is_err() {
                continue 'outer;
            } // fail that does occur rarely
            let s = line.unwrap();
            if s.starts_with("Uid:\t") {
                let this_uid = s.after("Uid:\t").before("\t").force_i64();
                if this_uid != uid {
                    continue 'outer;
                }
            } else if s.starts_with("Pid:\t") {
                pid = s.after("Pid:\t").force_i64();
            } else if s.starts_with("PPid:\t") {
                ppid = s.after("PPid:\t").force_i64();
            } else if s.starts_with("VmRSS:\t") {
                let t = s.after("VmRSS:\t");
                let t = t.to_string().replace(" ", "");
                rss = t.before("k").force_f64() / (1024 * 1024) as f64;
            }
        }
        let f = format!("/proc/{}/cmdline", procs[i]);
        let f = File::open(&f);
        if f.is_err() {
            continue;
        }
        let f = f.unwrap();
        let f = BufReader::new(f);
        if let Some(line) = f.lines().next() {
            if line.is_err() {
                continue 'outer;
            } // fail that does occur rarely
            cmd = line.unwrap();
            cmd = cmd.replace(" ", " ");
        }
        if cmd.len() <= RIGHT {
            println!("{:6}  {:6} {:7.2}  {}", ppid, pid, rss, cmd);
        } else {
            let mut start = 0;
            while start < cmd.len() {
                let stop = min(start + RIGHT, cmd.len());
                if start == 0 {
                    let c = &cmd[0..stop];
                    println!("{:6}  {:6} {:7.2}  {}", ppid, pid, rss, c);
                } else {
                    let c = &cmd[start..stop];
                    println!("                        {}", c);
                }
                start += RIGHT;
            }
        }
    }
}
