use log::error;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::path;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(io) => write!(f, "Io error: {}", io),
        }
    }
}

pub struct Storage {
    _sync_thread: JoinHandle<()>,
    map: Arc<RwLock<HashMap<i64, String>>>,
}

impl Storage {
    pub fn new<S: Into<String>>(db_path: S) -> Result<Storage, Error> {
        let db_file_name = db_path.into();
        let db = match File::open(&db_file_name) {
            Ok(mut file) => {
                let mut serialized_db = String::new();
                file.read_to_string(&mut serialized_db)?;
                Self::db_from_string(serialized_db)
            }
            Err(err) => {
                if err.kind() != ErrorKind::NotFound {
                    return Err(From::from(err));
                }
                HashMap::new()
            }
        };

        let db = Arc::new(RwLock::new(db));
        let in_thread_db = db.clone();

        let thread = thread::spawn(move || {
            fn persist_db(db_file_name: &str, serialized_db: String) -> Result<(), io::Error> {
                let dir = path::Path::new(db_file_name)
                    .parent()
                    .unwrap_or(Path::new("./"));
                let mut temp_file = tempfile::Builder::new().tempfile_in(dir)?;
                temp_file.write_all(serialized_db.as_bytes())?;
                match temp_file.persist(db_file_name) {
                    Err(err) => {
                        error!("Db persist error {:?}", err);
                    }
                    _ => (),
                }
                Ok(())
            }

            loop {
                let db = Self::db_to_string(&in_thread_db.read().unwrap());
                if let Err(e) = persist_db(&db_file_name, db) {
                    error!("An error has occurred while updating db {:?}", e)
                }
                thread::sleep(Duration::new(10, 0))
            }
        });

        Ok(Storage {
            _sync_thread: thread,
            map: db,
        })
    }

    pub fn put(&self, chat_id: i64, lang_preference: String) {
        self.map.write().unwrap().insert(chat_id, lang_preference);
    }

    pub fn get(&self, chat_id: i64) -> Option<String> {
        self.map
            .read()
            .unwrap()
            .get(&chat_id)
            .map(|x| x.to_string())
    }

    fn db_to_string(db: &HashMap<i64, String>) -> String {
        let mut res_string = String::new();
        for (k, v) in db.iter() {
            res_string.push_str(&format!("{} {}\n", k, v))
        }
        res_string
    }

    fn db_from_string(db: String) -> HashMap<i64, String> {
        let mut response = HashMap::new();
        for line in db.split('\n').filter(|&x| x != "") {
            let (chat_id, lang) = line.split_at(line.find(' ').unwrap());
            response.insert(i64::from_str(chat_id).unwrap(), lang[1..].to_string());
        }
        response
    }
}
