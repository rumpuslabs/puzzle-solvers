mod words;

use rayon::prelude::*;
use std::env;
use std::fmt;
use std::io::{self, Write};
use std::ops::Deref;
use words::*;

fn main() {
    let mut args = env::args();
    args.next();
    let (wordlist, num_games) = match args.next().as_deref() {
        Some("-l") => (WordList::Lewdle, 1),
        Some("-m") => (WordList::WordMaster, 1),
        Some("-d") => (WordList::Wordle, 2),
        Some("-q") => (WordList::Wordle, 4),
        Some("-o") => (WordList::Wordle, 8),
        Some("-s") => (WordList::Wordle, 16),
        _ => (WordList::Wordle, 1),
    };

    let (all_words, initial_words) = words(wordlist);
    let mut past_guesses = vec![];
    //let mut suggestion = find_suggestion(&all_words, &[initial_words.clone()].into();
    let mut suggestion = match wordlist {
        WordList::Wordle => WordRef(b"roate").into(),
        WordList::WordMaster => WordRef(b"serai").into(),
        WordList::Lewdle => WordRef(b"loins").into(),
    };
    println!("{} words. Suggestion: {}", initial_words.len(), suggestion);

    let mut remaining_words_lists = vec![];
    for _ in 0..num_games {
        remaining_words_lists.push(initial_words.clone());
    }

    loop {
        let guess = loop {
            print!("Enter guess: ");
            io::stdout().flush().unwrap();

            let mut guess = String::new();

            io::stdin().read_line(&mut guess).unwrap();
            let guess: Vec<u8> = guess.trim().bytes().collect();

            if guess.len() == 5 && guess.iter().all(|c| (b'a'..=b'z').contains(c)) {
                break Word(guess.try_into().unwrap());
            } else if guess.is_empty() {
                break suggestion;
            } else {
                continue;
            }
        };

        past_guesses.push(guess);

        for (i, remaining_words) in remaining_words_lists.iter_mut().enumerate() {
            if remaining_words.len() == 1 {
                continue;
            }

            if num_games > 1 {
                println!("Game {}:", i + 1);
            }

            let pattern = loop {
                print!("Enter pattern (gy.): ");
                io::stdout().flush().unwrap();

                let mut pattern = String::new();

                io::stdin().read_line(&mut pattern).unwrap();
                let pattern: Vec<u8> = pattern.trim().bytes().collect();

                if pattern.len() == 5
                    && pattern
                        .iter()
                        .all(|c| *c == b'.' || *c == b'y' || *c == b'g')
                {
                    break Pattern(pattern.try_into().unwrap());
                } else {
                    continue;
                }
            };

            remaining_words.retain(|word| wordle_match(word, &(&guess).into(), &pattern));
        }

        if remaining_words_lists.iter().any(Vec::is_empty) {
            println!("0 words.");
            break;
        } else if remaining_words_lists.iter().all(|list| list.len() == 1) {
            print!("Got it:");
            for remaining_words in remaining_words_lists.iter() {
                print!(" {}", remaining_words[0]);
            }
            println!();
            break;
        } else if remaining_words_lists.iter().all(|list| list.len() <= 20) {
            print!("Words remaining:");
            for remaining_words in remaining_words_lists.iter() {
                print!(" {}", remaining_words.len());
            }
            println!();
            for (i, remaining_words) in remaining_words_lists.iter().enumerate() {
                if remaining_words.len() > 1 {
                    println!("Game {}: {} words remaining:", i + 1, remaining_words.len());
                    for &word in remaining_words_lists[i].iter() {
                        println!("  {}", word);
                    }
                } else {
                    println!("Game {}: {}", i + 1, remaining_words[0]);
                }
            }
        } else {
            print!("Words remaining:");
            for remaining_words in remaining_words_lists.iter() {
                print!(" {}", remaining_words.len());
            }
            println!();
            for (i, remaining_words) in remaining_words_lists.iter().enumerate() {
                if remaining_words.len() == 1 {
                    println!("  Game {}: {}", i + 1, remaining_words[0]);
                }
            }
        }

        suggestion = find_suggestion(&all_words, &remaining_words_lists).into();
        for remaining_words in remaining_words_lists.iter() {
            if remaining_words.len() == 1 {
                let word = remaining_words[0].into();
                if !past_guesses.contains(&word) {
                    suggestion = word;
                    break;
                }
            }
        }
        println!("Suggestion: {}", suggestion);
    }
}

pub fn find_suggestion(
    all_words: &[WordRef<'static>],
    remaining_words_lists: &[Vec<WordRef<'static>>],
) -> WordRef<'static> {
    // Find the guess with the smallest ...
    let best_word = all_words
        .par_iter()
        .min_by_key(|&guess| {
            // ... total ...
            remaining_words_lists
                .iter()
                .map(|remaining_words| {
                    // ... expected value of ...
                    remaining_words
                        .iter()
                        .map(|answer| {
                            // ... count of words remaining after that guess, given that answer.
                            let pattern = wordle_pattern(guess, answer);

                            let remaining_words: Vec<_> = remaining_words
                                .iter()
                                .filter(|&candidate| wordle_match(candidate, guess, &pattern))
                                .collect();
                            let count = remaining_words.len();
                            if count > 0 {
                                if remaining_words.contains(&guess) {
                                    count - 1
                                } else {
                                    count
                                }
                            } else {
                                0
                            }
                        })
                        .sum::<usize>()
                    // If we were truly calculating the expected value of number of words
                    // remaining, we'd divide by the length of the word list here. But as we're
                    // feeding it to a min() function and every answer is drawn from the same word
                    // list, there's no point, we get the same answer.
                })
                .sum::<usize>()
        })
        .unwrap();
    *best_word
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pattern([u8; 5]);

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Pattern")
            .field(&Word(self.0).to_string())
            .finish()
    }
}

