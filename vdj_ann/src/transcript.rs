// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Code that analyzes transcripts.

use crate::annotate::get_cdr3_using_ann;
use crate::refx::RefData;
use amino::{have_start, have_stop};
use debruijn::{dna_string::DnaString, kmer::Kmer20, Mer, Vmer};
use hyperbase::Hyper;
use itertools::iproduct;
use kmer_lookup::make_kmer_lookup_20_single;
use std::cmp::max;
use vdj_types::{VdjChain, VDJ_CHAINS};
use vector_utils::{lower_bound1_3, unique_sort};

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TEST FOR VALID VDJ SEQUENCE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

const MIN_DELTA: i32 = -25;
const MIN_DELTA_IGH: i32 = -55;
const MAX_DELTA: i32 = 35;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ContigStatus {
    full_length: Option<bool>,
    has_vstart: Option<bool>,
    inframe: Option<bool>,
    no_premature_stop: Option<bool>,
    has_cdr3: Option<bool>,
    has_expected_size: Option<bool>,
    correct_ann_order: Option<bool>,
}

impl ContigStatus {
    fn is_productive(&self) -> bool {
        match (
            self.full_length,
            self.has_vstart,
            self.inframe,
            self.no_premature_stop,
            self.has_cdr3,
            self.has_expected_size,
            self.correct_ann_order,
        ) {
            (
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
            ) => true,
            (_, _, _, _, _, _, _) => false,
        }
    }

    fn order_by(&self) -> u16 {
        match (
            self.full_length,
            self.has_vstart,
            self.inframe,
            self.no_premature_stop,
            self.has_cdr3,
            self.has_expected_size,
            self.correct_ann_order,
        ) {
            (Some(_), Some(_), Some(_), Some(_), Some(_), Some(_), Some(_)) => 0,
            (Some(_), Some(_), Some(_), Some(_), Some(_), Some(_), _) => 1,
            (Some(_), Some(_), Some(_), Some(_), Some(_), _, _) => 2,
            (Some(_), Some(_), Some(_), Some(_), _, _, _) => 3,
            (Some(_), Some(_), Some(_), _, _, _, _) => 4,
            (Some(_), Some(_), _, _, _, _, _) => 5,
            (Some(_), _, _, _, _, _, _) => 6,
            _ => 7,
        }
    }
}

pub struct VdjChainSpecificVJGenes {
    v_type: String,
    j_type: String,
}

