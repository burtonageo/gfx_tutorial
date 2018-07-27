#![allow(dead_code)]

use std::error::Error;
use std::fmt;

pub trait Asset: Sized {
    type LoadParams;
    type LoadError: Error;

    fn load(params: &Self::LoadParams) -> Result<Self, Self::LoadError>;
}

#[derive(Debug)]
pub struct LazyLoad<T> {
    asset: Option<T>,
}

impl<T: Asset> LazyLoad<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(&mut self, params: &T::LoadParams) -> Result<(), T::LoadError> {
        if self.asset.is_none() {
            self.asset = Some(T::load(params)?);
        }
        Ok(())
    }

    pub fn unload(&mut self) {
        self.asset = None;
    }

    pub fn into_loaded(self) -> Result<T, Self> {
        match self.asset {
            Some(asset) => Ok(asset),
            None => Err(self),
        }
    }

    pub fn get(&self) -> Result<&T, NotLoadedError> {
        self.asset.as_ref().ok_or(NotLoadedError(()))
    }

    pub fn get_mut(&mut self) -> Result<&mut T, NotLoadedError> {
        self.asset.as_mut().ok_or(NotLoadedError(()))
    }
}

impl<T: Asset> Default for LazyLoad<T> {
    fn default() -> Self {
        LazyLoad {
            asset: None,
        }
    }
}

#[derive(Debug)]
pub struct NotLoadedError(());

impl fmt::Display for NotLoadedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.description())
    }
}

impl Error for NotLoadedError {
    fn description(&self) -> &str {
        "Lazy asset not loaded"
    }
}
