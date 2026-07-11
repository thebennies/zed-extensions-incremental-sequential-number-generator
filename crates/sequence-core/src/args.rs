//! Shared "last argument is the count" argument convention, used by both
//! the Zed extension's slash command and the standalone CLI so they behave
//! identically and are covered by one shared test suite.

/// Splits `args` into `(spec, count)`.
///
/// Whitespace-splitting callers (Zed's slash-command dispatch, a shell
/// invoking the CLI) hand us a `Vec<String>` where a spec containing spaces
/// (e.g. a word list like `apple, banana, cherry`) has already been split
/// into multiple tokens. The convention this function implements: the
/// **last** argument is always the cursor count; everything before it is
/// rejoined with spaces to reconstruct the original spec.
///
/// Falls back to `count = 1` if the last argument isn't a valid number
/// (per the PRD's "gracefully fall back rather than abort" edge-case
/// guidance) - in that case the *entire* argument list is treated as the
/// spec. This function only errors on a completely empty argument list,
/// since there is nothing to generate at all.
pub fn parse_spec_and_count(args: &[String]) -> Result<(String, usize), String> {
    let (last, rest) = args
        .split_last()
        .ok_or_else(|| "expected at least one argument: <spec> <count>".to_string())?;

    match last.parse::<usize>() {
        Ok(count) if !rest.is_empty() => Ok((rest.join(" "), count.max(1))),
        // No explicit count was given (only a spec) - default to a single value.
        _ => Ok((args.join(" "), 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_spec_and_trailing_count() {
        let args = vec!["1:2:%02d".to_string(), "4".to_string()];
        assert_eq!(
            parse_spec_and_count(&args).unwrap(),
            ("1:2:%02d".to_string(), 4)
        );
    }

    #[test]
    fn rejoins_word_list_split_by_whitespace() {
        let args = vec![
            "apple,".to_string(),
            "banana,".to_string(),
            "cherry".to_string(),
            "5".to_string(),
        ];
        assert_eq!(
            parse_spec_and_count(&args).unwrap(),
            ("apple, banana, cherry".to_string(), 5)
        );
    }

    #[test]
    fn defaults_to_count_one_without_explicit_count() {
        let args = vec!["a:1".to_string()];
        assert_eq!(parse_spec_and_count(&args).unwrap(), ("a:1".to_string(), 1));
    }

    #[test]
    fn empty_args_is_an_error() {
        assert!(parse_spec_and_count(&[]).is_err());
    }
}
