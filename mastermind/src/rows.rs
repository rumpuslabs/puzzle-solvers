use anyhow::{Context, Result};
use bittyset::BitSet;
use itertools::{repeat_n, Either, Itertools};
use rayon::prelude::*;
use std::fmt;
use std::rc::Rc;

#[cfg(test)]
mod test;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row(usize);

#[derive(Clone, Debug)]
struct RowIter {
    def: RowDef,
    remaining: Row,
}

impl Iterator for RowIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.def.num_holes > 0 {
            let next = self.remaining.0 >> (self.def.shift * (self.def.num_holes - 1));
            self.def.num_holes -= 1;
            if self.def.num_holes > 0 {
                self.remaining.0 &= (1 << (self.def.shift * self.def.num_holes)) - 1;
            }
            Some(next.try_into().expect("colour should fit in a u8"))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    num_black: u8,
    num_white: u8,
}

impl Score {
    pub fn new(num_black: u8, num_white: u8) -> Self {
        Self {
            num_black,
            num_white,
        }
    }

    pub fn index(&self, def: RowDef) -> usize {
        if self.num_black == def.num_holes {
            def.score_count() - 1
        } else {
            let num_black = self.num_black as usize;
            let num_white = self.num_white as usize;
            num_black * (2 * def.num_holes as usize + 3 - num_black) / 2 + num_white
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ScoreType {
    Black,
    White(u8),
    Neither,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PossIterState {
    Done,
    Once,
    Many(u8),
}

#[derive(Clone)]
struct Possibilities {
    s: ScoreType,
    v: u8,
    whites: Rc<Vec<u8>>,
    colours: Rc<Vec<u8>>,
    next: PossIterState,
}

impl Possibilities {
    fn new(s: ScoreType, v: u8, whites: Rc<Vec<u8>>, colours: Rc<Vec<u8>>) -> Self {
        use PossIterState::*;
        use ScoreType::*;

        Self {
            s,
            v,
            next: if let Neither = s {
                if !colours.is_empty() {
                    Many(0)
                } else {
                    Done
                }
            } else {
                Once
            },
            whites,
            colours,
        }
    }
}

impl Iterator for Possibilities {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        use PossIterState::*;
        use ScoreType::*;

        if let Done = self.next {
            return None;
        }

        match self.s {
            Black => {
                debug_assert_eq!(self.next, Once);
                self.next = Done;
                Some(self.v)
            }

            White(i) => {
                debug_assert_eq!(self.next, Once);
                let i = i as usize;
                self.next = Done;
                if self.whites[i] != self.v {
                    Some(self.whites[i])
                } else {
                    None
                }
            }

            Neither => {
                if let Many(i) = self.next {
                    let mut i = i as usize;
                    if self.colours[i] == self.v {
                        i += 1;
                        if i == self.colours.len() {
                            return None;
                        }
                    }
                    self.next = if i < self.colours.len() - 1 {
                        Many(i as u8 + 1)
                    } else {
                        Done
                    };
                    Some(self.colours[i])
                } else {
                    unreachable!()
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowDef {
    num_holes: u8,
    num_colours: u8,
    base: u8,
    shift: u8,
}

impl RowDef {
    pub fn new(num_holes: u8, num_colours: u8) -> Self {
        let base = num_colours.next_power_of_two();

        Self {
            num_holes,
            num_colours,
            base,
            shift: base.ilog2() as u8,
        }
    }

    pub fn lim(&self) -> usize {
        1usize << (self.shift * self.num_holes)
    }

    pub fn score(&self, a: Row, b: Row) -> Score {
        let mut score = Score {
            num_black: 0,
            num_white: 0,
        };
        let mut spares_a: Vec<u8> = vec![0; self.num_colours.into()];
        let mut spares_b: Vec<u8> = vec![0; self.num_colours.into()];

        for (ae, be) in self.row_iter(a).zip(self.row_iter(b)) {
            if ae == be {
                score.num_black += 1;
            } else {
                if spares_a[be as usize] > 0 {
                    score.num_white += 1;
                    spares_a[be as usize] -= 1;
                } else {
                    spares_b[be as usize] += 1;
                }

                if spares_b[ae as usize] > 0 {
                    score.num_white += 1;
                    spares_b[ae as usize] -= 1;
                } else {
                    spares_a[ae as usize] += 1;
                }
            }
        }

        score
    }

    pub fn score_count(&self) -> usize {
        let num_holes = self.num_holes as usize;
        (num_holes + 2) * (num_holes + 1) / 2 - 1
    }

    fn row_iter(&self, row: Row) -> RowIter {
        RowIter {
            def: *self,
            remaining: row,
        }
    }

    pub fn capacity(&self) -> Result<usize> {
        let base = self
            .num_colours
            .checked_next_power_of_two()
            .context("Too many colours")?;

        usize::checked_pow(
            base.try_into().expect("usize should be > u8"),
            self.num_holes.try_into().expect("usize should be >= u32"),
        )
        .context("Too many possibilities")
    }

    #[cfg(test)]
    fn combos(&self) -> usize {
        usize::pow(self.num_colours as usize, self.num_holes as u32)
    }

    pub fn row(&self, pegs: &[u8]) -> FatRow {
        assert!(pegs.len() as u8 == self.num_holes);

        FatRow {
            def: *self,
            row: Row(pegs.iter().fold(0usize, |a, e| {
                assert!(*e < self.num_colours);
                a << self.shift | *e as usize
            })),
        }
    }

    pub fn compatible_with(&self, guess: &FatRow, score: Score) -> RowSet {
        use ScoreType::*;

        let def = self;
        let num_holes = def.num_holes as usize;
        let num_colours = def.num_colours;
        let num_black = score.num_black as usize;
        let num_white = score.num_white as usize;
        let num_neither = num_holes - num_black - num_white;

        let set = repeat_n(Black, num_black)
            .chain((0..score.num_white).map(White))
            .chain(repeat_n(Neither, num_neither))
            .permutations(num_holes)
            .sorted_unstable()
            .dedup()
            .flat_map(|s_vec| {
                let s_vec = s_vec.into_iter().zip(def.row_iter(guess.row));

                let white_candidates = s_vec
                    .clone()
                    .filter_map(|(s, v)| if let Black = s { None } else { Some(v) })
                    .collect_vec();

                (0..white_candidates.len())
                    .combinations(num_white)
                    .flat_map(move |white_indexes| {
                        let mut white_indexes = white_indexes.into_iter().peekable();

                        let (whites, unused_whites): (Vec<_>, Vec<_>) =
                            white_candidates.iter().enumerate().partition_map(|(i, c)| {
                                if white_indexes.peek() == Some(&i) {
                                    white_indexes.next();
                                    Either::Left(c)
                                } else {
                                    Either::Right(c)
                                }
                            });

                        let colours = Rc::new(
                            (0..num_colours)
                                .filter(|c| !unused_whites.contains(c))
                                .collect_vec(),
                        );

                        let whites = Rc::new(whites);

                        s_vec
                            .clone()
                            .map(|(s, v)| Possibilities::new(s, v, whites.clone(), colours.clone()))
                            .multi_cartesian_product()
                    })
            })
            .map(|row| def.row(&row))
            .fold(
                BitSet::with_capacity(self.capacity().unwrap()),
                |mut set, row| {
                    set.insert(row.index());
                    set
                },
            );

        RowSet { def: *self, set }
    }
}

#[derive(Clone, Debug)]
struct AllPossibleRowsIter {
    def: RowDef,
    next: Row,
}

impl AllPossibleRowsIter {
    fn new(def: RowDef) -> Self {
        Self { def, next: Row(0) }
    }

    fn incr(def: RowDef, index: usize) -> usize {
        if def.base == def.num_colours
            || ((index & (def.base as usize - 1)) as u8) < def.num_colours - 1
        {
            index + 1
        } else {
            Self::incr(def, index >> def.shift) << def.shift
        }
    }
}

impl Iterator for AllPossibleRowsIter {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.0 < self.def.lim() {
            let next = self.next;
            self.next.0 = Self::incr(self.def, self.next.0);
            Some(next)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FatRow {
    row: Row,
    def: RowDef,
}

impl FatRow {
    pub fn index(&self) -> usize {
        self.row.0
    }

    pub fn score(&self, other: &FatRow) -> Score {
        debug_assert_eq!(self.def, other.def);
        self.def.score(self.row, other.row)
    }

    pub fn compatible_with(&self, other: &FatRow, score: Score) -> bool {
        debug_assert_eq!(self.def, other.def);
        self.def.score(self.row, other.row) == score
    }
}

impl fmt::Display for FatRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut first = true;

        for e in self.def.row_iter(self.row) {
            if !first {
                write!(f, " ")?;
            } else {
                first = false;
            }
            write!(f, "{}", e)?;
        }

        Ok(())
    }
}

impl fmt::Debug for FatRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "Row {{ ")?;
        fmt::Display::fmt(self, f)?;
        write!(f, " }}")
    }
}

pub struct RowSetIter<'a> {
    iter: bittyset::Iter<'a, usize>,
    def: RowDef,
}

impl<'a> Iterator for RowSetIter<'a> {
    type Item = FatRow;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|index| FatRow {
            row: Row(index),
            def: self.def,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RowSet {
    def: RowDef,
    set: BitSet,
}

impl RowSet {
    pub fn new(def: RowDef) -> Result<Self> {
        Ok(Self {
            def,
            set: BitSet::with_capacity(def.capacity()?),
        })
    }

    pub fn insert_all(&mut self) {
        let possible_rows = AllPossibleRowsIter::new(self.def);
        for i in possible_rows {
            self.set.insert(i.0);
        }
    }

    pub fn insert(&mut self, row: &FatRow) {
        self.set.insert(row.index());
    }

    pub fn count(&self) -> usize {
        self.set.len()
    }

    pub fn iter(&self) -> RowSetIter<'_> {
        RowSetIter {
            def: self.def,
            iter: self.set.iter(),
        }
    }

    pub fn compatible_with(&self, guess: &FatRow, score: Score) -> Self {
        let mut set = self.def.compatible_with(guess, score).set;
        set.intersect_with(&self.set);
        Self { def: self.def, set }
    }

    pub fn intersection(&self, other: &BitSet) -> BitSet {
        self.set.intersection(other)
    }

    pub fn contains(&self, guess: &FatRow) -> bool {
        self.set.contains(guess.index())
    }
}

impl fmt::Debug for RowSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for &'a RowSet {
    type Item = FatRow;
    type IntoIter = RowSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoParallelIterator for &'a RowSet {
    type Item = FatRow;
    type Iter = rayon::iter::IterBridge<RowSetIter<'a>>;

    fn into_par_iter(self) -> Self::Iter {
        self.iter().par_bridge()
    }
}
