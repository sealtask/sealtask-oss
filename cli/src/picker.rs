use crate::output::{CliError, CliResult};
use crate::table::{sanitize_cell, short_unique_ids};
use crate::terminal;
use console::{Key, Term};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use sealtask_client_core::PublicError;
use std::cmp::Ordering;
use std::fmt;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::io;
#[cfg(windows)]
use std::io::IsTerminal;
#[cfg(unix)]
use std::sync::atomic::{AtomicI32, Ordering as AtomicOrdering};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const PICKER_LABEL_MAX_INPUT_BYTES: usize = 4_096;
const PICKER_LABEL_MAX_OUTPUT_BYTES: usize = 512;
const PICKER_LABEL_MAX_WIDTH: usize = 80;
const PICKER_QUERY_MAX_BYTES: usize = 512;
const PICKER_MAX_ROWS: usize = 12;
#[cfg(unix)]
const PICKER_TERMINATION_SIGNALS: [libc::c_int; 4] =
    [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT];
#[cfg(unix)]
static PICKER_SIGNAL: AtomicI32 = AtomicI32::new(0);

/// An item that can be selected without exposing its decrypted label to logs.
pub(crate) struct PickerCandidate {
    id: Uuid,
    name: Option<String>,
}

struct PreparedCandidate {
    id: Uuid,
    label: Option<String>,
    selector: String,
    short_selector: String,
    search: String,
}

#[derive(Default)]
struct PickerState {
    query: String,
    selected: usize,
}

struct TerminalFrame<'a> {
    terminal: &'a Term,
    rendered_lines: usize,
    cursor_hidden: bool,
    restored: bool,
}

#[cfg(unix)]
struct PickerSignalGuard {
    previous_actions: Vec<(libc::c_int, libc::sigaction)>,
}

enum PickerOutcome {
    Selected(Uuid),
    Cancelled(&'static str),
    Failed(io::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyAction {
    Continue,
    Select,
    Cancel,
}

impl PickerCandidate {
    pub(crate) fn new(id: Uuid, name: Option<String>) -> Self {
        Self { id, name }
    }
}

impl fmt::Debug for PickerCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PickerCandidate")
            .field("id", &self.id)
            .field("name_present", &self.name.is_some())
            .finish()
    }
}

impl Drop for PickerCandidate {
    fn drop(&mut self) {
        self.name.zeroize();
    }
}

impl Drop for PreparedCandidate {
    fn drop(&mut self) {
        self.label.zeroize();
        self.selector.zeroize();
        self.short_selector.zeroize();
        self.search.zeroize();
    }
}

impl Drop for PickerState {
    fn drop(&mut self) {
        self.query.zeroize();
    }
}

impl<'a> TerminalFrame<'a> {
    fn start(terminal: &'a Term) -> io::Result<Self> {
        terminal.hide_cursor()?;
        Ok(Self {
            terminal,
            rendered_lines: 0,
            cursor_hidden: true,
            restored: false,
        })
    }

    fn render(&mut self, lines: &[String]) -> io::Result<()> {
        self.clear_rendered_lines()?;
        for line in lines {
            match self.terminal.write_line(line) {
                Ok(()) => self.rendered_lines += 1,
                Err(error) => {
                    let _ = self.terminal.clear_line();
                    return Err(error);
                }
            }
        }
        self.terminal.flush()
    }

    fn read_key(&self) -> io::Result<Key> {
        self.terminal.read_key_raw()
    }

    fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let mut first_error = self.clear_rendered_lines().err();
        if self.cursor_hidden {
            if let Err(error) = self.terminal.show_cursor()
                && first_error.is_none()
            {
                first_error = Some(error);
            }
            self.cursor_hidden = false;
        }
        if let Err(error) = self.terminal.flush()
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        self.restored = true;
        first_error.map_or(Ok(()), Err)
    }

    fn clear_rendered_lines(&mut self) -> io::Result<()> {
        let mut first_error = self.terminal.clear_line().err();
        if self.rendered_lines > 0
            && let Err(error) = self.terminal.clear_last_lines(self.rendered_lines)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        self.rendered_lines = 0;
        first_error.map_or(Ok(()), Err)
    }
}

