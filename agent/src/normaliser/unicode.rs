#[must_use]
pub fn normalize_unicode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());

    for ch in input.chars() {
        // 1. Strip zero-width characters
        match ch {
            '\u{200B}' // Zero Width Space
            | '\u{200C}' // Zero Width Non-Joiner (ZWNJ)
            | '\u{200D}' // Zero Width Joiner (ZWJ)
            | '\u{2060}' // Word Joiner
            | '\u{FEFF}' // Byte Order Mark (BOM) / Zero Width No-Break Space
            => continue,
            _ => {}
        }

        // 2. Fold common homoglyphs into ASCII equivalents
        let folded = match ch {
            '\u{0430}' => 'a', // Cyrillic small letter a
            '\u{0410}' => 'A', // Cyrillic capital letter A
            '\u{0435}' => 'e', // Cyrillic small letter ie
            '\u{0415}' => 'E', // Cyrillic capital letter IE
            '\u{043E}' => 'o', // Cyrillic small letter o
            '\u{041E}' => 'O', // Cyrillic capital letter O
            '\u{0440}' => 'p', // Cyrillic small letter er
            '\u{0420}' => 'P', // Cyrillic capital letter ER
            '\u{0441}' => 'c', // Cyrillic small letter es
            '\u{0421}' => 'C', // Cyrillic capital letter ES
            '\u{0443}' => 'y', // Cyrillic small letter u
            '\u{0423}' => 'Y', // Cyrillic capital letter U
            '\u{0445}' => 'x', // Cyrillic small letter ha
            '\u{0425}' => 'X', // Cyrillic capital letter HA
            '\u{0456}' => 'i', // Cyrillic small letter Ukrainian i
            '\u{0406}' => 'I', // Cyrillic capital letter Ukrainian I
            other => other,
        };

        result.push(folded);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_homoglyphs_to_canonical() {
        // "jоhn" contains Cyrillic 'о' (U+043E) instead of Latin 'o'
        let cyrillic_john = "j\u{043E}hn";
        assert_eq!(normalize_unicode(cyrillic_john), "john");
    }

    #[test]
    fn strips_zero_width_chars() {
        // "j​o​h​n" contains Zero Width Spaces (U+200B), ZWNJ (U+200C), ZWJ (U+200D), Word Joiner (U+2060), and BOM (U+FEFF)
        let zw_input = "j\u{200B}o\u{200C}h\u{200D}n\u{2060}.\u{FEFF}smith@acme.com";
        assert_eq!(normalize_unicode(zw_input), "john.smith@acme.com");
    }
}
