// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TOP LEVEL DOCUEMENTATION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

//! # Introduction
//!
//! Tracebacks from rust are remarkably complete: they essentially always
//! extend from the 'broken' code line all the way to the main program.
//! We may take this for granted but it is not always the case in other languages,
//! including C++.
//!
//! However, there are a few issues.  The first is that you need to turn 'debug'
//! on, by adding the lines
//! <pre>
//! [profile.release]
//! debug = true</pre>
//! to your top-level Cargo.toml.  The computational performance hit for running
//! with debug on appears to be small.  So this is not really an issue, but rather
//! just something that you need to remember to do.
//! <br><br>
//! The other two issues are that you have to set the environment variable
//! RUST_BACKTRACE to 1, or you get no traceback at all, and the tracebacks can be
//! unnecessarily long and difficult to read.
//! <br><br>
//! The latter two issues are addressed by this crate, which makes tracebacks
//! automatic and as succinct as possible, in some cases about 
//! <font color="red"> ten times 
//! shorter</font> than what you would otherwise get.  We also provide other
//! functionality for working with tracebacks, as described below.  This includes
//! the capability of <font color="red"> trivially profiling code</font>.
//!
//! # Example of standard versus pretty trace output
//! <div>
//! <img src="../../../images/long_vs_short_traceback.png"/>
//! </div>
//!
//! # What this code can do
//! The following functionality is supported:
//!
//! 1. Catch panics and emit an abbreviated, cleaned up traceback.
//!
//!    Full tracebacks in rust can be very long, so this provides a quick view.
//!    And rust default behavior is to not provide tracebacks at all.
//!
//! 2. Provide a full traceback if you want that instead or in addition to the
//!    shortened traceback.
//!
//!    The abbreviation code might "mess up", so this feature could be useful.
//!
//! 3. Catch Ctrl-C and convert to panic, and thence as above.  Note that this will
//!    only trace the master thread, so if you have parallelization, you probably
//!    need to turn that off first.  If you press Ctrl-C twice in rapid succession,
//!    you won't get a traceback.
//!
//!    One use case of this is to find an infinite loop bug.
//!
//! 4. Read thread status from a user-defined structure and report that with the
//!    traceback so you can see not only "where you are in your code" but also
//!    "where you are in your data".
//!
//!    This can be very useful for reproducing bugs.
//!
//! 5. Collect a random sample of pretty tracebacks by interrupting the code
//!    repeatedly.
//!
//!    This can be highly useful for profiling.
//!
//! This code was developed at 10x Genomics, and is based in part on C++ code 
//! developed at the Whitehead Institute Center for Genome
//! Research / Broad Institute starting in 2000, and included in
//! <https://github.com/CompRD/BroadCRD>.
//!
//! # How to use it
//!
//! <b>ALL:</b>
//! <pre>use force_pretty_trace::*;</pre>
//!
//! <b>USE CASE #1.</b>  Force a pretty and abbreviated traceback upon panic and
//! upon Ctrl-C interrupts.
//!
//! Just put this at the beginning of your main program:
//!
//! <pre>
//!   force_pretty_trace();
//!</pre>
//!
//! <b>USE CASE #2.</b>  Same as #1, but you want to see a full traceback instead.  
//! Just set the environment variable RUST_FULL_TRACE.  Alternatively, to get both, 
//! use
//!<pre>   force_pretty_trace_with_log( &log_file_name );</pre>
//!
//! which will get you a shortened traceback on stderr, and a full traceback in the
//! given file.
//!
//! <b>USE CASE #3.</b>  Same as above, but also dump the short log to a file 
//! descriptor.
//!
//! <pre>
//!   force_pretty_trace_with_log_plus( &log_file_name, fd );</pre>
//!
//! <b>USE CASE #4.</b>  Also pass a map that assigns a message to each thread.  You
//! can use this to assign a "status" to each thread, and that status will get
//! printed upon panic.
//!
//! To use this, put the following code at the beginning of your main:
//!
//!<pre>
//!   let thread_message = new_thread_message();
//!   force_pretty_trace_with_message( &thread_message );</pre>
//!
//! and put the following in a place that will get executed once by each thread:
//!
//! <pre>
//!   thread_message.insert( thread::current().id(),
//!       "message that describes what the thread is doing" );</pre>
//!
//! <b> USE CASE #5.</b> Happening mode.  Collect and collate a fixed-size random 
//! sample of pretty tracebacks at a rate of approximately one per second, then
//! exit.  For this you 
//! may specify strings A,...,Z that will be grepped for in the traceback.  This is
//! useful because you may only be interested in where your code is executing in 
//! particular crates that you're working on.
//!
//! <pre>
//!   let whitelist = "A|...|Z";
//!   let hcount = number of tracebacks to gather;
//!   let mut haps = Happening::new_initialize( &whitelist, hcount );
//!   force_pretty_trace_with_happening( &haps );</pre>
//!
//! Use cases can also be combined -- see force_pretty_trace_fancy below.
//!
//! # Example of happening mode output
//!
//! <div>
//! <img src="../../../images/happening.png"/ height=600 width=500>
//! </div>
//!
//! # Issues, buggy things and missing features
//!
//! ◼ The code parses the output of a formatted stack trace, rather then
//!   generating output directly from a formal stack trace structure (which it
//!   should do).  This
//!   makes it vulnerable to changes in how rust formats the stack trace.
//!
//! ◼ There is an ugly blacklist of strings that is also fragile.
//!
//! ◼ Pretty traces containing more than ten items may not be correctly handled.
//!
//! ◼ Out-of-memory events could be converted to panics, then traced.
//!
//! ◼ Happening mode yields no output if your program exits before obtaining the
//!   requested number of stack traces.
//!
//! ◼ Happening mode does not yield a stack trace if the code is executing inside
//!   the allocator.  In our test cases this is around 15% of the time.

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
extern crate stats;
extern crate string_utils;
extern crate vec_utils;

