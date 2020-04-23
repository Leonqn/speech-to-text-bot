use std::fmt;
use std::io;
use std::io::Write;
use std::process::Command;
use std::str;
use tempfile::NamedTempFile;
use anyhow::anyhow;

pub enum MediaKind {
    Ogg(Vec<u8>),
    Mp4(Vec<u8>),
}


pub fn convert(media_kind: MediaKind) -> anyhow::Result<Vec<u8>> {
    write_media_file(media_kind)
        .and_then(convert_int)
}

fn convert_int(file: NamedTempFile) -> anyhow::Result<Vec<u8>> {
    let avconv_result = Command::new("avconv")
        .args(&[
            "-i",
            &file.path().to_str().expect("path should be UTF-8"),
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "wav",
            "pipe:",
        ])
        .output()?;

    if avconv_result.status.success() {
        Ok(avconv_result.stdout)
    } else {
        let error_string = str::from_utf8(&avconv_result.stderr)?;
        Err(anyhow!("Avconv error: {}", error_string))
    }
}

fn write_media_file(media_kind: MediaKind) -> anyhow::Result<NamedTempFile> {
    match media_kind {
        MediaKind::Ogg(audio) => {
            let mut file = tempfile::Builder::new().suffix(".oga").tempfile()?;
            file.write_all(&audio)?;
            Ok(file)
        }
        MediaKind::Mp4(video) => {
            let mut file = tempfile::Builder::new().suffix("mp4").tempfile()?;
            file.write_all(&video)?;
            Ok(file)
        }
    }
}
