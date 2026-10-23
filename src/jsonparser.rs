use std::fmt::Write;
use std::ops::Range;

use logos::{Lexer, Logos};

use crate::flatjson::{ContainerType, Index, OptionIndex, Row, Value};
use crate::jsontokenizer::JsonToken;

// Canonical text of the invalid row closing containers cut short by the end of input or of a record.
const TRUNCATED: &str = "<truncated>";

struct JsonParser<'a> {
    tokenizer: Lexer<'a, JsonToken>,
    parents: Vec<Index>,
    rows: Vec<Row>,
    pretty_printed: String,
    max_depth: usize,

    peeked_token: Option<Option<JsonToken>>,

    // 1-based input lines of parse errors.
    error_lines: Vec<usize>,
    // Line number at byte offset `line_offset`, for counting lines incrementally.
    line: usize,
    line_offset: usize,
    // Set by recovery to close all open containers and resume at the top level.
    unwinding: bool,
}

// Parses a sequence of top-level values, recovering from errors by recording
// unparsable input as `Value::Invalid` rows. Fails only if no valid value is found.
pub fn parse(json: String) -> Result<(Vec<Row>, String, usize, Vec<usize>), String> {
    let mut parser = JsonParser {
        tokenizer: JsonToken::lexer(&json),
        parents: vec![],
        rows: vec![],
        pretty_printed: String::new(),
        max_depth: 0,
        peeked_token: None,
        error_lines: vec![],
        line: 1,
        line_offset: 0,
        unwinding: false,
    };

    parser.parse_top_level_json();

    if parser
        .rows
        .iter()
        .all(|row| matches!(row.value, Value::Invalid))
    {
        return Err(match parser.error_lines.first() {
            Some(line) => format!("No valid JSON found (first parse error on line {line})"),
            None => "Empty input".to_string(),
        });
    }

    Ok((
        parser.rows,
        parser.pretty_printed,
        parser.max_depth,
        parser.error_lines,
    ))
}

impl<'a> JsonParser<'a> {
    fn next_token(&mut self) -> Option<JsonToken> {
        if self.peeked_token.is_some() {
            self.peeked_token.take().unwrap()
        } else {
            self.tokenizer.next()
        }
    }

    fn advance(&mut self) {
        self.next_token();
    }

    fn advance_and_consume_whitespace(&mut self) {
        self.advance();
        self.consume_whitespace();
    }

    fn peek_token_or_eof(&mut self) -> Option<JsonToken> {
        if self.peeked_token.is_none() {
            self.peeked_token = Some(self.tokenizer.next());
        }

        self.peeked_token.unwrap()
    }

    // Source offset of the peeked token, or the input length at EOF.
    fn peeked_token_start(&mut self) -> usize {
        match self.peek_token_or_eof() {
            Some(_) => self.tokenizer.span().start,
            None => self.tokenizer.source().len(),
        }
    }

    fn consume_whitespace(&mut self) {
        while let Some(JsonToken::Whitespace | JsonToken::Newline) = self.peek_token_or_eof() {
            self.advance();
        }
    }

    fn parse_top_level_json(&mut self) {
        let mut prev_top_level = OptionIndex::Nil;
        let mut num_child = 0;

        loop {
            self.consume_whitespace();

            if self.peek_token_or_eof().is_none() {
                break;
            }

            self.unwinding = false;

            if prev_top_level.is_some() {
                self.pretty_printed.push('\n');
            }

            let next_top_level = self.parse_elem().unwrap_or_else(|()| self.recover());

            self.link_sibling(next_top_level, prev_top_level, num_child);
            prev_top_level = OptionIndex::Index(next_top_level);
            num_child += 1;
        }
    }

    fn link_sibling(&mut self, child: Index, prev_sibling: OptionIndex, index_in_parent: usize) {
        self.rows[child].prev_sibling = prev_sibling;
        self.rows[child].index_in_parent = index_in_parent;
        if let OptionIndex::Index(prev) = prev_sibling {
            self.rows[prev].next_sibling = OptionIndex::Index(child);
        }
    }

