use super::*;

#[test]
fn output_mode_json_flag_wins_regardless_of_tty() {
    assert_eq!(
        OutputMode::resolve(true, false, false, true),
        OutputMode::Json
    );
    assert_eq!(
        OutputMode::resolve(true, false, false, false),
        OutputMode::Json
    );
}

#[test]
fn output_mode_plain_flag_wins_regardless_of_tty() {
    assert_eq!(
        OutputMode::resolve(false, true, false, true),
        OutputMode::Text
    );
    assert_eq!(
        OutputMode::resolve(false, true, false, false),
        OutputMode::Text
    );
}

#[test]
fn output_mode_defaults_to_json_off_a_tty() {
    assert_eq!(
        OutputMode::resolve(false, false, false, false),
        OutputMode::Json
    );
}

#[test]
fn output_mode_defaults_to_text_on_a_tty() {
    assert_eq!(
        OutputMode::resolve(false, false, false, true),
        OutputMode::Text
    );
}

#[test]
fn output_mode_explicit_color_always_implies_text_off_a_tty() {
    assert_eq!(
        OutputMode::resolve(false, false, true, false),
        OutputMode::Text
    );
}

#[test]
fn output_mode_json_flag_wins_over_explicit_color_always() {
    assert_eq!(
        OutputMode::resolve(true, false, true, false),
        OutputMode::Json
    );
}

#[test]
fn color_mode_always_wins_off_a_tty() {
    assert_eq!(
        ColorMode::resolve(ColorChoice::Always, false, false),
        ColorMode::On
    );
}

#[test]
fn color_mode_never_wins_on_a_tty() {
    assert_eq!(
        ColorMode::resolve(ColorChoice::Never, false, true),
        ColorMode::Off
    );
}

#[test]
fn color_mode_auto_is_off_when_no_color_env_is_set_even_on_a_tty() {
    assert_eq!(
        ColorMode::resolve(ColorChoice::Auto, true, true),
        ColorMode::Off
    );
}

#[test]
fn color_mode_auto_is_off_when_stdout_is_not_a_tty() {
    assert_eq!(
        ColorMode::resolve(ColorChoice::Auto, false, false),
        ColorMode::Off
    );
}

#[test]
fn color_mode_auto_is_on_for_a_plain_tty() {
    assert_eq!(
        ColorMode::resolve(ColorChoice::Auto, false, true),
        ColorMode::On
    );
}

#[test]
fn paint_wraps_text_in_the_styles_ansi_escape_when_on() {
    let painted = paint(ColorMode::On, Style::Red, "boom");
    assert_eq!(painted, "\x1b[31mboom\x1b[0m");
}

#[test]
fn paint_returns_the_text_unchanged_when_off() {
    assert_eq!(paint(ColorMode::Off, Style::Green, "ok"), "ok");
}

fn exit_of(result: io::Result<()>) -> Option<(i32, Option<String>)> {
    exit_for(&result).map(|exit| (exit.code, exit.message))
}

struct FailingFlush(io::ErrorKind);

impl Write for FailingFlush {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(self.0))
    }
}

#[test]
fn a_successful_write_continues_without_a_message() {
    assert_eq!(exit_of(Ok(())), None);
}

#[test]
fn a_closed_reader_exits_zero_without_a_message() {
    assert_eq!(
        exit_of(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        Some((0, None))
    );
}

#[test]
fn a_permission_error_exits_one_naming_the_cause() {
    assert_eq!(
        exit_of(Err(io::Error::from(io::ErrorKind::PermissionDenied))),
        Some((
            1,
            Some("failed writing to stdout: permission denied".to_owned())
        ))
    );
}

#[test]
fn an_arbitrary_write_error_exits_one_with_its_text() {
    let (code, message) = exit_of(Err(io::Error::other("disk full"))).expect("an exit");
    assert_eq!(code, 1);
    assert!(message.expect("a message").contains("disk full"));
}

#[test]
fn emit_leaves_the_exact_bytes_in_the_writer() {
    let mut sink = Vec::new();
    emit(&mut sink, |w| w.write_all(b"no newline")).expect("write succeeds");
    assert_eq!(sink, b"no newline");
}

#[test]
fn emit_formats_arguments_into_the_writer() {
    let mut sink = Vec::new();
    emit(&mut sink, |w| w.write_fmt(format_args!("{}-{}", 1, 2))).expect("write succeeds");
    assert_eq!(sink, b"1-2");
}

#[test]
fn a_failing_flush_is_returned_and_exits_one() {
    let mut writer = FailingFlush(io::ErrorKind::PermissionDenied);
    let result = emit(&mut writer, |w| w.write_all(b"x"));
    assert_eq!(
        result.as_ref().map_err(io::Error::kind),
        Err(io::ErrorKind::PermissionDenied)
    );
    assert_eq!(exit_for(&result).map(|exit| exit.code), Some(1));
}

#[test]
fn a_flush_into_a_closed_reader_exits_zero() {
    let mut writer = FailingFlush(io::ErrorKind::BrokenPipe);
    let result = emit(&mut writer, |w| w.write_all(b"x"));
    assert_eq!(exit_for(&result).map(|exit| exit.code), Some(0));
}
