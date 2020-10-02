// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Build reference sequence from Ensembl files.  This uses .gtf, .gff3, and .fa
// files.  The .gff3 file is used only to demangle gene names.  It is not possible
// to use the .gff3 (and not the .gff) to get all the gene info because it's not
// all present in the .gff3.
//
// This finds the coordinates of all TCR/BCR genes on the reference and then
// extracts the sequence from the reference.
// NO LONGER ACCURATE:
// In cases where a given gene is
// present on a chromosome record and also on one or more alt records, we pick
// the chromosome record.  Otherwise if it is present on more than one alt record,
// we pick the lexicographically minimal record name.  See "exceptions" below for
// special handling for particular genes.
//
// HOW TO USE THIS
//
// 1. Download files from Ensembl:
//    build_vdj_ref DOWNLOAD
//    You don't need to do this unless you're updating to a new Ensembl release.
//
// 2. Create reference files:
//    build_vdj_ref HUMAN
//    build_vdj_ref MOUSE
//    *** These must be run from the root of the repo! ***
//    You don't need to do this unless you're changing this code.
//
//    These files get ultimately moved to:
//    /mnt/opt/refdata_cellranger/vdj/
//        vdj_GRCh38_alts_ensembl-*.*.*
//        vdj_GRCm38_alts_ensembl-*.*.*
//    with the cellranger release version substituted in.  This will not work
//    immediately on pluto, and may depend on overnight auto-syncronization to
//    the pluto filesystem.
//
//    To make jenkins work, the human files also need to be copied to
//    /mnt/test/refdata/testing/vdj_GRCh38_alts_ensembl
//    by someone who has permission to do that.
//
//    Experimental (assuming the right files have been downloaded from ensembl):
//    build_vdj_ref BALBC
//    (won't work now because will try to write to a directory that doesn't exist)
//    However, this works poorly.  Many genes are missing.  Here are a few examples:
//
//    gene            in whole-genome   in BALB/c data
//                    BALB/c assembly   e.g. lena 77990
//
//    IGKV4-53        no                yes
//    IGHV12-1        no                yes
//    IGHV1-unknown1  no                yes.
//
// 3. For debugging:
//    build_vdj_ref NONE  [no fasta output].
//
// TODO
// ◼ Decide what the exon structure of C segments should be.
//
// ◼ Genes added by sequence appear as if in GRCh or GRCm, but this is wrong.
// ◼ Should look for GenBank accessions that have these.
//
// See also build_supp_ref.rs.
//
// Observed differences with IMGT for human TCR:
//
// 1. Output here includes 5' UTRs.
// 2. Our TRAV6 is 57 bases longer on the 5' end.  We see full length alignemnts
//    of our TRAV6 to transcripts so our TRAV6 appears to be correct.
// 3. We don't have the broken transcript TRBV20-1*01.
// 4. We exclude many pseudogenes.
//
// Observed differences with "GRCh" reference for human TCR:
//
// 1. Our C segments are correct.
// 2. Our L+V segments start with start codons.
// 3. Our TRBV11-2 and TRAJ37 are correct.
//
// For both: this code has the advantage of producing reproducible results from
// defined external files.

use debruijn::{dna_string::*, *};
use fasta_tools::*;
use flate2::read::MultiGzDecoder;
use pretty_trace::PrettyTrace;
use process::Command;
use sha2::*;
use std::io::copy;
use std::io::Write;
use std::{
    collections::HashMap,
    env, fs,
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    *,
};
use string_utils::*;
use vector_utils::*;

use io_utils::{fwrite, fwriteln, open_for_read, open_for_write_new};

fn header_from_gene(gene: &str, is_utr: bool, record: &mut usize, source: &str) -> String {
    let mut gene = gene.to_string();
    if gene.ends_with(" ") {
        gene = gene.rev_before(" ").to_string();
    }
    let genev = gene.as_bytes();
    let mut xx = "None";
    if gene == "IGHD"
        || gene == "IGHE"
        || gene == "IGHM"
        || gene.starts_with("IGHG")
        || gene.starts_with("IGHA")
    {
        xx = gene.after("IGH");
    }
    let header_tail = format!(
        "{}{}|{}{}{}|{}|00",
        genev[0] as char,
        genev[1] as char,
        genev[0] as char,
        genev[1] as char,
        genev[2] as char,
        xx
    );
    *record += 1;
    let region_type: String;
    if is_utr {
        region_type = "5'UTR".to_string();
    } else if gene == "IGHD"
        || gene == "IGHE"
        || gene == "IGHM"
        || gene.starts_with("IGHG")
        || gene.starts_with("IGHA")
    {
        region_type = "C-REGION".to_string();
    } else if genev[3] == b'V' {
        region_type = "L-REGION+V-REGION".to_string();
    } else {
        region_type = format!("{}-REGION", genev[3] as char);
    }
    format!(
        "{}|{} {}|{}|{}|{}",
        record, gene, source, gene, region_type, header_tail
    )
}

fn print_fasta<R: Write>(out: &mut R, header: &str, seq: &DnaStringSlice, none: bool) {
    if none {
        return;
    }
    fwriteln!(out, ">{}\n{}", header, seq.to_string());
}

fn print_oriented_fasta<R: Write>(
    out: &mut R,
    header: &str,
    seq: &DnaStringSlice,
    fw: bool,
    none: bool,
) {
    if none {
        return;
    }
    if fw {
        print_fasta(out, header, seq, none);
    } else {
        let seq_rc = seq.rc();
        print_fasta(out, header, &seq_rc, none);
    }
}

// add_gene: coordinates are one-based

fn add_gene<R: Write>(
    out: &mut R,
    gene: &str,
    record: &mut usize,
    chr: &str,
    start: usize,
    stop: usize,
    to_chr: &HashMap<String, usize>,
    refs: &Vec<DnaString>,
    none: bool,
    is_utr: bool,
    source: &str,
) {
    if none {
        return;
    }
    if !to_chr.contains_key(&chr.to_string()) {
        eprintln!("gene = {}, chr = {}", gene, chr);
    }
    let chrid = to_chr[chr];
    let seq = refs[chrid].slice(start - 1, stop);
    let header = header_from_gene(&gene, is_utr, record, source);
    print_fasta(out, &header, &seq.slice(0, seq.len()), none);
}

// two exon version

fn add_gene2<R: Write>(
    out: &mut R,
    gene: &str,
    record: &mut usize,
    chr: &str,
    start1: usize,
    stop1: usize,
    start2: usize,
    stop2: usize,
    to_chr: &HashMap<String, usize>,
    refs: &Vec<DnaString>,
    none: bool,
    fw: bool,
    source: &str,
) {
    if none {
        return;
    }
    let chrid = to_chr[chr];
    let seq1 = refs[chrid].slice(start1 - 1, stop1);
    let seq2 = refs[chrid].slice(start2 - 1, stop2);
    let mut seq = seq1.to_owned();
    for i in 0..seq2.len() {
        seq.push(seq2.get(i));
    }
    if !fw {
        seq = seq.rc();
    }
    let header = header_from_gene(&gene, false, record, source);
    print_fasta(out, &header, &seq.slice(0, seq.len()), none);
}

