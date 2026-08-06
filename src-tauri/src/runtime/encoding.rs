//! Text decoding helpers for network streams and Windows process output.

/// Append a network chunk to `pending`, decoding only complete UTF-8 sequences into `out`.
/// Incomplete trailing bytes stay in `pending` until the next chunk arrives.
///
/// Avoids `String::from_utf8_lossy` on each chunk, which turns split multi-byte
/// characters (common for CJK) into U+FFFD replacement characters (`���`).
pub fn append_utf8_chunk(pending: &mut Vec<u8>, chunk: &[u8], out: &mut String) {
    if chunk.is_empty() && pending.is_empty() {
        return;
    }
    pending.extend_from_slice(chunk);
    loop {
        match std::str::from_utf8(pending) {
            Ok(text) => {
                out.push_str(text);
                pending.clear();
                break;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                if valid_up_to > 0 {
                    let valid = std::str::from_utf8(&pending[..valid_up_to])
                        .expect("valid_up_to marks a UTF-8 prefix");
                    out.push_str(valid);
                    pending.drain(..valid_up_to);
                    continue;
                }
                match error.error_len() {
                    Some(len) => {
                        out.push(char::REPLACEMENT_CHARACTER);
                        let drain = len.min(pending.len());
                        pending.drain(..drain);
                    }
                    None => break,
                }
            }
        }
    }
}

/// Decode process stdout/stderr bytes.
/// Prefer strict UTF-8; on Windows console output fall back to GB18030 / GBK.
pub fn decode_process_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    #[cfg(windows)]
    {
        for encoding in [encoding_rs::GB18030, encoding_rs::GBK] {
            let (cow, _, had_errors) = encoding.decode(bytes);
            if !had_errors {
                return cow.into_owned();
            }
        }
        let (cow, _, _) = encoding_rs::GB18030.decode(bytes);
        return cow.into_owned();
    }

    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_streaming_keeps_split_cjk_intact() {
        let text = "中文测试";
        let bytes = text.as_bytes();
        assert!(bytes.len() > 3);

        let mut pending = Vec::new();
        let mut out = String::new();
        append_utf8_chunk(&mut pending, &bytes[..2], &mut out);
        assert!(
            out.is_empty(),
            "incomplete lead bytes must not emit replacement"
        );
        assert!(!pending.is_empty());

        append_utf8_chunk(&mut pending, &bytes[2..], &mut out);
        assert_eq!(out, text);
        assert!(pending.is_empty());
    }

    #[test]
    fn utf8_streaming_handles_byte_at_a_time() {
        let text = "你好世界";
        let mut pending = Vec::new();
        let mut out = String::new();
        for byte in text.as_bytes() {
            append_utf8_chunk(&mut pending, &[*byte], &mut out);
        }
        assert_eq!(out, text);
        assert!(pending.is_empty());
    }

    #[test]
    fn decode_process_bytes_prefers_utf8() {
        assert_eq!(decode_process_bytes("hello 中文".as_bytes()), "hello 中文");
    }

    #[cfg(windows)]
    #[test]
    fn decode_process_bytes_accepts_gbk() {
        // "中文" in GBK
        let gbk = [0xD6, 0xD0, 0xCE, 0xC4];
        assert_eq!(decode_process_bytes(&gbk), "中文");
    }
}
