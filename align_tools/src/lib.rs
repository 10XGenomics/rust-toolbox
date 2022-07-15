// Copyright (c) 2021 10x Genomics, Inc. All rights reserved.
//
// Some alignment tools.

use bio_edit::alignment::pairwise::Aligner;
use bio_edit::alignment::AlignmentOperation;
use bio_edit::alignment::{
    Alignment,
    AlignmentOperation::{Del, Ins, Match, Subst, Xclip, Yclip},
};
use debruijn::dna_string::DnaString;
use itertools::Itertools;
use std::cmp::min;
use std::fmt::Write;
use string_utils::{stringme, strme};
use vector_utils::reverse_sort;

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
    if !del.is_empty() {
        x.push(format!("del({})", del.iter().format(",")));
    }
    if !ins.is_empty() {
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
    if sub == 0 && del.is_empty() && ins.is_empty() {
        "0".to_string()
    } else {
        let mut s = if sub > 0 {
            format!("{}", sub)
        } else {
            String::new()
        };
        s.reserve_exact(2 * (del.len() + ins.len()));
        for d in del {
            write!(s, "D{}", d).unwrap();
        }
        for i in ins {
            write!(s, "I{}", i).unwrap();
        }
        s
    }
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

// Exhibit a "visual" version of an alignment.  This assumes that only certain alignment operations
// are used and would need to be tweaked if other operations are present.  You can set width to
// the expected terminal width.

pub fn vis_align(s1: &[u8], s2: &[u8], ops: &[AlignmentOperation], width: usize) -> String {
    let (mut pos1, mut pos2) = (0, 0);
    let (mut t1, mut t2) = (Vec::<u8>::new(), Vec::<u8>::new());
    let mut d = Vec::<u8>::new();
    for i in 0..ops.len() {
        if ops[i] == Match || ops[i] == Subst {
            if pos1 >= s1.len() {
                eprintln!(
                    "\nIn vis_align, something wrong with ops.\n\
                    s1 = {}\ns2 = {}\nops = {:?}\n",
                    strme(s1),
                    strme(s2),
                    ops
                );
            }
            t1.push(s1[pos1]);
            t2.push(s2[pos2]);
            pos1 += 1;
            pos2 += 1;
            if ops[i] == Match {
                d.push(b' ');
            } else {
                d.push(b'*');
            }
        } else if ops[i] == Del {
            t1.push(b' ');
            t2.push(s2[pos2]);
            pos2 += 1;
            d.push(b'|');
        } else if ops[i] == Ins {
            t1.push(s1[pos1]);
            t2.push(b' ');
            pos1 += 1;
            d.push(b'|');
        } else {
            panic!("unknown operation {:?}", ops[i]);
        }
    }
    let n = t1.len(); // = t2.len()
    let mut x = Vec::<u8>::new();
    let mut start = 0;
    while start < n {
        let stop = min(start + width, n);
        for seq in [&d, &t1, &t2].iter() {
            x.append(&mut seq[start..stop].to_vec().clone());
            x.push(b'\n');
        }
        if stop < n {
            x.push(b'\n');
        }
        start = stop;
    }
    stringme(&x)
}
