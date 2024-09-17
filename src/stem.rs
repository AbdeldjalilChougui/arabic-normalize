//! Light stemming: strip common prefixes and suffixes without trying to find
//! the root. Implements Light10 (Larkey, Ballesteros & Connell, 2007), the
//! classic baseline for Arabic information retrieval.
//!
//! Input should already be normalized (no diacritics/tatweel). Works on
//! either ة or ه forms, since both are in the suffix list.

const ARTICLES: [&str; 6] = ["وال", "بال", "كال", "فال", "ال", "لل"];
const SUFFIXES: [&str; 10] = ["ها", "ان", "ات", "ون", "ين", "يه", "ية", "ه", "ة", "ي"];

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Light10 stem of a single word. Returns a sub-slice of the input (zero-copy).
///
/// 1. Remove a leading و ("and") if at least 3 characters remain.
/// 2. Remove one definite article (ال، وال، بال، كال، فال، لل) if at least 2 remain.
/// 3. Go once through the suffix list in order, removing each suffix found
///    at the end if at least 2 characters remain.
pub fn light10(word: &str) -> &str {
    let mut w = word;
    if let Some(rest) = w.strip_prefix('و') {
        if char_len(rest) >= 3 {
            w = rest;
        }
    }
    for art in ARTICLES {
        if let Some(rest) = w.strip_prefix(art) {
            if char_len(rest) >= 2 {
                w = rest;
                break;
            }
        }
    }
    for suf in SUFFIXES {
        if let Some(rest) = w.strip_suffix(suf) {
            if char_len(rest) >= 2 {
                w = rest;
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_stems() {
        let cases = [
            ("الكتاب", "كتاب"),
            ("والكتاب", "كتاب"),
            ("بالقلم", "قلم"),
            ("للمعلمين", "معلم"),
            ("المكتبات", "مكتب"),
            ("مدرستها", "مدرست"),
            ("المعلمون", "معلم"),
            ("الجامعية", "جامع"),
            ("فلسطيني", "فلسطين"),
            ("ولد", "ولد"),   // و not removed: only 2 chars would remain
            ("ال", "ال"),     // too short to strip
            ("كتب", "كتب"),
            ("hello", "hello"),
        ];
        for (w, want) in cases {
            assert_eq!(light10(w), want, "{w}");
        }
    }
}
