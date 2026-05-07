use crate::{paths, url_provider::NativeUrlSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    path::Path,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeIncomingMessage {
    url: Option<String>,
    title: Option<String>,
    timestamp: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeResponse {
    ok: bool,
    accepted_url: Option<String>,
    error: Option<String>,
}

pub fn run_stdio_host() -> io::Result<()> {
    let config_dir = paths::fallback_config_dir();
    let url_path = paths::native_url_path_from_config_dir(config_dir);
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    match read_native_message(&mut stdin) {
        Ok(Some(bytes)) => {
            let response = handle_message_bytes(&url_path, &bytes);
            write_native_message(&mut stdout, &response)?;
        }
        Ok(None) => {}
        Err(err) => {
            let response = NativeResponse {
                ok: false,
                accepted_url: None,
                error: Some(err.to_string()),
            };
            write_native_message(&mut stdout, &response)?;
        }
    }

    Ok(())
}

fn handle_message_bytes(path: &Path, bytes: &[u8]) -> NativeResponse {
    match parse_incoming_url(bytes) {
        Ok(url) => match NativeUrlSnapshot::write(path, url.clone()) {
            Ok(()) => NativeResponse {
                ok: true,
                accepted_url: url,
                error: None,
            },
            Err(err) => NativeResponse {
                ok: false,
                accepted_url: None,
                error: Some(err.to_string()),
            },
        },
        Err(err) => NativeResponse {
            ok: false,
            accepted_url: None,
            error: Some(err),
        },
    }
}

fn parse_incoming_url(bytes: &[u8]) -> Result<Option<String>, String> {
    let message: NativeIncomingMessage =
        serde_json::from_slice(bytes).map_err(|err| err.to_string())?;
    let _ = (&message.title, message.timestamp);
    Ok(message.url.and_then(|url| {
        let trimmed = url.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }))
}

fn read_native_message<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }

    let length = u32::from_le_bytes(length) as usize;
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}

fn write_native_message<W: Write, T: Serialize>(writer: &mut W, message: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(message)?;
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_native_message_url() {
        let url =
            parse_incoming_url(br#"{"url":" https://www.bilibili.com/video/BV1abc ","title":"x"}"#)
                .expect("valid native message");

        assert_eq!(url, Some("https://www.bilibili.com/video/BV1abc".into()));
    }

    #[test]
    fn reads_length_prefixed_native_message() {
        let payload = br#"{"url":"https://github.com"}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);

        let result = read_native_message(&mut bytes.as_slice()).expect("message read");

        assert_eq!(result, Some(payload.to_vec()));
    }
}
