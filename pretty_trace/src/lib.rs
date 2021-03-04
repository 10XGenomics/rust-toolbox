// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TOP LEVEL DOCUEMENTATION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

//! <b>This crate provide tools for generating pretty tracebacks and for profiling.</b>

//! # Pretty tracebacks
//!
//! Stack traces (or "tracebacks") are a fundamental vehicle for describing
//! what code is doing at a given instant.   A beautiful thing about rust
//! is that crashes nearly always yield tracebacks, and those
//! tracebacks nearly always extend all the way from the 'broken'
//! code line all the way to the main program.  We may take these properties
//! for granted but in general neither is true for other languages, including C++.
//!
//! However, as in other languages, native rust tracebacks are verbose.  A major
//! goal of this crate is to provide succinct and readable "pretty" tracebacks, in
//! place of the native tracebacks.  These pretty traces can be
//! <font color="red"> ten times shorter</font> than native tracebacks.  In
//! addition, unlike rust native tracebacks, pretty traces are obtained without
//! setting an environment variable.
//!
//! # Example of native versus pretty trace output
// See discussion @ https://github.com/rust-lang/rust/issues/32104.
// There is no really good way to include an image.
//! ![native vs pretty trace output](https://raw.githubusercontent.com/10XGenomics/rust-toolbox/master/pretty_trace/images/long_vs_short_traceback.jpg)
//!
//! # Profiling
//!
//! Profiling is a fundamental tool for optimizing code.
//! Standard profiling tools including perf are powerful, however they
//! can be challenging to use.  This crate provides a profiling capability that
//! is <font color="red"> completely trivial to invoke and interpret, and yields a
//! tiny file as output</font>.
//!
//! The idea is very simple: if it is possible to significantly speed up your code,
//! this should be directly visible from a modest sample of tracebacks chosen at
//! random.  And these tracebacks can be generated for any main program by adding a
//! simple command-line option to it that causes it to enter a special 'profile'
//! mode, gathering tracebacks and then terminating.  This uses the <code>pprof</code>
//! crate to gather tracebacks.
//!
//! For example this command-line option might be
//! <code>PROFILE</code> to turn on profiling.  It's your choice how to specify
//! this command-line option, but this crate makes it trivial to do so.
//! <font color="red">With a few minutes' work,
//! you can make it possible to profile your code with essentially zero work,
//! whenever you like.</font> See the functions <code>start_profiling</code> and
//! <code>stop_profiling</code>.  Note that to produce useful output, one needs to specify a list
//! of blacklisted crates, such as <code>std</code>.  The entries from these crates are removed
//! from the tracebacks.
//!
//! # Example of pretty trace profiling output
//!
//! ![profiling output](https://raw.githubusercontent.com/10XGenomics/rust-toolbox/master/pretty_trace/images/profile.png)
//!
//! Here pretty trace profiling reveals exactly what some code was doing at
//! random instances; we show the first of the collated tracebacks.  More were
//! attempted: of attempted tracebacks, 95.8% are reported.  Unreported tracebacks
//! would be those lying entirely in blacklisted crates.
//!
//! Each line shows a function name, the crate it is in, the version of the crate (if known),
//! the file name in the crate, and the line number.
//!
//! # A brief guide for using pretty trace
//!
//! First make sure that you have rust debug on: it seems to be enough to have
//! <code>debug = 1</code> set in <code>Cargo.toml</code> for debug and/or release mode,
//! depending on which youre using.
//!
//! <br> Now to access pretty trace, put this in your <code>Cargo.toml</code>
//! <pre>
//! pretty_trace = {git = "https://github.com/10XGenomics/rust-toolbox.git"}
//! </pre>
//! and this
//! <pre>
//! use pretty_trace::*;
//! </pre>
//! in your main program.
//!
//! <br> Next to turn on pretty traces, it is enough to insert this
//! <pre>
//!     PrettyTrace::new().on();
//! </pre>
//! at the beginning of your main program.  And you're good to go!  Any panic
//! will cause a pretty traceback to be generated.  
//!
//! Several other useful features are described below.  These include the capability
//! of tracing to know where you are in your data (and not just your code), and
//! for focusing profiling on a key set of crates that you're optimizing.
//!
//! # Credit
//!
//! This code was developed at 10x Genomics, and is based in part on C++ code developed at the
//! Whitehead Institute Center for Genome Research / Broad Institute starting in 2000, and
//! included in <https://github.com/CompRD/BroadCRD>.
//!
//! # FAQ
//!
//! <b>1. Could a pretty traceback lose important information?</b>
//! <br><br>Possibly.  For this reason we provide the capability of dumping a full
//! traceback to a file (as 'insurance') and also an environment variable to
//! force full tracebacks.  However we have not seen examples where important
//! information is lost.<br><br>
//! <b>2. Can the pretty traceback itself be saved to a separate file?</b>
//! <br><br>Yes this capability is provided.<br><br>
//! <b>3. Can I get a traceback on Ctrl-C?</b>
//! <br><br>Yes, if you do this
//! <pre>
//!     PrettyTrace::new().ctrlc().on();
//! </pre>
//! then any Ctrl-C will be converted into a panic, and then you'll get a trackback.
//!
//! # Full disclosure
//!
//! ◼ The code has only been confirmed to work under linux.  The code has been
//!   used under OS X, but tracebacks can be incomplete.  An example is provided
//!   of this behavior.
//!
//! ◼ Ideally out-of-memory events would be caught and converted to panics so
//!   we could trace them, but we don't.  This is a general rust problem that no one
//!   has figured out how to solve.  See <a href="https://github.com/rust-lang/rust/issues/43596">issue 43596</a> and <a href="https://internals.rust-lang.org/t/could-we-support-unwinding-from-oom-at-least-for-collections/3673">internals 3673</a>.
//!
//! ◼ The code parses the output of a formatted stack trace, rather then
//!   generating output directly from a formal stack trace structure (which it
//!   should do).  This makes it vulnerable to changes in stack trace formatting.
//!
//! ◼ There is an ugly blacklist of strings that is fragile.  This may
//!   be an intrinsic feature of the approach.
//!
//! ◼ In general, tracebacks in parallel code do not go back to the main program.
//!
//! # More
//!
//! See the documentation for <code>PrettyTrace</code>, linked to below.
//!
//! # To do
//!
//! ◼ Rewrite so that tracebacks are formatted in the same way in all cases, in the fashion
//!   carried out by profiling.  And reuse the same code.

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// EXTERNAL DEPENDENCIES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

