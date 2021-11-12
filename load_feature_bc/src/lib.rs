// Copyright (c) 2019 10x Genomics, Inc. All rights reserved.

// This file contains 10x-specific stuff for working with cellranger runs.

use flate2::read::MultiGzDecoder;
use io_utils::{open_for_read, read_maybe_unzipped};
use std::path::Path;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use std::{i32, str, usize};
use string_utils::TextUtils;

// Load the raw feature barcode matrix from a cellranger run.  This
// returns the features, the barcodes, and a sparse matrix, like this:
// gex_sparse_matrix[barcode_index] = { (feature_index, count) },
// where the indices are zero-based.

pub fn load_feature_bc_matrix(
    outs: impl AsRef<Path>,
    features: &mut Vec<String>,
    barcodes: &mut Vec<String>,
    gex_sparse_matrix: &mut Vec<Vec<(i32, i32)>>,
) {
    let outs = outs.as_ref();
    let mut dir = outs.join("raw_feature_bc_matrix");
    if dir.exists() {
        dir.set_file_name("raw_gene_bc_matrices_mex");
    }
    dir.push("GRCh38");
    if dir.exists() {
        dir.push("genes.tsv.gz");
    } else {
        dir.pop();
        dir.push("features.tsv.gz");
    }
    read_maybe_unzipped(&dir, features);
    dir.set_file_name("barcodes.tsv.gz");
    read_maybe_unzipped(&dir, barcodes);
    dir.set_file_name("matrix.mtx.gz");
    let mut matrix_file = dir;

    if matrix_file.exists() {
        let gz = MultiGzDecoder::new(File::open(&matrix_file).unwrap());
        _load_feature_bc_matrix(BufReader::new(gz), barcodes, gex_sparse_matrix);
    } else {
        matrix_file.set_extension("");
        _load_feature_bc_matrix(open_for_read![&matrix_file], barcodes, gex_sparse_matrix);
    };
    fn _load_feature_bc_matrix(
        f: impl BufRead,
        barcodes: &[String],
        gex_sparse_matrix: &mut Vec<Vec<(i32, i32)>>,
    ) {
        gex_sparse_matrix.resize_with(
            gex_sparse_matrix.len() + barcodes.len(),
            Vec::<(i32, i32)>::new,
        );
        for (line_num, line) in f.lines().enumerate() {
            let s = line.unwrap();
            if line_num > 2 {
                let fields = s.splitn(4, ' ').collect::<Vec<&str>>();
                let feature = fields[0].force_i32() - 1;
                let bc = fields[1].force_i32() - 1;
                let count = fields[2].force_i32();
                gex_sparse_matrix[bc as usize].push((feature, count));
            }
        }
    }
}