impl Drop for TerminalFrame<'_> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(unix)]
impl PickerSignalGuard {
    fn install() -> io::Result<Self> {
        PICKER_SIGNAL.store(0, AtomicOrdering::SeqCst);

        // SAFETY: `sigaction` is valid when zeroed, and `sigemptyset`
        // initializes the mask before the action is installed.
        let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
        action.sa_sigaction = capture_picker_signal as *const () as libc::sighandler_t;
        action.sa_flags = 0;
        // SAFETY: `action.sa_mask` is a valid writable signal set.
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut guard = Self {
            previous_actions: Vec::with_capacity(PICKER_TERMINATION_SIGNALS.len()),
        };
        for signal in PICKER_TERMINATION_SIGNALS {
            // SAFETY: the pointers remain valid for this call, and the handler
            // only performs an atomic store.
            let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
            // SAFETY: `signal` is one of the supported termination signals.
            if unsafe { libc::sigaction(signal, &action, &mut previous) } != 0 {
                let error = io::Error::last_os_error();
                let _ = guard.restore_actions();
                return Err(error);
            }
            guard.previous_actions.push((signal, previous));
        }
        Ok(guard)
    }

    fn restore(mut self) -> io::Result<()> {
        self.restore_actions()
    }

    fn restore_actions(&mut self) -> io::Result<()> {
        let mut first_error = None;
        while let Some((signal, previous)) = self.previous_actions.pop() {
            // SAFETY: `previous` came from a successful `sigaction` call for
            // this exact signal.
            if unsafe { libc::sigaction(signal, &previous, std::ptr::null_mut()) } != 0
                && first_error.is_none()
            {
                first_error = Some(io::Error::last_os_error());
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(unix)]
impl Drop for PickerSignalGuard {
    fn drop(&mut self) {
        let _ = self.restore_actions();
    }
}

#[cfg(unix)]
extern "C" fn capture_picker_signal(signal: libc::c_int) {
    let _ =
        PICKER_SIGNAL.compare_exchange(0, signal, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst);
}

/// Return the full, unambiguous selector accepted by command resolvers.
pub(crate) fn selector_for(id: Uuid) -> String {
    format!("id:{}", id.simple())
}

/// Fail before any network request if an interactive picker cannot be opened.
pub(crate) fn ensure_picker_terminal() -> CliResult<()> {
    open_picker_terminal().map(drop)
}

/// Select a candidate entirely inside the user's controlling terminal.
///
/// Decrypted labels are written only to the attended picker terminal. They are
/// not passed to another process, stored in a temporary file, or included in
/// errors. On Unix the picker uses `/dev/tty` directly, so redirected standard
/// streams remain free of decrypted data.
pub(crate) fn pick_candidate(
    entity_kind: &str,
    candidates: Vec<PickerCandidate>,
) -> CliResult<Uuid> {
    terminal::clear_active_progress();
    if candidates.is_empty() {
        return Err(PublicError::validation(
            "no candidates are available for interactive selection; pass an explicit id: selector or broaden the picker scope",
        )
        .into());
    }

    let candidates = prepare_candidates(candidates);
    let terminal = open_picker_terminal()?;
    interact(&terminal, entity_kind, &candidates)
}

/// Display decrypted content only on the attended controlling terminal.
///
/// Unix writes directly to `/dev/tty`. Windows requires attended stdin and
/// stderr console handles, so redirected standard streams never receive the
/// displayed plaintext.
pub(crate) fn show_private_document(title: String, lines: Vec<String>) -> CliResult<()> {
    terminal::clear_active_progress();
    let terminal = open_picker_terminal()?;
    #[cfg(unix)]
    let signals = PickerSignalGuard::install().map_err(picker_signal_error)?;
    let mut title = Zeroizing::new(title);
    let mut lines = Zeroizing::new(lines);
    let sanitized_title = sanitize_cell(&title);
    title.zeroize();
    *title = sanitized_title;
    for line in lines.iter_mut() {
        let sanitized = sanitize_cell(line);
        line.zeroize();
        *line = sanitized;
    }

    let mut frame = TerminalFrame::start(&terminal).map_err(picker_io_error)?;
    let mut offset = 0_usize;
    let mut wrapped_width = None;
    let mut visual_lines = Zeroizing::new(Vec::new());
    let outcome = loop {
        #[cfg(unix)]
        if PICKER_SIGNAL.load(AtomicOrdering::SeqCst) != 0 {
            break Err(CliError::interrupted(
                "private browse view interrupted",
                &[],
            ));
        }
        let (terminal_rows, terminal_columns) = terminal.size();
        let width = usize::from(terminal_columns).max(1);
        let visible_rows = usize::from(terminal_rows).saturating_sub(2).max(1);
        if wrapped_width != Some(width) {
            let next_lines = wrap_private_lines(&lines, width);
            visual_lines.zeroize();
            *visual_lines = next_lines;
            wrapped_width = Some(width);
        }
        let maximum_offset = visual_lines.len().saturating_sub(visible_rows);
        offset = offset.min(maximum_offset);
        let mut rendered = Vec::with_capacity(visible_rows.saturating_add(2));
        rendered.push(truncate_width(&title, width));
        rendered.extend(visual_lines.iter().skip(offset).take(visible_rows).cloned());
        rendered.push(truncate_width(
            "↑/↓ scroll · PgUp/PgDn page · Home/End jump · q/Esc close",
            width,
        ));
        let render_result = frame.render(&rendered);
        rendered.zeroize();
        if let Err(error) = render_result {
            break Err(picker_io_error(error));
        }

        match frame.read_key() {
            Ok(Key::CtrlC | Key::Char('\u{3}')) => {
                break Err(CliError::interrupted(
                    "private browse view interrupted",
                    &[],
                ));
            }
            Ok(Key::Escape | Key::Enter | Key::Char('q') | Key::Char('Q')) => break Ok(()),
            Ok(Key::ArrowUp | Key::Char('\u{10}')) => {
                offset = offset.saturating_sub(1);
            }
            Ok(Key::ArrowDown | Key::Char('\u{e}')) => {
                offset = offset.saturating_add(1).min(maximum_offset);
            }
            Ok(Key::PageUp) => {
                offset = offset.saturating_sub(visible_rows);
            }
            Ok(Key::PageDown) => {
                offset = offset.saturating_add(visible_rows).min(maximum_offset);
            }
            Ok(Key::Home) => offset = 0,
            Ok(Key::End) => offset = maximum_offset,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                break Err(CliError::interrupted(
                    "private browse view interrupted",
                    &[],
                ));
            }
            Err(error) => break Err(picker_io_error(error)),
        }
    };

    let cleanup = frame.restore();
    let result = match (outcome, cleanup) {
        (result, Ok(())) => result,
        (Err(error), Err(_)) => Err(error),
        (Ok(()), Err(error)) => Err(PublicError::unexpected(format!(
            "private browse view could not restore the controlling terminal: {error}; reopen the terminal if its display is inconsistent"
        ))
        .into()),
    };
    #[cfg(unix)]
    let result = finish_picker_signal_supervision(signals, result);
    result
}

fn interact(
    terminal: &Term,
    entity_kind: &str,
    candidates: &[PreparedCandidate],
) -> CliResult<Uuid> {
    #[cfg(unix)]
    let signals = PickerSignalGuard::install().map_err(picker_signal_error)?;
    let mut frame = TerminalFrame::start(terminal).map_err(picker_io_error)?;
    let mut state = PickerState::default();
    let matcher = SkimMatcherV2::default();
    let outcome = loop {
        #[cfg(unix)]
        if PICKER_SIGNAL.load(AtomicOrdering::SeqCst) != 0 {
            break PickerOutcome::Cancelled("interactive selection interrupted");
        }
        let ranked = ranked_indices(candidates, &state.query, &matcher);
        normalize_selection(&mut state, ranked.len());
        let (terminal_rows, terminal_columns) = terminal.size();
        let option_rows = usize::from(terminal_rows)
            .saturating_sub(2)
            .clamp(1, PICKER_MAX_ROWS);
        let width = usize::from(terminal_columns).max(1);
        let mut lines = render_lines(entity_kind, candidates, &ranked, &state, option_rows, width);
        let render_result = frame.render(&lines);
        lines.zeroize();
        if let Err(error) = render_result {
            break PickerOutcome::Failed(error);
        }

        let key = match frame.read_key() {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                break PickerOutcome::Cancelled("interactive selection interrupted");
            }
            Err(error) => break PickerOutcome::Failed(error),
        };
        match apply_key(&mut state, &ranked, key) {
            KeyAction::Continue => {}
            KeyAction::Select => {
                let Some(index) = ranked.get(state.selected) else {
                    continue;
                };
                break PickerOutcome::Selected(candidates[*index].id);
            }
            KeyAction::Cancel => {
                break PickerOutcome::Cancelled(
                    "interactive selection cancelled before choosing an item",
                );
            }
        }
    };

    let cleanup = frame.restore();
    let result = match (outcome, cleanup) {
        (PickerOutcome::Selected(id), Ok(())) => Ok(id),
        (PickerOutcome::Cancelled(message), Ok(())) => {
            Err(CliError::interrupted(message, &[]))
        }
        (PickerOutcome::Failed(error), Ok(())) => Err(picker_io_error(error)),
        (PickerOutcome::Cancelled(message), Err(cleanup_error)) => {
            Err(CliError::interrupted(
                format!("{message}; terminal cleanup failed: {cleanup_error}"),
                &[],
            ))
        }
        (_, Err(cleanup_error)) => Err(PublicError::unexpected(format!(
            "interactive picker could not restore the terminal: {cleanup_error}; reopen the terminal if its display is inconsistent"
        ))
        .into()),
    };
    #[cfg(unix)]
    let result = finish_picker_signal_supervision(signals, result);
    result
}

fn prepare_candidates(candidates: Vec<PickerCandidate>) -> Vec<PreparedCandidate> {
    let mut prepared = candidates
        .into_iter()
        .map(|candidate| {
            let label = candidate.name.as_deref().and_then(prepare_label);
            let selector = selector_for(candidate.id);
            let mut search = label
                .as_deref()
                .map(search_text)
                .unwrap_or_else(String::new);
            if !search.is_empty() {
                search.push(' ');
            }
            search.push_str(&selector);
            PreparedCandidate {
                id: candidate.id,
                label,
                selector,
                short_selector: String::new(),
                search,
            }
        })
        .collect::<Vec<_>>();

    // Resolve duplicate IDs predictably before applying the user-facing sort.
    prepared.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| compare_labels(left.label.as_deref(), right.label.as_deref()))
    });
    prepared.dedup_by(|left, right| left.id == right.id);
    prepared.sort_by(|left, right| {
        compare_labels(left.label.as_deref(), right.label.as_deref())
            .then_with(|| left.id.cmp(&right.id))
    });

    let ids = prepared
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    for (candidate, prefix) in prepared.iter_mut().zip(short_unique_ids(&ids)) {
        candidate.short_selector = format!("id:{prefix}");
    }
    prepared
}

