use find_folder;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub fn get_assets_folder() -> Result<&'static Path, GetAssetsFolderError> {
    lazy_static! {
        static ref ASSETS_FOLDER: Result<PathBuf, Box<Error + Send + Sync>> = {
            let mut bin_folder = ::std::env::current_exe()?;
            bin_folder.pop();
            let assets_folder = find_folder::Search::ParentsThenKids(2, 2)
                                    .of(bin_folder)
                                    .for_folder("data")?;
            println!("assets_folder: {:?}", assets_folder);
            Ok(assets_folder)
        };
    }

    match *ASSETS_FOLDER {
        Ok(ref path) => Ok(path.as_path()),
        Err(ref e) => Err(GetAssetsFolderError(e.as_ref())),
    }
}

#[derive(Clone, Debug)]
pub struct GetAssetsFolderError(&'static (Error + Send + Sync + 'static));

impl fmt::Display for GetAssetsFolderError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0, fmtr)
    }
}

impl Error for GetAssetsFolderError {
    #[inline]
    fn description(&self) -> &str {
        self.0.description()
    }

    #[inline]
    fn cause(&self) -> Option<&Error> {
        Some(self.0)
    }
}

pub fn open_file_relative_to_assets<P: AsRef<Path>>(
    relative_path: P,
) -> Result<File, OpenAssetsFileError> {
    let mut path = get_assets_folder()?.to_path_buf();
    path.push(relative_path.as_ref());
    Ok(File::open(&path)?)
}

#[derive(Debug)]
pub enum OpenAssetsFileError {
    Io(io::Error),
    GetAssetsFolder(GetAssetsFolderError),
}

impl From<io::Error> for OpenAssetsFileError {
    #[inline]
    fn from(e: io::Error) -> Self {
        OpenAssetsFileError::Io(e)
    }
}

impl From<GetAssetsFolderError> for OpenAssetsFileError {
    #[inline]
    fn from(e: GetAssetsFolderError) -> Self {
        OpenAssetsFileError::GetAssetsFolder(e)
    }
}

impl fmt::Display for OpenAssetsFileError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OpenAssetsFileError::Io(ref e) => write!(fmtr, "{}: {}", self.description(), e),
            OpenAssetsFileError::GetAssetsFolder(ref e) => {
                write!(fmtr, "{}: {}", self.description(), e)
            }
        }
    }
}

impl Error for OpenAssetsFileError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            OpenAssetsFileError::Io(_) => "an IO error occurred",
            OpenAssetsFileError::GetAssetsFolder(_) => {
                "an error occurred while finding the assets folder"
            }
        }
    }

    #[inline]
    fn cause(&self) -> Option<&Error> {
        match *self {
            OpenAssetsFileError::Io(ref e) => Some(e),
            OpenAssetsFileError::GetAssetsFolder(ref e) => Some(e),
        }
    }
}
