// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

// This file describes a data structure that reasonably efficiently encodes a non-editable sparse
// matrix of nonnegative integers, each < 2^32, and most which are very small, together with
// string labels for rows and columns.  The number of rows and columns are assumed to be ≤ 2^32.
// The structure is optimized for the case where the number of columns is ≤ 2^16.  Space
// requirements roughly double if this is exceeded.
//
// The defining property and raison d'être for this data structure is that it can be read directly
// ("mirrored") as a single block of bytes, and thus reading is fast and especially fast on
// subsequent reads once the data are cached.  This makes it appropriate for interactive
// applications.  But note that actually extracting values from the data structure can be slow.
//
// The data structure is laid out as a vector of bytes, representing a mix of u8, u16 and u32
// entries, in general unaligned, as follow in the case where the number of columns is ≤ 2^16:
//
// 1. "MirrorSparseMatrix binary file \n" (32 bytes)
// 2. code version (4 bytes)
// 3. storage version (4 bytes)
// 4. number of rows (u32) = n
// 5. number of columns (u32) = k                                    *** ADDED ***
// 6. for each i in 0..n, byte start of data for row i (n x u32)
// 7. for each i in 0..=n, byte start of string label for row i      *** ADDED ***
// 8. for each j in 0..=k, byte start of string label for column j   *** ADDED ***
// 9. for each row:
//    (a) number of sparse entries whose value is stored in u8 (u16) = m1
//    (b) number of sparse entries whose value is stored in u16 (u16) = m2
//    (c) number of sparse entries whose value is stored in u32 (u16) = m4 [note redundant]
//    (a') data for (a), m1 entries of form
//         - column identifier (u16)
//         - value (u8)
//    (b') data for (b), m2 entries of form
//         - column identifier (u16)
//         - value (u16)
//    (c') data for (c), m4 entries of form
//         - column identifier (u16)
//         - value (u32).
//
// The case where the number of columns is > 2^16 is the same except that all the u16 entries are
// changed to u32.
//
// Initial API:
// * read from disk
// * write to disk
// * build from (Vec<Vec<(i32,i32)>, Vec<String>, Vec<String>) representation
// * report the number of rows and the number of columns
// * report the sum of entries for a given row
// * report the sum of entries for a given column
// * return the value of a given matrix entry
// * return a given row
// * return a row or column label.
//
// The initial version was 0.  In version 1, string labels for rows and columns were added.
// A version 0 file can no longer be read, except that, you can read the version and exit.
// All functions after item 4 above will assert strangely or return garbage.  It would be
// better to first call get_code_version_from_file.

use binary_vec_io::{binary_read_to_ref, binary_read_vec, binary_write_vec};
use std::cmp::max;

#[derive(Clone)]
pub struct MirrorSparseMatrix {
    x: Vec<u8>,
}

pub fn get_code_version_from_file(f: &str) -> u32 {
    assert_eq!(std::mem::size_of::<usize>(), 8); // for the usize at the beginning of the file
    let mut ff = std::fs::File::open(&f).unwrap();
    let mut x = vec![0_u32; 11];
    binary_read_to_ref::<u32>(&mut ff, &mut x[0], 11).unwrap();
    x[10]
}

pub fn read_from_file(s: &mut MirrorSparseMatrix, f: &str) {
    let mut ff = std::fs::File::open(&f).unwrap();
    binary_read_vec::<u8>(&mut ff, &mut s.x).unwrap();
    if s.code_version() != 0 && s.code_version() != 1 {
        panic!(
            "\nMirrorSparseMatrix: code_version has to be 0 or 1, but it is {}.\n",
            s.code_version()
        );
    }
    if s.storage_version() != 0 && s.storage_version() != 1 {
        panic!(
            "\nMirrorSparseMatrix: storage_version has to be 0 or 1, but it is {}.\n",
            s.storage_version()
        );
    }
}

