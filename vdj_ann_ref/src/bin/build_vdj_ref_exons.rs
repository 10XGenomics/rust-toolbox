// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Modified version of build_vdj_ref.rs.
//
// Purpose: print just the IG CDS exons, but also print flanking sequences.
//
// Motivation: to understand splice sites.
//
// Usage:
//
// build_vdj_ref_exons HUMAN > filename
// build_vdj_ref_exons MOUSE > filename

use debruijn::dna_string::DnaString;
use flate2::read::MultiGzDecoder;
use pretty_trace::PrettyTrace;
use process::Command;
use sha2::{Digest, Sha256};
use std::io::copy;
use std::io::Write;
use std::{
    collections::HashMap,
    env, eprintln, format, fs,
    fs::File,
    i32,
    io::{BufRead, BufReader},
    print, println, process, str, u8, usize, vec, write, writeln,
};
use string_utils::{cap1, strme, TextUtils};
use vector_utils::{bin_member, erase_if, unique_sort};

use io_utils::{fwrite, fwriteln, open_for_read, open_for_write_new};

// copied from tenkit2/pack_dna.rs:

pub fn reverse_complement(x: &mut [u8]) {
    x.reverse();
    for v in x {
        *v = match *v {
            b'A' => b'T',
            b'C' => b'G',
            b'G' => b'C',
            b'T' => b'A',
            _ => *v,
        }
    }
}

type ExonInfo = (String, String, String, i32, i32, String, bool, String);

