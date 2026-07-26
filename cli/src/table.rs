use std::cmp::Reverse;
use std::io::{self, IsTerminal};

use terminal_size::{Width, terminal_size as read_terminal_size};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

const COLUMN_SEPARATOR: &str = "  ";
const COLUMN_SEPARATOR_WIDTH: usize = 2;
const DEFAULT_TERMINAL_WIDTH: usize = 100;
const MIN_UUID_PREFIX_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Alignment {
    #[default]
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Retention {
    Required,
    Optional(u8),
}

/// A declarative table column.
///
/// Required columns remain visible at ordinary terminal widths. Optional
/// columns are removed from the lowest retention priority upward when their
/// declared minimum widths do not fit. Higher optional priorities therefore
/// remain visible longer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Column {
    header: String,
    min_width: usize,
    max_width: usize,
    grow_weight: u8,
    alignment: Alignment,
    retention: Retention,
    preserve: bool,
}

impl Column {
    pub(crate) fn required(header: impl Into<String>, min_width: usize, max_width: usize) -> Self {
        Self::new(header, min_width, max_width, Retention::Required)
    }

    pub(crate) fn optional(
        header: impl Into<String>,
        min_width: usize,
        max_width: usize,
        retention_priority: u8,
    ) -> Self {
        Self::new(
            header,
            min_width,
            max_width,
            Retention::Optional(retention_priority),
        )
    }

    #[must_use]
    pub(crate) const fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub(crate) const fn flex(mut self, grow_weight: u8) -> Self {
        self.grow_weight = if grow_weight == 0 { 1 } else { grow_weight };
        self
    }

    /// Keep the complete sanitized value visible, even on a narrower terminal.
    ///
    /// This is intended for bounded, actionable values such as selectors. A
    /// preserved column may make a rendered line wider than the requested
    /// terminal width because an intact selector is safer than a truncated one.
    #[must_use]
    pub(crate) const fn preserve(mut self) -> Self {
        self.preserve = true;
        self
    }

    fn new(
        header: impl Into<String>,
        min_width: usize,
        max_width: usize,
        retention: Retention,
    ) -> Self {
        let min_width = min_width.max(1);
        Self {
            header: sanitize_cell(&header.into()),
            min_width,
            max_width: max_width.max(min_width),
            grow_weight: 1,
            alignment: Alignment::Left,
            retention,
            preserve: false,
        }
    }
}

