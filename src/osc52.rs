use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::io::{self, Write};

pub fn copy_to_clipboard<W: Write>(writer: &mut W, content: &str) -> io::Result<()> {
    write!(
        writer,
        "\x1b]52;c;{}\x07",
        STANDARD.encode(content.as_bytes())
    )?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::copy_to_clipboard;

    #[test]
    fn writes_osc52_clipboard_sequence() {
        let mut output = Vec::new();

        copy_to_clipboard(&mut output, "hello").unwrap();

        assert_eq!(output, b"\x1b]52;c;aGVsbG8=\x07");
    }
}
