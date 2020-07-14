// Copyright (c) 2018 10x Genomics, Inc. All rights reserved.

// Read a fasta file.

extern crate debruijn;
extern crate flate2;
extern crate io_utils;
extern crate string_utils;

use debruijn::dna_string::*;
use flate2::read::MultiGzDecoder;
use io_utils::*;
use std::process::Command;
use std::{
    fs::File,
    io::{prelude::*, BufReader},
};
use string_utils::*;

// This allows either a fasta file or a gzipped one.  This APPENDS to the
// dv and headers vectors.
// ◼ Kill the code duplication below.

pub fn read_fasta_into_vec_dna_string_plus_headers(
    f: &String,
    dv: &mut Vec<DnaString>,
    headers: &mut Vec<String>,
) {
    if !f.ends_with(".gz") {
        let fin = open_for_read![&f];
        let mut last: String = String::new();
        let mut first = true;
        for line in fin.lines() {
            let s = line.unwrap();
            if first {
                if !s.starts_with(">") {
                    panic!("fasta format failure reading {}", f);
                }
                first = false;
                headers.push(s.get(1..).unwrap().to_string());
            } else {
                if s.starts_with(">") {
                    dv.push(DnaString::from_dna_string(&last));
                    last.clear();
                    headers.push(s.get(1..).unwrap().to_string());
                } else {
                    last += &s;
                }
            }
        }
        dv.push(DnaString::from_dna_string(&last));
    } else {
        let gz = MultiGzDecoder::new(std::fs::File::open(&f).unwrap());
        let fin = std::io::BufReader::new(gz);
        let mut last: String = String::new();
        let mut first = true;
        for line in fin.lines() {
            let s = line.unwrap();
            if first {
                if !s.starts_with(">") {
                    panic!("fasta format failure");
                }
                first = false;
                headers.push(s.get(1..).unwrap().to_string());
            } else {
                if s.starts_with(">") {
                    dv.push(DnaString::from_dna_string(&last));
                    last.clear();
                    headers.push(s.get(1..).unwrap().to_string());
                } else {
                    last += &s;
                }
            }
        }
        dv.push(DnaString::from_dna_string(&last));
    }
}

pub fn read_fasta_contents_into_vec_dna_string_plus_headers(
    f: &String,
    dv: &mut Vec<DnaString>,
    headers: &mut Vec<String>,
) {
    let mut last: String = String::new();
    let mut first = true;
    let lines = f.split('\n').collect::<Vec<&str>>();
    for i in 0..lines.len() {
        let s = &lines[i];
        if first {
            if !s.starts_with(">") {
                panic!("fasta format failure reading {}", f);
            }
            first = false;
            headers.push(s.get(1..).unwrap().to_string());
        } else {
            if s.starts_with(">") {
                dv.push(DnaString::from_dna_string(&last));
                last.clear();
                headers.push(s.get(1..).unwrap().to_string());
            } else {
                last += &s;
            }
        }
    }
    dv.push(DnaString::from_dna_string(&last));
}

// This APPENDS.

pub fn read_fasta_headers(f: &String, headers: &mut Vec<String>) {
    let fin = open_for_read![&f];
    let mut last: String = String::new();
    let mut first = true;
    for line in fin.lines() {
        let s = line.unwrap();
        if first {
            if !s.starts_with(">") {
                panic!("fasta format failure reading {}", f);
            }
            first = false;
            headers.push(s.get(1..).unwrap().to_string());
        } else {
            if s.starts_with(">") {
                last.clear();
                headers.push(s.get(1..).unwrap().to_string());
            } else {
                last += &s;
            }
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// LOAD GENBANK ACCESSION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn load_genbank_accession(accession: &String, bases: &mut DnaString) {
    let link = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/\
         efetch.fcgi?db=nucleotide&amp;id={}&amp;rettype=fasta",
        accession
    );
    let o = Command::new("csh")
        .arg("-c")
        .arg(format!("curl \"{}\"", link))
        .output()
        .expect("failed to execute curl command");
    let fasta = String::from_utf8(o.stdout).unwrap();
    let mut fasta = fasta.after("\n").to_string();
    // ◼ The following assert should not be necessary: the DnaString constructor
    // ◼ should gag if it gets nonsense input.
    assert!(!fasta.contains("moved"));
    fasta = fasta.replace("\n", "");
    *bases = DnaString::from_dna_string(&fasta);
}

