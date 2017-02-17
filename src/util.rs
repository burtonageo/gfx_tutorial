use find_folder;
use std::error::Error;
use std::fmt;
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