fn prepare_label(value: &str) -> Option<String> {
    let (prefix, input_truncated) = utf8_prefix(value, PICKER_LABEL_MAX_INPUT_BYTES);
    let mut sanitized = sanitize_cell(prefix);
    let mut normalized_source = sanitized.nfkc().collect::<String>();
    sanitized.zeroize();
    let mut normalized = sanitize_cell(&normalized_source);
    normalized_source.zeroize();
    if normalized.trim().is_empty() {
        normalized.zeroize();
        return None;
    }
    let result = bound_text(
        normalized.trim(),
        PICKER_LABEL_MAX_WIDTH,
        PICKER_LABEL_MAX_OUTPUT_BYTES,
        input_truncated,
    );
    normalized.zeroize();
    Some(result)
}

fn search_text(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

fn ranked_indices(
    candidates: &[PreparedCandidate],
    query: &str,
    matcher: &SkimMatcherV2,
) -> Vec<usize> {
    let mut normalized_query = search_text(query);
    if normalized_query.is_empty() {
        return (0..candidates.len()).collect();
    }
    let mut ranked = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            matcher
                .fuzzy_match(&candidate.search, &normalized_query)
                .map(|score| (index, score))
        })
        .collect::<Vec<_>>();
    normalized_query.zeroize();
    ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_index.cmp(right_index))
    });
    ranked.into_iter().map(|(index, _)| index).collect()
}