pub fn write_to_file(s: &MirrorSparseMatrix, f: &str) {
    assert!(s.code_version() > 0);
    let mut ff =
        std::fs::File::create(&f).unwrap_or_else(|_| panic!("Failed to create file {}.", f));
    binary_write_vec::<u8>(&mut ff, &s.x).unwrap();
}

fn get_u8_at_pos(v: &[u8], pos: usize) -> u8 {
    v[pos]
}

fn get_u16_at_pos(v: &[u8], pos: usize) -> u16 {
    let mut z = [0_u8; 2];
    z.clone_from_slice(&v[pos..(2 + pos)]);
    u16::from_le_bytes(z)
}

fn get_u32_at_pos(v: &[u8], pos: usize) -> u32 {
    let mut z = [0_u8; 4];
    z.clone_from_slice(&v[pos..(4 + pos)]);
    u32::from_le_bytes(z)
}

fn _put_u8_at_pos(v: &mut Vec<u8>, pos: usize, val: u8) {
    v[pos] = val;
}

fn _put_u16_at_pos(v: &mut Vec<u8>, pos: usize, val: u16) {
    let z = val.to_le_bytes();
    v[pos..(2 + pos)].clone_from_slice(&z[..2]);
}

fn put_u32_at_pos(v: &mut Vec<u8>, pos: usize, val: u32) {
    let z = val.to_le_bytes();
    v[pos..(4 + pos)].clone_from_slice(&z[..4]);
}

fn push_u8(v: &mut Vec<u8>, val: u8) {
    v.push(val);
}

fn push_u16(v: &mut Vec<u8>, val: u16) {
    let z = val.to_le_bytes();
    for i in 0..2 {
        v.push(z[i]);
    }
}

fn push_u32(v: &mut Vec<u8>, val: u32) {
    let z = val.to_le_bytes();
    for i in 0..4 {
        v.push(z[i]);
    }
}

impl MirrorSparseMatrix {
    pub fn new() -> MirrorSparseMatrix {
        let v = Vec::<u8>::new();
        MirrorSparseMatrix { x: v }
    }

    pub fn initialized(&self) -> bool {
        !self.x.is_empty()
    }

    fn header_size() -> usize {
        32  // text header
        + 4 // code version
        + 4 // storage version
        + 4 // number of rows
        + 4 // number of columns
    }

    fn code_version(&self) -> usize {
        get_u32_at_pos(&self.x, 32) as usize
    }

    fn storage_version(&self) -> usize {
        get_u32_at_pos(&self.x, 36) as usize
    }

