// Farby kódu ako v Electron Fluxe: témy zo src/renderer/themes.js (VS Code Dark, Flux, One Dark…) prevedené
// na tému pre syntect. Monaco má vlastné tokeny, tu sú to rozsahy TextMate – priradené podľa významu.
use crate::gen;
use eframe::egui;
use egui_extras::syntax_highlighting::{CodeTheme, SyntectSettings};
use std::str::FromStr;
use syntect::highlighting::{Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSet, ThemeSettings};

pub struct Code {
    pub dark: bool,
    // syntect sa načíta až pri prvom kóde a uvoľní sa, keď sa dlho nepoužíva (definície jazykov ~11 MB)
    settings: std::cell::RefCell<Option<SyntectSettings>>,
    flux_theme: Theme,
    theme: CodeTheme,
    used: std::cell::Cell<Option<std::time::Instant>>,
    gen: u64, // nová téma = nové farby v Hl
}

static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
const CHECK: usize = 16; // stav parsera sa pamätá každých 16 riadkov

// Farby po riadkoch pre editor: počíta sa len po riadok, ktorý je vidieť (+ kúsok pod ním), a pri úprave
// len od zmeneného riadku ďalej – veľký súbor sa nefarbí celý pri každom písmene ani pri každej snímke.
#[derive(Default)]
pub struct Hl {
    src: String,                       // text, ku ktorému patria farby
    pub starts: Vec<usize>,            // začiatok každého riadku (bajty)
    spans: Vec<Vec<(u32, egui::Color32)>>, // ofarbené riadky: (dĺžka v bajtoch, farba)
    checks: Vec<(syntect::parsing::ParseState, syntect::highlighting::HighlightState)>, // stav na začiatku riadku i*CHECK
    lang: String,
    code_gen: u64,
    pub gen: u64,      // pribúda pri každej zmene textu
    words: (u64, usize), // počet slov pre stavový riadok (gen, počet)
    // rozložený text editora pre túto verziu textu: (gen, šírka zalomenia, písmo, výška riadku) → bez nového rozkladania
    pub galley: Option<(u64, f32, egui::FontId, f32, std::sync::Arc<egui::Galley>)>,
}

impl Hl {
    pub fn lines(&self) -> usize {
        self.starts.len()
    }
    pub fn same_text(&self, text: &str) -> bool {
        !self.starts.is_empty() && self.src == text
    }
    pub fn line_text(&self, i: usize) -> &str {
        let a = self.starts[i];
        let b = self.starts.get(i + 1).copied().unwrap_or(self.src.len());
        &self.src[a..b]
    }
    pub fn spans(&self, i: usize) -> Option<&[(u32, egui::Color32)]> {
        self.spans.get(i).map(|v| v.as_slice())
    }
    pub fn done(&self) -> usize {
        self.spans.len()
    }
    // počet slov – prepočíta sa len po zmene textu
    pub fn words(&mut self) -> usize {
        if self.words.0 != self.gen {
            self.words = (self.gen, self.src.split_whitespace().count());
        }
        self.words.1
    }
}

fn col(c: u32) -> Color {
    Color { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8, a: 255 }
}

// téma podľa settings.codeTheme (rovnaký kľúč ako v Electron Fluxe)
pub fn theme_of(id: &str) -> (bool, [u32; 16], bool) {
    gen::code_theme(id).or_else(|| crate::plugins::theme(id)).or_else(|| gen::code_theme(gen::DEFAULT_THEME)).expect("default theme")
}

