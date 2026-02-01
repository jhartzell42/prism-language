//! This is the Prism runtime. This is an FRP runtime designed
//! for the Prism programming language, or for other components
//! of the Prism application development system. It's modelled
//! on [Reflex FRP](https://reflex-frp.org/).

#![warn(missing_docs)]

pub mod behavior;
pub mod dynamic;
pub mod event;
pub mod runtime;
pub mod widget;

#[cfg(test)]
mod test;
