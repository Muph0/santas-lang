use std::{fmt, hash::Hash, sync::Arc};

use super::{ECode, Error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loc {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}
impl Loc {
    pub(crate) fn new(line: u32, col: u32, len: u32) -> Self {
        Self { line, col, len }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct SourceStr {
    pub source_name: Arc<str>,
    pub string: Arc<str>,
    pub loc: Loc,
}
impl SourceStr {
    pub fn display_at(&self) -> impl fmt::Display {
        DisplaySourceStr(self)
    }
}
impl PartialEq for SourceStr {
    fn eq(&self, other: &Self) -> bool {
        self.string == other.string
    }
}
impl Hash for SourceStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.string.hash(state);
    }
}
struct DisplaySourceStr<'a>(&'a SourceStr);
impl<'a> fmt::Display for DisplaySourceStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" at {}:{}:{}",
            self.0.string, self.0.source_name, self.0.loc.line, self.0.loc.col
        )
    }
}

pub(crate) struct LineMap<'i> {
    source_name: Arc<str>,
    newlines: Vec<usize>, // positions of '\n'
    source: &'i str,
}
impl<'i> LineMap<'i> {
    /// Build from a &str, storing line endings
    pub fn new(source_name: &Arc<str>, source: &'i str) -> Self {
        let mut newlines = Vec::new();
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                newlines.push(i);
            }
        }
        Self {
            source_name: source_name.clone(),
            newlines,
            source,
        }
    }

    pub fn slice_loc(&self, sub: &'i str) -> Loc {
        // Find the starting byte offset of `sub` inside `source`
        let start = {
            let base = self.source.as_ptr() as usize;
            let sub_ptr = sub.as_ptr() as usize;
            sub_ptr - base
        };

        let len = sub.len() as u32;

        // Find the line number using binary search in newline positions
        let line_index = match self.newlines.binary_search(&start) {
            Ok(idx) => idx,  // exactly at a newline → same line
            Err(idx) => idx, // number of newlines before start
        };

        // Compute column: distance from last newline (or start of file)
        let col = if line_index == 0 {
            start as u32
        } else {
            (start - self.newlines[line_index - 1] - 1) as u32
        };

        Loc {
            line: line_index as u32 + 1, // lines are 1-based
            col: col,                    // columns are 0-based
            len,
        }
    }

    /// Map a subslice back to its starting (line, col, len) in the original string.
    pub fn map_slice(&self, sub: &'i str) -> SourceStr {
        let loc = self.slice_loc(sub);

        SourceStr {
            source_name: self.source_name.clone(),
            string: Arc::from(sub),
            loc,
        }
    }

    pub fn error_at(&self, sub: &'i str, code: ECode) -> Error {
        Error {
            source_name: self.source_name.clone(),
            loc: Some(self.slice_loc(sub)),
            code,
        }
    }

    pub(crate) fn error_at_loc(&self, loc: Loc, code: ECode) -> Error {
        Error {
            source_name: self.source_name.clone(),
            loc: Some(loc),
            code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_map(src: &str) -> LineMap<'_> {
        let name: Arc<str> = Arc::from("test_source");
        LineMap::new(&name, src)
    }

    #[test]
    fn maps_first_line() {
        let src = "hello world\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[0..5]; // "hello"
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "hello");
        assert_eq!(sstr.loc.line, 1);
        assert_eq!(sstr.loc.col, 0);
        assert_eq!(sstr.loc.len, 5);
    }

    #[test]
    fn newline_is_same_line() {
        let src = "hello world\r\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[12..13];
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "\n");
        assert_eq!(sstr.loc.line, 1);
        assert_eq!(sstr.loc.col, 12);
        assert_eq!(sstr.loc.len, 1);
    }

    #[test]
    fn maps_middle_of_first_line() {
        let src = "hello world\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[6..11]; // "world"
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "world");
        assert_eq!(sstr.loc.line, 1);
        assert_eq!(sstr.loc.col, 6);
        assert_eq!(sstr.loc.len, 5);
    }

    #[test]
    fn maps_second_line() {
        let src = "hello world\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[12..18]; // "second"
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "second");
        assert_eq!(sstr.loc.line, 2);
        assert_eq!(sstr.loc.col, 0);
        assert_eq!(sstr.loc.len, 6);
    }

    #[test]
    fn maps_column_in_second_line() {
        let src = "hello world\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[19..23]; // "line"
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "line");
        assert_eq!(sstr.loc.line, 2);
        assert_eq!(sstr.loc.col, 7);
        assert_eq!(sstr.loc.len, 4);
    }

    #[test]
    fn maps_third_line_offset() {
        let src = "hello world\nsecond line\nthird";
        let map = make_map(src);

        let sub = &src[24..29]; // "third"
        let sstr = map.map_slice(sub);

        assert_eq!(sstr.string.as_ref(), "third");
        assert_eq!(sstr.loc.line, 3);
        assert_eq!(sstr.loc.col, 0);
        assert_eq!(sstr.loc.len, 5);
    }
}