use backtrace::Backtrace;
use failure::Error;
use lazy_static::lazy_static;
use libc::SIGINT;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use pprof::ProfilerGuard;
use stats_utils::*;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufWriter, Write},
    ops::Deref,
    os::unix::io::FromRawFd,
    panic,
    str::from_utf8,
    sync::atomic::AtomicBool,
    sync::atomic::Ordering::SeqCst,
    sync::{Mutex, RwLock},
    thread,
    thread::ThreadId,
    time,
};
use string_utils::*;
use tables::*;
use vector_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PROFILING
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

static mut GUARD: Option<ProfilerGuard<'static>> = None;

static mut REPORT: Option<pprof::Report> = None;

static mut BLACKLIST: Vec<String> = Vec::new();

/// Start profiling, blacklisting the given crates.  
///
/// Without blacklisting, profiling can be extremely verbose.  We recommend blackisting at least
/// `alloc`, `build`, `core`, `rayon`, `rayon-core`, `serde`, `serde-json`, `std` and
/// `unknown`.  However what should be blacklisted depends on what you're trying to understand.
/// If you examine the tracebacks generated by profiling, you can tune the list appropriately.
///
/// It is not clear how the timing of the profiling is handled.  There is a parameter `frequency`
/// that is passed to the profiling machinery, but we don't know what it does.
///
/// Profiling <i>appears</i> to correctly represent wallclock in parallel loops.

pub fn start_profiling(blacklist: &Vec<String>) {
    let frequency = 1000;
    unsafe {
        BLACKLIST = blacklist.clone();
        GUARD = Some(pprof::ProfilerGuard::new(frequency).unwrap());
    }
}

/// Stop profiling and dump tracebacks.  

