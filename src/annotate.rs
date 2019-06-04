// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains code to annotate a contig, in the sense of finding alignments
// to VDJ reference contigs.  Also to find CDR3 sequences.  And some related things.

use bio::alignment::AlignmentOperation::*;
use debruijn::{dna_string::*, kmer::*, *};
use itertools::*;
use refx::*;
use serde::{Deserialize, Serialize};
use stats_utils::*;
use std::{
    cmp::{max, min},
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};
use string_utils::*;
use tenkit2::align_tools::*;
use tenkit2::amino::*;
use transcript::*;
use vec_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// START CODONS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn print_start_codon_positions(tig: &DnaString, log: &mut Vec<u8>) {
    let mut starts = Vec::<usize>::new();
    if tig.len() < 3 {
        return;
    }
    for i in 0..tig.len() - 3 {
        if have_start(&tig, i) {
            starts.push(i);
        }
    }
    fwriteln!(log, "start codons at {}", starts.iter().format(", "));
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// ASSIGN CHAIN TYPE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Assign a chain type to a given DNA sequence b.
//
// The chain type is either -1, meaning unassigned, or an index into the vector
// "IGH","IGK","IGL","TRA","TRB","TRD","TRG"
// representing a forward alignment to the chain type, or 7 + such an index,
// representing a reverse alignment.
//
// This takes as input the sequence b, plus the following auxiliary data
// structures:
// * a 20-mer kmer lookup table for the VDJ reference sequences,
//   both TCR and BCR;
// * a classification vector that assigns each reference sequence to either a
//   chain type index or -1.
//
// ◼ Ns are incorrectly handled.  See lena 100349 for lots of examples.