impl Code {
    pub fn new(id: &str) -> Self {
        let (dark, c, _italic) = theme_of(id);
        let k = |name: &str| c[gen::THEME_KEYS.iter().position(|x| *x == name).unwrap_or(0)];
        // (rozsahy, farba) – konkrétnejší rozsah vyhráva
        let rules: &[(&str, &str)] = &[
            ("comment, punctuation.definition.comment", "comment"),
            ("keyword, storage.type, storage.modifier, keyword.control, keyword.operator.logical, keyword.operator.word, keyword.other", "keyword"),
            ("keyword.operator, punctuation, meta.brace, punctuation.separator, punctuation.accessor", "delimiter"),
            ("string, punctuation.definition.string, constant.character.escape", "string"),
            ("constant.numeric", "number"),
            ("constant.language, variable.language, storage.type.primitive", "storage"),
            ("constant.other, variable.other.constant", "constant"),
            ("entity.name.type, entity.name.class, support.class, support.type, entity.other.inherited-class, storage.type.class.jsdoc", "type"),
            ("entity.name.function, support.function, variable.function, meta.function-call.generic, entity.name.function.decorator", "function"),
            ("meta.generic-name, variable, variable.other, variable.other.readwrite", "variable"),
            ("variable.parameter", "parameter"),
            ("variable.other.member, variable.other.property, meta.property-name, support.type.property-name, meta.object-literal.key", "property"),
            ("entity.name.tag, punctuation.definition.tag", "tag"),
            ("entity.other.attribute-name", "attr"),
            ("string.regexp", "regexp"),
            ("markup.heading, entity.name.section", "keyword"),
        ];
        let scopes = rules
            .iter()
            .filter_map(|(sel, key)| Some(ThemeItem { scope: ScopeSelectors::from_str(sel).ok()?, style: StyleModifier { foreground: Some(col(k(key))), background: None, font_style: None } }))
            .collect();
        let theme = Theme { name: Some(id.into()), author: None, settings: ThemeSettings { foreground: Some(col(k("fg"))), ..Default::default() }, scopes };
        Code { dark, settings: Default::default(), flux_theme: theme, theme: if dark { CodeTheme::dark(14.0) } else { CodeTheme::light(14.0) }, used: Default::default(), gen: GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    // zapamätané (egui cache podľa textu a adresy nastavení), dá sa volať každý snímok
    pub fn highlight(&self, ctx: &egui::Context, style: &egui::Style, code: &str, lang: &str) -> egui::text::LayoutJob {
        self.used.set(Some(std::time::Instant::now()));
        let mut s = self.settings.borrow_mut();
        let set = s.get_or_insert_with(|| SyntectSettings { ps: syntect::parsing::SyntaxSet::load_defaults_newlines(), ts: self.theme_set() });
        egui_extras::syntax_highlighting::highlight_with(ctx, style, &self.theme, code, lang, set)
    }

    // zosúladí Hl s textom: pri zmene zahodí farby od zmeneného riadku (zvyšok sa dofarbí, keď bude treba)
    pub fn sync(&self, hl: &mut Hl, text: &str, lang: &str) {
        if hl.code_gen != self.gen || hl.lang != lang {
            *hl = Hl { lang: lang.to_string(), code_gen: self.gen, ..Default::default() };
        } else if hl.src == text && !hl.starts.is_empty() {
            return;
        }
        // prvý rozdielny bajt → riadok, od ktorého farby neplatia
        let same = hl.src.bytes().zip(text.bytes()).take_while(|(a, b)| a == b).count();
        let first = match hl.starts.binary_search(&same) {
            Ok(i) => i.saturating_sub(1),
            Err(i) => i.saturating_sub(1),
        };
        hl.gen += 1;
        hl.src.clear();
        hl.src.push_str(text);
        hl.starts.clear();
        hl.starts.push(0);
        hl.starts.extend(text.match_indices('\n').map(|(i, _)| i + 1).filter(|i| *i < text.len()));
        let keep = (first / CHECK) * CHECK; // od posledného uloženého stavu pred zmenou
        hl.spans.truncate(keep.min(hl.spans.len()));
        hl.checks.truncate(keep / CHECK + 1);
    }

    // dofarbí riadky do `upto` (vrátane), najviac `budget` času; true = hotovo
    pub fn ensure(&self, hl: &mut Hl, upto: usize, budget: std::time::Duration) -> bool {
        use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
        use syntect::parsing::{ParseState, ScopeStack};
        let upto = upto.min(hl.lines().saturating_sub(1));
        if hl.lines() == 0 || hl.done() > upto {
            return true;
        }
        self.used.set(Some(std::time::Instant::now()));
        let t0 = std::time::Instant::now();
        let mut s = self.settings.borrow_mut();
        let set = s.get_or_insert_with(|| SyntectSettings { ps: syntect::parsing::SyntaxSet::load_defaults_newlines(), ts: self.theme_set() });
        let ps = &set.ps;
        let syntax = ps.find_syntax_by_name(&hl.lang).or_else(|| ps.find_syntax_by_extension(&hl.lang)).unwrap_or_else(|| ps.find_syntax_plain_text());
        let hi = Highlighter::new(&self.flux_theme);
        let fg = self.flux_theme.settings.foreground.map(|c| egui::Color32::from_rgb(c.r, c.g, c.b)).unwrap_or(egui::Color32::GRAY);
        let mut i = hl.done();
        // stav na začiatku riadku i: z posledného uloženého, prípadne dopočítať pár riadkov
        let cp = i / CHECK;
        let (mut parse, mut hs) = if let Some(c) = hl.checks.get(cp).cloned() { c } else { (ParseState::new(syntax), HighlightState::new(&hi, ScopeStack::new())) };
        hl.checks.truncate(cp + 1);
        if hl.checks.is_empty() {
            hl.checks.push((parse.clone(), hs.clone()));
        }
        let from = cp * CHECK;
        hl.spans.truncate(from);
        i = from;
        let mut n = 0usize;
        while i <= upto || (i % CHECK != 0 && i < hl.lines()) {
            if i >= hl.lines() {
                break;
            }
            if i % CHECK == 0 && i / CHECK >= hl.checks.len() {
                hl.checks.push((parse.clone(), hs.clone()));
            }
            let line = hl.line_text(i).to_string();
            let mut out = vec![];
            match parse.parse_line(&line, ps) {
                Ok(ops) => {
                    for (style, piece) in HighlightIterator::new(&mut hs, &ops, &line, &hi) {
                        let c = egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                        match out.last_mut() {
                            Some((len, lc)) if *lc == c => *len += piece.len() as u32,
                            _ => out.push((piece.len() as u32, c)),
                        }
                    }
                }
                Err(_) => out.push((line.len() as u32, fg)),
            }
            hl.spans.push(out);
            i += 1;
            n += 1;
            // časový limit kontrolovať len občas; zastaviť na hranici uloženého stavu
            if n % 64 == 0 && t0.elapsed() > budget && i % CHECK == 0 {
                return false;
            }
        }
        hl.done() > upto
    }

    // egui_extras vyberá tému podľa mena – obe mená (tmavá/svetlá) ukazujú na tú istú tému Fluxu
    fn theme_set(&self) -> ThemeSet {
        let mut ts = ThemeSet::new();
        ts.themes.insert("base16-mocha.dark".into(), self.flux_theme.clone());
        ts.themes.insert("Solarized (light)".into(), self.flux_theme.clone());
        ts
    }

    // kód nebol na obrazovke dlhšie ako `idle` → uvoľniť syntect (pri ďalšom kóde sa načíta znova)
    pub fn release_if_idle(&self, idle: std::time::Duration) {
        if self.used.get().is_some_and(|t| t.elapsed() > idle) {
            self.used.set(None);
            *self.settings.borrow_mut() = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(code: &Code, text: &str) -> Vec<Vec<(u32, egui::Color32)>> {
        let mut h = Hl::default();
        code.sync(&mut h, text, "py");
        assert!(code.ensure(&mut h, usize::MAX, std::time::Duration::from_secs(60)));
        (0..h.lines()).map(|i| h.spans(i).unwrap().to_vec()).collect()
    }

    #[test]
    fn hl_incremental_equals_full() {
        let code = Code::new(gen::DEFAULT_THEME);
        let base: String = (0..200).map(|i| format!("def f{i}(x):\n    return 'a{i}'  # c\n")).collect();
        let mut h = Hl::default();
        code.sync(&mut h, &base, "py");
        assert!(code.ensure(&mut h, usize::MAX, std::time::Duration::from_secs(60)));
        // otvorený reťazec v strede zmení farby všetkého pod ním
        let edited = base.replacen("def f50(x):", "s = \"\"\"\ndef f50(x):", 1);
        code.sync(&mut h, &edited, "py");
        assert!(h.done() <= 110, "cache kept only lines before the edit");
        assert!(code.ensure(&mut h, usize::MAX, std::time::Duration::from_secs(60)));
        let inc: Vec<_> = (0..h.lines()).map(|i| h.spans(i).unwrap().to_vec()).collect();
        assert_eq!(inc, all(&code, &edited));
        // a späť
        code.sync(&mut h, &base, "py");
        assert!(code.ensure(&mut h, usize::MAX, std::time::Duration::from_secs(60)));
        let back: Vec<_> = (0..h.lines()).map(|i| h.spans(i).unwrap().to_vec()).collect();
        assert_eq!(back, all(&code, &base));
    }

    #[test]
    fn hl_window_only() {
        let code = Code::new(gen::DEFAULT_THEME);
        let text: String = (0..5000).map(|i| format!("x{i} = {i}\n")).collect();
        let mut h = Hl::default();
        code.sync(&mut h, &text, "py");
        assert!(code.ensure(&mut h, 40, std::time::Duration::from_secs(60)));
        assert!(h.done() > 40 && h.done() < 100, "only the visible window: {}", h.done());
        assert_eq!(h.line_text(3), "x3 = 3\n");
    }
}
