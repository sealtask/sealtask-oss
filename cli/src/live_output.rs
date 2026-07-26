use crate::output::{CliResult, write_stdout_flushed};
use crate::terminal;

const MOVE_UP_AND_CLEAR_LINE: &str = "\x1b[1A\r\x1b[2K";

/// A bounded stdout region that can be redrawn without clearing unrelated
/// terminal history. Redirected streams must use append-only output instead.
#[derive(Default)]
pub(crate) struct LiveRegion {
    rendered_lines: usize,
}

impl LiveRegion {
    pub(crate) fn render(&mut self, content: &str) -> CliResult<()> {
        let mut frame = String::with_capacity(
            content.len().saturating_add(
                self.rendered_lines
                    .saturating_mul(MOVE_UP_AND_CLEAR_LINE.len()),
            ),
        );
        for _ in 0..self.rendered_lines {
            frame.push_str(MOVE_UP_AND_CLEAR_LINE);
        }
        frame.push_str(content);
        if !content.ends_with('\n') {
            frame.push('\n');
        }
        write_stdout_flushed(format_args!("{frame}"))?;
        self.rendered_lines = terminal::stdout_rendered_terminal_rows(content).max(1);
        Ok(())
    }

    /// Finish without erasing the final frame. Every rendered frame is newline
    /// terminated, so preserving it leaves the terminal at a clean line and
    /// keeps the operator's final state in scrollback.
    pub(crate) fn finish(&mut self) {
        self.rendered_lines = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_accounting_is_bounded_to_the_latest_frame() {
        let mut region = LiveRegion::default();
        assert_eq!(region.rendered_lines, 0);
        region.rendered_lines = terminal::stdout_rendered_terminal_rows("one\ntwo\n");
        assert_eq!(region.rendered_lines, 2);
    }
}
