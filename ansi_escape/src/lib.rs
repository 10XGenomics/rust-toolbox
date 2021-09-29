// Copyright (c) 2019 10X Genomics, Inc. All rights reserved.

pub mod ansi_to_html;

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
// 6  11                                            yellow
//
// sources:
// 1. https://jonasjacek.github.io/colors (256 color RGB values) -- should get better URL
// 2. https://en.wikipedia.org/wiki/ANSI_escape_code (8 bit color escape codes)
// 3. http://mkweb.bcgsc.ca/colorblind/img/colorblindness.palettes.trivial.png (good color palette)
//    which refers to Wong, B. (2011) Points of View: Color Blindness.  Nature Methods 8:441.
//    (URL was broken when last tested but article is publicly accessible.)

pub fn print_color(s: usize, log: &mut Vec<u8>) {
    assert!(s < 7);
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
    } else if s == 5 {
        log.append(&mut b"[38;5;36m".to_vec());
    } else {
        log.append(&mut b"[01m[38;5;11m".to_vec());
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
    } else if i == 5 {
        2
    } else {
        6
    }
}

// Return ANSI 256 color escape sequence.

pub fn ansi_256(n: usize) -> Vec<u8> {
    let mut x = b"[38;5;".to_vec();
    x.append(&mut format!("{}", n).as_bytes().to_vec());
    x.push(b'm');
    x
}

// A 13-color palette for color blindness =
// http://mkweb.bcgsc.ca/colorblind/img/colorblindness.palettes.8-12-13.pdf.
//
// The RGB codes are shown below.  However, not all closely matched the empirical mac RGB
// codes for ansi color escape sequences.  So we just picked an ANSI 256-color index
// that seemed visually close in comparison to the output of color_table.
//
// The function print_color13 gives the actual RGB codes.
//
// N  COLOR           R    G    B   ANSI
// ----------------------------------
// 0   black           0    0    0     0
// 1   teal blue       0  110  130    23
// 2   purple        130   20  160    53
// 3   blue            0   90  200    20
// 4   azure           0  160  250    33
// 5   pink          250  120  250   170
// 6   aqua           20  210  220     6
// 7   raspberry     170   10   60     1
// 8   green          10  155   75    28
// 9   vermillion    255  130   95   209
// 10  yellow        234  214   68   185
// 11  light green   160  250  130    84
// 12  banana mania  250  230  190   223

pub fn print_color13_ansi(s: usize, log: &mut Vec<u8>) {
    assert!(s < 13);
    if s == 0 {
        log.append(&mut ansi_256(0));
    } else if s == 1 {
        log.append(&mut ansi_256(23));
    } else if s == 2 {
        log.append(&mut ansi_256(53));
    } else if s == 3 {
        log.append(&mut ansi_256(20));
    } else if s == 4 {
        log.append(&mut ansi_256(33));
    } else if s == 5 {
        log.append(&mut ansi_256(170));
    } else if s == 6 {
        log.append(&mut ansi_256(6));
    } else if s == 7 {
        log.append(&mut ansi_256(1));
    } else if s == 8 {
        log.append(&mut ansi_256(28));
    } else if s == 9 {
        log.append(&mut ansi_256(209));
    } else if s == 10 {
        log.append(&mut ansi_256(185));
    } else if s == 11 {
        log.append(&mut ansi_256(84));
    } else if s == 12 {
        log.append(&mut ansi_256(223));
    }
}

pub fn print_color13(s: usize) -> (usize, usize, usize) {
    assert!(s < 13);
    if s == 0 {
        (0, 0, 0)
    } else if s == 1 {
        (0, 110, 130)
    } else if s == 2 {
        (130, 20, 160)
    } else if s == 3 {
        (0, 90, 200)
    } else if s == 4 {
        (0, 160, 250)
    } else if s == 5 {
        (250, 120, 250)
    } else if s == 6 {
        (20, 210, 220)
    } else if s == 7 {
        (170, 10, 60)
    } else if s == 8 {
        (10, 155, 75)
    } else if s == 9 {
        (255, 130, 95)
    } else if s == 10 {
        (234, 214, 68)
    } else if s == 11 {
        (160, 250, 130)
    } else {
        (250, 230, 190)
    }
}

// Miscellaneous escape codes.

pub fn emit_red_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[31m".to_vec());
}

pub fn emit_blue_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[38;5;12m".to_vec());
}

pub fn emit_green_escape(log: &mut Vec<u8>) {
    log.append(&mut b"[32m".to_vec());
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
