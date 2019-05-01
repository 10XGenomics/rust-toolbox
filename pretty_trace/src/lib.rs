// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TOP LEVEL DOCUEMENTATION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

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
//! addition, unlike native tracebacks, pretty traces are obtained without setting
//! an environment variable.
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
//! mode, gathering tracebacks and then terminating.  
//!
//! For example this might be
//! <code>PROF=100</code> to profile 100 events.  It's your choice how to specify
//! this command-line option, but this crate makes it trivial to do so.
//! <font color="red">With about one minute's work,
//! you can make it possible to profile your code with essentially zero work,
//! whenever you like.</font>
//!
//! # Example of pretty trace profiling output
//!
//! <p style="line-height:1.0">
//! <font size="2" face="courier">
//! PRETTY TRACE PROFILE
//! <br><br>TRACED = 81.3%
//! <br><br>TOTAL = 100
//! <br><br>[1] COUNT = 13
//! <br>1: vdj_asm_tools::contigs::make_contigs
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/contigs.rs:494
//! <br>2: vdj_asm_tools::process::process_barcode
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/process.rs:1388
//! <br>3: vdj_asm_demo::process_project_core
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:202
//! <br>4: vdj_asm_demo::main
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:890
//! <br>&nbsp&nbsp vdj_asm_demo::main
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:854
//! <br>
//! <br>[2] COUNT = 6
//! <br>1: tenkit2::hyper::Hyper::build_from_reads
//! <br>&nbsp&nbsp ◼ tenkit2/src/hyper.rs:325
//! <br>2: vdj_asm_tools::process::process_barcode
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/process.rs:851
//! <br>3: vdj_asm_demo::process_project_core
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:202
//! <br>4: vdj_asm_demo::main
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:890
//! <br>&nbsp&nbsp vdj_asm_demo::main
//! <br>&nbsp&nbsp ◼ vdj_asm_tools/src/bin/vdj_asm_demo.rs:854
//! <br>...
//! </font>
//! </p>
//!
//! Here pretty trace profiling reveals exactly what some code was doing at 100
//! random instances; we show the first 19 of 100 collated tracebacks.  More were
//! attempted: of attempted tracebacks, 81.3% were successful.  Most fails are
//! due to cases where the stack trace would have 'walked' into the allocator, as
//! discussed at "Full Disclosure" below.
//!
//! # A brief guide for using pretty trace
//!
//! First make sure that you have rust debug on, by adding these lines
//! <pre>
//! [profile.release]
//! debug = true</pre>
//! to your top-level <code>Cargo.toml</code>.  We recommend always doing this,
//! regardless of
//! whether you use this crate.  The computational performance hit appears to be
//! small (although you will get larger executable files).  Using
//! <code>debug = 1</code>
//! does not work.  Then compile with <code>cargo build --release</code>.
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
//! <br> To instead profile, e.g. for 100 events, do this
//! <pre>
//!     PrettyTrace::new().profile(100).on();
//! </pre>
//!
//! Several other useful features are described below.  This include the capability
//! of tracing to know where you are in your data (and not just your code), and
//! for focusing profiling on a key set of crates that you're optimizing.
//!
//! # Credit
//!
//! This code was developed at 10x Genomics, and is based in part on C++ code
//! developed at the Whitehead Institute Center for Genome
//! Research / Broad Institute starting in 2000, and included in
//! <https://github.com/CompRD/BroadCRD>.
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
//! ◼ The code parses the output of a formatted stack trace, rather then
//!   generating output directly from a formal stack trace structure (which it
//!   should do).  This makes it vulnerable to changes in stack trace formatting.
//!
//! ◼ There is an ugly blacklist of strings that is fragile.  This may
//!   be an intrinsic feature of the approach.
//!
//! ◼ Ideally out-of-memory events would be caught and converted to panics so
//!   we could trace them, but we don't.
//!
//! ◼ Profile mode only sees the main thread.  This seems intrinsic to the
//!   approach.  So you may need to modify your code to run single-threaded to
//!   effectively use this mode.
//!
//! ◼ Profile mode yields no output if your program exits before obtaining the
//!   requested number of stack traces.
//!
//! ◼ Profile mode does not yield a stack trace if the code is executing inside
//!   the allocator.  In our test cases this is around 15% of the time.
//!
//! ◼ This is a preliminary version, which likely has bugs.
//!
//! # More
//!
//! See the documentation for <code>PrettyTrace</code>, linked to below.

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// EXTERNAL DEPENDENCIES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

