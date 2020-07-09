// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

//! # Stirling numbers of the second kind and friends
//!
//! For integers <code>0 ≤ k ≤ n</code>, the Stirling number of the second kind <code>S(n,k)</code>
//! is the number of <code>k</code>-element partitions of a set of size <code>n</code>.
//! See <a href="https://en.wikipedia.org/wiki/Stirling_numbers_of_the_second_kind">wikipedia</a>.
//!
//! This crate consists of a few functions related to these Stirling numbers.

extern crate num_traits;

#[cfg(not(debug_assertions))]
#[cfg(test)]
extern crate num_bigint;
#[cfg(not(debug_assertions))]
#[cfg(test)]
extern crate num_rational;
#[cfg(not(debug_assertions))]
#[cfg(test)]
extern crate rand;
#[cfg(not(debug_assertions))]
#[cfg(test)]
extern crate rayon;
#[cfg(not(debug_assertions))]
#[cfg(test)]
extern crate vector_utils;

use num_traits::{Num, One, Zero};
use std::ops::MulAssign;

/// Build a table of Stirling numbers of the second kind <code>S(n,k)</code>, for
/// <code>n ≤ n_max</code>.
/// <br>&nbsp;
///
/// <b>Method.</b>  Use the recurrence relation:
/// <pre>
/// S(n,0) = delta(0,n)
/// S(n,n) = 1
/// S(n,k) = k * S(n-1,k) + S(n-1,k-1) if 1 ≤ k < n.
/// </pre>
/// Reasonable choices for <code>T</code> are <code>f64</code> and <code>BigUint</code>, which
/// are exact integers.
///
/// <b>Computational complexity.</b>  <code>O(n_max^2)</code> assuming that <code>T</code>
/// is a fixed-size type like <code>f64</code>.
///
/// <b>Testing and accuracy.</b>  For <code>T = f64</code> and <code>n_max = 219</code>, by
/// comparing with exact values, we verify that the table entries are accurate to 14 decimal places
/// (assuming that the exact values are correct).  For <code>n_max = 220</code>, some table
/// entries are infinite.  We also check one table entry versus wikipedia, so that the test is
/// not just checking internal consistency.

pub fn stirling2_table<T: Num + Clone + From<u32>>(n_max: usize) -> Vec<Vec<T>> {
    let mut s = Vec::<Vec<T>>::new();
    let zero: T = Zero::zero();
    let one: T = One::one();
    for n in 0..=n_max {
        s.push(vec![zero.clone(); n + 1]);
    }
    s[0][0] = one.clone();
    for n in 1..=n_max {
        s[n][0] = zero.clone();
        for k in 1..n {
            s[n][k] = T::from(k as u32) * s[n - 1][k].clone() + s[n - 1][k - 1].clone();
        }
        s[n][n] = one.clone();
    }
    s
}

/// Compute a table of "Stirling ratios", Stirling numbers divided by the asympotic approximation
/// <code>k^n / k!</code>, which is useful because the ratios are numerically better behaved than
/// the Stirling numbers themselves.
/// <br>&nbsp;
///
/// <b>Motivation.</b>
/// The utility of these ratios is that their numerical behavior is better than the Stirling
/// numbers themselves: the table can be computed up to much larger <code>n</code> without turning
/// to junk.  The ratios also work more naturally with some applications e.g. in
/// <code>
/// p_at_most_m_distinct_in_sample_of_x_from_n.
/// </code>
/// <br><br>
/// We don't have a reference for this material.
///
/// <b>Method.</b>  The ratio <code>SR(n,k) := S(n,k) / ( k^n / k! )</code>
/// with the special case definition <code>SR(0,0) = 1</code> satisfies the recurrence relation:
/// <br>
/// <pre>
/// SR(n,0) = delta(0,n)
/// SR(n,n) = n! / n^n
/// SR(n,k) = SR(n-1,k) + SR(n-1,k-1) * ((k-1)/k))^(n-1)
///           if 1 ≤ k < n.
/// </pre>
///
/// <b>Computational complexity.</b>  <code>O(n_max^2)</code> assuming that <code>T</code>
/// is a fixed-size type like <code>f64</code>.
///
/// <b>Testing and accuracy.</b>  Tested using <code>f64</code>.  For <code>n_max = 722</code>, the
/// values are accurate to 12 digits; this fails for <code>723</code>.

