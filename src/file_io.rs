use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub trait SerdeStr: Serialize + for<'de> Deserialize<'de> {
    fn de_from_str(string: &str) -> Result<Self, io::Error>
    where
        Self: Sized;
    fn ser_to_string(&self) -> Result<String, io::Error>;
}

#[macro_export]
macro_rules! impl_serde_str_json {
    ($($t:ty),*) => {
        $(
            impl $crate::SerdeStr for $t {
                fn de_from_str(string: &str) -> Result<Self, std::io::Error> {
                    Ok(serde_json::from_str(string)?)
                }
                fn ser_to_string(&self) -> Result<String, std::io::Error> {
                    Ok(serde_json::to_string(self)?)
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! impl_serde_str_toml {
    ($($t:ty),*) => {
        $(
            impl $crate::SerdeStr for $t {
                fn de_from_str(string: &str) -> Result<Self, std::io::Error> {
                    Ok(toml::from_str(string).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
                }
                fn ser_to_string(&self) -> Result<String, std::io::Error> {
                    Ok(toml::to_string(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
                }
            }
        )*
    };
}

/// Easy access to the a file (configuration file, data file, etc.)
/// Provides (safe?) load and save operations
pub struct FileIO<T, S = ()> {
    _content: std::marker::PhantomData<T>,
    _serde: std::marker::PhantomData<S>,
    pub path: PathBuf,
}

impl<T> FileIO<T>
where
    T: SerdeStr,
{
    pub fn new(path: PathBuf) -> Self {
        Self {
            _content: std::marker::PhantomData,
            _serde: std::marker::PhantomData,
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
    pub fn load(&self) -> io::Result<T> {
        self.ensure_parent()?;
        let string = fs::read_to_string(&self.path.canonicalize()?)?;
        let conf = SerdeStr::de_from_str(&string)?;
        Ok(conf)
    }
    pub fn save(&self, conf: &T) -> io::Result<()> {
        self.ensure_parent()?;
        let s = SerdeStr::ser_to_string(conf)?;
        fs::write(&self.path, s)?;
        Ok(())
    }
    pub fn load_or_init(&self, init: impl Fn() -> T) -> io::Result<T> {
        match self.load() {
            Ok(conf) => Ok(conf),
            Err(_) => {
                let conf = init();
                self.save(&conf)?;
                Ok(conf)
            }
        }
    }
    pub fn backup_and_save(&self, conf: &T) -> io::Result<()> {
        self.ensure_parent()?;
        if self.path.exists() {
            // back up the old file
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
        self.save(conf)?;
        Ok(())
    }
    pub fn edit(&self) -> io::Result<()> {
        let editor = std::env::var("EDITOR")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "$EDITOR envvar not set"))?;
        let status = std::process::Command::new(editor)
            .arg(&self.path)
            .status()?;
        if !status.success() {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to edit config file and exit gracefully",
            ))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Conf {
        pub name: String,
    }
    impl SerdeStr for Conf {
        fn de_from_str(string: &str) -> Result<Self, io::Error> {
            let json = serde_json::from_str(&string)?;
            Ok(json)
        }
        fn ser_to_string(&self) -> Result<String, io::Error> {
            let s = serde_json::to_string(self)?;
            Ok(s)
        }
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
        let loaded_conf = file_io.load();
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
        file_io.save(&conf).unwrap();
        let loaded_conf = file_io.load();
        fs::remove_file(&path).unwrap();
        assert_eq!(loaded_conf.unwrap(), conf);
    }
}