    pub fn build_from_vec(
        x: &[Vec<(i32, i32)>],
        row_labels: &[String],
        col_labels: &[String],
    ) -> MirrorSparseMatrix {
        let mut max_col = 0_i32;
        for i in 0..x.len() {
            for j in 0..x[i].len() {
                max_col = max(max_col, x[i][j].0);
            }
        }
        let mut storage_version = 0_u32;
        if max_col >= 65536 {
            storage_version = 1_u32;
        }
        let hs = MirrorSparseMatrix::header_size();
        let mut v = Vec::<u8>::new();
        let mut total_bytes = hs + 4 * x.len();
        for i in 0..x.len() {
            let (mut m1, mut m2, mut m4) = (0, 0, 0);
            for j in 0..x[i].len() {
                if x[i][j].1 < 256 {
                    m1 += 1;
                } else if x[i][j].1 < 65536 {
                    m2 += 1;
                } else {
                    m4 += 1;
                }
            }
            if storage_version == 0 {
                total_bytes += 6 + 3 * m1 + 4 * m2 + 6 * m4;
            } else {
                total_bytes += 12 + 5 * m1 + 6 * m2 + 8 * m4;
            }
        }
        let (n, k) = (row_labels.len(), col_labels.len());
        assert_eq!(n, x.len());
        total_bytes += 4 * (1 + n);
        total_bytes += 4 * (1 + k);
        let byte_start_of_row_labels = total_bytes;
        for i in 0..n {
            total_bytes += row_labels[i].len();
        }
        let byte_start_of_col_labels = total_bytes;
        for j in 0..k {
            total_bytes += col_labels[j].len();
        }
        v.reserve(total_bytes);
        v.append(&mut b"MirrorSparseMatrix binary file \n".to_vec());
        assert_eq!(v.len(), 32);
        const CURRENT_CODE_VERSION: usize = 1;
        let code_version = CURRENT_CODE_VERSION as u32;
        push_u32(&mut v, code_version);
        push_u32(&mut v, storage_version);
        push_u32(&mut v, n as u32);
        push_u32(&mut v, k as u32);
        assert_eq!(v.len(), hs);
        for _ in 0..n {
            push_u32(&mut v, 0_u32);
        }

        // Define row and column label starts.

        let mut pos = byte_start_of_row_labels;
        for i in 0..=n {
            push_u32(&mut v, pos as u32);
            if i < n {
                pos += row_labels[i].len();
            }
        }
        let mut pos = byte_start_of_col_labels;
        for j in 0..=k {
            push_u32(&mut v, pos as u32);
            if j < k {
                pos += col_labels[j].len();
            }
        }

        // Insert matrix entries.

        for i in 0..n {
            let p = v.len() as u32;
            put_u32_at_pos(&mut v, hs + 4 * i, p);
            let (mut m1, mut m2, mut m4) = (0, 0, 0);
            for j in 0..x[i].len() {
                if x[i][j].1 < 256 {
                    m1 += 1;
                } else if x[i][j].1 < 65536 {
                    m2 += 1;
                } else {
                    m4 += 1;
                }
            }
            if storage_version == 0 {
                push_u16(&mut v, m1 as u16);
                push_u16(&mut v, m2 as u16);
                push_u16(&mut v, m4 as u16);
            } else {
                push_u32(&mut v, m1 as u32);
                push_u32(&mut v, m2 as u32);
                push_u32(&mut v, m4 as u32);
            }
            for j in 0..x[i].len() {
                if x[i][j].1 < 256 {
                    if storage_version == 0 {
                        push_u16(&mut v, x[i][j].0 as u16);
                    } else {
                        push_u32(&mut v, x[i][j].0 as u32);
                    }
                    push_u8(&mut v, x[i][j].1 as u8);
                }
            }
            for j in 0..x[i].len() {
                if x[i][j].1 >= 256 && x[i][j].1 < 65536 {
                    if storage_version == 0 {
                        push_u16(&mut v, x[i][j].0 as u16);
                    } else {
                        push_u32(&mut v, x[i][j].0 as u32);
                    }
                    push_u16(&mut v, x[i][j].1 as u16);
                }
            }
            for j in 0..x[i].len() {
                if x[i][j].1 >= 65536 {
                    if storage_version == 0 {
                        push_u16(&mut v, x[i][j].0 as u16);
                    } else {
                        push_u32(&mut v, x[i][j].0 as u32);
                    }
                    push_u32(&mut v, x[i][j].1 as u32);
                }
            }
        }

        // Insert row and column labels.

        for i in 0..n {
            for p in 0..row_labels[i].len() {
                v.push(row_labels[i].as_bytes()[p]);
            }
        }
        for j in 0..k {
            for p in 0..col_labels[j].len() {
                v.push(col_labels[j].as_bytes()[p]);
                pos += 1;
            }
        }

        // Done.

        assert_eq!(total_bytes, v.len());
        MirrorSparseMatrix { x: v }
    }

    pub fn nrows(&self) -> usize {
        get_u32_at_pos(&self.x, 40) as usize
    }

    pub fn ncols(&self) -> usize {
        get_u32_at_pos(&self.x, 44) as usize
    }

