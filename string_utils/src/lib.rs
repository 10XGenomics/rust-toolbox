// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains some miscellaneous string utilities.

extern crate vec_utils;

use std::cmp::max;
use vec_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// THINGS USED A LOT: SHORTHAND EXPRESSIONS FOR COMMON FUNCTIONALITY
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Make a &[u8] into an &str or an  string.

pub fn strme(s: &[u8]) -> &str {
    std::str::from_utf8(s).unwrap()
}

pub fn stringme(s: &[u8]) -> String {
    String::from_utf8(s.to_vec()).unwrap()
}

pub trait TextUtils<'a> {
    fn force_usize(&self) -> usize;
    fn force_i32(&self) -> i32;
    fn force_i64(&self) -> i64;
    fn force_u64(&self) -> u64;
    fn force_f64(&self) -> f64;

    // s.before(t): return the part of s before the first instance of t
    // (or panic if t is not contained in s)

    fn before(&'a self, u: &str) -> &'a str;

    // s.after(t): return the part of s after the first instance of t
    // (or panic if t is not contained in s)

    fn after(&'a self, t: &str) -> &'a str;

    // s.between(t,u): return the part of s after the first instance of t and
    // before the first instance of u after that

    fn between(&'a self, t: &str, u: &str) -> &'a str;

    // s.rev_before(t): start from the end s, find the first instance of t, and
    // return what's before that

    fn rev_before(&'a self, t: &str) -> &'a str;

    // s.rev_after(t): start from the end s, find the first instance of t, and
    // return what's after that

    fn rev_after(&'a self, t: &str) -> &'a str;
}

impl<'a> TextUtils<'a> for str {
    fn force_usize(&self) -> usize {
        self.parse::<usize>()
            .expect(&format!("could not convert \"{}\" to usize", self))
    }
    fn force_i32(&self) -> i32 {
        self.parse::<i32>()
            .expect(&format!("could not convert \"{}\" to i32", self))
    }
    fn force_i64(&self) -> i64 {
        self.parse::<i64>()
            .expect(&format!("could not convert \"{}\" to i64", self))
    }
    fn force_u64(&self) -> u64 {
        self.parse::<u64>()
            .expect(&format!("could not convert \"{}\" to u64", self))
    }
    fn force_f64(&self) -> f64 {
        self.parse::<f64>()
            .expect(&format!("could not convert \"{}\" to f64", self))
    }

    fn before(&'a self, u: &str) -> &'a str {
        let r = self
            .find(u)
            .expect(&format!("failed to find \"{}\" in \"{}\"", u, self));
        &self[0..r]
    }

    fn after(&'a self, t: &str) -> &'a str {
        let l = self
            .find(t)
            .expect(&format!("after failed to find \"{}\" in \"{}\"", t, self))
            + t.len();
        &self[l..self.len()]
    }

    fn between(&'a self, t: &str, u: &str) -> &'a str {
        let a = self.after(t);
        let r = a.find(u).expect(&format!(
            "between( \"{}\", \"{}\", \"{}\" ) failed at second part",
            self, t, u
        ));
        &a[0..r]
    }

    fn rev_before(&'a self, t: &str) -> &'a str {
        let l = 0;
        let r = self.rfind(t).unwrap();
        &self[l..r]
    }

    fn rev_after(&'a self, t: &str) -> &'a str {
        let l = self.rfind(t).unwrap();
        &self[l + t.len()..self.len()]
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// THINGS USED OCCASIONALLY
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Parse a line, breaking at commas, but not if they're in quotes.  And strip
// the quotes.

pub fn parse_csv(x: &[u8]) -> Vec<Vec<u8>> {
    let mut y = Vec::<Vec<u8>>::new();
    let (mut quotes, mut i) = (0, 0);
    while i < x.len() {
        let mut j = i;
        while j < x.len() {
            if quotes % 2 == 0 && x[j] == b',' {
                break;
            }
            if x[j] == b'"' {
                quotes += 1;
            }
            j += 1;
        }
        let mut start = i;
        let mut stop = j;
        if stop - start >= 2 && x[start] == b'"' && x[stop - 1] == b'"' {
            start += 1;
            stop -= 1;
        }
        y.push(x[start..stop].to_vec());
        i = j + 1;
    }
    y
}

// Quote a bunch of strings.

pub fn quote_vec(x: &[&str]) -> Vec<String> {
    let mut y = vec![String::new(); x.len()];
    for i in 0..x.len() {
        y[i] = format!("\"{}\"", x[i]);
    }
    y
}

// Convert a sorted list into a an abbreviated string.

pub fn abbrev_list<T: Eq + std::fmt::Display>(x: &[T]) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i < x.len() {
        if i > 0 {
            s.push_str(", ");
        }
        let j = next_diff(&x, i);
        if j - i == 1 {
            s.push_str(&format!("{}", x[i]));
        } else {
            s.push_str(&format!("{}^{}", x[i], j - i));
        }
        i = j;
    }
    s
}

// capitalize first letter

pub fn cap1(s: &str) -> String {
    let mut x = s.as_bytes().to_vec();
    let c = x[0].to_ascii_uppercase();
    x[0] = c;
    String::from_utf8(x.to_vec()).unwrap()
}

// stolen from internet, add commas to number

pub fn add_commas(n: usize) -> String {
    let s = format!("{}", n);
    let mut result = String::with_capacity(s.len() + ((s.len() - 1) / 3));
    let mut i = s.len();
    for c in s.chars() {
        result.push(c);
        i -= 1;
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
    }
    result
}

// decimal_diffs: given two strings, determine if they are identical except for
// numerical differences, as e.g.
// woof_1.2x_3
// woof_10.3x_7
// would be (two differences).  And return the positions of the differing strings
// and their float vallues, as
// diffs = {(start1,stop1,start2,start2,stop2,x1,x2)}.  If the two strings are
// identical or do not satisfy the requirements, an empty vector of diffs is
// returned.

pub fn decimal_diffs(
    s1: &[u8],
    s2: &[u8],
    diffs: &mut Vec<(usize, usize, usize, usize, f64, f64)>,
) {
    let (n1, n2) = (s1.len(), s2.len());
    diffs.clear();
    let (mut i1, mut i2) = (0, 0);
    loop {
        if i1 == n1 && i2 == n2 {
            return;
        }
        if i1 == n1 || i2 == n2 {
            diffs.clear();
            return;
        }
        let d1 = (s1[i1] >= b'0' && s1[i1] <= b'9') || s1[i1] == b'.';
        let d2 = (s2[i2] >= b'0' && s2[i2] <= b'9') || s2[i2] == b'.';
        if d1 != d2 || (!d1 && s1[i1] != s2[i2]) {
            diffs.clear();
            return;
        }
        if !d1 {
            i1 += 1;
            i2 += 1;
            continue;
        }
        let (mut j1, mut j2) = (i1 + 1, i2 + 1);
        let (mut dots1, mut dots2) = (0, 0);
        while j1 < n1 {
            if s1[j1] == b'.' {
                if dots1 == 1 {
                    break;
                }
                dots1 += 1;
            } else if !(s1[j1] >= b'0' && s1[j1] <= b'9') {
                break;
            }
            j1 += 1;
        }
        while j2 < n2 {
            if s2[j2] == b'.' {
                if dots2 == 1 {
                    break;
                }
                dots2 += 1;
            } else if !(s2[j2] >= b'0' && s2[j2] <= b'9') {
                break;
            }
            j2 += 1;
        }
        if s1[i1..j1] != s2[i2..j2] {
            let x1 = strme(&s1[i1..j1]).force_f64();
            let x2 = strme(&s2[i2..j2]).force_f64();
            diffs.push((i1, j1, i2, j2, x1, x2));
        }
        i1 = j1;
        i2 = j2;
    }
}

// Horizontal concatention.  Consider two vectors of strings, to be thought of as 
// rows to be printed.  Create a new vector of strings that is the horizontal
// concatenation of these rows, first padding the first vector with blanks on the
// right to achieve equal length and then adding additional specified separation.

pub fn hcat( col1: &Vec<String>, col2: &Vec<String>, sep: usize ) -> Vec<String> {
    let mut cat = Vec::<String>::new();
    let height = max( col1.len(), col2.len() );
    let mut width1 = 0;
    for x in col1 {
        width1 = max( width1, x.len() + sep );
    }
    for i in 0..height {
        let mut s = String::new();
        if i < col1.len() {
            s = col1[i].clone();
        }
        while s.len() < width1 {
            s += " ";
        }
        if i < col2.len() {
            s += &col2[i];
        }
        cat.push(s);
    }
    cat
}
