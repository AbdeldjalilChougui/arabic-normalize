# arabic-normalize

Fast, configurable Arabic text normalization for search and matching, in Rust, with
Python bindings.

- Removes diacritics (harakat, tanwin, shadda, sukun, dagger alef, Quranic marks) and
  tatweel (ـ).
- Unifies alef forms `أ إ آ ٱ → ا`. Optional: `ى → ي`, `ة → ه`, `ؤ → و`, `ئ → ي`.
- Optional Persian/Urdu letter mapping: `ک → ك`, `ی ے → ي`, `ۀ ہ ھ → ه`.
- Converts digits Arabic-Indic `٠-٩` / Persian `۰-۹` ↔ Western `0-9`.
- Strips zero-width and bidi control characters (ZWNJ, ZWJ, LRM, RLM, ALM, BOM).
- Light stemming for search (Light10: `والمكتبات → مكتب`).
- **Zero-copy fast path:** text that needs no change comes back as `Cow::Borrowed`
  with no allocation, and pure-ASCII input is detected with one SIMD-friendly check.

## Install

```sh
cargo add arabic-normalize      # Rust library
pip install arabic-normalize    # Python bindings (prebuilt wheels: Linux, macOS, Windows)
```

## Quick start (Rust)

```rust
use arabic_normalize::{Normalizer, Digits};

let n = Normalizer::builder()      // starts from the conservative defaults
    .alef_maqsura(true)
    .ta_marbuta(true)
    .digits(Digits::Western)
    .build();
assert_eq!(n.normalize("مَدْرَسَةُ عَلَى الطَّرِيقِ ١٢"), "مدرسه علي الطريق 12");

let search = Normalizer::search();  // everything on
assert_eq!(search.search_terms("وَالمَكْتَبَاتُ الجامِعِيَّة"), ["مكتب", "جامع"]);
```

Default config: diacritics, tatweel, alef forms and invisible marks. Letters that can
change meaning (`ة/ه`, `ى/ي`, hamza seats) are only changed when you ask, or by the
`search()` preset.

## Quick start (Python)

`pip install arabic-normalize`, or build from source:

```sh
cd python
uv tool install maturin
maturin build --release -o dist
uv venv && uv pip install dist/*.whl
.venv/bin/python tests/test_smoke.py
```

```python
import arabic_normalize as an

an.normalize("بِسْمِ اللَّهِ")              # 'بسم الله'
an.normalize_for_search("مدرسة على")   # 'مدرسه علي'
an.to_western_digits("٢٠٢٤")           # '2024'
an.light_stem("والكتاب")               # 'كتاب'
n = an.Normalizer(ta_marbuta=True, digits="western")
n.normalize_many(["…", "…"])           # releases the GIL while working
```

## How it works

The config is compiled into a 256-entry action table for the Arabic block
(U+0600–U+06FF): keep, drop, or replace with a code point. The hot loop is one table
read per char. Chars outside the block only need a few range checks: extended Arabic
marks, bidi controls, and ASCII digits in `ArabicIndic` mode. `normalize` scans until
the first char that changes. If there is none, it returns the input borrowed.
Otherwise it copies the clean prefix once and maps the rest. Every mapping is one char
to at most one char, so output is never longer than input.

## Tests and benchmarks

```sh
cargo test                     # unit + doc tests + proptest properties
cargo bench                    # criterion: vocalized / already-clean / ASCII text
cd python && maturin build -o dist && uv pip install dist/*.whl && .venv/bin/python tests/test_smoke.py
```

Properties checked on random strings that mix letters, variants, marks, digits,
invisible chars and arbitrary Unicode, under random configs:
- idempotence, and the second pass is always zero-copy
- `Borrowed` exactly when the text is unchanged
- the output never grows
- nothing the search preset removes survives it
- `normalize_into` matches `normalize`
- the digit round trip is lossless
- stems are non-empty substrings

## Status / limitations

- Light10 is a light stemmer: it strips affixes and does not find roots, so it
  sometimes merges unrelated words or misses related ones (broken plurals such as
  `كتب`/`كتاب` are not merged).
- Arabic presentation forms (U+FB50–U+FEFF ligatures) are not folded. Run NFKC first
  if your text contains them.
- No tokenizer beyond splitting on non-word characters, and no stopword list.

## License

MIT, see [LICENSE](LICENSE).