use backtrace::Backtrace;
use backtrace::*;
use chashmap::CHashMap;
use failure::Error;
use io_utils::*;
use libc::{kill, SIGINT, SIGKILL, SIGUSR1};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use stats::*;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::io::FromRawFd;
use std::{
    env,
    fs::{remove_file, File},
};
use std::{ops::Deref, panic, process, str::from_utf8};
use std::{sync::Mutex, thread, thread::ThreadId, time};
use string_utils::*;
use vec_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// USER INTERFACE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn force_pretty_trace() {
    let tm = new_thread_message();
    force_pretty_trace_fancy(String::new(), -1 as i32, &tm, &Happening::new());
}

pub fn force_pretty_trace_with_log(log_file_name: String) {
    let tm = new_thread_message();
    force_pretty_trace_fancy(log_file_name, -1 as i32, &tm, &Happening::new());
}

pub fn force_pretty_trace_with_log_plus(log_file_name: String, fd: i32) {
    let tm = new_thread_message();
    force_pretty_trace_fancy(log_file_name, fd, &tm, &Happening::new());
}

pub fn force_pretty_trace_with_happening(happening: &Happening) {
    let tm = new_thread_message();
    force_pretty_trace_fancy(String::new(), -1 as i32, &tm, happening);
}

pub fn force_pretty_trace_with_message(thread_message: &'static CHashMap<ThreadId, String>) {
    force_pretty_trace_fancy(String::new(), -1 as i32, &thread_message, &Happening::new());
}

/*

pub fn force_pretty_trace_fancy(

    log_file_name: String, fd: i32,
    thread_message: &'static CHashMap<ThreadId,String>, happening: &Happening );

*/

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// HAPPENING STRUCTURE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Data structure for control of happening handling.

pub struct Happening {
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
    // happening.initialize( &"a|b|c", 250 );

    pub fn initialize(&mut self, whitelist: &str, hcount: usize) {
        self.on = true;
        let x = whitelist.split('|').collect::<Vec<&str>>();
        for i in 0..x.len() {
            self.whitelist.push(x[i].to_string());
        }
        self.hcount = hcount;
    }

