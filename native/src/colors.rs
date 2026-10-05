// Farby v kóde (ako VS Code): #hex, rgb()/rgba(), hsl()/hsla() a pomenované farby v CSS.
// Editor pred ne kreslí štvorček; klik otvorí výber farby a zmena sa zapíše späť v rovnakom zápise.
use eframe::egui::{text::LayoutJob, Color32, TextFormat};

// medzera pred farbou na štvorček (px); editor ju vkladá do rozloženia textu, takže nič neprekrýva
pub const GAP: f32 = 13.0;
// väčší súbor štvorčeky nemá (rozloženie by sa pri každej zmene prechádzalo celé)
pub const MAX_BYTES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fmt {
    Hex(usize), // počet číslic (3, 4, 6, 8)
    Rgb,
    Hsl,
    Named,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub start: usize, // bajt v riadku
    pub len: usize,   // bajtov
    pub color: Color32,
    pub fmt: Fmt,
}

const NAMED: &[(&str, u32)] = &[
    ("black", 0x000000), ("white", 0xffffff), ("red", 0xff0000), ("green", 0x008000), ("blue", 0x0000ff), ("yellow", 0xffff00), ("orange", 0xffa500),
    ("purple", 0x800080), ("pink", 0xffc0cb), ("gray", 0x808080), ("grey", 0x808080), ("brown", 0xa52a2a), ("cyan", 0x00ffff), ("magenta", 0xff00ff),
    ("lime", 0x00ff00), ("navy", 0x000080), ("teal", 0x008080), ("olive", 0x808000), ("maroon", 0x800000), ("silver", 0xc0c0c0), ("gold", 0xffd700),
    ("aqua", 0x00ffff), ("fuchsia", 0xff00ff), ("coral", 0xff7f50), ("crimson", 0xdc143c), ("indigo", 0x4b0082), ("violet", 0xee82ee), ("salmon", 0xfa8072),
    ("tomato", 0xff6347), ("turquoise", 0x40e0d0), ("tan", 0xd2b48c), ("khaki", 0xf0e68c), ("beige", 0xf5f5dc), ("ivory", 0xfffff0), ("lavender", 0xe6e6fa),
    ("plum", 0xdda0dd), ("orchid", 0xda70d6), ("chocolate", 0xd2691e), ("skyblue", 0x87ceeb), ("steelblue", 0x4682b4), ("royalblue", 0x4169e1),
    ("dodgerblue", 0x1e90ff), ("hotpink", 0xff69b4), ("darkred", 0x8b0000), ("darkgreen", 0x006400), ("darkblue", 0x00008b), ("lightgray", 0xd3d3d3),
    ("lightgrey", 0xd3d3d3), ("darkgray", 0xa9a9a9), ("darkgrey", 0xa9a9a9), ("whitesmoke", 0xf5f5f5), ("slategray", 0x708090), ("rebeccapurple", 0x663399),
];

fn is_css(lang: &str) -> bool {
    matches!(lang, "css" | "scss" | "less" | "html" | "htm" | "vue" | "svelte")
}

// začiatky farieb v riadku (bajty); rýchly predfilter, aby bežný riadok nestál nič
pub fn starts(line: &str, lang: &str) -> Vec<usize> {
    let css = is_css(lang);
    if line.len() > 3000 || !(line.contains('#') || line.contains("rgb") || line.contains("hsl") || (css && line.contains(':'))) {
        return vec![];
    }
    find(line, lang).into_iter().map(|f| f.start).collect()
}

// začiatky farieb v celom texte (bajty od začiatku textu)
pub fn all_starts(text: &str, lang: &str) -> Vec<usize> {
    if text.len() > MAX_BYTES {
        return vec![];
    }
    let mut out = vec![];
    let mut off = 0;
    for line in text.split('\n') {
        out.extend(starts(line, lang).into_iter().map(|s| off + s));
        off += line.len() + 1;
    }
    out
}

// text[from..to] do rozloženia tak, aby pred každou farbou (toks = jej začiatky, vzostupne) bola medzera GAP
pub fn append(job: &mut LayoutJob, text: &str, from: usize, to: usize, toks: &[usize], fmt: &TextFormat) {
    if from >= to {
        return;
    }
    let lead = |a: usize| if toks.binary_search(&a).is_ok() { GAP } else { 0.0 };
    let mut a = from;
    for &t in toks.iter().filter(|t| **t > from && **t < to) {
        job.append(&text[a..t], lead(a), fmt.clone());
        a = t;
    }
    job.append(&text[a..to], lead(a), fmt.clone());
}

// jazyky, kde „#“ začína komentár: #hex len v úvodzovkách
fn hash_comment(lang: &str) -> bool {
    matches!(lang, "py" | "pyw" | "sh" | "bash" | "rb" | "r" | "ps1" | "yml" | "yaml" | "toml" | "pl" | "pm" | "jl" | "conf" | "ini")
}

