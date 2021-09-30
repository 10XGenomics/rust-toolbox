// Copyright (c) 2020 10X Genomics, Inc. All rights reserved.

// Convert text containing ANSI escape codes to html.  There is a public function
// convert_text_with_ansi_escapes_to_html, and there is also a related public function
// compress_ansi_escapes.
//
// FEATURES AND LIMITATIONS
//
// - This code only recognizes certain escape codes.  However it's not clear in all cases how
//   escape codes should be translated, e.g. is 01;47 background white or background gray?
//   On a terminal, when tested, it appeared as background gray, but the wikipedia article seems
//   to suggest that it should be background white.  So we do not attempt to translate it.
//
// - Colors are translated so as to match the ANSI escape character rendering on a Mac High Sierra
//   terminal window.  We do not know how general this is.  It seems possible that different
//   software developers have chosen to interpret ANSI escape characters differently.
//
// - The run time and space usage are not optimized.  In fact they are very bad.
//
// - After a newline, possibly a </span> should be emitted.
//
// REFERENCES
//
// 1. https://en.wikipedia.org/wiki/ANSI_escape_code
//
// A general guide to ANSI escape codes.
//
// 2. https://github.com/theZiz/aha
//
// A program that translates text with arbitrary ANSI escape codes to html, so more general
// than what is done here.  However this code produces html that is shorter.  And this code
// translates colors differently.
//
// svg translation added

use std::cmp::max;
use string_utils::{strme, TextUtils};
use vector_utils::VecUtils;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// This does not translate background!

