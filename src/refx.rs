// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains code to make reference data.
//
// ◼ Experiment with building a reference from scratch.  This would be a better
// ◼ solution than ad hoc editing of a flawed reference.
//
// ◼ Document reference sequence requirements so that a customer who wishes to
// ◼ create a reference for a new species will know the conventions used by the
// ◼ code.

use debruijn::{dna_string::*, kmer::*};
use io_utils::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use string_utils::*;
use tenkit2::io::*;
use tenkit2::kmer_lookup::*;
use vec_utils::*;

// RefData: this is a packaging of reference data appropriate for VDJ analysis.

pub struct RefData {
    pub refs: Vec<DnaString>,
    pub rheaders: Vec<String>,
    pub rkmers_plus: Vec<(Kmer12, i32, i32)>,
    // which V segments have matched UTRs in the reference:
    pub has_utr: HashMap<String, bool>,
    pub name: Vec<String>,
    pub segtype: Vec<String>,    // U, V, D, J or C
    pub rtype: Vec<i32>,         // index in "IGH","IGK","IGL","TRA","TRB","TRD","TRG" or -1
    pub igjs: Vec<usize>,        // index of all IGJ segments
    pub cs: Vec<usize>,          // index of all C segments
    pub id: Vec<i32>,            // the number after ">" on the header line
    pub transcript: Vec<String>, // transcript name from header line
}

impl<'a> RefData {
    pub fn new() -> RefData {
        RefData {
            refs: Vec::<DnaString>::new(),
            rheaders: Vec::<String>::new(),
            rkmers_plus: Vec::<(Kmer12, i32, i32)>::new(),
            has_utr: HashMap::<String, bool>::new(),
            name: Vec::<String>::new(),
            segtype: Vec::<String>::new(),
            rtype: Vec::<i32>::new(),
            igjs: Vec::<usize>::new(),
            cs: Vec::<usize>::new(),
            id: Vec::<i32>::new(),
            transcript: Vec::<String>::new(),
        }
    }
    pub fn is_u(&self, i: usize) -> bool {
        self.segtype[i] == "U".to_string()
    }
    pub fn is_v(&self, i: usize) -> bool {
        self.segtype[i] == "V".to_string()
    }
    pub fn is_d(&self, i: usize) -> bool {
        self.segtype[i] == "D".to_string()
    }
    pub fn is_j(&self, i: usize) -> bool {
        self.segtype[i] == "J".to_string()
    }
    pub fn is_c(&self, i: usize) -> bool {
        self.segtype[i] == "C".to_string()
    }

    pub fn from_fasta(path: &String) -> Self {
        // TODO: Use impl AsRef<Path> instead of &String throughout
        let mut refdata = RefData::new();
        make_vdj_ref_data_core(
            &mut refdata,
            path,
            false, // extended
            true,  // is_tcr
            true,  // is_bcr
        );
        refdata
    }

    pub fn from_fasta_with_filter(path: &String, ids_to_use: &HashSet<i32>) -> Self {
        let mut refdata = RefData::new();
        make_vdj_ref_data_core_core(
            &mut refdata,
            path,
            false, // extended
            true,  // is_tcr
            true,  // is_bcr
            Some(ids_to_use),
        );
        refdata
    }
}

pub fn vdj_ref_path(species: &String, imgt: bool) -> String {
    if imgt && species == "human" {
        return String::from(
            "/mnt/opt/refdata_cellranger/vdj/\
             vdj_IMGT_20170916-2.1.0/fasta/regions.fa",
        );
    }
    if !imgt && species == "human" {
        // ◼ temporary location
        return String::from(
            "/mnt/park/compbio/jaffe/vdj_refs/human/\
             fasta/regions.fa",
        );
    }
    if imgt && species == "mouse" {
        return String::from(
            "/mnt/opt/refdata_cellranger/vdj/\
             vdj_IMGT_mouse_20180723-2.2.0/fasta/regions.fa",
        );
    }
    if !imgt && species == "mouse" {
        // ◼ temporary location
        return String::from(
            "/mnt/park/compbio/jaffe/vdj_refs/mouse/\
             fasta/regions.fa",
        );
    }
    String::from("")
}

pub fn make_vdj_ref_data_core(
    refdata: &mut RefData,
    ref_fasta_path: &String,
    extended: bool,
    is_tcr: bool,
    is_bcr: bool,
) {
    make_vdj_ref_data_core_core(refdata, ref_fasta_path, extended, is_tcr, is_bcr, None);
}

// ids_to_use_opt: Optional hashSet of ids. If specified only reference
// entries with id in the HashSet is used to construct RefData

