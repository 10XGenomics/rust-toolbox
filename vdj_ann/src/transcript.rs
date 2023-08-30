// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Code that analyzes transcripts.

use crate::annotate::get_cdr3_using_ann;
use crate::refx::RefData;
use amino::{have_start, have_stop};
use bitflags::bitflags;
use debruijn::dna_string::DnaString;
use debruijn::kmer::Kmer20;
use debruijn::{Mer, Vmer};
use hyperbase::Hyper;
use io_utils::fwriteln;
use kmer_lookup::make_kmer_lookup_20_single;
use std::cmp::max;
use std::io::prelude::*;
use vector_utils::{lower_bound1_3, unique_sort};

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TEST FOR VALID VDJ SEQUENCE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// #[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
// pub enum UnproductiveContigCause {
//     NoCdr3,
//     Misordered,
//     NotFull,
//     TooLarge,
// }
bitflags! {
    #[derive(Debug, Default,PartialEq)]
    pub struct UnproductiveContig: u16 {
        const NOT_FULL_LEN = 1u16;
        const MISSING_V_START = 2u16;
        const PREMATURE_STOP = 4u16;
        const FRAMESHIFT = 8u16;
        const NO_CDR3 = 16u16;
        const SIZE_DELTA = 32u16;
        const MISORDERED =  64u16;
    }
}

#[derive(Debug)]
pub struct ContigStatus {
    pub productive: bool,
    pub unproductive_cause: UnproductiveContig,
}

// ann: { ( start on sequence, match length, ref tig, start on ref tig, mismatches on sequence ) }.

#[derive(Debug)]
pub struct Annotation {
    pub seq_start: usize, // Start position on sequence
    pub match_len: usize, // Match length
    pub ref_tig: usize,   // Reference tig index (ordered in ref.fa starting from 0)
    pub tig_start: usize, // Start position on reference tig
    pub mismatches: usize,
}
impl From<(i32, i32, i32, i32, i32)> for Annotation {
    fn from(ann_element: (i32, i32, i32, i32, i32)) -> Self {
        Annotation {
            seq_start: ann_element.0 as usize,
            match_len: ann_element.1 as usize,
            ref_tig: ann_element.2 as usize,
            tig_start: ann_element.3 as usize,
            mismatches: ann_element.4 as usize,
        }
    }
}

