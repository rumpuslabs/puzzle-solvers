use anyhow::{anyhow, Result};
use itertools::Itertools;
use rayon::prelude::*;
use std::io::{self, Write};
use std::sync::{Mutex, RwLock};

mod rows;
use rows::*;

fn main() -> Result<()> {
    print!("Enter Holes, Colours: ");
    io::stdout().flush()?;

    let mut numbers = String::new();
    io::stdin().read_line(&mut numbers)?;
    let numbers: Vec<u8> = numbers
        .trim()
        .split(&[' ', 'x', ','])
        .map(str::parse)
        .collect::<Result<_, _>>()?;

    let (&num_holes, &num_colours) = if let Some((h, c)) = numbers.iter().collect_tuple() {
        (h, c)
    } else {
        return Err(anyhow!("Please provide exactly 2 numbers"));
    };

    if num_holes < 1 {
        return Err(anyhow!("Holes must be at least 1"));
    }

    if num_colours < 2 {
        return Err(anyhow!("Colours must be at least 2"));
    }

    let def = RowDef::new(num_holes, num_colours);
    let empty_set = RowSet::new(def)?;

    let mut all_guesses = empty_set.clone();
    all_guesses.insert_all();
    let all_guesses = all_guesses; // remove 'mut'

    let possible_scores = (0..num_holes)
        .flat_map(|num_black| {
            (0..(num_holes - num_black)).map(move |num_white| Score::new(num_black, num_white))
        })
        .collect_vec();

    let mut candidates = all_guesses.clone();

    while candidates.count() > 1 {
        println!("{} candidates left", candidates.count());

        let mut scores = Vec::new();
        scores.resize_with(def.capacity()?, || {
            RwLock::new(vec![0usize; def.score_count()])
        });

        let total = candidates.count();
        let step = total / 1000;
        let progress = Mutex::new(0);
        candidates.par_iter().for_each(|answer| {
            for score in &possible_scores {
                for guess in &def.compatible_with(&answer, *score) {
                    scores[guess.index()].write().unwrap()[score.index(def)] += 1;
                }
            }
            let mut progress = progress.lock().unwrap();
            *progress += 1;
            if (step == 0) || (*progress % step) == 0 {
                print!("\r{:.1}%", (*progress as f32) * 100.0 / (total as f32));
                io::stdout().flush().unwrap();
            }
        });

        let guess_scores: Vec<_> = all_guesses
            .par_iter()
            .filter_map(|guess| {
                let (sum, count) = scores[guess.index()]
                    .read()
                    .unwrap()
                    .iter()
                    .filter(|&&v| v > 0)
                    .fold((0, 0), |(sum, count), v| (sum + v, count + 1));
                if count > 0 {
                    Some((guess, sum * 100 / count))
                } else {
                    None
                }
            })
            .collect();

        if let Some((_, score)) = guess_scores
            .iter()
            .min_by_key(|(guess, v)| (*v, !candidates.contains(guess), guess.index()))
        {
            let score = score.clone();
            let (guess_scores_in, guess_scores_out): (Vec<_>, Vec<_>) = guess_scores
                .into_iter()
                .filter(|(_, v)| *v == score)
                .partition(|(guess, _)| candidates.contains(guess));

            let (mut guess_scores, in_candidates) = if guess_scores_in.len() > 0 {
                (guess_scores_in, true)
            } else {
                (guess_scores_out, false)
            };

            guess_scores.sort_unstable();
            println!(
                "\rRecommended guesses ({} with score = {}{}):",
                guess_scores.len(),
                (score as f32) / 100.0,
                if in_candidates {
                    ""
                } else {
                    ", not in candidates"
                }
            );
            for (guess, _) in guess_scores.into_iter().take(10) {
                println!("  {guess}");
            }
        } else {
            println!();
            return Err(anyhow!("No best guess found"));
        }

        print!("Enter guess: ");
        io::stdout().flush()?;

        let mut numbers = String::new();
        io::stdin().read_line(&mut numbers)?;
        let numbers: Vec<u8> = numbers
            .trim()
            .split(&[' ', ','])
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        if numbers.len() != (num_holes as usize) {
            return Err(anyhow!("Guess has incorrect number of holes"));
        }

        let guess = def.row(&numbers);

        print!("Enter score (black, white): ");
        io::stdout().flush()?;

        let mut numbers = String::new();
        io::stdin().read_line(&mut numbers)?;
        let numbers: Vec<u8> = numbers
            .trim()
            .split(&[' ', ','])
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        if numbers.len() != 2 {
            return Err(anyhow!("Score must have 2 parts"));
        }

        let score = Score::new(numbers[0], numbers[1]);

        candidates = candidates.compatible_with(&guess, score);
    }

    match candidates.iter().next() {
        Some(solution) => println!("Solution: {}", solution),
        None => println!("No valid solutions left!"),
    }

    Ok(())
}