pub fn stirling2_ratio_table<T: Num + Clone + MulAssign + From<u32>>(n_max: usize) -> Vec<Vec<T>> {
    let mut s = Vec::<Vec<T>>::new();
    let zero: T = Zero::zero();
    let one: T = One::one();
    for n in 0..=n_max {
        s.push(vec![zero.clone(); n + 1]);
    }
    s[0][0] = one.clone();
    let mut z = Vec::<T>::new();
    for n in 1..=n_max {
        s[n][0] = zero.clone();
        for k in 1..n - 1 {
            z[k - 1] *= T::from((k - 1) as u32) / T::from(k as u32);
        }
        if n >= 2 {
            let mut u = one.clone();
            for _ in 0..n - 1 {
                u *= T::from((n - 2) as u32) / T::from((n - 1) as u32);
            }
            z.push(u);
        }
        for k in 1..n {
            let x = z[k - 1].clone(); // = ((k-1)/k)^(n-1)
            s[n][k] = s[n - 1][k].clone() + s[n - 1][k - 1].clone() * x;
        }
        s[n][n] = one.clone(); // now set to n! / n^n
        for j in 1..=n {
            s[n][n] *= T::from(j as u32) / T::from(n as u32);
        }
    }
    s
}

/// Compute the probability of selecting at most <code>m</code> distinct elements in
/// <code>x</code> random draws with
/// replacement from a set of size <code>n</code>.
/// <br>&nbsp;
///
/// <b>Method.</b>  The probability of selecting <i>exactly</i> <code>m</code> distinct elements is
/// <br><code>Z(m,x,n) = S(x,m) * ( n * ... * (n-m+1) ) / n^x</code>
/// <br>where <code>S(x,m)</code> is the Stirling number of the second kind.
/// Reference:
/// <a href="https://math.stackexchange.com/questions/32800/probability-distribution-of-coverage-of-a-set-after-x-independently-randomly">stack exchange question 32800</a>
/// on the "probability distribution of coverage of a set after x independently randomly selected
/// members of the set".
///
/// In terms of Stirling ratios <code>SR</code>,
/// <code>Z(m,x,n) = SR(x,m) * (m/n)^x * choose(n,m)</code>.
///
/// Thus the probability of selecting at most <code>m</code> distinct elements in <code>x</code>
/// random draws with replacement from a set of size <code>n</code> is:
/// <code>
/// <br>sum( SR(x,u) * (u/n)^x * choose(n,u), u = 0..=m )<br>
/// = 1 - sum( SR(x,u) * (u/n)^x * choose(n,u), u = m+1..=x ).
/// </code>
/// <br>
/// which is computed below, using a precomputed Stirling ratio table.
/// <br><br>
/// <b>Complexity.  </b> <code>O( (x-m) * x )</code>.  If one wants to speed this up, probably one
/// can do it by truncating the sum, without significantly affecting accuracy.
/// <br><br>
/// <b>Testing and accuracy.</b> For <code>T = f64</code>, we test one value for this by
/// simulation.  For
/// <code>m = 27</code>, <code>x = 30</code>, <code>n = 2500</code>, the function computes
/// <code>0.0005953</code> (rounded), versus <code>0.0005936</code> (rounded) for simulation
/// using a sample of size <code>100,000,000</code>.

#[allow(clippy::many_single_char_names)]
pub fn p_at_most_m_distinct_in_sample_of_x_from_n(
    m: usize,
    x: usize,
    n: usize,
    sr: &[Vec<f64>],
) -> f64 {
    let mut p = 1.0;
    for u in m + 1..=x {
        let mut z = sr[x][u];
        for _ in 0..x {
            z *= u as f64 / n as f64;
        }
        for v in 1..=u {
            z *= (n - v + 1) as f64 / (u - v + 1) as f64;
        }
        p -= z;
    }
    if p < 0.0 {
        p = 0.0;
    }
    p
}

#[cfg(test)]
mod tests {

    // Test stirling stuff.  Works with "cargo test --release".  We don't allow
    // "cargo test" because it is insanely slow.