extern crate backtrace;
extern crate chashmap;
extern crate failure;
extern crate io_utils;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate nix;
extern crate stats_utils;
extern crate string_utils;
extern crate vec_utils;

use backtrace::Backtrace;
use backtrace::*;
use chashmap::CHashMap;
use failure::Error;
use io_utils::*;
use libc::{kill, SIGINT, SIGKILL, SIGUSR1};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use stats_utils::*;
use std::{
    env,
    fs::{remove_file, File},
    io::{BufRead, BufReader, BufWriter, Write},
    ops::Deref,
    os::unix::io::FromRawFd,
    panic, process,
    str::from_utf8,
    sync::Mutex,
    thread,
    thread::ThreadId,
    time,
};
use string_utils::*;
use vec_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRETTY TRACE STRUCTURE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

/// A `PrettyTrace` is the working structure for this crate.  See also the top-level
/// crate documentation.

pub struct PrettyTrace {
    // filename to dump full traceback to upon panic
    pub full_file: Option<String>,
    // file descriptor to dump second copy of traceback to upon panic
    pub fd: Option<i32>,
    // thread message
    pub message: Option<&'static CHashMap<ThreadId, String>>,
    // is profile mode on?
    pub profile: bool,
    // count for profile mode
    pub count: Option<usize>,
    // whitelist for profile mode
    pub whitelist: Option<Vec<String>>,
    // convert Ctrl-Cs to panics
    pub ctrlc: bool,
}

/// Normal usage of `PrettyTrace` is to call
/// <pre>
/// PrettyTrace::new().&lt set some things >.on();
/// </pre>
/// once near the begining of your main program.

impl PrettyTrace {
    /// Initialize a <code>PrettyTrace</code> object.  This does nothing
    /// in and of itself.

    pub fn new() -> PrettyTrace {
        PrettyTrace {
            full_file: None,
            fd: None,
            profile: false,
            message: None,
            count: None,
            whitelist: None,
            ctrlc: false,
        }
    }

    /// Cause a <code>PrettyTrace</code> object to do something: change the
    /// behavior of response to <code>panic!</code> to produce a prettified
    /// traceback and perform profiling, if <code>profile()</code> has been called.

    pub fn on(&mut self) {
        let mut fd = -1 as i32;
        if self.fd.is_some() {
            fd = self.fd.unwrap() as i32;
        }
        let mut haps = Happening::new();
        if self.profile {
            if self.whitelist.is_none() {
                self.whitelist = Some(Vec::<String>::new());
            }
            haps.initialize(&self.whitelist.clone().unwrap(), self.count.unwrap());
        }

        let mut full_file = String::new();
        if self.full_file.is_some() {
            full_file = self.full_file.clone().unwrap();
        }
        if self.message.is_some() {
            force_pretty_trace_fancy(full_file, fd, &self.message.unwrap(), &haps, self.ctrlc);
        } else {
            let tm = new_thread_message();
            force_pretty_trace_fancy(full_file, fd, &tm, &haps, self.ctrlc);
        }
    }

    /// Cause a <code>Ctrl-C</code> interrupt to be turned into a panic, and thence
    /// produce a traceback for the main thread.  This does not allow you to see
    /// what other threads are doing.  If you <code>Ctrl-C</code> twice in rapid
    /// succession, you may elide the traceback, but this is inconsistent.

    pub fn ctrlc(&mut self) -> &mut PrettyTrace {
        self.ctrlc = true;
        self
    }

