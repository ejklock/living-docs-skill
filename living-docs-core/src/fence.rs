//! The one reader of fenced code blocks, as CommonMark defines them: every
//! reader of a body that must skip fenced lines asks it.

/// The open fence a body is inside: its character and the length of the run
/// that opened it; `None` outside any fence.
pub(crate) type Fence = Option<(char, usize)>;

/// Updates `fence` for a fence line and reports whether `text` was one. A fence
/// is a run of at least three backticks or tildes after up to three spaces. It
/// closes on a run of the same character at least as long, with nothing after
/// it. A backtick fence's info string holds no backtick, so such a line opens
/// nothing.
pub(crate) fn toggle_fence(text: &str, fence: &mut Fence) -> bool {
    let indent = text.len() - text.trim_start_matches(' ').len();
    let rest = &text[indent..];
    let Some(mark) = rest.chars().next().filter(|c| matches!(c, '`' | '~')) else {
        return false;
    };
    let run = rest.chars().take_while(|c| *c == mark).count();
    if indent > 3 || run < 3 {
        return false;
    }
    match *fence {
        None if mark == '`' && rest[run..].contains('`') => return false,
        None => *fence = Some((mark, run)),
        Some((open, width)) if open == mark && run >= width && rest[run..].trim().is_empty() => {
            *fence = None;
        }
        Some(_) => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(text: &str) -> Fence {
        let mut fence = None;
        toggle_fence(text, &mut fence);
        fence
    }

    #[test]
    fn a_backtick_or_tilde_run_of_three_or_four_opens_a_fence() {
        assert_eq!(opened("```"), Some(('`', 3)));
        assert_eq!(opened("````rust"), Some(('`', 4)));
        assert_eq!(opened("~~~"), Some(('~', 3)));
        assert_eq!(opened("~~~~ text"), Some(('~', 4)));
    }

    #[test]
    fn a_run_of_two_opens_nothing() {
        let mut fence = None;
        assert!(!toggle_fence("``", &mut fence));
        assert_eq!(fence, None);
    }

    #[test]
    fn a_shorter_closing_run_does_not_close_and_a_longer_one_does() {
        let mut fence = Some(('`', 4));
        assert!(!toggle_fence("```", &mut fence));
        assert_eq!(fence, Some(('`', 4)));
        assert!(toggle_fence("`````", &mut fence));
        assert_eq!(fence, None);
    }

    #[test]
    fn a_closing_run_of_the_same_length_closes() {
        let mut fence = Some(('~', 3));
        assert!(toggle_fence("~~~\n", &mut fence));
        assert_eq!(fence, None);
    }

    #[test]
    fn a_closing_run_of_the_other_character_does_not_close() {
        let mut fence = Some(('`', 3));
        assert!(!toggle_fence("~~~", &mut fence));
        assert_eq!(fence, Some(('`', 3)));
    }

    #[test]
    fn a_closing_run_with_an_info_string_does_not_close() {
        let mut fence = Some(('`', 3));
        assert!(!toggle_fence("``` rust", &mut fence));
        assert_eq!(fence, Some(('`', 3)));
    }

    #[test]
    fn up_to_three_spaces_of_indent_open_and_four_do_not() {
        assert_eq!(opened("   ```"), Some(('`', 3)));
        assert_eq!(opened("    ```"), None);
        assert_eq!(opened("\t```"), None);
    }

    #[test]
    fn a_backtick_in_a_backtick_info_string_opens_nothing() {
        assert_eq!(opened("``` a`b"), None);
        assert!(!toggle_fence("``` a`b", &mut None));
        assert_eq!(opened("~~~ a`b"), Some(('~', 3)));
    }
}
