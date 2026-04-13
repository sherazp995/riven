use riven_core::lexer::token::Span;

/// Stores the byte offset of each line start for fast position lookups.
pub struct LineIndex {
    /// line_starts[i] = byte offset where line i begins (0-indexed)
    line_starts: Vec<u32>,
    /// The full source text (needed for UTF-16 conversion)
    source: String,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self {
            line_starts,
            source: source.to_string(),
        }
    }

    /// Convert a byte offset to an LSP Position.
    pub fn position_of(&self, byte_offset: usize) -> lsp_types::Position {
        let byte_offset = byte_offset.min(self.source.len());
        let line =
            self.line_starts.partition_point(|&start| (start as usize) <= byte_offset).saturating_sub(1);
        let line_start = self.line_starts[line] as usize;

        // Convert byte column to UTF-16 column
        let end = byte_offset.min(self.source.len());
        let line_text = &self.source[line_start..end];
        let utf16_col = line_text.encode_utf16().count();

        lsp_types::Position {
            line: line as u32,
            character: utf16_col as u32,
        }
    }

    /// Convert an LSP Position to a byte offset.
    pub fn byte_offset_of(&self, position: lsp_types::Position) -> usize {
        let line = position.line as usize;
        if line >= self.line_starts.len() {
            return self.source.len();
        }
        let line_start = self.line_starts[line] as usize;

        let line_end = self
            .line_starts
            .get(line + 1)
            .map(|&s| s as usize)
            .unwrap_or(self.source.len());
        let line_text = &self.source[line_start..line_end];

        let mut utf16_count = 0u32;
        let mut byte_offset = line_start;
        for ch in line_text.chars() {
            if utf16_count >= position.character {
                break;
            }
            utf16_count += ch.len_utf16() as u32;
            byte_offset += ch.len_utf8();
        }
        byte_offset
    }

    /// Convert a Riven Span to an LSP Range.
    pub fn span_to_range(&self, span: &Span) -> lsp_types::Range {
        lsp_types::Range {
            start: self.position_of(span.start),
            end: self.position_of(span.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line() {
        let idx = LineIndex::new("hello");
        let pos = idx.position_of(3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn multi_line() {
        let idx = LineIndex::new("aaa\nbbb\nccc");
        assert_eq!(
            idx.position_of(4),
            lsp_types::Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            idx.position_of(8),
            lsp_types::Position {
                line: 2,
                character: 0
            }
        );
    }

    #[test]
    fn utf8_multi_byte() {
        // e-acute is 2 bytes in UTF-8, 1 code unit in UTF-16
        let idx = LineIndex::new("caf\u{00e9}!");
        let pos = idx.position_of(5);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 4);
    }

    #[test]
    fn emoji_utf16_surrogate_pair() {
        // Emoji is 4 bytes in UTF-8, 2 code units in UTF-16
        let idx = LineIndex::new("a\u{1F600}b");
        let pos = idx.position_of(5);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn roundtrip_byte_offset() {
        let idx = LineIndex::new("let x = 42\nlet y = 99\n");
        let pos = idx.position_of(11); // '\n' at end of first line
        let offset = idx.byte_offset_of(pos);
        assert_eq!(offset, 11);
    }

    #[test]
    fn empty_source() {
        let idx = LineIndex::new("");
        let pos = idx.position_of(0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn span_to_range() {
        let idx = LineIndex::new("let x = 42\nlet y = 99\n");
        let span = Span {
            start: 4,
            end: 5,
            line: 1,
            column: 5,
        };
        let range = idx.span_to_range(&span);
        assert_eq!(
            range.start,
            lsp_types::Position {
                line: 0,
                character: 4
            }
        );
        assert_eq!(
            range.end,
            lsp_types::Position {
                line: 0,
                character: 5
            }
        );
    }

    #[test]
    fn byte_offset_of_second_line() {
        let idx = LineIndex::new("let x = 42\nlet y = 99\n");
        let offset = idx.byte_offset_of(lsp_types::Position {
            line: 1,
            character: 4,
        });
        // line 1 starts at byte 11, char 4 = byte 15
        assert_eq!(offset, 15);
    }
}
