// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.
//
// Emit a color-blind-friendly ANSI color escape sequence, as indicated below,
// and a color assignment defined by the code:
//
// s  code  content                   ideal color
// 0  75  = rgb(95,175,255)  close to (86,180,233)  bold + light blue
// 1  166 = rgb(215,95,0)    close to (213,94,0)    bold + vermillion
// 2  178 = rgb(215,175,0)   close to (230,159,0)   orange
// 3  25  = rgb(0,95,175)    close to (0,114,178)   bold + blue
// 4  175 = rgb(215,135,175) close to (204,121,167) bold + reddish purple
// 5  36  = rgb(0,175,135)   close to (0,158,115)   bluish green
//
// sources:
// https://jonasjacek.github.io/colors (256 color RGB values) -- should get better URL
// https://en.wikipedia.org/wiki/ANSI_escape_code (8 bit color escape codes)
// http://mkweb.bcgsc.ca/colorblind/img/colorblindness.palettes.trivial.png (good color palette)
// which refers to Wong, B. (2011) Points of View: Color Blindness.  Nature Methods 8:441.

pub fn print_color(s: usize, log: &mut Vec<u8>) {
    assert!(s <= 6);
    if s == 0 {
        log.append(&mut b"[01m[38;5;75m".to_vec());
    } else if s == 1 {
        log.append(&mut b"[01m[38;5;166m".to_vec());
    } else if s == 2 {
        // At one point this was made bold, which makes it more readable when printed, but
        // it's uglier if bold and overall contrast is reduced.
        log.append(&mut b"[38;5;178m".to_vec());
    } else if s == 3 {
        log.append(&mut b"[01m[38;5;25m".to_vec());
    } else if s == 4 {
        log.append(&mut b"[01m[38;5;175m".to_vec());
    } else {
        log.append(&mut b"[38;5;36m".to_vec());
    }
}

// Return a color order that is optimized in such a way that e.g. colors 0, 1, 2 are the best
// subset, and so that adjacent colors are maximally different (as best we could arrange them).

pub fn best_color_order(i: usize) -> usize {
    if i == 0 {
        3
    } else if i == 1 {
        4
    } else if i == 2 {
        5
    } else if i == 3 {
        1
    } else if i == 4 {
        0
    } else {
        2
    }
}

// Miscellaneous escape codes.

pub fn emit_red_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[01;31m".to_vec());
}

pub fn emit_blue_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[38;5;12m".to_vec());
}

pub fn emit_green_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[01;32m".to_vec());
}

pub fn emit_bold_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[01m".to_vec());
}

pub fn emit_end_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[0m".to_vec());
}

pub fn bold(s: &str) -> String {
    format!("[01m{}[0m", s)
}

pub fn emit_eight_bit_color_escape(log: &mut Vec<u8>, c: usize) {
    log.append(&mut b"[38;5;".to_vec());
    log.append(&mut format!("{}", c).as_bytes().to_vec());
    log.push(b'm');
}

pub fn emit_disable_alternate_screen_buffer_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[?1049l".to_vec());
}
