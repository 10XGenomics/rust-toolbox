// Copyright (c) 2021 10x Genomics, Inc. All rights reserved.

// This file contains functions that take as input an amino acid reference sequence for a V
// segment, along with its chain type (IGH, IGK, IGL, TRA or TRB), and attempt to find features in
// the sequence.
//
// These functions can fail, in particular for reference sequences that are damaged and possibly
// for other sequences.
//
// The same functions can also be applied to a protein sequence, however the sequence needs to be
// modified to add a fake leader sequence on the left (we use MXXXXXXXXXXXXXXXXXXXX), and to
// truncate on the right to trim a bit beyond the start of the CDR3.

use string_utils::strme;
use vector_utils::{reverse_sort, sort_sync3};

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, attempt to find the start of the
// framework one region, which is immmediately after the signal peptide.  Chain type is one of
// IGH, IGK, IGL, TRA or TRB, and is not used at the moment.
//
// We find the best scoring location for this motif, starting in the first 50 amino acids
// (positions start at one).
//
// pos   weight  values
// 1     150     Q, E, D, G, K
// 2      50     V, I, Q, A
// 4     100     L, V, M
// 6     250     Q, E
// 22    250     C
// 23    250     C
//
// If the starting amino acid is C, we add one to the start position.

pub fn fr1_start(aa: &[u8], chain_type: &str) -> usize {
    // Define PWM.

    let mut pwm: [Vec<(usize, u8)>; 23] = Default::default();

    // #1

    pwm[0] = vec![
        (150, b'Q'),
        (150, b'E'),
        (150, b'D'),
        (150, b'G'),
        (150, b'K'),
    ];

    pwm[1] = vec![(50, b'V'), (50, b'I'), (50, b'Q'), (50, b'A')]; // #2
    pwm[3] = vec![(100, b'L'), (100, b'V'), (100, b'M')]; //          #4
    pwm[5] = vec![(250, b'Q'), (250, b'E')]; //                       #6

    // #22

    pwm[21] = if chain_type == "IGH" {
        vec![(500, b'C')]
    } else {
        vec![(250, b'C')]
    };

    // #23

    pwm[22] = vec![(250, b'C')];

    // Score positions.

    let end = (aa.len() - pwm.len()).min(40);
    let mut score_pos = Vec::<(usize, usize)>::with_capacity(end + 1);
    for j in 0..=end {
        if chain_type == "IGL" && j > 25 {
            break;
        }
        let mut score = 0;
        for p in 0..pwm.len() {
            for l in 0..pwm[p].len() {
                if pwm[p][l].1 == aa[j + p] {
                    score += pwm[p][l].0;
                }
            }
        }
        score_pos.push((score, j));
    }
    reverse_sort(&mut score_pos);
    let mut p = score_pos[0].1;
    if aa[p] == b'C' {
        p += 1;
    }
    p
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, attempt to find the start of the
// CDR1 region.  Note that there is more than one convention regarding the CDR1 start, and these
// conventions appear to differ by fixed offsets.  The convention used here is for IMGT.
// Chain type is one of IGH, IGK, IGL, TRA or TRB.

pub fn cdr1_start(aa: &[u8], chain_type: &str, verbose: bool) -> Option<usize> {
    // Define PWM for eight amino acids.

    const Z: usize = 19;
    let pwm = [
        /* #19 */
        vec![(50, b'V')],
        /* #20 */
        vec![(30, b'T')],
        /* #21 */
        vec![(200, b'L'), (200, b'I'), (200, b'V'), (200, b'M')],
        /* #22 */
        vec![(80, b'S'), (80, b'T'), (80, b'R')],
        /* #23 */
        vec![(250, b'C')],
        /* #24 */
        Vec::default(),
        /* #25 */
        Vec::default(),
        /* #26 */
        vec![(100, b'S'), (100, b'I'), (100, b'D')],
    ];

    // Score positions.

    let fr1 = fr1_start(aa, chain_type);
    let end = (fr1 + 27).saturating_sub(pwm.len());
    let mut score_pos = Vec::<(usize, usize)>::with_capacity(end.saturating_sub(7 + Z - 2));
    for j in 7 + Z - 1..=(aa.len() - pwm.len()).min(end) {
        let mut score = 0;
        for p in 0..pwm.len() {
            for l in 0..pwm[p].len() {
                if pwm[p][l].1 == aa[j + p] {
                    score += pwm[p][l].0;
                }
            }
        }
        if verbose {
            let seq = &aa[j..j + pwm.len()];
            println!("j = {}, seq = {}, score = {}", j, strme(seq), score);
        }
        score_pos.push((score, j));
    }
    reverse_sort(&mut score_pos);
    let mut add = 3;
    if chain_type.starts_with("TR") || chain_type == "IGH" {
        add = 6;
    }
    if score_pos.is_empty() {
        return None;
    }
    Some(score_pos[0].1 + add + 2)
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, attempt to find the start of the
// FR2 region.  Chain type is one of IGH, IGK, IGL, TRA or TRB'

pub fn fr2_start(aa: &[u8], chain_type: &str, verbose: bool) -> Option<usize> {
    // Define PWM for six amino acids.

    const Z: usize = 39;
    let pwm = [
        // #39
        vec![(50, b'L'), (50, b'M'), (50, b'V'), (50, b'F')],
        // #40
        if chain_type == "IGL" {
            vec![(250, b'Y')]
        } else {
            Vec::default()
        },
        // #41
        vec![(250, b'W')],
        // #42
        vec![(150, b'Y')],
        // #43
        vec![(100, b'R')],
        // #44
        vec![(250, b'Q')],
        // #45-46
        Vec::default(),
        Vec::default(),
        // #47
        vec![(110, b'G')],
        // #48
        vec![(60, b'K'), (60, b'Q')],
        // #49
        vec![(40, b'G'), (40, b'K'), (40, b'A')],
    ];

    // Score positions.

    const START: usize = 2 + Z - 1;
    const STOP: usize = 35 + Z - 1;
    let mut score_pos = Vec::<(usize, usize)>::with_capacity(STOP - START + 1);
    for j in START..=(aa.len() - pwm.len()).min(STOP) {
        if chain_type == "IGH" {
            const STOP: usize = 24 + Z - 1;
            if j > STOP {
                continue;
            }
        }
        let mut score = 0;
        for p in 0..pwm.len() {
            for l in 0..pwm[p].len() {
                if pwm[p][l].1 == aa[j + p] {
                    score += pwm[p][l].0;
                }
            }
        }
        if verbose {
            println!("j = {}, score = {}", j, score);
        }
        score_pos.push((score, j));
    }
    reverse_sort(&mut score_pos);
    let mut add = 0;
    if chain_type != "IGH" {
        add = 1;
    }
    if chain_type == "IGK" || chain_type == "IGL" {
        add += 2;
    }
    if score_pos.is_empty() {
        None
    } else {
        Some(score_pos[0].1 + add - 1)
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, attempt to find the start of the
// CDR2 region.  Chain type is one of IGH, IGK, IGL, TRA or TRB.

pub fn cdr2_start(aa: &[u8], chain_type: &str, verbose: bool) -> Option<usize> {
    let s2 = fr2_start(aa, chain_type, false);
    s2?;
    let s2 = s2.unwrap();
    let mut add = 0_isize;
    if chain_type == "IGH" {
        // Six amino acids preceeding the CDR2 start.

        let pwm = [
            // #50
            vec![(80, b'L')],
            // #51
            vec![(80, b'E')],
            // #52
            vec![(80, b'W')],
            // #53
            vec![(40, b'V'), (40, b'M'), (40, b'I'), (40, b'L')],
            // #54
            vec![(40, b'G'), (40, b'S'), (40, b'A')],
            // #55
            vec![],
        ];

        // Score positions.

        let mut score_pos = Vec::<(usize, usize)>::with_capacity(6);
        for j in s2 + 8..=(aa.len() - pwm.len()).min(s2 + 13) {
            let mut score = 0;
            for p in 0..pwm.len() {
                for l in 0..pwm[p].len() {
                    if pwm[p][l].1 == aa[j + p] {
                        score += pwm[p][l].0;
                    }
                }
            }
            if verbose {
                println!("j = {}, score = {}", j, score);
            }
            score_pos.push((score, j));
        }
        reverse_sort(&mut score_pos);
        if score_pos.is_empty() {
            return None;
        }
        Some(score_pos[0].1 + 7)
    } else if chain_type == "TRA" {
        // Six amino acids preceeding the CDR2 start.

        let pwm = [
            // #50
            vec![(15, b'P'), (15, b'L')],
            // #51
            vec![(15, b'Q'), (15, b'V'), (15, b'E'), (15, b'T'), (15, b'I')],
            // #52
            vec![(20, b'L'), (20, b'F')],
            // #53
            vec![(35, b'L')],
            // #54
            vec![(15, b'L'), (15, b'I')],
            // #55
            vec![],
        ];

        // Score positions.

        let mut score_pos = Vec::<(usize, usize)>::with_capacity(3);
        for j in s2 + 10..=(aa.len() - pwm.len()).min(s2 + 12) {
            let mut score = 0;
            for (p, wm) in pwm.iter().enumerate() {
                for l in wm {
                    if l.1 == aa[j + p] {
                        score += l.0;
                    }
                }
            }
            // if verbose {
            //     println!("j = {}, score = {}", j, score);
            // }
            score_pos.push((score, j));
        }
        reverse_sort(&mut score_pos);
        if score_pos.is_empty() {
            None
        } else {
            Some(score_pos[0].1 + 6)
        }
    } else {
        if chain_type == "IGK" || chain_type == "IGL" {
            add = -2;
        }
        Some(s2 + (17 + add) as usize)
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn cdr3_start(aa: &[u8], _chain_type: &str, _verbose: bool) -> usize {
    let motif = [b"LQPEDSAVYYC", b"VEASQTGTYFC", b"ATSGQASLYLC"];
    let nm = motif[0].len();
    let reach = 18;
    let mut scores = Vec::<(usize, usize)>::new();
    for j in aa.len().saturating_sub(nm + reach)..=aa.len().saturating_sub(nm) {
        let mut score = 0;
        for k in 0..nm {
            for l in 0..motif.len() {
                if aa[j + k] == motif[l][k] {
                    score += 1;
                    if aa[j + k] == b'Q' {
                        break;
                    }
                }
            }
        }
        scores.push((score, j + nm));
    }
    reverse_sort(&mut scores);
    scores[0].1
}

pub fn cdr3_score(aa: &[u8], _chain_type: &str, _verbose: bool) -> usize {
    let motif = [b"LQPEDSAVYYC", b"VEASQTGTYFC", b"ATSGQASLYLC"];
    let nm = motif[0].len();
    const REACH: usize = 18;
    let mut scores = Vec::<(usize, usize)>::with_capacity(REACH + 1);
    for j in aa.len().saturating_sub(nm + REACH)..=aa.len().saturating_sub(nm) {
        let mut score = 0;
        for k in 0..nm {
            for l in 0..motif.len() {
                if aa[j + k] == motif[l][k] {
                    score += 1;
                    if aa[j + k] == b'Q' {
                        break;
                    }
                }
            }
        }
        scores.push((score, j + nm));
    }
    reverse_sort(&mut scores);
    scores[0].0
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, attempt to find the start of the
// FR3 region.

pub fn fr3_start(aa: &[u8], chain_type: &str, verbose: bool) -> Option<usize> {
    // First find the start of the CDR3.

    let cdr3_start = cdr3_start(aa, chain_type, verbose);
    /*
    use string_utils::*;
    println!(
        "bases before cdr3 = {}",
        strme(&aa[cdr3_start - 6..cdr3_start])
    );
    */

    // Do IGK and IGL.

    if chain_type == "IGK" || chain_type == "IGL" {
        let pwm = [
            // x1
            vec![(100, b'G')],
            // x2
            Vec::default(),
            // x3
            vec![(100, b'P')],
            // x4
            Vec::default(),
            // x5
            vec![(100, b'R')],
            // x6
            vec![(100, b'F')],
            // x7
            Vec::default(),
            // x8
            vec![(100, b'G')],
        ];

        // Score positions.

        if cdr3_start < 35 {
            return None;
        }
        let mut score_pos = Vec::<(usize, usize)>::with_capacity(35 - 28 + 1);
        for j in cdr3_start - 35..=cdr3_start - 28 {
            // changed to 39
            let mut score = 0;
            for (p, wm) in pwm.iter().enumerate() {
                for l in wm {
                    if l.1 == aa[j + p] {
                        score += l.0;
                    }
                }
            }
            if verbose {
                println!("j = {}, score = {}", j, score);
            }
            score_pos.push((score, j));
        }
        reverse_sort(&mut score_pos);
        if !score_pos.is_empty() {
            Some(score_pos[0].1)
        } else {
            None
        }

    // Do IGH.
    } else if chain_type == "IGH" {
        let pwm = [
            // #2
            vec![(600, b'Y'), (600, b'N')],
            // #3
            vec![(500, b'Y')],
            // #4-6
            vec![(400, b'A'), (400, b'N')],
            Vec::default(),
            Vec::default(),
            // #7
            vec![(850, b'F'), (850, b'L')],
            // #8
            vec![(800, b'K'), (800, b'Q'), (800, b'R')],
            // #9
            Vec::default(),
            // #10
            vec![(1000, b'R'), (1000, b'K')],
            // #11
            vec![(700, b'F'), (700, b'V'), (700, b'A'), (700, b'L')],
        ];

        // Score positions.

        let mut score_pos = Vec::<(usize, isize)>::with_capacity(42 - 32 + 1);
        for j in cdr3_start.saturating_sub(42 - 2)..=cdr3_start.saturating_sub(32 - 2) {
            let mut score = 0;
            for (p, wm) in pwm.iter().enumerate() {
                for l in wm {
                    if l.1 == aa[j + p] {
                        score += l.0;
                    }
                }
            }
            // use string_utils::*;
            // println!("score of {} = {}", strme(&aa[j..j + pwm.len()]), score);
            if verbose {
                println!("j = {}, score = {}", j, score);
            }
            score_pos.push((score, -(j as isize)));
        }
        reverse_sort(&mut score_pos);
        if !score_pos.is_empty() {
            if -score_pos[0].1 >= 1 {
                Some((-score_pos[0].1) as usize - 1)
            } else {
                None
            }
        } else {
            None
        }

    // Do TRA.
    } else if chain_type == "TRA" {
        let pwm = [
            // #m0
            vec![(50, b'E'), (50, b'V'), (50, b'N'), (50, b'K')],
            // #m1
            vec![(50, b'T'), (50, b'K'), (50, b'A'), (50, b'E')],
            // #m2
            vec![(50, b'E'), (50, b'S')],
            // #m3
            vec![(50, b'N'), (50, b'D'), (50, b'S')],
            // #m4
            vec![(50, b'G'), (50, b'N')],
            // #m5
            vec![(80, b'R'), (80, b'G'), (80, b'M')],
            // #m6
            vec![(50, b'F'), (50, b'Y'), (50, b'A'), (50, b'I')],
            // #m7
            vec![(50, b'S'), (50, b'T')],
            // #m8
            vec![(50, b'A'), (50, b'V')],
            // #m9
            vec![(50, b'T'), (50, b'E')],
            // #m10
            Vec::default(),
            // #m11
            vec![(50, b'N'), (50, b'D')],
        ];

        // Score positions.

        let mut score_pos = Vec::<(usize, usize)>::with_capacity(36 - 33 + 1);
        for j in cdr3_start - 36..=cdr3_start - 33 {
            let mut score = 0;
            for (p, wm) in pwm.iter().enumerate() {
                for l in wm {
                    if l.1 == aa[j + p] {
                        score += l.0;
                    }
                }
            }
            // println!("score of {} = {}", strme(&aa[j..j + pwm.len()]), score);
            if verbose {
                println!("start = {}, score = {}", j + 1, score);
            }
            score_pos.push((score, j));
        }
        reverse_sort(&mut score_pos);
        if !score_pos.is_empty() {
            Some(score_pos[0].1 + 1)
        } else {
            None
        }

    // Do TRB.
    } else {
        let pwm = [
            // #m1
            Vec::default(),
            // #m2
            Vec::default(),
            // #m3
            vec![(50, b'K'), (50, b'E'), (50, b'D')],
            // #m4
            vec![(200, b'G'), (200, b'S'), (200, b'Q')],
            // #m5
            vec![(200, b'D'), (200, b'E'), (200, b'G'), (200, b'S')],
            // #m6
            vec![(200, b'I'), (200, b'V'), (200, b'L'), (200, b'M')],
            // #m7
            vec![(100, b'P'), (100, b'S')],
        ];

        // Score positions.

        let mut score_pos = Vec::<(usize, usize)>::with_capacity(38 - 35 + 1);
        for j in cdr3_start - 38..=cdr3_start - 35 {
            let mut score = 0;
            for (p, wm) in pwm.iter().enumerate() {
                for l in wm {
                    if l.1 == aa[j + p] {
                        score += l.0;
                    }
                }
            }
            // println!("score of {} = {}", strme(&aa[j..j + pwm.len()]), score);
            if verbose {
                println!("j = {}, score = {}", j, score);
            }
            score_pos.push((score, j));
        }
        reverse_sort(&mut score_pos);
        if !score_pos.is_empty() {
            Some(score_pos[0].1)
        } else {
            None
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, return the CDR1 sequence.
// Chain type is one of IGH, IGK, IGL, TRA or TRB, and is not used at the moment.

pub fn cdr1(aa: &[u8], chain_type: &str, verbose: bool) -> Option<Vec<u8>> {
    let fr2 = fr2_start(aa, chain_type, verbose);
    let cdr1 = cdr1_start(aa, chain_type, verbose);
    if fr2.is_none() || cdr1.is_none() || cdr1.unwrap() > fr2.unwrap() {
        return None;
    }
    Some(aa[cdr1.unwrap()..fr2.unwrap()].to_vec())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Given the amino acid sequence for a V reference sequence, return the CDR2 sequence.
// Chain type is one of IGH, IGK, IGL, TRA or TRB.

pub fn cdr2(aa: &[u8], chain_type: &str, verbose: bool) -> Option<Vec<u8>> {
    let start = cdr2_start(aa, chain_type, verbose);
    start?;
    let start = start.unwrap();
    let stop = fr3_start(aa, chain_type, false);
    stop?;
    let stop = stop.unwrap();
    if start > stop {
        return None;
    }
    Some(aa[start..stop].to_vec())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn fwr1(aa: &[u8], chain_type: &str, verbose: bool) -> Option<Vec<u8>> {
    let fr1 = fr1_start(aa, chain_type);
    let cdr1 = cdr1_start(aa, chain_type, verbose);
    cdr1?;
    if fr1 > cdr1.unwrap() {
        return None;
    }
    Some(aa[fr1..cdr1.unwrap()].to_vec())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn fwr2(aa: &[u8], chain_type: &str, verbose: bool) -> Option<Vec<u8>> {
    let fr2 = fr2_start(aa, chain_type, verbose);
    fr2?;
    let cdr2 = cdr2_start(aa, chain_type, verbose);
    cdr2?;
    if fr2.unwrap() > cdr2.unwrap() {
        return None;
    }
    Some(aa[fr2.unwrap()..cdr2.unwrap()].to_vec())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn fwr3(aa: &[u8], chain_type: &str, verbose: bool) -> Option<Vec<u8>> {
    let fr3 = fr3_start(aa, chain_type, verbose);
    fr3?;
    let cdr3 = cdr3_start(aa, chain_type, verbose);
    if fr3.unwrap() > cdr3 {
        return None;
    }
    Some(aa[fr3.unwrap()..cdr3].to_vec())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn score_fwr3(aa: &[u8], r: usize, freqs: &[Vec<Vec<(u32, u8)>>]) -> f64 {
    let chain_type;
    if r == 0 {
        chain_type = "IGH";
    } else if r == 1 {
        chain_type = "IGK";
    } else if r == 2 {
        chain_type = "IGL";
    } else if r == 3 {
        chain_type = "TRA";
    } else {
        chain_type = "TRB";
    }
    let cdr3 = cdr3_start(aa, chain_type, false);
    let motif = freqs[0].len();
    let mut score = 0.0;
    for j in 0..motif {
        let x = aa[cdr3 - j - 1];
        let mut m = 0;
        let mut total = 0;
        for k in 0..freqs[r][j].len() {
            let count = freqs[r][j][k].0;
            let y = freqs[r][j][k].1;
            total += count;
            if y == x {
                m += count;
            }
        }
        score += m as f64 / total as f64;
    }
    score
}

pub fn score_fwr3_at_end(aa: &[u8], r: usize, freqs: &[Vec<Vec<(u32, u8)>>]) -> f64 {
    let cdr3 = aa.len();
    let motif = freqs[0].len();
    let mut score = 0.0;
    for j in 0..motif {
        if cdr3 < j + 1 {
            return 0.0;
        }
        let x = aa[cdr3 - j - 1];
        let mut m = 0;
        let mut total = 0;
        for k in 0..freqs[r][j].len() {
            let count = freqs[r][j][k].0;
            let y = freqs[r][j][k].1;
            total += count;
            if y == x {
                m += count;
            }
        }
        score += m as f64 / total as f64;
    }
    score
}

pub fn score4(aa: &[u8], r: usize) -> usize {
    let chain_type;
    if r == 0 {
        chain_type = "IGH";
    } else if r == 1 {
        chain_type = "IGK";
    } else if r == 2 {
        chain_type = "IGL";
    } else if r == 3 {
        chain_type = "TRA";
    } else {
        chain_type = "TRB";
    }
    let cdr3 = cdr3_start(aa, chain_type, false);
    let n = aa.len();
    assert!(n >= 22);
    let mut score = 0;
    let x = aa[cdr3 - 4];
    if x == b'V' || x == b'T' || x == b'L' {
        score += 1;
    }
    let x = aa[cdr3 - 3];
    if x == b'Y' {
        score += 1;
    }
    let x = aa[cdr3 - 2];
    if x == b'Y' || x == b'F' || x == b'L' {
        score += 1;
    }
    let x = aa[cdr3 - 1];
    if x == b'C' {
        score += 3;
    }
    score
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Determine the IG chain type of an amino acid sequence that goes up to at least the end of
// the J segment, and also determine the stop position of the J segment.
// These are designed to be perfect for human and mouse reference J segments,
// but there could be SHM and/or germline changes, and other species might differ.

pub fn ig_j_gene(aa: &[u8]) -> Option<(String, usize)> {
    let iglj = [
        [b'L', b'M', b'V'].to_vec(),
        [b'F'].to_vec(),
        [b'G'].to_vec(),
        [].to_vec(),
        [b'G'].to_vec(),
        [b'T'].to_vec(),
        [b'K', b'Q'].to_vec(),
        [b'L', b'V'].to_vec(),
        [b'T'].to_vec(),
        [b'V'].to_vec(),
        [b'L'].to_vec(),
    ];
    let igkj = [
        [b'R', b'S', b'T'].to_vec(),
        [b'F'].to_vec(),
        [b'R', b'S', b'G'].to_vec(),
        [].to_vec(),
        [b'G'].to_vec(),
        [b'T'].to_vec(),
        [b'K', b'Q', b'R'].to_vec(),
        [b'L', b'V', b'F'].to_vec(),
        [b'D', b'E'].to_vec(),
        [b'I', b'M', b'L'].to_vec(),
        [b'K'].to_vec(),
    ];
    let ighj = [
        [b'W'].to_vec(),
        [b'G'].to_vec(),
        [].to_vec(),
        [b'G'].to_vec(),
        [b'T'].to_vec(),
        [].to_vec(),
        [b'L', b'V'].to_vec(),
        [b'T'].to_vec(),
        [b'V'].to_vec(),
        [b'S'].to_vec(),
        [b'A', b'S'].to_vec(),
    ];
    // A guess, not tested or optimized:
    const MIN_MATCHES: usize = 5;
    let len = iglj.len();
    if aa.len() < len {
        return None;
    }
    let mut max_matches = vec![0; 3];
    let mut best_pos = vec![0; 3];
    let chains = ["IGL".to_string(), "IGK".to_string(), "IGH".to_string()];
    for (ip, x) in [&iglj, &igkj, &ighj].iter().enumerate() {
        for p in 0..=aa.len() - len {
            let mut matchesx = 0;
            for j in 0..len {
                for l in 0..x[j].len() {
                    if aa[p + j] == x[j][l] {
                        matchesx += 1;
                    }
                }
            }
            if matchesx >= max_matches[ip] {
                max_matches[ip] = matchesx;
                best_pos[ip] = p;
            }
        }
    }
    let mut u = vec![0, 1, 2];
    sort_sync3(&mut max_matches, &mut best_pos, &mut u);
    if max_matches[2] >= MIN_MATCHES {
        Some((chains[u[2]].clone(), best_pos[2] + len))
    } else {
        None
    }
}
