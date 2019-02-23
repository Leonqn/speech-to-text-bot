use tokio_threadpool::ThreadPool;
use futures::{Future, lazy};
use futures::sync::oneshot;
use std::io;
use std::io::Write;
use tempfile::NamedTempFile;
use std::process::Command;
use std::str;
use futures::sync::oneshot::Canceled;
use std::fmt;

pub enum MediaKind {
    Ogg(Vec<u8>),
    Mp4(Vec<u8>),
}

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    AvconvError(String),
    ConversionCanceled,
}

impl From<io::Error> for Error {
    fn from(x: io::Error) -> Self {
        Error::IoError(x)
    }
}

impl From<Canceled> for Error {
    fn from(_: Canceled) -> Self {
        Error::ConversionCanceled
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(io) =>
                write!(f, "Io error: {}", io),
            Error::AvconvError(msg) =>
                write!(f, "Codec error: {}", msg),
            Error::ConversionCanceled =>
                write!(f, "Cancelled"),
        }
    }
}


pub struct MediaConverter {
    thread_pool: ThreadPool
}

impl MediaConverter {
    pub fn new() -> MediaConverter {
        let thread_pool = ThreadPool::new();
        MediaConverter {
            thread_pool
        }
    }


    pub fn convert(&self, media_kind: MediaKind) -> impl Future<Item=Vec<u8>, Error=Error> {
        let (tx, rx) = oneshot::channel();
        self.thread_pool.spawn(lazy(move || {
            let convert_result = Self::write_media_file(media_kind)
                .and_then(Self::convert_int);
            tx.send(convert_result).map_err(|_| ())?;
            Ok(())
        }));
        rx.then(|result| {
            result?
        })
    }

    fn convert_int(file: NamedTempFile) -> Result<Vec<u8>, Error> {
        let avconv_result = Command::new("avconv")
            .args(&["-i", &file.path().to_str().expect("path should be UTF-8"), "-vn", "-acodec", "pcm_s16le", "-ac", "1", "-ar", "16000", "-f", "wav", "pipe:"])
            .output()?;

        if avconv_result.status.success() {
            Ok(avconv_result.stdout)
        } else {
            let error_string = str::from_utf8(&avconv_result.stderr).map_err(|_| {
                Error::AvconvError("Avconv error".to_string())
            })?.to_string();

            Err(Error::AvconvError(error_string))
        }
    }

    fn write_media_file(media_kind: MediaKind) -> Result<NamedTempFile, Error> {
        match media_kind {
            MediaKind::Ogg(audio) => {
                let mut file = tempfile::Builder::new().suffix(".oga")
                    .tempfile()?;
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
}