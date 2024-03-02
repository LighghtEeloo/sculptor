use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

/// Implement this trait to get ProjectInfo, which provides directories for the project
pub trait AppAuthor {
    fn app_name() -> &'static str;
    fn author() -> &'static str;
}

pub trait LazyProjectDirs {
    fn lazy_project_dirs() -> Lazy<ProjectDirs>;
}

impl<T: AppAuthor> LazyProjectDirs for T {
    fn lazy_project_dirs() -> Lazy<ProjectDirs> {
        Lazy::new(|| {
            ProjectDirs::from("", Self::author(), Self::app_name())
                .expect("No valid config directory fomulated")
        })
    }
}

/// Provides directories for the project
pub trait ProjectInfo: LazyProjectDirs {
    fn project_dirs() -> ProjectDirs {
        Self::lazy_project_dirs().to_owned()
    }
    fn config_dir() -> PathBuf {
        Self::lazy_project_dirs().config_dir().to_path_buf()
    }
    fn data_dir() -> PathBuf {
        Self::lazy_project_dirs().data_dir().to_path_buf()
    }
    fn cache_dir() -> PathBuf {
        Self::lazy_project_dirs().cache_dir().to_path_buf()
    }
    fn state_dir() -> Option<PathBuf> {
        Some(Self::lazy_project_dirs().state_dir()?.to_path_buf())
    }
}
impl<T: LazyProjectDirs> ProjectInfo for T {}

pub trait SerdeStr<Proof>: Serialize + for<'de> Deserialize<'de> {
    fn de_from_str(string: &str) -> Result<Self, io::Error>
    where
        Self: Sized;
    fn ser_to_string(&self) -> Result<String, io::Error>;
}

pub struct SerdeViaJson;

impl<T> SerdeStr<SerdeViaJson> for T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn de_from_str(string: &str) -> Result<Self, io::Error> {
        Ok(serde_json::from_str(string)?)
    }
    fn ser_to_string(&self) -> Result<String, io::Error> {
        Ok(serde_json::to_string(self)?)
    }
}

pub struct SerdeViaToml;
impl<T> SerdeStr<SerdeViaToml> for T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn de_from_str(string: &str) -> Result<Self, io::Error> {
        Ok(toml::from_str(string).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    }
    fn ser_to_string(&self) -> Result<String, io::Error> {
        Ok(toml::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    }
}

/// Easy access to the a file (configuration file, data file, etc.)
/// Provides (safe?) load and save operations
pub struct FileIO<T, S = ()> {
    _content: std::marker::PhantomData<T>,
    _serde: std::marker::PhantomData<S>,
    pub path: PathBuf,
}

impl<T, S> FileIO<T, S>
where
    T: SerdeStr<S>,
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
    pub fn load_str(&self) -> io::Result<T> {
        self.ensure_parent()?;
        let string = fs::read_to_string(&self.path.canonicalize()?)?;
        let conf = SerdeStr::de_from_str(&string)?;
        Ok(conf)
    }
    pub fn save(&self, conf: &T) -> io::Result<()> {
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
        let s = SerdeStr::ser_to_string(conf)?;
        fs::write(&self.path, s)?;
        Ok(())
    }
    pub fn load_or_init(&self, init: impl Fn() -> T) -> io::Result<T> {
        match self.load_str() {
            Ok(conf) => Ok(conf),
            Err(_) => {
                let conf = init();
                self.save(&conf)?;
                Ok(conf)
            }
        }
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
    impl SerdeStr<()> for Conf {
        fn de_from_str(string: &str) -> Result<Self, io::Error> {
            let json = serde_json::from_str(&string)?;
            Ok(json)
        }
        fn ser_to_string(&self) -> Result<String, io::Error> {
            let s = serde_json::to_string(self)?;
            Ok(s)
        }
    }
    // impl SerdeStr<SerdeViaToml> for Conf {}

    #[test]
    fn file_io_str() {
        let path = PathBuf::from("test_file_io_str.json");
        let conf = Conf {
            name: "test".to_string(),
        };
        let s = serde_json::to_string(&conf).unwrap();
        fs::write(&path, s).unwrap();
        let file_io = FileIO::<Conf>::new(path.clone());
        let loaded_conf = file_io.load_str();
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
        let loaded_conf = file_io.load_str();
        fs::remove_file(&path).unwrap();
        assert_eq!(loaded_conf.unwrap(), conf);
    }
}