    /// Define a file, that in the event that a traceback is triggered by a
    /// panic, will be used to dump a full traceback to.  The
    /// <i>raison d'etre</i> for this is that an abbreviated pretty traceback might
    /// in some cases elide useful information (although this has not been observed).
    ///
    /// You can also force <code>PrettyTrace</code> to emit full tracebacks by
    /// setting the environment variable <code>RUST_FULL_TRACE</code>.

    pub fn full_file(&mut self, full_file: &str) -> &mut PrettyTrace {
        self.full_file = Some(full_file.to_string());
        self
    }

    /// Define a file descriptor, that in the event a traceback is triggered by a
    /// panic, will be used to dump a second copy of the traceback to.

    pub fn fd(&mut self, fd: i32) -> &mut PrettyTrace {
        self.fd = Some(fd);
        self
    }

    /// Define a message object that will be used by threads to store their status.
    /// This is printed if a traceback is triggered by a panic, and where
    /// code is traversing data in a loop, can be used to determine not only where
    /// execution is in the code, but also where it is in the data.

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
        self
    }

    /// Request that a profile consisting of `count` traces be generated.
    /// If you use this, consider calling `whitelist` too.

    pub fn profile(&mut self, count: usize) -> &mut PrettyTrace {
        self.profile = true;
        self.count = Some(count);
        self
    }

    /// Define the whitelist for profile mode.  It is a list of strings that
    /// profile traces are matched against.  Only traces matching at least one of
    /// the strings are shown.  This allows tracebacks to be focused on a fixed set
    /// of crates that you're trying to optimize.  Setting this option can greatly
    /// increase the utility of profile mode.

    /// # Example
    /// <pre>
    ///    PrettyTrace::new()
    ///        .profile(100)
    ///        .whitelist( &vec![ "gerbilizer", "creampuff" ] )
    ///        .on();
    /// </pre>

    pub fn whitelist(&mut self, whitelist: &Vec<&str>) -> &mut PrettyTrace {
        let mut x = Vec::<String>::new();
        for i in 0..whitelist.len() {
            x.push(whitelist[i].to_string());
        }
        self.whitelist = Some(x);
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
}

impl Happening {
    pub fn new() -> Happening {
        Happening {
            on: false,
            whitelist: Vec::<String>::new(),
            hcount: 0,
        }
    }

    // EXAMPLE: set whitelist to a or b or c, hcount to 250
    // let mut happening = Happening::new();
    // happening.initialize( &vec![ "a", "b", "c" ], 250 );

    pub fn initialize(&mut self, whitelist: &Vec<String>, hcount: usize) {
        self.on = true;
        self.whitelist = whitelist.clone();
        self.hcount = hcount;
    }
}

