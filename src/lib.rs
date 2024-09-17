//! Fast, configurable Arabic text normalization for search and matching.
//!
//! ```
//! use arabic_normalize::{Normalizer, Digits};
//!
//! let n = Normalizer::builder()
//!     .diacritics(true)
//!     .tatweel(true)
//!     .alef(true)
//!     .digits(Digits::Western)
//!     .build();
//! assert_eq!(n.normalize("أَحْمَــد ٢٠٢٤"), "احمد 2024");
//!
//! // Text that needs no change is returned borrowed (no allocation).
//! assert!(matches!(n.normalize("hello"), std::borrow::Cow::Borrowed(_)));
//!
//! // Aggressive preset for search indexing, plus light stemming.
//! let s = Normalizer::search();
//! assert_eq!(s.search_terms("وَالمَكْتَبَاتُ الجامِعِيَّة"), vec!["مكتب", "جامع"]);
//! ```
//!
//! Every mapping is one char to at most one char, so normalization never
//! grows the text and is idempotent (property-tested).

pub mod stem;

use std::borrow::Cow;

/// What to do with digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Digits {
    /// Leave all digits alone.
    #[default]
    Keep,
    /// Arabic-Indic (٠-٩) and Eastern/Persian (۰-۹) digits → ASCII 0-9.
    Western,
    /// ASCII and Eastern/Persian digits → Arabic-Indic ٠-٩.
    ArabicIndic,
}

/// Which normalizations are enabled. Use [`Normalizer::builder`] to build one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Remove harakat (fatha, damma, kasra, tanwin, shadda, sukun), the dagger
    /// alef, combining maddah/hamza and Quranic annotation marks.
    pub diacritics: bool,
    /// Remove tatweel / kashida (ـ).
    pub tatweel: bool,
    /// أ إ آ ٱ ٲ ٳ → ا
    pub alef: bool,
    /// ى (alef maqsura) → ي
    pub alef_maqsura: bool,
    /// ة → ه
    pub ta_marbuta: bool,
    /// ؤ → و, ئ → ي
    pub hamza: bool,
    /// Persian/Urdu letter forms → Arabic: ک→ك, ی ے→ي, ۀ ہ ھ→ه, ۃ→ة
    pub persian: bool,
    /// Remove zero-width and bidi control characters (ZWNJ, ZWJ, LRM, RLM, ALM, BOM).
    pub bidi_marks: bool,
    pub digits: Digits,
}

impl Default for Config {
    /// Conservative defaults: remove diacritics, tatweel and invisible marks,
    /// unify alef forms. Letters that change meaning (ة/ه, ى/ي) are kept.
    fn default() -> Self {
        Config {
            diacritics: true,
            tatweel: true,
            alef: true,
            alef_maqsura: false,
            ta_marbuta: false,
            hamza: false,
            persian: false,
            bidi_marks: true,
            digits: Digits::Keep,
        }
    }
}

impl Config {
    /// Everything on, digits to ASCII: the usual choice for search indexes.
    pub fn search() -> Config {
        Config {
            diacritics: true,
            tatweel: true,
            alef: true,
            alef_maqsura: true,
            ta_marbuta: true,
            hamza: true,
            persian: true,
            bidi_marks: true,
            digits: Digits::Western,
        }
    }

    /// Everything off.
    pub fn none() -> Config {
        Config {
            diacritics: false,
            tatweel: false,
            alef: false,
            alef_maqsura: false,
            ta_marbuta: false,
            hamza: false,
            persian: false,
            bidi_marks: false,
            digits: Digits::Keep,
        }
    }
}

/// Is `c` an Arabic diacritic / Quranic mark removed by `diacritics`?
pub fn is_diacritic(c: char) -> bool {
    matches!(c,
        '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}'
        | '\u{0670}'
        | '\u{06D6}'..='\u{06DC}'
        | '\u{06DF}'..='\u{06E4}'
        | '\u{06E7}'..='\u{06E8}'
        | '\u{06EA}'..='\u{06ED}'
        | '\u{08D3}'..='\u{08E1}'
        | '\u{08E3}'..='\u{08FF}')
}

pub const TATWEEL: char = '\u{0640}';

pub fn is_bidi_mark(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

const KEEP: u32 = 0;
const DROP: u32 = 1;

/// A compiled normalizer: the config is turned into a 256-entry lookup table
/// for the Arabic block (U+0600–U+06FF), so the hot loop is one table read per char.
#[derive(Clone)]
pub struct Normalizer {
    config: Config,
    /// Action for U+0600 + i: KEEP, DROP, or the replacement char's code point.
    table: [u32; 256],
}

impl std::fmt::Debug for Normalizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Normalizer").field("config", &self.config).finish()
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Normalizer::new(Config::default())
    }
}