impl From<VdjChain> for VdjChainSpecificVJGenes {
    fn from(chain: VdjChain) -> Self {
        VdjChainSpecificVJGenes {
            v_type: format!("{chain}V"),
            j_type: format!("{chain}J"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Vstart {
    ref_id: usize,
    tig_start: usize,
}

#[derive(Clone, Copy)]
pub struct Jstop {
    ref_id: usize,
    tig_stop: usize,
}

#[derive(Debug)]
pub struct Annotation {
    tig_start: usize,
    match_length: usize,
    ref_id: usize,
    ref_start: usize,
}
#[derive(Debug)]
pub struct ChainSpecificContigStatus {
    _vdj_chain: VdjChain,
    state: Option<ContigStatus>,
}

impl ChainSpecificContigStatus {
    pub fn new(
        vdj_chain: VdjChain,
        ann: &[(i32, i32, i32, i32, i32)],
        reference: &RefData,
        contig: &DnaString,
    ) -> Self {
        let vj_pair = VdjChainSpecificVJGenes::from(vdj_chain);
        let rheaders = &reference.rheaders;
        let refs = &reference.refs;
        let annotation: Vec<Annotation> = ann
            .iter()
            .map(
                |(tig_start, match_length, ref_id, ref_start, _)| Annotation {
                    tig_start: *tig_start as usize,
                    match_length: *match_length as usize,
                    ref_id: *ref_id as usize,
                    ref_start: *ref_start as usize,
                },
            )
            .collect();
        let mut vstarts: Vec<Vstart> = annotation
            .iter()
            .filter(|a| reference.is_v(a.ref_id))
            .filter(|a| rheaders[a.ref_id].contains(vj_pair.v_type.as_str()))
            .filter(|a| a.ref_start == 0)
            .map(|a| Vstart {
                ref_id: a.ref_id,
                tig_start: a.tig_start,
            })
            .collect();
        let jstops: Vec<Jstop> = annotation
            .iter()
            .filter(|a| reference.is_j(a.ref_id))
            .filter(|a| rheaders[a.ref_id].contains(vj_pair.j_type.as_str()))
            .filter(|a| a.ref_start + a.match_length == refs[a.ref_id].len())
            .map(|a| Jstop {
                ref_id: a.ref_id,
                tig_stop: a.tig_start + a.match_length,
            })
            .collect();

        if vstarts.is_empty() & jstops.is_empty() {
            return ChainSpecificContigStatus {
                _vdj_chain: vdj_chain,
                state: None,
            };
        }

        let mut contig_status = ContigStatus {
            full_length: match (vstarts.is_empty(), jstops.is_empty()) {
                (false, false) => Some(true),
                (_, _) => Some(false),
            },
            ..Default::default()
        };

        // filter vstarts to require START codon
        vstarts.retain(|v| have_start(contig, v.tig_start));
        contig_status.has_vstart = match (contig_status.full_length, vstarts.is_empty()) {
            (Some(true), false) => Some(true),
            (_, _) => Some(false),
        };

        let inframe_pair = Self::find_inframe_vdj_pair(vstarts, jstops);
        contig_status.inframe = Some(inframe_pair.is_some());

        contig_status.no_premature_stop = if let Some((vstart, jstop)) = inframe_pair {
            let mut no_premature_stop = Some(true);
            for j in (vstart.tig_start..jstop.tig_stop - 3).step_by(3) {
                if have_stop(contig, j) {
                    no_premature_stop = Some(false);
                }
            }
            no_premature_stop
        } else {
            None
        };

        let mut cdr3 = Vec::<(usize, Vec<u8>, usize, usize)>::new();
        get_cdr3_using_ann(contig, reference, ann, &mut cdr3);
        contig_status.has_cdr3 = Some(!cdr3.is_empty());

        contig_status.has_expected_size =
            if let (Some((vstart, jstop)), Some(cdr3)) = (inframe_pair, cdr3.first()) {
                let expected_len = (refs[vstart.ref_id].len() + refs[jstop.ref_id].len()) as i32
                    + (3 * cdr3.1.len() as i32)
                    - 20;
                let observed_len = jstop.tig_stop as i32 - vstart.tig_start as i32;
                let delta = expected_len - observed_len;
                let min_delta = if vdj_chain == VdjChain::IGH {
                    MIN_DELTA_IGH
                } else {
                    MIN_DELTA
                };
                if delta < min_delta || delta > MAX_DELTA {
                    Some(false)
                } else {
                    Some(true)
                }
            } else {
                None
            };

        let mut correct_order = true;
        for j1 in 0..annotation.len() {
            let ref_id1 = annotation[j1].ref_id;
            for j2 in j1 + 1..annotation.len() {
                let ref_id2 = annotation[j2].ref_id;
                if (reference.is_j(ref_id1) && reference.is_v(ref_id2))
                    || (reference.is_j(ref_id1) && reference.is_u(ref_id2))
                    || (reference.is_j(ref_id1) && reference.is_d(ref_id2))
                    || (reference.is_v(ref_id1) && reference.is_u(ref_id2))
                    || (reference.is_c(ref_id1) && !reference.is_c(ref_id2))
                {
                    correct_order = false;
                }
            }
        }
        contig_status.correct_ann_order = Some(correct_order);

        ChainSpecificContigStatus {
            _vdj_chain: vdj_chain,
            state: Some(contig_status),
        }
    }

    fn find_inframe_vdj_pair(vstarts: Vec<Vstart>, jstops: Vec<Jstop>) -> Option<(Vstart, Jstop)> {
        let mut vj_combinations: Vec<(Vstart, Jstop, i32)> = iproduct!(vstarts, jstops)
            .map(|(v, j)| (v, j, j.tig_stop as i32 - v.tig_start as i32))
            .filter(|(_, _, n)| n > &0)
            .filter(|(_, _, n)| n % 3 == 1)
            .collect();
        vj_combinations.sort_by_key(|x| x.2);
        let inframe_pair = vj_combinations.last().map(|(v, j, _)| (*v, *j));
        inframe_pair
    }
}

pub fn is_valid(
    b: &DnaString,
    refdata: &RefData,
    ann: &[(i32, i32, i32, i32, i32)],
) -> (bool, ContigStatus) {
    let mut contig_status: Vec<ContigStatus> = VDJ_CHAINS
        .into_iter()
        .map(|chain| VdjChain::from_str(chain).unwrap())
        .map(|chain| ChainSpecificContigStatus::new(chain, ann, refdata, b))
        .filter_map(|contig_status| contig_status.state)
        .collect();
    contig_status.sort_by_key(|cs| std::cmp::Reverse(cs.order_by()));
    if let Some(cs) = contig_status.last() {
        return (cs.is_productive(), cs.clone());
    }
    (false, ContigStatus::default())
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
