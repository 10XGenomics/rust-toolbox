// Copyright (c) 2021 10x Genomics, Inc. All rights reserved.

// This file contains miscellaneous utilities for input and output.

use bincode::{deserialize_from, serialize_into};
use flate2::read::MultiGzDecoder;
use serde::{de::DeserializeOwned, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader};
use std::{fmt::Debug, fs::File, io::prelude::*, path::Path};
use string_utils::TextUtils;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// GET CONTENTS OF DIRECTORY
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn dir_list(d: &str) -> Vec<String> {
    let x = fs::read_dir(d).unwrap_or_else(|_| panic!("failed to read directory {}", d));
    let mut y = Vec::<String>::new();
    for f in x {
        let s: String = f.unwrap().file_name().into_string().unwrap();
        y.push(s);
    }
    y.sort();
    y
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// TEST FOR EXISTENCE OF FILE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn path_exists(p: impl AsRef<Path>) -> bool {
    p.as_ref().exists()
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// WRITE STUFF
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// fwriteln! is just like writeln! except that it has an expect call tacked on.
// fwrite! similar.

#[macro_export]
macro_rules! fwriteln {
    ($f:expr, $u:expr) => {
        writeln!( $f, $u ).expect("writeln! failed")
    };
    ($f:expr, $u:expr, $($x:tt)*) => {
        writeln!( $f, $u, $($x)* )
            .unwrap_or_else(|_| panic!( "writeln! failed while writing \"{}\"", $u ) )
    };
}

// fwrite! is just like write! except that it has an expect call tacked on.

#[macro_export]
macro_rules! fwrite {
    ($f:expr, $u:expr) => {
        write!( $f, $u ).expect( "write! failed" )
    };
    ($f:expr, $u:expr, $($x:tt)*) => {
        write!( $f, $u, $($x)* )
            .unwrap_or_else(|_| panic!( "write! failed while writing \"{}\"", $u ) )
    };
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// OPEN FILES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

#[macro_export]
macro_rules! open_for_read {
    ($filename:expr) => {
        ::std::io::BufReader::new(
            ::std::fs::File::open(::core::convert::AsRef::<::std::path::Path>::as_ref(
                $filename,
            ))
            .unwrap_or_else(|_| {
                panic!(
                    "Could not open file \"{}\"",
                    ::core::convert::AsRef::<::std::path::Path>::as_ref($filename)
                        .to_string_lossy(),
                )
            }),
        )
    };
}

pub fn open_userfile_for_read(f: impl AsRef<Path>) -> BufReader<File> {
    let f = f.as_ref();
    let g = File::open(f);
    if g.is_err() && g.as_ref().err().unwrap().kind() == std::io::ErrorKind::PermissionDenied {
        eprintln!(
            "\nCould not open file \n{}\nfor reading because you \
            lack read permission on this file.\n",
            f.to_string_lossy()
        );
        std::process::exit(1);
    }
    BufReader::new(g.unwrap_or_else(|_| panic!("Could not open file \"{}\"", f.to_string_lossy())))
}

#[macro_export]
macro_rules! open_for_write_new {
    ($filename:expr) => {
        ::std::io::BufWriter::new(
            ::std::fs::File::create(::core::convert::AsRef::<::std::path::Path>::as_ref(
                $filename,
            ))
            .unwrap_or_else(|_| {
                panic!(
                    "Could not create file \"{}\"",
                    ::core::convert::AsRef::<::std::path::Path>::as_ref($filename)
                        .to_string_lossy()
                )
            }),
        )
    };
}

pub fn open_lz4<P: AsRef<Path>>(filename: P) -> lz4::Decoder<File> {
    let f = File::open(filename).expect("Failed to open file for reading");
    lz4::Decoder::new(f).expect("Failed to create lz4 decoder")
}

// If you accidentally pass a gzipped file to this it will succeed in opening the file,
// but then when you try to run read_line, the read will return !is_ok().  This seems horrible.

pub fn open_maybe_compressed<P: AsRef<Path>>(filename: P) -> Box<dyn Read> {
    match filename.as_ref().extension().and_then(OsStr::to_str) {
        Some("lz4") => Box::new(open_lz4(filename)) as Box<dyn Read>,
        _ => Box::new(File::open(filename).expect("Failed to open file for reading"))
            as Box<dyn Read>,
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// READ A FILE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// read_maybe_unzipped( f, lines ): The filename f should have the form x.gz.
// If that exists, load it into lines.  Otherwise try to load x.

pub fn read_maybe_unzipped(f: impl AsRef<Path>, lines: &mut Vec<String>) {
    let f = f.as_ref();
    lines.clear();
    if path_exists(f) {
        let gz = MultiGzDecoder::new(File::open(f).unwrap());
        let b = BufReader::new(gz);
        for line in b.lines() {
            lines.push(line.unwrap());
        }
    } else {
        let g = f.with_extension("");
        let g = g.as_path();
        if path_exists(g) {
            let b = BufReader::new(File::open(g).unwrap());
            for line in b.lines() {
                lines.push(line.unwrap());
            }
        } else {
            panic!(
                "Could not find {} or {}.",
                f.to_string_lossy(),
                g.to_string_lossy()
            );
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// CODE TO DO READS AND WRITES USING SERDE
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn write_obj<T: Serialize, P: AsRef<Path> + Debug>(g: &T, filename: P) {
    let f = match std::fs::File::create(&filename) {
        Err(err) => panic!("couldn't create file {:?}: {}", filename, err),
        Ok(f) => f,
    };
    let mut writer = std::io::BufWriter::new(f);
    serialize_into(&mut writer, &g)
        .unwrap_or_else(|_| panic!("write_obj of file {:?} failed", filename))
}

pub fn read_obj<T: DeserializeOwned, P: AsRef<Path> + Debug>(filename: P) -> T {
    let f = match std::fs::File::open(&filename) {
        Err(err) => panic!("couldn't open file {:?}: {}", filename, err),
        Ok(f) => f,
    };
    let mut reader = std::io::BufReader::new(f);
    deserialize_from(&mut reader)
        .unwrap_or_else(|_| panic!("read_obj of file {:?} failed", filename))
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PRINT MACRO
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Print a list of things, useful for debugging.
// Example: if x is 3 and y is y, then printme!(x,y) yields
// x = 3, y = 7,

#[allow(unused_macros)]
#[macro_export]
macro_rules! printme {
        ( $( $x:expr ),* ) => {
            println!(concat!( $( stringify!($x), " = {}, ", )* ), $($x,)*)
        }
    }

#[allow(unused_macros)]
#[macro_export]
macro_rules! eprintme {
        ( $( $x:expr ),* ) => {
            eprintln!(concat!( $( stringify!($x), " = {}, ", )* ), $($x,)*)
        }
    }

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// GET METRIC VALUES
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Get the value of a metric from a json file or similar.  Returns a string.
// Removes outer quotes if present.  Panics if file not found, and returns empty
// string if the metric is not found.

pub fn get_metric_value(f: impl AsRef<Path>, metric: &str) -> String {
    let buf = open_for_read![&f];
    for line in buf.lines() {
        let s = line.unwrap();
        let metric_string = format!("\"{}\": ", metric);
        if s.contains(&metric_string) {
            let mut t = s.after(&metric_string).to_string();
            if t.ends_with(' ') {
                t.pop();
            }
            if t.ends_with(',') {
                t.pop();
            }
            if t.ends_with(".0") {
                t.pop();
                t.pop();
            }
            if t.starts_with('\"') && t.ends_with('\"') {
                t = t[1..t.len() - 1].to_string();
            }
            return t.to_string();
        }
    }
    String::default()
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// CODE FOR STREAMING A JSON VECTOR
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Read an entry from a json file that represents a vector.  This is not completely
// general as it depends on assumptions about the formatting of the file.
//
// To compare to and probably replace with:
// https://martian-lang.github.io/martian-rust/doc/martian_filetypes/json_file/
// index.html#lazy-readwrite-example

pub fn read_vector_entry_from_json<R: BufRead>(json: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut line = String::new();
    if json.read_line(&mut line).is_err() || line == *"" || line == *"[]" {
        return Ok(None);
    }
    if line == *"[\n" {
        line.clear();
        if json.read_line(&mut line).is_err() {
            return Err(
                "\nProblem reading json file, probably due to a defect in it.\n".to_string(),
            );
        }
    }
    let mut entry = Vec::<u8>::new();
    let (mut curlies, mut bracks, mut quotes) = (0_isize, 0_isize, 0_isize);
    let mut s = line.as_bytes();
    loop {
        if (s == b"]" || s == b"]\n") && curlies == 0 && bracks == 0 && quotes % 2 == 0 {
            if !entry.is_empty() {
                return Ok(Some(entry));
            } else {
                return Ok(None);
            }
        }
        let mut cpos = -1_isize;
        if s.is_empty() {
            return Err("\nError reading json file.  It is possible that the file \
                was truncated.\n"
                .to_string());
        }
        for i in (0..s.len() - 1).rev() {
            if s[i] == b',' {
                cpos = i as isize;
                break;
            }
            if s[i] != b' ' {
                break;
            }
        }
        let mut escaped = false;
        for i in 0..s.len() {
            if !escaped && s[i] == b'"' {
                quotes += 1;
            } else if !escaped && quotes % 2 == 0 {
                match s[i] {
                    b'{' => curlies += 1,
                    b'}' => curlies -= 1,
                    b'[' => bracks += 1,
                    b']' => bracks -= 1,
                    b',' => {
                        if i as isize == cpos && curlies == 0 && bracks == 0 && quotes % 2 == 0 {
                            return Ok(Some(entry));
                        }
                    }
                    _ => {}
                };
            }
            if s[i] == b'\\' && !escaped {
                escaped = true;
            } else {
                escaped = false;
            }
            entry.push(s[i]);
        }
        line.clear();
        if json.read_line(&mut line).is_err() {
            return Err("\nSomething appears to be defective in a json file.\n".to_string());
        }
        s = line.as_bytes();
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// READ FILE TO STRING AND PRINT FILE NAME IF IT DOESN'T EXIST
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn read_to_string_safe<P: AsRef<Path>>(path: P) -> String {
    fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "Could not open file \"{}\".",
            path.as_ref().to_str().unwrap()
        )
    })
}