/// Builder for [`Normalizer`], starting from [`Config::default`].
#[derive(Debug, Clone, Default)]
pub struct Builder {
    config: Config,
}

macro_rules! builder_flags {
    ($($(#[$m:meta])* $name:ident),*) => {
        $( $(#[$m])* pub fn $name(mut self, on: bool) -> Self { self.config.$name = on; self } )*
    };
}

impl Builder {
    builder_flags!(diacritics, tatweel, alef, alef_maqsura, ta_marbuta, hamza, persian, bidi_marks);

    pub fn digits(mut self, d: Digits) -> Self {
        self.config.digits = d;
        self
    }

    /// Start from another config (e.g. [`Config::none`] or [`Config::search`]).
    pub fn from_config(config: Config) -> Self {
        Builder { config }
    }

    pub fn build(self) -> Normalizer {
        Normalizer::new(self.config)
    }
}

impl Normalizer {
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// The aggressive [`Config::search`] preset.
    pub fn search() -> Normalizer {
        Normalizer::new(Config::search())
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn new(config: Config) -> Normalizer {
        let mut table = [KEEP; 256];
        let mut set = |c: char, action: u32| table[c as usize - 0x600] = action;
        if config.diacritics {
            for i in 0x600..0x700u32 {
                let c = char::from_u32(i).unwrap();
                if is_diacritic(c) {
                    set(c, DROP);
                }
            }
        }
        if config.tatweel {
            set(TATWEEL, DROP);
        }
        if config.bidi_marks {
            set('\u{061C}', DROP);
        }
        if config.alef {
            for c in ['آ', 'أ', 'إ', 'ٱ', 'ٲ', 'ٳ'] {
                set(c, 'ا' as u32);
            }
        }
        if config.hamza {
            set('ؤ', 'و' as u32);
            set('ئ', 'ي' as u32);
        }
        if config.persian {
            set('ک', 'ك' as u32);
            set('ی', 'ي' as u32);
            set('ے', 'ي' as u32);
            set('ۀ', 'ه' as u32);
            set('ہ', 'ه' as u32);
            set('ھ', 'ه' as u32);
            set('ۃ', if config.ta_marbuta { 'ه' } else { 'ة' } as u32);
        }
        if config.alef_maqsura {
            set('ى', 'ي' as u32);
        }
        if config.ta_marbuta {
            set('ة', 'ه' as u32);
        }
        for d in 0..10u32 {
            let arabic = char::from_u32(0x660 + d).unwrap();
            let persian = char::from_u32(0x6F0 + d).unwrap();
            match config.digits {
                Digits::Keep => {}
                Digits::Western => {
                    set(arabic, '0' as u32 + d);
                    set(persian, '0' as u32 + d);
                }
                Digits::ArabicIndic => set(persian, 0x660 + d),
            }
        }
        Normalizer { config, table }
    }

    /// What happens to `c`: `None` = keep, `Some(None)` = drop, `Some(Some(r))` = replace.
    #[inline]
    fn action(&self, c: char) -> Option<Option<char>> {
        let u = c as u32;
        if (0x600..0x700).contains(&u) {
            return match self.table[(u - 0x600) as usize] {
                KEEP => None,
                DROP => Some(None),
                r => Some(char::from_u32(r)),
            };
        }
        if u < 0x80 {
            if self.config.digits == Digits::ArabicIndic && c.is_ascii_digit() {
                return Some(char::from_u32(0x660 + (u - '0' as u32)));
            }
            return None;
        }
        if (self.config.diacritics && is_diacritic(c)) || (self.config.bidi_marks && is_bidi_mark(c)) {
            return Some(None);
        }
        None
    }

    /// Normalize `text`. Returns `Cow::Borrowed` when nothing changes, so
    /// already-clean text costs one scan and no allocation.
    pub fn normalize<'a>(&self, text: &'a str) -> Cow<'a, str> {
        // Fast path: pure ASCII only changes if ASCII digits must become Arabic-Indic.
        if self.config.digits != Digits::ArabicIndic && text.is_ascii() {
            return Cow::Borrowed(text);
        }
        for (i, c) in text.char_indices() {
            if self.action(c).is_some() {
                let mut out = String::with_capacity(text.len());
                out.push_str(&text[..i]);
                self.push_rest(&text[i..], &mut out);
                return Cow::Owned(out);
            }
        }
        Cow::Borrowed(text)
    }

    /// Normalize `text`, appending to `out` (reuses `out`'s allocation).
    pub fn normalize_into(&self, text: &str, out: &mut String) {
        out.reserve(text.len());
        self.push_rest(text, out);
    }

    fn push_rest(&self, text: &str, out: &mut String) {
        for c in text.chars() {
            match self.action(c) {
                None => out.push(c),
                Some(Some(r)) => out.push(r),
                Some(None) => {}
            }
        }
    }

    /// Would `normalize` change this text?
    pub fn needs_normalization(&self, text: &str) -> bool {
        matches!(self.normalize(text), Cow::Owned(_))
    }

    /// Normalize, split into words and light-stem each word (see [`stem::light10`]).
    /// Intended for building and querying search indexes.
    pub fn search_terms(&self, text: &str) -> Vec<String> {
        let norm = self.normalize(text);
        tokenize(&norm)
            .map(stem::light10)
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect()
    }
}

/// Is `c` part of a word? Letters, digits, and the marks a normalizer may not have removed.
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_diacritic(c) || c == TATWEEL
}

/// Split text into words (zero-copy).
pub fn tokenize(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !is_word_char(c)).filter(|w| !w.is_empty())
}

/// Shorthand: normalize with the default config.
pub fn normalize(text: &str) -> Cow<'_, str> {
    Normalizer::default().normalize(text)
}

