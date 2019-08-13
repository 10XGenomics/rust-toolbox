// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

// When tested under OS X, this code correctly panics, but the resulting 
// traceback does not reach the main program.  Interestingly, setting
// RUST_FULL_TRACE does not help, but commenting out the PrettyTrace line does.
//
// This was observed under macOS Version 10.13.5, and could conceivably be
// version dependent.

extern crate pretty_trace;

use pretty_trace::*;
use std::fs;

fn main() {
    PrettyTrace::new().on();
    let not = "file_that_does_not_exist";
    let _ = fs::read_to_string(&not).unwrap();
}
