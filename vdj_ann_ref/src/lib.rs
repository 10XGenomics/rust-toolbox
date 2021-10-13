// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains code to make reference data.
//
// ◼ Experiment with building a reference from scratch.  This would be a better
// ◼ solution than ad hoc editing of a flawed reference.
//
// ◼ Document reference sequence requirements so that a customer who wishes to
// ◼ create a reference for a new species will know the conventions used by the
// ◼ code.

use io_utils::read_to_string_safe;

use vdj_ann::refx::{make_vdj_ref_data_core, RefData};

pub fn human_ref() -> String {
    include_str!["../vdj_refs/human/fasta/regions.fa"].to_string()
}

pub fn human_supp_ref() -> String {
    include_str!["../vdj_refs/human/fasta/supp_regions.fa"].to_string()
}

pub fn human_ref_2_0() -> String {
    include_str!["../vdj_refs_2.0/human/fasta/regions.fa"].to_string()
}

pub fn human_ref_3_1() -> String {
    include_str!["../vdj_refs_3.1/human/fasta/regions.fa"].to_string()
}

pub fn human_ref_4_0() -> String {
    include_str!["../vdj_refs_4.0/human/fasta/regions.fa"].to_string()
}

pub fn mouse_ref() -> String {
    include_str!["../vdj_refs/mouse/fasta/regions.fa"].to_string()
}

pub fn mouse_supp_ref() -> String {
    include_str!["../vdj_refs/mouse/fasta/supp_regions.fa"].to_string()
}

pub fn mouse_ref_3_1() -> String {
    include_str!["../vdj_refs_3.1/mouse/fasta/regions.fa"].to_string()
}

pub fn mouse_ref_4_0() -> String {
    include_str!["../vdj_refs_4.0/mouse/fasta/regions.fa"].to_string()
}

// ids_to_use_opt: Optional hashSet of ids. If specified only reference
// entries with id in the HashSet is used to construct RefData

pub fn make_vdj_ref_data(
    refdata: &mut RefData,
    imgt: bool,
    species: &String,
    extended: bool,
    is_tcr: bool,
    is_bcr: bool,
) {
    let mut refx = String::new();
    let mut ext_refx = String::new();
    if !imgt && species == "human" {
        refx = human_ref();
        if extended {
            ext_refx = human_supp_ref();
        }
    }
    if !imgt && species == "mouse" {
        refx = mouse_ref();
        if extended {
            ext_refx = mouse_supp_ref();
        }
    }
    if imgt && species == "human" {
        refx = read_to_string_safe(
            "/mnt/opt/refdata_cellranger/vdj/\
             vdj_IMGT_20170916-2.1.0/fasta/regions.fa",
        );
    }
    if imgt && species == "mouse" {
        refx = read_to_string_safe(
            "/mnt/opt/refdata_cellranger/vdj/\
             vdj_IMGT_mouse_20180723-2.2.0/fasta/regions.fa",
        );
    }
    if refx.is_empty() {
        panic!("Reference file has zero length.");
    }
    make_vdj_ref_data_core(refdata, &refx, &ext_refx, is_tcr, is_bcr, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    // The following test checks for alignment of a D region.  This example was fixed by code
    // changes in March 2020.
    #[test]
    fn test_d_region_alignment() {
        use debruijn::dna_string::DnaString;
        use vdj_ann::annotate::annotate_seq;
        use vdj_ann::refx::make_vdj_ref_data_core;
        let seq = DnaString::from_acgt_bytes(
            b"GGAGGTGCGAATGACTCTGCTCTCTGTCCTGTCTCCTCATCTGCAAAATTAGGAAGCCTGTCTTGATTATCTCCAGGAA\
            CCTCCCACCTCTTCATTCCAGCCTCTGACAAACTCTGCACATTAGGCCAGGAGAAGCCCCCGAGCCAAGTCTCTTTTCTCATTCTC\
            TTCCAACAAGTGCTTGGAGCTCCAAGAAGGCCCCCTTTGCACTATGAGCAACCAGGTGCTCTGCTGTGTGGTCCTTTGTCTCCTGG\
            GAGCAAACACCGTGGATGGTGGAATCACTCAGTCCCCAAAGTACCTGTTCAGAAAGGAAGGACAGAATGTGACCCTGAGTTGTGAA\
            CAGAATTTGAACCACGATGCCATGTACTGGTACCGACAGGACCCAGGGCAAGGGCTGAGATTGATCTACTACTCACAGATAGTAAA\
            TGACTTTCAGAAAGGAGATATAGCTGAAGGGTACAGCGTCTCTCGGGAGAAGAAGGAATCCTTTCCTCTCACTGTGACATCGGCCC\
            AAAAGAACCCGACAGCTTTCTATCTCTGTGCCAGTAGTATTTTTCTTGCCGGGACAGGGGGCTGGAGCGGCACTGAAGCTTTCTTT\
            GGACAAGGCACCAGACTCACAGTTGTAGAGGACCTGAACAAGGTGTTCCCACCCGAGGTCGCTGTGTTTGAGCCATCAGA",
        );
        let (refx, ext_refx) = (human_ref(), String::new());
        let (is_tcr, is_bcr) = (true, false);
        let mut refdata = RefData::new();
        make_vdj_ref_data_core(&mut refdata, &refx, &ext_refx, is_tcr, is_bcr, None);
        let mut ann = Vec::<(i32, i32, i32, i32, i32)>::new();
        annotate_seq(&seq, &refdata, &mut ann, true, false, true);
        let mut have_d = false;
        for i in 0..ann.len() {
            if refdata.is_d(ann[i].2 as usize) {
                have_d = true;
            }
        }
        if !have_d {
            panic!("\nFailed to find alignment of D region.\n");
        }
    }
}
