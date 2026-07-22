pub fn join_spans(spans: &[&str]) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for (i, span) in spans.iter().enumerate() {
        if i > 0 {
            let prev = spans[i - 1];
            let prev_ends_w =
                prev.ends_with(|c: char| c.is_whitespace() || c == '.' || c == ',' || c == ';');
            let curr_starts_w =
                span.starts_with(|c: char| c.is_whitespace() || c == '.' || c == ',' || c == ';');

            let prev_word = prev.split_whitespace().last().unwrap_or("");
            let curr_word = span.split_whitespace().next().unwrap_or("");

            let both_are_complete_words = prev_word.len() > 3
                && curr_word.len() > 3
                && !prev_word.contains('@')
                && !curr_word.contains('@');

            if !prev_ends_w && !curr_starts_w && both_are_complete_words {
                result.push(' ');
            }
        }
        result.push_str(span);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_split_string() {
        // Given split spans: ["jo", "hn.sm", "ith@ac", "me.com"]
        let split_spans = vec!["jo", "hn.sm", "ith@ac", "me.com"];
        let reassembled = join_spans(&split_spans);

        assert_eq!(reassembled, "john.smith@acme.com");
    }

    #[test]
    fn reassembly_does_not_create_false_emails() {
        // Given distinct word tokens with trailing space: ["John ", "Smith ", "at ", "Acme"]
        let distinct_words = vec!["John ", "Smith ", "at ", "Acme"];
        let result = join_spans(&distinct_words);

        assert_eq!(result, "John Smith at Acme");
        assert!(!result.contains('@'));
    }
}