fn normalize_selection(state: &mut PickerState, result_count: usize) {
    if result_count == 0 {
        state.selected = 0;
    } else {
        state.selected = state.selected.min(result_count - 1);
    }
}

fn apply_key(state: &mut PickerState, ranked: &[usize], key: Key) -> KeyAction {
    match key {
        Key::CtrlC | Key::Char('\u{3}') | Key::Escape => KeyAction::Cancel,
        Key::Enter if !ranked.is_empty() => KeyAction::Select,
        Key::ArrowUp | Key::BackTab | Key::Char('\u{10}') => {
            state.selected = state.selected.saturating_sub(1);
            KeyAction::Continue
        }
        Key::ArrowDown | Key::Tab | Key::Char('\u{e}') => {
            if !ranked.is_empty() {
                state.selected = state.selected.saturating_add(1).min(ranked.len() - 1);
            }
            KeyAction::Continue
        }
        Key::Home => {
            state.selected = 0;
            KeyAction::Continue
        }
        Key::End if !ranked.is_empty() => {
            state.selected = ranked.len() - 1;
            KeyAction::Continue
        }
        Key::PageUp => {
            state.selected = state.selected.saturating_sub(PICKER_MAX_ROWS);
            KeyAction::Continue
        }
        Key::PageDown if !ranked.is_empty() => {
            state.selected = state
                .selected
                .saturating_add(PICKER_MAX_ROWS)
                .min(ranked.len() - 1);
            KeyAction::Continue
        }
        Key::Backspace | Key::Del => {
            state.query.pop();
            state.selected = 0;
            KeyAction::Continue
        }
        Key::Char('\u{15}') => {
            state.query.zeroize();
            state.selected = 0;
            KeyAction::Continue
        }
        Key::Char(character) if !character.is_control() => {
            if state.query.len().saturating_add(character.len_utf8()) <= PICKER_QUERY_MAX_BYTES {
                state.query.push(character);
                state.selected = 0;
            }
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

fn render_lines(
    entity_kind: &str,
    candidates: &[PreparedCandidate],
    ranked: &[usize],
    state: &PickerState,
    option_rows: usize,
    terminal_width: usize,
) -> Vec<String> {
    let width = terminal_width.max(1);
    let prompt = picker_prompt(entity_kind, &state.query, width);
    let mut lines = vec![prompt];
    if ranked.is_empty() {
        lines.push(truncate_width(
            "  No matches — keep typing, or press Esc to cancel",
            width,
        ));
        return lines;
    }

    let visible_rows = option_rows.max(1).min(ranked.len());
    let start = if state.selected >= visible_rows {
        state.selected + 1 - visible_rows
    } else {
        0
    };
    for (position, candidate_index) in ranked.iter().enumerate().skip(start).take(visible_rows) {
        lines.push(render_candidate_line(
            &candidates[*candidate_index],
            position == state.selected,
            width,
        ));
    }
    lines
}

fn picker_prompt(entity_kind: &str, query: &str, width: usize) -> String {
    let mut kind = prepare_label(entity_kind).unwrap_or_else(|| "item".to_string());
    let mut query = prepare_label(query).unwrap_or_default();
    let mut prompt = if query.is_empty() {
        format!("Pick {kind} | type to filter · ↑/↓ move · Enter select · Esc cancel")
    } else {
        format!("Pick {kind} | filter: {query}")
    };
    let result = truncate_width(&prompt, width);
    kind.zeroize();
    query.zeroize();
    prompt.zeroize();
    result
}

fn render_candidate_line(candidate: &PreparedCandidate, selected: bool, width: usize) -> String {
    let marker = if selected { "> " } else { "  " };
    let selector = &candidate.short_selector;
    let marker_width = UnicodeWidthStr::width(marker);
    let selector_width = UnicodeWidthStr::width(selector.as_str());
    let separator = "  ";
    let separator_width = UnicodeWidthStr::width(separator);
    if width <= marker_width.saturating_add(selector_width) {
        return truncate_owned(format!("{marker}{selector}"), width);
    }

    let label = candidate.label.as_deref().unwrap_or("<unnamed>");
    let label_width = width
        .saturating_sub(marker_width)
        .saturating_sub(separator_width)
        .saturating_sub(selector_width);
    let mut label = truncate_width(label, label_width);
    if label.is_empty() {
        truncate_owned(format!("{marker}{selector}"), width)
    } else {
        let mut line = format!("{marker}{label}{separator}{selector}");
        label.zeroize();
        let result = truncate_width(&line, width);
        line.zeroize();
        result
    }
}

fn utf8_prefix(value: &str, max_bytes: usize) -> (&str, bool) {
    if value.len() <= max_bytes {
        return (value, false);
    }
    let mut end = max_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (&value[..end], true)
}

fn bound_text(value: &str, max_width: usize, max_bytes: usize, truncated: bool) -> String {
    if !truncated && value.len() <= max_bytes && UnicodeWidthStr::width(value) <= max_width {
        return value.to_owned();
    }
    truncate_with_budget(value, max_width, max_bytes)
}

fn truncate_width(value: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(value) <= max_width {
        return value.to_owned();
    }
    truncate_with_budget(value, max_width, usize::MAX)
}

fn truncate_owned(mut value: String, max_width: usize) -> String {
    let result = truncate_width(&value, max_width);
    value.zeroize();
    result
}

fn wrap_private_lines(lines: &[String], max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut wrapped = Vec::new();
    for line in lines {
        if line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0_usize;
        for grapheme in UnicodeSegmentation::graphemes(line.as_str(), true) {
            let grapheme_width = UnicodeWidthStr::width(grapheme);
            if !current.is_empty() && current_width.saturating_add(grapheme_width) > max_width {
                wrapped.push(std::mem::take(&mut current));
                current_width = 0;
            }
            if current.is_empty() && grapheme_width > max_width {
                wrapped.push(truncate_width(grapheme, max_width));
                continue;
            }
            current.push_str(grapheme);
            current_width = current_width.saturating_add(grapheme_width);
        }
        if !current.is_empty() {
            wrapped.push(current);
        }
    }
    wrapped
}

fn truncate_with_budget(value: &str, max_width: usize, max_bytes: usize) -> String {
    if max_width == 0 || max_bytes == 0 {
        return String::new();
    }
    let marker = "…";
    if max_width < UnicodeWidthStr::width(marker) || max_bytes < marker.len() {
        return String::new();
    }
    let byte_budget = max_bytes.saturating_sub(marker.len());
    let width_budget = max_width.saturating_sub(UnicodeWidthStr::width(marker));
    let mut bounded = String::new();
    let mut width: usize = 0;
    for grapheme in UnicodeSegmentation::graphemes(value, true) {
        let next_width = UnicodeWidthStr::width(grapheme);
        if bounded.len().saturating_add(grapheme.len()) > byte_budget
            || width.saturating_add(next_width) > width_budget
        {
            break;
        }
        bounded.push_str(grapheme);
        width = width.saturating_add(next_width);
    }
    bounded.push_str(marker);
    bounded
}

fn compare_labels(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            let mut left_search = search_text(left);
            let mut right_search = search_text(right);
            let ordering = left_search.cmp(&right_search).then_with(|| left.cmp(right));
            left_search.zeroize();
            right_search.zeroize();
            ordering
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(unix)]
fn finish_picker_signal_supervision<T>(
    signals: PickerSignalGuard,
    result: CliResult<T>,
) -> CliResult<T> {
    let restore_result = signals.restore();
    let signal = PICKER_SIGNAL.load(AtomicOrdering::SeqCst);
    if signal != 0 {
        let mut message = format!(
            "private terminal interaction interrupted by {}; terminal cleanup was attempted",
            picker_signal_name(signal)
        );
        if let Err(error) = restore_result {
            message.push_str(&format!(
                "; the previous signal handlers could not be fully restored: {error}"
            ));
        }
        if let Err(error) = &result
            && error.to_string().contains("restore")
        {
            message.push_str(&format!("; {error}"));
        }
        return Err(CliError::interrupted(message, &[]));
    }
    restore_result.map_err(picker_signal_error)?;
    result
}

#[cfg(unix)]
fn picker_signal_name(signal: libc::c_int) -> &'static str {
    match signal {
        libc::SIGINT => "SIGINT",
        libc::SIGTERM => "SIGTERM",
        libc::SIGHUP => "SIGHUP",
        libc::SIGQUIT => "SIGQUIT",
        _ => "a termination signal",
    }
}

#[cfg(unix)]
fn picker_signal_error(error: io::Error) -> CliError {
    PublicError::unexpected(format!(
        "failed to supervise private terminal interruption safely: {error}"
    ))
    .into()
}

fn picker_io_error(error: io::Error) -> CliError {
    PublicError::unexpected(format!(
        "interactive picker failed: {error}; retry or pass an explicit id: selector"
    ))
    .into()
}

#[cfg(unix)]
fn open_picker_terminal() -> CliResult<Term> {
    ensure_picker_environment(std::env::var("TERM").ok().as_deref())?;
    let read = open_controlling_terminal()?;
    let write = open_controlling_terminal()?;
    ensure_attended_terminal(Term::read_write_pair(read, write))
}

#[cfg(unix)]
fn open_controlling_terminal() -> CliResult<std::fs::File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|error| picker_terminal_error(Some(&error)))
}

