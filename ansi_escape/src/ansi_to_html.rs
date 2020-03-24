// Copyright (c) 2020 10X Genomics, Inc. All rights reserved.

// Convert text containing ANSI escape codes to html.  There is a single public function
// convert_text_with_ansi_escapes_to_html.
//
// LIMITATIONS
//
// - This code only recognizes certain escape codes.  However it's not clear in all cases how
//   escape codes should be translated, e.g. is 01;47 background white or background gray?
//   On a terminal, when tested, it appeared as background gray, but the wikipedia article seems
//   to suggest that it should be background white.  So we do not attempt to translate it.
//
// - The run time and space usage are not optimized.
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
// than what is done here.  However this code produces html that is shorter.

use string_utils::*;

pub fn convert_text_with_ansi_escapes_to_html(
    x: &str,
    source: &str,
    title: &str,
    font_family: &str,
    font_size: usize,
) -> String {
    let y: Vec<char> = x.chars().collect();
    let mut html = html_head(&source, &title, &font_family, font_size);
    let mut states = Vec::<ColorState>::new();
    let mut current_state = ColorState::default();
    let mut i = 0;
    while i < y.len() {
        if y[i] != '' {
            if !states.is_empty() {
                let new_state = merge(&states);
                if new_state != current_state {
                    if !current_state.null() {
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

fn html_head(source: &str, title: &str, font_family: &str, font_size: usize) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n\
              <!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
              \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\n<!-- {} -->\n\
              <html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head>\n\
              <meta http-equiv=\"Content-Type\" content=\"application/xml+xhtml; \
              charset=UTF-8\"/>\n\
              <title>{}</title>\n\
              </head>\n<body>\n<pre style='font-family: \"{}\"; font-size: \"{}pt\";'>\n",
        source, title, font_family, font_size
    )
}

fn html_tail() -> String {
    format!("</pre>\n</body>\n</html>\n")
}

// Convert an ANSI escape color code in [0,256) to (r,g,b).
// See https://en.wikipedia.org/wiki/ANSI_escape_code.
// For 0-15, we use what wikipedia calls the "Terminal.app" colors for 30-37,90-97.
// as listed in the table for 3/4 bit.

fn color_256_to_rgb(c: u8) -> (u8, u8, u8) {

    if c == 0 {
        (0, 0, 0)
    } else if c == 1 {
        (194,54,33)
    } else if c == 2 {
        (37,188,36)
    } else if c == 3 {
        (173,173,39)
    } else if c == 4 {
        (73,46,225)
    } else if c == 5 {
        (211,56,211)
    } else if c == 6 {
        (51,187,200)
    } else if c == 7 {
        (203,204,205)
    } else if c == 8 {
        (129,131,131)
    } else if c == 9 {
        (252,57,31)
    } else if c == 10 {
        (49,231,34)
    } else if c == 11 {
        (234,236,35)
    } else if c == 12 {
        (88,51,255)
    } else if c == 13 {
        (249,53,248)
    } else if c == 14 {
        (20,240,240)
    } else if c == 15 {
        (233,235,235)

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
    */

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
    }
}

// Unpack an ANSI escape sequence into a vector of integers.  This assumes that semicolons are
// used as separators.

fn unpack_ansi_escape(x: &[u8]) -> Vec<u8> {
    let n = x.len();
    assert_eq!(x[0], b'');
    assert_eq!(x[1], b'[');
    assert_eq!(x[n - 1], b'm');
    let s = x[2..n - 1].split(|c| *c == b';').collect::<Vec<&[u8]>>();
    let mut y = Vec::<u8>::new();
    for i in 0..s.len() {
        y.push(strme(&s[i]).force_usize() as u8);
    }
    y
}

// Convert an rgb code to a seven-character html string.

fn rgb_to_html(rgb: &(u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

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
                s += &format!("font-weight:bold;")
            }
            s += "\">";
            s
        }
    }
}

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

// Translate an ANSI escape sequence into a ColorState.  This only works for certain ANSI escape
// sequences, but could be generalized.

fn ansi_escape_to_color_state(x: &[u8]) -> ColorState {
    let y = unpack_ansi_escape(&x);
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
            strme(&x)
        );
    }
}
