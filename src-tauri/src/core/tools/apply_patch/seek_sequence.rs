//! Locate a sequence of lines within a file with progressive fuzzy matching.

pub fn seek_sequence(
    lines: &[String],
    pattern: &[String],
    start: usize,
    eof: bool,
) -> Result<Option<usize>, usize> {
    if pattern.is_empty() {
        return Ok(Some(start));
    }
    if pattern.len() > lines.len() {
        return Ok(None);
    }
    let search_start = if eof && lines.len() >= pattern.len() {
        lines.len() - pattern.len()
    } else {
        start
    };

    let end = lines.len().saturating_sub(pattern.len());
    let exact = unique_position(search_start, end, |i| {
        lines[i..i + pattern.len()] == *pattern
    });
    if exact != Ok(None) {
        return exact;
    }
    let trailing = unique_position(search_start, end, |i| {
        for (p_idx, pat) in pattern.iter().enumerate() {
            if lines[i + p_idx].trim_end() != pat.trim_end() {
                return false;
            }
        }
        true
    });
    if trailing != Ok(None) {
        return trailing;
    }
    let trimmed = unique_position(search_start, end, |i| {
        for (p_idx, pat) in pattern.iter().enumerate() {
            if lines[i + p_idx].trim() != pat.trim() {
                return false;
            }
        }
        true
    });
    if trimmed != Ok(None) {
        return trimmed;
    }

    fn normalise(s: &str) -> String {
        s.trim()
            .chars()
            .map(|c| match c {
                '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
                | '\u{2212}' => '-',
                '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
                '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
                '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}'
                | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{205F}'
                | '\u{3000}' => ' ',
                other => other,
            })
            .collect::<String>()
    }

    unique_position(search_start, end, |i| {
        for (p_idx, pat) in pattern.iter().enumerate() {
            if normalise(&lines[i + p_idx]) != normalise(pat) {
                return false;
            }
        }
        true
    })
}

fn unique_position(
    start: usize,
    end: usize,
    mut matches: impl FnMut(usize) -> bool,
) -> Result<Option<usize>, usize> {
    let positions = (start..=end)
        .filter(|index| matches(*index))
        .collect::<Vec<_>>();
    match positions.as_slice() {
        [] => Ok(None),
        [position] => Ok(Some(*position)),
        _ => Err(positions.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_and_trimmed() {
        let lines = vec!["  foo".into(), "bar".into()];
        assert_eq!(
            seek_sequence(&lines, &["foo".into()], 0, false),
            Ok(Some(0))
        );
        assert_eq!(
            seek_sequence(&lines, &["bar".into()], 0, false),
            Ok(Some(1))
        );
    }

    #[test]
    fn rejects_ambiguous_sequences() {
        let lines = vec!["same".into(), "other".into(), "same".into()];
        assert_eq!(seek_sequence(&lines, &["same".into()], 0, false), Err(2));
    }
}