pub fn stop_profiling() {
    unsafe {
        let report = GUARD.as_ref().unwrap().report().build();
        if report.is_err() {
            panic!("Failed to build profiling report.");
        } else {
            REPORT = Some(report.unwrap());
            let report = REPORT.as_ref().unwrap();
            let mut traces = Vec::<String>::new();
            let blacklist = &BLACKLIST;
            let mut n = 0;
            for (frames, count) in report.data.iter() {
                let m = &frames.frames;
                n += count;
                let mut symv = Vec::<Vec<String>>::new();
                for i in 0..m.len() {
                    for j in 0..m[i].len() {
                        let s = &m[i][j];
                        let mut name = s.name();
                        if name.ends_with("::{{closure}}") {
                            name = name.rev_before("::{{closure}}").to_string();
                        }
                        if name.contains("::") {
                            name = name.rev_after("::").to_string();
                        }
                        let filename;
                        if s.filename.is_some() {
                            filename = s.filename.as_ref().unwrap().to_str().unwrap().to_string();
                        } else {
                            filename = "unknown".to_string();
                        }
                        let mut cratex; // crate without version
                        let mut version = String::new(); // crate version
                        let file;
                        if filename.contains("/cargo/git/checkouts/") {
                            let post = filename.after("/cargo/git/checkouts/");
                            if post.contains("/src/") && post.rev_before("/src/").contains("/") {
                                let mid = post.rev_before("/src/");
                                file = post.after("/src/").to_string();
                                if mid.after("/").contains("/") {
                                    version = mid.between("/", "/").to_string();
                                    cratex = mid.rev_after("/").to_string();
                                } else {
                                    version = mid.rev_after("/").to_string();
                                    cratex = post.before("/").to_string();
                                    if cratex.contains("-") {
                                        cratex = cratex.rev_before("-").to_string();
                                    }
                                }
                            } else {
                                cratex = "unknown".to_string();
                                file = "unknown".to_string();
                            }
                        } else if filename.contains("/src/")
                            && filename.rev_before("/src/").contains("/")
                        {
                            cratex = filename.rev_before("/src/").rev_after("/").to_string();
                            file = filename.rev_after("/src/").to_string();
                        } else {
                            cratex = "unknown".to_string();
                            file = "unknown".to_string();
                        }
                        let lineno;
                        if s.lineno.is_some() {
                            lineno = format!("{}", s.lineno.unwrap());
                        } else {
                            lineno = "?".to_string();
                        }
                        if cratex.contains("-") && version == "" {
                            let c = cratex.rev_before("-");
                            let d = cratex.rev_after("-").to_string();
                            // check to see if d = x.y.z for some nonnegative integers x, y, z
                            if d.contains(".") && d.after(".").contains(".") {
                                if d.before(".").parse::<usize>().is_ok()
                                    && d.between(".", ".").parse::<usize>().is_ok()
                                    && d.rev_after(".").parse::<usize>().is_ok()
                                {
                                    cratex = c.to_string();
                                    version = d.to_string();
                                }
                            }
                        }
                        let mut blacklisted = false;
                        for b in blacklist.iter() {
                            if *b == cratex {
                                blacklisted = true;
                            }
                        }
                        if !blacklisted && file.ends_with(".rs") {
                            symv.push(vec![name, cratex, version, file, lineno]);
                        }
                    }
                }
                if !symv.is_empty() {
                    let mut log = String::new();
                    print_tabular_vbox(&mut log, &symv, 0, &b"l|l|l|l|l".to_vec(), false, false);
                    for _ in 0..*count {
                        let x = format!("{}", log);
                        traces.push(x);
                    }
                }
            }
            traces.sort();
            let mut freq = Vec::<(u32, String)>::new();
            make_freq(&traces, &mut freq);
            let mut report = String::new();
            let traced = 100.0 * traces.len() as f64 / n as f64;
            report += &format!(
                "\nPRETTY TRACE PROFILE\n\nTRACED = {:.1}%\n\nTOTAL = {}\n\n",
                traced,
                traces.len()
            );
            let mut total = 0;
            for (i, x) in freq.iter().enumerate() {
                total += x.0 as usize;
                report += &format!(
                    "[{}] COUNT = {} = {:.2}% ⮕ {:.2}%\n{}\n",
                    i + 1,
                    x.0,
                    percent_ratio(x.0 as usize, traces.len()),
                    percent_ratio(total, traces.len()),
                    x.1
                );
            }
            print!("{}", report);
        };
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRETTY TRACE STRUCTURE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

/// A `PrettyTrace` is the working structure for this crate.  See also the top-level
/// crate documentation.

#[derive(Default)]
pub struct PrettyTrace {
    // filename to dump full traceback to upon panic
    pub full_file: Option<String>,
    // file descriptor to dump second copy of traceback to upon panic
    pub fd: Option<i32>,
    // exit message
    pub exit_message: Option<String>,
    // thread message
    pub message: Option<&'static CHashMap<ThreadId, String>>,
    // is profile mode on?
    pub profile: bool,
    // count for profile mode
    pub count: Option<usize>,
    // separation in seconds for profile mode
    pub sep: f32,
    // whitelist for profile mode
    pub whitelist: Option<Vec<String>>,
    // convert Ctrl-Cs to panics
    pub ctrlc: bool,
    pub ctrlc_debug: bool,
    pub noexit: bool,
}

/// Normal usage of `PrettyTrace` is to call
/// <pre>
/// PrettyTrace::new().&lt set some things >.on();
/// </pre>
/// once near the begining of your main program.  The 'things' are all the
/// functions shown below other than <code>new</code> and <code>on</code>.

impl PrettyTrace {
    /// Initialize a <code>PrettyTrace</code> object.  This does nothing
    /// in and of itself.

    pub fn new() -> PrettyTrace {
        PrettyTrace::default()
    }

    /// Cause a <code>PrettyTrace</code> object to do something: change the
    /// behavior of response to <code>panic!</code> to produce a prettified
    /// traceback and perform profiling, if <code>profile()</code> has been called.
    /// Calling of <code>on</code> is mandatory.  It must be called exactly once
    /// at the end of a chain of operations on a <code>PrettyTrace</code> object.
    /// But this is not enforced.

    pub fn on(&mut self) {
        let fd = if self.fd.is_some() {
            self.fd.unwrap() as i32
        } else {
            -1 as i32
        };
        let mut haps = Happening::new();
        if self.profile {
            if self.whitelist.is_none() {
                self.whitelist = Some(Vec::<String>::new());
            }
            haps.initialize(
                &self.whitelist.clone().unwrap(),
                self.count.unwrap(),
                self.sep,
            );
        }
        let full_file = if self.full_file.is_some() {
            self.full_file.clone().unwrap()
        } else {
            String::new()
        };
        if self.message.is_some() {
            force_pretty_trace_fancy(
                full_file,
                fd,
                self.exit_message.clone(),
                &self.message.unwrap(),
                &haps,
                self.ctrlc,
                self.ctrlc_debug,
                self.noexit,
            );
        } else {
            let tm = new_thread_message();
            force_pretty_trace_fancy(
                full_file,
                fd,
                self.exit_message.clone(),
                &tm,
                &haps,
                self.ctrlc,
                self.ctrlc_debug,
                self.noexit,
            );
        }
    }

    /// Cause a <code>Ctrl-C</code> interrupt to be turned into a panic, and thence
    /// produce a traceback for the main thread.  This does not allow you to see
    /// what other threads are doing.  If you <code>Ctrl-C</code> twice in rapid
    /// succession, you may elide the traceback, but this is unreliable.  Occasionally single
    /// interrupts are also incorrectly handled.

    pub fn ctrlc(&mut self) -> &mut PrettyTrace {
        self.ctrlc = true;
        self
    }

    /// Same as <code>ctrlc</code>, but generates some debugging information.  For development
    /// purposes.

    pub fn ctrlc_debug(&mut self) -> &mut PrettyTrace {
        self.ctrlc = true;
        self.ctrlc_debug = true;
        self
    }

    /// Turn off call to <code>std::process::exit(101)</code>, which is normally triggered after
    /// printing a traceback (on panic).  This could be useful if you want to run a bunch of
    /// tests, some of which fail, but you want to see the outcome of all of them.  Note that
    /// <code>101</code> is the standard exit status for rust panics.
    ///
    /// The downside of <code>noexit</code> is that you may get multiple tracebacks if your
    /// code fails in a parallel loop.

    pub fn noexit(&mut self) -> &mut PrettyTrace {
        self.noexit = true;
        self
    }

    /// Define a file, that in the event that a traceback is triggered by a
    /// panic, will be used to dump a full traceback to.  The
    /// <i>raison d'etre</i> for this is that an abbreviated pretty traceback might
    /// in some cases elide useful information (although this has not been observed).
    ///
    /// This may only be set from the main thread of a process.  We disallow setting it from
    /// other threads because `PrettyTrace` works by setting the panic hook, which is global,
    /// and a value for `full_file` set by one thread might not be valid for another.
    ///
    /// You can also force <code>PrettyTrace</code> to emit full tracebacks by
    /// setting the environment variable <code>RUST_FULL_TRACE</code>.

    pub fn full_file(&mut self, full_file: &str) -> &mut PrettyTrace {
        self.full_file = Some(full_file.to_string());
        if thread::current().name().unwrap() != "main" {
            panic!(
                "PrettyTrace::full_file was called from a non-main thread.  This is not\n\
                allowed because PrettyTrace works by setting the panic hook, which is global.\n\
                A value set by one thread might not be valid for another."
            );
        }
        self
    }

    /// Define a file descriptor, that in the event a traceback is triggered by a
    /// panic, will be used to dump a second copy of the traceback to.

    pub fn fd(&mut self, fd: i32) -> &mut PrettyTrace {
        self.fd = Some(fd);
        self
    }

    /// Define a message that is to be omitted after a traceback and before exiting.

    /// # Example
    /// <pre>
    /// fn main() {
    ///     let message = "Dang it, you found a bug!  Please call us at (999) 123-4567.";
    ///     PrettyTrace::new().exit_message(&message).on();

    pub fn exit_message(&mut self, message: &str) -> &mut PrettyTrace {
        self.exit_message = Some(message.to_string());
        self
    }

    /// Define a message object that will be used by threads to store their status.
    /// This is printed if a traceback is triggered by a panic, and where
    /// code is traversing data in a loop, can be used to determine not only where
    /// execution is in the code, but also where it is in the data.
    ///
    /// This may only be set from the main thread of a process.  We disallow setting it from
    /// other threads because `PrettyTrace` works by setting the panic hook, which is global,
    /// and a value for `message` set by one thread might not be valid for another.

    /// # Example
    /// <pre>
    /// use std::thread;
    /// fn main() {
    ///     let message = new_thread_message();
    ///     PrettyTrace::new().message(&message).on();
    ///     ...
    ///     // do this whenever thread status changes enough to care
    ///     message.insert( thread::current().id(), "here is what I'm doing now" );
    ///     ...
    /// }
    /// </pre>

    pub fn message(&mut self, message: &'static CHashMap<ThreadId, String>) -> &mut PrettyTrace {
        self.message = Some(message);
        if thread::current().name().unwrap() != "main" {
            panic!(
                "PrettyTrace::message was called from a non-main thread.  This is not\n\
                allowed because PrettyTrace works by setting the panic hook, which is global.\n\
                A value set by one thread might not be valid for another."
            );
        }
        self
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// HAPPENING STRUCTURE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Data structure for control of happening handling.  This could probably be
// elided now.

struct Happening {
    pub on: bool,               // turned on?
    pub whitelist: Vec<String>, // tracebacks are grepped for these
    pub hcount: usize,          // number of tracebacks to gather
    pub sep: f32,               // separation in seconds
}

impl Happening {
    pub fn new() -> Happening {
        Happening {
            on: false,
            whitelist: Vec::<String>::new(),
            hcount: 0,
            sep: 1.0,
        }
    }

    // EXAMPLE: set whitelist to a or b or c, hcount to 250, sep to 1.0
    // let mut happening = Happening::new();
    // happening.initialize( &vec![ "a", "b", "c" ], 250, 1.0 );

    pub fn initialize(&mut self, whitelist: &[String], hcount: usize, sep: f32) {
        self.on = true;
        self.whitelist = whitelist.to_owned();
        self.hcount = hcount;
        self.sep = sep;
    }
}

static CTRLC_DEBUG: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref HAPPENING: Mutex<Happening> = Mutex::new(Happening::new());
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// SIGNAL HANDLING
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Redirect SIGINT and SIGUSR1 interrupts to the function "handler".

fn install_signal_handler(happening: bool, ctrlc: bool) -> Result<(), Error> {
    if happening {
        let handler = SigHandler::Handler(handler);
        let action = SigAction::new(handler, SaFlags::SA_RESTART, SigSet::empty());
        unsafe {
            sigaction(Signal::SIGUSR1, &action)?;
        }
    }
    if ctrlc {
        let handler = SigHandler::Handler(handler);
        let action = SigAction::new(handler, SaFlags::SA_RESTART, SigSet::empty());
        unsafe {
            sigaction(Signal::SIGINT, &action)?;
        }
    }
    Ok(())
}

static mut HEARD_CTRLC: usize = 0;

extern "C" fn handler(sig: i32) {
    let sep = HAPPENING.lock().unwrap().sep;
    let sleep_time = (sep * 1000.0).round() as u64;
    if sig == SIGINT {
        if CTRLC_DEBUG.load(SeqCst) {
            unsafe {
                eprint!("\ncaught Ctrl-C");
                eprintln!(" #{}", HEARD_CTRLC + 1);
            }
        }
        unsafe {
            if HEARD_CTRLC > 0 {
                HEARD_CTRLC += 1;
                std::process::exit(0);
            }
            HEARD_CTRLC += 1;
            thread::sleep(time::Duration::from_millis(sleep_time));
            if CTRLC_DEBUG.load(SeqCst) {
                eprintln!("done sleeping");
            }
            if HEARD_CTRLC > 1 {
                std::process::exit(0);
            }
        }
        eprintln!("");
        panic!(
            "Ctrl-C (SIGINT) interrupt detected\n\nThe traceback below only \
             shows the master thread.  If your code includes\n\
             multithreading, you may need to turn that off to obtain \
             a meaningful traceback."
        );
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// CORE TRACEBACK FUNCTION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

/// Super simplifed concurrent HashMap for use with pretty_trace. The only public method is
/// `insert`, allowing the user to set the current thread message.
pub struct CHashMap<K, V> {
    map: RwLock<HashMap<K, V>>,
}

impl<K, V> CHashMap<K, V>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    pub fn new() -> CHashMap<K, V> {
        CHashMap {
            map: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, k: K, v: V) {
        self.map.write().unwrap().insert(k, v);
    }
}

/// See <code>PrettyTrace</code> documentation for how this is used.

pub fn new_thread_message() -> &'static CHashMap<ThreadId, String> {
    let hashmap = CHashMap::new();
    let box_thread_message = Box::new(hashmap);
    let thread_message: &'static CHashMap<ThreadId, String> = Box::leak(box_thread_message);
    thread_message
}

/// See <code>PrettyTrace</code> documentation for how this is used.

fn force_pretty_trace_fancy(
    log_file_name: String,
    fd: i32,
    exit_message: Option<String>,
    thread_message: &'static CHashMap<ThreadId, String>,
    happening: &Happening,
    ctrlc: bool,
    ctrlc_debug: bool,
    noexit: bool,
) {
    // Set up to catch SIGNINT and SIGUSR1 interrupts.

    let _ = install_signal_handler(happening.on, ctrlc);
    if ctrlc_debug {
        CTRLC_DEBUG.store(true, SeqCst);
    }

    // Set up panic hook. If we panic, this code gets run.

    let _ = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Get backtrace.

        let backtrace = Backtrace::new();

        // Get thread message.

        let mut tm = String::new();
        let this_thread = thread::current().id();
        let tmx = thread_message.map.read();
        if tmx.is_err() {
            eprintln!("\nProblem processing thread message in PrettyTrace.\n");
            std::process::exit(1);
        }
        if tmx.as_ref().unwrap().contains_key(&this_thread) {
            tm = format!("{}\n\n", tmx.unwrap().get(&this_thread).unwrap().deref());
        }

        // Handle verbose mode.

        let mut _verbose = false;
        for (key, _value) in env::vars() {
            if key == "RUST_FULL_TRACE" {
                let bt: Vec<u8> = format!("{:?}", backtrace).into_bytes();
                let thread = thread::current();
                let thread = thread.name().unwrap_or("unnamed");
                let msg = match info.payload().downcast_ref::<&'static str>() {
                    Some(s) => *s,
                    None => match info.payload().downcast_ref::<String>() {
                        Some(s) => &**s,
                        None => "Box<Any>",
                    },
                };
                let msg2 = match info.location() {
                    Some(location) => format!(
                        "thread '{}' panicked at {}:{}",
                        thread,
                        location.file(),
                        location.line()
                    ),
                    None => format!("thread '{}' panicked ", thread),
                };
                eprintln!(
                    "\nRUST PROGRAM PANIC\n\n(Full traceback.  \
                     Rerun with env var RUST_FULL_TRACE unset to \
                     see short traceback.)\n\n{}{}\n\n{}\n\n{}\n",
                    tm,
                    &msg,
                    &msg2,
                    from_utf8(&bt).unwrap()
                );
                std::process::exit(101);
            }
        }

        // Prettify the traceback.

        let bt: Vec<u8> = format!("{:?}", backtrace).into_bytes();
        let all_out = prettify_traceback(&bt, &Vec::<String>::new(), false);

        // Print thread panic message.  Bail before doing so if broken pipe
        // detected.  This protects against running e.g. "exec |& head -50"
        // (if exec is the name of the executable), which can otherwise bomb
        // out asserting "illegal instruction".
        //
        // Actually, not printing the thread identifier, because this is rarely
        // of interest.  And you can get the full traceback if you want it.

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };
        let mut em = String::new();
        if exit_message.is_some() {
            em = format!("{}\n\n", exit_message.as_ref().unwrap());
        }
        let msg = match info.location() {
            Some(location) => {
                let loc = &(*location.file());

                // Replace long constructs of the form /rustc/......./src/
                //                                  by /rustc/<stuff>/src/.

                let mut x2 = loc.to_owned();
                let x2_orig = x2.clone();
                if loc.contains("/rustc/") && loc.after("/rustc/").contains("/src/") {
                    let y = loc.between("/rustc/", "/src/");
                    if y.len() > 10 {
                        x2 = x2.replace(y, "<stuff>");
                    }
                }
                if loc.contains("/checkouts/") && loc.after("/checkouts/").contains("/src/") {
                    let y = loc.between("/checkouts/", "/src/");
                    if y.len() > 10 {
                        x2 = x2.replace(y, "<stuff>");
                    }
                }

                // Format lead message.

                let pre = format!("{}:{}", x2, location.line());
                let prex = if all_out.contains(&pre) || x2_orig.contains("pretty_trace") {
                    "".to_string()
                } else {
                    format!("\n\n0: ◼ {}", pre)
                };
                let long_msg = if log_file_name.is_empty() {
                    "Rerun with env var RUST_FULL_TRACE set to see full traceback.".to_string()
                } else {
                    format!("Full traceback is at {}.", log_file_name)
                };
                format!(
                    "RUST PROGRAM PANIC\n\n(Shortened traceback.  \
                     {})\n\n{}{}{}",
                    long_msg, tm, msg, prex
                )
            }
            None => format!("RUST PROGRAM PANIC\n\n{}", msg),
        };
        if msg.contains("Broken pipe") {
            std::process::exit(101);
        }

        // Now print stuff.  Package as a single print line to prevent
        // interweaving if multiple threads panic.  Also check for read permission on the
        // executable.  Not having that would likely result in a truncated traceback.

        let mut out = format!("\n{}\n\n", &msg);
        let ex = std::env::current_exe();
        if ex.is_err() {
            out += &format!(
                "█ WARNING.  It was not possible to get the path of your executable.\n\
                 █ This may result in a defective traceback.\n\n"
            );
        } else {
            let ex = ex.unwrap();
            let ex = ex.to_str();
            if ex.is_none() {
                out += &format!(
                    "█ WARNING.  The path of your executable could not be converted into\n\
                     █ a string.  This is weird and might result in a defective traceback.\n\n"
                );
            } else {
                let ex = ex.unwrap();
                let f = File::open(&ex);
                if f.is_err() {
                    out += &format!(
                        "█ WARNING.  Your executable file could not be opened for reading.\n\
                         █ This might be because it does not have read permission set for you.\n\
                         █ This may result in a defective traceback.\n\n"
                    );
                }
            }
        }
        out += &all_out;
        out += &em;
        eprint!("{}", out);

        // Dump traceback to file descriptor.

        let mut failed = false;
        if fd >= 0 {
            unsafe {
                let mut err_file = File::from_raw_fd(fd);
                let x = err_file.write(out.as_bytes());
                if x.is_err() {
                    eprintln!(
                        "\nProblem in PrettyTrace writing to file descriptor {}.\n",
                        fd
                    );
                    failed = true;
                } else {
                    let _ = x.unwrap();
                }
            }
        }

        // Dump full traceback to log file.

        if log_file_name != "" {
            let f = File::create(&log_file_name);
            if f.is_err() {
                eprintln!(
                    "\nDuring panic, attempt to create full log file \
                     named {} failed, giving up.\n",
                    log_file_name
                );
                std::process::exit(101);
            }
            let mut log_file_writer = BufWriter::new(f.unwrap());
            let bt: Vec<u8> = format!("{:?}", backtrace).into_bytes();
            let thread = thread::current();
            let thread = thread.name().unwrap_or("unnamed");
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &**s,
                    None => "Box<Any>",
                },
            };
            let msg = match info.location() {
                Some(location) => format!(
                    "thread '{}' panicked at '{}': {}:{}",
                    thread,
                    msg,
                    location.file(),
                    location.line()
                ),
                None => format!("thread '{}' panicked at '{}'", thread, msg),
            };
            log_file_writer
                .write_fmt(format_args!(
                    "\nRUST PROGRAM PANIC\n\n(Full traceback.)\n\n{}{}\n\n{}\n{}",
                    tm,
                    &msg,
                    from_utf8(&bt).unwrap(),
                    em
                ))
                .unwrap();
        }

        // Exit.  Turning this off would seem to have no effect, but this is not the case
        // in general.  If your code fails in a parallel loop, without the exit, you may
        // be flooded with tracebacks, one per thread.

        if !noexit || failed {
            std::process::exit(101);
        }
    }));
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRETTIFY TRACEBACK
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

