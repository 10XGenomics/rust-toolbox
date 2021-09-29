// Copyright (c) 2019 10x Genomics, Inc. All rights reserved.

// This file contains 10x-specific stuff for working with cellranger runs.

use flate2::read::MultiGzDecoder;
use io_utils::{open_for_read, path_exists, read_maybe_unzipped};
use std::{format, i32, str, usize};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use string_utils::TextUtils;

// Load the raw feature barcode matrix from a cellranger run.  This
// returns the features, the barcodes, and a sparse matrix, like this:
// gex_sparse_matrix[barcode_index] = { (feature_index, count) },
// where the indices are zero-based.

pub fn load_feature_bc_matrix(
    outs: &String,
    features: &mut Vec<String>,
    barcodes: &mut Vec<String>,
    gex_sparse_matrix: &mut Vec<Vec<(i32, i32)>>,
) {
    let mut dir = format!("{}/raw_feature_bc_matrix", outs);
    if !path_exists(&dir) {
        dir = format!("{}/raw_gene_bc_matrices_mex", outs);
    }
    let mut matrix_file = format!("{}/matrix.mtx.gz", dir);
    if path_exists(&format!("{}/GRCh38", dir)) {
        read_maybe_unzipped(&format!("{}/GRCh38/genes.tsv.gz", dir), features);
        read_maybe_unzipped(&format!("{}/GRCh38/barcodes.tsv.gz", dir), barcodes);
        matrix_file = format!("{}/GRCh38/matrix.mtx.gz", dir);
    } else {
        read_maybe_unzipped(&format!("{}/features.tsv.gz", dir), features);
        read_maybe_unzipped(&format!("{}/barcodes.tsv.gz", dir), barcodes);
    }

    // ◼ The duplication that follows is horrible.  There must be a better way.

    if path_exists(&matrix_file) {
        let gz = MultiGzDecoder::new(File::open(&matrix_file).unwrap());
        let f = BufReader::new(gz);
        for _i in 0..barcodes.len() {
            gex_sparse_matrix.push(Vec::<(i32, i32)>::new());
        }
        let mut line_count = 1;
        for line in f.lines() {
            let s = line.unwrap();
            if line_count > 3 {
                let fields = s.split(' ').collect::<Vec<&str>>();
                let feature = fields[0].force_i32() - 1;
                let bc = fields[1].force_i32() - 1;
                let count = fields[2].force_i32();
                gex_sparse_matrix[bc as usize].push((feature, count));
            }
            line_count += 1;
        }
    } else {
        let f = open_for_read![matrix_file.before(".gz")];
        for _i in 0..barcodes.len() {
            gex_sparse_matrix.push(Vec::<(i32, i32)>::new());
        }
        let mut line_count = 1;
        for line in f.lines() {
            let s = line.unwrap();
            if line_count > 3 {
                let fields = s.split(' ').collect::<Vec<&str>>();
                let feature = fields[0].force_i32() - 1;
                let bc = fields[1].force_i32() - 1;
                let count = fields[2].force_i32();
                gex_sparse_matrix[bc as usize].push((feature, count));
            }
            line_count += 1;
        }
    }
}