pub fn convert_text_with_ansi_escapes_to_svg(
    x: &str,
    font_family: &str,
    font_size: usize,
) -> String {
    // Compute separations.  These may be font-specific; optimized for Menlo.

    let vsep = (19.1 / 15.0) * font_size as f64;
    let hsep = 0.6 * font_size as f64;

    // Proceed.

    let lines0 = x.split('\n').collect::<Vec<&str>>();
    let height = vsep * (lines0.len() as f64 - 1.2);
    let mut lines = Vec::<String>::new();
    lines.push("<svg version=\"1.1\"".to_string());
    lines.push("".to_string()); // PLACEHOLDER
    lines.push("xmlns=\"http://www.w3.org/2000/svg\">".to_string());
    let mut max_width = 0;
    for m in 0..lines0.len() {
        let t = &lines0[m];
        let mut width = 0;
        let mut svg = String::new();
        svg += &format!(
            "<text x=\"{}\" y=\"{:.1}\" font-family=\"{}\" font-size=\"{}\" \
                style=\"white-space: pre;\">",
            0,
            (m + 1) as f64 * vsep,
            font_family,
            font_size,
        );
        let y: Vec<char> = t.chars().collect();
        let mut states = Vec::<ColorState>::new();
        let mut current_state = ColorState::default();
        let mut i = 0;
        while i < y.len() {
            if y[i] != '' {
                width += 1;
                if !states.is_empty() {
                    let new_state = merge(&states);
                    if new_state != current_state {
                        if !current_state.null() && !new_state.null() {
                            svg += "</tspan>";
                        }
                        svg += &new_state.svg();
                        current_state = new_state;
                    }
                    states.clear();
                }
                if y[i] != '<' {
                    svg.push(y[i]);
                } else {
                    svg += "&lt;";
                }
                i += 1;
            } else {
                let mut j = i + 1;
                loop {
                    if y[j] == 'm' {
                        break;
                    }
                    j += 1;
                }
                let mut e = Vec::<u8>::new();
                for m in i..=j {
                    e.push(y[m] as u8);
                }
                states.push(ansi_escape_to_color_state(&e));
                i = j + 1;
            }
        }
        max_width = max(width, max_width);
        if !states.is_empty() {
            svg += &merge(&states).svg();
        }
        svg += "</text>";
        lines.push(svg);
    }
    lines[1] = format!("viewBox=\"0 0 {} {}\"", max_width as f64 * hsep, height);
    let mut svg = String::new();
    for i in 0..lines.len() {
        svg += &lines[i];
        svg += "\n";
    }
    svg += "</svg>\n";
    svg
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

pub fn convert_text_with_ansi_escapes_to_html(
    x: &str,
    source: &str,
    title: &str,
    html_text: &str,
    font_family: &str,
    font_size: usize,
) -> String {
    let y: Vec<char> = x.chars().collect();
    let mut html = html_head(source, title, html_text, font_family, font_size);
    let mut states = Vec::<ColorState>::new();
    let mut current_state = ColorState::default();
    let mut i = 0;
    while i < y.len() {
        if y[i] != '' {
            if !states.is_empty() {
                let new_state = merge(&states);
                if new_state != current_state {
                    if !current_state.null() && !new_state.null() {
                        html += "</span>";
                    }
                    html += &new_state.html();
                    current_state = new_state;
                }
                states.clear();
            }
            if y[i] != '<' {
                html.push(y[i]);
            } else {
                html += "&lt;";
            }
            i += 1;
        } else {
            let mut j = i + 1;
            loop {
                if y[j] == 'm' {
                    break;
                }
                j += 1;
            }
            let mut e = Vec::<u8>::new();
            for m in i..=j {
                e.push(y[m] as u8);
            }
            states.push(ansi_escape_to_color_state(&e));
            i = j + 1;
        }
    }
    if !states.is_empty() {
        html += &merge(&states).html();
    }
    format!("{}{}", html, html_tail())
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Remove redundant ansi escape sequences.  Note that this only recognizes certain escapes.

pub fn compress_ansi_escapes(x: &str) -> String {
    let y: Vec<char> = x.chars().collect();
    let mut out = String::new();
    let mut escapes = Vec::<Vec<u8>>::new();
    let mut old_state = ColorState::default();
    let mut on = false;
    let mut i = 0;
    while i < y.len() {
        if y[i] != '' {
            if !escapes.is_empty() {
                let mut states = Vec::<ColorState>::new();
                for j in 0..escapes.len() {
                    states.push(ansi_escape_to_color_state(&pack_ansi_escape(&escapes[j])));
                }
                let new_state = merge(&states);
                if new_state != old_state {
                    let mut end = None;
                    for i in 0..escapes.len() {
                        if escapes[i].solo() && escapes[i][0] == 0 {
                            end = Some(i);
                        }
                    }
                    if end.is_some() {
                        escapes = escapes[end.unwrap() + 1..escapes.len()].to_vec();
                    }
                    if escapes.is_empty() {
                        // Emit end escape.

                        out += "[0m";
                        on = false;
                    } else {
                        let mut reset = false;
                        if on
                            && ((old_state.bold && !new_state.bold)
                                || (!old_state.color.is_empty() && new_state.color.is_empty())
                                || (!old_state.background.is_empty()
                                    && new_state.background.is_empty()))
                        {
                            out += "[0m";
                            reset = true;
                        }
                        on = true;

                        // Emit bold, then color, then background.

                        for e in escapes.iter() {
                            if e.solo() && e[0] == 1 {
                                if reset || new_state.bold != old_state.bold {
                                    out += &strme(&pack_ansi_escape(e)).to_string();
                                }
                                break;
                            }
                        }
                        for i in (0..escapes.len()).rev() {
                            let y = &escapes[i];
                            if (y.solo() && y[0] >= 30 && y[0] <= 37)
                                || (y.len() == 3 && y[0] == 38 && y[1] == 5)
                            {
                                if reset || new_state.color != old_state.color {
                                    out += &strme(&pack_ansi_escape(y)).to_string();
                                }
                                break;
                            }
                        }
                        for i in (0..escapes.len()).rev() {
                            let y = &escapes[i];
                            if (y.solo() && y[0] >= 40 && y[0] <= 47)
                                || (y.len() == 3 && y[0] == 48 && y[1] == 5)
                            {
                                if reset || new_state.background != old_state.background {
                                    out += &strme(&pack_ansi_escape(y)).to_string();
                                }
                                break;
                            }
                        }
                    }
                    old_state = new_state;
                }
                escapes.clear();
            }
            out.push(y[i]);
            i += 1;
        } else {
            let mut j = i + 1;
            loop {
                if y[j] == 'm' {
                    break;
                }
                j += 1;
            }
            let mut e = Vec::<u8>::new();
            for m in i..=j {
                e.push(y[m] as u8);
            }
            escapes.push(unpack_ansi_escape(&e));
            i = j + 1;
        }
    }
    if on {
        out += "[0m";
    }
    out
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

fn html_head(
    source: &str,
    title: &str,
    head_text: &str,
    font_family: &str,
    font_size: usize,
) -> String {
    let ff = format!("\"{}\"", font_family.replace(", ", "\", \""));
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n\
              <!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
              \"https://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\n<!-- {} -->\n\
              <html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head>\n\
              <meta http-equiv=\"Content-Type\" content=\"application/xml+xhtml; \
              charset=UTF-8\"/>\n\
              <title>{}</title>\n\
              {}\n\
              </head>\n<body>\n<pre style='font-family: {}; line-height: 110%'>\
              <span style=\"font-size: {}px\">",
        source, title, head_text, ff, font_size
    )
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

fn html_tail() -> String {
    "</span></pre>\n</body>\n</html>\n".to_string()
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Unpack an ANSI escape sequence into a vector of integers.  This assumes that semicolons are
// used as separators.

fn unpack_ansi_escape(x: &[u8]) -> Vec<u8> {
    let n = x.len();
    if x[0] != b'' {
        panic!(
            "unpack_ansi_escape passed something that is not an escape sequence: \"{}\" \
            (len={})",
            strme(x),
            x.len()
        );
    }
    assert_eq!(x[1], b'[');
    assert_eq!(x[n - 1], b'm');
    let s = x[2..n - 1].split(|c| *c == b';').collect::<Vec<&[u8]>>();
    let mut y = Vec::<u8>::new();
    for i in 0..s.len() {
        y.push(strme(s[i]).force_usize() as u8);
    }
    y
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Reverse the process.

fn pack_ansi_escape(y: &[u8]) -> Vec<u8> {
    let mut x = b"[".to_vec();
    for i in 0..y.len() {
        if i > 0 {
            x.push(b';');
        }
        x.append(&mut format!("{}", y[i]).as_bytes().to_vec());
    }
    x.push(b'm');
    x
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Convert an rgb code to a seven-character html string.

fn rgb_to_html(rgb: &(u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// ColorState semantics are as follows:
// - initially only one thing is set;
// - if nothing is set, it means clear;
// - after merging, any combination can be set (and nothing still means clear).

#[derive(Default, PartialEq, Eq)]
struct ColorState {
    color: String,
    background: String,
    bold: bool,
}

impl ColorState {
    fn null(&self) -> bool {
        self.color.is_empty() && self.background.is_empty() && !self.bold
    }
    fn html(&self) -> String {
        if self.null() {
            "</span>".to_string()
        } else {
            let mut s = "<span style=\"".to_string();
            if !self.color.is_empty() {
                s += &format!("color:{};", self.color);
            }
            if !self.background.is_empty() {
                s += &format!("background-color:{};", self.background);
            }
            if self.bold {
                s += &"font-weight:bold;".to_string()
            }
            s += "\">";
            s
        }
    }

    // This does not translate background!

    fn svg(&self) -> String {
        if self.null() {
            "</tspan>".to_string()
        } else {
            let mut s = "<tspan style=\"".to_string();
            if !self.color.is_empty() {
                s += &format!("fill: {};", self.color);
            }
            if self.bold {
                s += &"font-weight: bold;".to_string()
            }
            s += "\">";
            s
        }
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

fn merge(s: &Vec<ColorState>) -> ColorState {
    let mut x = ColorState::default();
    for i in 0..s.len() {
        if s[i].null() {
            x.color = String::new();
            x.background = String::new();
            x.bold = false;
        } else if !s[i].color.is_empty() {
            x.color = s[i].color.clone();
        } else if !s[i].background.is_empty() {
            x.background = s[i].background.clone();
        } else {
            x.bold = true;
        }
    }
    x
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Translate an ANSI escape sequence into a ColorState.  This only works for certain ANSI escape
// sequences, but could be generalized.

fn ansi_escape_to_color_state(x: &[u8]) -> ColorState {
    let y = unpack_ansi_escape(x);
    if y.len() == 3 && y[0] == 38 && y[1] == 5 {
        ColorState {
            color: rgb_to_html(&color_256_to_rgb(y[2])),
            background: String::new(),
            bold: false,
        }
    } else if y.len() == 3 && y[0] == 48 && y[1] == 5 {
        ColorState {
            color: String::new(),
            background: rgb_to_html(&color_256_to_rgb(y[2])),
            bold: false,
        }
    } else if y.len() == 1 && y[0] == 1 {
        ColorState {
            color: String::new(),
            background: String::new(),
            bold: true,
        }
    } else if y.len() == 1 && y[0] == 0 {
        ColorState {
            color: String::new(),
            background: String::new(),
            bold: false,
        }
    } else if y.len() == 1 && y[0] >= 30 && y[0] <= 37 {
        ColorState {
            color: rgb_to_html(&color_256_to_rgb(y[0] - 30)),
            background: String::new(),
            bold: false,
        }
    } else if y.len() == 1 && y[0] >= 40 && y[0] <= 47 {
        ColorState {
            color: String::new(),
            background: rgb_to_html(&color_256_to_rgb(y[0] - 40)),
            bold: false,
        }
    } else {
        panic!(
            "\nSorry, ANSI escape translation not implemented for {}.\n",
            strme(x)
        );
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Convert an ANSI escape color code in [0,256) to (r,g,b).
// See https://en.wikipedia.org/wiki/ANSI_escape_code.
//
// For 0-15, we use what wikipedia calls the "Terminal.app" colors for 30-37,90-97.
// as listed in the table for 3/4 bit.
//
// For 16-255, we ran color_table in bin, then used the digital color meter
// on a Mac (which is a standard app) to read off the values for sRGB.
// If you raise aperture size you get a somewhat more constant value.
//
// Note that the digital color meter has several other RGB scales.  It's not clear what this means.
//
// The colors assigned here appear to be very close to the colors assigned by the terminal app
// on a Mac, at least on High Sierra.  We do not know if these colors are consistent across
// Mac versions.
//
// This should be packed as an array as the current code appears to be ridiculously inefficient.

fn color_256_to_rgb(c: u8) -> (u8, u8, u8) {
    if c == 0 {
        (0, 0, 0)
    } else if c == 1 {
        (194, 54, 33)
    } else if c == 2 {
        (37, 188, 36)
    } else if c == 3 {
        (173, 173, 39)
    } else if c == 4 {
        (73, 46, 225)
    } else if c == 5 {
        (211, 56, 211)
    } else if c == 6 {
        (51, 187, 200)
    } else if c == 7 {
        (203, 204, 205)
    } else if c == 8 {
        (129, 131, 131)
    } else if c == 9 {
        (252, 57, 31)
    } else if c == 10 {
        (49, 231, 34)
    } else if c == 11 {
        (234, 236, 35)
    } else if c == 12 {
        (88, 51, 255)
    } else if c == 13 {
        (249, 53, 248)
    } else if c == 14 {
        (20, 240, 240)
    } else if c == 15 {
        (233, 235, 235)

    // Empirically determined values, see above.
    } else if c == 16 {
        (48, 48, 48)
    } else if c == 17 {
        (64, 41, 144)
    } else if c == 18 {
        (74, 44, 183)
    } else if c == 19 {
        (82, 47, 222)
    } else if c == 20 {
        (88, 50, 255)
    } else if c == 21 {
        (94, 52, 255)
    } else if c == 22 {
        (50, 128, 38)
    } else if c == 23 {
        (50, 127, 127)
    } else if c == 24 {
        (55, 126, 168)
    } else if c == 25 {
        (62, 125, 209)
    } else if c == 26 {
        (69, 124, 249)
    } else if c == 27 {
        (76, 122, 255)
    } else if c == 28 {
        (52, 163, 39)
    } else if c == 29 {
        (50, 162, 120)
    } else if c == 30 {
        (52, 161, 161)
    } else if c == 31 {
        (55, 160, 201)
    } else if c == 32 {
        (60, 159, 242)
    } else if c == 33 {
        (66, 158, 255)
    } else if c == 34 {
        (51, 196, 38)
    } else if c == 35 {
        (49, 196, 114)
    } else if c == 36 {
        (49, 195, 154)
    } else if c == 37 {
        (50, 195, 194)
    } else if c == 38 {
        (53, 194, 235)
    } else if c == 39 {
        (56, 193, 255)
    } else if c == 40 {
        (46, 229, 33)
    } else if c == 41 {
        (44, 229, 108)
    } else if c == 42 {
        (44, 229, 148)
    } else if c == 43 {
        (43, 228, 188)
    } else if c == 44 {
        (44, 228, 228)
    } else if c == 45 {
        (45, 227, 255)
    } else if c == 46 {
        (35, 255, 24)
    } else if c == 47 {
        (33, 255, 102)
    } else if c == 48 {
        (32, 255, 141)
    } else if c == 49 {
        (32, 255, 181)
    } else if c == 50 {
        (30, 255, 221)
    } else if c == 51 {
        (30, 255, 255)
    } else if c == 52 {
        (141, 49, 39)
    } else if c == 53 {
        (137, 51, 135)
    } else if c == 54 {
        (137, 52, 176)
    } else if c == 55 {
        (137, 53, 216)
    } else if c == 56 {
        (138, 54, 255)
    } else if c == 57 {
        (139, 55, 255)
    } else if c == 58 {
        (127, 125, 38)
    } else if c == 59 {
        (125, 125, 125)
    } else if c == 60 {
        (125, 124, 166)
    } else if c == 61 {
        (125, 123, 206)
    } else if c == 62 {
        (127, 122, 247)
    } else if c == 63 {
        (128, 121, 255)
    } else if c == 64 {
        (121, 160, 38)
    } else if c == 65 {
        (119, 160, 119)
    } else if c == 66 {
        (119, 160, 159)
    } else if c == 67 {
        (119, 159, 200)
    } else if c == 68 {
        (120, 158, 240)
    } else if c == 69 {
        (121, 157, 255)
    } else if c == 70 {
        (115, 195, 36)
    } else if c == 71 {
        (114, 194, 113)
    } else if c == 72 {
        (114, 194, 153)
    } else if c == 73 {
        (113, 193, 193)
    } else if c == 74 {
        (114, 193, 233)
    } else if c == 75 {
        (114, 192, 255)
    } else if c == 76 {
        (110, 229, 32)
    } else if c == 77 {
        (109, 228, 107)
    } else if c == 78 {
        (108, 228, 147)
    } else if c == 79 {
        (108, 227, 187)
    } else if c == 80 {
        (108, 227, 227)
    } else if c == 81 {
        (108, 226, 255)
    } else if c == 82 {
        (103, 255, 22)
    } else if c == 83 {
        (103, 255, 101)
    } else if c == 84 {
        (103, 255, 141)
    } else if c == 85 {
        (102, 255, 180)
    } else if c == 86 {
        (101, 255, 220)
    } else if c == 87 {
        (101, 255, 255)
    } else if c == 88 {
        (178, 54, 34)
    } else if c == 89 {
        (175, 55, 130)
    } else if c == 90 {
        (174, 56, 172)
    } else if c == 91 {
        (173, 56, 212)
    } else if c == 92 {
        (172, 57, 252)
    } else if c == 93 {
        (172, 58, 255)
    } else if c == 94 {
        (167, 123, 37)
    } else if c == 95 {
        (165, 123, 122)
    } else if c == 96 {
        (164, 122, 163)
    } else if c == 97 {
        (163, 122, 204)
    } else if c == 98 {
        (164, 121, 244)
    } else if c == 99 {
        (164, 120, 255)
    } else if c == 100 {
        (160, 159, 37)
    } else if c == 101 {
        (158, 158, 117)
    } else if c == 102 {
        (158, 158, 158)
    } else if c == 103 {
        (157, 157, 198)
    } else if c == 104 {
        (157, 156, 238)
    } else if c == 105 {
        (158, 156, 255)
    } else if c == 106 {
        (154, 193, 35)
    } else if c == 107 {
        (153, 193, 122)
    } else if c == 108 {
        (152, 193, 152)
    } else if c == 109 {
        (152, 192, 192)
    } else if c == 110 {
        (152, 191, 232)
    } else if c == 111 {
        (151, 191, 255)
    } else if c == 112 {
        (147, 227, 31)
    } else if c == 113 {
        (147, 227, 106)
    } else if c == 114 {
        (146, 227, 146)
    } else if c == 115 {
        (146, 226, 186)
    } else if c == 116 {
        (146, 226, 226)
    } else if c == 117 {
        (145, 225, 255)
    } else if c == 118 {
        (141, 255, 21)
    } else if c == 119 {
        (141, 255, 100)
    } else if c == 120 {
        (140, 255, 140)
    } else if c == 121 {
        (140, 255, 180)
    } else if c == 122 {
        (140, 255, 219)
    } else if c == 123 {
        (139, 255, 255)
    } else if c == 124 {
        (215, 57, 31)
    } else if c == 125 {
        (212, 58, 126)
    } else if c == 126 {
        (211, 58, 167)
    } else if c == 127 {
        (209, 59, 208)
    } else if c == 128 {
        (209, 59, 248)
    } else if c == 129 {
        (208, 59, 255)
    } else if c == 130 {
        (206, 121, 35)
    } else if c == 131 {
        (204, 121, 120)
    } else if c == 132 {
        (203, 120, 161)
    } else if c == 133 {
        (202, 120, 201)
    } else if c == 134 {
        (201, 119, 241)
    } else if c == 135 {
        (201, 119, 255)
    } else if c == 136 {
        (199, 156, 36)
    } else if c == 137 {
        (198, 156, 115)
    } else if c == 138 {
        (197, 156, 156)
    } else if c == 139 {
        (196, 155, 196)
    } else if c == 140 {
        (196, 155, 236)
    } else if c == 141 {
        (195, 154, 255)
    } else if c == 142 {
        (192, 192, 34)
    } else if c == 143 {
        (192, 191, 110)
    } else if c == 144 {
        (191, 191, 150)
    } else if c == 145 {
        (191, 191, 191)
    } else if c == 146 {
        (190, 190, 231)
    } else if c == 147 {
        (190, 189, 255)
    } else if c == 148 {
        (186, 226, 29)
    } else if c == 149 {
        (186, 226, 105)
    } else if c == 150 {
        (185, 225, 145)
    } else if c == 151 {
        (185, 225, 185)
    } else if c == 152 {
        (184, 225, 225)
    } else if c == 153 {
        (184, 224, 255)
    } else if c == 154 {
        (180, 255, 17)
    } else if c == 155 {
        (180, 255, 99)
    } else if c == 156 {
        (179, 255, 139)
    } else if c == 157 {
        (179, 255, 179)
    } else if c == 158 {
        (178, 255, 218)
    } else if c == 159 {
        (178, 255, 255)
    } else if c == 160 {
        (251, 59, 30)
    } else if c == 161 {
        (249, 59, 121)
    } else if c == 162 {
        (247, 60, 162)
    } else if c == 163 {
        (246, 60, 203)
    } else if c == 164 {
        (245, 60, 243)
    } else if c == 165 {
        (244, 60, 255)
    } else if c == 166 {
        (243, 119, 33)
    } else if c == 167 {
        (242, 119, 117)
    } else if c == 168 {
        (241, 119, 157)
    } else if c == 169 {
        (240, 118, 198)
    } else if c == 170 {
        (239, 118, 238)
    } else if c == 171 {
        (238, 117, 255)
    } else if c == 172 {
        (237, 154, 34)
    } else if c == 173 {
        (236, 154, 113)
    } else if c == 174 {
        (236, 154, 153)
    } else if c == 175 {
        (235, 153, 194)
    } else if c == 176 {
        (234, 153, 234)
    } else if c == 177 {
        (233, 152, 255)
    } else if c == 178 {
        (231, 189, 32)
    } else if c == 179 {
        (230, 189, 108)
    } else if c == 180 {
        (230, 189, 148)
    } else if c == 181 {
        (229, 189, 189)
    } else if c == 182 {
        (229, 188, 229)
    } else if c == 183 {
        (228, 187, 255)
    } else if c == 184 {
        (225, 224, 26)
    } else if c == 185 {
        (224, 224, 103)
    } else if c == 186 {
        (224, 224, 143)
    } else if c == 187 {
        (223, 223, 183)
    } else if c == 188 {
        (223, 223, 223)
    } else if c == 189 {
        (222, 222, 255)
    } else if c == 190 {
        (218, 255, 13)
    } else if c == 191 {
        (218, 255, 98)
    } else if c == 192 {
        (218, 255, 138)
    } else if c == 193 {
        (217, 255, 177)
    } else if c == 194 {
        (217, 255, 217)
    } else if c == 195 {
        (217, 255, 255)
    } else if c == 196 {
        (255, 59, 29)
    } else if c == 197 {
        (255, 60, 117)
    } else if c == 198 {
        (255, 60, 158)
    } else if c == 199 {
        (255, 60, 199)
    } else if c == 200 {
        (255, 60, 239)
    } else if c == 201 {
        (255, 60, 255)
    } else if c == 202 {
        (255, 116, 31)
    } else if c == 203 {
        (255, 117, 113)
    } else if c == 204 {
        (255, 116, 154)
    } else if c == 205 {
        (255, 116, 195)
    } else if c == 206 {
        (255, 116, 235)
    } else if c == 207 {
        (255, 115, 255)
    } else if c == 208 {
        (255, 152, 32)
    } else if c == 209 {
        (255, 151, 110)
    } else if c == 210 {
        (251, 151, 151)
    } else if c == 211 {
        (251, 151, 191)
    } else if c == 212 {
        (255, 150, 231)
    } else if c == 213 {
        (255, 150, 255)
    } else if c == 214 {
        (255, 187, 29)
    } else if c == 215 {
        (255, 187, 106)
    } else if c == 216 {
        (255, 187, 146)
    } else if c == 217 {
        (255, 186, 186)
    } else if c == 218 {
        (255, 186, 226)
    } else if c == 219 {
        (255, 185, 255)
    } else if c == 220 {
        (255, 222, 23)
    } else if c == 221 {
        (255, 222, 102)
    } else if c == 222 {
        (255, 222, 141)
    } else if c == 223 {
        (255, 221, 181)
    } else if c == 224 {
        (255, 221, 221)
    } else if c == 225 {
        (255, 220, 255)
    } else if c == 226 {
        (254, 255, 7)
    } else if c == 227 {
        (254, 255, 96)
    } else if c == 228 {
        (254, 255, 136)
    } else if c == 229 {
        (254, 255, 176)
    } else if c == 230 {
        (254, 255, 215)
    } else if c == 231 {
        (255, 255, 255)
    } else if c == 232 {
        (52, 52, 52)
    } else if c == 233 {
        (58, 58, 58)
    } else if c == 234 {
        (67, 67, 67)
    } else if c == 235 {
        (76, 76, 76)
    } else if c == 236 {
        (84, 84, 84)
    } else if c == 237 {
        (93, 93, 93)
    } else if c == 238 {
        (102, 102, 102)
    } else if c == 239 {
        (110, 110, 110)
    } else if c == 240 {
        (119, 119, 119)
    } else if c == 241 {
        (127, 127, 127)
    } else if c == 242 {
        (136, 136, 136)
    } else if c == 243 {
        (144, 144, 144)
    } else if c == 244 {
        (152, 152, 152)
    } else if c == 245 {
        (160, 160, 160)
    } else if c == 246 {
        (169, 169, 169)
    } else if c == 247 {
        (177, 177, 177)
    } else if c == 248 {
        (185, 185, 185)
    } else if c == 249 {
        (193, 193, 193)
    } else if c == 250 {
        (201, 201, 201)
    } else if c == 251 {
        (209, 209, 209)
    } else if c == 252 {
        (217, 217, 217)
    } else if c == 253 {
        (225, 225, 225)
    } else if c == 254 {
        (233, 233, 233)
    } else {
        (241, 241, 241)

        /*
        if c <= 6 {
            (128 * (c % 2), 128 * ((c / 2) % 2), 128 * (c / 4))
        } else if c == 7 {
            (192, 192, 192)
        } else if c == 8 {
            (128, 128, 128)
        } else if c <= 15 {
            (
                255 * ((c - 8) % 2),
                255 * (((c - 8) / 2) % 2),
                255 * ((c - 8) / 4),
            )
        } else if c <= 231 {
            let mut x = c - 16;
            let r = x / 36;
            x -= 36 * r;
            let g = x / 6;
            let b = x % 6;
            #[rustfmt::skip]
            fn f(m: u8) -> u8 { if m == 0 { 0 } else { 40 * m + 55 } }
            (f(r), f(g), f(b))
        } else {
            let z = (((c - 232) as usize * 32) / 3) as u8;
            (z, z, z)
        */
    }
}