fn hex_val(s: &str) -> Option<Color32> {
    let d: Vec<u8> = s.chars().map(|c| c.to_digit(16).map(|v| v as u8)).collect::<Option<Vec<u8>>>()?;
    let two = |a: u8, b: u8| a * 16 + b;
    Some(match d.len() {
        3 => Color32::from_rgb(d[0] * 17, d[1] * 17, d[2] * 17),
        4 => Color32::from_rgba_unmultiplied(d[0] * 17, d[1] * 17, d[2] * 17, d[3] * 17),
        6 => Color32::from_rgb(two(d[0], d[1]), two(d[2], d[3]), two(d[4], d[5])),
        8 => Color32::from_rgba_unmultiplied(two(d[0], d[1]), two(d[2], d[3]), two(d[4], d[5]), two(d[6], d[7])),
        _ => return None,
    })
}

fn num(s: &str, max: f32) -> Option<f32> {
    let s = s.trim();
    if let Some(p) = s.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().map(|v| v / 100.0 * max);
    }
    s.trim_end_matches("deg").parse::<f32>().ok()
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let f = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (f(r), f(g), f(b))
}

fn func_val(name: &str, args: &str) -> Option<Color32> {
    let parts: Vec<&str> = args.split([',', ' ', '/']).filter(|p| !p.trim().is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }
    let a = parts.get(3).and_then(|v| num(v, 1.0)).unwrap_or(1.0).clamp(0.0, 1.0);
    let alpha = (a * 255.0).round() as u8;
    if name.starts_with("rgb") {
        let c = |i: usize| num(parts[i], 255.0).map(|v| v.round().clamp(0.0, 255.0) as u8);
        Some(Color32::from_rgba_unmultiplied(c(0)?, c(1)?, c(2)?, alpha))
    } else {
        let h = num(parts[0], 360.0)?;
        let s = num(parts[1], 1.0)?.clamp(0.0, 1.0);
        let l = num(parts[2], 1.0)?.clamp(0.0, 1.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Some(Color32::from_rgba_unmultiplied(r, g, b, alpha))
    }
}

// farby v jednom riadku
pub fn find(line: &str, lang: &str) -> Vec<Found> {
    let mut out = vec![];
    let b = line.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
    let css = is_css(lang);
    // úvodzovky pred bajtom i (párne/nepárne) – pre jazyky s # komentármi
    let quoted = |i: usize| {
        let mut q: Option<u8> = None;
        for &c in &b[..i] {
            match q {
                Some(x) if c == x => q = None,
                None if c == b'"' || c == b'\'' => q = Some(c),
                None if c == b'#' && hash_comment(lang) => return false,
                _ => {}
            }
        }
        q.is_some()
    };
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'#' && (i == 0 || !word(b[i - 1]) && b[i - 1] != b'&') {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            let n = j - i - 1;
            if matches!(n, 3 | 4 | 6 | 8) && (j >= b.len() || !word(b[j])) && (!hash_comment(lang) || quoted(i)) {
                if let Some(col) = hex_val(&line[i + 1..j]) {
                    out.push(Found { start: i, len: j - i, color: col, fmt: Fmt::Hex(n) });
                    i = j;
                    continue;
                }
            }
        }
        if (c == b'r' || c == b'h') && (i == 0 || !word(b[i - 1])) {
            for name in ["rgba(", "rgb(", "hsla(", "hsl("] {
                if line[i..].starts_with(name) {
                    if let Some(close) = line[i..].find(')') {
                        if let Some(col) = func_val(name, &line[i + name.len()..i + close]) {
                            out.push(Found { start: i, len: close + 1, color: col, fmt: if name.starts_with("rgb") { Fmt::Rgb } else { Fmt::Hsl } });
                            i += close + 1;
                        }
                    }
                    break;
                }
            }
        }
        // pomenované farby len v hodnote CSS (za „:“)
        if css && c.is_ascii_alphabetic() && (i == 0 || !word(b[i - 1])) {
            let mut j = i;
            while j < b.len() && b[j].is_ascii_alphabetic() {
                j += 1;
            }
            let w = line[i..j].to_ascii_lowercase();
            let before = line[..i].trim_end();
            let in_value = before.ends_with(':') || (before.contains(':') && !before.contains(';') && !before.contains('{') && line[i..].contains([';', '"', '}']));
            if let Some((_, v)) = NAMED.iter().find(|(n, _)| *n == w).filter(|_| in_value && (j >= b.len() || !word(b[j]))) {
                out.push(Found { start: i, len: j - i, color: Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, *v as u8), fmt: Fmt::Named });
            }
            i = j.max(i + 1);
            continue;
        }
        i += 1;
    }
    out
}

