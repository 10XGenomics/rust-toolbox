// Copyright (c) 2018 10x Genomics, Inc. All rights reserved.

// Read a fasta file.

use debruijn::dna_string::DnaString;
use flate2::read::MultiGzDecoder;
use io_utils::open_for_read;
use std::process::Command;
use std::{io::prelude::*, path::Path};
use string_utils::TextUtils;

// Read a fasta file or gzipped fasta file and convert to a Vec<Vec<u8>>, in which
// outer vec entries alternate between header lines and base lines.

pub fn read_fasta_to_vec_vec_u8(f: impl AsRef<Path>) -> Vec<Vec<u8>> {
    let mut x = Vec::<Vec<u8>>::new();
    let f = f.as_ref();
    match f.extension() {
        Some(ex) if ex != "gz" => {
            let fin = open_for_read![f];
            let mut last: String = String::new();
            let mut first = true;
            for line in fin.lines() {
                let s = line.unwrap();
                if first {
                    if !s.starts_with('>') {
                        panic!("fasta format failure reading {}", f.to_string_lossy());
                    }
                    first = false;
                    x.push(s.get(1..).unwrap().as_bytes().to_vec());
                } else if s.starts_with('>') {
                    x.push(last.as_bytes().to_vec());
                    last.clear();
                    x.push(s.get(1..).unwrap().as_bytes().to_vec());
                } else {
                    last += &s;
                }
            }
            x.push(last.as_bytes().to_vec());
        }
        _ => {
            let gz = MultiGzDecoder::new(std::fs::File::open(&f).unwrap());
            let fin = std::io::BufReader::new(gz);
            let mut last: String = String::new();
            let mut first = true;
            for line in fin.lines() {
                let s = line.unwrap();
                if first {
                    if !s.starts_with('>') {
                        panic!("fasta format failure");
                    }
                    first = false;
                    x.push(s.get(1..).unwrap().as_bytes().to_vec());
                } else if s.starts_with('>') {
                    x.push(last.as_bytes().to_vec());
                    last.clear();
                    x.push(s.get(1..).unwrap().as_bytes().to_vec());
                } else {
                    last += &s;
                }
            }
            x.push(last.as_bytes().to_vec());
        }
    };
    x
}

// This allows either a fasta file or a gzipped one.  This APPENDS to the
// dv and headers vectors.
// ◼ Kill the code duplication below.

pub fn read_fasta_into_vec_dna_string_plus_headers(
    f: impl AsRef<Path>,
    dv: &mut Vec<DnaString>,
    headers: &mut Vec<String>,
) {
    let f = f.as_ref();
    match f.extension() {
        Some(ex) if ex == "gz" => {
            let fin = open_for_read![f];
            let mut last: String = String::new();
            let mut first = true;
            for line in fin.lines() {
                let s = line.unwrap();
                if first {
                    if !s.starts_with('>') {
                        panic!("fasta format failure reading {}", f.to_string_lossy());
                    }
                    first = false;
                    headers.push(s.get(1..).unwrap().to_string());
                } else if s.starts_with('>') {
                    dv.push(DnaString::from_dna_string(&last));
                    last.clear();
                    headers.push(s.get(1..).unwrap().to_string());
                } else {
                    last += &s;
                }
            }
            dv.push(DnaString::from_dna_string(&last));
        }
        _ => {
            let gz = MultiGzDecoder::new(std::fs::File::open(&f).unwrap());
            let fin = std::io::BufReader::new(gz);
            let mut last: String = String::new();
            let mut first = true;
            for line in fin.lines() {
                let s = line.unwrap();
                if first {
                    if !s.starts_with('>') {
                        panic!("fasta format failure");
                    }
                    first = false;
                    headers.push(s.get(1..).unwrap().to_string());
                } else if s.starts_with('>') {
                    dv.push(DnaString::from_dna_string(&last));
                    last.clear();
                    headers.push(s.get(1..).unwrap().to_string());
                } else {
                    last += &s;
                }
            }
            dv.push(DnaString::from_dna_string(&last));
        }
    };
}

pub fn read_fasta_contents_into_vec_dna_string_plus_headers(
    f: &str,
    dv: &mut Vec<DnaString>,
    headers: &mut Vec<String>,
) {
    let mut last: String = String::new();
    let mut first = true;
    let lines = f.split('\n').collect::<Vec<&str>>();
    for i in 0..lines.len() {
        let s = &lines[i];
        if first {
            if !s.starts_with('>') {
                panic!("fasta format failure reading {}", f);
            }
            first = false;
            headers.push(s.get(1..).unwrap().to_string());
        } else if s.starts_with('>') {
            dv.push(DnaString::from_dna_string(&last));
            last.clear();
            headers.push(s.get(1..).unwrap().to_string());
        } else {
            last += s;
        }
    }
    dv.push(DnaString::from_dna_string(&last));
}

// This APPENDS.

pub fn read_fasta_headers(f: impl AsRef<Path>, headers: &mut Vec<String>) {
    let f = f.as_ref();
    let fin = open_for_read![f];
    let mut last: String = String::new();
    let mut first = true;
    for line in fin.lines() {
        let s = line.unwrap();
        if first {
            if !s.starts_with('>') {
                panic!("fasta format failure reading {}", f.to_string_lossy());
            }
            first = false;
            headers.push(s.get(1..).unwrap().to_string());
        } else if s.starts_with('>') {
            last.clear();
            headers.push(s.get(1..).unwrap().to_string());
        } else {
            last += &s;
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// LOAD GENBANK ACCESSION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn load_genbank_accession(accession: &str, bases: &mut DnaString) {
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
    let fasta = String::from_utf8(o.stdout).expect("Response was not valid utf-8.");
    let mut fasta = fasta.after("\n").to_string();
    // ◼ The following assert should not be necessary: the DnaString constructor
    // ◼ should gag if it gets nonsense input.
    assert!(!fasta.contains("moved"));
    fasta = fasta.replace("\n", "");
    *bases = DnaString::from_dna_string(&fasta);
}
