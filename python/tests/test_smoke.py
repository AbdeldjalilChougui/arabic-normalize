"""Smoke test for the Python bindings. Runs with plain `python` or pytest."""

import arabic_normalize as an


def test_functions():
    assert an.normalize("بِسْمِ اللَّهِ") == "بسم الله"
    assert an.normalize("أحمـــد") == "احمد"
    assert an.strip_diacritics("كَتَبَ") == "كتب"
    assert an.to_western_digits("٢٠٢٤ ۱۴۰۳") == "2024 1403"
    assert an.to_arabic_indic_digits("2024") == "٢٠٢٤"
    assert an.normalize_for_search("مدرسة على") == "مدرسه علي"
    assert an.light_stem("والكتاب") == "كتاب"
    assert an.search_terms("وَالمَكْتَبَاتُ الجامِعِيَّة") == ["مكتب", "جامع"]


def test_normalizer_class():
    n = an.Normalizer(diacritics=False, digits="western")
    assert n.normalize("كِتَاب ٣") == "كِتَاب 3"
    assert n("أ") == "ا"
    assert an.Normalizer.search().normalize_many(["کتاب", "إسلام"]) == ["كتاب", "اسلام"]
    try:
        an.Normalizer(digits="roman")
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError")
    assert "diacritics" in repr(n)


if __name__ == "__main__":
    test_functions()
    test_normalizer_class()
    print("python smoke test: OK", an.__version__)
