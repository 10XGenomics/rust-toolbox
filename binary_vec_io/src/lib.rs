// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

// Write and read functions to which one passes a File, a ref to a number type
// defining the start of a 'vector' of entries, and the number of entries.
//
// See also crate memmap.

extern crate failure;

use self::failure::Error;
use std::io::Write;

pub trait BinaryInputOutputSafe {}
impl BinaryInputOutputSafe for i8 {}
impl BinaryInputOutputSafe for i16 {}
impl BinaryInputOutputSafe for i32 {}
impl BinaryInputOutputSafe for i64 {}
impl BinaryInputOutputSafe for u8 {}
impl BinaryInputOutputSafe for u16 {}
impl BinaryInputOutputSafe for u32 {}
impl BinaryInputOutputSafe for u64 {}
impl BinaryInputOutputSafe for f32 {}
impl BinaryInputOutputSafe for f64 {}
// i128, u128?

#[allow(dead_code)]
pub fn binary_write_from_ref<T>(f: &mut std::fs::File, p: &T, n: usize) -> Result<(), Error> {
    let raw = p as *const T as *const u8;
    unsafe {
        let sli: &[u8] = std::slice::from_raw_parts(raw, n * (std::mem::size_of::<T>()));
        f.write_all(sli)?;
        Ok(())
    }
}

pub fn binary_read_to_ref<T>(f: &mut std::fs::File, p: &mut T, n: usize) -> Result<(), Error> {
    let raw = p as *mut T as *mut u8;
    unsafe {
        let sli: &mut [u8] = std::slice::from_raw_parts_mut(raw, n * (std::mem::size_of::<T>()));
        // A read can be incomplete, so we try a second time if needed.  Conceivably more attempts
        // are necessary.
        use std::io::Read;
        let bytes_to_read = sli.len();
        let bytes_read = f.read(sli).unwrap();
        if bytes_read < bytes_to_read {
            let raw = raw.add(bytes_read);
            let sli: &mut [u8] = std::slice::from_raw_parts_mut(raw, bytes_to_read - bytes_read);
            let bytes_read2 = f.read(sli).unwrap();
            assert_eq!(bytes_read2, 0);
        }
    }
    Ok(())
}

// The functions binary_write_vec and binary_read_vec append, either to a file,
// in the first case, or to a vector, in the second case.

#[allow(dead_code)]
pub fn binary_write_vec<T>(f: &mut std::fs::File, x: &Vec<T>) -> Result<(), Error>
where
    T: BinaryInputOutputSafe,
{
    let n = x.len();
    binary_write_from_ref::<usize>(f, &n, 1)?;
    binary_write_from_ref::<T>(f, &x[0], x.len())
}

#[allow(dead_code)]
pub fn binary_read_vec<T>(f: &mut std::fs::File, x: &mut Vec<T>) -> Result<(), Error>
where
    T: BinaryInputOutputSafe,
{
    // Read the vector size.

    let mut n: usize = 0;
    binary_read_to_ref::<usize>(f, &mut n, 1)?;

    // Resize the vector without setting any of its entries.
    // (could use resize_without_setting)

    let len = x.len();
    if len + n > x.capacity() {
        let extra: usize = len + n - x.capacity();
        x.reserve(extra);
    }
    unsafe {
        x.set_len(len + n);
    }

    // Read the vector entries.

    binary_read_to_ref::<T>(f, &mut x[len], n)
}

pub fn binary_write_vec_vec<T>(f: &mut std::fs::File, x: &Vec<Vec<T>>) -> Result<(), Error>
where
    T: BinaryInputOutputSafe,
{
    let n = x.len();
    binary_write_from_ref::<usize>(f, &n, 1)?;
    for i in 0..n {
        binary_write_vec::<T>(f, &x[i])?;
    }
    Ok(())
}

pub fn binary_read_vec_vec<T>(f: &mut std::fs::File, x: &mut Vec<Vec<T>>) -> Result<(), Error>
where
    T: BinaryInputOutputSafe + Clone,
{
    let mut n: usize = 0;
    binary_read_to_ref::<usize>(f, &mut n, 1)?;
    let len = x.len();
    if len + n > x.capacity() {
        let extra: usize = len + n - x.capacity();
        x.reserve(extra);
    }
    x.resize(len + n, Vec::<T>::new());
    for i in 0..n {
        binary_read_vec::<T>(f, &mut x[i])?;
    }
    Ok(())
}
