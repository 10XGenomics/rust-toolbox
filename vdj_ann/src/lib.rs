// Copyright (c) 2019 10x Genomics, Inc. All rights reserved.

#![allow(clippy::all)]

extern crate align_tools;
extern crate amino;
extern crate bio;
extern crate debruijn;
extern crate fasta;
extern crate hyper;
#[macro_use]
extern crate io_utils;
extern crate kmer_lookup;
extern crate itertools;
extern crate serde;
extern crate serde_json;
extern crate stats_utils;
extern crate string_utils;
extern crate vector_utils;

// Modules introducing macros need to come before the modules that use them.

pub mod annotate;
pub mod refx;
pub mod transcript;
