// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Extract zero-based human or mouse exon positions from Ensembl gtf file:
// { { chr-name, start, stop, fw?, gene-name, exon ) }.

use std::{
    fs::File,
    io::{BufRead, BufReader},
    *,
};
use string_utils::*;
use vector_utils::*;

pub fn fetch_exons(species: &String, exons: &mut Vec<(String, i32, i32, bool, String, i32)>) {
    assert!(species == "human" || species == "mouse");

    // Define gtf file location.  See notes in bin/build_vdj_ref.fs.

    let root = "/mnt/opt/meowmix_git/ensembl/release-94/gtf";
    let gtf: String;
    if species == "human" {
        gtf = format!(
            "{}/homo_sapiens/Homo_sapiens.GRCh38.94.chr_patch_hapl_scaff.gtf",
            root
        );
    } else {
        gtf = format!("{}/mus_musculus/Mus_musculus.GRCm38.94.gtf", root);
    }

    // Parse the gtf file.

    exons.clear();

    let f = open_for_read![&gtf];
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
        if !fields8[4].contains("exon_number") {
            continue;
        }
        if !fields8[5].contains("gene_name") {
            continue;
        }
        let exon = fields8[4].between("\"", "\"").force_i32();
        let gene = fields8[5].between("\"", "\"");
        // println!( "" );
        // for j in 0..fields.len() { println!( "{}: {}", j, fields[j] ); }
        // for j in 0..fields8.len() { println!( "8.{}: {}", j, fields8[j] ); }
        let chr = fields[0];
        let (start, stop) = (fields[3].force_i32(), fields[4].force_i32());
        let mut fw = false;
        if fields[6] == "+" {
            fw = true;
        }
        exons.push((chr.to_string(), start - 1, stop, fw, gene.to_string(), exon));
    }
    unique_sort(exons);
}
