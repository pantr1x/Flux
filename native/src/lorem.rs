// Lorem ipsum – výplňový text. V editore „lorem“ + Tab (30 slov), „lorem10“ (10 slov),
// „lorem3s“ (3 vety), „lorem2p“ (2 odseky; v HTML ako <p>). Príkaz v palete vloží jeden odsek.

const WORDS: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua \
enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis aute irure in \
reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt culpa \
qui officia deserunt mollit anim id est laborum praesent elementum facilisis leo vel fringilla nunc mattis pellentesque \
habitant morbi tristique senectus netus malesuada fames ac turpis egestas integer feugiat scelerisque varius cras semper \
auctor neque vitae tortor condimentum lacinia viverra accumsan mauris pharetra massa ultricies mi purus faucibus ornare \
suspendisse sapien nec sagittis aliquam etiam erat blandit volutpat maecenas gravida arcu dictum tellus at urna";
const OPENING: [&str; 8] = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit"];

// malý generátor náhodných čísel (xorshift) – netreba kvôli tomu ďalšiu knižnicu
struct Rng(u64);
impl Rng {
    fn new() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        Rng(t | 1)
    }
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn cap(w: &str) -> String {
    let mut c = w.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

// n slov ako vety (veľké písmeno, občas čiarka, bodka); prvý text začína „Lorem ipsum dolor sit amet…“
fn words(rng: &mut Rng, n: usize, first: bool) -> String {
    let all: Vec<&str> = WORDS.split(' ').collect();
    let mut out = String::new();
    let mut len = 0;
    let mut goal = if first { OPENING.len() + 1 + rng.next(7) } else { 6 + rng.next(9) };
    for i in 0..n {
        let w = if first && i < OPENING.len() { OPENING[i] } else { all[rng.next(all.len())] };
        if i > 0 {
            out.push(' ');
        }
        out += &if len == 0 { cap(w) } else { w.to_string() };
        len += 1;
        if i == n - 1 || len >= goal {
            out.push('.');
            len = 0;
            goal = 6 + rng.next(9);
        } else if len > 3 && !(first && i < OPENING.len()) && rng.next(100) < 12 {
            out.push(',');
        }
    }
    out
}

// jedna veta: k slov, veľké písmeno len na začiatku, občas čiarka, na konci bodka
fn sentence(rng: &mut Rng, k: usize, first: bool) -> String {
    let all: Vec<&str> = WORDS.split(' ').collect();
    let mut out = String::new();
    for i in 0..k {
        let w = if first && i < OPENING.len() { OPENING[i] } else { all[rng.next(all.len())] };
        if i > 0 {
            out.push(' ');
        }
        out += &if i == 0 { cap(w) } else { w.to_string() };
        if i > 2 && i + 2 < k && !(first && i < OPENING.len()) && rng.next(100) < 12 {
            out.push(',');
        }
    }
    out.push('.');
    out
}

fn sentences(rng: &mut Rng, n: usize, first: bool) -> String {
    (0..n)
        .map(|i| {
            let k = 6 + rng.next(9);
            sentence(rng, k, first && i == 0)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// unit: 'w' slová, 's' vety, 'p' odseky; html = odseky ako <p>
pub fn text(count: usize, unit: char, html: bool) -> String {
    let mut rng = Rng::new();
    let count = count.clamp(1, 200);
    match unit {
        's' => sentences(&mut rng, count, true),
        'p' => {
            let list: Vec<String> = (0..count)
                .map(|i| {
                    let k = 4 + rng.next(3);
                    sentences(&mut rng, k, i == 0)
                })
                .collect();
            if html {
                list.iter().map(|p| format!("<p>{p}</p>")).collect::<Vec<_>>().join("\n")
            } else {
                list.join("\n\n")
            }
        }
        _ => words(&mut rng, count, true),
    }
}

// „lorem“, „lorem10“, „lorem3s“, „lorem2p“ → (počet, jednotka); None = nie je to skratka lorem
pub fn parse(word: &str) -> Option<(usize, char)> {
    let w = word.to_lowercase();
    let rest = w.strip_prefix("lorem")?;
    let (num, unit) = match rest.chars().last() {
        Some(c @ ('w' | 's' | 'p')) => (&rest[..rest.len() - 1], c),
        _ => (rest, 'w'),
    };
    if !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n = num.parse::<usize>().unwrap_or(if unit == 'w' { 30 } else { 1 });
    Some((n.max(1), unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorem_parse_and_text() {
        assert_eq!(parse("lorem"), Some((30, 'w')));
        assert_eq!(parse("Lorem10"), Some((10, 'w')));
        assert_eq!(parse("lorem3s"), Some((3, 's')));
        assert_eq!(parse("lorem2p"), Some((2, 'p')));
        assert_eq!(parse("loremx"), None);
        assert_eq!(parse("lore"), None);
        let w = text(10, 'w', false);
        assert!(w.starts_with("Lorem ipsum dolor sit amet"));
        assert_eq!(w.split_whitespace().count(), 10);
        assert!(w.ends_with('.'));
        let s3 = text(3, 's', false);
        assert_eq!(s3.matches('.').count(), 3);
        assert!(!s3.split(", ").skip(1).any(|w| w.starts_with(char::is_uppercase)));
        let p = text(2, 'p', true);
        assert_eq!(p.matches("<p>").count(), 2);
        assert_eq!(text(2, 'p', false).matches("\n\n").count(), 1);
    }
}
