// Copyright (c) 2018 10x Genomics, Inc. All rights reserved.

// Functions print_tabular and print_tabular_vbox for making pretty tables.  And related utilities.

extern crate io_utils;
extern crate itertools;
extern crate string_utils;

use io_utils::*;
use itertools::Itertools;
use std::cmp::{max, min};
use string_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Package characters with ANSI escape codes that come before them.

pub fn package_characters_with_escapes(c: &Vec<u8>) -> Vec<Vec<u8>> {
    let mut x = Vec::<Vec<u8>>::new();
    let mut escaped = false;
    let mut package = Vec::<u8>::new();
    for b in c.iter() {
        if escaped && *b != b'm' {
            package.push(*b);
        } else if *b == b'' {
            escaped = true;
            package.push(*b);
        } else if escaped && *b == b'm' {
            escaped = false;
            package.push(*b);
        } else {
            package.push(*b);
            x.push(package.clone());
            package.clear();
        }
    }
    x
}

fn package_characters_with_escapes_char(c: &Vec<char>) -> Vec<Vec<char>> {
    let mut x = Vec::<Vec<char>>::new();
    let mut escaped = false;
    let mut package = Vec::<char>::new();
    for b in c.iter() {
        if escaped && *b != 'm' {
            package.push(*b);
        } else if *b == '' {
            escaped = true;
            package.push(*b);
        } else if escaped && *b == 'm' {
            escaped = false;
            package.push(*b);
        } else {
            package.push(*b);
            x.push(package.clone());
            package.clear();
        }
    }
    x
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Print out a matrix, with left-justified entries, and given separation between
// columns.  (Justification may be changed by supplying an optional argument
// consisting of a string of l's and r's.)

pub fn print_tabular(
    log: &mut Vec<u8>,
    rows: &Vec<Vec<String>>,
    sep: usize,
    justify: Option<Vec<u8>>,
) {
    let just = match justify {
        Some(x) => x,
        None => Vec::<u8>::new(),
    };
    let nrows = rows.len();
    let mut ncols = 0;
    for i in 0..nrows {
        ncols = max(ncols, rows[i].len());
    }
    let mut maxcol = vec![0; ncols];
    for i in 0..rows.len() {
        for j in 0..rows[i].len() {
            maxcol[j] = max(maxcol[j], rows[i][j].len());
        }
    }
    for i in 0..rows.len() {
        for j in 0..rows[i].len() {
            let x = rows[i][j].clone();
            if j < just.len() && just[j] == b'r' {
                log.append(&mut vec![b' '; maxcol[j] - x.len()]);
                log.append(&mut x.as_bytes().to_vec());
                if j < rows[i].len() - 1 {
                    log.append(&mut vec![b' '; sep]);
                }
            } else {
                log.append(&mut x.as_bytes().to_vec());
                if j < rows[i].len() - 1 {
                    log.append(&mut vec![b' '; maxcol[j] - x.len() + sep]);
                }
            }
        }
        log.push(b'\n');
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Compute the visible length of a string, counting unicode characters as width one and
// ignoring some ASCII escape sequences.

pub fn visible_width(s: &str) -> usize {
    let mut n = 0;
    let mut escaped = false;
    for c in s.chars() {
        if escaped && c != 'm' {
        } else if c == '' {
            escaped = true;
        } else if escaped && c == 'm' {
            escaped = false;
        } else {
            n += 1;
        }
    }
    n
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Print out a matrix, with given separation between columns.  Rows of the matrix
// may contain arbitrary UTF-8 and some escape sequences.  Put the entire thing in a box, with
// extra vertical bars.  The argument justify consists of symbols l and r, denoting
// left and right justification for given columns, respectively, and the symbol | to
// denote a vertical bar.
//
// There is no separation printed on the far left or far right.
//
// By a "matrix entry", we mean one of the Strings in "rows".
//
// Entries that begin with a backslash are reserved for future features.
// Symbols other than l or r or | in "justify" are reserved for future features.
//
// An entry may be followed on the right by one more entries whose contents are
// exactly "\ext".  In that case the entries are treated as multi-column.  Padding
// is inserted as needed on the "right of the multicolumn".
//
// An entry may be "\hline", which gets you a horizontal line.  The normal use case is to
// use one or more of these in succession horizontally to connect two vertical lines.  Cannot
// be combined with \ext.
//
// Really only guaranteed to work for the tested cases.

pub fn print_tabular_vbox(
    log: &mut String,
    rows: &Vec<Vec<String>>,
    sep: usize,
    justify: &Vec<u8>,
    debug_print: bool,
) {
    let mut rrr = rows.clone();
    let nrows = rrr.len();
    let mut ncols = 0;
    for i in 0..nrows {
        ncols = max(ncols, rrr[i].len());
    }
    let mut vert = vec![false; ncols];
    let mut just = Vec::<u8>::new();
    let mut count = 0 as isize;
    for i in 0..justify.len() {
        if justify[i] == b'|' {
            assert!(count > 0);
            if count >= ncols as isize {
                eprintln!("\nposition of | in justify string is illegal");
                eprintme!(count, ncols);
            }
            assert!(count < ncols as isize);
            vert[(count - 1) as usize] = true;
        } else {
            just.push(justify[i]);
            count += 1;
        }
    }
    if just.len() != ncols {
        eprintln!(
            "\nError.  Your table has {} columns but the number of \
             l or r symbols in justify is {}.\nThese numbers should be equal.",
            ncols,
            just.len()
        );
        eprintln!("justify = {}", strme(&justify));
        assert_eq!(just.len(), ncols);
    }
    let mut maxcol = vec![0; ncols];
    let mut ext = vec![0; ncols];
    for i in 0..rrr.len() {
        for j in 0..rrr[i].len() {
            if j < rrr[i].len() - 1 && rrr[i][j + 1] == "\\ext".to_string() {
                continue;
            }
            if rrr[i][j] == "\\ext".to_string() || rrr[i][j] == "\\hline".to_string() {
                continue;
            }
            maxcol[j] = max(maxcol[j], visible_width(&rrr[i][j]));
        }
    }
    if debug_print {
        println!("maxcol = {}", maxcol.iter().format(","));
    }

    // Add space according to ext entries.

    for i in 0..rrr.len() {
        for j in 0..rrr[i].len() {
            if j < rrr[i].len() - 1
                && rrr[i][j + 1] == "\\ext".to_string()
                && rrr[i][j] != "\\ext".to_string()
            {
                let mut k = j + 1;
                while k < rrr[i].len() {
                    if rrr[i][k] != "\\ext".to_string() {
                        break;
                    }
                    k += 1;
                }
                let need = visible_width(&rrr[i][j]);
                let mut have = 0;
                for l in j..k {
                    have += maxcol[l];
                    if l < k - 1 {
                        have += sep;
                        if vert[l] {
                            have += sep + 1;
                        }
                    }
                }
                if debug_print {
                    println!("row {} column {}, have = {}, need = {}", i, j, have, need);
                }
                if have > need {
                    if debug_print {
                        println!(
                            "adding {} spaces to right of row {} col {}",
                            have - need,
                            i,
                            j
                        );
                    }
                    for _ in need..have {
                        rrr[i][j].push(' ');
                    }
                } else if need > have {
                    maxcol[k - 1] += need - have;
                    if debug_print {
                        println!("increasing maxcol[{}] to {}", k - 1, maxcol[k - 1]);
                    }
                    ext[k - 1] += need - have;
                }
                let mut m = 0;
                for u in 0..rrr.len() {
                    if rrr[u][j] != "\\ext".to_string() {
                        m = max(m, visible_width(&rrr[u][j]));
                    }
                }
                if m > visible_width(&rrr[i][j]) {
                    for _ in visible_width(&rrr[i][j])..m {
                        rrr[i][j].push(' ');
                    }
                }
            }
        }
    }

    // Create top boundary of table.

    log.push('┌');
    for i in 0..ncols {
        let mut n = maxcol[i];
        if i < ncols - 1 {
            n += sep;
        }
        for _ in 0..n {
            log.push('─');
        }
        if vert[i] {
            log.push('┬');
            for _ in 0..sep {
                log.push('─');
            }
        }
    }
    log.push('┐');
    log.push('\n');

    // Go through the rows.

    for i in 0..nrows {
        if debug_print {
            println!("now row {} = {}", i, rrr[i].iter().format(","));
            println!("0 - pushing │ onto row {}", i);
        }
        log.push('│');
        for j in 0..min(ncols, rrr[i].len()) {
            // Pad entries according to justification.

            let mut x = String::new();
            if j >= rrr[i].len() {
                for _ in 0..maxcol[j] {
                    x.push(' ');
                }
            } else if rrr[i][j] == "\\hline".to_string() {
                for _ in 0..maxcol[j] {
                    x.push('─');
                }
            } else {
                let r = rrr[i][j].clone();
                let rlen = visible_width(&r);
                let mut xlen = 0;
                if r != "\\ext".to_string() {
                    if just[j] == b'r' {
                        for _ in rlen..(maxcol[j] - ext[j]) {
                            x.push(' ');
                            xlen += 1;
                        }
                    }
                    if j < rrr[i].len() {
                        x += &r;
                        xlen += visible_width(&r);
                    }
                    if just[j] == b'r' {
                        for _ in (maxcol[j] - ext[j])..maxcol[j] {
                            x.push(' ');
                            xlen += 1;
                        }
                    }
                    if just[j] == b'l' {
                        for _ in xlen..maxcol[j] {
                            x.push(' ');
                        }
                    }
                }
            }
            for c in x.chars() {
                log.push(c);
            }

            // Add separations and separators.

            let mut add_sep = true;
            if j + 1 < rrr[i].len() && rrr[i][j + 1] == "\\ext".to_string() {
                add_sep = false;
            }
            let mut jp = j;
            while jp + 1 < rrr[i].len() {
                if rrr[i][jp + 1] != "\\ext".to_string() {
                    break;
                }
                jp += 1;
            }
            if add_sep && jp < ncols - 1 {
                if rrr[i][j] == "\\hline".to_string() {
                    for _ in 0..sep {
                        log.push('─');
                    }
                } else {
                    for _ in 0..sep {
                        log.push(' ');
                    }
                }
            }
            if vert[j] && rrr[i][j + 1] != "\\ext" {
                if debug_print {
                    println!("1 - pushing │ onto row {}, j = {}", i, j);
                }
                log.push('│');
                if rrr[i][j + 1] == "\\hline".to_string() {
                    for _ in 0..sep {
                        log.push('─');
                    }
                } else {
                    for _ in 0..sep {
                        log.push(' ');
                    }
                }
            }
        }
        if debug_print {
            println!("2 - pushing │ onto row {}", i);
        }
        log.push('│');
        log.push('\n');
    }
    log.push('└');
    for i in 0..ncols {
        let mut n = maxcol[i];
        if i < ncols - 1 {
            n += sep;
        }
        for _ in 0..n {
            log.push('─');
        }
        if vert[i] {
            if rrr[rrr.len() - 1][i + 1] != "\\ext" {
                log.push('┴');
            } else {
                log.push('─');
            }
            for _ in 0..sep {
                log.push('─');
            }
        }
    }
    log.push('┘');
    log.push('\n');

    // Convert into a super-character vec of matrices.  There is one vector entry per line.
    // In each matrix, an entry is a super_character: a rust character, together with the escape
    // code characters that came before it.

    let mut mat = Vec::<Vec<Vec<char>>>::new();
    {
        let mut all = Vec::<Vec<char>>::new();
        let mut z = Vec::<char>::new();
        for c in log.chars() {
            if c != '\n' {
                z.push(c);
            } else {
                if !z.is_empty() {
                    all.push(z.clone());
                }
                z.clear();
            }
        }
        if !z.is_empty() {
            all.push(z);
        }
        for i in 0..all.len() {
            mat.push(package_characters_with_escapes_char(&all[i]));
        }
    }

    // "Smooth" edges of hlines.

    for i in 0..mat.len() {
        for j in 0..mat[i].len() {
            if j > 0
                && mat[i][j - 1] == vec!['─']
                && mat[i][j] == vec!['│']
                && j + 1 < mat[i].len()
                && mat[i][j + 1] == vec!['─']
                && i + 1 < mat.len()
                && j < mat[i + 1].len()
                && mat[i + 1][j] != vec!['│']
            {
                mat[i][j] = vec!['┴'];
            } else if j > 0
                && mat[i][j - 1] == vec!['─']
                && mat[i][j] == vec!['│']
                && j + 1 < mat[i].len()
                && mat[i][j + 1] == vec!['─']
            {
                mat[i][j] = vec!['┼'];
            } else if mat[i][j] == vec!['│'] && j + 1 < mat[i].len() && mat[i][j + 1] == vec!['─']
            {
                mat[i][j] = vec!['├'];
            } else if j > 0
                && mat[i][j - 1] == vec!['─']
                && mat[i][j] == vec!['│']
                && (j + 1 == mat[i].len() || mat[i][j + 1] != vec!['─'])
            {
                mat[i][j] = vec!['┤'];
            }
        }
    }

    // Output matrix.

    log.clear();
    for i in 0..mat.len() {
        for j in 0..mat[i].len() {
            for k in 0..mat[i][j].len() {
                log.push(mat[i][j][k]);
            }
        }
        log.push('\n');
    }

    // Finish.

    if debug_print {
        println!("");
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

#[cfg(test)]
mod tests {

    // run this test using:
    // cargo test -p tenkit2 test_print_tabular_vbox

    use crate::print_tabular_vbox;

    // (should add some escape codes)

    #[test]
    fn test_print_tabular_vbox() {
        // test 1

        let mut rows = Vec::<Vec<String>>::new();
        let row = vec![
            "omega".to_string(),
            "superduperfineexcellent".to_string(),
            "\\ext".to_string(),
        ];
        rows.push(row);
        let row = vec![
            "woof".to_string(),
            "snarl".to_string(),
            "octopus".to_string(),
        ];
        rows.push(row);
        let row = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        rows.push(row);
        let row = vec![
            "hiccup".to_string(),
            "tomatillo".to_string(),
            "ddd".to_string(),
        ];
        rows.push(row);
        let mut log = String::new();
        let mut justify = Vec::<u8>::new();
        justify.push(b'r');
        justify.push(b'|');
        justify.push(b'l');
        justify.push(b'l');
        print_tabular_vbox(&mut log, &rows, 2, &justify, false);
        let answer = "┌────────┬─────────────────────────┐\n\
                      │ omega  │  superduperfineexcellent│\n\
                      │  woof  │  snarl      octopus     │\n\
                      │     a  │  b          c           │\n\
                      │hiccup  │  tomatillo  ddd         │\n\
                      └────────┴─────────────────────────┘\n";
        if log != answer {
            println!("\ntest 1 failed");
            println!("\nyour answer:\n{}", log);
            println!("correct answer:\n{}", answer);
        }
        if log != answer {
            panic!();
        }

        // test 2

        let mut rows = Vec::<Vec<String>>::new();
        let row = vec!["pencil".to_string(), "pusher".to_string()];
        rows.push(row);
        let row = vec!["\\hline".to_string(), "\\hline".to_string()];
        rows.push(row);
        let row = vec!["fabulous pumpkins".to_string(), "\\ext".to_string()];
        rows.push(row);
        let mut log = String::new();
        let mut justify = Vec::<u8>::new();
        justify.push(b'l');
        justify.push(b'|');
        justify.push(b'l');
        print_tabular_vbox(&mut log, &rows, 2, &justify, false);
        let answer = "┌────────┬────────┐\n\
                      │pencil  │  pusher│\n\
                      ├────────┴────────┤\n\
                      │fabulous pumpkins│\n\
                      └─────────────────┘\n";
        if log != answer {
            println!("\ntest 2 failed");
            println!("\nyour answer:\n{}", log);
            println!("correct answer:\n{}", answer);
        }
        if log != answer {
            panic!();
        }
    }
}
