use super::SelectionMode;

pub struct Line;

impl SelectionMode for Line {
    fn name(&self) -> &'static str {
        "LINE"
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let len_lines = buffer.len_lines();

        Ok(Box::new((0..len_lines).filter_map(move |line_index| {
            let line = buffer.get_line_by_line_index(line_index)?;
            let start = buffer.line_to_byte(line_index).ok()?;
            let end = start + line.len_bytes();

            // This is a weird hack, because `rope.len_lines`
            // returns an extra line which is empty if the rope ends with the newline character
            if start == end {
                return None;
            }

            Some(super::ByteRange::new(start..end))
        })))
    }
    fn up(
        &self,
        super::SelectionModeParams {
            buffer,
            current_selection,
            ..
        }: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let current_line = buffer.char_to_line(current_selection.extended_range().start)?;
        buffer
            .get_parent_lines(current_line)?
            .into_iter()
            .filter(|line| line.line < current_line)
            .next_back()
            .map(|line| {
                buffer
                    .line_to_byte_range(line.line)?
                    .to_selection(buffer, current_selection)
            })
            .transpose()
    }
}

#[cfg(test)]
mod test_line {
    use crate::{buffer::Buffer, context::Context, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a\n\n\nb\nc\n");
        Line.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..2, "a\n"),
                (2..3, "\n"),
                (3..4, "\n"),
                (4..6, "b\n"),
                (6..8, "c\n"),
            ],
        );
    }

    #[test]
    fn single_line_without_trailing_newline_character() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a");
        Line.assert_all_selections(&buffer, Selection::default(), &[(0..1, "a")]);
    }

    #[test]
    fn up() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "
fn f() {
    fn g() {
        let a = 1;
        let b = 2;
        let c = 3;
        let d = 4;
    }

}"
            .trim(),
        );

        let test = |selected_line: usize, expected: &str| {
            let start = buffer.line_to_char(selected_line).unwrap();
            let result = Line
                .up(crate::selection_mode::SelectionModeParams {
                    context: &Context::default(),
                    buffer: &buffer,
                    current_selection: &Selection::new((start..start + 1).into()),
                    cursor_direction: &crate::components::editor::CursorDirection::End,
                })
                .unwrap()
                .unwrap();

            let actual = buffer.slice(&result.extended_range()).unwrap();
            assert_eq!(actual, expected);
        };

        test(4, "    fn g() {");

        test(1, "fn f() {");
    }
}
