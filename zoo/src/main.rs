use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

#[derive(Clone, Debug, Default)]
struct Cost {
    word: String,
    amount: usize,
}

impl Cost {
    fn new(word: &str, amount: usize) -> Cost {
        Cost {
            word: word.to_uppercase(),
            amount,
        }
    }
}

fn puzzle_costs() -> Vec<Cost> {
    vec![
        Cost::new("EMU", 27),
        Cost::new("BILBY", 49),
        Cost::new("QUOLL", 49),
        Cost::new("ZOO", 50),
        Cost::new("ORYX", 52),
        Cost::new("KOALA", 58),
        Cost::new("OTTER", 60),
        Cost::new("BABOON", 62),
        Cost::new("CIVET", 66),
        Cost::new("FENICFOX", 69),
        Cost::new("IGUANA", 70),
        Cost::new("MEERCAT", 72),
        Cost::new("WALLABY", 74),
        Cost::new("FLAMINGO", 75),
        Cost::new("JAGUAR", 81),
        Cost::new("SEALION", 83),
        Cost::new("CHEETAH", 85),
        Cost::new("ADELAIDE", 99),
        Cost::new("REDPANDA", 104),
        Cost::new("KANGAROO", 106),
    ]
}

#[derive(Clone, Debug, Default)]
struct Prices([usize; 26]);

impl Prices {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, c: char, p: usize) {
        let cidx = c as usize - 'A' as usize;
        debug_assert_eq!(self.0[cidx], 0);

        self.0[cidx] = p;
    }

    fn get_price(&self, c: char) -> usize {
        let cidx = c as usize - 'A' as usize;
        self.0[cidx]
    }

    fn price_used(&self, p: usize) -> bool {
        self.0.contains(&p)
    }

    fn unused_prices(&self) -> impl Iterator<Item = usize> + '_ {
        (1..=26).filter(|p| !self.0.contains(p))
    }
}

impl<const N: usize> From<[(char, usize); N]> for Prices {
    fn from(arr: [(char, usize); N]) -> Prices {
        let mut prices = Prices::new();
        for (c, p) in arr {
            prices.insert(c, p);
        }
        prices
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
struct LetterSet(u32);

impl LetterSet {
    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, s: &str) {
        for c in s.chars() {
            let cidx = c as usize - 'A' as usize;
            self.0 |= 1 << cidx;
        }
    }

    fn difference(&self, other: &LetterSet) -> LetterSet {
        LetterSet(self.0 & !other.0)
    }

    fn count(&self) -> usize {
        self.0.count_ones() as usize
    }
}

impl<T> From<T> for LetterSet
where
    T: AsRef<str>,
{
    fn from(s: T) -> LetterSet {
        let mut l = LetterSet::new();
        l.add(s.as_ref());
        l
    }
}

impl fmt::Debug for LetterSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for i in 0..32 {
            if self.0 & (1 << i) != 0 {
                s.push(('A' as u32 + i).try_into().unwrap());
            }
        }
        f.debug_tuple("LetterSet").field(&s).finish()
    }
}

fn count_permutations(n: usize, r: usize) -> usize {
    (n - r + 1..=n).product()
}

fn count_solutions(word: &str, used_so_far: &LetterSet) -> usize {
    let newset = LetterSet::from(word).difference(used_so_far);
    let count = newset.count();
    if count == 0 {
        1
    } else {
        let remaining = 26 - used_so_far.count();
        count_permutations(remaining, count - 1)
    }
}

#[derive(Clone, Debug, Eq)]
struct Path {
    word_order: Vec<usize>,
    words_used: usize,
    letters_used: LetterSet,
    solutions: usize,
}

impl Path {
    fn new() -> Path {
        Path {
            word_order: Vec::new(),
            words_used: 0,
            letters_used: LetterSet::new(),
            solutions: 1,
        }
    }