/// Shorthand: remove only diacritics.
pub fn strip_diacritics(text: &str) -> Cow<'_, str> {
    Builder::from_config(Config::none()).diacritics(true).build().normalize(text)
}

/// Shorthand: convert Arabic-Indic / Persian digits to ASCII, nothing else.
pub fn to_western_digits(text: &str) -> Cow<'_, str> {
    Builder::from_config(Config::none()).digits(Digits::Western).build().normalize(text)
}

/// Shorthand: convert ASCII / Persian digits to Arabic-Indic, nothing else.
pub fn to_arabic_indic_digits(text: &str) -> Cow<'_, str> {
    Builder::from_config(Config::none()).digits(Digits::ArabicIndic).build().normalize(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_driven_cases() {
        let search = Normalizer::search();
        let default = Normalizer::default();
        let cases: &[(&Normalizer, &str, &str)] = &[
            (&default, "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ", "بسم الله الرحمن الرحيم"),
            (&default, "مـــحـــمـــد", "محمد"),
            (&default, "أحمد إبراهيم آمال ٱلله", "احمد ابراهيم امال الله"),
            (&default, "مدرسة على", "مدرسة على"),
            (&search, "مدرسة على", "مدرسه علي"),
            (&search, "مسؤول بئر", "مسوول بير"),
            (&search, "کتاب فارسی", "كتاب فارسي"),
            (&search, "١٢٣ ۴۵۶ 789", "123 456 789"),
            (&default, "a\u{200F}b\u{200C}c", "abc"),
            (&search, "الْقُرْآنُ", "القران"),
            (&default, "", ""),
        ];
        for (n, input, want) in cases {
            assert_eq!(n.normalize(input), *want, "input {input:?}");
        }
    }

    #[test]
    fn readme_example() {
        let n = Normalizer::builder().alef_maqsura(true).ta_marbuta(true).digits(Digits::Western).build();
        assert_eq!(n.normalize("مَدْرَسَةُ عَلَى الطَّرِيقِ ١٢"), "مدرسه علي الطريق 12");
    }

    #[test]
    fn digits_both_ways() {
        assert_eq!(to_western_digits("٠١٢٣٤٥٦٧٨٩ ۰۱۲۳۴۵۶۷۸۹"), "0123456789 0123456789");
        assert_eq!(to_arabic_indic_digits("Tel 0555-12 ۳۴"), "Tel ٠٥٥٥-١٢ ٣٤");
    }

    #[test]
    fn borrowed_when_unchanged() {
        let n = Normalizer::default();
        assert!(matches!(n.normalize("plain ascii 123"), Cow::Borrowed(_)));
        assert!(matches!(n.normalize("كتاب جميل"), Cow::Borrowed(_)));
        assert!(matches!(n.normalize("كِتاب"), Cow::Owned(_)));
        let indic = Builder::from_config(Config::none()).digits(Digits::ArabicIndic).build();
        assert!(matches!(indic.normalize("abc"), Cow::Borrowed(_)));
        assert!(matches!(indic.normalize("a1"), Cow::Owned(_)));
    }

    #[test]
    fn builder_toggles() {
        let n = Normalizer::builder().diacritics(false).build();
        assert_eq!(n.normalize("كِتَابٌ"), "كِتَابٌ");
        let n = Builder::from_config(Config::none()).tatweel(true).build();
        assert_eq!(n.normalize("أَمـير"), "أَمير");
    }

    #[test]
    fn search_terms_stem() {
        let n = Normalizer::search();
        assert_eq!(n.search_terms("والكتاب، للمعلمين!"), vec!["كتاب", "معلم"]);
        assert_eq!(n.search_terms("ABC 2024"), vec!["ABC", "2024"]);
    }
}