// farba z textu, ktorý je celý jedna farba („#3b82f6“, „rgb(1,2,3)“, „red“)
pub fn parse(s: &str) -> Option<Color32> {
    let s = s.trim();
    let f = find(&format!("a{{color: {s};}}"), "css");
    f.first().filter(|x| x.len == s.len()).map(|x| x.color)
}

// farba späť do textu v rovnakom zápise (pomenovaná → #hex)
pub fn format(c: Color32, fmt: Fmt) -> String {
    let [r, g, bl, a] = c.to_srgba_unmultiplied();
    match fmt {
        Fmt::Rgb if a < 255 => format!("rgba({r}, {g}, {bl}, {})", trim_alpha(a)),
        Fmt::Rgb => format!("rgb({r}, {g}, {bl})"),
        Fmt::Hsl => {
            let (h, s, l) = rgb_to_hsl(r, g, bl);
            if a < 255 {
                format!("hsla({h}, {s}%, {l}%, {})", trim_alpha(a))
            } else {
                format!("hsl({h}, {s}%, {l}%)")
            }
        }
        Fmt::Hex(n) if a == 255 && (n == 3 || n == 4) && [r, g, bl].iter().all(|v| v % 17 == 0) => format!("#{:x}{:x}{:x}", r / 17, g / 17, bl / 17),
        _ if a < 255 => format!("#{r:02x}{g:02x}{bl:02x}{a:02x}"),
        _ => format!("#{r:02x}{g:02x}{bl:02x}"),
    }
}

fn trim_alpha(a: u8) -> String {
    let v = (a as f32 / 255.0 * 100.0).round() / 100.0;
    let s = format!("{v}");
    s
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (h.rem_euclid(360.0), s)
    };
    (h.round() as i32, (s * 100.0).round() as i32, (l * 100.0).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_find_hex_rgb_hsl_named() {
        let f = find("  color: #3b82f6; background: rgba(255, 0, 0, 0.5); border-color: hsl(120, 100%, 25%);", "css");
        assert_eq!(f.len(), 3, "{f:?}");
        assert_eq!(f[0].color, Color32::from_rgb(0x3b, 0x82, 0xf6));
        assert_eq!(f[0].fmt, Fmt::Hex(6));
        assert_eq!(f[1].color, Color32::from_rgba_unmultiplied(255, 0, 0, 128));
        assert_eq!(f[2].color, Color32::from_rgb(0, 128, 0));
        let n = find("a { color: red; }", "css");
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].fmt, Fmt::Named);
        // „red“ v selektore nie
        assert!(find(".red { margin: 0 }", "css").is_empty());
        assert_eq!(find("<p style=\"color: #fff\">", "html").len(), 1);
    }

    #[test]
    fn colors_not_in_comments_or_ids() {
        assert!(find("# add numbers #add", "py").is_empty());
        assert_eq!(find("c = \"#ff0000\"  # red", "py").len(), 1);
        assert!(find("#include <stdio.h>", "c").is_empty());
        assert!(find("&#123; abc#fff", "html").is_empty());
        assert!(find("#abcde", "css").is_empty());
    }

    #[test]
    fn colors_format_round_trip() {
        assert_eq!(format(Color32::from_rgb(0x3b, 0x82, 0xf6), Fmt::Hex(6)), "#3b82f6");
        assert_eq!(format(Color32::from_rgb(255, 255, 255), Fmt::Hex(3)), "#fff");
        assert_eq!(format(Color32::from_rgb(10, 20, 30), Fmt::Hex(3)), "#0a141e");
        assert_eq!(format(Color32::from_rgb(1, 2, 3), Fmt::Rgb), "rgb(1, 2, 3)");
        assert_eq!(format(Color32::from_rgb(0, 128, 0), Fmt::Hsl), "hsl(120, 100%, 25%)");
        assert_eq!(format(Color32::from_rgb(255, 0, 0), Fmt::Named), "#ff0000");
        assert_eq!(format(Color32::from_rgba_unmultiplied(255, 0, 0, 128), Fmt::Rgb), "rgba(255, 0, 0, 0.5)");
        assert_eq!(parse("#ff0000"), Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(parse("  blue "), Some(Color32::from_rgb(0, 0, 255)));
        assert_eq!(parse("nope"), None);
    }

    #[test]
    fn colors_layout_gap() {
        let mut job = LayoutJob::default();
        let text = "a { color: #fff; x: red }";
        let toks = starts(text, "css");
        assert_eq!(toks, vec![11, 20]);
        append(&mut job, text, 0, text.len(), &toks, &TextFormat::default());
        assert_eq!(job.text, text);
        assert_eq!(job.sections.iter().filter(|s| s.leading_space > 0.0).count(), 2);
        assert!(starts("let x = 3;", "rs").is_empty());
    }
}