pub fn is_valid(
    b: &DnaString,
    refdata: &RefData,
    ann: &[(i32, i32, i32, i32, i32)],
    logme: bool,
    log: &mut Vec<u8>,
    is_gd: Option<bool>,
) -> ContigStatus {
    // Unwrap gamma/delta mode flag
    let gd_mode = is_gd.unwrap_or(false);
    let refs = &refdata.refs;
    let rheaders = &refdata.rheaders;
    // let not_full_length = false;
    let mut missing_vstart = true;
    // let premature_stop = true;
    // let frameshift_mut = false;
    // let missing_cdr3 = false;
    // let too_large = true;
    // let misordered = false;
    let mut contig_status: UnproductiveContig = Default::default();
    let mut never_full = true;
    // two passes, one for light chains and one for heavy chains
    for pass in 0..2 {
        let mut m = "A";
        if pass == 1 {
            m = "B";
        }
        println!(">> Pass: {:?}, m: {:?}", pass, m);
        let mut vstarts = Vec::<i32>::new();
        let mut jstops = Vec::<i32>::new();
        let mut first_vstart: i32 = -1;
        let mut first_vstart_len: i32 = -1;
        let mut last_jstop: i32 = -1;
        let mut last_jstop_len: i32 = -1;
        let mut igh = false;
        for j in 0..ann.len() {
            let annot = Annotation::from(ann[j]);
            if rheaders[annot.ref_tig].contains("IGH") {
                igh = true;
            }
            if !rheaders[annot.ref_tig].contains("5'UTR")
                && ((m == "A"
                    && (rheaders[annot.ref_tig].contains("TRAV")
                        || (rheaders[annot.ref_tig].contains("TRGV") && gd_mode)
                        || rheaders[annot.ref_tig].contains("IGHV")))
                    || (m == "B"
                        && (rheaders[annot.ref_tig].contains("TRBV")
                            || (rheaders[annot.ref_tig].contains("TRDV") && gd_mode)
                            || rheaders[annot.ref_tig].contains("IGLV")
                            || rheaders[annot.ref_tig].contains("IGKV"))))
            {
                if first_vstart < 0 {
                    first_vstart = annot.seq_start as i32;
                    first_vstart_len = (refs[annot.ref_tig].len() - annot.tig_start) as i32;
                }
                if annot.tig_start == 0 {
                    vstarts.push(annot.seq_start as i32);
                }
            }
            if (m == "A"
                && (rheaders[annot.ref_tig].contains("TRAJ")
                    || (rheaders[annot.ref_tig].contains("TRGJ") && gd_mode)
                    || rheaders[annot.ref_tig].contains("IGHJ")))
                || (m == "B"
                    && (rheaders[annot.ref_tig].contains("TRBJ")
                        || (rheaders[annot.ref_tig].contains("TRDJ") && gd_mode)
                        || rheaders[annot.ref_tig].contains("IGLJ")
                        || rheaders[annot.ref_tig].contains("IGKJ")))
            {
                last_jstop = (annot.seq_start + annot.match_len) as i32;
                last_jstop_len = (annot.tig_start + annot.match_len) as i32;
                if annot.tig_start + annot.match_len == refs[annot.ref_tig].len() {
                    jstops.push((annot.seq_start + annot.match_len) as i32);
                }
            }
        }
        unique_sort(&mut vstarts);
        unique_sort(&mut jstops);
        println!(">>>> vstarts: {:?}", vstarts);
        println!(">>>> jstops: {:?}", jstops);
        let mut full = false;
        // 2 passes to check frameshifts (finding the start/stop codon)
        for inner_pass in 1..3 {
            println!(">>>>>> inner_pass: {:?}", inner_pass);
            if inner_pass == 2 && full {
                continue;
            }
            let mut msg = "";
            if inner_pass == 2 {
                msg = "frameshifted ";
            };
            for start in vstarts.iter() {
                if !have_start(b, *start as usize) {
                    continue;
                }
                missing_vstart = false;
                for stop in jstops.iter() {
                    let n = stop - start;
                    // on second pass, go through with checking for stop codon regardless of n % 3 value
                    if inner_pass == 2 || n % 3 == 1 {
                        let mut stop_codon = false;
                        // shouldn't it be stop-3+1????????????????????????????????
                        for j in (*start..stop - 3).step_by(3) {
                            if have_stop(b, j as usize) {
                                stop_codon = true;
                            }
                        }
                        if !stop_codon {
                            if inner_pass == 1 {
                                full = true;
                                never_full = false;
                            } else {
                                contig_status |= UnproductiveContig::FRAMESHIFT;
                            }
                            if logme {
                                fwriteln!(
                                    log,
                                    "{}full length transcript of length {}",
                                    msg,
                                    b.len()
                                );
                            }
                        } else {
                            contig_status |= UnproductiveContig::PREMATURE_STOP;
                            if logme {
                                fwriteln!(
                                    log,
                                    "{}full length stopped transcript of length {}",
                                    msg,
                                    b.len()
                                );
                            }
                        }
                    }
                }
            }
        }
        let mut cdr3 = Vec::<(usize, Vec<u8>, usize, usize)>::new();
        get_cdr3_using_ann(b, refdata, ann, &mut cdr3);
        if cdr3.is_empty() {
            if logme {
                fwriteln!(log, "did not find CDR3");
            }
            contig_status |= UnproductiveContig::NO_CDR3;
        }
        let mut too_large = false;
        const MIN_DELTA: i32 = -25;
        const MIN_DELTA_IGH: i32 = -55;
        const MAX_DELTA: i32 = 35;
        if first_vstart >= 0 && last_jstop >= 0 && !cdr3.is_empty() {
            let delta = (last_jstop_len + first_vstart_len + 3 * cdr3[0].1.len() as i32 - 20)
                - (last_jstop - first_vstart);
            if logme {
                fwriteln!(log, "VJ delta = {}", delta);
            }
            let mut min_delta = MIN_DELTA;
            if igh {
                min_delta = MIN_DELTA_IGH;
            }
            if delta < min_delta || delta > MAX_DELTA {
                too_large = true;
                if logme {
                    fwriteln!(log, "delta too large");
                }
            }
        }
        let mut misordered = false;
        for j1 in 0..ann.len() {
            let annot1 = Annotation::from(ann[j1]);
            for j2 in j1 + 1..ann.len() {
                let annot2 = Annotation::from(ann[j2]);
                if (refdata.is_j(annot1.ref_tig) && refdata.is_v(annot2.ref_tig))
                    || (refdata.is_j(annot1.ref_tig) && refdata.is_u(annot2.ref_tig))
                    || (refdata.is_j(annot1.ref_tig) && refdata.is_d(annot2.ref_tig))
                    || (refdata.is_v(annot1.ref_tig) && refdata.is_u(annot2.ref_tig))
                    || (refdata.is_c(annot1.ref_tig) && !refdata.is_c(annot2.ref_tig))
                {
                    misordered = true;
                }
            }
        }
        if misordered {
            if logme {
                fwriteln!(log, "misordered");
            }
            contig_status |= UnproductiveContig::MISORDERED;
        }
        if too_large {
            if logme {
                fwriteln!(log, "too large");
            }
            contig_status |= UnproductiveContig::SIZE_DELTA;
        }
        if full && !too_large && !misordered && contig_status.is_empty() {
            return ContigStatus {
                productive: true,
                unproductive_cause: contig_status,
            };
        }
    }
    if missing_vstart {
        contig_status |= UnproductiveContig::MISSING_V_START;
    }
    if never_full {
        if logme {
            fwriteln!(log, "not full");
        }

        contig_status |= UnproductiveContig::NOT_FULL_LEN;
    }

    ContigStatus {
        productive: false,
        unproductive_cause: contig_status,
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// JUNCTION REGION CODE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given a valid contig, find the junction sequence, which we define to be the 100
// bases ending where the right end of a J region aligns to the contig.

pub fn junction_seq(
    tig: &DnaString,
    refdata: &RefData,
    ann: &[(i32, i32, i32, i32, i32)],
    jseq: &mut DnaString,
    is_gd: Option<bool>,
) {
    // Unwrap gamma/delta mode flag
    let gd_mode = is_gd.unwrap_or(false);
    let refs = &refdata.refs;
    let rheaders = &refdata.rheaders;
    const TAG: i32 = 100;
    let mut jstops = Vec::<i32>::new();
    for j in 0..ann.len() {
        let l = ann[j].0 as usize;
        let len = ann[j].1 as usize;
        let t = ann[j].2 as usize;
        let p = ann[j].3 as usize;
        if (rheaders[t].contains("TRAJ")
            || rheaders[t].contains("IGHJ")
            || rheaders[t].contains("TRBJ")
            || rheaders[t].contains("IGLJ")
            || rheaders[t].contains("IGKJ")
            || (rheaders[t].contains("TRGJ") && gd_mode)
            || (rheaders[t].contains("TRDJ") && gd_mode))
            && p + len == refs[t].len()
            && l + len >= TAG as usize
        {
            jstops.push((l + len) as i32);
        }
    }
    unique_sort(&mut jstops);
    // note: if called on a valid contig, jstops will not be empty
    assert!(!jstops.is_empty());
    // note: at this point is presumably a rare event for jstops to have > 1 element
    let jstop = jstops[0];
    let jstart = jstop - TAG;
    *jseq = tig.slice(jstart as usize, jstop as usize).to_owned();
}

// Given a valid contig, find the read support for the junction sequence.  Return a
// pair (UMIs, nreads), consisting of the number of UMIs that cover the junction
// sequence, and the total number of reads on those UMIs that cover the junction
// sequence.
//
// This is a very restrictive definition of junction support!
//
// Note that it is possible to find zero UMIs, because each UMI individually may
// not cover the junction.

pub fn junction_supp(
    tig: &DnaString,
    reads: &[DnaString],
    x: &Hyper,
    umi_id: &[i32],
    refdata: &RefData,
    ann: &[(i32, i32, i32, i32, i32)],
    jsupp: &mut (i32, i32),
    is_gd: Option<bool>,
) {
    let mut jseq = DnaString::new();
    junction_seq(tig, refdata, ann, &mut jseq, is_gd);
    junction_supp_core(reads, x, umi_id, &jseq, jsupp);
}

pub fn junction_supp_core(
    reads: &[DnaString],
    x: &Hyper,
    umi_id: &[i32],
    jseq: &DnaString,
    jsupp: &mut (i32, i32),
) {
    let mut ids = Vec::<i32>::new();
    // ◼ What we're doing here is converting a Vec<u32> into a Vec<i32>.
    // ◼ There should be a function to do that.
    for e in 0..x.h.g.edge_count() {
        for id in x.ids[e].iter() {
            ids.push(*id as i32);
        }
    }
    unique_sort(&mut ids);
    let tigs = vec![jseq.clone()];
    let mut kmers_plus = Vec::<(Kmer20, i32, i32)>::new();
    make_kmer_lookup_20_single(&tigs, &mut kmers_plus);
    let mut idi = 0;
    let k = x.h.k as usize;
    jsupp.0 = 0;
    jsupp.1 = 0;
    while idi < ids.len() {
        let mut idj = idi + 1;
        while idj < ids.len() && umi_id[ids[idj] as usize] == umi_id[ids[idi] as usize] {
            idj += 1;
        }
        let mut mm = Vec::<(i32, i32)>::new();
        for r in idi..idj {
            let ida = ids[r];
            let b = &reads[ida as usize];
            if b.len() < k {
                continue;
            }
            for j in 0..b.len() - k + 1 {
                let z: Kmer20 = b.get_kmer(j);
                let low = lower_bound1_3(&kmers_plus, &z) as usize;
                for m in low..kmers_plus.len() {
                    if kmers_plus[m].0 != z {
                        break;
                    }
                    let p = kmers_plus[m].2 as usize;
                    if j > 0 && p > 0 && b.get(j - 1) == jseq.get(p - 1) {
                        continue;
                    }
                    let mut len = k;
                    loop {
                        if j + len == b.len() || p + len == jseq.len() {
                            break;
                        }
                        if b.get(j + len) != jseq.get(p + len) {
                            break;
                        }
                        len += 1;
                    }
                    mm.push((p as i32, (p + len) as i32));
                }
            }
        }
        mm.sort_unstable();
        let mut cov = true;
        if mm.is_empty() || mm[0].0 > 0 {
            cov = false;
        }
        let mut reach = 0;
        for m in mm.iter() {
            if m.0 <= reach {
                reach = max(reach, m.1);
            }
        }
        if reach < jseq.len() as i32 {
            cov = false;
        }
        if cov {
            jsupp.0 += 1;
            jsupp.1 += mm.len() as i32;
        }
        idi = idj;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refx;

    #[test]
    fn test_is_valid() {
        use debruijn::dna_string::DnaString;
        use refx::RefData;

        let refdata =
            RefData::from_fasta(String::from("test/inputs/test_productive_is_valid_ref.fa"));

        let mut log: Vec<u8> = vec![];

        // // NO_CDR3
        // let b = DnaString::from_dna_string("ACATCTCTCTCATTAGAGGTTGATCTTTGAGGAAAACAGGGTGTTGCCTAAAGGATGAAAGTGTTGAGTCTGTTGTACCTGTTGACAGCCATTCCTGGTATCCTGTCTGATGTACAGCTTCAGGAGTCAGGACCTGGCCTCGTGAAACCTTCTCAGTCTCTGTCTCTCACCTGCTCTGTCACTGGCTACTCCATCACCAGTGGTTATTACTGGAACTGGATCCGGCAGTTTCCAGGAAACAAACTGGAATGGATGGGCTACATAAGCTACGACGGTAGCAATAACTACAACCCATCTCTCAAAAATCGAATCTCCATCACTCGTGACACATCTAAGAACCAGTTTTTCCTGAAGTTGAATTCTGTGACTACTGAGGACACAGCTACATATTACTGTGCAAGATCTACTATGATTACGACGGGGTTTGCTTACTGGGGCCAAGGGACTCTGGTCACTGTCTCTGCAG");
        // let ann = [
        //     (54, 148, 0, 0, 16),
        //     (205, 246, 0, 148, 58),
        //     (418, 48, 1, 0, 2),
        // ];
        // let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        // assert!(!return_value.productive);
        // assert_eq!(return_value.unproductive_cause, UnproductiveContig::NO_CDR3);

        // // NOT_FULL_LEN
        // let b = DnaString::from_dna_string("GAACACATGCCCAATGTCCTCTCCACAGACACTGAACACACTGACTCCAACCATGGGGTGGAGTCTGGATCTTTTTCTTCCTCCTGTCAGGAACTGCAGGTGTCCACTCTGAGGTCCAGCTGCAACAGTCTGGACCTGAGCTGGTGAAGCCTGGGGCTTCAGTGAAGATATCCTGCAAGGCTTCTGGCTACACATTCACTGACTACTACATGAACTGGGTGAAGCAGAGCCATGGAAAGAGCCTTGAGTGGATTGGACTTGTTAATCCTAACAATGGTGGTACTAGCTACAACCAGAAGTTCAAGGGCAAGGCCACATTGACTGTAGACAAGTCCTCCAGCACAGCCTACATGGAGCTCCGCAGCCTGACATCTGAGGACTCTGCGGTCTATTACTGTGCAAGAAGGGCTAGGGTAACTGGGATGCTATGGACTACTGGGGTCAAGGAACCTCAGTCACCGTCTCCTCAGAGAGTCAGTCCTTCCCAAATGTCTTCCCCCTCGTCTCCTGCGAGAGCCCCCTGTCTGATAAGAATCTGGTGGCCATGGGCTGCCTGGCCCGGGACTTCCTGCCCAGCACCATTTCCTTCACCTGGAACTACCAGAACAACACTGAAGTCATCCAGGGTATCAGAACCTTCCCAACACTGAGGACAGGGGGCAAGTACCTAGCCACCTCGCA");
        // let ann = [(64, 340, 2, 11, 11)];
        // let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        // assert!(!return_value.productive);
        // assert_eq!(
        //     return_value.unproductive_cause,
        //     UnproductiveContig::NOT_FULL_LEN
        // );

        // // [NOT_FULL_LEN, SIZE_DELTA]
        // let ann = [
        //     (64, 340, 2, 11, 11),
        //     (416, 54, 3, 0, 4),
        //     (470, 211, 4, 0, 0),
        // ];
        // let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        // assert!(!return_value.productive);
        // assert_eq!(
        //     return_value.unproductive_cause,
        //     UnproductiveContig::NOT_FULL_LEN | UnproductiveContig::SIZE_DELTA
        // );

        // // [Misordered, NotFull, TooLarge]
        // let ann = [
        //     (416, 54, 3, 0, 4),
        //     (64, 340, 2, 11, 11),
        //     (470, 211, 4, 0, 0),
        // ];
        // let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        // assert!(!return_value.productive);
        // assert_eq!(
        //     return_value.unproductive_cause,
        //     UnproductiveContig::NOT_FULL_LEN
        //         | UnproductiveContig::SIZE_DELTA
        //         | UnproductiveContig::MISORDERED
        // );

        // Productive
        let b = DnaString::from_dna_string("GGACCAAAATTCAAAGACAAAATGCATTGTCAAGTGCAGATTTTCAGCTTCCTGCTAATCAGTGCCTCAGTCATAATGTCCAGAGGACAAATTGTTCTCACCCAGTCTCCAGCAATCATGTCTGCATCTCCAGGGGAGAAGGTCACCATAACCTGCAGTGCCAGCTCAAGTGTAAGTTACATGCACTGGTTCCAGCAGAAGCCAGGCACTTCTCCCAAACTCTGGATTTATAGCACATCCAACCTGGCTTCTGGAGTCCCTGCTCGCTTCAGTGGCAGTGGATCTGGGACCTCTTACTCTCTCACAATCAGCCGAATGGAGGCTGAAGATGCTGCCACTTATTACTGCCAGCAAAGGAGTAGTTACCCGCTCACGTTCGGTGCTGGGACCAAGCTGGAGCTGAAACGGGCTGATGCTGCACCAACTGTATCCATCTTCCCACCATCCAGTGAGCAGTTAACATCTGGAGGTGCCTCAGTCGTGTGCTTC");
        let ann = [(21, 352, 5, 0, 3), (368, 38, 6, 0, 0), (406, 83, 7, 0, 0)];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        println!("\n\n{:?}\n", return_value);
        assert!(return_value.productive);
        assert!(return_value.unproductive_cause.is_empty());
        assert!(1 == 2);
    }
}