    pub fn new_initialize(whitelist: &str, hcount: usize) -> Happening {
        let mut haps = Happening::new();
        haps.initialize(whitelist, hcount);
        haps
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
    // eprintln!( "returning from in_allocator" ); // XXXXXXXXXXXXXXXXXXXXXXXXXX
    in_alloc
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// SIGNAL HANDLING
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Redirect SIGINT and SIGUSR1 interrupts to the function "handler".

fn install_signal_handler(happening: bool) -> Result<(), Error> {
    if happening {
        let handler = SigHandler::Handler(handler);
        let action = SigAction::new(handler, SaFlags::SA_RESTART, SigSet::empty());
        unsafe {
            sigaction(Signal::SIGUSR1, &action)?;
        }
    } else {
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
                std::process::exit(0);
            }
            HEARD_CTRLC += 1;
            thread::sleep(time::Duration::from_millis(400));
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

pub fn new_thread_message() -> &'static CHashMap<ThreadId, String> {
    let box_thread_message = Box::new(CHashMap::<ThreadId, String>::new());
    let thread_message: &'static CHashMap<ThreadId, String> = Box::leak(box_thread_message);
    thread_message
}

pub fn force_pretty_trace_fancy(
    log_file_name: String,
    fd: i32,
    thread_message: &'static CHashMap<ThreadId, String>,
    happening: &Happening,
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
                        "\nHAPPENING REPORT\n\nTRACED = {:.1}%\n\nTOTAL = {}\n\n",
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

    let _ = install_signal_handler(happening.on);

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

    // Format the backtrace to remove 'junk', and renumber the trace entries.

    let filter = true;
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
    ];
    let mut btlines2 = Vec::<Vec<u8>>::new();
    if !filter {
        btlines2 = btlines.clone();
    } else {
        let mut i = 1;
        let mut count = 1;
        while i < btlines.len() {
            let mut j = i + 1;
            while j < btlines.len() {
                let linex = btlines[j].clone();
                if linex.len() >= 5 && linex[4] == b':' {
                    break;
                }
                j += 1;
            }

            // Now btlines[i..j] is a block in the trace.

            let linex = btlines[i].clone();
            let s = strme(&linex);
            let mut junk = false;
            for b in blacklist.iter() {
                if s.contains(b) {
                    junk = true;
                }
            }
            if whitelist.len() > 0 {
                let mut good = false;
                for k in i..j {
                    let linex = btlines[k].clone();
                    let s = strme(&linex);
                    for b in whitelist.iter() {
                        if s.contains(b) {
                            good = true;
                        }
                    }
                }
                if !good {
                    junk = true;
                }
            }
            if s.contains(" main (") {
                if s.after(" main (").contains(")") {
                    if !s.between(" main (", ")").contains("(") {
                        junk = true;
                    }
                }
            }
            if !junk && !(j - i == 1 && s.ends_with("- main")) {
                let lineno = format!("{c:>width$}", c = count, width = 4);
                let linenox = lineno.as_bytes();
                for l in 0..4 {
                    btlines[i][l] = linenox[l];
                }
                count += 1;

                // The trace block has been accepted.  Now decide which lines
                // in the block to keep.  These are pushed onto btlines2.

                let mut k = i;
                let mut printed = false;
                while k < j {
                    let linex = btlines[k].clone();
                    let s = strme(&linex);
                    let linex2 = btlines[k + 1].clone();
                    let s2 = strme(&linex2);
                    let mut good = true;
                    if whitelist.len() > 0 {
                        good = false;
                        for b in whitelist.iter() {
                            if s.contains(b) {
                                good = true;
                            }
                        }
                    }
                    if good
                        && !s2.contains(".rs:0")
                        && ((!s.contains(" - <") && !s.contains("rayon::iter")) || k == i)
                    {
                        // Add back traceback entry number if needed.  Doesn't work
                        // if 10 or more.

                        if k > i && !printed {
                            if btlines[k].len() >= 5 && count < 10 {
                                btlines[k][3] = b'0' + count - 1;
                                btlines[k][4] = b':';
                            }
                        }
                        printed = true;
                        if s.contains("::") {
                            let cc = s.rfind("::").unwrap();
                            btlines2.push(btlines[k][0..cc].to_vec());
                        } else {
                            btlines2.push(btlines[k].clone());
                        }
                        if k + 1 < j {
                            btlines2.push(btlines[k + 1].clone());
                        }
                    }
                    k += 2;
                }
                btlines2.push(Vec::<u8>::new());
            }
            i = j;
        }
    }

    // Contract paths that look like " .../.../src/...".

    let src = b"/src/".to_vec();
    for i in 0..btlines2.len() {
        let mut x = Vec::<u8>::new();
        let mut y = btlines2[i].clone();
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
            btlines2[i] = x;
        }
    }

    // Make the lines prettier.

    let mut btlines3 = Vec::<Vec<u8>>::new();
    for i in 0..btlines2.len() {
        let linex = btlines2[i].clone();
        let s = strme(&linex);
        let mut x = Vec::<u8>::new();
        let init = 8;
        if s.len() < init {
            btlines3.push(linex.clone());
        } else {
            for j in 0..init {
                x.push(linex[j]);
            }
            let mut start = init;
            // if linex.len() >= 5 && linex[4] == b':' && s.contains( " - " ) {
            if s.contains(" - ") {
                start = s.find(" - ").unwrap() + 1;
            } else if s.contains(" at ") {
                start = s.find(" at ").unwrap() + 1;
            }
            for j in start..s.len() {
                x.push(linex[j]);
            }

            // Delete three leading blanks, then save line.

            let x2 = x.clone();
            if String::from_utf8(x2).unwrap().starts_with("        at") {
                let mut y = x[10..x.len()].to_vec();
                x = "   ◼".as_bytes().to_vec();
                x.append(&mut y);
            } else {
                if x.len() > 3 {
                    x = x[3..x.len()].to_vec();
                }
            }
            btlines3.push(x);
        }
    }
    let mut all_out = String::new();
    for i in 0..btlines3.len() {
        let x = &btlines3[i];
        let mut s = strme(&x);
        if s.contains("::{{closure}}") {
            s = s.rev_before("::{{closure}}");
        }
        if x.len() > 0 || !pack {
            all_out += &format!("{}\n", s);
        }
    }
    all_out
}
