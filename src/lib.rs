// we use a lot of manual indices instead of `take` and `skip`, so allow that
#![allow(clippy::needless_range_loop)]

pub mod blocks;
pub mod cache;
pub mod canvas;
pub mod cliffs;
pub mod config;
pub mod contours;
pub mod crop;
pub mod knolls;
pub mod merge;
pub mod process;
pub mod render;
pub mod util;
pub mod vegetation;
