use super::*;

#[test]
fn test_insert_all() {
    let mut set = RowSet::new(RowDef::new(4, 9)).unwrap();
    set.insert_all();
    assert_eq!(set.count(), set.def.combos());

    let mut set = RowSet::new(RowDef::new(8, 4)).unwrap();
    set.insert_all();
    assert_eq!(set.count(), set.def.combos());

    let mut set = RowSet::new(RowDef::new(7, 11)).unwrap();
    set.insert_all();
    assert_eq!(set.count(), set.def.combos());

    let mut set = RowSet::new(RowDef::new(6, 16)).unwrap();
    set.insert_all();
    assert_eq!(set.count(), set.def.combos());

    let mut set = RowSet::new(RowDef::new(16, 2)).unwrap();
    set.insert_all();
    assert_eq!(set.count(), set.def.combos());
}

#[test]
fn test_row_new() {
    let def = RowDef::new(4, 16);
    let row = def.row(&[1, 1, 2, 3]);
    assert_eq!(row.index(), 0x1123);

    let def = RowDef::new(5, 9);
    let row = def.row(&[1, 0, 1, 8, 5]);
    assert_eq!(row.index(), 0x10185);

    let def = RowDef::new(16, 3);
    let row = def.row(&[1, 0, 1, 2, 1, 0, 2, 2, 2, 1, 0, 2, 1, 2, 2, 0]);
    assert_eq!(row.index(), 0b01000110010010101001001001101000);
}

#[test]
fn test_score() {
    let def = RowDef::new(4, 8);
    let answer = def.row(&[1, 1, 2, 3]);

    let guess = def.row(&[1, 1, 2, 3]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 4,
            num_white: 0
        }
    );

    let guess = def.row(&[1, 1, 4, 3]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 3,
            num_white: 0
        }
    );

    let guess = def.row(&[1, 0, 4, 3]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 2,
            num_white: 0
        }
    );

    let guess = def.row(&[7, 0, 4, 3]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 1,
            num_white: 0
        }
    );

    let guess = def.row(&[7, 0, 4, 7]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 0,
            num_white: 0
        }
    );

    let guess = def.row(&[1, 1, 3, 4]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 2,
            num_white: 1
        }
    );

    let guess = def.row(&[1, 1, 4, 2]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 2,
            num_white: 1
        }
    );

    let guess = def.row(&[1, 1, 1, 2]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 2,
            num_white: 1
        }
    );

    let guess = def.row(&[3, 1, 2, 1]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 2,
            num_white: 2
        }
    );

    let guess = def.row(&[3, 1, 1, 2]);
    assert_eq!(
        answer.score(&guess),
        Score {
            num_black: 1,
            num_white: 3
        }
    );
}

fn compatible_with(def: RowDef, guess: &FatRow, score: Score) -> RowSet {
    RowSet {
        def,
        set: AllPossibleRowsIter::new(def)
            .filter(|candidate| {
                FatRow {
                    def,
                    row: *candidate,
                }
                .compatible_with(guess, score)
            })
            .fold(
                BitSet::with_capacity(def.capacity().unwrap()),
                |mut set, row| {
                    set.insert(row.0);
                    set
                },
            ),
    }
}

#[test]
fn test_compatible_with_3() -> Result<()> {
    let def = RowDef::new(3, 5);

    let guess = def.row(&[3, 1, 3]);

    let score = Score {
        num_black: 1,
        num_white: 1,
    };
    let mut set = RowSet::new(def)?;
    set.insert_all();
    assert_eq!(
        set.compatible_with(&guess, score),
        compatible_with(def, &guess, score)
    );

    let score = Score {
        num_black: 1,
        num_white: 2,
    };
    let mut set = RowSet::new(def)?;
    set.insert_all();
    assert_eq!(
        set.compatible_with(&guess, score),
        compatible_with(def, &guess, score)
    );

    let score = Score {
        num_black: 0,
        num_white: 0,
    };
    let mut set = RowSet::new(def)?;
    set.insert_all();
    assert_eq!(
        set.compatible_with(&guess, score),
        compatible_with(def, &guess, score)
    );

    Ok(())
}

#[test]
fn test_compatible_with_4() -> Result<()> {
    let def = RowDef::new(4, 8);

    let guess = def.row(&[3, 1, 2, 1]);

    let score = Score {
        num_black: 1,
        num_white: 1,
    };
    let mut set = RowSet::new(def)?;
    set.insert_all();
    assert_eq!(
        compatible_with(def, &guess, score)
            .set
            .difference(&set.compatible_with(&guess, score).set)
            .iter()
            .fold(RowSet::new(def)?, |mut s, e| {
                s.insert(&FatRow { def, row: Row(e) });
                s
            }),
        RowSet::new(def)?
    );

    Ok(())
}