fn parse_gtf_file(
    gtf: &str,
    demangle: &HashMap<String, String>,
    exons: &mut Vec<(String, String, String, i32, i32, String, bool, String)>,
) {
    let f = open_for_read![&gtf];
    exons.clear();
    for line in f.lines() {
        let s = line.unwrap();

        let fields: Vec<&str> = s.split_terminator('\t').collect();
        if fields.len() < 9 {
            continue;
        }
        let fields8: Vec<&str> = fields[8].split_terminator(';').collect();
        if fields8.len() < 6 {
            continue;
        }

        // Get type of entry.  If it's called a pseudogene and the type is exon,
        // change it to CDS.

        let mut biotype = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" gene_biotype") {
                biotype = fields8[i].between("\"", "\"").to_string();
            }
        }
        let mut cat = fields[2];
        if biotype.contains("pseudogene") && cat == "exon" {
            cat = "CDS";
        }
        if !biotype.starts_with("TR_") && !biotype.starts_with("IG_") {
            continue;
        }

        // Exclude certain types.

        if cat == "gene" {
            continue;
        }
        if cat == "transcript" || cat == "exon" {
            continue;
        }
        if cat == "start_codon" || cat == "stop_codon" {
            continue;
        }
        if cat == "three_prime_utr" {
            continue;
        }

        // Get gene name and demangle.

        let mut gene = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" gene_name") {
                gene = fields8[i].between("\"", "\"").to_string();
            }
        }
        gene = gene.to_uppercase();
        let mut gene2: String;
        if demangle.contains_key(&gene) {
            gene2 = demangle[&gene.clone()].clone();
        } else {
            continue;
        }

        // Special fixes.  Here the gff3 file is trying to impose a saner naming
        // scheme on certain genes, but we're sticking with the scheme that people
        // use.

        if gene2.starts_with("IGHCA") {
            gene2 = gene2.replace("IGHCA", "IGHA");
        }
        if gene2 == "IGHCD" {
            gene2 = "IGHD".to_string();
        }
        if gene2 == "IGHCE" {
            gene2 = "IGHE".to_string();
        }
        if gene2.starts_with("IGHCG") {
            gene2 = gene2.replace("IGHCG", "IGHG");
        }
        if gene2 == "IGHCM" {
            gene2 = "IGHM".to_string();
        }

        // For now, require havana (except for mouse strains).  Could try turning
        // this off, but there may be some issues.

        if !fields[1].contains("havana") && fields[1] != "mouse_genomes_project" {
            continue;
        }

        // Get transcript name.

        let mut tr = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" transcript_name") {
                tr = fields8[i].between("\"", "\"").to_string();
            }
        }

        // Get transcript id.

        let mut trid = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" transcript_id") {
                trid = fields8[i].between("\"", "\"").to_string();
            }
        }

        // Save in exons.

        let chr = fields[0];
        let start = fields[3].force_i32() - 1;
        let stop = fields[4].force_i32();
        let mut fw = false;
        if fields[6] == "+" {
            fw = true;
        }
        exons.push((
            gene2,
            tr,
            chr.to_string(),
            start,
            stop,
            cat.to_string(),
            fw,
            trid,
        ));
    }
    exons.sort();
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
    let mut none = false;
    let mut download = false;
    let mut species = String::new();
    match args[1].as_str() {
        "DOWNLOAD" => {
            download = true;
        }
        "HUMAN" => {
            species = "human".to_string();
        }
        "MOUSE" => {
            species = "mouse".to_string();
        }
        "BALBC" => {
            species = "balbc".to_string();
        }
        "NONE" => {
            none = true;
            species = "human".to_string();
        }
        _ => {
            eprintln!("Call with DOWNLOAD or HUMAN or MOUSE or NONE.");
            std::process::exit(1);
        }
    }

    // Define root output directory.

    let root = format!("vdj_ann/vdj_refs");
    let mut out = open_for_write_new![&format!("{}/{}/fasta/regions.fa", root, species)];

    // Define release.  If this is ever changed, the effect on the fasta output
    // files should be very carefully examined.  Specify sequence source.
    // Note that source2 depends on the cellranger version!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

    let release = 94;
    let version = "4.0.0";
    let source: String;
    let source2: String;
    if species == "human" {
        source = format!("GRCh38-release{}", release);
        source2 = format!("vdj_GRCh38_alts_ensembl-{}", version);
    } else if species == "mouse" {
        source = format!("GRCm38-release{}", release);
        source2 = format!("vdj_GRCm38_alts_ensembl-{}", version);
    } else {
        source = format!("BALB_cJ_v1.{}", release);
        source2 = source.clone();
    }

    // Define local directory.

    let internal = "/mnt/opt/meowmix_git/ensembl";

    // Set up for exceptions.  Coordinates are the usual 1-based coordinates used in
    // genomics.  If the bool field ("fw") is false, the given coordinates are used
    // to extract a sequence, and then it is reversed.

    let excluded_genes = vec![];
    let mut allowed_pseudogenes = Vec::<&str>::new();
    let mut deleted_genes = Vec::<&str>::new();
    let mut added_genes = Vec::<(&str, &str, usize, usize, bool)>::new();
    let mut added_genes2 = Vec::<(&str, &str, usize, usize, usize, usize, bool)>::new();
    let mut added_genes2_source = Vec::<(&str, usize, usize, usize, usize, bool, String)>::new();
    let mut left_trims = Vec::<(&str, usize)>::new();
    let mut right_trims = Vec::<(&str, i32)>::new();
    let mut added_genes_seq = Vec::<(&str, &str)>::new();

    // Define exceptions.

    if species == "human" {
        deleted_genes.push("IGHV1/OR15-9");
        deleted_genes.push("TRGV11");
        // deleting because nearly identical to TRBV6-2 and not in gtf:
        deleted_genes.push("TRBV6-3");
        allowed_pseudogenes.push("TRAJ8");
        allowed_pseudogenes.push("TRAV35");
        added_genes.push(("TRBD2", "7", 142795705, 142795720, false));
        added_genes.push(("TRAJ15", "14", 22529629, 22529688, false));
        added_genes.push(("IGLV6-57", "22", 22195713, 22195798, true)); // UTR

        // Not sure why this was here.  It doesn't start with ATG and is rc to
        // another gene perfectly IGKV2D-40 except longer.
        /*
        added_genes2.push( ( "IGKV2-40", "2", 89852177, 89852493,
            89851758, 89851805, false ) );
        */

        added_genes2.push((
            "TRBV11-2", "7", 142433956, 142434001, 142434094, 142434389, true,
        ));
        added_genes2_source.push((
            "TRGV11",
            107142,
            107184,
            107291,
            107604,
            true,
            "AC244625.2".to_string(),
        ));
        added_genes2_source.push((
            "IGHV1-8",
            86680235,
            86680541,
            86680628,
            86680673,
            false,
            "AC_000146.1".to_string(),
        ));
        right_trims.push(("TRAJ36", -1));
        right_trims.push(("TRAJ37", 3));
        left_trims.push(("IGLJ1", 89));
        left_trims.push(("IGLJ2", 104));
        left_trims.push(("IGLJ3", 113));
        left_trims.push(("TRBV20/OR9-2", 57));
        left_trims.push(("IGHA1", 1));
        left_trims.push(("IGHA2", 1));
        left_trims.push(("IGHE", 1));
        left_trims.push(("IGHG1", 1));
        left_trims.push(("IGHG2", 1));
        left_trims.push(("IGHG4", 1));
        left_trims.push(("IGHM", 1));

        // Add another allele of IGHJ6.

        added_genes_seq.push((
            "IGHJ6",
            "ATTACTACTACTACTACGGTATGGACGTCTGGGGCCAAGGGACCACGGTCACCGTCTCCTCAG",
        ));

        // Insertion of 3 bases on TRBV20-1 as indicated below.  Note that we use
        // the same name.

        added_genes_seq.push((
            "TRBV20-1",
            "ATGCTGCTGCTTCTGCTGCTTCTGGGGCCAG\
             CAG\
             GCTCCGGGCTTGGTGCTGTCGTCTCTCAACATCCGAGCAGGGTTATCTGTAAGAGTGGAACCTCTGTGAAG\
             ATCGAGTGCCGTTCCCTGGACTTTCAGGCCACAACTATGTTTTGGTATCGTCAGTTCCCGAAACAGAGTCT\
             CATGCTGATGGCAACTTCCAATGAGGGCTCCAAGGCCACATACGAGCAAGGCGTCGAGAAGGACAAGTTTC\
             TCATCAACCATGCAAGCCTGACCTTGTCCACTCTGACAGTGACCAGTGCCCATCCTGAAGACAGCAGCTTC\
             TACATCTGCAGTGCTAGAGA",
        ));

        // Insertion of 15 bases on TRBV7-7 as indicated below.  Note that we use
        // the same name.

        added_genes_seq.push((
            "TRBV7-7",
            "ATGGGTACCAGTCTCCTATGCTGGGTGGTCCTGGGTTTCCTAGGG\
             ACAGATTCTGTTTCC\
             ACAGATCACACAGGTGCTGGAGTCTCCCAGTCTCCCAGGTACAAAGTCACAAAGAGGGGACAGGATGTAAC\
             TCTCAGGTGTGATCCAATTTCGAGTCATGCAACCCTTTATTGGTATCAACAGGCCCTGGGGCAGGGCCCAG\
             AGTTTCTGACTTACTTCAATTATGAAGCTCAACCAGACAAATCAGGGCTGCCCAGTGATCGGTTCTCTGCA\
             GAGAGGCCTGAGGGATCCATCTCCACTCTGACGATTCAGCGCACAGAGCAGCGGGACTCAGCCATGTATCG\
             CTGTGCCAGCAGCTTAGC",
        ));

        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        // Begin human changes for cell ranger 4.1.
        // (see also mouse changes, below)
        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

        // 1. Replace IGKV2D-40.  It has a leader sequence of length 9 amino acids, which is an
        // extreme low outlier, and we observe in the whole genome reference and in 10x data a
        // left extension of it whose leader is 20 amino acids long, as expected, and which has a
        // leucine-rich stretch, as expected, unlike the short leader.

        deleted_genes.push("IGKV2D-40");
        added_genes2.push((
            "IGKV2D-40",
            "2",
            89851758,
            89851806,
            89852178,
            89852493,
            true,
        ));

        // 2. Delete IGKV2-18.  We previously added this gene to our reference but it is listed
        // in some places as a pseudogene, and the sequence we provided had a leader of length
        // 9 amino acids, which is an extremely short outlier.  The IMGT sequence for IGKV2-18
        // does not begin with a start codon.  We observe in all cases examined (more than 50),
        // that when IGKV2-18 appears in 10x data, it appears with a heavy chain and ANOTHER light
        // chain, which is odd.  We implemented this change by commenting out the previous
        // addition lines, and moved them here.

        // added_genes2.push((
        //     "IGKV2-18", "2", 89128701, 89129017, 89129435, 89129449, false,
        // ));

        // 3. Delete IGLV5-48.  This is truncated on the right.

        deleted_genes.push("IGLV5-48");

        // 4. Our previouse notes for TRBV21-1 said that our version began with a start codon.
        // That's great, but it has multiple frameshifts, we don't see it in 10x data, and it is
        // annotated as a pseudogene.  Therefore we are "unallowing" it here.  Note that there
        // are two versions.

        // allowed_pseudogenes.push("TRBV21-1");

        // 5. Add a gene that is present in the human reference and in our data, but which
        // we missed.

        added_genes_seq.push(("IGHV4-30-4",
            "ATGAAACACCTGTGGTTCTTCCTCCTGCTGGTGGCAGCTCCCAGATGGGTCCTGTCCCAGCTGCAGCTGCAGGAGTCGGGCCCAGGACTGGTGAAGCCTTCACAGACCCTGTCCCTCACCTGCACTGTCTCTGGTGGCTCCATCAGCAGTGGTGATTACTACTGGAGCTGGATCCGCCAGCCCCCAGGGAAGGGCCTGGAGTGGATTGGGTACATCTATTACAGTGGGAGCACCTACTACAACCCGTCCCTCAAGAGTCGAGTTACCATATCAGTAGACACGTCCAAGAACCAGTTCTCCCTGAAGCTGAGCTCTGTGACTGCCGCAGACACGGCCGTGTATTACTGT"));

        // 6. Add a gene that is present in the human reference and in our data, but which
        // we missed.

        added_genes_seq.push(("IGKV1-NL1",
            "ATGGACATGAGGGTCCCCGCTCAGCTCCTGGGGCTCCTGCTGCTCTGGCTCCCAGGTACCAGATGTGACATCCAGATGACCCAGTCTCCATCCTCCCTGTCTGCATCTGTAGGAGACAGAGTCACCATCACTTGCCGGGCGAGTCAGGGCATTAGCAATTCTTTAGCCTGGTATCAGCAGAAACCAGGGAAAGCCCCTAAGCTCCTGCTCTATGCTGCATCCAGATTGGAAAGTGGGGTCCCATCCAGGTTCAGTGGCAGTGGATCTGGGACGGATTACACTCTCACCATCAGCAGCCTGCAGCCTGAAGATTTTGCAACTTATTACTGT"));

        // 7. Add a gene that is present in the human reference and in our data, but which
        // we missed.

        added_genes_seq.push(("IGHV4-38-2",
            "ATGAAGCACCTGTGGTTTTTCCTCCTGCTGGTGGCAGCTCCCAGATGGGTCCTGTCCCAGGTGCAGCTGCAGGAGTCGGGCCCAGGACTGGTGAAGCCTTCGGAGACCCTGTCCCTCACCTGCACTGTCTCTGGTTACTCCATCAGCAGTGGTTACTACTGGGGCTGGATCCGGCAGCCCCCAGGGAAGGGGCTGGAGTGGATTGGGAGTATCTATCATAGTGGGAGCACCTACTACAACCCGTCCCTCAAGAGTCGAGTCACCATATCAGTAGACACGTCCAAGAACCAGTTCTCCCTGAAGCTGAGCTCTGTGACCGCCGCAGACACGGCCGTGTATTACTGT"));

        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        // End human changes for cell ranger 4.1.
        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
    }
    if species == "mouse" {
        // Doesn't start with start codon, is labeled a pseudogene by NCBI,
        // and not clear that we have an example that has a start codon ahead
        // of the official gene start.

        deleted_genes.push("IGHV1-67");

        // The following V segment is observed in BALB/c datasets 76836 and 77990,
        // although the sequence accession AJ851868.3 is 129S1.

        added_genes2_source.push((
            "IGHV12-1",
            361009,
            361054,
            361145,
            361449,
            true,
            "AJ851868.3".to_string(),
        ));

        // The following V segment is observed in BALB/c datasets 76836 and 77990.
        // The accession is an unplaced sequence in the Sanger assembly of BALB/c,
        // which is also on the Sanger ftp site at
        // ftp://ftp-mouse.sanger.ac.uk/current_denovo.
        // The best match to IMGT is to IGHV1-77 as shown below, and it equally well
        // matches IGHV1-66 and IGHV1-85.

        //           *       *                 *              * ***     **   *  *
        // ATGGAATGGAACTGGGTCGTTCTCTTCCTCCTGTCATTAACTGCAGGTGTCTATGCCCAGGGTCAGATGCAGCAGTCTGG
        // ATGGAATGGAGCTGGGTCTTTCTCTTCCTCCTGTCAGTAACTGCAGGTGTCCACTGCCAGGTCCAGCTGAAGCAGTCTGG
        //
        //                                   * *         *         *        *******
        // AGCTGAGCTGGTGAAGCCTGGGGCTTCAGTGAAGCTGTCCTGCAAGACTTCTGGCTTCACCTTCAGCAGTAGCTATATAA
        // AGCTGAGCTGGTGAAGCCTGGGGCTTCAGTGAAGATATCCTGCAAGGCTTCTGGCTACACCTTCACTGACTACTATATAA
        //
        // **   *       * *          * *             * **    ** *      *    *     **   *
        // GTTGGTTGAAGCAAAAGCCTGGACAGAGTCTTGAGTGGATTGCATGGATTTATGCTGGAACTGGTGGTACTAGCTATAAT
        // ACTGGGTGAAGCAGAGGCCTGGACAGGGCCTTGAGTGGATTGGAAAGATTGGTCCTGGAAGTGGTAGTACTTACTACAAT
        //
        // *         **         **        *     *                        **              *
        // CAGAAGTTCACAGGCAAGGCCCAACTGACTGTAGACACATCCTCCAGCACAGCCTACATGCAATTCAGCAGCCTGACAAC
        // GAGAAGTTCAAGGGCAAGGCCACACTGACTGCAGACAAATCCTCCAGCACAGCCTACATGCAGCTCAGCAGCCTGACATC
        //
        //             **      *
        // TGAGGACTCTGCCATCTATTACTGTGCAAGA
        // TGAGGACTCTGCAGTCTATTTCTGTGCAAGA

        // ◼ Correctly name this sequence.

        added_genes2_source.push((
            "IGHV1-unknown1",
            7084,
            7391,
            7475,
            7517,
            false,
            "LVXK01034187.1".to_string(),
        ));

        // Add form of TRAV4-4-DV10 seen in BALB/c.  Includes a 3-base indel and
        // SNPs.

        added_genes_seq.push((
            "TRAV4-4-DV10",
            "ATGCAGAGGAACCTGGGAGCTGTGCTGGGGATTCTGTGGGTGCAGATTTGCTGGGTGAGAGGGGATCAGG\
             TGGAGCAGAGTCCTTCAGCCCTGAGCCTCCACGAGGGAACCGATTCTGCTCTGAGATGCAATTTTACGACC\
             ACCATGAGGAGTGTGCAGTGGTTCCGACAGAATTCCAGGGGCAGCCTCATCAGTTTGTTCTACTTGGCTTC\
             AGGAACAAAGGAGAATGGGAGGCTAAAGTCAGCATTTGATTCTAAGGAGCGGCGCTACAGCACCCTGCACA\
             TCAGGGATGCCCAGCTGGAGGACTCAGGCACTTACTTCTGTGCTGCTGAGG",
        ));

        // Add form of TRAV13-1 or TRAV13D-1 seen in BALB/c.  Arbitrarily labeled
        // TRAV13-1.  Has 8 SNPs.

        added_genes_seq.push((
            "TRAV13-1",
            "ATGAACAGGCTGCTGTGCTCTCTGCTGGGGCTTCTGTGCACCCAGGTTTGCTGGGTGAAAGGACAGCAAG\
             TGCAGCAGAGCCCCGCGTCCTTGGTTCTGCAGGAGGGGGAGAATGCAGAGCTGCAGTGTAACTTTTCCACA\
             TCTTTGAACAGTATGCAGTGGTTTTACCAACGTCCTGAGGGAAGTCTCGTCAGCCTGTTCTACAATCCTTC\
             TGGGACAAAGCAGAGTGGGAGACTGACATCCACAACAGTCATCAAAGAACGTCGCAGCTCTTTGCACATTT\
             CCTCCTCCCAGATCACAGACTCAGGCACTTATCTCTGTGCTTTGGAAC",
        ));

        // Alt splicing, first exon of TRBV12-2 plus second exon of TRBV13-2,
        // very common.

        added_genes_seq.push((
            "TRBV12-2+TRBV13-2",
            "ATGTCTAACACTGCCTTCCCTGACCCCGCCTGGA\
             ACACCACCCTGCTATCTTGGGTTGCTCTCTTTCTCCTGGGAACAAAACACATGGAGGCTGCAGTCACCCAA\
             AGCCCAAGAAACAAGGTGGCAGTAACAGGAGGAAAGGTGACATTGAGCTGTAATCAGACTAATAACCACAA\
             CAACATGTACTGGTATCGGCAGGACACGGGGCATGGGCTGAGGCTGATCCATTATTCATATGGTGCTGGCA\
             GCACTGAGAAAGGAGATATCCCTGATGGATACAAGGCCTCCAGACCAAGCCAAGAGAACTTCTCCCTCATT\
             CTGGAGTTGGCTACCCCCTCTCAGACATCAGTGTACTTCTGTGCCAGCGGTGATG",
        ));

        // Insertion of 3 bases on TRAV16N as indicated below.  Note that we use
        // the same name.

        added_genes_seq.push((
            "TRAV16N",
            "ATGCTGATTCTAAGCCTGTTGGGAGCAGCCTTTGGCTCCATTTGTTTTGCA\
             GCA\
             ACCAGCATGGCCCAGAAGGTAACACAGACTCAGACTTCAATTTCTGTGGTGGAGAAGACAACGGTGACAAT\
             GGACTGTGTGTATGAAACCCGGGACAGTTCTTACTTCTTATTCTGGTACAAGCAAACAGCAAGTGGGGAAA\
             TAGTTTTCCTTATTCGTCAGGACTCTTACAAAAAGGAAAATGCAACAGTGGGTCATTATTCTCTGAACTTT\
             CAGAAGCCAAAAAGTTCCATCGGACTCATCATCACCGCCACACAGATTGAGGACTCAGCAGTATATTTCTG\
             TGCTATGAGAGAGGG",
        ));

        // Insertion of 3 bases on TRAV6N-5 as indicated below.  Note that we use
        // the same name.

        added_genes_seq.push((
            "TRAV6N-5",
            "ATGAACCTTTGTCCTGAACTGGGTATTCTACTCTTCCTAATGCTTTTTG\
             GAG\
             AAAGCAATGGAGACTCAGTGACTCAGACAGAAGGCCCAGTGACACTGTCTGAAGGGACTTCTCTGACTGTG\
             AACTGTTCCTATGAAACCAAACAGTACCCAACCCTGTTCTGGTATGTGCAGTATCCCGGAGAAGGTCCACA\
             GCTCCTCTTTAAAGTCCCAAAGGCCAACGAGAAGGGAAGCAACAGAGGTTTTGAAGCTACATACAATAAAG\
             AAGCCACCTCCTTCCACTTGCAGAAAGCCTCAGTGCAAGAGTCAGACTCGGCTGTGTACTACTGTGCTCTG\
             GGTGA",
        ));

        // Insertion of 15 bases on TRAV13N-4 as indicated below.  Actually this
        // appears to be the only form, so we should probably delete the form
        // we have, but not the UTR.  (Can't just push onto deleted_genes.)

        added_genes_seq.push((
            "TRAV13N-4",
            "ATGAAGAGGCTGCTGTGCTCTCTGCTGGGGCTCCTGTGCACCCAGGTTTGCT\
             GTGCTTCTCAATTAG\
             GGCTGAAAGAACAGCAAGTGCAGCAGAGTCCCGCATCCTTGGTTCTGCAGGAGGCGGAGAACGCAGAGCTC\
             CAGTGTAGCTTTTCCATCTTTACAAACCAGGTGCAGTGGTTTTACCAACGTCCTGGGGGAAGACTCGTCAG\
             CCTGTTGTACAATCCTTCTGGGACAAAGCAGAGTGGGAGACTGACATCCACAACAGTCATTAAAGAACGTC\
             GCAGCTCTTTGCACATTTCCTCCTCCCAGATCACAGACTCAGGCACTTATCTCTGTGCTATGGAAC",
        ));

        // Insertion of 21 bases on TRBV13-2, as indicated below.  Note that we
        // use the same name.

        added_genes_seq.push((
            "TRBV13-2",
            "ATGGGCTCCAGGCTCTTCTTCGTGCTCTCCAGTCTCCTGTGTTCAA\
             GTTTTGTCTTTCTTTTTATAG\
             AACACATGGAGGCTGCAGTCACCCAAAGCCCAAGAAACAAGGTGGCAGTAACAGGAGGAAAGGTGACATTG\
             AGCTGTAATCAGACTAATAACCACAACAACATGTACTGGTATCGGCAGGACACGGGGCATGGGCTGAGGCT\
             GATCCATTATTCATATGGTGCTGGCAGCACTGAGAAAGGAGATATCCCTGATGGATACAAGGCCTCCAGAC\
             CAAGCCAAGAGAACTTCTCCCTCATTCTGGAGTTGGCTACCCCCTCTCAGACATCAGTGTACTTCTGTGCC\
             AGCGGTGATG",
        ));

        // Fragment of constant region.  This is from GenBank V01526.1, which
        // points to "The structure of the mouse immunoglobulin in gamma 3 membrane
        // gene segment", Nucleic Acids Res. 1983 Oct 11;11(19):6775-85.  From that
        // article, it appears that the sequence is probably from an A/J mouse.
        // Since 10x only supports B6 and BALB/c mice, it's not clear why we should
        // have this sequence in the reference, however we have an enrichment primer
        // that matches this sequence and none of the other constant regions.
        //
        // This sequence is not long enough to be a full constant region sequence.
        //
        // We have another IGHG3 sequence, so this might be regarded as an alternate
        // allele, however these sequences seem to have no homology.
        //
        // Perhaps this sequence is just wrong.

        added_genes_seq.push((
            "IGHG3",
            "AGCTGGAACTGAATGGGACCTGTGCTGAGGCCCAGGATGGGGAGCTGGACGGGCTCTGGACGACCATCACC\
             ATCTTCATCAGCCTCTTCCTGCTCAGCGTGTGCTACAGCGCCTCTGTCACCCTGTTCAAGGTGAAGTGGAT\
             CTTCTCCTCAGTGGTGCAGGTGAAGCAGACGGCCATCCCTGACTACAGGAACATGATTGGACAAGGTGCC",
        ));

        // Trim TRAJ49.

        right_trims.push(("TRAJ49", 3));

        // Remove extra first base from a constant region.

        left_trims.push(("IGLC2", 1));

        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        // Begin mouse changes for cell ranger 4.1.
        // (see also human changes, above)
        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

        // 1. The gene TRAV23 is frameshifted.

        deleted_genes.push("TRAV23");

        // 2. The constant region gene IGHG2B has an extra base at its beginning.  We previously
        // added this sequence, and so we've moved that addition here, and deleted its first base.
        // It is a BALB/c constant region, from GenBank V00763.1.

        added_genes_seq.push((
            "IGHG2B",
            "CCAAAACAACACCCCCATCAGTCTATCCACTGGCCCCTGGGTGTGGAGATACAACTGGTTCCTCCGTGAC\
             CTCTGGGTGCCTGGTCAAGGGGTACTTCCCTGAGCCAGTGACTGTGACTTGGAACTCTGGATCCCTGTCCA\
             GCAGTGTGCACACCTTCCCAGCTCTCCTGCAGTCTGGACTCTACACTATGAGCAGCTCAGTGACTGTCCCC\
             TCCAGCACCTGGCCAAGTCAGACCGTCACCTGCAGCGTTGCTCACCCAGCCAGCAGCACCACGGTGGACAA\
             AAAACTTGAGCCCAGCGGGCCCATTTCAACAATCAACCCCTGTCCTCCATGCAAGGAGTGTCACAAATGCC\
             CAGCTCCTAACCTCGAGGGTGGACCATCCGTCTTCATCTTCCCTCCAAATATCAAGGATGTACTCATGATC\
             TCCCTGACACCCAAGGTCACGTGTGTGGTGGTGGATGTGAGCGAGGATGACCCAGACGTCCAGATCAGCTG\
             GTTTGTGAACAACGTGGAAGTACACACAGCTCAGACACAAACCCATAGAGAGGATTACAACAGTACTATCC\
             GGGTGGTCAGCACCCTCCCCATCCAGCACCAGGACTGGATGAGTGGCAAGGAGTTCAAATGCAAGGTGAAC\
             AACAAAGACCTCCCATCACCCATCGAGAGAACCATCTCAAAAATTAAAGGGCTAGTCAGAGCTCCACAAGT\
             ATACACTTTGCCGCCACCAGCAGAGCAGTTGTCCAGGAAAGATGTCAGTCTCACTTGCCTGGTCGTGGGCT\
             TCAACCCTGGAGACATCAGTGTGGAGTGGACCAGCAATGGGCATACAGAGGAGAACTACAAGGACACCGCA\
             CCAGTTCTTGACTCTGACGGTTCTTACTTCATATATAGCAAGCTCAATATGAAAACAAGCAAGTGGGAGAA\
             AACAGATTCCTTCTCATGCAACGTGAGACACGAGGGTCTGAAAAATTACTACCTGAAGAAGACCATCTCCC\
             GGTCTCCGGGTAAA",
        ));

        // 3. The gene IGKV12-89 shows a six base insertion in all 10x data, so we insert it here.

        deleted_genes.push("IGKV12-89");
        added_genes2.push((
            "IGKV12-89",
            "6",
            68834846,
            68835149,
            68835268,
            68835307,
            false,
        ));

        // 4. Fix a gene for which the canonical C at the end of FWR3 is seen as S.  In all our
        // data, we see C.  This is a single base change, except that we've truncated after the C.
        // The space after the gene name is to work around a crash.

        deleted_genes.push("IGHV8-9");
        added_genes_seq.push((
            "IGHV8-9 ",
            "ATGGACAGGCTTACTTCCTCATTCCTACTCCTGATTGTTCCTGTCTATGTCCTATCCCAGGTTACTCTGAAAGAGTCTGGCCCTGGGATATTGCAGCCCTCCCAGACCCTCAGTCTGACTTGTTCTTTCTCTGGGTTTTCACTGAGCACTTTTGGTATGGGTGTGAGCTGGATTCGTCAGCCTTCAGGGAATGGTCTGGAGTGGCTGGCACACATTTATTGGGATGATGACAAGCACTATAACCCATCCTTGAAGAGCCGGCTCACAATCTCCAAGGATACCTCCAACAACCAGGTATTCCTCAAGATCACGACTGTGGACACTGCAGATACTGCCACATACTACTGT",
        ));

        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        // End mouse changes for cell ranger 4.1.
        // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
    }
    if species == "balbc" {}

    // Normalize exceptions.

    deleted_genes.sort();
    allowed_pseudogenes.sort();
    left_trims.sort();
    right_trims.sort();

    // Define a function that returns the ensembl path for a particular dataset.
    // Note that these are for the ungzipped versions.

    fn ensembl_path(
        species: &str, // human or mouse or balbc
        ftype: &str,   // gff3 or gtf or fasta
        release: i32,  // release number
    ) -> String {
        let species_name = match species {
            "human" => "homo_sapiens",
            "mouse" => "mus_musculus",
            "balbc" => "mus_musculus_balbcj",
            _ => "",
        };
        let releasep = format!("release-{}", release);
        let csn = cap1(species_name);
        match (species, ftype) {
            ("mouse", "gff3") => format!(
                "{}/{}/{}/{}.GRCm38.{}.gff3",
                releasep, ftype, species_name, csn, release
            ),
            ("mouse", "gtf") => format!(
                "{}/{}/{}/{}.GRCm38.{}.gtf",
                releasep, ftype, species_name, csn, release
            ),
            ("mouse", "fasta") => format!(
                "{}/{}/{}/dna/{}.GRCm38.dna.toplevel.fa",
                releasep, ftype, species_name, csn
            ),

            ("balbc", "gff3") => format!(
                "{}/{}/{}/{}.BALB_cJ_v1.{}.gff3",
                releasep, ftype, species_name, csn, release
            ),
            ("balbc", "gtf") => format!(
                "{}/{}/{}/{}.BALB_cJ_v1.{}.gtf",
                releasep, ftype, species_name, csn, release
            ),
            ("balbc", "fasta") => format!(
                "{}/{}/{}/dna/{}.BALB_cJ_v1.dna.toplevel.fa",
                releasep, ftype, species_name, csn
            ),

            ("human", "gff3") => format!(
                "{}/{}/{}/{}.GRCh38.{}.chr_patch_hapl_scaff.gff3",
                releasep, ftype, species_name, csn, release
            ),
            ("human", "gtf") => format!(
                "{}/{}/{}/{}.GRCh38.{}.chr_patch_hapl_scaff.gtf",
                releasep, ftype, species_name, csn, release
            ),
            ("human", "fasta") => format!(
                "{}/{}/{}/dna/{}.GRCh38.dna.toplevel.fa",
                releasep, ftype, species_name, csn
            ),
            _ => format!(""),
        }
    }

    // Download files from ensembl site if requested.  Not fully tested, and it
    // would appear that a git command can fail without causing this code to panic.
    // This can't be fully tested since we don't want to run the git commands as
    // an experiment.
    // Note that the human fasta file would be 54 GB if uncompressed (owing to
    // gigantic runs of ends), so we don't uncompress fasta files.

    if download {
        fn fetch(species: &str, ftype: &str, release: i32) {
            println!("fetching {}.{}", species, ftype);
            let path = ensembl_path(&species, &ftype, release);
            let external = "ftp://ftp.ensembl.org/pub";
            let internal = "/mnt/opt/meowmix_git/ensembl";
            let dir = format!("{}/{}", internal, path.rev_before("/"));
            fs::create_dir_all(&dir).unwrap();
            let full_path = format!("{}/{}", internal, path);
            Command::new("wget")
                .current_dir(&dir)
                .arg(format!("{}/{}.gz", external, path))
                .status()
                .expect("wget failed");
            if ftype != "fasta" {
                Command::new("gunzip")
                    .arg(&full_path)
                    .status()
                    .expect("gunzip failed");
            }
            Command::new("git")
                .current_dir(&internal)
                .arg("add")
                .arg(&path)
                .status()
                .expect("git add failed");
            Command::new("git")
                .current_dir(&internal)
                .arg("commit")
                .arg(path)
                .status()
                .expect("git commit failed");
        }
        // ◼ Add balbc if we're going ot use it.
        for species in ["human", "mouse"].iter() {
            for ftype in ["gff3", "gtf", "fasta"].iter() {
                fetch(species, ftype, release);
            }
        }
        std::process::exit(0);
    }

    // Define input filenames.

    let gtf = format!("{}/{}", internal, ensembl_path(&species, "gtf", release));
    let fasta = format!(
        "{}/{}.gz",
        internal,
        ensembl_path(&species, "fasta", release)
    );

    // Generate reference.json.  Note version number.

    let mut json = open_for_write_new![&format!("{}/{}/reference.json", root, species)];
    fwriteln!(json, "{{");
    let mut sha256 = Sha256::new();
    copy(&mut File::open(&fasta).unwrap(), &mut sha256).unwrap();
    let hash = sha256.finalize();
    fwriteln!(json, r###"    "fasta_hash": "{:x}","###, hash);
    fwriteln!(json, r###"    "genomes": "{}","###, source2);
    let mut sha256 = Sha256::new();
    copy(&mut File::open(&gtf).unwrap(), &mut sha256).unwrap();
    let hash = sha256.finalize();
    fwriteln!(json, r###"    "gtf_hash": "{:x}","###, hash);
    fwriteln!(
        json,
        r###"    "input_fasta_files": "{}","###,
        ensembl_path(&species, "fasta", release)
    );
    fwriteln!(
        json,
        r###"    "input_gtf_files": "{}","###,
        ensembl_path(&species, "gtf", release)
    );
    fwriteln!(json, r###"    "mkref_version": "","###);
    fwriteln!(json, r###"    "type": "V(D)J Reference","###);
    fwriteln!(json, r###"    "version": "{}""###, version);
    fwrite!(json, "}}");

    // Load the gff3 file and use it to do two things:
    //
    // 1. Remove genes classed as non-functional.
    //    ◼ Except that we don't.  To consider later.
    //
    // 2. Convert gene names into the standard format.  This would not be needed,
    //    except that in the gtf file, for genes present only on alternate loci,
    //    only an accession identifier is given (in some and perhaps all cases).

    let gff3 = format!("{}/{}", internal, ensembl_path(&species, "gff3", release));
    let mut demangle = HashMap::<String, String>::new();
    let f = open_for_read![&gff3];
    for line in f.lines() {
        let s = line.unwrap();
        let fields: Vec<&str> = s.split_terminator('\t').collect();
        if fields.len() < 9 {
            continue;
        }
        if fields[2] != "gene" && fields[2] != "pseudogene" {
            continue;
        }
        let fields8: Vec<&str> = fields[8].split_terminator(';').collect();
        let (mut gene, mut gene2) = (String::new(), String::new());
        let mut biotype = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with("Name=") {
                gene = fields8[i].after("Name=").to_string();
            }
            if fields8[i].starts_with("description=") {
                gene2 = fields8[i].after("description=").to_string();
            }
            if fields8[i].starts_with("biotype=") {
                biotype = fields8[i].after("biotype=").to_string();
            }
        }

        // Test for appropriate gene type.
        // Note that we allow V and J pseudogenes, but only by explicit inclusion.

        if biotype != "TR_V_gene"
            && biotype != "TR_D_gene"
            && biotype != "TR_J_gene"
            && biotype != "TR_C_gene"
            && biotype != "TR_V_pseudogene"
            && biotype != "TR_J_pseudogene"
            && biotype != "IG_V_gene"
            && biotype != "IG_D_gene"
            && biotype != "IG_J_gene"
            && biotype != "IG_C_gene"
            && biotype != "IG_V_pseudogene"
            && biotype != "IG_J_pseudogene"
        {
            continue;
        }

        // Sanity check.

        if !gene2.starts_with("T cell receptor ")
            && !gene2.starts_with("T-cell receptor ")
            && !gene2.starts_with("immunoglobulin ")
            && !gene2.starts_with("Immunoglobulin ")
        {
            continue;
            // println!( "problem with gene = '{}', gene2 = '{}'", gene, gene2 );
        }

        // Maybe exclude nonfunctional.

        let exclude_non_functional = false;
        if exclude_non_functional && gene2.contains("(non-functional)") {
            continue;
        }

        // Fix gene.

        gene = gene.to_uppercase();
        gene = gene.replace("TCR", "TR");
        gene = gene.replace("BCR", "BR");
        gene = gene.replace("G-", "G");

        // Fix gene2.

        gene2 = gene2.replace("  ", " ");
        gene2 = gene2.replace("%2C", "");
        gene2 = gene2.replace("T cell receptor ", "TR");
        gene2 = gene2.replace("T-cell receptor ", "TR");
        gene2 = gene2.replace("immunoglobulin ", "IG");
        gene2 = gene2.replace("Immunoglobulin ", "IG");
        gene2 = gene2.replace("variable V", "V");

        // More fixing.  Replace e.g. "alpha " by A.

        for x in [
            "alpha",
            "beta",
            "gamma",
            "delta",
            "epsilon",
            "kappa",
            "lambda",
            "mu",
            "variable",
            "diversity",
            "joining",
            "constant",
            "heavy",
        ]
        .iter()
        {
            gene2 = gene2.replace(&format!("{} ", x), &x[0..1].to_uppercase());
        }

        // More fixing.

        gene2 = gene2.replace("region ", "");
        gene2 = gene2.replace("novel ", "");
        gene2 = gene2.replace("chain ", "");
        if gene2.contains("[") {
            gene2 = gene2.before("[").to_string();
        }
        if gene2.contains("(") {
            gene2 = gene2.before("(").to_string();
        }
        gene2 = gene2.replace(" ", "");
        if gene2.contains("identical") || gene2.contains("identicle") {
            continue;
        }
        gene2 = gene2.to_uppercase();

        // Ignore certain genes.

        if (biotype == "TR_V_pseudogene"
            || biotype == "TR_J_pseudogene"
            || biotype == "IG_V_pseudogene"
            || biotype == "IG_J_pseudogene")
            && !bin_member(&allowed_pseudogenes, &gene2.as_str())
        {
            continue;
        }
        if bin_member(&deleted_genes, &gene2.as_str()) {
            continue;
        }

        // Save result.

        demangle.insert(gene.clone(), gene2.clone());
    }

    // Parse the gtf file.

    let mut exons = Vec::<(String, String, String, i32, i32, String, bool, String)>::new();
    parse_gtf_file(&gtf, &demangle, &mut exons);

    // Find the chromosomes that we're using.

    let mut all_chrs = Vec::<String>::new();
    for k in 0..exons.len() {
        all_chrs.push(exons[k].2.clone());
    }
    unique_sort(&mut all_chrs);

    // Load fasta.  We only load the records that we need.  This is still slow
    // and it might be possible to speed it up.
    // ◼ Put this 'selective fasta loading' into its own function.

    let mut refs = Vec::<DnaString>::new();
    let mut rheaders = Vec::<String>::new();
    let gz = MultiGzDecoder::new(std::fs::File::open(&fasta).unwrap());
    let f = BufReader::new(gz);
    let mut last: String = String::new();
    let mut using = false;
    for line in f.lines() {
        let s = line.unwrap();
        if s.starts_with(">") {
            if using {
                refs.push(DnaString::from_dna_string(&last));
                last.clear();
            }
            if rheaders.len() == all_chrs.len() {
                break;
            }
            let mut h = s.get(1..).unwrap().to_string();
            if h.contains(" ") {
                h = h.before(" ").to_string();
            }
            if bin_member(&all_chrs, &h) {
                rheaders.push(h.clone());
                using = true;
            } else {
                using = false;
            }
        } else if using {
            last += &s
        }
    }
    if using {
        refs.push(DnaString::from_dna_string(&last));
    }
    let mut to_chr = HashMap::new();
    for i in 0..rheaders.len() {
        to_chr.insert(rheaders[i].clone(), i);
    }

    // Get the DNA sequences for the exons.

    let mut dna = Vec::<DnaString>::new();
    for i in 0..exons.len() {
        let chr = &exons[i].2;
        let chrid = to_chr[&chr.to_string()];
        let (start, stop) = (exons[i].3, exons[i].4);
        let seq = refs[chrid].slice(start as usize, stop as usize).to_owned();
        dna.push(seq);
    }

    // Remove transcripts having identical sequences.

    let mut to_delete = vec![false; exons.len()];
    let mut i = 0;
    let mut dnas = Vec::<(Vec<DnaString>, usize, usize)>::new();
    while i < exons.len() {
        let j = next_diff12_8(&exons, i as i32) as usize;
        let mut x = Vec::<DnaString>::new();
        for k in i..j {
            x.push(dna[k].clone());
        }
        dnas.push((x, i, j));
        i = j;
    }
    dnas.sort();
    for i in 1..dnas.len() {
        if dnas[i].0 == dnas[i - 1].0 {
            let (i, j) = (dnas[i].1, dnas[i].2);
            for k in i..j {
                to_delete[k] = true;
            }
        }
    }
    erase_if(&mut exons, &to_delete);

    // Build fasta.

    let mut i = 0;
    let mut record = 0;
    while i < exons.len() {
        let j = next_diff12_8(&exons, i as i32) as usize;
        let mut fws = Vec::<bool>::new();
        for k in i..j {
            fws.push(exons[k].6);
        }
        unique_sort(&mut fws);
        assert!(fws.len() == 1);
        let fw = fws[0];
        let gene = &exons[i].0;
        if bin_member(&excluded_genes, &gene.as_str()) {
            i = j;
            continue;
        }

        // The gene may appear on more than one record.  We pick the one that
        // is lexicographically minimal.  This should favor numbered chromosomes
        // over alt loci.
        // ◼ NOT SURE WHAT THIS IS DOING NOW.

        let mut chrs = Vec::<String>::new();
        for k in i..j {
            chrs.push(exons[k].2.clone());
        }
        unique_sort(&mut chrs);
        let chr = chrs[0].clone();
        let chrid = to_chr[&chr.to_string()];

        // Build the 5' UTR for V, if there is one.  We allow for the possibility
        // that there is an intron in the UTR, although this is very rare (once in
        // human TCR).

        let mut seq = DnaString::new();
        let trid = &exons[i].7;
        for k in i..j {
            if exons[k].2 != chr {
                continue;
            }
            let (start, stop) = (exons[k].3, exons[k].4);
            let cat = &exons[k].5;
            if cat == "five_prime_utr" {
                let seqx = refs[chrid].slice(start as usize, stop as usize);
                for i in 0..seqx.len() {
                    seq.push(seqx.get(i));
                }
            }
        }
        if seq.len() > 0 {
            let header = header_from_gene(&gene, true, &mut record, trid);
            print_oriented_fasta(&mut out, &header, &seq.slice(0, seq.len()), fw, none);
        }

        // Build L+V segment.
        // ◼ To do: separately track L.  Do not require that transcripts include L.

        if gene.starts_with("TRAV")
            || gene.starts_with("TRBV")
            || gene.starts_with("TRDV")
            || gene.starts_with("TRGV")
            || gene.starts_with("IGHV")
            || gene.starts_with("IGKV")
            || gene.starts_with("IGLV")
        {
            let mut seq = DnaString::new();
            let mut ncodons = 0;
            for k in i..j {
                if exons[k].2 != chr {
                    continue;
                }
                let (start, stop) = (exons[k].3, exons[k].4);
                let cat = &exons[k].5;
                if cat == "CDS" {
                    ncodons += 1;
                    let seqx = refs[chrid].slice(start as usize, stop as usize);
                    for i in 0..seqx.len() {
                        seq.push(seqx.get(i));
                    }
                }
            }
            if seq.len() > 0 {
                let header = header_from_gene(&gene, false, &mut record, trid);
                let mut seqx = seq.clone();
                if !fw {
                    seqx = seqx.rc();
                }
                let p = bin_position1_2(&right_trims, &gene.as_str());
                // negative right_trims incorrectly handled, to fix make code
                // same as for J
                let mut n = seq.len() as i32;
                if p >= 0 {
                    n -= right_trims[p as usize].1;
                }
                let mut m = 0;
                let p = bin_position1_2(&left_trims, &gene.as_str());
                if p >= 0 {
                    m = left_trims[p as usize].1;
                }

                // Save.  Mostly we require two exons.

                let standard = gene.starts_with("TRAV")
                    || gene.starts_with("TRBV")
                    || gene.starts_with("IGHV")
                    || gene.starts_with("IGKV")
                    || gene.starts_with("IGLV");
                if ncodons == 2 || !standard {
                    print_fasta(&mut out, &header, &seqx.slice(m, n as usize), none);
                } else {
                    record -= 1;
                }
            }
        }

        // Build J and D segments.

        if (gene.starts_with("TRAJ")
            || gene.starts_with("TRBJ")
            || gene.starts_with("TRDJ")
            || gene.starts_with("TRGJ")
            || gene.starts_with("IGHJ")
            || gene.starts_with("IGKJ")
            || gene.starts_with("IGLJ")
            || gene.starts_with("TRBD")
            || gene.starts_with("TRDD")
            || gene.starts_with("IGHD"))
            && gene != "IGHD"
        {
            let mut using = Vec::<usize>::new();
            for k in i..j {
                if exons[k].2 == chr && exons[k].5 != "five_prime_utr" {
                    using.push(k);
                }
            }
            if using.len() != 1 {
                eprintln!("problem with {}, have {} exons", gene, using.len());
            }
            // assert_eq!( using.len(), 1 );
            let k = using[0];
            let start = exons[k].3;
            let mut stop = exons[k].4;
            let p = bin_position1_2(&right_trims, &gene.as_str());
            if p >= 0 && right_trims[p as usize].1 < 0 {
                stop -= right_trims[p as usize].1;
            }
            let seq = refs[chrid].slice(start as usize, stop as usize);
            let mut n = seq.len() as i32;
            if p >= 0 && right_trims[p as usize].1 > 0 {
                n -= right_trims[p as usize].1;
            }
            let mut m = 0;
            let p = bin_position1_2(&left_trims, &gene.as_str());
            if p >= 0 {
                m = left_trims[p as usize].1;
            }
            let header = header_from_gene(&gene, false, &mut record, trid);
            let seqx = seq.clone();
            print_oriented_fasta(&mut out, &header, &seqx.slice(m, n as usize), fw, none);
        }

        // Build C segments.

        if gene.starts_with("TRAC")
            || gene.starts_with("TRBC")
            || gene.starts_with("TRDC")
            || gene.starts_with("TRGC")
            || gene.starts_with("IGKC")
            || gene.starts_with("IGLC")
            || gene.starts_with("IGHG")
            || gene == "IGHD"
            || gene == "IGHE"
            || gene == "IGHM"
            || gene.starts_with("IGHA")
        {
            let mut seq = DnaString::new();
            for k in i..j {
                if exons[k].2 != chr {
                    continue;
                }
                let (start, stop) = (exons[k].3, exons[k].4);
                let seqx = refs[chrid].slice(start as usize, stop as usize);
                for i in 0..seqx.len() {
                    seq.push(seqx.get(i));
                }
            }
            let mut m = 0;
            let p = bin_position1_2(&left_trims, &gene.as_str());
            if p >= 0 {
                m = left_trims[p as usize].1;
            }
            let header = header_from_gene(&gene, false, &mut record, trid);
            if fw {
                print_oriented_fasta(&mut out, &header, &seq.slice(m, seq.len()), fw, none);
            } else {
                print_oriented_fasta(&mut out, &header, &seq.slice(0, seq.len() - m), fw, none);
            }
        }

        // Advance.

        i = j;
    }

    // Add genes.

    for i in 0..added_genes.len() {
        add_gene(
            &mut out,
            &added_genes[i].0,
            &mut record,
            &added_genes[i].1,
            added_genes[i].2,
            added_genes[i].3,
            &to_chr,
            &refs,
            none,
            added_genes[i].4,
            &source,
        );
    }
    for i in 0..added_genes2.len() {
        add_gene2(
            &mut out,
            &added_genes2[i].0,
            &mut record,
            &added_genes2[i].1,
            added_genes2[i].2,
            added_genes2[i].3,
            added_genes2[i].4,
            added_genes2[i].5,
            &to_chr,
            &refs,
            none,
            added_genes2[i].6,
            &source,
        );
    }
    for i in 0..added_genes2_source.len() {
        let gene = &added_genes2_source[i].0;
        let start1 = added_genes2_source[i].1;
        let stop1 = added_genes2_source[i].2;
        let start2 = added_genes2_source[i].3;
        let stop2 = added_genes2_source[i].4;
        let fw = added_genes2_source[i].5;
        let source = &added_genes2_source[i].6;
        let mut seq = DnaString::new();
        load_genbank_accession(source, &mut seq);
        let seq1 = seq.slice(start1 - 1, stop1);
        let seq2 = seq.slice(start2 - 1, stop2);
        let mut seq = seq1.to_owned();
        for i in 0..seq2.len() {
            seq.push(seq2.get(i));
        }
        if !fw {
            seq = seq.rc();
        }
        let header = header_from_gene(&gene, false, &mut record, source);
        print_fasta(&mut out, &header, &seq.slice(0, seq.len()), none);
    }
    for i in 0..added_genes_seq.len() {
        let gene = &added_genes_seq[i].0;
        let seq = DnaString::from_dna_string(&added_genes_seq[i].1);
        let header = header_from_gene(&gene, false, &mut record, &source);
        print_fasta(&mut out, &header, &seq.slice(0, seq.len()), none);
    }
}