    #[cfg(debug_assertions)]
    #[test]
    fn test_vdj_stirling_stuff_fail() {
        println!(
            "\n\"cargo test\" deliberately fails here because without running in release mode,"
        );
        println!("the test would be too slow.\n");
        assert!(0 == 1);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_stirling_stuff() {
        use num_bigint::{BigInt, BigUint, ToBigUint};
        use num_rational::Ratio;
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
        use vector_utils::*;

        use super::*;

        // Helper functions.

        fn simulate_p_at_most_m_distinct_in_sample_of_x_from_n(
            m: usize,
            x: usize,
            n: usize,
            count: usize,
        ) -> f64 {
            let group = 1000;
            assert!(count % group == 0);
            let mut rng: StdRng = SeedableRng::from_seed([0 as u8; 32]);
            let mut seeds = Vec::<[u8; 32]>::new();
            for _ in 0..group {
                let mut x = [0 as u8; 32];
                for j in 0..x.len() {
                    x[j] = rng.gen_range(0, 255);
                }
                seeds.push(x);
            }
            let goods: Vec<_> = seeds
                .par_iter()
                .map(|seed| {
                    let mut rng: StdRng = SeedableRng::from_seed(*seed);
                    let mut goods = 0;
                    for _ in 0..count / group {
                        let mut sample = Vec::<usize>::new();
                        for _ in 0..x {
                            sample.push(rng.gen_range(0, n));
                        }
                        unique_sort(&mut sample);
                        if sample.len() <= m {
                            goods += 1;
                        }
                    }
                    goods
                })
                .collect();
            goods.iter().sum::<usize>() as f64 / count as f64
        }

        //
        // Check that the ratio of two big integers equals 1, up to some number of digits.
        //
        // Note that a cleaner way to do this would be via a function that rounded a BigInt
        // Rational to an f64.  There is in fact a pull request to create such a function,
        // last touched 7/7/19: https://github.com/rust-num/num-rational/pull/52.

        fn assert_equal_to_d_digits(x1: &BigUint, x2: &BigUint, d: usize) {
            let mut n = 1 as usize;
            for _ in 0..d {
                n *= 10;
            }
            let y1 = x1.clone() * n.to_biguint().unwrap();
            let y1x2 = y1 / x2.clone();

            if y1x2 != n.to_biguint().unwrap()
                && y1x2 != (n - 1).to_biguint().unwrap()
                && y1x2 != (n + 1).to_biguint().unwrap()
            {
                eprintln!("x1 != x2 to {} digits, y1x2 = {}", y1x2, d);
                assert!(0 == 1);
            }
        }

        // Test one value in stirling2_table<f64> versus value in wikipedia.

        let n_max = 3000;
        let s2 = stirling2_table::<f64>(n_max);
        assert_eq!(s2[10][5], 42525.0);

        // Compute exact stirling2_table entries.

        let nb = 722;
        let sbig = stirling2_table::<BigUint>(nb);

        // Test accuracy of stirling2_table entries.  For n = 219, the values are accurate to
        // 14 decimal places.

        let n = 219;
        for k in 1..=n {
            let r = Ratio::<BigInt>::from_float(s2[n][k]).unwrap();
            let (rnum, rden) = (
                r.numer().to_biguint().unwrap(),
                r.denom().to_biguint().unwrap(),
            );
            let x1 = sbig[n][k].clone() * rden;
            let x2 = rnum;
            assert_equal_to_d_digits(&x1, &x2, 14);
        }

        // Verify that Stirling ratios for n = 722 are accurate to 12 digits.  This is not
        // true for n = 723.

        let n_max = 2500;
        let n = nb;
        let sr = stirling2_ratio_table::<f64>(n_max);
        for k in 1..=n {
            let mut kf = 1.to_biguint().unwrap(); // compute k!
            for j in 1..=k {
                kf *= j.to_biguint().unwrap();
            }
            let mut kn = 1.to_biguint().unwrap(); // compute k^n
            for _ in 1..=n {
                kn *= k.to_biguint().unwrap();
            }
            let r = Ratio::<BigInt>::from_float(sr[n][k]).unwrap();
            let (rnum, rden) = (
                r.numer().to_biguint().unwrap(),
                r.denom().to_biguint().unwrap(),
            );
            let x1 = sbig[n][k].clone() * kf * rden;
            let x2 = kn * rnum;
            assert_equal_to_d_digits(&x1, &x2, 12);
        }

        // Validate one value for fn p_at_most_m_unique_in_sample_of_x_from_n.

        let m = 27;
        let x = 30;
        let n = 2500;
        let p1 = p_at_most_m_distinct_in_sample_of_x_from_n(m, x, n, &sr);
        let p2 = simulate_p_at_most_m_distinct_in_sample_of_x_from_n(m, x, n, 100_000_000);
        assert_eq!(format!("{:.7}", p1), "0.0005953");
        assert_eq!(format!("{:.7}", p2), "0.0005952");
    }
}
