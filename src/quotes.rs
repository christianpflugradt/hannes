//! Stable quote indices: append new sayings without reordering existing lines.
use std::{
    collections::{VecDeque, hash_map::RandomState},
    hash::BuildHasher,
    sync::OnceLock,
};

pub const HISTORY_LIMIT: usize = 20;

pub fn all() -> &'static [&'static str] {
    static QUOTES: OnceLock<Vec<&'static str>> = OnceLock::new();
    QUOTES.get_or_init(|| include_str!("confused-quotes.txt").lines().collect())
}

pub(crate) fn choose(recent: &VecDeque<usize>) -> usize {
    let available: Vec<_> = (0..all().len())
        .filter(|index| !recent.contains(index))
        .collect();
    // The collection has more than 20 entries; bounded history cannot exhaust it.
    available[RandomState::new().hash_one(()) as usize % available.len()]
}
