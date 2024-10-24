//! Python bindings for `arabic-normalize`.

use ::arabic_normalize as core;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn parse_digits(d: &str) -> PyResult<core::Digits> {
    match d {
        "keep" => Ok(core::Digits::Keep),
        "western" | "ascii" => Ok(core::Digits::Western),
        "arabic" | "arabic-indic" | "arabic_indic" => Ok(core::Digits::ArabicIndic),
        _ => Err(PyValueError::new_err(format!("digits must be 'keep', 'western' or 'arabic-indic', got {d:?}"))),
    }
}

/// A configured normalizer. Keyword arguments default to the conservative
/// preset: diacritics, tatweel, alef variants and invisible marks.
#[pyclass(name = "Normalizer", frozen)]
struct PyNormalizer {
    inner: core::Normalizer,
}

#[pymethods]
impl PyNormalizer {
    #[new]
    #[pyo3(signature = (*, diacritics=true, tatweel=true, alef=true, alef_maqsura=false, ta_marbuta=false,
                        hamza=false, persian=false, bidi_marks=true, digits="keep"))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        diacritics: bool,
        tatweel: bool,
        alef: bool,
        alef_maqsura: bool,
        ta_marbuta: bool,
        hamza: bool,
        persian: bool,
        bidi_marks: bool,
        digits: &str,
    ) -> PyResult<Self> {
        let config = core::Config {
            diacritics,
            tatweel,
            alef,
            alef_maqsura,
            ta_marbuta,
            hamza,
            persian,
            bidi_marks,
            digits: parse_digits(digits)?,
        };
        Ok(PyNormalizer { inner: core::Normalizer::new(config) })
    }

    /// The aggressive preset used for search indexing.
    #[staticmethod]
    fn search() -> Self {
        PyNormalizer { inner: core::Normalizer::search() }
    }

    fn normalize(&self, py: Python<'_>, text: &str) -> String {
        py.detach(|| self.inner.normalize(text).into_owned())
    }

    /// Normalize a list of strings, releasing the GIL while working.
    fn normalize_many(&self, py: Python<'_>, texts: Vec<String>) -> Vec<String> {
        py.detach(|| texts.iter().map(|t| self.inner.normalize(t).into_owned()).collect())
    }

    fn search_terms(&self, text: &str) -> Vec<String> {
        self.inner.search_terms(text)
    }

    fn __call__(&self, text: &str) -> String {
        self.inner.normalize(text).into_owned()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner.config())
    }
}

/// Normalize with the conservative default preset.
#[pyfunction]
fn normalize(text: &str) -> String {
    core::normalize(text).into_owned()
}

/// Normalize with the aggressive search preset.
#[pyfunction]
fn normalize_for_search(text: &str) -> String {
    core::Normalizer::search().normalize(text).into_owned()
}

#[pyfunction]
fn strip_diacritics(text: &str) -> String {
    core::strip_diacritics(text).into_owned()
}

#[pyfunction]
fn to_western_digits(text: &str) -> String {
    core::to_western_digits(text).into_owned()
}

#[pyfunction]
fn to_arabic_indic_digits(text: &str) -> String {
    core::to_arabic_indic_digits(text).into_owned()
}

/// Light10 stem of one (normalized) word.
#[pyfunction]
fn light_stem(word: &str) -> String {
    core::stem::light10(word).to_string()
}

/// Search-normalize, tokenize and light-stem.
#[pyfunction]
fn search_terms(text: &str) -> Vec<String> {
    core::Normalizer::search().search_terms(text)
}

#[pymodule]
fn arabic_normalize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNormalizer>()?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_for_search, m)?)?;
    m.add_function(wrap_pyfunction!(strip_diacritics, m)?)?;
    m.add_function(wrap_pyfunction!(to_western_digits, m)?)?;
    m.add_function(wrap_pyfunction!(to_arabic_indic_digits, m)?)?;
    m.add_function(wrap_pyfunction!(light_stem, m)?)?;
    m.add_function(wrap_pyfunction!(search_terms, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
