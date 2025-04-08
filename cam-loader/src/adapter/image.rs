use std::path::Path;

use crate::{
    Result,
    loader::{Loader, OwnedWriteBuffer},
};

use super::Config;

pub fn from_spec<B: OwnedWriteBuffer + Send + 'static>(
    spec: &Config,
    path: &Path,
) -> Result<Loader<B>> {
    let img = image::open(path)?.to_rgba8();

    Ok(Loader::new_blocking(
        img.width(),
        img.height(),
        4,
        move |dest| {
            dest.copy_from_slice(&img);
        },
    ))
}