/// A compact, borderless table that adapts to the available terminal width.
///
/// `render` detects the terminal width and falls back to 100 columns.
/// `render_with_width` is deterministic and is preferred for tests, redirected
/// output, and future `--columns` support. Rendered output is newline-terminated.
#[derive(Clone, Debug, Default)]
pub(crate) struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn new(columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: Vec::new(),
        }
    }

    pub(crate) fn push_row<I, S>(&mut self, cells: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut cells = cells
            .into_iter()
            .map(Into::into)
            .map(|cell| sanitize_cell(&cell))
            .collect::<Vec<_>>();
        debug_assert_eq!(
            cells.len(),
            self.columns.len(),
            "table rows should contain one cell per column"
        );
        cells.resize(self.columns.len(), String::new());
        cells.truncate(self.columns.len());
        self.rows.push(cells);
    }

    pub(crate) fn render(&self) -> String {
        self.render_with_width(terminal_width())
    }

    pub(crate) fn render_with_width(&self, width: usize) -> String {
        let layout = self.layout(width);
        if layout.indices.is_empty() {
            return String::new();
        }

        let mut rendered = String::new();
        let headers = self
            .columns
            .iter()
            .map(|column| column.header.as_str())
            .collect::<Vec<_>>();
        rendered.push_str(&render_line(&headers, &self.columns, &layout));
        rendered.push('\n');
        rendered.push_str(&"-".repeat(layout.rendered_width()));
        rendered.push('\n');

        for row in &self.rows {
            let cells = row.iter().map(String::as_str).collect::<Vec<_>>();
            rendered.push_str(&render_line(&cells, &self.columns, &layout));
            rendered.push('\n');
        }

        rendered
    }

    fn layout(&self, requested_width: usize) -> Layout {
        if self.columns.is_empty() {
            return Layout::default();
        }

        let requested_width = requested_width.max(1);
        let mut indices = (0..self.columns.len()).collect::<Vec<_>>();
        let desired_widths = (0..self.columns.len())
            .map(|index| self.desired_width(index))
            .collect::<Vec<_>>();

        while minimum_footprint(&indices, &self.columns, &desired_widths) > requested_width {
            let optional = indices
                .iter()
                .copied()
                .filter_map(|index| match self.columns[index].retention {
                    Retention::Required => None,
                    Retention::Optional(priority) => Some((priority, Reverse(index), index)),
                })
                .min()
                .map(|(_, _, index)| index);
            let Some(index) = optional else {
                break;
            };
            if indices.len() == 1 {
                break;
            }
            indices.retain(|candidate| *candidate != index);
        }

        // A terminal can be narrower than even one display cell per required
        // column. In that physically impossible case, retain preserved values
        // intact and remove non-preserved columns from the right. If only
        // preserved columns remain, overflowing is safer than corrupting an
        // actionable selector.
        while indices.len() > 1
            && narrowest_footprint(&indices, &self.columns, &desired_widths) > requested_width
        {
            let Some(layout_index) = indices
                .iter()
                .rposition(|index| !self.columns[*index].preserve)
            else {
                break;
            };
            indices.remove(layout_index);
        }

        let separator_width = separator_footprint(indices.len());
        let cell_budget = requested_width.saturating_sub(separator_width).max(1);
        let mut widths = indices
            .iter()
            .map(|index| minimum_column_width(&self.columns[*index], desired_widths[*index]))
            .collect::<Vec<_>>();
        let minimum_widths = indices
            .iter()
            .map(|index| {
                if self.columns[*index].preserve {
                    desired_widths[*index]
                } else {
                    1
                }
            })
            .collect::<Vec<_>>();
        shrink_to_budget(&mut widths, &minimum_widths, cell_budget);

        let visible_desired_widths = indices
            .iter()
            .map(|index| desired_widths[*index])
            .collect::<Vec<_>>();
        grow_to_budget(
            &indices,
            &self.columns,
            &visible_desired_widths,
            &mut widths,
            cell_budget,
        );

        Layout { indices, widths }
    }

    fn desired_width(&self, index: usize) -> usize {
        let column = &self.columns[index];
        let content_width = self
            .rows
            .iter()
            .filter_map(|row| row.get(index))
            .map(|cell| display_width(cell))
            .chain(std::iter::once(display_width(&column.header)))
            .max()
            .unwrap_or(column.min_width);
        if column.preserve {
            content_width.max(column.min_width)
        } else {
            content_width.clamp(column.min_width, column.max_width)
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Layout {
    indices: Vec<usize>,
    widths: Vec<usize>,
}

impl Layout {
    fn rendered_width(&self) -> usize {
        self.widths
            .iter()
            .sum::<usize>()
            .saturating_add(separator_footprint(self.widths.len()))
    }
}

fn minimum_column_width(column: &Column, desired_width: usize) -> usize {
    if column.preserve {
        desired_width
    } else {
        column.min_width
    }
}

fn minimum_footprint(indices: &[usize], columns: &[Column], desired_widths: &[usize]) -> usize {
    indices
        .iter()
        .map(|index| minimum_column_width(&columns[*index], desired_widths[*index]))
        .sum::<usize>()
        .saturating_add(separator_footprint(indices.len()))
}

fn narrowest_footprint(indices: &[usize], columns: &[Column], desired_widths: &[usize]) -> usize {
    indices
        .iter()
        .map(|index| {
            if columns[*index].preserve {
                desired_widths[*index]
            } else {
                1
            }
        })
        .sum::<usize>()
        .saturating_add(separator_footprint(indices.len()))
}

fn separator_footprint(column_count: usize) -> usize {
    column_count
        .saturating_sub(1)
        .saturating_mul(COLUMN_SEPARATOR_WIDTH)
}

fn shrink_to_budget(widths: &mut [usize], minimum_widths: &[usize], budget: usize) {
    while widths.iter().sum::<usize>() > budget {
        let Some((index, _)) = widths
            .iter()
            .enumerate()
            .filter(|(index, width)| **width > minimum_widths[*index])
            .max_by_key(|(index, width)| (**width, Reverse(*index)))
        else {
            break;
        };
        widths[index] -= 1;
    }
}

fn grow_to_budget(
    indices: &[usize],
    columns: &[Column],
    desired_widths: &[usize],
    widths: &mut [usize],
    budget: usize,
) {
    let mut remaining = budget.saturating_sub(widths.iter().sum());
    while remaining > 0 {
        let before = remaining;
        for (layout_index, column_index) in indices.iter().copied().enumerate() {
            for _ in 0..columns[column_index].grow_weight {
                if remaining == 0 {
                    return;
                }
                if widths[layout_index] < desired_widths[layout_index] {
                    widths[layout_index] += 1;
                    remaining -= 1;
                }
            }
        }
        if before == remaining {
            return;
        }
    }
}

fn render_line(cells: &[&str], columns: &[Column], layout: &Layout) -> String {
    let last_layout_index = layout.indices.len().saturating_sub(1);
    layout
        .indices
        .iter()
        .enumerate()
        .map(|(layout_index, column_index)| {
            let value = cells.get(*column_index).copied().unwrap_or_default();
            fit_cell(
                value,
                layout.widths[layout_index],
                columns[*column_index].alignment,
                layout_index != last_layout_index,
            )
        })
        .collect::<Vec<_>>()
        .join(COLUMN_SEPARATOR)
}

fn fit_cell(value: &str, width: usize, alignment: Alignment, pad_right: bool) -> String {
    let fitted = ellipsize(value, width);
    let padding = width.saturating_sub(display_width(&fitted));
    match alignment {
        Alignment::Left if pad_right => format!("{fitted}{}", " ".repeat(padding)),
        Alignment::Left => fitted,
        Alignment::Right => format!("{}{fitted}", " ".repeat(padding)),
    }
}

pub(crate) fn sanitize_cell(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| {
            if character.is_whitespace() {
                Some(' ')
            } else if character.is_control() || is_bidi_control(character) {
                None
            } else {
                Some(character)
            }
        })
        .collect()
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

pub(crate) fn ellipsize(value: &str, max_width: usize) -> String {
    if display_width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }

    let content_budget = max_width.saturating_sub(display_width("…"));
    let mut result = String::new();
    let mut used_width: usize = 0;
    for grapheme in UnicodeSegmentation::graphemes(value, true) {
        let grapheme_width = display_width(grapheme);
        if used_width.saturating_add(grapheme_width) > content_budget {
            break;
        }
        result.push_str(grapheme);
        used_width = used_width.saturating_add(grapheme_width);
    }
    result.push('…');
    result
}

pub(crate) fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

pub(crate) fn terminal_width() -> usize {
    let columns = std::env::var("COLUMNS").ok();
    select_terminal_width(columns.as_deref(), io::stdout().is_terminal(), || {
        read_terminal_size().map(|(Width(width), _)| usize::from(width))
    })
}

fn select_terminal_width<F>(
    columns: Option<&str>,
    stdout_is_terminal: bool,
    detect_width: F,
) -> usize
where
    F: FnOnce() -> Option<usize>,
{
    if let Some(width) = parse_terminal_width(columns) {
        return width;
    }
    if !stdout_is_terminal {
        return DEFAULT_TERMINAL_WIDTH;
    }

    detect_width()
        .filter(|width| *width > 0)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
}

fn parse_terminal_width(value: Option<&str>) -> Option<usize> {
    value?.parse().ok().filter(|width: &usize| *width > 0)
}

/// Returns the shortest unambiguous compact UUID prefixes in input order.
///
/// Prefixes contain at least eight hexadecimal digits and can be passed
/// directly to the selector resolver. Duplicate UUIDs cannot be disambiguated,
/// so duplicates are returned as full canonical UUIDs.
pub(crate) fn short_unique_ids(ids: &[Uuid]) -> Vec<String> {
    let compact = ids
        .iter()
        .map(|id| id.simple().to_string())
        .collect::<Vec<_>>();
    compact
        .iter()
        .enumerate()
        .map(|(index, id)| {
            if compact
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != index && other == id)
            {
                return ids[index].to_string();
            }

            let mut prefix_bytes = MIN_UUID_PREFIX_BYTES.min(id.len());
            while prefix_bytes < id.len()
                && compact.iter().enumerate().any(|(other_index, other)| {
                    other_index != index && other.starts_with(&id[..prefix_bytes])
                })
            {
                prefix_bytes += 1;
            }
            id[..prefix_bytes].to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn representative_table() -> Table {
        let mut table = Table::new([
            Column::required("ID", 8, 36),
            Column::required("Title", 12, 48).flex(4),
            Column::optional("Pri", 3, 3, 40).align(Alignment::Right),
            Column::optional("Due", 10, 10, 30),
            Column::optional("Project", 10, 24, 20).flex(2),
            Column::required("Status", 6, 10),
            Column::optional("Updated", 16, 16, 10),
        ]);
        table.push_row([
            "0198f128-2516-7a2a-bd8f-a98a3d8f1150",
            "Coordinate the international launch with the Prague team",
            "8",
            "2026-07-30",
            "Příliš žluťoučký 🦭",
            "Active",
            "2026-07-26 14:30",
        ]);
        table
    }

    #[test]
    fn adapts_at_common_terminal_widths_and_hides_low_priority_columns_first() {
        let table = representative_table();
        let cases = [
            (40, vec!["ID", "Title", "Pri", "Status"]),
            (72, vec!["ID", "Title", "Pri", "Due", "Project", "Status"]),
            (
                100,
                vec!["ID", "Title", "Pri", "Due", "Project", "Status", "Updated"],
            ),
            (
                140,
                vec!["ID", "Title", "Pri", "Due", "Project", "Status", "Updated"],
            ),
        ];

        for (width, expected_headers) in cases {
            let layout = table.layout(width);
            let headers = layout
                .indices
                .iter()
                .map(|index| table.columns[*index].header.as_str())
                .collect::<Vec<_>>();
            assert_eq!(headers, expected_headers, "terminal width {width}");

            let rendered = table.render_with_width(width);
            for line in rendered.lines() {
                assert!(
                    display_width(line) <= width,
                    "line exceeds {width} columns: {line:?}"
                );
            }
        }
    }

    #[test]
    fn ellipsis_never_splits_a_grapheme_cluster() {
        let combined = "Cafe\u{301} noir";
        assert_eq!(ellipsize(combined, 5), "Cafe\u{301}…");

        let emoji = ellipsize("👩‍💻abc", 3);
        assert_eq!(emoji, "👩‍💻…");
        assert_eq!(
            UnicodeSegmentation::graphemes(emoji.as_str(), true).count(),
            2
        );
        assert_eq!(display_width(&emoji), 3);
    }

    #[test]
    fn measures_and_pads_cells_by_unicode_display_width() {
        assert_eq!(display_width("Příliš"), 6);
        assert_eq!(display_width("界"), 2);
        assert_eq!(display_width("e\u{301}"), 1);

        let mut table = Table::new([
            Column::required("Name", 4, 4),
            Column::required("N", 1, 1).align(Alignment::Right),
        ]);
        table.push_row(["界", "7"]);
        assert_eq!(table.render_with_width(7).lines().nth(2), Some("界    7"));
    }

    #[test]
    fn sanitizes_cells_before_they_can_inject_rows_or_terminal_controls() {
        let unsafe_value =
            "hello\nforged\u{1b}[31m\tred\u{7}\u{202e}reversed\u{202c}\u{2066}isolated\u{2069}";
        assert_eq!(
            sanitize_cell(unsafe_value),
            "hello forged[31m redreversedisolated"
        );

        let mut table = Table::new([Column::required("Value", 5, 40)]);
        table.push_row([unsafe_value]);
        let rendered = table.render_with_width(40);
        assert_eq!(rendered.lines().count(), 3);
        assert!(
            rendered
                .chars()
                .all(|character| !character.is_control() || character == '\n')
        );
        assert!(rendered.contains("hello forged[31m red"));
    }

    #[test]
    fn preserved_columns_never_ellipsize_actionable_values() {
        let selector = "id:018f4a76c";
        let mut table = Table::new([
            Column::required("ID", 4, 8).preserve(),
            Column::required("Title", 4, 12),
        ]);
        table.push_row([selector, "Task"]);

        let rendered = table.render_with_width(8);
        let row = rendered.lines().nth(2).expect("data row");

        assert_eq!(row, selector);
        assert!(!row.contains('…'));
        assert!(display_width(row) > 8);
    }

    #[test]
    fn strips_bidi_controls_without_removing_legitimate_joiners() {
        let bidi_controls = [
            '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
            '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
        ];
        let unsafe_value = bidi_controls
            .into_iter()
            .flat_map(|control| ['a', control])
            .collect::<String>();

        assert_eq!(sanitize_cell(&unsafe_value), "aaaaaaaaaaaa");
        assert_eq!(sanitize_cell("👩‍💻"), "👩‍💻");
    }

    #[test]
    fn computes_short_uuid_prefixes_that_remain_unique() {
        let ids = [
            Uuid::parse_str("018f4a76-c9f2-7f38-a09a-2ac748db8ee8").expect("UUID"),
            Uuid::parse_str("018f4a76-d9f2-7f38-a09a-2ac748db8ee8").expect("UUID"),
            Uuid::parse_str("018f4a77-c9f2-7f38-a09a-2ac748db8ee8").expect("UUID"),
        ];

        assert_eq!(
            short_unique_ids(&ids),
            ["018f4a76c", "018f4a76d", "018f4a77"]
        );
    }

    #[test]
    fn duplicate_uuids_fall_back_to_the_full_canonical_value() {
        let id = Uuid::parse_str("018f4a76-c9f2-7f38-a09a-2ac748db8ee8").expect("UUID");
        assert_eq!(
            short_unique_ids(&[id, id]),
            [id.to_string(), id.to_string()]
        );
    }

    #[test]
    fn terminal_width_selection_prefers_valid_columns() {
        assert_eq!(
            select_terminal_width(Some("72"), true, || panic!("must not probe")),
            72
        );
        assert_eq!(
            select_terminal_width(Some("72"), false, || panic!("must not probe")),
            72
        );
    }

    #[test]
    fn terminal_width_selection_probes_only_for_tty_output() {
        assert_eq!(select_terminal_width(None, true, || Some(140)), 140);
        assert_eq!(
            select_terminal_width(Some("invalid"), true, || Some(72)),
            72
        );
        assert_eq!(
            select_terminal_width(Some("0"), false, || panic!("must not probe")),
            DEFAULT_TERMINAL_WIDTH
        );
        assert_eq!(
            select_terminal_width(None, false, || panic!("must not probe")),
            DEFAULT_TERMINAL_WIDTH
        );
        assert_eq!(
            select_terminal_width(None, true, || None),
            DEFAULT_TERMINAL_WIDTH
        );
    }
}