    // Fails on an unexpected token (or end of input) at the start of the value.
    fn parse_elem(&mut self) -> Result<usize, ()> {
        self.consume_whitespace();

        self.max_depth = self.max_depth.max(self.parents.len());

        match self.peek_token_or_eof() {
            Some(JsonToken::OpenCurly) => Ok(self.parse_container(ContainerType::Object)),
            Some(JsonToken::OpenSquare) => Ok(self.parse_container(ContainerType::Array)),
            Some(JsonToken::Null) => Ok(self.parse_primitive(Value::Null)),
            Some(JsonToken::True | JsonToken::False) => Ok(self.parse_primitive(Value::Boolean)),
            Some(JsonToken::Number) => Ok(self.parse_primitive(Value::Number)),
            Some(JsonToken::String) => Ok(self.parse_primitive(Value::String)),

            Some(JsonToken::Whitespace | JsonToken::Newline) => {
                panic!("Should have just consumed whitespace");
            }

            Some(
                JsonToken::Error
                | JsonToken::InvalidString
                | JsonToken::CloseCurly
                | JsonToken::CloseSquare
                | JsonToken::Colon
                | JsonToken::Comma,
            )
            | None => Err(()),
        }
    }

    // Parses a container; errors inside it are recovered from, so it always succeeds.
    fn parse_container(&mut self, container_type: ContainerType) -> usize {
        let open_index = self.create_row(Value::OpenContainer {
            container_type,
            collapsed: false,
            // To be set when parsing is complete.
            first_child: 0,
            close_index: 0,
        });

        self.parents.push(open_index);
        self.pretty_printed.push_str(container_type.open_str());
        self.advance_and_consume_whitespace();

        let mut prev_sibling = OptionIndex::Nil;
        let mut num_children = 0;
        let mut needs_separator = false;

        while !self.unwinding {
            let mut key_range = None;
            let child = match self.parse_container_entry(
                container_type,
                num_children,
                needs_separator,
                &mut key_range,
            ) {
                Ok(Some(child)) => {
                    needs_separator = true;
                    child
                }
                Ok(None) => {
                    self.advance();
                    break;
                }
                Err(()) => {
                    // Recovery resumes at the start of an entry; a comma is optional there.
                    needs_separator = false;
                    self.recover()
                }
            };
            self.rows[child].key_range = key_range;
            self.consume_whitespace();

            if num_children == 0
                && let Value::OpenContainer {
                    ref mut first_child,
                    ..
                } = self.rows[open_index].value
            {
                *first_child = child;
            }

            self.link_sibling(child, prev_sibling, num_children);
            num_children += 1;
            prev_sibling = OptionIndex::Index(child);
        }

        self.parents.pop();

        if num_children == 0 {
            self.rows[open_index].value = match container_type {
                ContainerType::Object => Value::EmptyObject,
                ContainerType::Array => Value::EmptyArray,
            };
            self.rows[open_index].range.end = self.rows[open_index].range.start + 2;
        } else {
            if container_type == ContainerType::Object {
                // Print space inside closing brace.
                self.pretty_printed.push(' ');
            }

            let close_index = self.create_row(Value::CloseContainer {
                container_type,
                collapsed: false,
                last_child: prev_sibling.unwrap(),
                open_index,
            });

            // Update end of the container range; we add the closing
            // character to pretty_printed below, hence the + 1.
            self.rows[open_index].range.end = self.pretty_printed.len() + 1;

            if let Value::OpenContainer {
                close_index: ref mut index,
                ..
            } = self.rows[open_index].value
            {
                *index = close_index;
            }
        }

        self.pretty_printed.push_str(container_type.close_str());
        open_index
    }

