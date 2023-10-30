// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Code that analyzes transcripts.

use crate::annotate::get_cdr3_using_ann;
use crate::refx::RefData;
use amino::{have_start, have_stop};
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

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum UnproductiveContigCause {
    NoCdr3,
    Misordered,
    NotFull,
    TooLarge,
}

pub fn is_valid(
    b: &DnaString,
    refdata: &RefData,
    ann: &[(i32, i32, i32, i32, i32)],
    logme: bool,
    log: &mut Vec<u8>,
    is_gd: Option<bool>,
) -> (bool, Vec<UnproductiveContigCause>) {
    // Unwrap gamma/delta mode flag
    let gd_mode = is_gd.unwrap_or(false);
    let refs = &refdata.refs;
    let rheaders = &refdata.rheaders;
    let mut ret_vec = Vec::new();
    for pass in 0..2 {
        let mut m = "A";
        if pass == 1 {
            m = "B";
        }
        let mut vstarts = Vec::<i32>::new();
        let mut jstops = Vec::<i32>::new();
        let mut first_vstart: i32 = -1;
        let mut first_vstart_len: i32 = -1;
        let mut last_jstop: i32 = -1;
        let mut last_jstop_len: i32 = -1;
        let mut igh = false;
        for j in 0..ann.len() {
            let l = ann[j].0 as usize;
            let len = ann[j].1 as usize;
            let t = ann[j].2 as usize;
            let p = ann[j].3 as usize;
            if rheaders[t].contains("IGH") {
                igh = true;
            }
            if !rheaders[t].contains("5'UTR")
                && ((m == "A"
                    && (rheaders[t].contains("TRAV")
                        || (rheaders[t].contains("TRGV") && gd_mode)
                        || rheaders[t].contains("IGHV")))
                    || (m == "B"
                        && (rheaders[t].contains("TRBV")
                            || (rheaders[t].contains("TRDV") && gd_mode)
                            || rheaders[t].contains("IGLV")
                            || rheaders[t].contains("IGKV"))))
            {
                if first_vstart < 0 {
                    first_vstart = l as i32;
                    first_vstart_len = (refs[t].len() - p) as i32;
                }
                if p == 0 {
                    vstarts.push(l as i32);
                }
            }
            if (m == "A"
                && (rheaders[t].contains("TRAJ")
                    || (rheaders[t].contains("TRGJ") && gd_mode)
                    || rheaders[t].contains("IGHJ")))
                || (m == "B"
                    && (rheaders[t].contains("TRBJ")
                        || (rheaders[t].contains("TRDJ") && gd_mode)
                        || rheaders[t].contains("IGLJ")
                        || rheaders[t].contains("IGKJ")))
            {
                last_jstop = (l + len) as i32;
                last_jstop_len = (p + len) as i32;
                if p + len == refs[t].len() {
                    jstops.push((l + len) as i32);
                }
            }
        }
        unique_sort(&mut vstarts);
        unique_sort(&mut jstops);
        let mut full = false;
        for pass in 1..3 {
            if pass == 2 && full {
                continue;
            }
            let mut msg = "";
            if pass == 2 {
                msg = "frameshifted ";
            };
            for start in vstarts.iter() {
                if !have_start(b, *start as usize) {
                    continue;
                }
                for stop in jstops.iter() {
                    let n = stop - start;
                    if pass == 2 || n % 3 == 1 {
                        let mut stop_codon = false;
                        // shouldn't it be stop-3+1????????????????????????????????
                        for j in (*start..stop - 3).step_by(3) {
                            if have_stop(b, j as usize) {
                                stop_codon = true;
                            }
                        }
                        if !stop_codon {
                            if pass == 1 {
                                full = true;
                            }
                            if logme {
                                fwriteln!(
                                    log,
                                    "{}full length transcript of length {}",
                                    msg,
                                    b.len()
                                );
                            }
                        } else if logme {
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
        let mut cdr3 = Vec::<(usize, Vec<u8>, usize, usize)>::new();
        get_cdr3_using_ann(b, refdata, ann, &mut cdr3);
        if cdr3.is_empty() {
            if logme {
                fwriteln!(log, "did not find CDR3");
            }
            return (false, vec![UnproductiveContigCause::NoCdr3]);
        }
        let mut too_large = false;
        const MIN_DELTA: i32 = -25;
        const MIN_DELTA_IGH: i32 = -55;
        const MAX_DELTA: i32 = 35;
        if first_vstart >= 0 && last_jstop >= 0 {
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
            let t1 = ann[j1].2 as usize;
            for j2 in j1 + 1..ann.len() {
                let t2 = ann[j2].2 as usize;
                if (refdata.is_j(t1) && refdata.is_v(t2))
                    || (refdata.is_j(t1) && refdata.is_u(t2))
                    || (refdata.is_j(t1) && refdata.is_d(t2))
                    || (refdata.is_v(t1) && refdata.is_u(t2))
                    || (refdata.is_c(t1) && !refdata.is_c(t2))
                {
                    misordered = true;
                }
            }
        }
        if misordered {
            if logme {
                fwriteln!(log, "misordered");
            }
            ret_vec.push(UnproductiveContigCause::Misordered);
        }
        if too_large {
            if logme {
                fwriteln!(log, "too large");
            }
            ret_vec.push(UnproductiveContigCause::TooLarge);
        }
        if !full {
            if logme {
                fwriteln!(log, "not full");
            }
            ret_vec.push(UnproductiveContigCause::NotFull);
        }
        if full && !too_large && !misordered {
            return (true, vec![]);
        }
    }
    ret_vec.sort_unstable();
    ret_vec.dedup();
    (false, ret_vec)
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
            RefData::from_fasta(&String::from("test/inputs/test_productive_is_valid_ref.fa"));

        let mut log: Vec<u8> = vec![];

        // NoCdr3
        let b = DnaString::from_dna_string("ACATCTCTCTCATTAGAGGTTGATCTTTGAGGAAAACAGGGTGTTGCCTAAAGGATGAAAGTGTTGAGTCTGTTGTACCTGTTGACAGCCATTCCTGGTATCCTGTCTGATGTACAGCTTCAGGAGTCAGGACCTGGCCTCGTGAAACCTTCTCAGTCTCTGTCTCTCACCTGCTCTGTCACTGGCTACTCCATCACCAGTGGTTATTACTGGAACTGGATCCGGCAGTTTCCAGGAAACAAACTGGAATGGATGGGCTACATAAGCTACGACGGTAGCAATAACTACAACCCATCTCTCAAAAATCGAATCTCCATCACTCGTGACACATCTAAGAACCAGTTTTTCCTGAAGTTGAATTCTGTGACTACTGAGGACACAGCTACATATTACTGTGCAAGATCTACTATGATTACGACGGGGTTTGCTTACTGGGGCCAAGGGACTCTGGTCACTGTCTCTGCAG");
        let ann = [
            (54, 148, 0, 0, 16),
            (205, 246, 0, 148, 58),
            (418, 48, 1, 0, 2),
        ];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        assert!(!return_value.0);
        assert!(return_value
            .1
            .iter()
            .all(|item| vec![UnproductiveContigCause::NoCdr3].contains(item)));

        // NotFull
        let b = DnaString::from_dna_string("GAACACATGCCCAATGTCCTCTCCACAGACACTGAACACACTGACTCCAACCATGGGGTGGAGTCTGGATCTTTTTCTTCCTCCTGTCAGGAACTGCAGGTGTCCACTCTGAGGTCCAGCTGCAACAGTCTGGACCTGAGCTGGTGAAGCCTGGGGCTTCAGTGAAGATATCCTGCAAGGCTTCTGGCTACACATTCACTGACTACTACATGAACTGGGTGAAGCAGAGCCATGGAAAGAGCCTTGAGTGGATTGGACTTGTTAATCCTAACAATGGTGGTACTAGCTACAACCAGAAGTTCAAGGGCAAGGCCACATTGACTGTAGACAAGTCCTCCAGCACAGCCTACATGGAGCTCCGCAGCCTGACATCTGAGGACTCTGCGGTCTATTACTGTGCAAGAAGGGCTAGGGTAACTGGGATGCTATGGACTACTGGGGTCAAGGAACCTCAGTCACCGTCTCCTCAGAGAGTCAGTCCTTCCCAAATGTCTTCCCCCTCGTCTCCTGCGAGAGCCCCCTGTCTGATAAGAATCTGGTGGCCATGGGCTGCCTGGCCCGGGACTTCCTGCCCAGCACCATTTCCTTCACCTGGAACTACCAGAACAACACTGAAGTCATCCAGGGTATCAGAACCTTCCCAACACTGAGGACAGGGGGCAAGTACCTAGCCACCTCGCA");
        let ann = [(64, 340, 2, 11, 11)];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        assert!(!return_value.0);
        assert!(return_value
            .1
            .iter()
            .all(|item| vec![UnproductiveContigCause::NotFull].contains(item)));

        // [NotFull, TooLarge]
        let ann = [
            (64, 340, 2, 11, 11),
            (416, 54, 3, 0, 4),
            (470, 211, 4, 0, 0),
        ];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        assert!(!return_value.0);
        assert!(return_value.1.iter().all(|item| vec![
            UnproductiveContigCause::NotFull,
            UnproductiveContigCause::TooLarge
        ]
        .contains(item)));

        // [Misordered, NotFull, TooLarge]
        let ann = [
            (416, 54, 3, 0, 4),
            (64, 340, 2, 11, 11),
            (470, 211, 4, 0, 0),
        ];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        assert!(!return_value.0);
        assert!(return_value.1.iter().all(|item| vec![
            UnproductiveContigCause::NotFull,
            UnproductiveContigCause::TooLarge,
            UnproductiveContigCause::Misordered
        ]
        .contains(item)));

        // Productive
        let b = DnaString::from_dna_string("GGACCAAAATTCAAAGACAAAATGCATTGTCAAGTGCAGATTTTCAGCTTCCTGCTAATCAGTGCCTCAGTCATAATGTCCAGAGGACAAATTGTTCTCACCCAGTCTCCAGCAATCATGTCTGCATCTCCAGGGGAGAAGGTCACCATAACCTGCAGTGCCAGCTCAAGTGTAAGTTACATGCACTGGTTCCAGCAGAAGCCAGGCACTTCTCCCAAACTCTGGATTTATAGCACATCCAACCTGGCTTCTGGAGTCCCTGCTCGCTTCAGTGGCAGTGGATCTGGGACCTCTTACTCTCTCACAATCAGCCGAATGGAGGCTGAAGATGCTGCCACTTATTACTGCCAGCAAAGGAGTAGTTACCCGCTCACGTTCGGTGCTGGGACCAAGCTGGAGCTGAAACGGGCTGATGCTGCACCAACTGTATCCATCTTCCCACCATCCAGTGAGCAGTTAACATCTGGAGGTGCCTCAGTCGTGTGCTTC");
        let ann = [(21, 352, 5, 0, 3), (368, 38, 6, 0, 0), (406, 83, 7, 0, 0)];
        let return_value = is_valid(&b, &refdata, &ann, false, &mut log, None);
        assert!(return_value.0);
        assert!(return_value.1.is_empty());
    }
}
