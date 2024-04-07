use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{self, Write};

fn main() -> Result<()> {
    let f = File::open("words.csv")?;
    let mut reader = BufReader::new(f);

    let mut words = WordTrie::new();

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let mut fields: Vec<_> = line.trim().split(',').map(str::to_owned).collect();
        if fields.len() != 2 {
            return Err(anyhow!("Bad word list file"));
        }

        let anagrams = fields.pop().unwrap();
        let mut alphagram: Vec<char> = fields.pop().unwrap().trim().chars().collect();
        alphagram.sort_unstable();
        let alphagram: String = alphagram.into_iter().collect();
        words.add(&alphagram, (alphagram.len(), anagrams));
    }

    print!("Enter letters: ");
    io::stdout().flush()?;

    let mut letters = String::new();
    io::stdin().read_line(&mut letters)?;
    letters.make_ascii_uppercase();
    let mut letters: Vec<char> = letters.trim().chars().collect();
    letters.sort_unstable();
    let letters: String = letters.into_iter().collect();

    let mut substrings = words.find_substrings(letters);
    substrings.sort_unstable();
    substrings.dedup();
    for (len, word) in substrings {
        println!("{}: {}", len, word);
    }

    Ok(())
}

#[derive(Debug)]
struct WordTrie<T> {
    entries: HashMap<char, WordTrie<T>>,
    value: Option<T>,
}

impl<'a, T> WordTrie<T> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            value: None,
        }
    }

    fn add(&mut self, word: &str, value: T) {
        if word.is_empty() {
            self.value = Some(value);
        } else {
            let mut chars = word.chars();
            let first = chars.next().unwrap();
            let rest: String = chars.collect();

            let entry = self.entries.entry(first).or_insert_with(WordTrie::new);
            entry.add(&rest, value);
        }
    }

    fn find_substrings(&self, letters: String) -> Vec<&T> {
        let mut acc = vec![];
        self.find_substrings_internal(letters, &mut acc);
        acc
    }

    fn find_substrings_internal(&'a self, letters: String, acc: &mut Vec<&'a T>) {
        if let Some(ref value) = &self.value {
            acc.push(value);
        }

        if !letters.is_empty() {
            let mut chars = letters.chars();
            let first = chars.next().unwrap();
            let rest: String = chars.collect();

            if let Some(entry) = self.entries.get(&first) {
                entry.find_substrings_internal(rest.clone(), acc);
            }

            self.find_substrings_internal(rest, acc);
        }
    }
}