    // Parses a separator if present (a trailing comma is allowed), then a key
    // (objects) and a value. Returns None at the closing token. A key parsed
    // before a failure is stored in key_range.
    fn parse_container_entry(
        &mut self,
        container_type: ContainerType,
        num_children: usize,
        needs_separator: bool,
        key_range: &mut Option<Range<usize>>,
    ) -> Result<Option<usize>, ()> {
        let close_token = match container_type {
            ContainerType::Object => JsonToken::CloseCurly,
            ContainerType::Array => JsonToken::CloseSquare,
        };

        let mut has_separator = false;
        if num_children != 0 && self.peek_token_or_eof() == Some(JsonToken::Comma) {
            has_separator = true;
            self.advance_and_consume_whitespace();
        }

        if self.peek_token_or_eof() == Some(close_token) {
            return Ok(None);
        }

        // An entry (possibly invalid) follows, so print its separator.
        if num_children != 0 {
            self.pretty_printed.push_str(", ");
        } else if container_type == ContainerType::Object {
            // Add space inside objects.
            self.pretty_printed.push(' ');
        }

        if needs_separator && !has_separator {
            return Err(());
        }

        if container_type == ContainerType::Object {
            if self.peek_token_or_eof() != Some(JsonToken::String) {
                return Err(());
            }

            let key_start = self.pretty_printed.len();
            self.pretty_printed.push_str(self.tokenizer.slice());
            *key_range = Some(key_start..self.pretty_printed.len());
            self.pretty_printed.push_str(": ");
            self.advance_and_consume_whitespace();

            if self.peek_token_or_eof() != Some(JsonToken::Colon) {
                return Err(());
            }
            self.advance_and_consume_whitespace();
        }

        self.parse_elem().map(Some)
    }

    fn parse_primitive(&mut self, value: Value) -> usize {
        let row_index = self.create_row(value);

        // String tokens include the quotation marks.
        self.pretty_printed.push_str(self.tokenizer.slice());
        self.rows[row_index].range.end = self.pretty_printed.len();

        self.advance();
        row_index
    }

    // Records unparsable input at the peeked token as an invalid row and skips
    // past it. The invalid entry extends to a comma or closing bracket at its
    // nesting level, or to the end of the line. At the end of input or the
    // next record (see `at_unwinding_point`), the open containers are instead
    // truncated.
    fn recover(&mut self) -> usize {
        let source = self.tokenizer.source();
        let start = self.peeked_token_start();

        let (error_offset, text) = if self.at_unwinding_point() {
            self.unwinding = true;
            let end_of_truncated_input = source[..start].trim_end().len();
            (end_of_truncated_input, "")
        } else {
            let first_token = self.next_token();
            let mut end = self.tokenizer.span().end;
            let mut depth = match first_token {
                Some(JsonToken::OpenCurly | JsonToken::OpenSquare) => 1,
                _ => 0,
            };

            while first_token != Some(JsonToken::Comma) {
                match self.peek_token_or_eof() {
                    None | Some(JsonToken::Newline) => break,
                    Some(JsonToken::Comma | JsonToken::CloseCurly | JsonToken::CloseSquare)
                        if depth == 0 =>
                    {
                        break;
                    }
                    Some(JsonToken::OpenCurly | JsonToken::OpenSquare) => depth += 1,
                    Some(JsonToken::CloseCurly | JsonToken::CloseSquare) => depth -= 1,
                    Some(_) => {}
                }
                self.advance();
                end = self.tokenizer.span().end;
            }

            self.consume_whitespace();
            self.unwinding = self.at_unwinding_point();
            (start, &source[start..end])
        };

        self.record_error_line(error_offset);

        let row_index = self.create_row(Value::Invalid);
        if text.is_empty() {
            self.pretty_printed.push_str(TRUNCATED);
        }
        for ch in text.chars() {
            if ch.is_control() {
                write!(self.pretty_printed, "\\u{:04x}", ch as u32).unwrap();
            } else {
                self.pretty_printed.push(ch);
            }
        }
        self.rows[row_index].range.end = self.pretty_printed.len();
        row_index
    }

    // Whether recovery should close all open containers: at the end of input,
    // or inside a container at an unindented line not starting with a
    // separator or closing bracket, which is most likely the next JSON Lines
    // record.
    fn at_unwinding_point(&mut self) -> bool {
        match self.peek_token_or_eof() {
            None => true,
            Some(
                JsonToken::Comma
                | JsonToken::Colon
                | JsonToken::CloseCurly
                | JsonToken::CloseSquare,
            ) => false,
            Some(_) => {
                let start = self.tokenizer.span().start;
                !self.parents.is_empty()
                    && (start == 0 || self.tokenizer.source().as_bytes()[start - 1] == b'\n')
            }
        }
    }