pub fn make_vdj_ref_data_core_core(
    refdata: &mut RefData,
    ref_fasta_path: &String,
    extended: bool,
    is_tcr: bool,
    is_bcr: bool,
    ids_to_use_opt: Option<&HashSet<i32>>,
) {
    // Define convenient abbreviations.

    let mut refs = &mut refdata.refs;
    let mut rheaders = &mut refdata.rheaders;
    let mut rkmers_plus = &mut refdata.rkmers_plus;

    // Determine species
    // ◼ Very bad.

    let mut species: String = "unknown".to_string();
    if ref_fasta_path.contains("GRCm") || ref_fasta_path.contains("mouse") {
        species = "mouse".to_string();
    }
    if ref_fasta_path.contains("GRCh")
        || ref_fasta_path.contains("IMGT_2")
        || ref_fasta_path.contains("human")
    {
        species = "human".to_string();
    }

    // Parse the fasta file.

    refs.clear();
    rheaders.clear();
    read_fasta_into_vec_dna_string_plus_headers(&ref_fasta_path, &mut refs, &mut rheaders);
    if fs::metadata(&ref_fasta_path).unwrap().len() == 0 {
        panic!("Reference file at {} has zero length.", ref_fasta_path);
    }

    // Filter by ids if requested.

    if let Some(ids_to_use) = ids_to_use_opt {
        let mut to_delete = vec![false; refs.len()];
        for i in 0..refs.len() {
            let id = rheaders[i].before("|").force_i32();
            to_delete[i] = !ids_to_use.contains(&id);
        }
        erase_if(&mut refs, &to_delete);
        erase_if(&mut rheaders, &to_delete);
    }

    // Now build stuff.

    let mut rheaders2 = Vec::<String>::new();
    let types = vec!["IGH", "IGK", "IGL", "TRA", "TRB", "TRD", "TRG"];
    refdata.rtype = vec![-1 as i32; refs.len()];
    for i in 0..rheaders.len() {
        let v: Vec<&str> = rheaders[i].split_terminator('|').collect();
        let mut s: String = String::new();
        s.push('|');
        s.push_str(v[0]);
        s.push('|');
        s.push_str(v[2]);
        refdata.name.push(v[2].to_string());
        s.push('|');
        s.push_str(v[3]);
        s.push('|');
        rheaders2.push(s);
        match v[3] {
            "5'UTR" => {
                refdata.segtype.push("U".to_string());
            }
            "L-REGION+V-REGION" => {
                refdata.segtype.push("V".to_string());
            }
            "D-REGION" => {
                refdata.segtype.push("D".to_string());
            }
            "J-REGION" => {
                refdata.segtype.push("J".to_string());
            }
            "C-REGION" => {
                refdata.segtype.push("C".to_string());
            }
            _ => {
                refdata.segtype.push("?".to_string());
            }
        }
        for j in 0..types.len() {
            if rheaders[i].contains(types[j]) {
                refdata.rtype[i] = j as i32;
            }
        }
        refdata.transcript.push(v[1].after(" ").to_string());
    }
    *rheaders = rheaders2;

    // Filter by TCR/BCR.

    if !is_tcr || !is_bcr {
        let mut to_delete = vec![false; refs.len()];
        for i in 0..refs.len() {
            if !is_tcr && (rheaders[i].contains("|TR") || rheaders[i].starts_with("TR")) {
                to_delete[i] = true;
            }
            if !is_bcr && (rheaders[i].contains("|IG") || rheaders[i].starts_with("IG")) {
                to_delete[i] = true;
            }
        }
        erase_if(refs, &to_delete);
        erase_if(rheaders, &to_delete);
        erase_if(&mut refdata.name, &to_delete);
        erase_if(&mut refdata.segtype, &to_delete);
        erase_if(&mut refdata.transcript, &to_delete);
        erase_if(&mut refdata.rtype, &to_delete);
    }

    // Fill in igjs and cs.

    for i in 0..rheaders.len() {
        if refdata.segtype[i] == "J".to_string() && refdata.rtype[i] >= 0 && refdata.rtype[i] < 3 {
            refdata.igjs.push(i);
        }
        if refdata.segtype[i] == "C".to_string() {
            refdata.cs.push(i);
        }
    }

    // Fill in id.

    for i in 0..rheaders.len() {
        refdata.id.push(rheaders[i].between("|", "|").force_i32());
    }

    // Extend the reference.

    if (species == "human" || species == "mouse")
        && extended
        && ref_fasta_path.ends_with("/regions.fa")
    {
        let mut refs2 = Vec::<DnaString>::new();
        let mut rheaders2 = Vec::<String>::new();
        let aux_ref = &format!(
            "{}/supp_regions.fa",
            ref_fasta_path.rev_before("/regions.fa")
        );
        if path_exists(&aux_ref) {
            read_fasta_into_vec_dna_string_plus_headers(&aux_ref, &mut refs2, &mut rheaders2);
            refs.append(&mut refs2);
            rheaders.append(&mut rheaders2);
            // ◼ Note not appending to refdata.name.  This may be a bug.
        }
    }

    // Make lookup table for reference.

    make_kmer_lookup_12_single(&refs, &mut rkmers_plus);

    // Determine which V segments have matching UTRs in the reference.

    for t in 0..rheaders.len() {
        if !rheaders[t].contains("segment") {
            let name = rheaders[t].after("|").between("|", "|");
            if rheaders[t].contains("UTR") {
                refdata.has_utr.insert(name.to_string(), true);
            }
        }
    }
    for t in 0..rheaders.len() {
        if !rheaders[t].contains("segment") {
            let name = rheaders[t].after("|").between("|", "|");
            if rheaders[t].contains("V-REGION") {
                refdata.has_utr.entry(name.to_string()).or_insert(false);
            }
        }
    }
}

pub fn make_vdj_ref_data(
    refdata: &mut RefData,
    imgt: bool,
    species: &String,
    extended: bool,
    is_tcr: bool,
    is_bcr: bool,
) {
    let ref_fasta = vdj_ref_path(&species, imgt);
    make_vdj_ref_data_core(refdata, &ref_fasta, extended, is_tcr, is_bcr);
}
