// Medzery ako bodky (nastavenie `whitespace`): `indent` ako VS Code „boundary“ – odsadenie, medzery za sebou a medzery na konci,
// jedna medzera medzi slovami bez bodky; `all` = každá medzera; `off` = nič.
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Mode {
    Off,
    #[default]
    Indent,
    All,
}

impl Mode {
    // `whitespace` = „off“ | „indent“ | „all“; staré `showWhitespace: false` (0.9.46) = off
    pub fn from_settings(whitespace: &Value, legacy: &Value) -> Mode {
        match whitespace.as_str() {
            Some("off") => Mode::Off,
            Some("all") => Mode::All,
            Some("indent") => Mode::Indent,
            _ if legacy.as_bool() == Some(false) => Mode::Off,
            _ => Mode::Indent,
        }
    }
}

// stĺpce (indexy znakov riadka), kde sa kreslí bodka; trailing = bodky aj na konci riadka (editor; v termináli je tam prázdna mriežka)
pub fn dot_cols(chars: &[char], mode: Mode, trailing: bool) -> Vec<usize> {
    if mode == Mode::Off {
        return vec![];
    }
    let is_sp = |c: char| c == ' ';
    let last_text = chars.iter().rposition(|c| !c.is_whitespace() && *c != '\0');
    let first_text = chars.iter().position(|c| !c.is_whitespace() && *c != '\0');
    let mut out = vec![];
    for (i, &c) in chars.iter().enumerate() {
        if !is_sp(c) {
            continue;
        }
        let (Some(first), Some(last)) = (first_text, last_text) else {
            // prázdny riadok: v editore bodky (trailing), v termináli nič
            if trailing && mode != Mode::Off {
                out.push(i);
            }
            continue;
        };
        let leading = i < first;
        let tail = i > last;
        if tail && !trailing {
            continue;
        }
        let run = (i > 0 && is_sp(chars[i - 1])) || chars.get(i + 1).is_some_and(|n| is_sp(*n));
        if mode == Mode::All || leading || tail || run {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols(s: &str, m: Mode, t: bool) -> Vec<usize> {
        dot_cols(&s.chars().collect::<Vec<_>>(), m, t)
    }

    #[test]
    fn ws_indent_rules() {
        // odsadenie áno, jedna medzera medzi slovami nie
        assert_eq!(cols("    print(f'a = {b}')", Mode::Indent, true), vec![0, 1, 2, 3]);
        // dve medzery za sebou áno
        assert_eq!(cols("a  b", Mode::Indent, true), vec![1, 2]);
        // koniec riadka len keď trailing
        assert_eq!(cols("a b  ", Mode::Indent, true), vec![3, 4]);
        assert_eq!(cols("a b  ", Mode::Indent, false), Vec::<usize>::new());
    }

    #[test]
    fn ws_all_and_off() {
        assert_eq!(cols("a b c", Mode::All, true), vec![1, 3]);
        assert_eq!(cols("  a b", Mode::Off, true), Vec::<usize>::new());
        // terminál: medzery za posledným znakom sa nekreslia ani pri „all“
        assert_eq!(cols("a b   ", Mode::All, false), vec![1]);
    }

    #[test]
    fn ws_legacy_setting() {
        assert_eq!(Mode::from_settings(&Value::Null, &Value::Bool(false)), Mode::Off);
        assert_eq!(Mode::from_settings(&Value::Null, &Value::Bool(true)), Mode::Indent);
        assert_eq!(Mode::from_settings(&Value::String("all".into()), &Value::Bool(false)), Mode::All);
    }
}
