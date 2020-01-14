// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Computational performance stats.

extern crate io_utils;
extern crate libc;
extern crate string_utils;

use io_utils::*;
use libc::{c_long, getuid, rusage, suseconds_t, time_t, timeval, RLIMIT_NPROC, RUSAGE_SELF};
use libc::{rlimit, setrlimit};
use std::{
    cmp::min,
    fs::File,
    io::{BufRead, BufReader},
    process::*,
    time::Instant,
};
use string_utils::*;

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
    let procfn = format!("/proc/self/status");
    let prob1 = "\nWARNING: nthreads() failed to access /proc status file.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let mut threads: i64 = -1;
    let f = std::fs::File::open(procfn);
    if !f.is_ok() {
        println!("{}{}", prob1, crap);
    } else {
        let f = f.unwrap();
        let f = std::io::BufReader::new(f);
        for line in f.lines() {
            let s = line.unwrap();
            if s.starts_with("Threads:") {
                let mut s = s.after("Threads:").to_string();
                s = s.replace("\t", "").to_string();
                s = s.replace(" ", "").to_string();
                threads = s.force_i64();
                break;
            }
        }
    }
    threads
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

// Report peak memory usage in bytes or gigabytes, as determined by reading
// proc filesystem.

pub fn peak_mem_usage_bytes() -> i64 {
    let procfn = format!("/proc/self/status");
    let prob1 = "\nWARNING: peak_mem_usage_bytes( ) failed to \
                 access /proc status file.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let mut bytes: i64 = -1;
    let f = std::fs::File::open(procfn);
    if !f.is_ok() {
        println!("{}{}", prob1, crap);
    } else {
        let f = f.unwrap();
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
                } else if !split[1].parse::<i64>().is_ok() {
                    printfail("Bad count field.");
                } else {
                    bytes = split[1].force_i64() * 1024;
                }
                break;
            }
        }
    }
    bytes
}

pub fn peak_mem_usage_gb() -> f64 {
    peak_mem_usage_bytes() as f64 / ((1024 * 1024 * 1024) as f64)
}

// Report available memory gigabytes.

pub fn available_mem_gb() -> Option<f64> {
    let procfn = format!("/proc/meminfo");
    // let mut bytes : i64 = -1;
    let f = std::fs::File::open(procfn);
    if !f.is_ok() {
        return None;
    } else {
        let f = f.unwrap();
        let f = std::io::BufReader::new(f);
        for line in f.lines() {
            let s = line.unwrap();
            if s.starts_with("MemAvailable") {
                let split: Vec<&str> = s.split_whitespace().collect();
                if split.len() != 3 || split[2] != "kB" {
                    return None;
                }
                if !split[1].parse::<i64>().is_ok() {
                    return None;
                }
                return Some(split[1].force_f64() / (1024 * 1024) as f64);
            }
        }
    }
    None
}

// Report getrusage stats.

pub fn getrusage() -> rusage {
    let mut usage = rusage {
        ru_utime: timeval {
            tv_sec: 0 as time_t,
            tv_usec: 0 as suseconds_t,
        },
        ru_stime: timeval {
            tv_sec: 0 as time_t,
            tv_usec: 0 as suseconds_t,
        },
        ru_maxrss: 0 as c_long,
        ru_ixrss: 0 as c_long,
        ru_idrss: 0 as c_long,
        ru_isrss: 0 as c_long,
        ru_minflt: 0 as c_long,
        ru_majflt: 0 as c_long,
        ru_nswap: 0 as c_long,
        ru_inblock: 0 as c_long,
        ru_oublock: 0 as c_long,
        ru_msgsnd: 0 as c_long,
        ru_msgrcv: 0 as c_long,
        ru_nsignals: 0 as c_long,
        ru_nvcsw: 0 as c_long,
        ru_nivcsw: 0 as c_long,
    };
    unsafe {
        libc::getrusage(RUSAGE_SELF, (&mut usage) as *mut rusage);
    }
    usage
}

// Return current memory usage.

pub fn mem_usage_bytes() -> i64 {
    let procfn = format!("/proc/self/statm");
    let prob1 = "\nWARNING: mem_usage_bytes( ) failed to access /proc/self/statm.\n";
    let crap = "This suggests something is badly broken in the \
                operating environment.\nContinuing nonetheless, by returning -1.";
    let f = std::fs::File::open(procfn);
    if !f.is_ok() {
        println!("{}{}", prob1, crap);
    } else {
        let f = f.unwrap();
        let f = std::io::BufReader::new(f);
        for line in f.lines() {
            let s = line.unwrap();
            let fields: Vec<&str> = s.split_whitespace().collect();
            // ◼ The page size should not be hardcoded.
            let page_size = 4096 as i64;
            return fields[1].force_i64() * page_size;
        }
    }
    -1 as i64
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
    println!("{:>6}  {:>6} {:>7}  {}", "PPID", "PID", "GB", "CMD");
    const RIGHT: usize = 60;
    'outer: for i in 0..procs.len() {
        if !procs[i].parse::<i64>().is_ok() {
            continue;
        }
        let (mut pid, mut ppid) = (-1 as i64, -1 as i64);
        let mut rss = -1 as f64;
        let mut cmd = String::new();
        let f = format!("/proc/{}/status", procs[i]);
        let f = File::open(&f);
        if !f.is_ok() {
            continue;
        }
        let f = f.unwrap();
        let f = BufReader::new(f);
        for line in f.lines() {
            if !line.is_ok() {
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
        if !f.is_ok() {
            continue;
        }
        let f = f.unwrap();
        let f = BufReader::new(f);
        for line in f.lines() {
            if !line.is_ok() {
                continue 'outer;
            } // fail that does occur rarely
            cmd = line.unwrap();
            cmd = cmd.replace(" ", " ");
            break;
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
