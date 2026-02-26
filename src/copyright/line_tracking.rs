use super::prepare::prepare_text_line;

pub(super) struct PreparedLineCache<'a> {
    raw_lines: &'a [&'a str],
    prepared: Vec<Option<String>>,
}

impl<'a> PreparedLineCache<'a> {
    pub(super) fn new(raw_lines: &'a [&'a str]) -> Self {
        Self {
            raw_lines,
            prepared: vec![None; raw_lines.len()],
        }
    }

    pub(super) fn raw_line_count(&self) -> usize {
        self.raw_lines.len()
    }

    pub(super) fn get(&mut self, line_number: usize) -> Option<&str> {
        let idx = line_number.checked_sub(1)?;
        if idx >= self.raw_lines.len() {
            return None;
        }
        if self.prepared[idx].is_none() {
            let raw = self.raw_lines[idx];
            self.prepared[idx] = Some(prepare_text_line(raw));
        }
        self.prepared[idx].as_deref()
    }
}

pub(super) struct LineNumberIndex {
    newline_offsets: Vec<usize>,
    content_len: usize,
}

impl LineNumberIndex {
    pub(super) fn new(content: &str) -> Self {
        let newline_offsets = content
            .as_bytes()
            .iter()
            .enumerate()
            .filter_map(|(idx, b)| (*b == b'\n').then_some(idx))
            .collect();

        Self {
            newline_offsets,
            content_len: content.len(),
        }
    }

    pub(super) fn line_number_at_offset(&self, byte_offset: usize) -> usize {
        let offset = byte_offset.min(self.content_len);
        1 + self
            .newline_offsets
            .partition_point(|&line_break| line_break < offset)
    }
}