    fn start_of_row(&self, row: usize) -> usize {
        let pos = MirrorSparseMatrix::header_size() + row * 4;
        get_u32_at_pos(&self.x, pos) as usize
    }

    pub fn row_label(&self, i: usize) -> String {
        let row_labels_start = MirrorSparseMatrix::header_size() + self.nrows() * 4;
        let label_start = get_u32_at_pos(&self.x, row_labels_start + i * 4);
        let label_stop = get_u32_at_pos(&self.x, row_labels_start + (i + 1) * 4);
        let label_bytes = &self.x[label_start as usize..label_stop as usize];
        String::from_utf8(label_bytes.to_vec()).unwrap()
    }

    pub fn col_label(&self, j: usize) -> String {
        let col_labels_start = MirrorSparseMatrix::header_size() + self.nrows() * 8 + 4;
        let label_start = get_u32_at_pos(&self.x, col_labels_start + j * 4);
        let label_stop = get_u32_at_pos(&self.x, col_labels_start + (j + 1) * 4);
        let label_bytes = &self.x[label_start as usize..label_stop as usize];
        String::from_utf8(label_bytes.to_vec()).unwrap()
    }

    pub fn row(&self, row: usize) -> Vec<(usize, usize)> {
        let mut all = Vec::<(usize, usize)>::new();
        let s = self.start_of_row(row);
        if self.storage_version() == 0 {
            let m1 = get_u16_at_pos(&self.x, s) as usize;
            let m2 = get_u16_at_pos(&self.x, s + 2) as usize;
            let m4 = get_u16_at_pos(&self.x, s + 4) as usize;
            for i in 0..m1 {
                let pos = s + 6 + 3 * i;
                let col = get_u16_at_pos(&self.x, pos) as usize;
                let entry = get_u8_at_pos(&self.x, pos + 2) as usize;
                all.push((col, entry));
            }
            for i in 0..m2 {
                let pos = s + 6 + 3 * m1 + 4 * i;
                let col = get_u16_at_pos(&self.x, pos) as usize;
                let entry = get_u16_at_pos(&self.x, pos + 2) as usize;
                all.push((col, entry));
            }
            for i in 0..m4 {
                let pos = s + 6 + 3 * m1 + 4 * m2 + 6 * i;
                let col = get_u16_at_pos(&self.x, pos) as usize;
                let entry = get_u32_at_pos(&self.x, pos + 2) as usize;
                all.push((col, entry));
            }
        } else {
            let m1 = get_u32_at_pos(&self.x, s) as usize;
            let m2 = get_u32_at_pos(&self.x, s + 4) as usize;
            let m4 = get_u32_at_pos(&self.x, s + 8) as usize;
            for i in 0..m1 {
                let pos = s + 12 + 5 * i;
                let col = get_u32_at_pos(&self.x, pos) as usize;
                let entry = get_u8_at_pos(&self.x, pos + 4) as usize;
                all.push((col, entry));
            }
            for i in 0..m2 {
                let pos = s + 12 + 5 * m1 + 6 * i;
                let col = get_u32_at_pos(&self.x, pos) as usize;
                let entry = get_u16_at_pos(&self.x, pos + 4) as usize;
                all.push((col, entry));
            }
            for i in 0..m4 {
                let pos = s + 12 + 5 * m1 + 6 * m2 + 8 * i;
                let col = get_u32_at_pos(&self.x, pos) as usize;
                let entry = get_u32_at_pos(&self.x, pos + 4) as usize;
                all.push((col, entry));
            }
        }
        all.sort_unstable();
        all
    }

