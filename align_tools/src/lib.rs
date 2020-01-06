// Copyright (c) 2019 10x Genomics, Inc. All rights reserved.

// Some alignment tools.

extern crate bio;
extern crate debruijn;
extern crate itertools;
extern crate vector_utils;

use bio::alignment::pairwise::*;
use bio::alignment::{Alignment, AlignmentOperation::*};
use debruijn::dna_string::*;
use itertools::Itertools;
use vector_utils::*;

// Define the complexity of an alignment to be its number of mismatches plus
// its number of indel operations, where an indel is a deletion or insertion of
// an arbitrary number of bases.  Ignores clips.

pub fn complexity(a: &Alignment) -> usize {
    let ops = &a.operations;
    let (mut comp, mut i) = (0, 0);
    while i < ops.len() {
        if ops[i] == Del || ops[i] == Ins {
            let mut j = i + 1;
            while j < ops.len() && ops[j] == ops[i] {
                j += 1;
            }
            comp += 1;
            i = j - 1;
        } else if ops[i] == Subst {
            comp += 1;
        }
        i += 1;
    }
    comp
}

// Return a string that summarizes an alignment, e.g
// del(4,1) + ins(2) + sub(3)
// would mean a 2 deletions of sizes 4 and 1, an insertion of size 2, and
// 3 substitutions.  Indels are of the first sequence, relative
// to the second.  Ignores clips.

pub fn summary(a: &Alignment) -> String {
    let ops = &a.operations;
    let mut sub = 0;
    let (mut del, mut ins) = (Vec::<usize>::new(), Vec::<usize>::new());
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == Del || ops[i] == Ins {
            let mut j = i + 1;
            while j < ops.len() && ops[j] == ops[i] {
                j += 1;
            }
            if ops[i] == Del {
                del.push(j - i);
            } else {
                ins.push(j - i);
            }
            i = j - 1;
        } else if ops[i] == Subst {
            sub += 1;
        }
        i += 1;
    }
    let mut x = Vec::<String>::new();
    reverse_sort(&mut del);
    reverse_sort(&mut ins);
    if del.len() > 0 {
        x.push(format!("del({})", del.iter().format(",")));
    }
    if ins.len() > 0 {
        x.push(format!("ins({})", ins.iter().format(",")));
    }
    if sub > 0 {
        x.push(format!("sub({})", sub));
    }
    format!("{}", x.iter().format(" + "))
}

pub fn summary_less(a: &Alignment) -> String {
    let ops = &a.operations;
    let mut sub = 0;
    let (mut del, mut ins) = (Vec::<usize>::new(), Vec::<usize>::new());
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == Del || ops[i] == Ins {
            let mut j = i + 1;
            while j < ops.len() && ops[j] == ops[i] {
                j += 1;
            }
            if ops[i] == Del {
                del.push(j - i);
            } else {
                ins.push(j - i);
            }
            i = j - 1;
        } else if ops[i] == Subst {
            sub += 1;
        }
        i += 1;
    }
    reverse_sort(&mut del);
    reverse_sort(&mut ins);
    let mut s = String::new();
    if sub == 0 && del.is_empty() && ins.is_empty() {
        s = "0".to_string();
    }
    else {
        if sub > 0 {
            s = format!("{}", sub);
        }
        for i in 0..del.len() {
            s += &format!( "D{}", del[i] );
        }
        for i in 0..ins.len() {
            s += &format!( "I{}", ins[i] );
        }
    }
    s
}

// Like summary, but show more detail on indels.

pub fn summary_more(x: &DnaString, y: &DnaString, a: &Alignment) -> String {
    let ops = &a.operations;
    let mut sub = 0;
    let (mut del, mut ins) = (Vec::<String>::new(), Vec::<String>::new());
    let mut i = 0;
    let (mut p1, mut p2) = (a.xstart, a.ystart);
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
                p1 += 1;
                p2 += 1;
            }
            Subst => {
                sub += 1;
                p1 += 1;
                p2 += 1;
            }
            Del => {
                del.push(format!(
                    "del: {} ==> ∅; {} ==> {}({})",
                    p1,
                    p2,
                    y.slice(p2, p2 + opcount).to_string(),
                    opcount
                ));
                p2 += opcount;
            }
            Ins => {
                ins.push(format!(
                    "ins: {} ==> {}({}); {} ==> ∅",
                    p1,
                    x.slice(p1, p1 + opcount).to_string(),
                    opcount,
                    p2
                ));
                p1 += opcount;
            }
            Xclip(d) => {
                p1 += d;
            }
            Yclip(d) => {
                p2 += d;
            }
        }
        i += opcount;
    }
    let mut x = Vec::<String>::new();
    x.append(&mut del);
    x.append(&mut ins);
    if sub > 0 {
        x.push(format!("{} substitutions", sub));
    }
    format!("{}", x.iter().format("\n"))
}

// Return a "standard" affine alignment of x to y.  This is intended to be
// applied to the case where x is to be fully aligned to part of y.

pub fn affine_align(x: &DnaString, y: &DnaString) -> Alignment {
    let score = |a: u8, b: u8| if a == b { 1i32 } else { -1i32 };
    let mut aligner = Aligner::new(-6, -1, &score);
    aligner.semiglobal(&x.to_ascii_vec(), &y.to_ascii_vec())
}
