extern crate core;

#[cfg(feature = "gresource")]
pub mod gresource;

pub mod read;
pub mod write;

#[cfg(test)]
pub(crate) mod test;

mod util;