    pub fn sum_of_row(&self, row: usize) -> usize {
        let s = self.start_of_row(row);
        let mut sum = 0;
        if self.storage_version() == 0 {
            let m1 = get_u16_at_pos(&self.x, s) as usize;
            let m2 = get_u16_at_pos(&self.x, s + 2) as usize;
            let m4 = get_u16_at_pos(&self.x, s + 4) as usize;
            for i in 0..m1 {
                let pos = s + 6 + 3 * i + 2;
                sum += get_u8_at_pos(&self.x, pos) as usize;
            }
            for i in 0..m2 {
                let pos = s + 6 + 3 * m1 + 4 * i + 2;
                sum += get_u16_at_pos(&self.x, pos) as usize;
            }
            for i in 0..m4 {
                let pos = s + 6 + 3 * m1 + 4 * m2 + 6 * i + 2;
                sum += get_u32_at_pos(&self.x, pos) as usize;
            }
        } else {
            let m1 = get_u32_at_pos(&self.x, s) as usize;
            let m2 = get_u32_at_pos(&self.x, s + 4) as usize;
            let m4 = get_u32_at_pos(&self.x, s + 8) as usize;
            for i in 0..m1 {
                let pos = s + 12 + 5 * i + 4;
                sum += get_u8_at_pos(&self.x, pos) as usize;
            }
            for i in 0..m2 {
                let pos = s + 12 + 5 * m1 + 6 * i + 4;
                sum += get_u16_at_pos(&self.x, pos) as usize;
            }
            for i in 0..m4 {
                let pos = s + 12 + 5 * m1 + 6 * m2 + 8 * i + 4;
                sum += get_u32_at_pos(&self.x, pos) as usize;
            }
        }
        sum
    }

    pub fn sum_of_col(&self, col: usize) -> usize {
        let mut sum = 0;
        if self.storage_version() == 0 {
            for row in 0..self.nrows() {
                let s = self.start_of_row(row);
                let m1 = get_u16_at_pos(&self.x, s) as usize;
                let m2 = get_u16_at_pos(&self.x, s + 2) as usize;
                let m4 = get_u16_at_pos(&self.x, s + 4) as usize;
                for i in 0..m1 {
                    let pos = s + 6 + 3 * i;
                    let f = get_u16_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u8_at_pos(&self.x, pos + 2) as usize;
                    }
                }
                for i in 0..m2 {
                    let pos = s + 6 + 3 * m1 + 4 * i;
                    let f = get_u16_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u16_at_pos(&self.x, pos + 2) as usize;
                    }
                }
                for i in 0..m4 {
                    let pos = s + 6 + 3 * m1 + 4 * m2 + 6 * i;
                    let f = get_u16_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u32_at_pos(&self.x, pos + 2) as usize;
                    }
                }
            }
        } else {
            for row in 0..self.nrows() {
                let s = self.start_of_row(row);
                let m1 = get_u32_at_pos(&self.x, s) as usize;
                let m2 = get_u32_at_pos(&self.x, s + 4) as usize;
                let m4 = get_u32_at_pos(&self.x, s + 8) as usize;
                for i in 0..m1 {
                    let pos = s + 12 + 5 * i;
                    let f = get_u32_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u8_at_pos(&self.x, pos + 4) as usize;
                    }
                }
                for i in 0..m2 {
                    let pos = s + 12 + 5 * m1 + 6 * i;
                    let f = get_u32_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u16_at_pos(&self.x, pos + 4) as usize;
                    }
                }
                for i in 0..m4 {
                    let pos = s + 12 + 5 * m1 + 6 * m2 + 8 * i;
                    let f = get_u32_at_pos(&self.x, pos) as usize;
                    if f == col {
                        sum += get_u32_at_pos(&self.x, pos + 4) as usize;
                    }
                }
            }
        }
        sum
    }

    pub fn value(&self, row: usize, col: usize) -> usize {
        let s = self.start_of_row(row);
        if self.storage_version() == 0 {
            let m1 = get_u16_at_pos(&self.x, s) as usize;
            let m2 = get_u16_at_pos(&self.x, s + 2) as usize;
            let m4 = get_u16_at_pos(&self.x, s + 4) as usize;
            for i in 0..m1 {
                let pos = s + 6 + 3 * i;
                let f = get_u16_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u8_at_pos(&self.x, pos + 2) as usize;
                }
            }
            for i in 0..m2 {
                let pos = s + 6 + 3 * m1 + 4 * i;
                let f = get_u16_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u16_at_pos(&self.x, pos + 2) as usize;
                }
            }
            for i in 0..m4 {
                let pos = s + 6 + 3 * m1 + 4 * m2 + 6 * i;
                let f = get_u16_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u32_at_pos(&self.x, pos + 2) as usize;
                }
            }
            0
        } else {
            let m1 = get_u32_at_pos(&self.x, s) as usize;
            let m2 = get_u32_at_pos(&self.x, s + 4) as usize;
            let m4 = get_u32_at_pos(&self.x, s + 8) as usize;
            for i in 0..m1 {
                let pos = s + 12 + 5 * i;
                let f = get_u32_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u8_at_pos(&self.x, pos + 4) as usize;
                }
            }
            for i in 0..m2 {
                let pos = s + 12 + 5 * m1 + 6 * i;
                let f = get_u32_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u16_at_pos(&self.x, pos + 4) as usize;
                }
            }
            for i in 0..m4 {
                let pos = s + 12 + 5 * m1 + 6 * m2 + 8 * i;
                let f = get_u32_at_pos(&self.x, pos) as usize;
                if f == col {
                    return get_u32_at_pos(&self.x, pos + 4) as usize;
                }
            }
            0
        }
    }
}