lazy_static! {
    static ref HAPPENING: Mutex<Happening> = Mutex::new(Happening::new());
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TEST TO SEE IF CODE WAS INTERRUPTED WHILE IN THE MEMORY ALLOCATOR
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// If you catch an interrupt, the code may have been in the memory allocator
// at the time it was interrupted.  In such cases, almost any memory allocation,
// e.g. pushing back onto a vector, may cause a hang or crash.  The following code
// attempts to test for the "in allocator" state and can be used to avoid
// dangerous operations.  The code works by comparing to a hardcoded list of
// names, and it is hard to believe that this works, but it appears to do so.

fn test_in_allocator() -> bool {
    // eprintln!( "\nTESTING FOR ALLOCATOR" );
    let mut in_alloc = false;
    // The following lock line (copied from the Backtrace crate) doesn't
    // seem necessary here (and would require plumbing to compile anyway).
    // let _guard = ::lock::lock();
    trace(|frame| {
        resolve(frame.ip() as *mut _, |symbol| {
            // eprintln!( "symbol name = {:?}", symbol.name() );
            match symbol.name() {
                Some(x) => {
                    if x.as_str().unwrap() == "realloc"
                        || x.as_str().unwrap() == "__GI___libc_malloc"
                        || x.as_str().unwrap() == "malloc_consolidate"
                        || x.as_str().unwrap() == "_int_free"
                        || x.as_str().unwrap() == "calloc"
                        || x.as_str().unwrap() == "_int_malloc"
                    {
                        // eprintln!( "in allocator" );
                        in_alloc = true;
                        // break;
                    }
                }
                None => {}
            }
        });
        !in_alloc
    });
    in_alloc
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
static mut PROCESSING_SIGUSR1: bool = false;

extern "C" fn handler(sig: i32) {
    if sig == SIGINT {
        unsafe {
            if HEARD_CTRLC > 0 {
                HEARD_CTRLC += 1;
                std::process::exit(0);
            }
            HEARD_CTRLC += 1;
            thread::sleep(time::Duration::from_millis(1000));
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
    if sig == SIGUSR1 {
        // Test to see if we appear to have interrupted the allocator.  In that
        // case, give up.  If we were to instead try to create a backtrace, the
        // backtrace code would push stuff onto a vector, and with high probability
        // something bad would happen in the allocator, and the kernel would kill
        // the process.  Of course this means that the stack traces we see are
        // somewhat biased.

        unsafe {
            PROCESSING_SIGUSR1 = true;
        }
        if test_in_allocator() {
            unsafe {
                PROCESSING_SIGUSR1 = false;
            }
            return;
        }

        // Now do the backtrace.

        let backtrace = Backtrace::new();
        let tracefile = format!("/tmp/traceback_from_process_{}", process::id());
        let mut tf = open_for_write_new![&tracefile];
        let raw = false; // for debugging
        if raw {
            fwriteln!(tf, "RAW BACKTRACE\n");
            fwriteln!(tf, "{:?}", backtrace);
            fwriteln!(tf, "\nPRETTIFIED BACKTRACE\n");
        }
        let mut whitelist = Vec::<String>::new();
        for x in HAPPENING.lock().unwrap().whitelist.iter() {
            whitelist.push(x.clone());
        }
        fwriteln!(tf, "{}", prettify_traceback(&backtrace, &whitelist, true));
        unsafe {
            PROCESSING_SIGUSR1 = false;
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// CORE TRACEBACK FUNCTION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

/// See <code>PrettyTrace</code> documentation for how this is used.

pub fn new_thread_message() -> &'static CHashMap<ThreadId, String> {
    let box_thread_message = Box::new(CHashMap::<ThreadId, String>::new());
    let thread_message: &'static CHashMap<ThreadId, String> = Box::leak(box_thread_message);
    thread_message
}

fn force_pretty_trace_fancy(
    log_file_name: String,
    fd: i32,
    thread_message: &'static CHashMap<ThreadId, String>,
    happening: &Happening,
    ctrlc: bool,
) {
    // Launch happening thread, which imits SIGUSR1 interrupts.  Usually, it will
    // hang after some number of iterations, and at that point we kill ourself,
    // because exiting won't stop the hang.

    if happening.on {
        // Set HAPPENING.  The following doesn't work so copying by hand.
        // *HAPPENING.get_mut().unwrap() = happening.clone();

        HAPPENING.lock().unwrap().on = happening.on;
        HAPPENING
            .lock()
            .unwrap()
            .whitelist
            .append(&mut happening.whitelist.clone());
        HAPPENING.lock().unwrap().hcount = happening.hcount;
        let hcount = happening.hcount;

        // Gather tracebacks.

        thread::spawn(move || {
            let pid = std::process::id();
            let tracefile = format!("/tmp/traceback_from_process_{}", pid);
            let mut traces = Vec::<String>::new();
            let (mut interrupts, mut tracebacks) = (0, 0);
            loop {
                thread::sleep(time::Duration::from_millis(1000));
                if path_exists(&tracefile) {
                    remove_file(&tracefile).unwrap();
                }
                unsafe {
                    if kill(pid as i32, SIGUSR1) != 0 {
                        break;
                    }
                }
                interrupts += 1;
                for _ in 0..100 {
                    // wait briefly for tracefile
                    if !path_exists(&tracefile) {
                        thread::sleep(time::Duration::from_millis(10));
                    }
                }
                if !path_exists(&tracefile) {
                    unsafe {
                        thread::sleep(time::Duration::from_millis(1000));
                        if PROCESSING_SIGUSR1 {
                            thread::sleep(time::Duration::from_millis(5000));
                            kill(pid as i32, SIGKILL);
                        }
                    }
                }
                if !path_exists(&tracefile) {
                    continue;
                } // or should we break?
                let f = open_for_read![&tracefile];
                let mut trace = String::new();
                for line in f.lines() {
                    let s = line.unwrap();
                    trace += &format!("{}\n", s);
                }
                if trace.len() > 0 {
                    traces.push(trace);
                    tracebacks += 1;
                }
                if traces.len() == hcount {
                    traces.sort();
                    let mut freq = Vec::<(u32, String)>::new();
                    make_freq(&traces, &mut freq);
                    let mut report = String::new();
                    report += &format!(
                        "\nPRETTY TRACE PROFILE\n\nTRACED = {:.1}%\n\nTOTAL = {}\n\n",
                        percent_ratio(tracebacks, interrupts),
                        traces.len()
                    );
                    for i in 0..freq.len() {
                        report += &format!("[{}] COUNT = {}\n{}", i + 1, freq[i].0, freq[i].1);
                    }
                    print!("{}", report);
                    std::process::exit(0);
                }
            }
        });
    }

    // Set up to catch SIGNINT and SIGUSR1 interrupts.

    let _ = install_signal_handler(happening.on, ctrlc);

    // Setup panic hook. If we panic, this code gets run.

    panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Get backtrace.

        let backtrace = Backtrace::new();

        // Get thread message.

        let mut tm = String::new();
        let this_thread = thread::current().id();
        if thread_message.contains_key(&this_thread) {
            tm = format!("{}\n\n", thread_message.get(&this_thread).unwrap().deref());
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
                eprintln!(
                    "\nRUST PROGRAM PANIC\n\n(Full traceback.  \
                     Rerun with env var RUST_FULL_TRACE unset to \
                     see short traceback.)\n\n{}{}\n\n{}\n",
                    tm,
                    &msg,
                    from_utf8(&bt).unwrap()
                );
                std::process::exit(1);
            }
        }

        // Prettify the traceback.

        let all_out = prettify_traceback(&backtrace, &Vec::<String>::new(), false);

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
        let msg = match info.location() {
            Some(location) => {
                let loc = location.file().clone();

                // Replace long constructs of the form /rustc/......./src/
                //                                  by /rustc/<stuff>/src/.

                let mut x2 = loc.to_owned();
                if loc.contains("/rustc/") {
                    if loc.after("/rustc/").contains("/src/") {
                        let y = loc.between("/rustc/", "/src/");
                        if y.len() > 10 {
                            x2 = x2.replace(y, "<stuff>");
                        }
                    }
                }
                if loc.contains("/checkouts/") {
                    if loc.after("/checkouts/").contains("/src/") {
                        let y = loc.between("/checkouts/", "/src/");
                        if y.len() > 10 {
                            x2 = x2.replace(y, "<stuff>");
                        }
                    }
                }

                // Format lead message.

                let mut pre = format!("{}:{}", x2, location.line());
                let mut prex = format!("\n\n0: ◼ {}", pre);
                if all_out.contains(&pre) || pre.contains("pretty_trace") {
                    prex = "".to_string();
                }

                let mut long_msg = "Rerun with env var RUST_FULL_TRACE set to see full \
                                    traceback."
                    .to_string();
                if log_file_name.len() > 0 {
                    long_msg = format!("Full traceback is at {}.", log_file_name);
                }
                format!(
                    "RUST PROGRAM PANIC\n\n(Shortened traceback.  \
                     {})\n\n{}{}{}",
                    long_msg, tm, msg, prex
                )
            }
            None => format!("RUST PROGRAM PANIC\n\n{}", msg),
        };
        if msg.contains("Broken pipe") {
            std::process::exit(1);
        }

        // Now print stuff.  Package as a single print line to prevent
        // interweaving if multiple threads panic.

        let mut out = format!("\n{}\n\n", &msg);
        out += &all_out;
        eprint!("{}", out);

        // Dump traceback to file descriptor.

        if fd >= 0 {
            unsafe {
                let mut err_file = File::from_raw_fd(fd);
                let _ = err_file.write(out.as_bytes()).unwrap();
            }
        }

        // Dump full traceback to log file.

        if log_file_name != "" {
            let f = File::create(&log_file_name);
            if !f.is_ok() {
                eprintln!(
                    "\nDuring panic, attempt to create full log file \
                     named {} failed, giving up.\n",
                    log_file_name
                );
                std::process::exit(1);
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
                    "\nRUST PROGRAM PANIC\n\n(Full traceback.)\n\n{}{}\n\n{}\n",
                    tm,
                    &msg,
                    from_utf8(&bt).unwrap()
                ))
                .unwrap();
        }

        // Exit.

        std::process::exit(1);
    }));
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRETTIFY TRACEBACK
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

fn prettify_traceback(backtrace: &Backtrace, whitelist: &Vec<String>, pack: bool) -> String {
    // Parse the backtrace into lines.

    let bt: Vec<u8> = format!("{:?}", backtrace).into_bytes();
    let mut btlines = Vec::<Vec<u8>>::new();
    let mut line = Vec::<u8>::new();
    for i in 0..bt.len() {
        if bt[i] == b'\n' {
            // Replace long constructs of the form /rustc/......./src/
            //                                  by /rustc/<stuff>/src/.

            let x = stringme(&line);
            let mut x2 = x.clone();
            if x.contains("/rustc/") {
                if x.after("/rustc/").contains("/src/") {
                    let y = x.between("/rustc/", "/src/");
                    if y.len() > 10 {
                        x2 = x2.replace(y, "<stuff>");
                    }
                }
            }
            if x.contains("/checkouts/") {
                if x.after("/checkouts/").contains("/src/") {
                    let y = x.between("/checkouts/", "/src/");
                    if y.len() > 10 {
                        x2 = x2.replace(y, "<stuff>");
                    }
                }
            }
            btlines.push(x2.as_bytes().to_vec());

            // Reset line.

            line.clear();
        } else {
            line.push(bt[i]);
        }
    }

    // Convert the traceback into a Vec<Vec<Vec<String>>>>.  The outer vector corresponds to the
    // traceback block.  Within each block is a vector of traceback entries, and each entry is
    // itself a vector of on or two lines.  It is two, except when there is no code line number.
    // The initial blanks and block numbers are stripped off.

    let mut blocks = Vec::<Vec<Vec<Vec<u8>>>>::new();
    let mut block = Vec::<Vec<Vec<u8>>>::new();
    let mut blocklet = Vec::<Vec<u8>>::new();
    for i in 0..btlines.len() {

        // Ignore blank lines.

        if btlines[i].len() == 0 {
            continue;
        }

        // Determine if this line begins a block, i.e. looks like <blanks><number>:<...>.

        let mut s = btlines[i].as_slice();
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
        if j < s.len() && s[j] == b':' {
            if block.len() > 0 {
                if blocklet.len() > 0 {
                    block.push( blocklet.clone() );
                    blocklet.clear();
                }
                blocks.push( block.clone() );
                block.clear();
                s = &s[j+1 .. s.len() ];
            }
        }

        // Proceed.

        let mut j = 0;
        while j < s.len() {
            if s[j] != b' ' {
                break;
            }
            j += 1;
        }
        s = &s[ j .. s.len() ];
        blocklet.push( s.to_vec() );
        if s.starts_with( b"at " ) {
            block.push( blocklet.clone() );
            blocklet.clear();
        }
    }
    if blocklet.len() > 0 {
        block.push( blocklet.clone() );
    }
    if block.len() > 0 {
        blocks.push( block.clone() );
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
        "<unknown>",
        "/panic.rs",
        "/panicking.rs",
        "catch_unwind",
        "lang_start_internal",
        "libstd/rt.rs",
    ];

    // Remove certain 'unwanted' blocklets.

    for i in 0..blocks.len() {
        let mut to_delete = vec![ false; blocks[i].len() ];
        for j in 0..blocks[i].len() {

            // Blocklet may not contain a blacklisted string.

            'outer1: for k in 0..blocks[i][j].len() {
                let s = strme(&blocks[i][j][k]);
                for b in blacklist.iter() {
                    if s.contains(b) {
                        // CRAZY EXEMPTION BECAUSE OF HOW TEST IS NAMED!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                        if s.contains( "pretty_trace_tests" ) { // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                            continue; // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                        } // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                        to_delete[j] = true;
                        break 'outer1;
                    }
                }
            }

            // Blocklet must contain a whitelisted string (if whitelist provided).

            if !to_delete[j] && whitelist.len() > 0 {
                let mut good = false;
                'outer2: for k in 0..blocks[i][j].len() {
                    let s = strme(&blocks[i][j][k]);
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

            let s = strme(&blocks[i][j][0]);
            if !to_delete[j] && blocks[i][j].len() == 1 && s.ends_with( "- main" ) {
                to_delete[j] = true;
            }

            // Don't allow blocklets whose first line has the form ... main(...).

            if s.contains(" main (") {
                if s.after(" main (").contains(")") {
                    if !s.between(" main (", ")").contains("(") {
                        to_delete[j] = true;
                    }
                }
            }

        }
        erase_if( &mut blocks[i], &to_delete );
    }

    // Remove any block that has length zero.
        
    let mut to_delete = vec![ false; blocks.len() ];
    for i in 0..blocks.len() {
        if blocks[i].len() == 0 {
            to_delete[i] = true;
        }
    }
    erase_if( &mut blocks, &to_delete );

    // stuff from earlier version, not addressing now

    // !s2.contains(".rs:0")
    // ((!s.contains(" - <") && !s.contains("rayon::iter")) || k == i)

    // if s.contains("::{{closure}}") {
    //     s = s.rev_before("::{{closure}}");
    // }

    // Contract paths that look like " .../.../src/...".

    let src = b"/src/".to_vec();
    for i in 0..blocks.len() {
        for j in 0..blocks[i].len() {
            if blocks[i][j].len() == 2 {
                let mut x = Vec::<u8>::new();
                let mut y = blocks[i][j][1].clone();
                for j in 0..y.len() {
                    if contains_at(&y, &src, j) {
                        for k in (0..j).rev() {
                            if y[k] != b'/' {
                                continue;
                            }
                            for l in (0..k).rev() {
                                if y[l] == b' ' {
                                    for m in 0..l + 1 {
                                        x.push(y[m]);
                                    }
                                    for m in k + 1..y.len() {
                                        x.push(y[m]);
                                    }
                                    break;
                                }
                                if x.len() > 0 {
                                    break;
                                }
                            }
                            if x.len() > 0 {
                                break;
                            }
                        }
                    }
                    if x.len() > 0 {
                        break;
                    }
                }
                if x.len() > 0 {
                    blocks[i][j][1] = y;
                }
            }
        }
    }

    // Emit prettified output.

    let mut all_out = String::new();
    for i in 0..blocks.len() {
        let num = format!( "{}: ", i+1 );
        let sub = stringme( &vec![ b' '; num.len() ].as_slice() );
        for j in 0..blocks[i].len() {
            for k in 0..blocks[i][j].len() {
                if j == 0 && k == 0 { 
                    all_out += &num;
                }
                else {
                    all_out += &sub;
                }
                if k > 0 {
                    all_out += "◼ ";
                }
                let mut s = stringme(&blocks[i][j][k]);
                if k == 0 {
                    if s.contains( "::" ) {
                        let cc = s.rfind("::").unwrap();
                        s.truncate(cc);
                    }
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
