use directories::ProjectDirs;
use once_cell::sync::Lazy;
use std::path::PathBuf;

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