impl Default for MirrorSparseMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    // test with: cargo test -p mirror_sparse_matrix  -- --nocapture

    use super::*;
    use io_utils::printme;
    use pretty_trace::PrettyTrace;

    #[test]
    fn test_mirror_sparse_matrix() {
        PrettyTrace::new().on();
        // Dated comment:
        // We have observed that with cargo test --release, tracebacks here can be incomplete,
        // and indeed this is true even if one doesn't use pretty trace.  In such cases, running
        // without --release works.
        // Really should document this example in pretty_trace.
        for storage_version in 0..2 {
            printme!(storage_version);
            let mut x = Vec::<Vec<(i32, i32)>>::new();
            let (n, k) = (10, 100);
            for i in 0..n {
                let mut y = Vec::<(i32, i32)>::new();
                for j in 0..k {
                    let col: usize;
                    if storage_version == 0 {
                        col = i + j;
                    } else {
                        col = 10000 * i + j;
                    }
                    y.push((col as i32, (i * i * j) as i32));
                }
                x.push(y);
            }
            let test_row = 9;
            let mut row_sum = 0;
            for j in 0..x[test_row].len() {
                row_sum += x[test_row][j].1 as usize;
            }
            let (mut row_labels, mut col_labels) = (Vec::<String>::new(), Vec::<String>::new());
            for i in 0..n {
                row_labels.push(format!("row {}", i));
            }
            for j in 0..k {
                col_labels.push(format!("col {}", j));
            }
            let y = MirrorSparseMatrix::build_from_vec(&x, &row_labels, &col_labels);
            let row_sum2 = y.sum_of_row(test_row);
            assert_eq!(row_sum, row_sum2);
            let test_col;
            if storage_version == 0 {
                test_col = 15;
            } else {
                test_col = 90001;
            }
            let mut col_sum = 0;
            for i in 0..x.len() {
                for j in 0..x[i].len() {
                    assert_eq!(x[i][j].1 as usize, y.value(i, x[i][j].0 as usize));
                    if x[i][j].0 as usize == test_col {
                        col_sum += x[i][j].1 as usize;
                    }
                }
            }
            let col_sum2 = y.sum_of_col(test_col);
            printme!(col_sum, col_sum2);
            assert_eq!(col_sum, col_sum2);
            assert_eq!(y.storage_version(), storage_version);
            assert_eq!(y.row_label(5), row_labels[5]);
            assert_eq!(y.col_label(7), col_labels[7]);
        }
    }
}