impl Deref for Pattern {
    type Target = [u8; 5];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn wordle_pattern(guess: &WordRef<'_>, answer: &WordRef<'_>) -> Pattern {
    let mut remaining_letters = *answer.0;
    let mut result: [u8; 5] = b".....".to_owned();
    for i in 0..5 {
        if guess[i] == answer[i] {
            result[i] = b'g';
            remaining_letters[i] = b'*';
        } else if let Some(j) = remaining_letters.iter().position(|c| c == &guess[i]) {
            result[i] = b'y';
            remaining_letters[j] = b'*';
        }
    }
    Pattern(result)
}

fn wordle_match(candidate: &WordRef<'_>, guess: &WordRef<'_>, pattern: &Pattern) -> bool {
    let mut remaining_letters = *candidate.0;
    for i in 0..5 {
        match pattern[i] {
            b'g' => {
                // Right letter, right place - keep words that have this letter
                // in this position
                if candidate[i] != guess[i] {
                    return false;
                }
                remaining_letters[i] = b'*';
            }
            b'y' => {
                // Right letter, wrong place - keep words that have this letter
                // somewhere but not the given position
                if let Some(j) = remaining_letters.iter().position(|c| c == &guess[i]) {
                    remaining_letters[j] = b'*';
                    if candidate[i] == guess[i] {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => {
                // Wrong letter - keep words that don't have this letter
                // (or only have this letter somewhere that will match green later)
                if remaining_letters
                    .iter()
                    .enumerate()
                    .any(|(j, c)| c == &guess[i] && (c != &guess[j] || i == j))
                {
                    return false;
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_wordle_pattern() {
        assert_eq!(
            Pattern(b".....".to_owned()),
            wordle_pattern(&WordRef(b"tidal"), &WordRef(b"nouse"))
        );
        assert_eq!(
            Pattern(b"...g.".to_owned()),
            wordle_pattern(&WordRef(b"tidal"), &WordRef(b"ocean"))
        );
        assert_eq!(
            Pattern(b".g..g".to_owned()),
            wordle_pattern(&WordRef(b"buddy"), &WordRef(b"funky"))
        );
        assert_eq!(
            Pattern(b"gg..g".to_owned()),
            wordle_pattern(&WordRef(b"buddy"), &WordRef(b"buggy"))
        );
        assert_eq!(
            Pattern(b".gg..".to_owned()),
            wordle_pattern(&WordRef(b"bongo"), &WordRef(b"tonal"))
        );
        assert_eq!(
            Pattern(b"...y.".to_owned()),
            wordle_pattern(&WordRef(b"achoo"), &WordRef(b"bobby"))
        );
        assert_eq!(
            Pattern(b"....y".to_owned()),
            wordle_pattern(&WordRef(b"debag"), &WordRef(b"foggy"))
        );
        assert_eq!(
            Pattern(b"y.yyy".to_owned()),
            wordle_pattern(&WordRef(b"alarm"), &WordRef(b"karma"))
        );
        assert_eq!(
            Pattern(b"ygg.y".to_owned()),
            wordle_pattern(&WordRef(b"alarm"), &WordRef(b"llama"))
        );
        assert_eq!(
            Pattern(b"g..gg".to_owned()),
            wordle_pattern(&WordRef(b"stone"), &WordRef(b"skene"))
        );
        assert_eq!(
            Pattern(b"g..y.".to_owned()),
            wordle_pattern(&WordRef(b"shred"), &WordRef(b"skene"))
        );
    }

    #[test]
    fn test_wordle_match() {
        assert!(wordle_match(
            &WordRef(b"nouse"),
            &WordRef(b"tidal"),
            &Pattern(b".....".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"ocean"),
            &WordRef(b"tidal"),
            &Pattern(b"...g.".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"tonal"),
            &WordRef(b"bongo"),
            &Pattern(b".gg..".to_owned())
        ));
        assert!(!wordle_match(
            &WordRef(b"tonal"),
            &WordRef(b"bongo"),
            &Pattern(b".gg.y".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"bobby"),
            &WordRef(b"achoo"),
            &Pattern(b"...y.".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"foggy"),
            &WordRef(b"debag"),
            &Pattern(b"....y".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"stone"),
            &WordRef(b"skene"),
            &Pattern(b"g..gg".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"skene"),
            &WordRef(b"shred"),
            &Pattern(b"g..y.".to_owned())
        ));
        assert!(wordle_match(
            &WordRef(b"nutty"),
            &WordRef(b"jetty"),
            &Pattern(b"..ggg".to_owned())
        ));
        assert!(!wordle_match(
            &WordRef(b"jetty"),
            &WordRef(b"jetty"),
            &Pattern(b"..ggg".to_owned())
        ));
    }
}
