#![allow(dead_code)] // Option adapters are wired by the coordinator when the Python parser API lands.

use kordoc_ir::{PageNumber, PageSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidPageNumber;

/// Translate numeric values from the Python/options adapter into the shared IR option type.
///
/// Values stay fractional here; format parsers apply their range and rounding rules once they
/// know the document's page count.
pub(crate) fn numeric_pages(values: Vec<f64>) -> Result<PageSelection, InvalidPageNumber> {
    values
        .into_iter()
        .map(|value| PageNumber::new(value).ok_or(InvalidPageNumber))
        .collect::<Result<Vec<_>, _>>()
        .map(PageSelection::Numbers)
}

#[cfg(test)]
mod tests {
    use super::{InvalidPageNumber, numeric_pages};
    use kordoc_ir::PageSelection;

    #[test]
    fn omitted_options_stay_unset_and_fractional_page_values_are_preserved() {
        let pages = numeric_pages(vec![1.5, 3.0]).unwrap();
        let PageSelection::Numbers(pages) = pages else {
            panic!("numeric pages translated to the wrong variant");
        };
        assert_eq!(
            pages.iter().map(|page| page.get()).collect::<Vec<_>>(),
            [1.5, 3.0]
        );
        assert!(matches!(
            numeric_pages(vec![f64::INFINITY]),
            Err(InvalidPageNumber)
        ));
        assert!(matches!(
            numeric_pages(vec![f64::NAN]),
            Err(InvalidPageNumber)
        ));
    }
}