    fn record_error_line(&mut self, offset: usize) {
        debug_assert!(self.line_offset <= offset);
        let source = self.tokenizer.source();
        self.line += source.as_bytes()[self.line_offset..offset]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        self.line_offset = offset;
        self.error_lines.push(self.line);
    }

    // Add a new row to the FlatJson representation.
    //
    // self.pretty_printed should NOT include the added row yet;
    // we use the current length of self.pretty_printed as the
    // starting index of the row's range.
    fn create_row(&mut self, value: Value) -> usize {
        let index = self.rows.len();

        let parent = match self.parents.last() {
            None => OptionIndex::Nil,
            Some(row_index) => OptionIndex::Index(*row_index),
        };

        let range_start = self.pretty_printed.len();

        self.rows.push(Row {
            // Set correctly by us
            parent,
            depth: self.parents.len(),
            value,

            // The start of this range is set by us, but then we set
            // the end when we're done parsing the row. We'll set
            // the default end to be one character so we don't have to
            // update it after ']' and '}'.
            range: range_start..range_start + 1,

            // To be filled in by caller
            prev_sibling: OptionIndex::Nil,
            next_sibling: OptionIndex::Nil,
            index_in_parent: 0,
            key_range: None,
        });

        index
    }
}
#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::flatjson::parse_top_level_json;

    // Returns the pretty-printed document, the texts of invalid rows, and error lines.
    fn parse_lenient(json: &str) -> (String, Vec<String>, Vec<usize>) {
        let fj = parse_top_level_json(json.to_owned()).unwrap();
        let invalid =
            fj.0.iter()
                .filter(|row| matches!(row.value, Value::Invalid))
                .map(|row| fj.1[row.range.clone()].to_owned())
                .collect();
        (fj.pretty_printed(), invalid, fj.3.clone())
    }

    #[test]
    fn test_valid_input_has_no_errors() {
        let (_, invalid, error_lines) = parse_lenient("{\"a\": [1, 2]}\n[3]\n");
        assert!(invalid.is_empty());
        assert!(error_lines.is_empty());
    }

    #[test]
    fn test_jsonl_truncated_line_closes_containers() {
        let (pretty, invalid, error_lines) =
            parse_lenient("{\"a\": 1}\n{\"b\": {\"c\": \"tru\n{\"d\": [4]}\n");
        assert_eq!(
            indoc! {r#"
                {
                  "a": 1
                }
                {
                  "b": {
                    "c": "tru
                  }
                }
                {
                  "d": [
                    4
                  ]
                }
            "#},
            pretty
        );
        assert_eq!(vec!["\"tru"], invalid);
        assert_eq!(vec![2], error_lines);
    }

    #[test]
    fn test_jsonl_line_ending_inside_container() {
        let (pretty, invalid, error_lines) = parse_lenient("{\"a\": [1\n\n{\"b\": 2}\n");
        assert_eq!(
            indoc! {r#"
                {
                  "a": [
                    1,
                    <truncated>
                  ]
                }
                {
                  "b": 2
                }
            "#},
            pretty
        );
        assert_eq!(vec![TRUNCATED], invalid);
        assert_eq!(vec![1], error_lines);
    }

    #[test]
    fn test_truncated_input_closes_containers() {
        let (pretty, invalid, error_lines) = parse_lenient("[1, {\"a\": [2,\n");
        assert_eq!(
            indoc! {r#"
                [
                  1,
                  {
                    "a": [
                      2,
                      <truncated>
                    ]
                  }
                ]
            "#},
            pretty
        );
        assert_eq!(vec![TRUNCATED], invalid);
        assert_eq!(vec![1], error_lines);
    }

    #[test]
    fn test_invalid_entries_are_skipped() {
        let json = indoc! {r#"
            {
              "a": 1
              "b": 2,
              "c": [3, x]
            }
            [1,,2, x {"y": [5, 6]}, {"a" 3}, [4 5], "\u00"]
        "#};
        let (pretty, invalid, error_lines) = parse_lenient(json);
        assert_eq!(
            indoc! {r#"
                {
                  "a": 1,
                  "b": 2,
                  "c": [
                    3,
                    x
                  ]
                }
                [
                  1,
                  ,,
                  2,
                  x {"y": [5, 6]},
                  {
                    "a": 3
                  },
                  [
                    4,
                    5
                  ],
                  "\u00"
                ]
            "#},
            pretty
        );
        assert_eq!(
            vec![
                "\"b\": 2",
                "x",
                ",",
                "x {\"y\": [5, 6]}",
                "3",
                "5",
                "\"\\u00\""
            ],
            invalid
        );
        assert_eq!(vec![3, 4, 6, 6, 6, 6, 6], error_lines);
    }

    #[test]
    fn test_trailing_commas_are_allowed() {
        let (pretty, invalid, _) = parse_lenient("[1, 2,]\n{\"a\": 3,\n}");
        assert_eq!("[\n  1,\n  2\n]\n{\n  \"a\": 3\n}\n", pretty);
        assert!(invalid.is_empty());
    }

    #[test]
    fn test_invalid_top_level_line() {
        let (pretty, invalid, error_lines) = parse_lenient("1\nnot json\r\n[2]\n");
        assert_eq!("1\nnot json\n[\n  2\n]\n", pretty);
        assert_eq!(vec!["not json"], invalid);
        assert_eq!(vec![2], error_lines);
    }

    #[test]
    fn test_invalid_text_escapes_control_characters() {
        let (_, invalid, _) = parse_lenient("[\u{1b}x\ty, \"a\tb\", 1]");
        assert_eq!(vec!["\\u001bx\\u0009y", "\"a\\u0009b\""], invalid);
    }

    #[test]
    fn test_input_without_valid_values_is_error() {
        assert!(parse_top_level_json("not json".to_owned()).is_err());
        assert!(parse_top_level_json("x\n}\n".to_owned()).is_err());
        assert!(parse_top_level_json("".to_owned()).is_err());
        assert!(parse_top_level_json(" \n ".to_owned()).is_err());
    }

    #[test]
    fn test_row_ranges() {
        //            0 2    7  10   15    21   26    32     39 42
        let json = r#"{ "a": 1, "b": true, "c": null, "ddd": [] }"#.to_owned();
        let (rows, _, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..43); // Object
        assert_eq!(rows[1].key_range, Some(2..5)); // "a": 1
        assert_eq!(rows[1].range, 7..8); // "a": 1
        assert_eq!(rows[2].key_range, Some(10..13)); // "b": true
        assert_eq!(rows[2].range, 15..19); // "b": true
        assert_eq!(rows[3].key_range, Some(21..24)); // "c": null
        assert_eq!(rows[3].range, 26..30); // "c": null
        assert_eq!(rows[4].range, 39..41); // "ddd": []
        assert_eq!(rows[5].range, 42..43); // }

        //            01   5        14     21 23
        let json = r#"[14, "apple", false, {}]"#.to_owned();
        let (rows, _, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..24); // Array
        assert_eq!(rows[1].range, 1..3); // 14
        assert_eq!(rows[2].range, 5..12); // "apple"
        assert_eq!(rows[3].range, 14..19); // false
        assert_eq!(rows[4].range, 21..23); // {}
        assert_eq!(rows[5].range, 23..24); // ]

        //            01 3      10     17    23  27   32   37 40    46   51
        let json = r#"[{ "abc": "str", "de": 14, "f": null }, true, false]"#.to_owned();
        let (rows, _, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..52); // Array
        assert_eq!(rows[1].range, 1..38); // Object
        assert_eq!(rows[2].key_range, Some(3..8)); // "abc": "str"
        assert_eq!(rows[2].range, 10..15); // "abc": "str"
        assert_eq!(rows[3].key_range, Some(17..21)); // "de": 14
        assert_eq!(rows[3].range, 23..25); // "de": 14
        assert_eq!(rows[4].key_range, Some(27..30)); // "f": null
        assert_eq!(rows[4].range, 32..36); // "f": null
        assert_eq!(rows[5].range, 37..38); // }
        assert_eq!(rows[6].range, 40..44); // true
        assert_eq!(rows[7].range, 46..51); // false
        assert_eq!(rows[8].range, 51..52); // ]
    }
}
