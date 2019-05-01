// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

// Partial test for correctness of PrettyTrace::ctrlc();

extern crate libc;
extern crate nix;
extern crate pretty_trace;
extern crate rayon;
extern crate string_utils;

use libc::{kill, SIGINT};
use nix::unistd::{fork, pipe, ForkResult};
use pretty_trace::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::{thread, time};
use string_utils::*;

fn main() {
    // Create a pipe.

    let pipefd = pipe().unwrap();

    // Set up tracebacks with ctrlc and using the pipe.

    PrettyTrace::new().ctrlc().fd(pipefd.1).on();

    // Create stuff needed for computation we're going to interrupt.

    let mut results = vec![(1 as usize, 0 as usize); 100_000_000];

    // State what we're doing.

    let bar =
        "▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓";
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

            // Evaluate the traceback.  We check only for whether "main" appears.

            println!("{}", bar);
            println!("TESTING THE PANIC FOR CORRECTNESS");
            println!("{}", bar);
            let s = strme(&buffer);
            let mut have_main = false;
            let lines: Vec<&str> = s.split_terminator('\n').collect();
            for i in 0..lines.len() {
                if lines[i].contains("main") {
                    have_main = true;
                }
            }
            if have_main {
                println!("\ngood: found main program\n");
            } else {
                println!("\nFAIL: DID NOT FIND MAIN PROGRAM\n");
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

            results.par_iter_mut().for_each(|r| {
                for _ in 0..10_000 {
                    r.1 += 1 + r.0 * r.1;
                }
            });
        }
        Err(_) => println!("Fork failed"),
    }
}
