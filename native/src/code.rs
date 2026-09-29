// Farby kódu ako v Electron Fluxe: témy zo src/renderer/themes.js (VS Code Dark, Flux, One Dark…) prevedené
// na tému pre syntect. Monaco má vlastné tokeny, tu sú to rozsahy TextMate – priradené podľa významu.
use crate::gen;
use eframe::egui;
use egui_extras::syntax_highlighting::{CodeTheme, SyntectSettings};
use std::str::FromStr;
use syntect::highlighting::{Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSet, ThemeSettings};

pub struct Code {
    pub dark: bool,
    settings: SyntectSettings,
    theme: CodeTheme,
}

fn col(c: u32) -> Color {
    Color { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8, a: 255 }
}

// téma podľa settings.codeTheme (rovnaký kľúč ako v Electron Fluxe)
pub fn theme_of(id: &str) -> (bool, [u32; 16], bool) {
    gen::code_theme(id).or_else(|| gen::code_theme(gen::DEFAULT_THEME)).expect("default theme")
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
        // egui_extras vyberá tému podľa mena – obe mená (tmavá/svetlá) ukazujú na tú istú tému Fluxu
        let mut ts = ThemeSet::new();
        ts.themes.insert("base16-mocha.dark".into(), theme.clone());
        ts.themes.insert("Solarized (light)".into(), theme);
        Code { dark, settings: SyntectSettings { ps: syntect::parsing::SyntaxSet::load_defaults_newlines(), ts }, theme: if dark { CodeTheme::dark(14.0) } else { CodeTheme::light(14.0) } }
    }

    // zapamätané (egui cache podľa textu a adresy nastavení), dá sa volať každý snímok
    pub fn highlight(&self, ctx: &egui::Context, style: &egui::Style, code: &str, lang: &str) -> egui::text::LayoutJob {
        egui_extras::syntax_highlighting::highlight_with(ctx, style, &self.theme, code, lang, &self.settings)
    }
}