fn parse_gtf_file(gtf: &str, demangle: &HashMap<String, String>, exons: &mut Vec<ExonInfo>) {
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
        // change it to CDS. [NOT]

        let mut biotype = String::new();
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" gene_biotype") {
                biotype = fields8[i].between("\"", "\"").to_string();
            }
        }
        let cat = fields[2];
        if !biotype.starts_with("IG_") {
            continue;
        }
        // if biotype.starts_with("IG_C") {
        //     continue;
        // }

        // Exclude certain types.

        if cat != "CDS" && cat != "five_prime_utr" {
            continue;
        }

        // Get gene name and demangle.

        let mut gene = "";
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" gene_name") {
                gene = fields8[i].between("\"", "\"");
            }
        }
        let gene = gene.to_uppercase();
        let gene2 = demangle.get(&gene);
        if gene2.is_none() {
            continue;
        }
        let mut gene2 = gene2.unwrap().clone();

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

        /*
        if !gene2.starts_with("IGHV") && !gene2.starts_with("IGKV") && !gene2.starts_with("IGLV")
            && !gene2.starts_with("IGHJ") && !gene2.starts_with("IGKJ")
            && !gene2.starts_with("IGLJ") && !gene2.starts_with("IGHD") {
            continue;
        }
        */

        // For now, require havana (except for mouse strains).  Could try turning
        // this off, but there may be some issues.

        if !fields[1].contains("havana") && fields[1] != "mouse_genomes_project" {
            continue;
        }

        // Get transcript name.

        let mut tr = "";
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" transcript_name") {
                tr = fields8[i].between("\"", "\"");
            }
        }

        // Get transcript id.

        let mut trid = "";
        for i in 0..fields8.len() {
            if fields8[i].starts_with(" transcript_id") {
                trid = fields8[i].between("\"", "\"");
            }
        }

        // Save in exons.

        let chr = fields[0];
        let start = fields[3].force_i32() - 1;
        let stop = fields[4].force_i32();
        let fw = fields[6] == "+";
        exons.push((
            gene2,
            tr.to_string(),
            chr.to_string(),
            start,
            stop,
            cat.to_string(),
            fw,
            trid.to_string(),
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
    let species = match args[1].as_str() {
        "DOWNLOAD" => "",
        "HUMAN" => "human",
        "MOUSE" => "mouse",
        "BALBC" => "balbc",
        "NONE" => "human",
        _ => {
            eprintln!("Call with DOWNLOAD or HUMAN or MOUSE or NONE.");
            std::process::exit(1);
        }
    };
    let download = species.is_empty();

    // Define root output directory.

    let root = "vdj_ann/vdj_refs".to_string();

    // Define release.  If this is ever changed, the effect on the fasta output
    // files should be very carefully examined.  Specify sequence source.
    // Note that source2 depends on the cellranger version!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

    let release = 94;
    let version = "4.0.0";
    let source2: String;
    if species == "human" {
        source2 = format!("vdj_GRCh38_alts_ensembl-{}", version);
    } else if species == "mouse" {
        source2 = format!("vdj_GRCm38_alts_ensembl-{}", version);
    } else {
        source2 = String::new();
    }

    // Define local directory.

    let internal = "/mnt/opt/meowmix_git/ensembl";

    // Set up for exceptions.  Coordinates are the usual 1-based coordinates used in
    // genomics.  If the bool field ("fw") is false, the given coordinates are used
    // to extract a sequence, and then it is reversed.

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
        allowed_pseudogenes.push("TRBV21-1");
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
        added_genes2.push((
            "IGKV2-18", "2", 89128701, 89129017, 89129435, 89129449, false,
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

        // A BALB/c constant region, from GenBank V00763.1.

        added_genes_seq.push((
            "IGHG2B",
            "GCCAAAACAACACCCCCATCAGTCTATCCACTGGCCCCTGGGTGTGGAGATACAACTGGTTCCTCCGTGAC\
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

        // Trim TRAJ49.

        right_trims.push(("TRAJ49", 3));

        // Remove extra first base from a constant region.

        left_trims.push(("IGLC2", 1));
    }
    if species == "balbc" {}

    // Normalize exceptions.

    deleted_genes.sort_unstable();
    allowed_pseudogenes.sort_unstable();
    left_trims.sort_unstable();
    right_trims.sort_unstable();

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
            _ => String::new(),
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
            let path = ensembl_path(species, ftype, release);
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

    let gtf = format!("{}/{}", internal, ensembl_path(species, "gtf", release));
    let fasta = format!(
        "{}/{}.gz",
        internal,
        ensembl_path(species, "fasta", release)
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
        ensembl_path(species, "fasta", release)
    );
    fwriteln!(
        json,
        r###"    "input_gtf_files": "{}","###,
        ensembl_path(species, "gtf", release)
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

    let gff3 = format!("{}/{}", internal, ensembl_path(species, "gff3", release));
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
        if gene2.contains('[') {
            gene2 = gene2.before("[").to_string();
        }
        if gene2.contains('(') {
            gene2 = gene2.before("(").to_string();
        }
        gene2 = gene2.replace(' ', "");
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

    let mut exons = Vec::<ExonInfo>::new();
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
        if s.starts_with('>') {
            if using {
                refs.push(DnaString::from_dna_string(&last));
                last.clear();
            }
            if rheaders.len() == all_chrs.len() {
                break;
            }
            let mut h = s.get(1..).unwrap().to_string();
            if h.contains(' ') {
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

    // Get the DNA sequences for the exons.  Extended by ten in both directions.

    const EXT: usize = 60;
    let mut dna = Vec::<DnaString>::new();
    let mut starts = Vec::<usize>::new();
    let mut stops = Vec::<usize>::new();
    for i in 0..exons.len() {
        let chr = &exons[i].2;
        let chrid = to_chr[chr];
        starts.push(exons[i].3 as usize);
        stops.push(exons[i].4 as usize);
        let (start, stop) = (exons[i].3 - EXT as i32, exons[i].4 + EXT as i32);
        let seq = refs[chrid].slice(start as usize, stop as usize).to_owned();
        dna.push(seq);
    }

    // Put exons in order.

    for _pass in 1..=2 {
        for i in 1..exons.len() {
            if exons[i].1 == exons[i - 1].1 && exons[i].5 != "CDS" && exons[i - 1].5 == "CDS" {
                dna.swap(i, i - 1);
                exons.swap(i, i - 1);
                starts.swap(i, i - 1);
                stops.swap(i, i - 1);
            }
        }
    }
    for i in 1..exons.len() {
        if exons[i].1 == exons[i - 1].1
            && dna[i].len() < dna[i - 1].len()
            && exons[i].5 == "CDS"
            && exons[i - 1].5 == "CDS"
        {
            dna.swap(i, i - 1);
            exons.swap(i, i - 1);
            starts.swap(i, i - 1);
            stops.swap(i, i - 1);
        }
    }

    // Exclude solo exons.

    let mut to_delete = vec![false; exons.len()];
    for i in 1..exons.len() - 1 {
        if exons[i].1 != exons[i - 1].1
            && exons[i].1 != exons[i + 1].1
            && exons[i].0.as_bytes()[3] == b'V'
        {
            to_delete[i] = true;
        }
    }
    erase_if(&mut exons, &to_delete);
    erase_if(&mut dna, &to_delete);
    erase_if(&mut starts, &to_delete);
    erase_if(&mut stops, &to_delete);

    // Build modified fasta.

    println!();
    for i in 0..exons.len() {
        let mut x = dna[i].to_ascii_vec();
        if !exons[i].6 {
            reverse_complement(&mut x);
        }
        let n = x.len();
        if i > 0 && exons[i].1 != exons[i - 1].1 {
            println!();
        }
        print!(
            ">{}, transcript = {}, len = {} = {} % 3",
            exons[i].0,
            exons[i].1,
            n - EXT - EXT,
            (n - EXT - EXT) % 3
        );
        if exons[i].5 == "CDS" && exons[i].0.as_bytes()[3] == b'V' {
            if i < exons.len() - 1 && exons[i].1 == exons[i + 1].1 {
                let intron = if !exons[i].6 {
                    starts[i] - stops[i + 1]
                } else {
                    starts[i + 1] - stops[i]
                };
                print!(", intron = {}", intron);
            }
        } else if exons[i].0.as_bytes()[3] == b'V' {
            print!(", 5'-UTR");
        }
        println!(
            "\n{}|{}|{}",
            strme(&x[0..EXT]),
            strme(&x[EXT..n - EXT]),
            strme(&x[n - EXT..n])
        );
    }
}