#[cfg(windows)]
fn open_picker_terminal() -> CliResult<Term> {
    ensure_picker_environment(std::env::var("TERM").ok().as_deref())?;
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(picker_terminal_error(None));
    }
    ensure_attended_terminal(Term::stderr())
}

#[cfg(not(any(unix, windows)))]
fn open_picker_terminal() -> CliResult<Term> {
    Err(PublicError::validation(
        "interactive selection is not supported on this platform; pass an explicit id: selector instead",
    )
    .into())
}

fn ensure_picker_environment(term: Option<&str>) -> CliResult<()> {
    if term.is_some_and(|term| term.eq_ignore_ascii_case("dumb")) {
        Err(PublicError::validation(
            "interactive selection is unavailable when TERM=dumb; pass an explicit id: selector instead",
        )
        .into())
    } else {
        Ok(())
    }
}

fn ensure_attended_terminal(terminal: Term) -> CliResult<Term> {
    if terminal.is_term() {
        Ok(terminal)
    } else {
        Err(picker_terminal_error(None))
    }
}

fn picker_terminal_error(error: Option<&io::Error>) -> CliError {
    let detail = error.map_or_else(String::new, |error| format!(" ({error})"));
    PublicError::validation(format!(
        "interactive selection requires a controlling terminal{detail}; pass an explicit id: selector instead"
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Uuid {
        Uuid::parse_str(value).unwrap()
    }

    fn candidates() -> Vec<PreparedCandidate> {
        prepare_candidates(vec![
            PickerCandidate::new(
                id("0198f128-2516-7a2a-bd8f-a98a3d8f1151"),
                Some("Release notes".to_string()),
            ),
            PickerCandidate::new(
                id("0298f128-2516-7a2a-bd8f-a98a3d8f1152"),
                Some("Operations".to_string()),
            ),
            PickerCandidate::new(id("0398f128-2516-7a2a-bd8f-a98a3d8f1153"), None),
        ])
    }

    #[test]
    fn selector_is_full_lowercase_compact_uuid() {
        let id = id("0198F128-2516-7A2A-BD8F-A98A3D8F1150");
        assert_eq!(selector_for(id), "id:0198f12825167a2abd8fa98a3d8f1150");
    }

    #[test]
    fn debug_redacts_decrypted_name() {
        let candidate = PickerCandidate::new(
            id("0198f128-2516-7a2a-bd8f-a98a3d8f1150"),
            Some("confidential launch".to_string()),
        );
        let debug = format!("{candidate:?}");
        assert!(debug.contains("name_present: true"));
        assert!(!debug.contains("confidential"));
        assert!(!debug.contains("launch"));
    }

    #[test]
    fn malicious_labels_are_sanitized_and_nfkc_normalized() {
        let candidate_id = id("0198f128-2516-7a2a-bd8f-a98a3d8f1150");
        let candidates = prepare_candidates(vec![PickerCandidate::new(
            candidate_id,
            Some("Ｆｕｌｌ\n\u{202e}evil\u{1b}[31m\0\t\u{212b}".to_string()),
        )]);
        assert_eq!(candidates[0].label.as_deref(), Some("Full evil[31m Å"));
        for forbidden in ['\n', '\u{202e}', '\u{1b}', '\0'] {
            assert!(!candidates[0].label.as_deref().unwrap().contains(forbidden));
        }
    }

    #[test]
    fn huge_labels_are_bounded_by_bytes_and_width() {
        let label = prepare_label(&"界".repeat(10_000)).unwrap();
        assert!(label.len() <= PICKER_LABEL_MAX_OUTPUT_BYTES);
        assert!(UnicodeWidthStr::width(label.as_str()) <= PICKER_LABEL_MAX_WIDTH);
        assert!(label.ends_with('…'));
    }

    #[test]
    fn candidates_sort_deterministically_and_deduplicate_ids() {
        let duplicate = id("0198f128-2516-7a2a-bd8f-a98a3d8f1150");
        let beta = id("0298f128-2516-7a2a-bd8f-a98a3d8f1151");
        let unnamed = id("0398f128-2516-7a2a-bd8f-a98a3d8f1152");
        let prepared = prepare_candidates(vec![
            PickerCandidate::new(unnamed, None),
            PickerCandidate::new(beta, Some("Beta".to_string())),
            PickerCandidate::new(duplicate, Some("Zulu".to_string())),
            PickerCandidate::new(duplicate, Some("Alpha".to_string())),
        ]);
        assert_eq!(prepared.len(), 3);
        assert_eq!(prepared[0].id, duplicate);
        assert_eq!(prepared[0].label.as_deref(), Some("Alpha"));
        assert_eq!(prepared[1].id, beta);
        assert_eq!(prepared[2].id, unnamed);
    }

    #[test]
    fn fuzzy_ranking_is_normalized_and_stable_for_ties() {
        let candidates = candidates();
        let matcher = SkimMatcherV2::default();
        let release = ranked_indices(&candidates, "ＲＥＬＥＡＳＥ", &matcher);
        assert_eq!(
            candidates[release[0]].label.as_deref(),
            Some("Release notes")
        );

        let empty = ranked_indices(&candidates, "", &matcher);
        assert_eq!(empty, vec![0, 1, 2]);
    }

    #[test]
    fn full_selector_is_searchable() {
        let candidates = candidates();
        let matcher = SkimMatcherV2::default();
        let ranked = ranked_indices(&candidates, "0298f1282516", &matcher);
        assert_eq!(
            candidates[ranked[0]].id,
            id("0298f128-2516-7a2a-bd8f-a98a3d8f1152")
        );
    }

    #[test]
    fn ctrl_c_and_escape_cancel_without_mutating_query() {
        for key in [Key::CtrlC, Key::Char('\u{3}'), Key::Escape] {
            let mut state = PickerState {
                query: "secret".to_string(),
                selected: 0,
            };
            assert_eq!(apply_key(&mut state, &[0], key), KeyAction::Cancel);
            assert_eq!(state.query, "secret");
        }
    }

    #[test]
    fn navigation_typing_backspace_and_clear_are_deterministic() {
        let mut state = PickerState::default();
        assert_eq!(
            apply_key(&mut state, &[0, 1, 2], Key::ArrowDown),
            KeyAction::Continue
        );
        assert_eq!(state.selected, 1);
        apply_key(&mut state, &[0, 1, 2], Key::Char('é'));
        assert_eq!(state.query, "é");
        assert_eq!(state.selected, 0);
        apply_key(&mut state, &[0], Key::Backspace);
        assert!(state.query.is_empty());
        state.query.push_str("sensitive");
        apply_key(&mut state, &[0], Key::Char('\u{15}'));
        assert!(state.query.is_empty());
    }

    #[test]
    fn enter_selects_only_when_a_match_exists() {
        let mut state = PickerState::default();
        assert_eq!(apply_key(&mut state, &[], Key::Enter), KeyAction::Continue);
        assert_eq!(apply_key(&mut state, &[0], Key::Enter), KeyAction::Select);
    }

    #[test]
    fn rendered_lines_never_wrap_at_narrow_or_common_widths() {
        let candidates = candidates();
        let state = PickerState::default();
        for width in [1, 8, 20, 40, 80, 100] {
            let ranked = (0..candidates.len()).collect::<Vec<_>>();
            let lines = render_lines("task", &candidates, &ranked, &state, 12, width);
            assert!(lines.len() <= PICKER_MAX_ROWS + 1);
            for line in lines {
                assert!(
                    UnicodeWidthStr::width(line.as_str()) <= width,
                    "{line:?} exceeds width {width}"
                );
                assert!(!line.contains('\n'));
                assert!(!line.contains('\r'));
            }
        }
    }

    #[test]
    fn private_documents_wrap_long_lines_without_discarding_text() {
        let source = vec![
            "0123456789abcdefghijklmnopqrstuvwxyz".to_string(),
            String::new(),
            "界界界".to_string(),
        ];
        let wrapped = wrap_private_lines(&source, 10);

        assert!(wrapped.iter().all(|line| {
            UnicodeWidthStr::width(line.as_str()) <= 10
                && !line.contains('\n')
                && !line.contains('\r')
        }));
        assert_eq!(wrapped[..4].concat(), source[0]);
        assert!(wrapped.iter().any(String::is_empty));
        assert_eq!(wrapped.last(), Some(&source[2]));
    }

    #[test]
    fn viewport_contains_selected_row_without_rendering_every_candidate() {
        let candidates = (0..100)
            .map(|offset| {
                PickerCandidate::new(
                    Uuid::from_u128(1_000 + offset),
                    Some(format!("Task {offset:03}")),
                )
            })
            .collect::<Vec<_>>();
        let candidates = prepare_candidates(candidates);
        let ranked = (0..candidates.len()).collect::<Vec<_>>();
        let state = PickerState {
            query: String::new(),
            selected: 50,
        };
        let lines = render_lines("task", &candidates, &ranked, &state, 5, 60);
        assert_eq!(lines.len(), 6);
        assert_eq!(
            lines.iter().filter(|line| line.starts_with("> ")).count(),
            1
        );
        assert!(lines.iter().any(|line| line.contains("Task 050")));
    }

    #[test]
    fn no_match_frame_is_bounded_and_contains_no_candidate() {
        let candidates = candidates();
        let state = PickerState {
            query: "private query".to_string(),
            selected: 0,
        };
        let lines = render_lines("task", &candidates, &[], &state, 4, 32);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("No matches"));
        assert!(!lines.iter().any(|line| line.contains("Operations")));
    }

    #[test]
    fn duplicate_short_prefixes_expand_for_visual_disambiguation() {
        let candidates = prepare_candidates(vec![
            PickerCandidate::new(
                id("0198f128-1111-7a2a-bd8f-a98a3d8f1150"),
                Some("One".to_string()),
            ),
            PickerCandidate::new(
                id("0198f128-2222-7a2a-bd8f-a98a3d8f1151"),
                Some("Two".to_string()),
            ),
        ]);
        assert_ne!(candidates[0].short_selector, candidates[1].short_selector);
        assert!(candidates[0].short_selector.len() > "id:0198f128".len());
    }

    #[test]
    fn dumb_terminal_is_rejected_with_explicit_selector_hint() {
        let error = ensure_picker_environment(Some("dumb")).unwrap_err();
        assert_eq!(error.code(), "validation");
        assert!(error.to_string().contains("explicit id: selector"));
    }

    #[test]
    fn utf8_prefix_never_splits_a_character() {
        let value = "é".repeat(PICKER_LABEL_MAX_INPUT_BYTES);
        let (prefix, truncated) = utf8_prefix(&value, PICKER_LABEL_MAX_INPUT_BYTES - 1);
        assert!(truncated);
        assert!(prefix.is_char_boundary(prefix.len()));
        assert!(prefix.len() < PICKER_LABEL_MAX_INPUT_BYTES);
    }
}
