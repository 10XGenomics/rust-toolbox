// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Compute some stats.
// ◼ Presumably much of this is available elsewhere.

use std::f64;

// Compute N50 of some numbers.

pub fn n50(v: &[i32]) -> i32 {
    if v.is_empty() {
        return 0;
    }
    for n in v.iter() {
        assert!(*n > 0);
    }
    let mut sum: i64 = 0;
    let mut half: i64 = 0;
    for n in v.iter() {
        sum += i64::from(*n);
    }
    let mut vs = v.to_owned();
    vs.sort();
    for i in 0..vs.len() {
        half += i64::from(vs[i]);
        if 2 * half == sum && i < vs.len() - 1 {
            return (vs[i] + vs[i + 1]) / 2;
        }
        if 2 * half >= sum {
            return vs[i];
        }
    }
    0 // never executed
}

pub fn n90(v: &[i32]) -> i32 {
    if v.is_empty() {
        return 0;
    }
    for n in v.iter() {
        assert!(*n > 0);
    }
    let mut sum: i64 = 0;
    let mut part: i64 = 0;
    for n in v.iter() {
        sum += i64::from(*n);
    }
    let mut vs = v.to_owned();
    vs.sort();
    for i in 0..vs.len() {
        part += i64::from(vs[i]);
        if 10 * (part as i64) == 9 * sum as i64 && i < vs.len() - 1 {
            return (vs[i] + vs[i + 1]) / 2;
        }
        if part as f64 / sum as f64 >= 0.9_f64 {
            return vs[i];
        }
    }
    0 // never executed
}

// Compute mean of some numbers, returning zero on empty vector.

pub fn mean(v: &[i32]) -> f64 {
    let sum1 = v.len() as f64;
    let mut sum2 = 0_f64;
    for x in v.iter() {
        sum2 += f64::from(*x);
    }
    if sum1 == 0_f64 {
        return 0_f64;
    }
    sum2 / sum1
}

// Compute "length-weighted" mean of some numbers.  Normally this would be applied
// to positive numbers.  Returns 0 if the sum of the numbers is zero.
// ◼ Not sure what the right term for this is.  The word "length" is confusing.

pub fn len_weighted_mean(v: &[i32]) -> f64 {
    let mut sum1 = 0_f64;
    let mut sum2 = 0_f64;
    for x in v.iter() {
        sum1 += f64::from(*x);
        sum2 += f64::from(*x) * f64::from(*x);
    }
    if sum1 == 0_f64 {
        return 0_f64;
    }
    sum2 / sum1
}

// Compute %CV of some numbers, returning zero on an empty vector.
// Divides by n, not n-1.

pub fn cv(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0_f64;
    }
    let n = v.len() as f64;
    let mut mean = 0_f64;
    for x in v {
        mean += x;
    }
    mean /= n;
    let mut x = 0_f64;
    for y in v {
        x += (y - mean) * (y - mean);
    }
    x = (x / n).sqrt();
    100.0 as f64 * x / mean
}

// Compute absolute value of difference between two numbers.

pub fn abs_diff(a: usize, b: usize) -> usize {
    if a <= b {
        return b - a;
    }
    a - b
}

pub fn abs_diff_f64(a: f64, b: f64) -> f64 {
    if a <= b {
        return b - a;
    }
    a - b
}

// Compute percent ratio.

pub fn percent_ratio(a: usize, b: usize) -> f64 {
    100_f64 * a as f64 / b as f64
}

// Code to make a random vector.  This uses a hardcoded pseudo-random number
// generator so that the behavior can be exactly reproduced in another language.

pub fn make_random_vec(x: &mut Vec<i64>, n: usize) {
    x.resize(n, 0);
    x[0] = 0;
    for i in 1..x.len() {
        x[i] = 6_364_136_223_846_793_005i64
            .wrapping_mul(x[i - 1])
            .wrapping_add(1_442_695_040_888_963_407);
    }
}
