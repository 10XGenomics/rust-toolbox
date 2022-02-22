// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains miscellaneous vector utilities.

use superslice::Ext;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// DISTANCE BETWEEN TWO VECTORS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Return the number of positions at which two vectors of equal length differ.

pub fn distance<T: Eq>(x1: &[T], x2: &[T]) -> usize {
    assert_eq!(x1.len(), x2.len());
    let mut dist = 0;
    for i in 0..x1.len() {
        if x1[i] != x2[i] {
            dist += 1;
        }
    }
    dist
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// SORT A VECTOR
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Reverse sort a vector.

pub fn reverse_sort<T: Ord>(x: &mut Vec<T>) {
    x.sort_by(|a, b| b.cmp(a));
}

// Unique sort a vector.

pub fn unique_sort<T: Ord>(x: &mut Vec<T>) {
    x.sort();
    x.dedup();
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// DOES VECTOR CONTAIN ANOTHER VECTOR
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Test to see if a vector contains another vector at a given position.

pub fn contains_at<T: Eq>(s: &[T], t: &[T], p: usize) -> bool {
    if p + t.len() > s.len() {
        return false;
    }
    for i in 0..t.len() {
        if s[p + i] != t[i] {
            return false;
        }
    }
    true
}

// Determine if vector x contains vector y.

pub fn contains<T: Eq>(x: &[T], y: &[T]) -> bool {
    if y.len() > x.len() {
        return false;
    }
    for i in 0..=x.len() - y.len() {
        let mut matches = true;
        for j in 0..y.len() {
            if x[i + j] != y[j] {
                matches = false;
                break;
            }
        }
        if matches {
            return true;
        }
    }
    false
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// UNSIGNED VECTOR SIZE AND SOME SPECIAL SIZES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub trait VecUtils<'a> {
    fn ilen(&self) -> isize;

    fn solo(&self) -> bool;

    fn duo(&self) -> bool;
}

impl<'a, T> VecUtils<'a> for [T] {
    fn ilen(&self) -> isize {
        self.len() as isize
    }

    fn solo(&self) -> bool {
        self.len() == 1
    }

    fn duo(&self) -> bool {
        self.len() == 2
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// ERASE GIVEN ELEMENTS OF A VECTOR
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Erase elements in a vector that are flagged by another vector.  Both vectors
// must have the same length.

pub fn erase_if<T>(x: &mut Vec<T>, to_delete: &[bool]) {
    let mut count = 0;
    for (j, &delete) in to_delete.iter().take(x.len()).enumerate() {
        if !delete {
            if j != count {
                x.swap(j, count);
            }
            count += 1;
        }
    }
    x.truncate(count);
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// INTERSECTION FUNCTIONS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Determine if two sorted vectors have an element in common.

pub fn meet<T: Ord>(x: &[T], y: &[T]) -> bool {
    let mut i = 0;
    let mut j = 0;
    while i < x.len() && j < y.len() {
        if x[i] < y[j] {
            i += 1;
        } else if y[j] < x[i] {
            j += 1;
        } else {
            return true;
        }
    }
    false
}

// Find the intersection size of two sorted vectors.  If an element occurs
// repeatedly, say n1 times in one vector and n2 times in the other vector, then
// that contributes min(n1,n2) to the total.

pub fn meet_size<T: Ord>(x: &[T], y: &[T]) -> usize {
    let mut i = 0;
    let mut j = 0;
    let mut count = 0;
    while i < x.len() && j < y.len() {
        if x[i] < y[j] {
            i += 1;
        } else if y[j] < x[i] {
            j += 1;
        } else {
            count += 1;
            i += 1;
            j += 1;
        }
    }
    count
}

// Compute the intersection of two sorted vectors.

pub fn intersection<T: Ord + Clone>(x: &[T], y: &[T], z: &mut Vec<T>) {
    z.clear();
    let (mut ix, mut iy) = (0, 0);
    while ix < x.len() && iy < y.len() {
        if x[ix] < y[iy] {
            ix += 1;
        } else if y[iy] < x[ix] {
            iy += 1;
        } else {
            z.push(x[ix].clone());
            ix += 1;
            iy += 1;
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// FREQUENCY FUNCTIONS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Count elements in a sorted vector by type.  The output consists of a reverse
// sorted vector of pairs (m,v) where m is the multiplicity of an element v.

pub fn make_freq<T: Ord + Clone>(x: &[T], freq: &mut Vec<(u32, T)>) {
    freq.clear();
    let mut j = 0;
    loop {
        if j == x.len() {
            break;
        }
        let mut k = j + 1;
        loop {
            if k == x.len() || x[k] != x[j] {
                break;
            }
            k += 1;
        }
        let t = x[j].clone();
        freq.push(((k - j) as u32, t));
        j = k;
    }
    freq.sort_by(|a, b| b.cmp(a)); // freq.reverse_sort();
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// MEMBERSHIP
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Test to see if a sorted vector contains a given element.

pub fn bin_member<T: Ord>(x: &[T], d: &T) -> bool {
    x.binary_search(d).is_ok()
}

// Return the position of an element in an unsorted vector.
// Returns -1 if not present.

pub fn position<T: Ord>(x: &[T], d: &T) -> i32 {
    for (i, y) in x.iter().enumerate() {
        if y == d {
            return i as i32;
        }
    }
    -1_i32
}

// Return the position of an element in a sorted vector, or using just the first
// position.  Returns -1 if not present.

pub fn bin_position<T: Ord>(x: &[T], d: &T) -> i32 {
    match x.binary_search(d) {
        Ok(p) => p as i32,
        Err(_e) => -1,
    }
}

pub fn bin_position1_2<S: Ord, T: Ord>(x: &[(S, T)], d: &S) -> i32 {
    match x.binary_search_by_key(&d, |&(ref a, ref _b)| a) {
        Ok(p) => p as i32,
        Err(_e) => -1,
    }
}

pub fn bin_position1_3<S: Ord, T: Ord, U: Ord>(x: &[(S, T, U)], d: &S) -> i32 {
    match x.binary_search_by_key(&d, |&(ref a, ref _b, ref _c)| a) {
        Ok(p) => p as i32,
        Err(_e) => -1,
    }
}

pub fn bin_position1_4<S: Ord, T: Ord, U: Ord, V: Ord>(x: &[(S, T, U, V)], d: &S) -> i32 {
    match x.binary_search_by_key(&d, |&(ref a, ref _b, ref _c, ref _d)| a) {
        Ok(p) => p as i32,
        Err(_e) => -1,
    }
}

pub fn bin_position1_5<S: Ord, T: Ord, U: Ord, V: Ord, W: Ord>(
    x: &[(S, T, U, V, W)],
    d: &S,
) -> i32 {
    match x.binary_search_by_key(&d, |&(ref a, ref _b, ref _c, ref _d, ref _e)| a) {
        Ok(p) => p as i32,
        Err(_e) => -1,
    }
}

// Find lower/upper bounds.

pub fn lower_bound<T: Ord>(x: &[T], d: &T) -> i64 {
    x.lower_bound(d) as i64
}

pub fn upper_bound<T: Ord>(x: &[T], d: &T) -> i64 {
    x.upper_bound(d) as i64
}

pub fn lower_bound1_2<S: Ord, T: Ord>(x: &[(S, T)], d: &S) -> i64 {
    x.lower_bound_by_key(&d, |(a, _b)| a) as i64
}

pub fn upper_bound1_2<S: Ord, T: Ord>(x: &[(S, T)], d: &S) -> i64 {
    x.upper_bound_by_key(&d, |(a, _b)| a) as i64
}

pub fn lower_bound1_3<S: Ord, T: Ord, U: Ord>(x: &[(S, T, U)], d: &S) -> i64 {
    x.lower_bound_by_key(&d, |(a, _b, _c)| a) as i64
}

pub fn upper_bound1_3<S: Ord, T: Ord, U: Ord>(x: &[(S, T, U)], d: &S) -> i64 {
    x.upper_bound_by_key(&d, |(a, _b, _c)| a) as i64
}

// Compute the number of instances of a given element in a sorted vector.

pub fn count_instances<T: Ord>(x: &[T], d: &T) -> i32 {
    (x.upper_bound(d) - x.lower_bound(d)) as i32
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// NEXT DIFFERENCE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Find first element that's different in a sorted vector, or different in
// first position.

pub fn next_diff<T: Eq>(x: &[T], i: usize) -> usize {
    let mut j = i + 1;
    loop {
        if j == x.len() || x[j] != x[i] {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_2<T: Eq, U: Eq>(x: &[(T, U)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_3<T: Eq, U: Eq, V: Eq>(x: &[(T, U, V)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_4<T: Eq, U: Eq, V: Eq, W: Eq>(x: &[(T, U, V, W)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff12_3<T: Eq, U: Eq, V: Eq>(x: &[(T, U, V)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32
            || x[j as usize].0 != x[i as usize].0
            || x[j as usize].1 != x[i as usize].1
        {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff12_4<T: Eq, U: Eq, V: Eq, W: Eq>(x: &[(T, U, V, W)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32
            || x[j as usize].0 != x[i as usize].0
            || x[j as usize].1 != x[i as usize].1
        {
            return j;
        }
        j += 1;
    }
}

#[allow(clippy::type_complexity)]
pub fn next_diff12_8<S: Eq, T: Eq, U: Eq, V: Eq, W: Eq, X: Eq, Y: Eq, Z: Eq>(
    x: &[(S, T, U, V, W, X, Y, Z)],
    i: i32,
) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32
            || x[j as usize].0 != x[i as usize].0
            || x[j as usize].1 != x[i as usize].1
        {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_5<T: Eq, U: Eq, V: Eq, W: Eq, X: Eq>(x: &[(T, U, V, W, X)], i: i32) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_6<T: Eq, U: Eq, V: Eq, W: Eq, X: Eq, Y: Eq>(
    x: &[(T, U, V, W, X, Y)],
    i: i32,
) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

pub fn next_diff1_7<T: Eq, U: Eq, V: Eq, W: Eq, X: Eq, Y: Eq, Z: Eq>(
    x: &[(T, U, V, W, X, Y, Z)],
    i: i32,
) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

#[allow(clippy::type_complexity)]
pub fn next_diff1_8<S: Eq, T: Eq, U: Eq, V: Eq, W: Eq, X: Eq, Y: Eq, Z: Eq>(
    x: &[(S, T, U, V, W, X, Y, Z)],
    i: i32,
) -> i32 {
    let mut j: i32 = i + 1;
    loop {
        if j == x.len() as i32 || x[j as usize].0 != x[i as usize].0 {
            return j;
        }
        j += 1;
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// RESIZE WITHOUT SETTING
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

/// resize_without_setting: Resize a vector to the given size without initializing
/// the entries.  Capacity is not reduced if it exceeds the given size.
///
/// # Safety
///
/// After a call to this function, all entries in the vector must be set before
/// being read.  Only be used when the type is 'fixed width', e.g. not on
/// Strings or Vecs, cannot own any allocated objects, or implement the Drop
/// trait.
///
/// Panics if allocation fails.
#[warn(clippy::uninit_vec)]
pub unsafe fn resize_without_setting<T>(x: &mut Vec<T>, n: usize) {
    x.clear();
    x.reserve(n);
    x.set_len(n); /* unsafe */
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// SORT SYNC VECTORS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn sort_sync2<T: Ord + Clone, S1: Ord + Clone>(t: &mut Vec<T>, s1: &mut Vec<S1>) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
}

pub fn sort_sync3<T: Ord + Clone, S1: Ord + Clone, S2: Ord + Clone>(
    t: &mut Vec<T>,
    s1: &mut Vec<S1>,
    s2: &mut Vec<S2>,
) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
    *s2 = permutation.apply_slice(&s2[..]);
}

pub fn sort_sync4<T: Ord + Clone, S1: Ord + Clone, S2: Ord + Clone, S3: Ord + Clone>(
    t: &mut Vec<T>,
    s1: &mut Vec<S1>,
    s2: &mut Vec<S2>,
    s3: &mut Vec<S3>,
) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
    *s2 = permutation.apply_slice(&s2[..]);
    *s3 = permutation.apply_slice(&s3[..]);
}

pub fn sort_sync5<
    T: Ord + Clone,
    S1: Ord + Clone,
    S2: Ord + Clone,
    S3: Ord + Clone,
    S4: Ord + Clone,
>(
    t: &mut Vec<T>,
    s1: &mut Vec<S1>,
    s2: &mut Vec<S2>,
    s3: &mut Vec<S3>,
    s4: &mut Vec<S4>,
) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
    *s2 = permutation.apply_slice(&s2[..]);
    *s3 = permutation.apply_slice(&s3[..]);
    *s4 = permutation.apply_slice(&s4[..]);
}

pub fn sort_sync6<
    T: Ord + Clone,
    S1: Ord + Clone,
    S2: Ord + Clone,
    S3: Ord + Clone,
    S4: Ord + Clone,
    S5: Ord + Clone,
>(
    t: &mut Vec<T>,
    s1: &mut Vec<S1>,
    s2: &mut Vec<S2>,
    s3: &mut Vec<S3>,
    s4: &mut Vec<S4>,
    s5: &mut Vec<S5>,
) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
    *s2 = permutation.apply_slice(&s2[..]);
    *s3 = permutation.apply_slice(&s3[..]);
    *s4 = permutation.apply_slice(&s4[..]);
    *s5 = permutation.apply_slice(&s5[..]);
}

pub fn sort_sync7<
    T: Ord + Clone,
    S1: Ord + Clone,
    S2: Ord + Clone,
    S3: Ord + Clone,
    S4: Ord + Clone,
    S5: Ord + Clone,
    S6: Ord + Clone,
>(
    t: &mut Vec<T>,
    s1: &mut Vec<S1>,
    s2: &mut Vec<S2>,
    s3: &mut Vec<S3>,
    s4: &mut Vec<S4>,
    s5: &mut Vec<S5>,
    s6: &mut Vec<S6>,
) {
    let permutation = permutation::sort(&t[..]);
    *t = permutation.apply_slice(&t[..]);
    *s1 = permutation.apply_slice(&s1[..]);
    *s2 = permutation.apply_slice(&s2[..]);
    *s3 = permutation.apply_slice(&s3[..]);
    *s4 = permutation.apply_slice(&s4[..]);
    *s5 = permutation.apply_slice(&s5[..]);
    *s6 = permutation.apply_slice(&s6[..]);
}
