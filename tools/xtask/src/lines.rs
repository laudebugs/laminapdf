//! Byte offsets to 1-based line numbers, for checks that report `path:line`.

/// The byte offset where each line of `text` starts.
pub(crate) fn starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

/// The 1-based line that holds byte `offset`, given the line starts from [`starts`].
pub(crate) fn line_of(starts: &[usize], offset: usize) -> usize {
    starts.partition_point(|&start| start <= offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_should_map_to_one_based_lines() {
        let starts = starts("ab\ncd\n\nef");
        assert_eq!([0, 2, 3, 6, 7].map(|offset| line_of(&starts, offset)), [1, 1, 2, 3, 4]);
    }
}
