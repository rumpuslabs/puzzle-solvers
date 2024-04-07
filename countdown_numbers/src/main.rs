use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Write};
use std::ops::{Add, Div, Mul, Sub};
use std::time::Instant;

fn main() -> Result<()> {
    print!("Enter numbers: ");
    io::stdout().flush()?;

    let mut numbers = String::new();
    io::stdin().read_line(&mut numbers)?;
    let mut numbers: Vec<u32> = numbers
        .trim()
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;

    if numbers.len() != 6 {
        return Err(anyhow!("Please provide exactly 6 numbers"));
    }

    numbers.sort_unstable();
    numbers.reverse();

    print!("Enter target: ");
    io::stdout().flush().unwrap();

    let mut target = String::new();
    io::stdin().read_line(&mut target)?;
    let target: u32 = target.trim().parse()?;

    println!(
        "Making {} from {}",
        target,
        numbers
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!();

    let start = Instant::now();
    let solutions = find_path(target, numbers.into_iter().map(Step::Literal).collect());

    if !solutions.is_empty() {
        let mut solutions: Vec<_> = solutions.into_iter().map(Step::simplify).collect();

        solutions.sort();
        solutions.dedup();
        println!("Searching took {:.2?}", start.elapsed());
        println!();

        solutions.sort_by_key(Step::len);

        solutions.iter().rev().for_each(|solution| {
            println!("{} = {}", solution.value(), solution);
        });
        println!("{} solutions.", solutions.len());
    } else {
        println!("Searching took {:.2?}", start.elapsed());
        println!();

        println!("No solution found.");
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    Literal(u32),
    Operation { operator: Op, operands: Vec<Step> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Step {
    fn value(&self) -> u32 {
        match self {
            Step::Literal(value) => *value,
            Step::Operation { operator, operands } => {
                let opfn = match operator {
                    Op::Add => Add::add,
                    Op::Sub => Sub::sub,
                    Op::Mul => Mul::mul,
                    Op::Div => Div::div,
                };

                operands.iter().map(Step::value).reduce(opfn).unwrap()
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Step::Literal(_) => 1,
            Step::Operation { operands, .. } => operands.iter().map(Step::len).sum(),
        }
    }

    fn simplify(self) -> Self {
        let old = self.clone();
        let new = match self {
            Step::Literal(_) => self,
            Step::Operation { operator, operands } => {
                let mut operands: Vec<_> = operands.into_iter().map(Step::simplify).collect();

                let mut changed = false;
                let (new_operator, new_operands) = match operator {
                    Op::Add => {
                        operands.sort();
                        let mut new_operator = operator;
                        let mut new_operands = vec![];
                        let mut sub_operands = vec![];
                        for mut operand in operands {
                            match operand {
                                Step::Operation {
                                    operator: Op::Add,
                                    ref mut operands,
                                } => {
                                    changed = true;
                                    new_operands.append(operands)
                                }
                                Step::Operation {
                                    operator: Op::Sub,
                                    ref mut operands,
                                } => {
                                    let mut tail_operands = operands.split_off(1);
                                    sub_operands.append(&mut tail_operands);
                                    new_operands.append(operands);
                                }
                                operand => new_operands.push(operand),
                            }
                        }

                        if !sub_operands.is_empty() {
                            new_operator = Op::Sub;
                            new_operands = vec![Step::Operation {
                                operator: Op::Add,
                                operands: new_operands,
                            }
                            .simplify()];
                            new_operands.append(&mut sub_operands);

                            new_operands[1..].sort();
                            new_operands[1..].reverse();
                        } else {
                            new_operands.sort();
                            new_operands.reverse();
                        }

                        (new_operator, new_operands)
                    }
                    Op::Sub => {
                        operands[1..].sort();
                        let mut new_operands = vec![];
                        for (i, mut operand) in operands.into_iter().enumerate() {
                            if i == 0 {
                                match operand {
                                    Step::Operation {
                                        operator: Op::Sub,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        new_operands.append(operands)
                                    }
                                    operand => new_operands.push(operand),
                                }
                            } else {
                                match operand {
                                    Step::Operation {
                                        operator: Op::Add,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        new_operands.append(operands)
                                    }
                                    Step::Operation {
                                        operator: Op::Sub,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        // (a - b - c - (d - e - f))
                                        // new_operands = (a - b - c)
                                        // operands = (d - e - f)
                                        let mut tail_operands = new_operands.split_off(1);
                                        // new_operands = (a)
                                        // tail_operands = (b - c)
                                        new_operands.append(&mut operands.split_off(1));
                                        // new_operands = (a + e + f)
                                        // operands = (d)
                                        new_operands = vec![Step::Operation {
                                            operator: Op::Add,
                                            operands: new_operands,
                                        }
                                        .simplify()];
                                        // new_operands = ((a + e + f))
                                        new_operands.append(&mut tail_operands);
                                        // new_operands = ((a + e + f) - b - c)
                                        new_operands.append(operands);
                                        // new_operands = ((a + e + f) - b - c - d)
                                    }
                                    operand => new_operands.push(operand),
                                }
                            }
                        }
                        new_operands[1..].sort();
                        new_operands[1..].reverse();
                        (operator, new_operands)
                    }
                    Op::Mul => {
                        operands.sort();
                        let mut new_operator = operator;
                        let mut new_operands = vec![];
                        let mut div_operands = vec![];
                        for mut operand in operands {
                            match operand {
                                Step::Operation {
                                    operator: Op::Mul,
                                    ref mut operands,
                                } => {
                                    changed = true;
                                    new_operands.append(operands)
                                }
                                Step::Operation {
                                    operator: Op::Div,
                                    ref mut operands,
                                } => {
                                    let mut tail_operands = operands.split_off(1);
                                    div_operands.append(&mut tail_operands);
                                    new_operands.append(operands);
                                }
                                operand => new_operands.push(operand),
                            }
                        }

                        if !div_operands.is_empty() {
                            new_operator = Op::Div;
                            new_operands = vec![Step::Operation {
                                operator: Op::Mul,
                                operands: new_operands,
                            }
                            .simplify()];
                            new_operands.append(&mut div_operands);

                            new_operands[1..].sort();
                            new_operands[1..].reverse();
                        } else {
                            new_operands.sort();
                            new_operands.reverse();
                        }

                        (new_operator, new_operands)
                    }
                    Op::Div => {
                        operands[1..].sort();
                        let mut new_operands = vec![];
                        for (i, mut operand) in operands.into_iter().enumerate() {
                            if i == 0 {
                                match operand {
                                    Step::Operation {
                                        operator: Op::Div,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        new_operands.append(operands)
                                    }
                                    operand => new_operands.push(operand),
                                }
                            } else {
                                match operand {
                                    Step::Operation {
                                        operator: Op::Mul,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        new_operands.append(operands)
                                    }
                                    Step::Operation {
                                        operator: Op::Div,
                                        ref mut operands,
                                    } => {
                                        changed = true;
                                        // (a / b / c / (d / e / f))
                                        // new_operands = (a / b / c)
                                        // operands = (d / e / f)
                                        let mut tail_operands = new_operands.split_off(1);
                                        // new_operands = (a)
                                        // tail_operands = (b / c)
                                        new_operands.append(&mut operands.split_off(1));
                                        // new_operands = (a * e * f)
                                        // operands = (d)
                                        new_operands = vec![Step::Operation {
                                            operator: Op::Mul,
                                            operands: new_operands,
                                        }
                                        .simplify()];
                                        // new_operands = ((a * e * f))
                                        new_operands.append(&mut tail_operands);
                                        // new_operands = ((a * e * f) / b / c)
                                        new_operands.append(operands);
                                        // new_operands = ((a * e * f) / b / c / d)
                                    }
                                    operand => new_operands.push(operand),
                                }
                            }
                        }
                        new_operands[1..].sort();
                        new_operands[1..].reverse();
                        (operator, new_operands)
                    }
                };

                let new_operation = Step::Operation {
                    operator: new_operator,
                    operands: new_operands,
                };

                if changed {
                    new_operation.simplify()
                } else {
                    new_operation
                }
            }
        };
        if new.value() != old.value() {
            dbg!(old.value());
            dbg!(&old);
            dbg!(new.value());
            dbg!(&new);
            panic!();
        }
        new
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step::Literal(value) => value.fmt(f),
            Step::Operation { operator, operands } => write!(
                f,
                "({})",
                operands
                    .iter()
                    .map(Step::to_string)
                    .collect::<Vec<_>>()
                    .join(&format!(" {} ", operator.to_string()))
            ),
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
        }
    }
}

fn find_path(target: u32, numbers: Vec<Step>) -> Vec<Step> {
    let mut candidates: VecDeque<Vec<Step>> = vec![numbers].into();
    let mut solutions = Vec::new();

    while let Some(numbers) = candidates.pop_front() {
        numbers.iter().enumerate().for_each(|(ix, x)| {
            let mut new_numbers = numbers.clone();
            new_numbers.swap_remove(ix);

            new_numbers.iter().enumerate().for_each(|(iy, y)| {
                if x.value() >= y.value() {
                    let mut new_numbers = new_numbers.clone();
                    new_numbers.swap_remove(iy);

                    let mut context = (target, &new_numbers, &mut candidates, &mut solutions);

                    do_step(x, y, Op::Add, &mut context);

                    if x.value() > y.value() {
                        do_step(x, y, Op::Sub, &mut context);
                    }

                    if y.value() != 1 {
                        do_step(x, y, Op::Mul, &mut context);

                        if x.value() % y.value() == 0 {
                            do_step(x, y, Op::Div, &mut context);
                        }
                    }
                }
            });
        });
    }

    solutions
}

fn do_step(
    x: &Step,
    y: &Step,
    operator: Op,
    context: &mut (u32, &Vec<Step>, &mut VecDeque<Vec<Step>>, &mut Vec<Step>),
) {
    let (target, numbers, ref mut candidates, ref mut solutions) = context;

    let new_step = Step::Operation {
        operator,
        operands: vec![x.clone(), y.clone()],
    };

    if new_step.value() == *target {
        solutions.push(new_step);
    } else if !numbers.is_empty() {
        let mut new_numbers = (*numbers).clone();
        new_numbers.push(new_step);
        candidates.push_back(new_numbers);
    }
}