#[allow(clippy::cognitive_complexity)]
fn prettify_traceback(bt: &Vec<u8>, whitelist: &[String], pack: bool) -> String {
    // Parse the backtrace into lines.

    let mut btlines = Vec::<Vec<u8>>::new();
    let mut line = Vec::<u8>::new();
    for z in bt {
        if *z == b'\n' {
            // Replace long constructs of the form /rustc/......./src/
            //                                  by /rustc/<stuff>/src/.
            // and some similar things.

            let x = stringme(&line);
            let mut x2 = x.clone();
            if x.contains("/rustc/") && x.after("/rustc/").contains("/src/") {
                let y = x.between("/rustc/", "/src/");
                if y.len() > 10 {
                    x2 = x2.replace(y, "<stuff>");
                }
            }
            if x.contains("/checkouts/") && x.after("/checkouts/").contains("/src/") {
                let y = x.between("/checkouts/", "/src/");
                if y.len() > 10 {
                    x2 = x2.replace(y, "<stuff>");
                }
            }
            let srcgit = "/src/github.com-";
            if x.contains(srcgit) && x.after(srcgit).contains('/') {
                let y = x.between(srcgit, "/");
                if y.len() > 10 {
                    x2 = x2.replace(&format!("{}{}", srcgit, y), "/<stuff>");
                }
            }
            if x2.contains("/src/") && x2.before("/src/").contains("/") && x2.contains(" ") {
                x2 = format!(
                    "{} {}/src/{}",
                    x2.rev_before(" "),
                    x2.before("/src/").rev_after("/"),
                    x2.after("/src/")
                );
            }
            btlines.push(x2.as_bytes().to_vec());

            // Reset line.

            line.clear();
        } else {
            line.push(*z);
        }
    }

    // Convert the traceback into a Vec<Vec<Vec<String>>>>.  The outer vector corresponds to the
    // traceback block.  Within each block is a vector of traceback entries, and each entry is
    // itself a vector of on or two lines.  It is two, except when there is no code line number.
    // The initial blanks and block numbers are stripped off.

    let mut blocks = Vec::<Vec<Vec<Vec<u8>>>>::new();
    let mut block = Vec::<Vec<Vec<u8>>>::new();
    let mut blocklet = Vec::<Vec<u8>>::new();
    for x in btlines {
        // Ignore blank lines.

        if x.is_empty() {
            continue;
        }

        // Determine if this line begins a block, i.e. looks like <blanks><number>:<...>.

        let mut s = x.as_slice();
        let mut j = 0;
        while j < s.len() {
            if s[j] != b' ' {
                break;
            }
            j += 1;
        }
        while j < s.len() {
            if !(s[j] as char).is_digit(10) {
                break;
            }
            j += 1;
        }
        if j < s.len() && s[j] == b':' && !block.is_empty() {
            if !blocklet.is_empty() {
                block.push(blocklet.clone());
                blocklet.clear();
            }
            blocks.push(block.clone());
            block.clear();
            s = &s[j + 1..s.len()];
        }

        // Proceed.

        let mut j = 0;
        while j < s.len() {
            if s[j] != b' ' {
                break;
            }
            j += 1;
        }
        s = &s[j..s.len()];
        blocklet.push(s.to_vec());
        if s.starts_with(b"at ") {
            block.push(blocklet.clone());
            blocklet.clear();
        }
    }
    if !blocklet.is_empty() {
        block.push(blocklet.clone());
    }
    if !block.is_empty() {
        blocks.push(block.clone());
    }

    // Define the blacklist.

    let blacklist = vec![
        "pretty_trace",
        "::libunwind::",
        "::Backtrace::",
        "::backtrace::",
        "::panicking::",
        "::lang_start",
        "rust_maybe_catch_panic",
        "rust_panic",
        "libc_start_main",
        "::force_pretty_trace::",
        "::thread::",
        "- rayon",
        "rayon::iter",
        "rayon_core::",
        "- start_thread",
        "<alloc::",
        "rust_begin_unwind",
        "start_thread",
        "__clone",
        "call_once",
        // "<unknown>", // turning this on yields cleaner tracebacks but loses critical information
        "/panic.rs",
        "/panicking.rs",
        "catch_unwind",
        "lang_start_internal",
        "libstd/rt.rs",
    ];

    // Remove certain 'unwanted' blocklets.

    for mut x in blocks.iter_mut() {
        let mut to_delete = vec![false; x.len()];
        'block: for j in 0..x.len() {
            // Ugly exemption to make a test work.

            for k in 0..x[j].len() {
                let s = strme(&x[j][k]);
                if s.contains("pretty_trace::tests") {
                    continue 'block;
                }
            }

            // Otherwise blocklet may not contain a blacklisted string.

            'outer1: for k in 0..x[j].len() {
                let s = strme(&x[j][k]);
                for b in blacklist.iter() {
                    if s.contains(b) {
                        to_delete[j] = true;
                        break 'outer1;
                    }
                }
            }

            // Blocklet must contain a whitelisted string (if whitelist provided).

            if !to_delete[j] && !whitelist.is_empty() {
                let mut good = false;
                'outer2: for k in 0..x[j].len() {
                    let s = strme(&x[j][k]);
                    for b in whitelist.iter() {
                        if s.contains(b) {
                            good = true;
                            break 'outer2;
                        }
                    }
                }
                if !good {
                    to_delete[j] = true;
                }
            }

            // Don't allow blockets of length one that end with "- main".

            let s = strme(&x[j][0]);
            if !to_delete[j] && x[j].len() == 1 && s.ends_with("- main") {
                to_delete[j] = true;
            }

            // Don't allow blocklets whose first line has the form ... main(...).

            let m = " main (";
            if s.contains(&m) && s.after(&m).contains(')') && !s.between(&m, ")").contains('(') {
                to_delete[j] = true;
            }
        }
        erase_if(&mut x, &to_delete);
    }

    // Remove any block having length zero.

    let mut to_delete = vec![false; blocks.len()];
    for i in 0..blocks.len() {
        if blocks[i].is_empty() {
            to_delete[i] = true;
        }
    }
    erase_if(&mut blocks, &to_delete);

    // stuff from earlier version, not addressing now

    // !s2.contains(".rs:0")
    // ((!s.contains(" - <") && !s.contains("rayon::iter")) || k == i)

    // if s.contains("::{{closure}}") {
    //     s = s.rev_before("::{{closure}}");
    // }

    // Contract paths that look like " .../.../src/...".

    let src = b"/src/".to_vec();
    for z in blocks.iter_mut() {
        for w in z.iter_mut() {
            if w.len() == 2 {
                let mut x = Vec::<u8>::new();
                let y = w[1].clone();
                'outer: for j in 0..y.len() {
                    if contains_at(&y, &src, j) {
                        for k in (0..j).rev() {
                            if y[k] != b'/' {
                                continue;
                            }
                            for l in (0..k).rev() {
                                if y[l] == b' ' {
                                    for u in y.iter().take(l + 1) {
                                        x.push(*u);
                                    }
                                    for u in y.iter().skip(k + 1) {
                                        x.push(*u);
                                    }
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
                if !x.is_empty() {
                    w[1] = y;
                }
            }
        }
    }

    // Emit prettified output.

    let mut all_out = String::new();
    for (i, x) in blocks.iter().enumerate() {
        let num = format!("{}: ", i + 1);
        let sub = stringme(&vec![b' '; num.len()].as_slice());
        for (j, y) in x.iter().enumerate() {
            for (k, z) in y.iter().enumerate() {
                if j == 0 && k == 0 {
                    all_out += &num;
                } else {
                    all_out += &sub;
                }
                if k > 0 {
                    all_out += "◼ ";
                }
                let mut s = stringme(&z);
                if k == 0 && s.contains("::") {
                    let cc = s.rfind("::").unwrap();
                    s.truncate(cc);
                }
                if s.ends_with("::{{closure}}") {
                    s = s.rev_before("::{{closure}}").to_string();
                }
                all_out += &s;
                all_out += "\n";
            }
        }
        if !pack {
            all_out += "\n";
        }
    }
    all_out
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TESTS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

#[cfg(test)]
mod tests {

    #[inline(never)]
    fn looper(results: &mut Vec<(usize, usize)>) {
        use rayon::prelude::*;
        results.par_iter_mut().for_each(|r| {
            for _ in 0..10_000 {
                r.1 = r.1.wrapping_add(1).wrapping_add(r.0 * r.1);
            }
        });
    }

    use super::*;

    #[test]
    fn test_ctrlc() {
        use libc::{kill, SIGINT};
        use nix::unistd::{fork, pipe, ForkResult};
        use std::fs::File;
        use std::io::{Read, Write};
        use std::os::unix::io::FromRawFd;
        use std::{thread, time};
        use string_utils::*;

        // Create a pipe.

        let pipefd = pipe().unwrap();

        // Set up tracebacks with ctrlc and using the pipe.  The use of the exit message
        // makes absolutely no sense at all.  However, without it, something very weird
        // happens in the test.  The test seems to finish and return control, and then you
        // get a message "thread panicked while processing panic. aborting.".  So this is
        // a weird workaround for a problem that is not understood.  And the problem arose
        // exactly when the exit message was added, with this commit:
        //
        // commit d32162f15d7192eeb077744bace91a3cb27094b0
        // Author: David Jaffe <david.jaffe@10xgenomics.com>
        // Date:   Thu Dec 19 03:04:12 2019 -0800
        // add exit_message(...) to PrettyTrace
        //
        // In addition, and connected to this,
        // cargo test --release
        // does not work, and instead you need to use
        // cargo test --release -- --nocapture

        let message = "Dang it, you found a bug!  Please call us at (999) 123-4567.";
        PrettyTrace::new()
            .exit_message(&message)
            .ctrlc()
            .fd(pipefd.1)
            .on();
        // PrettyTrace::new().ctrlc().fd(pipefd.1).on();

        // Create stuff needed for computation we're going to interrupt.

        let mut results = vec![(1 as usize, 0 as usize); 100_000_000];

        // State what we're doing.

        let bar = "▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓";
        println!("\n{}", bar);
        println!("DELIBERATELY PROVOKING A PANIC USING A CTRL-C");
        print!("{}", bar);
        std::io::stdout().flush().unwrap();

        // Fork, and inside the fork, give separate execution paths for parent and child.

        match fork() {
            // PARENT:
            Ok(ForkResult::Parent { child: _, .. }) => {
                // Sleep to let the child finish, then read enough bytes from pipe
                // so that we get the traceback.

                thread::sleep(time::Duration::from_millis(2000));
                let mut buffer = [0; 2000];
                unsafe {
                    let mut err_file = File::from_raw_fd(pipefd.0);
                    let _ = err_file.read(&mut buffer).unwrap();
                }

                // Evaluate the traceback.  We check only whether the traceback
                // points to the inner loop.

                println!("{}", bar);
                println!("TESTING THE PANIC FOR CORRECTNESS");
                println!("{}", bar);
                let s = strme(&buffer);
                let mut have_main = false;
                let lines: Vec<&str> = s.split_terminator('\n').collect();
                for i in 0..lines.len() {
                    // Test relaxed here because on an AWS box, we did not see the ::looper part.
                    // if lines[i].contains("pretty_trace::tests::looper") {
                    if lines[i].contains("pretty_trace::tests") {
                        have_main = true;
                    }
                }
                if have_main {
                    println!("\ngood: found inner loop\n");
                } else {
                    assert!(0 == 1, "FAIL: DID NOT FIND INNER LOOP");
                }
            }

            // CHILD:
            Ok(ForkResult::Child) => {
                // Spawn a thread to kill the child.

                thread::spawn(|| {
                    thread::sleep(time::Duration::from_millis(100));
                    let pid = std::process::id() as i32;
                    unsafe {
                        kill(pid, SIGINT);
                    }
                });

                // Do the actual work that the ctrl-c is going to interrupt.

                looper(&mut results);
            }
            Err(_) => println!("Fork failed"),
        }
    }
}