pub fn chain_type(
    b: &DnaString,
    rkmers_plus_full_20: &Vec<(Kmer20, i32, i32)>,
    rtype: &Vec<i32>,
) -> i8 {
    let n = 7;
    let k = 20;
    if b.len() < k {
        return -1 as i8;
    }
    let mut count_this = vec![0; 2 * n];
    let brc = b.rc();
    for l in 0..b.len() - k + 1 {
        let mut is_type = vec![false; 2 * n];
        for pass in 0..2 {
            let mut z = 0;
            if pass == 1 {
                z = n;
            }
            let x: Kmer20;
            if pass == 0 {
                x = b.get_kmer(l);
            } else {
                x = brc.get_kmer(l);
            }
            let low = lower_bound1_3(&rkmers_plus_full_20, &x) as usize;
            for j in low..rkmers_plus_full_20.len() {
                if rkmers_plus_full_20[j].0 != x {
                    break;
                }
                let t = rkmers_plus_full_20[j].1 as usize;
                if rtype[t] >= 0 {
                    is_type[z + rtype[t] as usize] = true;
                }
            }
        }
        let mut nt = 0;
        for l in 0..2 * n {
            if is_type[l] {
                nt += 1;
            }
        }
        if nt == 1 {
            for l in 0..2 * n {
                if is_type[l] {
                    count_this[l] += 1;
                }
            }
        }
    }
    // let m = count_this.max();
    let mut m = 0;
    for l in 0..2 * n {
        m = max(m, count_this[l]);
    }
    let mut best = 0;
    for l in 0..2 * n {
        if count_this[l] == m {
            best = l;
        }
    }
    reverse_sort(&mut count_this);
    if count_this[0] > count_this[1] {
        best as i8
    } else {
        -1 as i8
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// ANNOTATE SEQUENCES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given a DnaString, enumerate matches to reference sequences.  Matches are
// defined to be gap-free alignments seeded on 12-mer matches, with mismatches
// allowed in the following cases:
//
// 1. Given two successive maximal perfect matches of length >= 12 having the same
//    offset, mismatches are allowed between them, so long as the error rate for
//    the extended match is at most 10%.
//
// 2. We always allow extension over a single mismatch so long as 5 perfectly
//    matching bases follow.
//
// However, we require a 20-mer match except for J regions.
// (see below for details)
//
// The structure of the output is:
// { ( start on sequence, match length, ref tig, start on ref tig, mismatches ) }.

pub fn annotate_seq(
    b: &DnaString,
    refdata: &RefData,
    ann: &mut Vec<(i32, i32, i32, i32, i32)>,
    allow_weak: bool,
    allow_improper: bool,
    abut: bool,
) {
    let mut log = Vec::<u8>::new();
    annotate_seq_core(
        b,
        refdata,
        ann,
        allow_weak,
        allow_improper,
        abut,
        &mut log,
        false,
    );
}

fn print_alignx(log: &mut Vec<u8>, a: &(i32, i32, i32, i32, Vec<i32>), refdata: &RefData) {
    let t = a.2 as usize;
    let l = a.0;
    let len = a.1;
    let p = a.3;
    let mis = a.4.len();
    fwriteln!(
        log,
        "{}-{} ==> {}-{} on {} (mis={})",
        l,
        l + len,
        p,
        p + len,
        refdata.rheaders[t],
        mis
    );
}

pub fn annotate_seq_core(
    b: &DnaString,
    refdata: &RefData,
    ann: &mut Vec<(i32, i32, i32, i32, i32)>,
    allow_weak: bool,
    allow_improper: bool,
    abut: bool,
    log: &mut Vec<u8>,
    verbose: bool,
) {
    // Unpack refdata.

    let refs = &refdata.refs;
    let rheaders = &refdata.rheaders;
    let rkmers_plus = &refdata.rkmers_plus;

    // Heuristic constants.

    const K: usize = 12;
    const MIN_PERF_EXT: usize = 5;
    const MAX_RATE: f64 = 0.15;

    // Find maximal perfect matches of length >= 20, or 12 for J regions, so long
    // as we have extension to a 20-mer with only one mismatch.

    let mut perf = Vec::<(i32, i32, i32, i32)>::new();
    if b.len() < K {
        return;
    }
    for l in 0..(b.len() - K + 1) as usize {
        let x: Kmer12 = b.get_kmer(l);
        let low = lower_bound1_3(&rkmers_plus, &x) as usize;
        for r in low..rkmers_plus.len() {
            if rkmers_plus[r].0 != x {
                break;
            }
            let t = rkmers_plus[r as usize].1 as usize;
            let mut p = rkmers_plus[r as usize].2 as usize;
            if l > 0 && p > 0 && b.get(l - 1) == refs[t].get(p - 1) {
                continue;
            }
            let mut len = K;
            while l + len < b.len() && p + len < refs[t].len() {
                if b.get(l + len) != refs[t].get(p + len) {
                    break;
                }
                len += 1;
            }
            let mut ok = len >= 20;
            if !ok && allow_weak {
                let mut ext1 = len + 1;
                let mut lx = l as i32 - 2;
                let mut px = p as i32 - 2;
                while lx >= 0 && px >= 0 {
                    if b.get(lx as usize) != refs[t].get(px as usize) {
                        break;
                    }
                    ext1 += 1;
                    lx -= 1;
                    px -= 1;
                }
                let mut ext2 = len + 1;
                let mut lx = l + len + 1;
                let mut px = p + len + 1;
                while lx < b.len() && px < refs[t].len() {
                    if b.get(lx) != refs[t].get(px) {
                        break;
                    }
                    ext2 += 1;
                    lx += 1;
                    px += 1;
                }
                if ext1 >= 20 || ext2 >= 20 {
                    ok = true;
                }
            }
            if ok {
                perf.push((t as i32, p as i32 - l as i32, l as i32, len as i32));
            }
        }
    }
    perf.sort();

    // Merge perfect matches.  We track the positions on b of mismatches.
    // semi = {(t, off, pos on b, len, positions on b of mismatches)}
    // where off = pos on ref - pos on b

    let mut semi = Vec::<(i32, i32, i32, i32, Vec<i32>)>::new();
    let mut i = 0;
    while i < perf.len() {
        let j = next_diff12_4(&perf, i as i32);
        let (t, off) = (perf[i].0, perf[i].1);
        let mut join = vec![false; j as usize - i];
        let mut mis = Vec::<Vec<i32>>::new();
        for _k in i..j as usize {
            mis.push(Vec::<i32>::new());
        }
        for k in i..j as usize - 1 {
            let (l1, len1) = (perf[k].2, perf[k].3);
            let (l2, len2) = (perf[k + 1].2, perf[k + 1].3);
            for z in l1 + len1..l2 {
                if b.get(z as usize) != refs[t as usize].get((z + off) as usize) {
                    mis[k - i].push(z);
                }
            }

            // XXX:
            // println!( "\ntrying merge" );
            // printme!( t, l1, l2, len1, len2, mis[k-i].len() );

            if mis[k - i].len() as f64 / (l2 + len2 - l1) as f64 <= MAX_RATE {
                join[k - i] = true;
            }
        }
        let mut k1 = i;
        while k1 < j as usize {
            // let mut k2 = k1 + 1;
            let mut k2 = k1;
            let mut m = Vec::<i32>::new();
            // m.append( &mut mis[k1-i].clone() );
            while k2 < j as usize {
                // if !join[k2-i-1] { break; }
                if !join[k2 - i] {
                    break;
                }
                m.append(&mut mis[k2 - i].clone());
                k2 += 1;
            }
            semi.push((t, off, perf[k1].2, perf[k2].2 + perf[k2].3 - perf[k1].2, m));
            k1 = k2 + 1;
        }
        i = j as usize;
    }

    // Extend backwards and then forwards.

    for i in 0..semi.len() {
        let t = semi[i].0;
        let off = semi[i].1;
        let mut l = semi[i].2;
        let mut len = semi[i].3;
        let mut mis = semi[i].4.clone();
        while l > MIN_PERF_EXT as i32 && l + off > MIN_PERF_EXT as i32 {
            let mut ok = true;
            for j in 0..MIN_PERF_EXT {
                if b.get((l - j as i32 - 2) as usize)
                    != refs[t as usize].get((l + off - j as i32 - 2) as usize)
                {
                    ok = false;
                }
            }
            if !ok {
                break;
            }
            mis.push(l - 1);
            l -= MIN_PERF_EXT as i32 + 1;
            len += MIN_PERF_EXT as i32 + 1;
            while l > 0 && l + off > 0 {
                if b.get(l as usize - 1) != refs[t as usize].get((l + off - 1) as usize) {
                    break;
                }
                l -= 1;
                len += 1;
            }
        }
        while l + len < (b.len() - MIN_PERF_EXT) as i32
            && l + len + off < (refs[t as usize].len() - MIN_PERF_EXT) as i32
        {
            let mut ok = true;
            for j in 0..MIN_PERF_EXT {
                if b.get((l + len + j as i32 + 1) as usize)
                    != refs[t as usize].get((l + off + len + j as i32 + 1) as usize)
                {
                    ok = false;
                }
            }
            if !ok {
                break;
            }
            mis.push(l + len);
            len += MIN_PERF_EXT as i32 + 1;
            while l + len < b.len() as i32 && l + off + len < refs[t as usize].len() as i32 {
                if b.get((l + len) as usize) != refs[t as usize].get((l + off + len) as usize) {
                    break;
                }
                len += 1;
            }
        }
        semi[i].2 = l;
        semi[i].3 = len;
        semi[i].4 = mis;
    }
    for i in 0..semi.len() {
        semi[i].4.sort();
    }

    // Add some 40-mers with the same offset having <= 6 mismatches.
    // semi = {(t, off, pos on b, len, positions on b of mismatches)}
    // where off = pos on ref - pos on b
    //
    // Note that implementation is asymmetric: we don't look to the right of p2, not for
    // any particularly good reason.
    // 
    // This was added to get the heavy chain V segment of the mouse A20 cell line to be annotated.
    // This is dubious because the cell line is ~30 years old and of uncertain ancestry.  Thus
    // we're not sure if it arose from supported mouse strains or if the V segment might have
    // been corrupted during the growth of the cell line.  The A20 heavy chain V differs by 20%
    // from the reference.

    let mut i = 0;
    while i < semi.len() {
        let mut j = i + 1;
        let t = semi[i].0;
        let off = semi[i].1;
        while j < semi.len() {
            if semi[j].0 != t || semi[j].1 != off {
                break;
            }
            j += 1;
        }
        const L : i32 = 40;
        const MAX_DIFFS : usize = 6;
        let p1 = off + semi[i].2;
        // let p2 = off + semi[j-1].2 + semi[j-1].3;
        if -off >= 0 && p1 - off <= b.len() as i32 {
            for p in 0..p1-L {
                let l = p - off;
                let mut diffs = 0;
                for m in 0..L {
                    if b.get((l+m) as usize) != refs[t as usize].get((p+m) as usize) {
                        diffs += 1;
                        if diffs > MAX_DIFFS {
                            break;
                        }
                    }
                }
                if diffs <= MAX_DIFFS {
                    let mut x = Vec::<i32>::new();
                    for m in 0..L {
                        if b.get((l+m) as usize) != refs[t as usize].get((p+m) as usize) {
                            x.push(l+m);
                        }
                    }
                    semi.push( ( t, off, p - off, L, x ) );
                    break;
                }
            }
        }
        i = j;
    }
    semi.sort();

    // Allow extension over some mismatches on right if it gets us to the end on
    // the reference.  Ditto for left.
    // ◼ Not documented above.

    if allow_weak {
        let max_mis = 5;
        for i in 0..semi.len() {
            let t = semi[i].0;
            let off = semi[i].1;
            let mut l = semi[i].2;
            let mut len = semi[i].3;
            let mut mis = semi[i].4.clone();
            let mut mis_count = 0;
            while l + len < b.len() as i32 && l + len + off < refs[t as usize].len() as i32 {
                if b.get((l + len as i32) as usize)
                    != refs[t as usize].get((l + off + len as i32) as usize)
                {
                    mis.push(l + len);
                    mis_count += 1;
                }
                len += 1;
            }
            if mis_count <= max_mis && l + len + off == refs[t as usize].len() as i32 {
                semi[i].3 = len;
                semi[i].4 = mis;
            }
        }
        for i in 0..semi.len() {
            let t = semi[i].0;
            let off = semi[i].1;
            let mut l = semi[i].2;
            let mut len = semi[i].3;
            let mut mis = semi[i].4.clone();
            let mut mis_count = 0;
            while l > 0 && l + off > 0 {
                if b.get((l - 1 as i32) as usize)
                    != refs[t as usize].get((l + off - 1 as i32) as usize)
                {
                    mis.push(l - 1);
                    mis_count += 1;
                }
                l -= 1;
                len += 1;
            }
            if mis_count <= max_mis && l + off == 0 {
                semi[i].2 = l;
                semi[i].3 = len;
                semi[i].4 = mis;
            }
        }
        for i in 0..semi.len() {
            semi[i].4.sort();
        }
    }

    // Extend between match blocks.
    // ◼ This is pretty crappy.  What we should do instead is arrange the initial
    // ◼ extension between match blocks so it can be iterated.

    let mut to_delete = vec![false; semi.len()];
    for i1 in 0..semi.len() {
        let t1 = semi[i1].0;
        if t1 < 0 {
            continue;
        }
        let off1 = semi[i1].1;
        let (mut l1, mut len1) = (semi[i1].2, semi[i1].3);
        let mut mis1 = semi[i1].4.clone();
        for i2 in 0..semi.len() {
            let t2 = semi[i2].0;
            let off2 = semi[i2].1;
            if t2 != t1 || off2 != off1 {
                continue;
            }
            let (mut l2, mut len2) = (semi[i2].2, semi[i2].3);
            if l1 + len1 >= l2 {
                continue;
            }
            let mut mis2 = semi[i2].4.clone();
            let mut mis3 = Vec::<i32>::new();
            for l in l1 + len1..l2 {
                if b.get(l as usize) != refs[t1 as usize].get((l + off1) as usize) {
                    mis3.push(l);
                }
            }
            let nmis = mis1.len() + mis2.len() + mis3.len();
            if nmis as f64 / ((l2 + len2) - l1) as f64 > MAX_RATE {
                continue;
            }
            semi[i1].3 = (l2 + len2) - l1;
            semi[i1].4.append(&mut mis3);
            semi[i1].4.append(&mut mis2.clone());
            semi[i2].0 = -1 as i32;
            to_delete[i2] = true;
        }
    }
    erase_if(&mut semi, &to_delete);

    // Transform to create annx, having structure:
    // { ( sequence start, match length, ref tig, ref tig start, {mismatches} ) }.

    let mut annx = Vec::<(i32, i32, i32, i32, Vec<i32>)>::new();
    for x in semi.iter() {
        annx.push((x.2, x.3, x.0, x.2 + x.1, x.4.clone()));
    }
    unique_sort(&mut annx);

    // Delete matches that are 'too improper'.

    if !allow_improper {
        let mut to_delete: Vec<bool> = vec![false; annx.len()];
        for i in 0..annx.len() {
            let tmp = annx[i].0;
            annx[i].0 = annx[i].2;
            annx[i].2 = tmp;
            let tmp = annx[i].1;
            annx[i].1 = annx[i].3;
            annx[i].3 = tmp;
        }
        annx.sort();
        let mut i1 = 0;
        loop {
            if i1 == annx.len() {
                break;
            }
            let j1 = next_diff1_5(&annx, i1 as i32);
            let mut min_imp = 1000000000;
            for k in i1..j1 as usize {
                let imp = min(annx[k].1, annx[k].2);
                min_imp = min(imp, min_imp);
            }
            const MAX_IMP: i32 = 60;
            if min_imp > MAX_IMP {
                for k in i1..j1 as usize {
                    to_delete[k] = true;
                }
            }
            i1 = j1 as usize;
        }
        erase_if(&mut annx, &to_delete);
        for i in 0..annx.len() {
            let tmp = annx[i].0;
            annx[i].0 = annx[i].2;
            annx[i].2 = tmp;
            let tmp = annx[i].1;
            annx[i].1 = annx[i].3;
            annx[i].3 = tmp;
        }
        annx.sort();
    }

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nINITIAL ALIGNMENTS\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // Amongst V segments starting at zero on the V segment, if some start with
    // a start codon, delete the others.

    let mut have_starter = false;
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if !rheaders[t].contains("segment") && refdata.is_v(t) && annx[i].3 == 0 {
            let p = annx[i].0 as usize;
            if b.get(p) == 0 // A
                && b.get(p+1) == 3 // T
                && b.get(p+2) == 2
            {
                // G
                have_starter = true;
            }
        }
    }
    if have_starter {
        let mut to_delete: Vec<bool> = vec![false; annx.len()];
        for i in 0..annx.len() {
            let t = annx[i].2 as usize;
            if !rheaders[t].contains("segment") && refdata.is_v(t) && annx[i].3 == 0 {
                let p = annx[i].0 as usize;
                if !(b.get(p) == 0 && b.get(p + 1) == 3 && b.get(p + 2) == 2) {
                    to_delete[i] = true;
                }
            }
        }
        erase_if(&mut annx, &to_delete);
    }

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS ONE\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // Remove inferior matches of the edge.  Two alignments are compared if the
    // length of their overlap on the contig is at least 85% of one of the alignment
    // lengths len1 and len2.  We compute the mismatch rates r1 and r2 between the
    // overlap interval and the respective references.  The first alignment wins if
    // at least one of the following happens:
    // 1. len1  > len2 and r1 <= r2
    // 2. len1 >= len2 and r2 <  r2
    // 3. len1 >= 1.5 * len2.
    //
    // Modified: multiple aligns of the same V segment are now group together in
    // the calculation.   And add indel penalty.
    //
    // ◼ For efficiency, inner loop should check to see if already deleted.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    let mut ts = Vec::<(usize, usize)>::new(); // { ( contig index, annx index ) }
    for i in 0..annx.len() {
        ts.push((annx[i].2 as usize, i));
    }
    ts.sort();
    let mut i1 = 0;
    while i1 < ts.len() {
        let j1 = next_diff1_2(&ts, i1 as i32) as usize;
        let mut tlen1 = 0;
        for k in i1..j1 {
            tlen1 += annx[ts[k].1].1;
        }
        let mut i2 = 0;
        while i2 < ts.len() {
            let j2 = next_diff1_2(&ts, i2 as i32) as usize;
            let mut tlen2 = 0;
            for k in i2..j2 {
                tlen2 += annx[ts[k].1].1;
            }
            let (mut m1, mut m2) = (0, 0);
            let mut over = 0;
            let mut offsets1 = Vec::<i32>::new();
            let mut offsets2 = Vec::<i32>::new();
            for k1 in i1..j1 {
                let u1 = ts[k1].1;
                offsets1.push(annx[u1].0 - annx[u1].3);
            }
            for k2 in i2..j2 {
                let u2 = ts[k2].1;
                offsets2.push(annx[u2].0 - annx[u2].3);
            }
            offsets1.sort();
            offsets2.sort();
            m1 += offsets1[offsets1.len() - 1] - offsets1[0];
            m2 += offsets2[offsets2.len() - 1] - offsets2[0];
            for k1 in i1..j1 {
                let u1 = ts[k1].1;
                let l1 = annx[u1].0;
                let len1 = annx[u1].1;
                for k2 in i2..j2 {
                    let u2 = ts[k2].1;
                    let l2 = annx[u2].0;
                    let len2 = annx[u2].1;
                    let start = max(l1, l2);
                    let stop = min(l1 + len1, l2 + len2);
                    if !(start < stop) {
                        continue;
                    }
                    over += stop - start;
                    for x in annx[u1].4.iter() {
                        if *x >= start && *x < stop {
                            m1 += 1;
                        }
                    }
                    for x in annx[u2].4.iter() {
                        if *x >= start && *x < stop {
                            m2 += 1;
                        }
                    }
                }
            }

            // Get mismatch rates.

            let (r1, r2) = (m1 as f64 / tlen1 as f64, m2 as f64 / tlen2 as f64);

            // Require that one of the intervals is at least 85% overlapped.

            const MIN_OVERLAP_FRAC: f64 = 0.85;
            if over as f64 / (min(tlen1, tlen2) as f64) >= MIN_OVERLAP_FRAC {
                // Decide if the second match is inferior.

                if (tlen1 > tlen2 && r1 <= r2)
                    || (tlen1 >= tlen2 && r1 < r2)
                    || tlen1 as f64 >= 1.5 * tlen2 as f64
                {
                    if verbose {
                        fwriteln!(
                            log,
                            "\nsee tlen1 = {}, tlen2 = {}, m1 = {}, m2 = {}, \
                             r1 = {:.3}, r2 = {:.3}\nthis alignment",
                            tlen1,
                            tlen2,
                            m1,
                            m2,
                            r1,
                            r2
                        );
                        for k in i1..j1 {
                            let t = ts[k].0;
                            let u = ts[k].1;
                            let l = annx[u].0;
                            let len = annx[u].1;
                            let p = annx[u].3;
                            let mis = annx[u].4.len();
                            fwriteln!(
                                log,
                                "{}-{} ==> {}-{} on {}(mis={})",
                                l,
                                l + len,
                                p,
                                p + len,
                                rheaders[t],
                                mis
                            );
                        }
                        fwriteln!(log, "beats this alignment");
                        for k in i2..j2 {
                            let t = ts[k].0;
                            let u = ts[k].1;
                            let l = annx[u].0;
                            let len = annx[u].1;
                            let p = annx[u].3;
                            let mis = annx[u].4.len();
                            fwriteln!(
                                log,
                                "{}-{} ==> {}-{} on {}(mis={})",
                                l,
                                l + len,
                                p,
                                p + len,
                                rheaders[t],
                                mis
                            );
                        }
                    }
                    for k in i2..j2 {
                        to_delete[ts[k].1] = true;
                    }
                }
            }
            i2 = j2;
        }
        i1 = j1;
    }
    erase_if(&mut annx, &to_delete);

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS TWO\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // If there are two alignments to a particular V region, or a UTR, try to edit
    // them so that their start/stop position abut perfect on one side (either the
    // contig or the reference), and do not overlap on the other side, thus
    // exhibiting an indel.
    // ◼ The approach to answering this seems very inefficient.
    // ◼ When this was moved here, some UTR alignments disappeared.

    if abut {
        let mut to_delete: Vec<bool> = vec![false; annx.len()];
        for i1 in 0..annx.len() {
            let t1 = annx[i1].2 as usize;
            if rheaders[t1].contains("segment") {
                continue;
            }
            if !refdata.is_u(t1) && !refdata.is_v(t1) {
                continue;
            }
            for i2 in 0..annx.len() {
                if i2 == i1 || annx[i2].2 as usize != t1 {
                    continue;
                }
                let t2 = annx[i2].2 as usize;
                let (l1, mut l2) = (annx[i1].0 as usize, annx[i2].0 as usize);
                if l1 >= l2 {
                    continue;
                }
                let (mut len1, mut len2) = (annx[i1].1 as usize, annx[i2].1 as usize);
                if l1 + len1 > l2 + len2 {
                    continue;
                }
                let (p1, mut p2) = (annx[i1].3 as usize, annx[i2].3 as usize);
                let (start1, stop1) = (l1 as usize, (l2 + len2) as usize);
                let (start2, stop2) = (p1 as usize, (p2 + len2) as usize);
                if !(start1 < stop1 && start2 < stop2) {
                    continue;
                }
                let b1 = b.slice(start1, stop1).to_owned();
                let b2 = refs[t1].slice(start2, stop2).to_owned();
                let a = affine_align(&b1, &b2);
                let mut del = Vec::<(usize, usize, usize)>::new();
                let mut ins = Vec::<(usize, usize, usize)>::new();
                let ops = &a.operations;
                let mut i = 0;
                let (mut z1, mut z2) = (l1 + a.xstart, p1 + a.ystart);
                if a.ystart > 0 {
                    continue;
                }
                let mut matches = 0;
                while i < ops.len() {
                    let mut opcount = 1;
                    while i + opcount < ops.len()
                        && (ops[i] == Del || ops[i] == Ins)
                        && ops[i] == ops[i + opcount]
                    {
                        opcount += 1;
                    }
                    match ops[i] {
                        Match => {
                            matches += 1;
                            z1 += 1;
                            z2 += 1;
                        }
                        Subst => {
                            z1 += 1;
                            z2 += 1;
                        }
                        Del => {
                            del.push((z1, z2, opcount));
                            if verbose {
                                fwriteln!(log, "\nsee del[{}]", opcount);
                            }
                            z2 += opcount;
                        }
                        Ins => {
                            ins.push((z1, z2, opcount));
                            if verbose {
                                fwriteln!(log, "\nsee ins[{}]", opcount);
                            }
                            z1 += opcount;
                        }
                        Xclip(d) => {
                            z1 += d;
                        }
                        Yclip(d) => {
                            z2 += d;
                        }
                    }
                    i += opcount;
                }
                if verbose {
                    fwriteln!(log, "\ntrying to merge\n{}\n{}", rheaders[t1], rheaders[t2]);
                    fwriteln!(log, "|del| = {}, |ins| = {}", del.len(), ins.len());
                }
                if del.solo() && ins.len() == 0 {
                    let (l, p, n) = (del[0].0, del[0].1, del[0].2);
                    if n != (p2 + len2 - p1) - (l2 + len2 - l1) {
                        continue;
                    }
                    len1 = l - l1;
                    if len1 >= matches {
                        continue;
                    }
                    len2 = l2 + len2 - l1 - len1;
                    l2 = l;
                    p2 = p + n;
                }
                if del.len() == 0 && ins.solo() {
                    let (l, p, n) = (ins[0].0, ins[0].1, ins[0].2);
                    if n != (p1 + len1 - p2) - (l1 + len1 - l2) {
                        continue;
                    }
                    len1 = l - l1;
                    if len1 >= matches {
                        continue;
                    }
                    len2 = p2 + len2 - p1 - len1;
                    l2 = l + n;
                    p2 = p;
                }
                if del.len() + ins.len() == 0 {
                    to_delete[i2] = true;
                    len1 = (annx[i2].0 + annx[i2].1 - annx[i1].0) as usize;
                    annx[i1].1 = len1 as i32;
                    annx[i1].4.truncate(0);
                    for j in 0..len1 {
                        if b.get(l1 + j) != refs[t1].get(p1 + j) {
                            annx[i1].4.push((l1 + j) as i32);
                        }
                    }
                }
                if del.len() + ins.len() == 1 {
                    annx[i2].0 = l2 as i32;
                    annx[i1].1 = len1 as i32;
                    annx[i2].1 = len2 as i32;
                    annx[i2].3 = p2 as i32;
                    annx[i1].4.truncate(0);
                    annx[i2].4.truncate(0);
                    for j in 0..len1 {
                        if b.get(l1 + j) != refs[t1].get(p1 + j) {
                            annx[i1].4.push((l1 + j) as i32);
                        }
                    }
                    for j in 0..len2 {
                        if b.get(l2 + j) != refs[t1].get(p2 + j) {
                            annx[i2].4.push((l2 + j) as i32);
                        }
                    }
                }
            }
        }
        erase_if(&mut annx, &to_delete);
    }

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS THREE\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // Choose between segments if one clearly wins.  For this calculation, we
    // put UTR and V segments together.  The way the choice is made could be refined.
    //
    // ◼ At least in some cases, a better way of comparing errors would be to first
    // ◼ extend the alignments so that their endpoints on the contigs agree, to the
    // ◼ extent that this is possible.
    //
    // ◼ This code has not really been adequately tested to see if the right
    // ◼ choices are being made.
    //
    // ◼ Note danger with nonstandard references.
    //
    // ◼ Really should have ho_interval here.

    let mut combo = Vec::<(String, i32, usize)>::new();
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if !rheaders[t].contains("segment") {
            combo.push((
                refdata.name[t].clone() + "." + &refdata.transcript[t],
                refdata.id[t],
                i,
            ));
        }
    }
    combo.sort();
    //                     cov                 mis    locs        rstarts
    let mut data = Vec::<(Vec<(usize, usize)>, usize, Vec<usize>, Vec<usize>)>::new();
    let mut i = 0;
    while i < combo.len() {
        let j = next_diff1_3(&combo, i as i32) as usize;
        let mut cov = Vec::<(usize, usize)>::new();
        let mut mis = 0;
        let mut locs = Vec::<usize>::new();
        let mut rstarts = Vec::<usize>::new();
        for k in i..j {
            locs.push(combo[k].2 as usize);
            let a = &annx[combo[k].2];
            rstarts.push(a.3 as usize);
            cov.push((a.0 as usize, (a.0 + a.1) as usize));
            mis += a.4.len();
        }
        data.push((cov, mis, locs, rstarts));
        i = j;
    }
    let mut to_delete = vec![false; annx.len()];
    let mut deleted = vec![false; data.len()];
    for i1 in 0..data.len() {
        if deleted[i1] {
            continue;
        }
        for i2 in 0..data.len() {
            if i2 == i1 {
                continue;
            }
            let t1 = annx[data[i1].2[0]].2 as usize;
            let t2 = annx[data[i2].2[0]].2 as usize;
            let mut same_class = false;
            if refdata.segtype[t1] == refdata.segtype[t2] {
                same_class = true;
            } else if refdata.is_v(t1) && refdata.is_u(t2) {
                same_class = true;
            } else if refdata.is_u(t1) && refdata.is_v(t2) {
                same_class = true;
            }
            if !same_class {
                continue;
            }

            // Find mismatch positions.

            let n = b.len();
            let (mut mis1, mut mis2) = (vec![false; n], vec![false; n]);
            for j in data[i1].2.iter() {
                for p in annx[*j].4.iter() {
                    mis1[*p as usize] = true;
                }
            }
            for j in data[i2].2.iter() {
                for p in annx[*j].4.iter() {
                    mis2[*p as usize] = true;
                }
            }

            // Compute the fraction of i2 coverage that's outside i1 coverage.
            // ◼ This is horrendously inefficient.  Use ho intervals.

            let name1 = &refdata.name[t1];
            let name2 = &refdata.name[t2];
            let (mut utr1, mut utr2) = (false, false);
            if refdata.is_v(t1) || refdata.is_u(t1) {
                utr1 = refdata.has_utr[name1];
                utr2 = refdata.has_utr[name2];
            }
            let (mut cov1, mut cov2) = (vec![false; n], vec![false; n]);
            for j in 0..data[i1].0.len() {
                let t = annx[data[i1].2[j]].2;
                if utr2 || !refdata.is_u(t as usize) {
                    let x = &data[i1].0[j];
                    for m in x.0..x.1 {
                        cov1[m] = true;
                    }
                }
            }
            for j in 0..data[i2].0.len() {
                let t = annx[data[i2].2[j]].2;
                if utr1 || !refdata.is_u(t as usize) {
                    let x = &data[i2].0[j];
                    for m in x.0..x.1 {
                        cov2[m] = true;
                    }
                }
            }
            let (mut total1, mut total2) = (0, 0);
            for l in 0..n {
                if cov1[l] {
                    total1 += 1;
                }
            }
            for l in 0..n {
                if cov2[l] {
                    total2 += 1;
                }
            }
            let mut share = 0;
            for l in 0..n {
                if cov1[l] && cov2[l] {
                    share += 1;
                }
            }
            let outside1 = percent_ratio(total1 - share, total1);
            let outside2 = percent_ratio(total2 - share, total2);

            // Find the number of mismatches in the overlap region.

            let (mut m1, mut m2) = (0, 0);
            for l in 0..n {
                if cov1[l] && cov2[l] {
                    if mis1[l] {
                        m1 += 1;
                    }
                    if mis2[l] {
                        m2 += 1;
                    }
                }
            }

            // Compute error rates.
            // ◼ This is incorrect in the case where the UTR has been excluded.

            let err1 = percent_ratio(data[i1].1, total1);
            let err2 = percent_ratio(data[i2].1, total2);

            // Compute zstops.

            let (mut zstop1, mut zstop2) = (0, 0);
            for l in 0..data[i1].2.len() {
                let t = annx[data[i1].2[l] as usize].2 as usize;
                if refdata.is_v(t) {
                    if data[i1].3[l] == 0 || data[i1].0[l].0 == 0 {
                        zstop1 = max(zstop1, data[i1].0[l].1);
                    }
                }
            }
            for l in 0..data[i2].2.len() {
                let t = annx[data[i2].2[l] as usize].2 as usize;
                if refdata.is_v(t) {
                    if data[i2].3[l] == 0 || data[i2].0[l].0 == 0 {
                        zstop2 = max(zstop2, data[i2].0[l].1);
                    }
                }
            }

            // Decide if the first wins.  And symmetrize to prevent double
            // deletion.  Be very careful to respect this if editing!

            let (mut win1, mut win2) = (false, false);
            if zstop1 > zstop2 + 20 && (outside2 <= 10.0 || total2 - share <= 10) {
                win1 = true;
            } else if outside1 >= 10.0 && outside2 <= 1.0 && err1 - err2 <= 2.5 {
                win1 = true;
            } else if zstop1 == 0 && zstop2 > 0 {
            } else if outside2 <= 10.0 || total2 - share <= 10 {
                if m1 < m2
                    || (m1 == m2 && err1 < err2)
                    || (m1 == m2 && err1 == err2 && outside1 > outside2)
                    || (m1 == m2 && err1 == err2 && outside1 == outside2 && t1 < t2)
                {
                    win1 = true;
                }
            }

            // Symmetrization.

            if zstop2 > zstop1 + 20 && (outside1 <= 10.0 || total1 - share <= 10) {
                win2 = true;
            } else if outside2 >= 10.0 && outside1 <= 1.0 && err2 - err1 <= 2.5 {
                win2 = true;
            } else if zstop2 == 0 && zstop1 > 0 {
            } else if outside1 <= 10.0 || total1 - share <= 10 {
                if m2 < m1
                    || (m2 == m1 && err2 < err1)
                    || (m2 == m1 && err2 == err1 && outside2 > outside1)
                    || (m2 == m1 && err2 == err1 && outside2 == outside1 && t2 < t1)
                {
                    win2 = true;
                }
            }
            if win2 {
                win1 = false;
            }

            // Verbose logging.

            if verbose {
                fwriteln!(log, "\nCOMPARING");
                for l in 0..data[i1].2.len() {
                    let t = annx[data[i1].2[l] as usize].2 as usize;
                    let cov = &data[i1].0[l];
                    let mis = data[i1].1;
                    fwriteln!(
                        log,
                        "{}, cov = {}-{}, mis = {}",
                        rheaders[t],
                        cov.0,
                        cov.1,
                        mis
                    );
                }
                fwriteln!(log, "TO");
                for l in 0..data[i2].2.len() {
                    let t = annx[data[i2].2[l] as usize].2 as usize;
                    let cov = &data[i2].0[l];
                    let mis = data[i2].1;
                    fwriteln!(
                        log,
                        "{}, cov = {}-{}, mis = {}",
                        rheaders[t],
                        cov.0,
                        cov.1,
                        mis
                    );
                }
                fwriteln!(log, "zstop1 = {}, zstop2 = {}", zstop1, zstop2);
                fwriteln!(log, "m1 = {}, m2 = {}", m1, m2);
                fwriteln!(log, "err1 = {}, err2 = {}", err1, err2);
                fwriteln!(
                    log,
                    "total1 = {}, total2 = {}, share = {}",
                    total1,
                    total2,
                    share
                );
                fwriteln!(
                    log,
                    "outside1 = {:.1}%, outside2 = {:.1}%, \
                     total2 - share = {}, err1 = {:.1}%, err2 = {:.1}%",
                    outside1,
                    outside2,
                    total2 - share,
                    err1,
                    err2
                );
                fwriteln!(log, "win1 = {}, win2 = {}", win1, win2);
            }

            // Pick "randomly" in case of tie.

            if outside1 == 0.0
                && outside2 == 0.0
                && zstop1 == zstop2
                && m1 == m2
                && err1 == err2
                && t1 < t2
            {
                win1 = true;
            }

            // Make decision.

            if win1 {
                for l in data[i2].2.iter() {
                    to_delete[*l] = true;
                }
                deleted[i2] = true;
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS FOUR\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // If two V segments are aligned starting at 0 on the reference and one
    // is aligned a lot further, it wins.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        for i2 in 0..annx.len() {
            let (t1, t2) = (annx[i1].2 as usize, annx[i2].2 as usize);
            if rheaders[t1].contains("segment") || rheaders[t2].contains("segment") {
                continue;
            }
            if !refdata.is_v(t1) || !refdata.is_v(t2) {
                continue;
            }
            let (len1, len2) = (annx[i1].1, annx[i2].1);
            let (p1, p2) = (annx[i1].3, annx[i2].3);
            if p1 > 0 {
                continue;
            }
            const MIN_EXT: i32 = 50;
            if (p2 > 0 && len1 >= len2) || (p2 == 0 && len1 >= len2 + MIN_EXT) {
                if verbose {
                    fwriteln!(log, "");
                    print_alignx(log, &annx[i1], &refdata);
                    fwriteln!(log, "beats");
                    print_alignx(log, &annx[i2], &refdata);
                }
                to_delete[i2] = true;
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // For IG, if we have a C segment that aligns starting at zero, and a V segment
    // that aligns, but no J segment, try to find a J segment alignment.  For now we
    // assume that the J aligns up to exactly the point where the C starts, or to
    // one base after.  We require that the last 20 bases of the J match with at
    // most 5 mismatches.

    let (mut igv, mut igj) = (false, false);
    let mut igc = -1 as i32;
    const J_TOT: i32 = 20;
    const J_MIS: i32 = 5;
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if rheaders[t].contains("segment") {
            continue;
        }
        let rt = refdata.rtype[t];
        if rt >= 0 && rt < 3 {
            if refdata.segtype[t] == "V".to_string() {
                igv = true;
            } else if refdata.segtype[t] == "J".to_string() {
                igj = true;
            } else {
                if refdata.segtype[t] == "C".to_string()
                    && annx[i].3 == 0
                    && annx[i].0 >= J_TOT
                    && refs[t].len() >= J_TOT as usize
                {
                    igc = annx[i].0;
                }
            }
        }
    }
    if igc >= 0 && igv && !igj {
        let mut best_t = -1 as i32;
        let mut best_mis = 1000000;
        let mut best_z = -1 as i32;
        for z in 0..2 {
            for l in 0..refdata.igjs.len() {
                let t = refdata.igjs[l];
                let n = refs[t].len();
                if n > igc as usize + z {
                    continue;
                }
                let i = igc as usize + z - n; // start of J on contig
                let (mut total, mut mis) = (0, 0);
                for j in (0..n).rev() {
                    total += 1;
                    if b.get(i + j) != refs[t].get(j) {
                        mis += 1;
                        if total <= J_TOT && mis > J_MIS {
                            break;
                        }
                    }
                }
                if total == n as i32 {
                    if mis < best_mis {
                        best_t = t as i32;
                        best_mis = mis;
                        best_z = z as i32;
                    }
                }
            }
        }
        if best_t >= 0 {
            let t = best_t as usize;
            let n = refs[t].len() as i32;
            let i = igc + best_z - n;
            let mut mis = Vec::<i32>::new();
            for j in 0..n {
                if b.get((i + j) as usize) != refs[t].get(j as usize) {
                    mis.push(i + j);
                }
            }
            annx.push((i, n, best_t, 0 as i32, mis));
            annx.sort();
        }
    }

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS FIVE\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // A J segment that goes up to its end beats any J segment that doesn't.
    // If they both go up to the end, choose.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        for i2 in 0..annx.len() {
            let (t1, t2) = (annx[i1].2 as usize, annx[i2].2 as usize);
            if rheaders[t1].contains("segment") || rheaders[t2].contains("segment") {
                continue;
            }
            if !refdata.is_j(t1) || !refdata.is_j(t2) {
                continue;
            }
            let (len1, len2) = (annx[i1].1, annx[i2].1);
            let (l1, l2) = (annx[i1].0, annx[i2].0);
            let (p1, p2) = (annx[i1].3, annx[i2].3);
            if len1 + p1 == refs[t1].len() as i32 && len2 + p2 < refs[t2].len() as i32 {
                to_delete[i2] = true;
            }
            if len1 + p1 == refs[t1].len() as i32 && len2 + p2 == refs[t2].len() as i32 {
                let (mut mis1, mut mis2) = (0, 0);
                let mut y1 = refs[t1].len() as i32 - 1;
                let mut y2 = refs[t2].len() as i32 - 1;
                let (mut x1, mut x2) = (y1 + l1 - p1, y2 + l2 - p2);
                loop {
                    if b.get(x1 as usize) != refs[t1].get(y1 as usize) {
                        mis1 += 1;
                    }
                    if b.get(x2 as usize) != refs[t2].get(y2 as usize) {
                        mis2 += 1;
                    }
                    if x1 == 0 || y1 == 0 || x2 == 0 || y2 == 0 {
                        break;
                    }
                    x1 -= 1;
                    y1 -= 1;
                    x2 -= 1;
                    y2 -= 1;
                }
                if mis1 < mis2 || (mis1 == mis2 && t1 < t2) {
                    to_delete[i2] = true;
                }
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Pick between C segments starting at zero.  And favor zero.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        for i2 in 0..annx.len() {
            if i2 == i1 {
                continue;
            }
            let (t1, t2) = (annx[i1].2 as usize, annx[i2].2 as usize);
            if rheaders[t1].contains("segment") || rheaders[t2].contains("segment") {
                continue;
            }
            if !refdata.is_c(t1) || !refdata.is_c(t2) {
                continue;
            }
            let (l1, l2) = (annx[i1].0, annx[i2].0);
            let (p1, p2) = (annx[i1].3, annx[i2].3);
            if p1 > 0 {
                continue;
            }
            if p1 == 0 && p2 > 0 {
                to_delete[i2] = true;
            }
            let (mut mis1, mut mis2) = (0, 0);
            let (mut y1, mut y2) = (p1, p2);
            let (mut x1, mut x2) = (l1, l2);
            loop {
                if b.get(x1 as usize) != refs[t1].get(y1 as usize) {
                    mis1 += 1;
                }
                if b.get(x2 as usize) != refs[t2].get(y2 as usize) {
                    mis2 += 1;
                }
                x1 += 1;
                y1 += 1;
                x2 += 1;
                y2 += 1;
                if x1 == b.len() as i32 || y1 == refs[t1].len() as i32 {
                    break;
                }
                if x2 == b.len() as i32 || y2 == refs[t2].len() as i32 {
                    break;
                }
            }
            if mis1 < mis2 || (mis1 == mis2 && t1 < t2) {
                to_delete[i2] = true;
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Pick between V segments starting at zero.  And favor zero.

    let mut nv = 0;
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if rheaders[t].contains("segment") {
            continue;
        }
        if refdata.is_v(t) {
            nv += 1;
        }
    }
    if nv == 2 {
        let mut to_delete: Vec<bool> = vec![false; annx.len()];
        for i1 in 0..annx.len() {
            for i2 in 0..annx.len() {
                let (t1, t2) = (annx[i1].2 as usize, annx[i2].2 as usize);
                if t2 == t1 {
                    continue;
                }
                if rheaders[t1].contains("segment") || rheaders[t2].contains("segment") {
                    continue;
                }
                if !refdata.is_v(t1) || !refdata.is_v(t2) {
                    continue;
                }
                let (l1, l2) = (annx[i1].0, annx[i2].0);
                let (p1, p2) = (annx[i1].3, annx[i2].3);
                if p1 > 0 {
                    continue;
                }
                if p2 > 0 {
                    to_delete[i2] = true;
                }
                let (mut mis1, mut mis2) = (0, 0);
                let (mut y1, mut y2) = (p1, p2);
                let (mut x1, mut x2) = (l1, l2);
                loop {
                    if b.get(x1 as usize) != refs[t1].get(y1 as usize) {
                        mis1 += 1;
                    }
                    if b.get(x2 as usize) != refs[t2].get(y2 as usize) {
                        mis2 += 1;
                    }
                    x1 += 1;
                    y1 += 1;
                    x2 += 1;
                    y2 += 1;
                    if x1 == b.len() as i32 || y1 == refs[t1].len() as i32 {
                        break;
                    }
                    if x2 == b.len() as i32 || y2 == refs[t2].len() as i32 {
                        break;
                    }
                }
                if mis1 < mis2 {
                    to_delete[i2] = true;
                }
            }
        }
        erase_if(&mut annx, &to_delete);
    }

    // Remove UTR annotations that have no matching V annotation.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    let (mut u, mut v) = (Vec::<String>::new(), Vec::<String>::new());
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if !rheaders[t].contains("segment") {
            let name = rheaders[t].after("|").between("|", "|");
            if rheaders[t].contains("UTR") {
                u.push(name.to_string());
            }
            if rheaders[t].contains("V-REGION") {
                v.push(name.to_string());
            }
        }
    }
    v.sort();
    for i in 0..u.len() {
        if !bin_member(&v, &u[i]) {
            for j in 0..annx.len() {
                let t = annx[j].2 as usize;
                if !rheaders[t].contains("segment") {
                    let name = rheaders[t].after("|").between("|", "|");
                    if rheaders[t].contains("UTR") && u[i] == name {
                        to_delete[j] = true;
                    }
                }
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Log alignments.

    if verbose {
        fwriteln!(log, "\nALIGNMENTS SIX\n");
        for i in 0..annx.len() {
            print_alignx(log, &annx[i], &refdata);
        }
    }

    // In light of the previous calculation, see if one V is aligned much better
    // than another V.  This is done by looking for simple indel events.
    // Probably will have to be generalized.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    let mut vs = Vec::<(usize, usize)>::new();
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if !rheaders[t as usize].contains("V-REGION") {
            continue;
        }
        vs.push((t, i));
    }
    vs.sort();
    //                     len parts errs  index
    let mut score = Vec::<(i32, usize, usize, usize)>::new();
    let mut j = 0;
    let mut nonsimple = false;
    let mut have_split = false;
    let max_indel = 27;
    let min_len_gain = 100;
    while j < vs.len() {
        let k = next_diff1_2(&mut vs, j as i32) as usize;
        if k - j == 1 {
            score.push((annx[j].1, k - j, annx[j].4.len(), vs[j].1));
        } else if k - j == 2 {
            let (i1, i2) = (vs[j].1, vs[j + 1].1);
            let (a1, a2) = (&annx[i1], &annx[i2]);
            let mut simple = false;
            let (l1, p1, len1) = (a1.0, a1.3, a1.1);
            let (l2, p2, len2) = (a2.0, a2.3, a2.1);
            if l1 + len1 == l2
                && p1 + len1 < p2
                && (p2 - p1 - len1) % 3 == 0
                && p2 - p1 - len1 <= max_indel
            {
                simple = true;
            }
            if l1 + len1 < l2
                && p1 + len1 == p2
                && (l2 - l1 - len1) % 3 == 0
                && l2 - l1 - len1 <= max_indel
            {
                simple = true;
            }
            if simple {
                have_split = true;
                score.push((len1 + len2, k - j, a1.4.len() + a2.4.len(), vs[j].1));
            } else {
                nonsimple = true;
            }
        } else {
            nonsimple = true;
        }
        j = k;
    }
    if !nonsimple && score.duo() && have_split {
        reverse_sort(&mut score);
        if score[0].0 >= score[1].0 + min_len_gain && score[1].1 == 1 {
            to_delete[score[1].3] = true;
        }
    }
    erase_if(&mut annx, &to_delete);

    // Remove certain subsumed alignments.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        for i2 in 0..annx.len() {
            if i2 == i1 || annx[i1].2 != annx[i2].2 {
                continue;
            }
            let (l1, l2) = (annx[i1].0, annx[i2].0);
            let (len1, len2) = (annx[i1].1, annx[i2].1);
            let (p1, p2) = (annx[i1].3, annx[i2].3);
            if l1 != l2 || p1 != p2 {
                continue;
            }
            if len1 > len2 {
                to_delete[i2] = true;
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // If we see TRBJ1 and not TRBJ2, delete any TRBC2.  And conversely.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    let (mut j1, mut j2) = (false, false);
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if rheaders[t].contains("TRBJ1") {
            j1 = true;
        }
        if rheaders[t].contains("TRBJ2") {
            j2 = true;
        }
    }
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if j1 && !j2 && rheaders[t].contains("TRBC2") {
            to_delete[i] = true;
        }
        if !j1 && j2 && rheaders[t].contains("TRBC1") {
            to_delete[i] = true;
        }
    }
    erase_if(&mut annx, &to_delete);

    // Pick between equally performant Js and likewise for Cs.

    let mut to_delete = vec![false; annx.len()];
    for pass in 0..2 {
        for i1 in 0..annx.len() {
            let t1 = annx[i1].2;
            if pass == 1 {
                if !rheaders[t1 as usize].contains("J-REGION") {
                    continue;
                }
            } else {
                if !rheaders[t1 as usize].contains("C-REGION") {
                    continue;
                }
            }
            for i2 in 0..annx.len() {
                let t2 = annx[i2].2;
                if pass == 1 {
                    if !rheaders[t2 as usize].contains("J-REGION") {
                        continue;
                    }
                } else {
                    if !rheaders[t2 as usize].contains("C-REGION") {
                        continue;
                    }
                }
                let (l1, l2) = (annx[i1].0, annx[i2].0);
                let (len1, len2) = (annx[i1].1, annx[i2].1);
                if l1 != l2 || len1 != len2 {
                    continue;
                }
                let (p1, p2) = (annx[i1].3, annx[i2].3);
                if pass == 1 {
                    if p1 + len1 != refs[t1 as usize].len() as i32 {
                        continue;
                    }
                    if p2 + len2 != refs[t2 as usize].len() as i32 {
                        continue;
                    }
                } else if p1 > 0 || p2 > 0 {
                    continue;
                }
                if annx[i1].4.len() != annx[i2].4.len() {
                    continue;
                }
                if t1 < t2 {
                    to_delete[i2] = true;
                }
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Pick between Cs.

    let mut to_delete = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        let t1 = annx[i1].2;
        if !rheaders[t1 as usize].contains("C-REGION") {
            continue;
        }
        for i2 in 0..annx.len() {
            let t2 = annx[i2].2;
            if !rheaders[t2 as usize].contains("C-REGION") {
                continue;
            }
            let (l1, l2) = (annx[i1].0 as usize, annx[i2].0 as usize);
            let (len1, len2) = (annx[i1].1 as usize, annx[i2].1 as usize);
            // let (p1,p2) = (annx[i1].3,annx[i2].3);
            if l1 + len1 != l2 + len2 {
                continue;
            }
            if l1 + annx[i1].4.len() >= l2 + annx[i2].4.len() {
                continue;
            }
            to_delete[i2] = true;
        }
    }
    erase_if(&mut annx, &to_delete);

    // Again remove UTR annotations that have no matching V annotation.
    // ◼ DANGER with nonstandard references.
    // ◼ Note repetition.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    let (mut u, mut v) = (Vec::<String>::new(), Vec::<String>::new());
    for i in 0..annx.len() {
        let t = annx[i].2 as usize;
        if !rheaders[t].contains("segment") {
            let name = rheaders[t].after("|").between("|", "|");
            if rheaders[t].contains("UTR") {
                u.push(name.to_string());
            }
            if rheaders[t].contains("V-REGION") {
                v.push(name.to_string());
            }
        }
    }
    v.sort();
    for i in 0..u.len() {
        if !bin_member(&v, &u[i]) {
            for j in 0..annx.len() {
                let t = annx[j].2 as usize;
                if !rheaders[t].contains("segment") {
                    let name = rheaders[t].after("|").between("|", "|");
                    if rheaders[t].contains("UTR") && u[i] == name {
                        to_delete[j] = true;
                    }
                }
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Remove some subsumed extended annotations.

    let mut to_delete: Vec<bool> = vec![false; annx.len()];
    for i1 in 0..annx.len() {
        let l1 = annx[i1].0 as usize;
        let len1 = annx[i1].1 as usize;
        for i2 in 0..annx.len() {
            let t2 = annx[i2].2 as usize;
            let l2 = annx[i2].0 as usize;
            let len2 = annx[i2].1 as usize;
            if len2 >= len1 {
                continue;
            }
            if !rheaders[t2].contains("before") && !rheaders[t2].contains("after") {
                continue;
            }
            if l1 <= l2 && l1 + len1 >= l2 + len2 {
                to_delete[i2] = true;
            }
        }
    }
    erase_if(&mut annx, &to_delete);

    // Transform.

    ann.clear();
    for x in annx.iter() {
        ann.push((x.0, x.1, x.2, x.3, x.4.len() as i32));
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRINT ANNOTATIONS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Print annotations, marking any V annotations that are out of frame.

pub fn print_some_annotations(
    refdata: &RefData,
    ann: &Vec<(i32, i32, i32, i32, i32)>,
    log: &mut Vec<u8>,
    verbose: bool,
) {
    let refs = &refdata.refs;
    let rheaders = &refdata.rheaders;
    if verbose {
        fwriteln!(log, "");
    }
    let mut vstart = Vec::<i32>::new();
    for l in 0..ann.len() {
        let estart = ann[l].0;
        let t = ann[l].2 as usize;
        let tstart = ann[l].3;
        if tstart == 0 && (rheaders[t].contains("V-REGION") || rheaders[t].contains("L+V")) {
            vstart.push(estart);
        }
    }
    for l in 0..ann.len() {
        let (estart, len) = (ann[l].0, ann[l].1);
        let t = ann[l].2 as usize;
        let tstart = ann[l].3;
        let mis = ann[l].4;
        fwrite!(
            log,
            "{}-{} ==> {}-{} on {} [len={}] (mis={})",
            estart,
            estart + len,
            tstart,
            tstart + len,
            rheaders[t],
            refs[t].len(),
            mis
        );
        if vstart.solo()
            && (rheaders[t].contains("V-REGION") || rheaders[t].contains("L+V"))
            && (estart - vstart[0] - tstart) % 3 != 0
        {
            fwrite!(log, " [SHIFT!]");
        }
        fwriteln!(log, "");
    }
}

pub fn print_annotations(
    b: &DnaString,
    refdata: &RefData,
    log: &mut Vec<u8>,
    allow_improper: bool,
    abut: bool,
    verbose: bool,
) {
    let mut ann = Vec::<(i32, i32, i32, i32, i32)>::new();
    annotate_seq_core(
        b,
        refdata,
        &mut ann,
        true,
        allow_improper,
        abut,
        log,
        verbose,
    );
    print_some_annotations(refdata, &ann, log, verbose);
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// CDR3
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given a DNA sequence, return a CDR3 sequence in it (if found), its start
// position on the DNA sequence, and left and right scores (see below).  The CDR3
// sequence is an amino acid sequence having length between 5 and 27, starting with
// a C, and not containing a stop codon.
//
// In addition, we score the CDR3 and flanking sequences versus left and right
// motifs, and require a minimum score to pass.  These motifs were derived by
// appropriately stacking up V and J segments and looking for high multiplicity
// amino acids at given positions (see jcon.rs).
//
// If more than one CDR3 sequence is found, we first reduce to those having the
// highest score.  Then we choose the ones having the greatest start position.
// Finally we pick the longest motif.
//
// ◼ The interpretation of the tig slice is ugly.  See comments at
// ◼ get_cdr3_using_ann.

pub fn cdr3_motif_left() -> Vec<Vec<u8>> {
    vec![
        b"LQPEDSAVYY".to_vec(),
        b"VEASQTGTYF".to_vec(),
        b"ATSGQASLYL".to_vec(),
    ]
}
pub fn cdr3_motif_right() -> Vec<Vec<u8>> {
    vec![b"LTFG.GTRVTV".to_vec(), b"LIWG.GSKLSI".to_vec()]
}

pub fn cdr3_min_len() -> usize {
    5
}

pub fn cdr3_max_len() -> usize {
    27
}

pub fn get_cdr3(tig: &DnaStringSlice, cdr3: &mut Vec<(usize, Vec<u8>, usize, usize)>) {
    const MIN_TOTAL_CDR3_SCORE: usize = 10; // about as high as one can go
    let (left, right) = (cdr3_motif_left(), cdr3_motif_right());
    cdr3.clear();
    if tig.len() < 3 * (cdr3_max_len() + 3) {
        return;
    }
    let x = tig.to_owned().to_ascii_vec();
    for i in 0..3 {
        // go through three frames
        let a = aa_seq(&x, i);
        for j in 0..a.len() - min(a.len(), (cdr3_min_len() + 3) + 1) {
            if a[j] == b'C' {
                // CDR3 starts at position j on a
                let first_f = j + (cdr3_min_len() - 3);
                let last_f = min(a.len() - 4, j + (cdr3_max_len() - 1));
                for k in first_f..last_f {
                    if k + right[0].len() - 1 >= a.len() {
                        break;
                    }
                    let mut rscore = 0;
                    for m in 0..right[0].len() {
                        let mut hit = false;
                        for r in 0..right.len() {
                            if a[k + m] == right[r][m] {
                                hit = true;
                            }
                        }
                        if hit {
                            rscore += 1;
                        }
                    }
                    if rscore >= 4 {
                        let mut st = false;
                        for l in j + 1..k + 2 {
                            if a[l] == b'*' {
                                st = true;
                            }
                        }
                        let ll = left[0].len();
                        if !st && j >= ll {
                            let mut lscore = 0;
                            for m in 0..ll {
                                let mut hit = false;
                                for r in 0..left.len() {
                                    if a[j - ll + m] == left[r][m] {
                                        hit = true;
                                    }
                                }
                                if hit {
                                    lscore += 1;
                                }
                            }
                            // ◼ It's possible that the lscore + rscore
                            // ◼ bound should be increased.
                            if lscore >= 3 && lscore + rscore >= MIN_TOTAL_CDR3_SCORE {
                                cdr3.push((
                                    tig.start + i + 3 * j,
                                    a[j..k + 2 + 1].to_vec(),
                                    lscore,
                                    rscore,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Only return cdr3s having the maximum score.

    let mut m = 0;
    for i in 0..cdr3.len() {
        m = max(m, cdr3[i].2 + cdr3[i].3);
    }
    let mut to_delete = vec![false; cdr3.len()];
    for i in 0..cdr3.len() {
        if cdr3[i].2 + cdr3[i].3 < m {
            to_delete[i] = true;
        }
    }
    erase_if(cdr3, &to_delete);
    cdr3.sort();

    // Prefer later start and prefer longer CDR3.

    if cdr3.len() > 1 {
        // ◼ This is awkward.
        let n = cdr3.len();
        cdr3.swap(0, n - 1);
        cdr3.truncate(1);
    }
}

pub fn print_cdr3(tig: &DnaStringSlice, log: &mut Vec<u8>) {
    let mut cdr3 = Vec::<(usize, Vec<u8>, usize, usize)>::new();
    get_cdr3(&tig, &mut cdr3);
    for i in 0..cdr3.len() {
        fwriteln!(
            log,
            "cdr3 = {} at {}, score = {} + {}",
            strme(&cdr3[i].1),
            cdr3[i].0,
            cdr3[i].2,
            cdr3[i].3
        );
    }
}

// Given annotations of a DNA sequence, return a slice showing where the CDR3
// sequence should live, or a null slice.  This uses some empirically determined
// bounds.
//
// ◼ This seems very unlikely to be optimal.  The value of LOW_RELV_CDR3 was
// ◼ lowered to make BCR work, which suggests that measuring relative to the end
// ◼ of the V segment is not right.

pub fn cdr3_loc<'a>(
    tig: &'a DnaString,
    refdata: &RefData,
    ann: &Vec<(i32, i32, i32, i32, i32)>,
) -> DnaStringSlice<'a> {
    // Given the design of this function, the following bounds appear to be optimal
    // except possibly for changes less than ten.
    const LOW_RELV_CDR3: isize = -40;
    const HIGH_RELV_CDR3: isize = 20;
    if ann.len() == 0 {
        return tig.slice(0, 0);
    }
    let mut i = ann.len() - 1;
    loop {
        let t = ann[i].2 as usize;
        if !refdata.rheaders[t].contains("segment") && refdata.is_v(t) {
            let (l, p) = (ann[i].0 as isize, ann[i].3 as isize);
            let vstop_on_tig = l + refdata.refs[t].len() as isize - p;
            let mut start = vstop_on_tig + LOW_RELV_CDR3;
            if start < 0 {
                start = 0;
            }
            if start > tig.len() as isize {
                start = tig.len() as isize;
            }
            let mut stop = vstop_on_tig + HIGH_RELV_CDR3 + 3 * cdr3_max_len() as isize;
            if stop > tig.len() as isize {
                stop = tig.len() as isize;
            }
            return tig.slice(start as usize, stop as usize);
        }
        if i == 0 {
            return tig.slice(0, 0);
        }
        i -= 1;
    }
}

// Given a DNA sequence and annotations of it, as defined by annotate_seq, find
// CDR3 positions on it, constrained by the annotation.  This uses empirically
// determined bounds relative to that annotation.

pub fn get_cdr3_using_ann(
    tig: &DnaString,
    refdata: &RefData,
    ann: &Vec<(i32, i32, i32, i32, i32)>,
    cdr3: &mut Vec<(usize, Vec<u8>, usize, usize)>,
) {
    let window = cdr3_loc(tig, refdata, ann);

    // Enlarge the window because get_cdr3 looks for motifs to the left and right
    // of the actual CDR3.
    // ◼ Pretty ugly.  This should really be inside get_cdr3.

    let start = max(0, window.start as isize - cdr3_motif_left()[0].ilen() * 3);
    let mut stop = start + window.length as isize + cdr3_motif_right()[0].ilen() * 3;
    if stop > tig.len() as isize {
        stop = tig.len() as isize;
    }
    if stop < start {
        stop = start;
    }
    let window2 = tig.slice(start as usize, stop as usize);

    // Now find the CDR3.

    get_cdr3(&window2, cdr3)
}

pub fn print_cdr3_using_ann(
    tig: &DnaString,
    refdata: &RefData,
    ann: &Vec<(i32, i32, i32, i32, i32)>,
    log: &mut Vec<u8>,
) {
    let mut cdr3 = Vec::<(usize, Vec<u8>, usize, usize)>::new();
    get_cdr3_using_ann(tig, refdata, ann, &mut cdr3);
    for i in 0..cdr3.len() {
        fwriteln!(
            log,
            "cdr3 = {} at {}, score = {} + {}",
            strme(&cdr3[i].1),
            cdr3[i].0,
            cdr3[i].2,
            cdr3[i].3
        );
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// ANNOTATION STRUCTURE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Here we have code that presents annotations.json.

// Coordinates in an AnnotationUnit are zero-based.  The alignment score is computed
// using the following penalties:
// MATCH_SCORE = 2
// MISMATCH_PENALTY = 3
// GAP_OPEN_PENALTY = 5
// EXTEND_PENALTY = 1
// which are copied from cellranger/lib/python/cellranger/vdj/constants.py.

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationFeature {
    pub chain: String,        // chain type of the reference record, e.g. TRA
    pub display_name: String, // same as gene_name
    pub feature_id: usize,    // id of reference record
    pub gene_name: String,    // name of reference record e.g. TRAV14-1
    pub region_type: String,  // region type e.g. L-REGION+V-REGION
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationUnit {
    pub contig_match_start: usize,     // start on contig
    pub contig_match_end: usize,       // stop on contig
    pub annotation_match_start: usize, // start on reference record
    pub annotation_match_end: usize,   // stop on reference record
    pub annotation_length: usize,      // length of reference record
    pub cigar: String,                 // cigar of the alignment
    pub score: i32,                    // score of the alignment
    pub mismatches: Vec<usize>,        // unused
    pub feature: AnnotationFeature,    // feature type
}

impl AnnotationUnit {
    // Given one or two alignment entities as produced by annotate_seq, of the same
    // contig to the same reference sequence, produce an AnnotationUnit.

    pub fn from_annotate_seq(
        b: &DnaString,
        refdata: &RefData,
        ann: &Vec<(i32, i32, i32, i32, i32)>,
    ) -> AnnotationUnit {
        // Sanity check the inputs.  Obviously these conditions should be checked
        // before calling, so that they can never fail.

        let na = ann.len();
        assert!(na == 1 || na == 2);
        if ann.len() == 2 {
            assert!(ann[0].2 == ann[1].2);
            assert!(
                (ann[0].0 + ann[0].1 == ann[1].0 && ann[0].3 + ann[0].1 < ann[1].3)
                    || (ann[0].0 + ann[0].1 < ann[1].0 && ann[0].3 + ann[0].1 == ann[1].3)
            );
        }

        // Build a cigar string for a single alignment, having an indel in the case
        // where there are two alignment entities.  This does not show mismatches.

        let mut cig = String::new();
        let left1 = ann[0].0 as usize;
        let len1 = ann[0].1 as usize;
        let right1 = b.len() - left1 - len1;
        if left1 > 0 {
            cig += &format!("{}S", left1);
        }
        cig += &format!("{}M", len1);
        if na == 1 && right1 > 0 {
            cig += &format!("{}S", right1);
        }
        if na == 2 {
            let n1 = ann[1].0 - ann[0].0 - ann[0].1;
            let n2 = ann[1].3 - ann[0].3 - ann[0].1;
            if n1 == 0 {
                cig += &format!("{}D", n2);
            }
            if n2 == 0 {
                cig += &format!("{}I", n1);
            }
            let left2 = ann[1].0 as usize;
            let len2 = ann[1].1 as usize;
            let right2 = b.len() - left2 - len2;
            cig += &format!("{}M", len2);
            if right2 > 0 {
                cig += &format!("{}S", right2);
            }
        }

        // Test for internal soft clipping, which would be a bug.
        // ◼ This is horrible.  We should have a function validate_cigar_string
        // ◼ that validates a cigar string in its entirety, not just test for one
        // ◼ type of anomaly.

        let mut s_pos = Vec::new();
        let mut char_pos = 0;
        for c in cig.chars() {
            if c.is_ascii_alphabetic() {
                if c == 'S' {
                    s_pos.push(char_pos)
                }
                char_pos += 1;
            }
        }
        for p in &s_pos {
            if *p != 0 && *p != char_pos - 1 {
                panic!("Illegal internal soft clipping in cigar {}", cig);
            }
        }

        // Compute alignment score.

        let mut s = 0 as i32;
        let t = ann[0].2 as usize;
        let r = &refdata.refs[t];
        for l in 0..na {
            for i in 0..ann[l].1 {
                if b.get((ann[l].0 + i) as usize) == r.get((ann[l].3 + i) as usize) {
                    s += 2;
                } else {
                    s -= 3;
                }
            }
        }
        if na == 2 {
            let n1 = ann[1].0 - ann[0].0 - ann[0].1;
            let n2 = ann[1].3 - ann[0].3 - ann[0].1;
            let n = max(n1, n2);
            s += 4 + n;
        }

        // Build the rest.

        let types = vec!["IGH", "IGK", "IGL", "TRA", "TRB", "TRD", "TRG"];
        let mut chain_type = String::new();
        for i in 0..types.len() {
            if refdata.rheaders[t].contains(&types[i]) {
                chain_type = types[i].to_string();
                break;
            }
        }
        let v: Vec<&str> = refdata.rheaders[t].split_terminator('|').collect();
        AnnotationUnit {
            contig_match_start: ann[0].0 as usize,
            contig_match_end: (ann[na - 1].0 + ann[na - 1].1) as usize,
            annotation_match_start: ann[0].3 as usize,
            annotation_match_end: (ann[na - 1].3 + ann[na - 1].1) as usize,
            annotation_length: refdata.refs[t].len(),
            cigar: cig,
            score: s,
            mismatches: Vec::<usize>::new(),
            feature: AnnotationFeature {
                chain: chain_type,
                display_name: refdata.name[t].clone(),
                feature_id: v[1].force_usize(),
                gene_name: refdata.name[t].clone(),
                region_type: v[3].to_string(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContigAnnotation {
    // raw data for the contig
    barcode: String,     // the barcode
    contig_name: String, // name of the contig
    sequence: String,    // nucleotide sequence for contig
    quals: String,       // contig quality scores

    // contig support
    read_count: usize, // number of reads assigned to contig
    umi_count: usize,  // number of UMIs assigned to the contig

    // amino acid sequence
    //
    // The start position of the amino acid sequence on the contig is unspecified.
    // ◼ This seems like a flaw.
    start_codon_pos: Option<usize>, // start pos on contig of start codon
    stop_codon_pos: Option<usize>,  // start pos on contig of stop codon
    aa_sequence: Option<String>,    // amino acid sequence
    frame: Option<usize>,           // null and never changed (unused field)

    // CDR3
    cdr3: Option<String>,      // amino acid sequence for CDR3, or null
    cdr3_seq: Option<String>,  // nucleotide sequence for CDR3, or null
    cdr3_start: Option<usize>, // start position in bases on contig of CDR3
    cdr3_stop: Option<usize>,  // stop position in bases on contig of CDR3

    // annotations
    pub annotations: Vec<AnnotationUnit>,    // the annotations
    primer_annotations: Vec<AnnotationUnit>, // [], never filled in
    clonotype: Option<String>,               // null, filled in later
    info: HashMap<String, String>,           // {} initially, may be filled in later

    // state of the contig
    high_confidence: bool,    // declared high confidence?
    is_cell: bool,            // was the barcode declared a cell?
    productive: Option<bool>, // productive?  (null means not full length)
    filtered: bool,           // true and never changed (unused field)
}

impl ContigAnnotation {
    // Given the alignment entities produced by annotate_seq, produce a
    // ContigAnnotation.  This is done so as to produce at most one V, D, J and C,
    // each.  Pairs of alignment entities that are separated by an indel get
    // collapsed in this process.

    pub fn from_annotate_seq(
        b: &DnaString,                        // the contig
        q: &[u8],                             // qual scores for the contig
        tigname: &String,                     // name of the contig
        refdata: &RefData,                    // reference data
        ann: &Vec<(i32, i32, i32, i32, i32)>, // output of annotate_seq
        nreads: usize,                        // number of reads assigned to contig
        numis: usize,                         // number of umis assigned to contig
        high_confidencex: bool,               // declared high confidence?
        is_cellx: bool,                       // was the barcode declared a cell?
        productivex: bool,                    // productive?
    ) -> ContigAnnotation {
        let mut vstart = -1 as i32;
        for i in 0..ann.len() {
            let t = ann[i].2 as usize;
            if refdata.is_v(t) && ann[i].3 == 0 {
                vstart = ann[i].0;
            }
        }
        let mut aa = String::new();
        let mut stop = -1 as i32;
        let x = b.to_owned().to_ascii_vec();
        if vstart >= 0 {
            let y = aa_seq(&x, vstart as usize);
            aa = stringme(&y);
            for i in 0..y.len() {
                if y[i] == b'*' {
                    stop = vstart + 3 * (i as i32);
                    break;
                }
            }
        }
        let (mut cdr3x, mut cdr3x_dna) = (String::new(), String::new());
        let (mut cdr3x_start, mut cdr3x_stop) = (-1 as i32, -1 as i32);
        let mut cdr3y = Vec::<(usize, Vec<u8>, usize, usize)>::new();
        if refdata.refs.len() > 0 {
            get_cdr3_using_ann(b, refdata, ann, &mut cdr3y);
        } else {
            get_cdr3(&b.slice(0, b.len()), &mut cdr3y);
        }
        if cdr3y.len() > 0 {
            cdr3x = stringme(&cdr3y[0].1);
            let start = cdr3y[0].0;
            for i in start..start + 3 * cdr3x.len() {
                cdr3x_dna.push(x[i] as char);
            }
            cdr3x_start = start as i32;
            cdr3x_stop = (start + 3 * cdr3x.len()) as i32;
        }
        let mut qp = q.to_vec();
        for i in 0..q.len() {
            qp[i] += 33;
        }
        ContigAnnotation {
            barcode: tigname.before("_").to_string(),
            contig_name: tigname.clone(),
            sequence: b.to_string(),
            quals: stringme(&qp),
            read_count: nreads,
            umi_count: numis,
            start_codon_pos: match vstart {
                -1 => None,
                _ => Some(vstart as usize),
            },
            stop_codon_pos: match stop {
                -1 => None,
                _ => Some(stop as usize),
            },
            aa_sequence: match vstart {
                -1 => None,
                _ => Some(aa),
            },
            frame: None,
            cdr3: match cdr3x.is_empty() {
                true => None,
                _ => Some(cdr3x.clone()),
            },
            cdr3_seq: match cdr3x.is_empty() {
                true => None,
                _ => Some(cdr3x_dna),
            },
            cdr3_start: match cdr3x.is_empty() {
                true => None,
                _ => Some(cdr3x_start as usize),
            },
            cdr3_stop: match cdr3x.is_empty() {
                true => None,
                _ => Some(cdr3x_stop as usize),
            },
            annotations: make_annotation_units(b, refdata, ann),
            primer_annotations: Vec::<AnnotationUnit>::new(),
            clonotype: None,
            info: HashMap::new(),
            high_confidence: high_confidencex,
            is_cell: is_cellx,
            productive: Some(productivex),
            filtered: true,
        }
    }

    // Produce a ContigAnnotation from a sequence.

    pub fn from_seq(
        b: &DnaString,         // the contig
        q: &[u8],              // qual scores for the contig
        tigname: &String,      // name of the contig
        refdata: &RefData,     // reference data
        nreads: usize,         // number of reads assigned to contig
        numis: usize,          // number of umis assigned to contig
        high_confidence: bool, // declared high confidence?
        is_cell: bool,         // was the barcode declared a cell?
    ) -> ContigAnnotation {
        let mut ann = Vec::<(i32, i32, i32, i32, i32)>::new();
        annotate_seq(&b, &refdata, &mut ann, true, false, true);
        let mut log = Vec::<u8>::new();
        let productive = is_valid(&b, refdata, &ann, false, &mut log);
        ContigAnnotation::from_annotate_seq(
            b,
            q,
            tigname,
            refdata,
            &ann,
            nreads,
            numis,
            high_confidence,
            is_cell,
            productive,
        )
    }

    // Output with four space indentation.  Ends with comma and newline.

    pub fn write(&self, out: &mut BufWriter<File>) {
        let buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
        self.serialize(&mut ser).unwrap();
        fwriteln!(out, "{},", String::from_utf8(ser.into_inner()).unwrap());
    }

    // Print.

    pub fn print(&self, log: &mut Vec<u8>) {
        log.append(&mut serde_json::to_vec_pretty(&self).unwrap());
    }
}

// Given the alignment entities produced by annotate_seq, produce an AnnotationUnit.
// This is done so as to produce at most one V, D, J and C, each.  Pairs of
// alignment entities that are separated by an indel get collapsed in this process.

pub fn make_annotation_units(
    b: &DnaString,
    refdata: &RefData,
    ann: &Vec<(i32, i32, i32, i32, i32)>,
) -> Vec<AnnotationUnit> {
    let mut x = Vec::<AnnotationUnit>::new();
    let rtype = vec!["U", "V", "D", "J", "C"];
    for i in 0..rtype.len() {
        let mut locs = Vec::<(usize, usize, usize)>::new();
        let mut j = 0;
        while j < ann.len() {
            let t = ann[j].2 as usize;
            if refdata.segtype[t] != rtype[i].to_string() {
                j += 1;
                continue;
            }
            let mut entries = 1;
            let mut len = ann[j].1;
            if j < ann.len() - 1 && ann[j + 1].2 as usize == t {
                if (ann[j].0 + ann[j].1 == ann[j + 1].0 && ann[j].3 + ann[j].1 < ann[j + 1].3)
                    || (ann[j].0 + ann[j].1 < ann[j + 1].0 && ann[j].3 + ann[j].1 == ann[j + 1].3)
                {
                    entries = 2;
                    len += ann[j + 1].1;
                }
            }
            let mut score = len as usize;
            if refdata.segtype[t] == "V".to_string() && ann[j].3 == 0 {
                score += 1_000_000;
            }
            if refdata.segtype[t] == "J".to_string()
                && (ann[j].3 + ann[j].1) as usize == refdata.refs[t].len()
            {
                score += 1_000_000;
            }
            locs.push((score, j, entries));
            j += entries;
        }
        reverse_sort(&mut locs);
        if locs.len() > 0 {
            let (j, entries) = (locs[0].1, locs[0].2);
            let mut annx = Vec::<(i32, i32, i32, i32, i32)>::new();
            for k in j..j + entries {
                annx.push(ann[k].clone());
            }
            x.push(AnnotationUnit::from_annotate_seq(b, refdata, &annx));
        }
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_internal_soft_clipping() {
        use refx::RefData;

        let refdata = RefData::from_fasta(&String::from(
            "test/inputs/test_no_internal_soft_clipping_ref.fa",
        ));
        // println!("Loaded reference with {} entries", refdata.id.len());

        let contig_seq = DnaString::from_acgt_bytes("AGGAACTGCTCAGTTAGGACCCAGACGGAACCATGGAAGCCCCAGCGCAGCT\
        TCTCTTCCTCCTGCTACTCTGGCTCCCAGATACCACTGGAGAAATAGTGATGACGCAGTCTCCAGCCACCCTGTCTGTGTCTCCAGGGGAAAGAGCC\
        ACCCTCTCCTGCAGGGCCAGTCAGAGTGTTAGCAGCAGCTACTTAGCCTGGTACCAGCAGAAACCTGGCCAGGCTCCCAGGCTCCTCATCTATGGTG\
        CATCCACCAGGGCCACTGGTATCCCAGCCAGGTTCAGTGGCAGTGGGTCTGGGACAGAGTTCACTCTCACCATCAGCAGCCTGCAGTCTGAAGATTT\
        TGCAGTTTATTACTGTCAGCAGTATAATAACTGGCTCATGTACACTTTTGGCCAGGGGACCAAGCTGGAGATCAAACGAACTGTGGCTGCACCATCT\
        GTCTTCATCTTCCCGCCATCTGATGAGCAGTTGAAATCTGGAACTGCCTCTGTTGTGTGCCTGCTGAATAACTTCTATCCCAGAGAGGCCAAAGTAC\
        AGTGGAAGGTGGATAACGC".as_bytes());

        // Phred quality passed in, convert to raw quality, the ContigAnnotation add the
        // offset back when writing!
        let contig_qual: Vec<u8> = "III]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]]]]]]]]]]]]]]]IIII!!!IIIIIIIIIIII]]]]]]]]]X]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\
        ]]".as_bytes().iter().map(|x| x-33).collect();
        let annotation = ContigAnnotation::from_seq(
            &contig_seq,
            &contig_qual,
            &"clonotype125_consensus_1".to_string(),
            &refdata,
            120,
            2,
            true,  // high_confidence
            false, // is_cell, should be changed to None
        );

        // println!("{:#?}", annotation);
        for ann in &annotation.annotations {
            let mut s_pos = Vec::new();
            let mut char_pos = 0;
            for c in ann.cigar.chars() {
                if c.is_ascii_alphabetic() {
                    if c == 'S' {
                        s_pos.push(char_pos)
                    }
                    char_pos += 1;
                }
            }
            if !s_pos.is_empty() {
                println!("Cigar : {:?}", ann.cigar);
                println!("Soft clipping at : {:?}", s_pos);
                for p in &s_pos {
                    assert!(*p == 0 || *p == (char_pos - 1))
                }
            }
        }
    }
}
