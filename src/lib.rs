use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub struct FileIO<T: Serialize + for<'de> Deserialize<'de>> {
    _phantom: std::marker::PhantomData<T>,
    pub path: PathBuf,
}

impl<T> FileIO<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(path: PathBuf) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            path,
        }
    }
    fn ensure_parent(&self) -> io::Result<()> {
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "The path has no parent directory",
            )
        })?;
        fs::create_dir_all(parent)?;
        Ok(())
    }
    pub fn load_str(&self, from_string: impl Fn(&str) -> Result<T, io::Error>) -> io::Result<T> {
        self.ensure_parent()?;
        let string = fs::read_to_string(&self.path.canonicalize()?)?;
        let conf = from_string(&string)?;
        Ok(conf)
    }
    pub fn save(
        &self,
        conf: &T,
        to_string: impl Fn(&T) -> Result<String, io::Error>,
    ) -> io::Result<()> {
        self.ensure_parent()?;
        // back up the old file
        if self.path.exists() {
            let mut ext = self
                .path
                .extension()
                .unwrap_or_default()
                .to_owned()
                .into_string()
                .unwrap_or_default();
            ext += ".";
            ext += &time::OffsetDateTime::now_utc().unix_timestamp().to_string();
            ext += ".bak";
            let backup_path = self.path.with_extension(ext);
            fs::rename(&self.path, &backup_path)?;
        }
        let s = to_string(conf)?;
        fs::write(&self.path, s)?;
        Ok(())
    }
    pub fn load_or_init(
        &self,
        from_string: impl Fn(&str) -> Result<T, io::Error>,
        to_string: impl Fn(&T) -> Result<String, io::Error>,
        init: impl Fn() -> T,
    ) -> io::Result<T> {
        match self.load_str(from_string) {
            Ok(conf) => Ok(conf),
            Err(_) => {
                let conf = init();
                self.save(&conf, to_string)?;
                Ok(conf)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Conf {
        pub name: String,
    }

    #[test]
    fn file_io_str() {
        let path = PathBuf::from("test_file_io_str.json");
        let conf = Conf {
            name: "test".to_string(),
        };
        let s = serde_json::to_string(&conf).unwrap();
        fs::write(&path, s).unwrap();
        let file_io = FileIO::<Conf>::new(path.clone());
        let loaded_conf = file_io.load_str(|string| {
            println!("{:?}", string);
            let json = serde_json::from_str(&string)?;
            Ok(json)
        });
        fs::remove_file(&path).unwrap();
        assert_eq!(loaded_conf.unwrap(), conf);
    }

    #[test]
    fn file_io_save() {
        let path = PathBuf::from("test_file_io_save.json");
        let conf = Conf {
            name: "test".to_string(),
        };
        let file_io = FileIO::<Conf>::new(path.clone());
        file_io
            .save(&conf, |conf| {
                let s = serde_json::to_string(conf)?;
                Ok(s)
            })
            .unwrap();
        let loaded_conf = file_io.load_str(|string| {
            let json = serde_json::from_str(&string)?;
            Ok(json)
        });
        // fs::remove_file(&path).unwrap();
        assert_eq!(loaded_conf.unwrap(), conf);
    }
}
