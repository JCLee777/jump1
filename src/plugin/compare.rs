//! Shared state for the built-in structured-data comparison views.

use crate::i18n::{I18n, Message};
use gpui_kit::Pixels;
use similar::{DiffTag, TextDiff};
use std::{
    ops::Range,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub enum Comparison {
    Empty,
    Equal,
    FormattingDifferent,
    Different,
    Invalid,
}

impl Comparison {
    pub fn message(self, i18n: &I18n) -> &'static str {
        i18n.text(match self {
            Self::Empty => Message::CompareEmpty,
            Self::Equal => Message::CompareEqual,
            Self::FormattingDifferent => Message::CompareFormattingDifferent,
            Self::Different => Message::CompareDifferent,
            Self::Invalid => Message::CompareInvalid,
        })
    }
}

pub struct ChangedRanges {
    pub left: Vec<Range<usize>>,
    pub right: Vec<Range<usize>>,
    pub locations: Vec<DiffLocation>,
}

#[derive(Clone, Copy)]
pub struct DiffLocation {
    pub left: usize,
    pub right: usize,
}

/// Compare visible source lines; inserted and deleted lines are marked only
/// on the side where text exists.
pub fn changed_ranges(left: &str, right: &str) -> ChangedRanges {
    let left_offsets = line_offsets(left);
    let right_offsets = line_offsets(right);
    let diff = TextDiff::configure()
        .timeout(Duration::from_millis(30))
        .diff_lines(left, right);
    let mut ranges = ChangedRanges {
        left: Vec::new(),
        right: Vec::new(),
        locations: Vec::new(),
    };
    for op in diff.ops() {
        if op.tag() == DiffTag::Equal {
            continue;
        }
        let old = op.old_range();
        let new = op.new_range();
        ranges.locations.push(DiffLocation {
            left: left_offsets[old.start],
            right: right_offsets[new.start],
        });
        if old.start < old.end {
            ranges
                .left
                .push(left_offsets[old.start]..left_offsets[old.end]);
        }
        if new.start < new.end {
            ranges
                .right
                .push(right_offsets[new.start]..right_offsets[new.end]);
        }
    }
    ranges
}

fn line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(text.lines().count() + 1);
    offsets.push(0);
    for line in text.split_inclusive('\n') {
        offsets.push(offsets.last().copied().unwrap_or(0) + line.len());
    }
    offsets
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Default)]
pub struct ScrollSync {
    left: Option<Pixels>,
    right: Option<Pixels>,
    following: Option<(Side, Instant)>,
    ignore_next: [bool; 2],
}

impl ScrollSync {
    pub fn observe(&mut self, side: Side, offset: Pixels, enabled: bool) -> Option<Pixels> {
        let (previous, ignore_next) = match side {
            Side::Left => (&mut self.left, &mut self.ignore_next[0]),
            Side::Right => (&mut self.right, &mut self.ignore_next[1]),
        };
        let ignore = std::mem::replace(ignore_next, false);
        if previous.replace(offset) == Some(offset) || !enabled || ignore {
            return None;
        }
        if let Some((target, since)) = self.following {
            if target == side && since.elapsed() < Duration::from_millis(150) {
                self.following = None;
                return None;
            }
        }
        let other = match side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        self.following = Some((other, Instant::now()));
        Some(offset)
    }

    pub fn follow(&mut self, side: Side) {
        self.following = Some((side, Instant::now()));
    }

    pub fn keep_independent_positions(&mut self) {
        self.ignore_next = [true; 2];
    }
}
