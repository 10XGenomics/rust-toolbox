// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// Various utilties for getting pipeline information.

extern crate io_utils;
extern crate string_utils;

use io_utils::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::Command,
    thread, time,
};
use string_utils::*;

// Find path of the pipestance associated to a given lena id.  Iterates to
// overcome sporadic failures for the marsoc URL.

pub fn pipestance(lena: &String) -> String {
    let mut m = String::new();
    let nreps = 5;
    let sleeptime = 10;
    for rep in 0..nreps {
        let o = Command::new("csh")
            .arg("-c")
            .arg(format!(
                "curl http://marsoc.fuzzplex.com/pipestance/{}",
                lena
            ))
            .output()
            .expect("failed to execute marsoc http");
        m = String::from_utf8(o.stdout).unwrap();
        if !m.contains("502 Bad Gateway") {
            break;
        }
        if rep < nreps - 1 {
            thread::sleep(time::Duration::from_millis(sleeptime));
            continue;
        }
        panic!(
            "502 Bad Gateway from curl http://marsoc.fuzzplex.com/pipestance/{}",
            lena
        );
    }
    m
}

// Return path to head pipestance.

pub fn pipestance_head(lena: &String) -> String {
    let _ = lena.parse::<usize>().expect(&format!(
        "you passed \"{}\" to pipestance_head, and that's not a lena id",
        lena
    ));
    let m = pipestance(&lena);
    format!(
        "/mnt/analysis/marsoc/pipestances{}/HEAD",
        m.between("pipestance", "\"")
    )
}

// Return path to outs directory for a given lena id.

pub fn get_outs(lena: &String) -> String {
    let _ = lena.parse::<usize>().expect(&format!(
        "you passed \"{}\" to get_outs, and that's not a lena id",
        lena
    ));
    format!("{}/outs", pipestance_head(&lena))
}

// Return read path for a given lena id.  Return empty string on failure.

pub fn pipestance_read_path(pipestance_head: &String) -> String {
    let invocation = format!("{}/_invocation", &pipestance_head);
    if !path_exists(&invocation) {
        return "".to_string();
    }
    let f = open_for_read![&invocation];
    for line in f.lines() {
        let s = line.unwrap();
        if s.contains("\"read_path\": ") {
            return s.after("\"read_path\": ").between("\"", "\"").to_string();
        }
    }
    "".to_string()
}

pub fn pipestance_read_path_exists(pipestance_head: &String) -> bool {
    let rp = pipestance_read_path(&&pipestance_head);
    rp != "".to_string() && path_exists(&rp)
}

// Get the total cpu hours used by a martian pipeline.  This is obtained from the
// _perf file.  The thing that is labeled cpu_hours in _perf is actually wall clock
// time, so we don't report that.  Instead we report the thing that is labeled
// usertime.  We find the first instance of usertime in the file.  This SHOULD be
// the total, but conceivably it's not, and if we ever find such an instance, we
// would need to change this code.
//
// Return -1 if we don't find the stat.

pub fn total_cpu_hours(dir: &str) -> f64 {
    let perf = format!("{}/_perf", dir);
    if path_exists(&perf) {
        let f = open_for_read![&perf];
        for pline in f.lines() {
            let t = pline.unwrap();
            if t.contains("usertime") {
                return t.between(": ", ",").force_f64() / 3600.0;
            }
        }
    }
    return -1.0;
}

// Get the end to end wallclock hours used by a martian pipeline.  This is obtained
// from the _perf file.  We find the first instance of walltime in the file.  This
// SHOULD be the total, but conceivably it's not, and if we ever find such an
// instance, we would need to change this code.
//
// Return -1 if we don't find the stat.

pub fn total_wall_hours(dir: &str) -> f64 {
    let perf = format!("{}/_perf", dir);
    if path_exists(&perf) {
        let f = open_for_read![&perf];
        for pline in f.lines() {
            let t = pline.unwrap();
            if t.contains("walltime") {
                return t.between(": ", ",").force_f64() / 3600.0;
            }
        }
    }
    return -1.0;
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct PipelineInfo {
    pub pipeline_name: String,
    pub pipestance_dir: String,
    pub descrip: String,
}

// Make a sorted list of all complete pipestances on marsoc, returning the
// lena id, pipeline name, pipeline directory, and description.

pub fn get_all_pipestances() -> Vec<(i32, PipelineInfo)> {
    let mut info = Vec::<(i32, PipelineInfo)>::new();
    // ◼: doing this via a command is ugly and somewhat dangerous
    let o = Command::new("csh")
        .arg("-c")
        .arg(format!(
            "curl http://marsoc.fuzzplex.com/api/get-pipestances | gunzip"
        ))
        .output()
        .expect("failed to execute curl");
    let marsoc = o.stdout;
    let mut i = 1;
    while i < marsoc.len() {
        assert_eq!(marsoc[i], b'{');
        let mut j = i + 1;
        while j < marsoc.len() {
            if marsoc[j] == b'}' {
                break;
            }
            j += 1;
        }
        assert!(j < marsoc.len());
        let fields: Vec<&[u8]> = marsoc[i + 1..j]
            .split(|x| *x == b',')
            .collect::<Vec<&[u8]>>();
        let mut lena = -1 as i32;
        let mut complete = false;
        let mut fc = String::new();
        let mut descrip = String::new();
        let mut pipeline = String::new();
        for l in 0..fields.len() {
            let f = stringme(&fields[l].to_vec());
            if f == "\"state\":\"complete\"" {
                complete = true;
            }
            if f.contains("\"psid\":\"") {
                let m = f.after("\"psid\":\"");
                if m.contains("\"") {
                    let x = m.before("\"");
                    if x.parse::<i32>().is_ok() {
                        lena = x.force_i32();
                    }
                }
            }
            if f.contains("\"fcid\":\"") {
                let m = f.after("\"fcid\":\"");
                if m.contains("\"") {
                    fc = m.before("\"").to_string();
                }
            }
            if f.contains("\"name\":\"") {
                let m = f.after("\"name\":\"");
                if m.contains("\"") {
                    descrip = m.before("\"").to_string();
                }
            }
            if f.contains("\"pipeline\":\"") {
                let m = f.after("\"pipeline\":\"");
                if m.contains("\"") {
                    pipeline = m.before("\"").to_string();
                }
            }
        }
        if complete && lena >= 0 {
            let dir = format!(
                "/mnt/analysis/marsoc/pipestances/{}/{}/{}/HEAD",
                fc, pipeline, lena
            );
            info.push((
                lena,
                PipelineInfo {
                    pipeline_name: pipeline,
                    pipestance_dir: dir,
                    descrip: descrip,
                },
            ));
        }
        j += 1;
        if j == marsoc.len() {
            break;
        }
        if marsoc[j] == b']' {
            break;
        }
        assert_eq!(marsoc[j], b',');
        j += 1;
        i = j;
    }
    info.sort();
    info
}
