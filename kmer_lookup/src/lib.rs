// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Kmer lookup.

extern crate debruijn;
extern crate rayon;
extern crate vector_utils;

use debruijn::{dna_string::*, kmer::*, Vmer, *};
use rayon::prelude::*;
use std::iter::Extend;
use vector_utils::*;

/// Given a vector of DnaStrings dv, create a sorted vector whose entries are
/// (kmer, e, estart), where the kmer starts at position estart on dv[e].
pub fn make_kmer_lookup_single<K: Kmer>(dv: &Vec<DnaString>, x: &mut Vec<(K, i32, i32)>) {
    let sz = dv
        .iter()
        .filter(|b| b.len() >= K::k())
        .map(|b| b.len() - K::k() + 1)
        .sum();
    x.clear();
    x.reserve(sz);

    for (i, b) in dv.iter().enumerate() {
        for (j, kmer) in b.iter_kmers().enumerate() {
            x.push((kmer, i as i32, j as i32));
        }
    }

    x.sort();
}

/// Included for backward compatibility. Use make_kmer_lookup_single
pub fn make_kmer_lookup_20_single(dv: &Vec<DnaString>, x: &mut Vec<(Kmer20, i32, i32)>) {
    make_kmer_lookup_single(dv, x);
}

/// Included for backward compatibility. Use make_kmer_lookup_single
pub fn make_kmer_lookup_12_single(dv: &Vec<DnaString>, x: &mut Vec<(Kmer12, i32, i32)>) {
    make_kmer_lookup_single(dv, x);
}

/// Just create a unique sorted vector of kmers.
pub fn make_kmer_lookup_single_simple<K: Kmer>(dv: &Vec<DnaString>, x: &mut Vec<K>) {
    let sz = dv
        .iter()
        .filter(|b| b.len() >= K::k())
        .map(|b| b.len() - K::k() + 1)
        .sum();
    x.clear();
    x.reserve(sz);

    x.extend(dv.iter().flat_map(|b| b.iter_kmers::<K>()));
    unique_sort(x);
}

/// Included for backward compatibility. Use make_kmer_lookup_single_simple
pub fn make_kmer_lookup_20_single_simple<K: Kmer>(dv: &Vec<DnaString>, x: &mut Vec<Kmer20>) {
    make_kmer_lookup_single_simple(dv, x);
}

/// Included for backward compatibility. Use make_kmer_lookup_single_simple
pub fn make_kmer_lookup_12_single_simple<K: Kmer>(dv: &Vec<DnaString>, x: &mut Vec<Kmer12>) {
    make_kmer_lookup_single_simple(dv, x);
}

pub fn make_kmer_lookup_20_parallel(dv: &Vec<DnaString>, x: &mut Vec<(Kmer20, i32, i32)>) {
    const K: usize = 20; // sadly this does not templatize over K
    x.clear();
    let mut starts: Vec<usize> = Vec::with_capacity(dv.len() + 1);
    starts.push(0);
    for s in dv.iter() {
        let z: usize = *(starts.last().unwrap());
        let mut y = z;
        if s.len() >= K {
            y += s.len() - K + 1;
        }
        starts.push(y);
    }
    let xsize: usize = *(starts.last().unwrap());
    unsafe {
        resize_without_setting(x, xsize);
    }
    const CHUNKSIZE: usize = 1000;
    let x_start: usize = &x[0] as *const (Kmer20, i32, i32) as usize;
    x.par_chunks_mut(CHUNKSIZE).for_each(|slice| {
        let start: usize =
            (slice.as_ptr() as usize - x_start) / std::mem::size_of::<(Kmer20, i32, i32)>();
        let mut spos = (upper_bound(&starts, &start) - 1) as usize;
        let mut i = start - starts[spos];
        for s in slice {
            while starts[spos] + i == starts[spos + 1] {
                spos += 1;
                i = 0;
            }
            let b: &DnaString = &(dv[spos]);
            s.0 = b.get_kmer(i);
            s.1 = spos as i32;
            s.2 = i as i32;
            i = i + 1;
        }
    });
    /* this is faster in C++, at least for k=48: */
    x.par_sort();
}

// Same but replace each kmer by the min of it and its rc, and if we use rc,
// adjust pos accordingly.

pub fn make_kmer_lookup_20_oriented_single<K: Kmer>(
    dv: &Vec<DnaString>,
    x: &mut Vec<(K, i32, i32)>,
) {
    let sz = dv
        .iter()
        .filter(|b| b.len() >= K::k())
        .map(|b| b.len() - K::k() + 1)
        .sum();
    x.clear();
    x.reserve(sz);

    for (i, b) in dv.iter().enumerate() {
        for (j, kmer) in b.iter_kmers::<K>().enumerate() {
            let kmer_rc = kmer.rc();

            let item = if kmer < kmer_rc {
                (kmer, i as i32, j as i32)
            } else {
                (kmer_rc, i as i32, -(j as i32) - 1)
            };
            x.push(item);
        }
    }

    x.sort();
}

// Same but replace each kmer by the min of it and its rc, and if we use rc,
// adjust pos accordingly.

pub fn make_kmer_lookup_oriented_single(dv: &Vec<DnaString>, x: &mut Vec<(Kmer20, i32, i32)>) {
    const K: usize = 20; // this does not templatize over K
                         // Kmer20 probably takes 8 bytes and could take 5.
    x.clear();
    if dv.is_empty() {
        return;
    }
    let mut starts: Vec<usize> = Vec::with_capacity(dv.len() + 1);
    starts.push(0);
    for s in dv.iter() {
        let z: usize = *(starts.last().unwrap());
        if s.len() < K {
            starts.push(z);
        } else {
            let y: usize = z + s.len() as usize - K + 1;
            starts.push(y);
        }
    }
    let xsize: usize = *(starts.last().unwrap());
    unsafe {
        resize_without_setting(x, xsize);
    }
    for i in 0..dv.len() {
        let b: &DnaString = &(dv[i]);
        if b.len() >= K {
            for j in 0..b.len() - K + 1 {
                let r = starts[i] + j;
                let y: Kmer20 = b.get_kmer(j);
                let yrc = y.rc();
                if y < yrc {
                    x[r].0 = y;
                    x[r].1 = i as i32;
                    x[r].2 = j as i32;
                } else {
                    x[r].0 = yrc;
                    x[r].1 = i as i32;
                    x[r].2 = -(j as i32) - 1;
                }
            }
        }
    }
    x.sort();
    // Note that the parallel version of this sort x.part_sort() is faster in C++.
}

// Determine if a sequence perfectly matches in forward orientation.

pub fn match_12(b: &DnaString, dv: &Vec<DnaString>, x: &Vec<(Kmer12, i32, i32)>) -> bool {
    let y: Kmer12 = b.get_kmer(0);
    let low = lower_bound1_3(&x, &y);
    let high = upper_bound1_3(&x, &y);
    for m in low..high {
        let mut l = 12;
        let t = x[m as usize].1 as usize;
        let mut p = x[m as usize].2 as usize + 12;
        while l < b.len() && p < dv[t].len() {
            if b.get(l) != dv[t].get(p) {
                break;
            }
            l += 1;
            p += 1;
        }
        if l == b.len() {
            return true;
        }
    }
    false
}
