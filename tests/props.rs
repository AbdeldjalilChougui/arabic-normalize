//! Property tests over random mixes of Arabic letters, variants, marks,
//! digits, Persian letters, invisible characters, ASCII and arbitrary Unicode.

use arabic_normalize::{is_bidi_mark, is_diacritic, stem, Builder, Config, Digits, Normalizer, TATWEEL};
use proptest::prelude::*;
use std::borrow::Cow;

fn interesting_char() -> impl Strategy<Value = char> {
    prop_oneof![
        prop::sample::select(vec![
            'ا', 'أ', 'إ', 'آ', 'ٱ', 'ب', 'ت', 'ة', 'ه', 'ى', 'ي', 'ؤ', 'ئ', 'ء', 'و', 'ك', 'ل', 'م', 'ن',
            'ک', 'ی', 'ے', 'ۀ', 'ہ', 'ھ', 'ۃ', 'پ', 'گ',
            '\u{064B}', '\u{064E}', '\u{0650}', '\u{0651}', '\u{0652}', '\u{0670}', '\u{06D6}', '\u{0654}',
            TATWEEL, '\u{200C}', '\u{200F}', '\u{061C}', '\u{FEFF}',
            '٠', '٥', '٩', '۰', '۴', '۹', '0', '5', '9',
            'a', 'Z', ' ', '-', '،', '.',
        ]),
        any::<char>(),
    ]
}

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(interesting_char(), 0..40).prop_map(|v| v.into_iter().collect())
}

fn config() -> impl Strategy<Value = Config> {
    (any::<[bool; 8]>(), 0..3u8).prop_map(|(f, d)| Config {
        diacritics: f[0],
        tatweel: f[1],
        alef: f[2],
        alef_maqsura: f[3],
        ta_marbuta: f[4],
        hamza: f[5],
        persian: f[6],
        bidi_marks: f[7],
        digits: [Digits::Keep, Digits::Western, Digits::ArabicIndic][d as usize],
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn idempotent(cfg in config(), s in text()) {
        let n = Normalizer::new(cfg);
        let once = n.normalize(&s).into_owned();
        let twice = n.normalize(&once);
        prop_assert_eq!(&once, &twice);
        // and the second pass must be the zero-copy one
        prop_assert!(matches!(twice, Cow::Borrowed(_)));
    }

    #[test]
    fn borrowed_iff_unchanged(cfg in config(), s in text()) {
        let n = Normalizer::new(cfg);
        let out = n.normalize(&s);
        match out {
            Cow::Borrowed(b) => prop_assert_eq!(b, s.as_str()),
            Cow::Owned(o) => prop_assert_ne!(o, s),
        }
    }

    #[test]
    fn never_grows(cfg in config(), s in text()) {
        let n = Normalizer::new(cfg);
        prop_assert!(n.normalize(&s).chars().count() <= s.chars().count());
    }

    #[test]
    fn removed_things_stay_removed(s in text()) {
        let out = Normalizer::search().normalize(&s).into_owned();
        for c in out.chars() {
            prop_assert!(!is_diacritic(c), "diacritic {:?} left in {:?}", c, out);
            let invisible = c == TATWEEL || is_bidi_mark(c);
            prop_assert!(!invisible, "tatweel/bidi mark left in {:?}", out);
            prop_assert!(!"أإآٱىةؤئکیۀ".contains(c), "variant {:?} left", c);
            let indic = ('\u{0660}'..='\u{0669}').contains(&c) || ('\u{06F0}'..='\u{06F9}').contains(&c);
            prop_assert!(!indic, "non-ASCII digit left in {:?}", out);
        }
    }

    #[test]
    fn normalize_into_matches_normalize(cfg in config(), s in text()) {
        let n = Normalizer::new(cfg);
        let mut buf = String::from("prefix:");
        n.normalize_into(&s, &mut buf);
        prop_assert_eq!(buf, format!("prefix:{}", n.normalize(&s)));
    }

    #[test]
    fn digit_roundtrip(digits in "[0-9 ]{0,30}") {
        let indic = Builder::from_config(Config::none()).digits(Digits::ArabicIndic).build();
        let western = Builder::from_config(Config::none()).digits(Digits::Western).build();
        let there = indic.normalize(&digits).into_owned();
        prop_assert!(!there.chars().any(|c| c.is_ascii_digit()));
        prop_assert_eq!(western.normalize(&there), digits);
    }

    #[test]
    fn stem_is_a_substring_and_nonempty(s in text()) {
        for w in arabic_normalize::tokenize(&s) {
            let st = stem::light10(w);
            prop_assert!(w.contains(st));
            prop_assert!(!st.is_empty());
        }
    }
}