    fn append(&self, word: &str, word_idx: usize) -> Path {
        let mut new = self.clone();
        new.word_order.push(word_idx);
        new.words_used |= 1 << word_idx;
        new.letters_used.add(word);
        new.solutions *= count_solutions(word, &self.letters_used);
        new
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> Ordering {
        other.solutions.cmp(&self.solutions)
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.solutions == other.solutions
    }
}

fn find_shortest_path(costs: &[Cost]) -> Vec<usize> {
    let mut candidates: BinaryHeap<Path> = BinaryHeap::from([Path::new()]);

    loop {
        let p = candidates.pop().unwrap();
        if p.letters_used.0 == (1 << 26) - 1 {
            return p.word_order;
        }

        for (i, cost) in costs.iter().enumerate() {
            if p.words_used & (1 << i) == 0 {
                candidates.push(p.append(&cost.word, i));
            }
        }
    }
}

fn calc_sum(cost: &Cost, prices: &Prices) -> (usize, String) {
    let mut undef = String::new();

    let sum: usize = cost
        .word
        .chars()
        .map(|c| {
            let price = prices.get_price(c);
            if price == 0 && !undef.contains(c) {
                undef.push(c);
            }
            price
        })
        .sum();

    (sum, undef)
}

fn find_solutions(cost: &Cost, so_far: &Prices) -> Vec<Prices> {
    // #1. Sum already assigned letters in cost.word
    let (sum, undef) = calc_sum(cost, so_far);
    if sum > cost.amount {
        return vec![];
    }

    match undef.len() {
        //  #2a. If undef == 0, check sum == amount
        //          => return [so_far] if true, [] if false
        0 => {
            if cost.amount == sum {
                vec![so_far.clone()]
            } else {
                vec![]
            }
        }

        //  #2b. If undef == 1, check (amount - sum) is in (1..=26) and unallocated
        //          => return [so_far.insert(unassigned[0], amount - sum)]
        //      else
        //          => return []
        1 => {
            let target_char = undef.chars().next().unwrap();
            let mut target_price = cost.amount - sum;
            let count = cost.word.chars().filter(|&c| c == target_char).count();
            if target_price % count == 0 {
                target_price /= count;
            } else {
                return vec![];
            }

            if (1..=26).contains(&target_price) && !so_far.price_used(target_price) {
                let mut new_prices = so_far.clone();
                new_prices.insert(target_char, target_price);
                vec![new_prices]
            } else {
                vec![]
            }
        }

        //  #2c. Else loop through all unallocated prices in so_far <= (amount - sum),
        //          assign to first unassigned letter in so_far, and recurse
        _ => {
            let next_char = undef.chars().next().unwrap();
            so_far
                .unused_prices()
                .filter_map(|p| {
                    if p <= cost.amount - sum {
                        let mut new_prices = so_far.clone();
                        new_prices.insert(next_char, p);
                        Some(new_prices)
                    } else {
                        None
                    }
                })
                .flat_map(|prices| find_solutions(cost, &prices))
                .collect()
        }
    }
}

fn solve(costs: &[Cost], path: &[usize], prices: &Prices) -> Option<Prices> {
    find_solutions(&costs[path[0]], prices)
        .iter()
        .find_map(|new_prices| {
            if path.len() == 1 {
                Some(new_prices.clone())
            } else {
                solve(costs, &path[1..], new_prices)
            }
        })
}

fn main() {
    let costs = puzzle_costs();

    let path = find_shortest_path(&costs);

    if let Some(prices) = solve(&costs, &path, &Prices::new()) {
        for c in 'A'..='Z' {
            println!("{c}: {}", prices.get_price(c));
        }
        let b = prices.get_price('B');
        let c = prices.get_price('C');
        let e = prices.get_price('E');
        let f = prices.get_price('F');
        let k = prices.get_price('K');
        let t = prices.get_price('T');
        let w = prices.get_price('W');
        println!(
            "Cache location: S34 54.{}  E138 36.{}",
            k * e * f + c,
            t * b * e + w
        );
    } else {
        println!("No solution.");
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_count_solutions() {
        assert_eq!(1, count_solutions("E", &LetterSet::from("")));

        assert_eq!(26 * 25, count_solutions("EMU", &LetterSet::from("")));

        assert_eq!(
            26 * 25 * 24 * 23 * 22 * 21 * 20,
            count_solutions("FLAMINGO", &LetterSet::from(""))
        );

        assert_eq!(25, count_solutions("EMU", &LetterSet::from("E")));

        assert_eq!(
            23 * 22 * 21 * 20 * 19 * 18,
            count_solutions("FLAMINGO", &LetterSet::from("EMU"))
        );

        assert_eq!(26 * 25 * 24, count_solutions("BILBY", &LetterSet::from("")));

        assert_eq!(
            23 * 22 * 21,
            count_solutions("BILBY", &LetterSet::from("EMU"))
        );

        assert_eq!(
            26 * 25 * 24,
            count_solutions("BABOON", &LetterSet::from(""))
        );
    }
}
