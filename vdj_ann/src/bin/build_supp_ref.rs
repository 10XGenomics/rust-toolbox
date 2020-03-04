// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Build a supplementary VDJ reference, in the following way.  We find first exons
// for TCR V, D and J genes from the Ensemble gtf.  The supplementary sequences
// consist of up to 600 bases immediately before these exons.  We exclude those
// cases where the bases overlap another gtf exon.
//
// Also add some special sequences.
//
// ◼ This has a hardcoded and inappropriate fasta location.
// ◼ The code writes to a temporary fasta location, like build_vdj_ref.rs.
// ◼ If would be better if this were built by build_vdj_ref.rs and in a consistent
//   manner with it.
//
// Does the main event here occur in clones?  See Mike's hypothesis.  Alternatively,
// one could imagine that what we're seeing is a common failure mode of
// VDJ reaarrangement.
//
// Pile of additional examples: to work through.
//
// USAGE (ASSUMING YOU ARE RUNNING FROM TOP LEVEL OF REPO):
//
// build_supp_ref HUMAN > vdj_ann/vdj_refs/human/supp_regions.fa
// build_supp_ref MOUSE > vdj_ann/vdj_refs/mouse/supp_regions.fa
//
// And these also need to go into the references on /mnt/opt and possibly /mnt/test.

extern crate debruijn;
extern crate exons;
extern crate fasta;
extern crate pretty_trace;
extern crate string_utils;

// extern crate vdj_asm_utils;
// use vdj_asm_utils::*;

use debruijn::{dna_string::*, *};
use exons::*;
use fasta::*;
use pretty_trace::*;
use std::{collections::HashMap, env};
use string_utils::TextUtils;

fn print_fasta(header: &str, seq: &DnaStringSlice) {
    println!(">{}\n{}", header, seq.to_string());
}

fn main() {
    // Force panic to yield a traceback, and make it a pretty one.

    PrettyTrace::new().on();

    // Parse arguments.

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Please supply exactly one argument.");
        std::process::exit(1);
    }
    let species: &str;
    if args[1] == "HUMAN" {
        species = "human";
    } else if args[1] == "MOUSE" {
        species = "mouse";
    } else {
        panic!("Unknown species.");
    }

    // Load the exons.

    let mut exons = Vec::<(String, i32, i32, bool, String, i32)>::new();
    fetch_exons(&species.to_string(), &mut exons);

    // Hardcoded and inappropriate fasta location.
    // source = ftp://ftp.ensembl.org/pub/release-94/fasta/homo_sapiens/
    //          dna/Homo_sapiens.GRCh38.dna.primary_assembly.fa.gz
    // DO NOT USE
    //          ftp://ftp.ensembl.org/pub/release-94/fasta/homo_sapiens/
    //          dna/Homo_sapiens.GRCh38.dna.toplevel.fa.gz
    // because it unzips to a 54 GB file in which 95% of the bases are Ns.
    // Notice weirdness in the process: somehow much faster to first download
    // to laptop.
    //
    // Addendum: above outdated, see build_vdj_ref.rs.

    let fasta: String;
    if species == "human" {
        fasta = "/mnt/assembly/peeps/jaffe/\
                 Homo_sapiens.GRCh38.dna.primary_assembly.fa"
            .to_string();
    } else {
        fasta = "/mnt/opt/meowmix_git/ensembl/release-94/fasta/mus_musculus/dna/\
                 Mus_musculus.GRCm38.dna.toplevel.fa.gz"
            .to_string();
    }

    // Load fasta.
    // ◼ This is too slow for human.

    let mut refs = Vec::<DnaString>::new();
    let mut rheaders = Vec::<String>::new();
    read_fasta_into_vec_dna_string_plus_headers(&fasta, &mut refs, &mut rheaders);
    let mut to_chr = HashMap::new();
    for i in 0..rheaders.len() {
        let chr = rheaders[i].before(" ");
        to_chr.insert(chr.to_string(), i);
    }

    // Fix size.

    const MIN_EXTRA: usize = 60;
    const MAX_EXTRA: i32 = 6000;

    // First define the exons we're using.

    let mut exons2 = Vec::<(String, i32, i32, bool, String, i32)>::new();
    for i in 0..exons.len() {
        // Unpack data.

        exons[i].4 = exons[i].4.to_uppercase();
        let gene = &exons[i].4;
        let exon = exons[i].5;

        // Restrict to certain intervals.

        // Only use TR{A,B}{V,D,J,C}.  Or TRDD, not sure what those are.
        // ◼ Well, what's going on with TRDD?
        // Added some IG.

        if !gene.starts_with("TRAV")
            && !gene.starts_with("TRAD")
            && !gene.starts_with("TRAJ")
            && !gene.starts_with("TRAC")
            && !gene.starts_with("TRBV")
            && !gene.starts_with("TRBD")
            && !gene.starts_with("TRBJ")
            && !gene.starts_with("TRBC")
            && !gene.starts_with("TRDD")
            && !gene.starts_with("IG")
        {
            continue;
        }

        // Ignore cases where we've already seen the exon.

        if i > 0 && *gene == exons[i - 1].4 && exon == exons[i - 1].5 {
            continue;
        }

        // Save.

        exons2.push(exons[i].clone());
    }

    // Go through the exons we're using.

    for i in 1..exons2.len() {
        // Unpack data.

        let chr = &exons2[i].0;
        let start = exons2[i].1;
        let gene = &exons2[i].4;
        let exon = exons2[i].5;

        // Ignore cases where there's not enough room.

        let gap = start - exons2[i - 1].2;
        if gap < 0 {
            continue;
        }
        if i > 0 && *chr == exons2[i - 1].0 && gap < MIN_EXTRA as i32 {
            continue;
        }
        if !to_chr.contains_key(&chr.to_string()) {
            continue;
        }
        let chrid = to_chr[&chr.to_string()];

        // Case 1.  Gap is not too long, use it all.  Create sequence and emit as
        // rust code.  Then code the rc.

        if gap <= 2 * MAX_EXTRA {
            let len = gap;
            let (xstart, xstop) = (start - len, start);
            let header = format!("segment before {} exon {}", gene, exon);
            let seq = refs[chrid].slice(xstart as usize, xstop as usize);
            print_fasta(&header, &seq);
            let seq_rc = seq.rc();
            let header_rc = format!("rc of {}", header);
            print_fasta(&header_rc, &seq_rc);
        }
        // Case 2.  Gap is really long, do both sides.
        else {
            let len = MAX_EXTRA;
            let (xstart, xstop) = (start - len, start);
            let header = format!("segment before {} exon {}", gene, exon);
            let seq = refs[chrid].slice(xstart as usize, xstop as usize);
            print_fasta(&header, &seq);
            let seq_rc = seq.rc();
            let header_rc = format!("rc of {}", header);
            print_fasta(&header_rc, &seq_rc);
            let (xstart, xstop) = (start - gap, start - gap + len);
            let header = format!("segment after {} exon {}", exons2[i - 1].4, exons2[i - 1].5);
            let seq = refs[chrid].slice(xstart as usize, xstop as usize);
            print_fasta(&header, &seq);
            let seq_rc = seq.rc();
            let header_rc = format!("rc of {}", header);
            print_fasta(&header_rc, &seq_rc);
        }
    }
}
