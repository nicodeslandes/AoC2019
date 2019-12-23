extern crate generic_matrix;
extern crate num;
extern crate regex;

use crate::num::Integer;
use crate::num::Signed;
use crate::num::ToPrimitive;
use generic_matrix::Matrix;
use num::integer::gcd;
use num::rational::BigRational;
use num::BigInt;
use num::One;
use num::Zero;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::result::Result;

type MainResult<T> = Result<T, Box<dyn ::std::error::Error>>;
const DECK_LENGTH: usize = 119315717514047;
const CARD_INDEX: usize = 2020;
const LOOPS: usize = 101741582076661;

// const DECK_LENGTH: usize = 10007;
// const CARD_INDEX: usize = 2019;
// const LOOPS: usize = 1;

#[derive(Debug)]
enum Operation {
    DealWithIncrement(usize),
    DealIntoNewStack,
    Cut(i32),
}

fn main() -> MainResult<()> {
    let file_name = env::args().nth(1).expect("Enter a file name");
    let operations = read_operations(&file_name)?;

    //println!("Operations: {:?}", operations);

    // f = a.x + b

    //                (a 0)
    // (y  1) = (x 1) (b 1)
    // Y = X.M

    // M = (a 0)
    //     (b 1)

    // M^2 = (a^2  0)
    //       (ab+b 1)
    // b = f(0)
    // a = f(1) - b

    // fn(x) = c.x + d
    let b = shuffle(&operations, BigInt::zero());
    let a = shuffle(&operations, BigInt::one()) - b.clone();
    let a = if a.is_negative() { a + DECK_LENGTH } else { a };

    let m = Matrix::from_vec(2, 2, vec![a, BigInt::zero(), b.clone(), BigInt::one()]);
    println!("M: {:?}", m);

    let x = Matrix::from_vec(1, 2, vec![CARD_INDEX, 1]);
    let y = x.clone() * m.clone();

    println!("Result: {}", y[(0, 0)].clone() % DECK_LENGTH);

    // We need to calculate x so that x * M^n = (2020 1), ie x = (2020 1) * (M^n)^-1

    let mn = pow(m, LOOPS);

    //       (x    0)
    //       (y    1)
    // (a 0) (ax   0)
    // (b 1) (bx+y 1)  x = 1/a; bx+y = 0 => y = -b/a
    let mut mn_inv = Matrix::from_vec(
        2,
        2,
        vec![
            BigRational::new(BigInt::one(), mn[(0, 0)].clone()),
            BigRational::zero(),
            BigRational::new(-mn[(0, 1)].clone(), mn[(0, 0)].clone()),
            BigRational::one(),
        ],
    );

    //normalize(&mut mn_inv);
    println!("M^n: {:?}\nM^-n: {:?}", mn.clone(), mn_inv.clone());
    let x = Matrix::from_fn(1, 2, |i, j| {
        BigRational::new(BigInt::from(x[(i, j)]), BigInt::one())
    });
    let y = x * mn_inv;
    let res = y[(0, 0)].clone();

    // We need to inverse res's denominator in Z/pZ
    // Get Bézout's coefficients
    let denom = res.denom();
    let g = BigInt::extended_gcd(&denom, &BigInt::from(DECK_LENGTH));
    println!("x: {}, y: {}", g.x, g.y);

    //normalize(&mut y);
    println!("Result: {}", g.x.clone() * res.numer()); // % DECK_LENGTH);
    println!("Result: {}", (g.x * res.numer()) % DECK_LENGTH);

    // f2(x) = a.(ax + b) + b = a2.x + (ab + b)
    // f3(x) = a.(a2.x + (ab + b)) + b = a3.x + (a2b + ab + b)
    // f(n + m)(x) = (f(n) o f(m))(x)
    // f(n^α + m^β) = f(n^α) ο f(m^β)

    //println!("Result: {}", result);
    Ok(())
}

pub fn pow(mut base: Matrix<BigInt>, mut exp: usize) -> Matrix<BigInt> {
    if exp == 0 {
        return Matrix::one(2, 2);
    }

    while exp & 1 == 0 {
        base = base.clone() * base;
        normalize(&mut base);
        exp >>= 1;
    }
    if exp == 1 {
        return base;
    }

    let mut acc = base.clone();
    while exp > 1 {
        exp >>= 1;
        base = base.clone() * base;
        normalize(&mut base);
        if exp & 1 == 1 {
            acc = acc * base.clone();
            normalize(&mut acc);
        }
    }
    acc
}

fn normalize(mat: &mut Matrix<BigInt>) {
    *mat = Matrix::from_fn(mat.row(), mat.column(), |i, j| {
        ((mat[(i, j)].clone() % DECK_LENGTH) + DECK_LENGTH) % DECK_LENGTH
    });
}

fn shuffle(operations: &Vec<Operation>, index: BigInt) -> BigInt {
    let mut index = index;
    for op in operations {
        index = apply_operation(&op, index);
    }

    index
}
fn apply_operation(op: &Operation, index: BigInt) -> BigInt {
    match *op {
        Operation::DealIntoNewStack => DECK_LENGTH - index - 1,
        Operation::Cut(n) => {
            let cut_index = if n >= 0 {
                n as i64
            } else {
                n as i64 + DECK_LENGTH as i64
            } as usize;

            let i = index - cut_index as i64;
            if i.is_negative() {
                i + DECK_LENGTH as i64
            } else {
                i
            }
        }
        Operation::DealWithIncrement(incr) => (index * incr) % DECK_LENGTH,
    }
}
fn read_operations(file_name: &str) -> MainResult<Vec<Operation>> {
    let file = File::open(file_name)?;
    let mut operations: Vec<Operation> = vec![];

    let mut reader = BufReader::new(&file);

    let re = Regex::new(
        r#"(?x)(?:deal\s+with\s+increment\s+(\d+))|
        (?:cut\s+(-?\d+))|
        (?:deal into new stack)"#,
    )?;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        line = line.trim().to_string();
        if read == 0 {
            break;
        }

        let op = {
            if line == "deal into new stack" {
                Operation::DealIntoNewStack
            } else {
                let capture = re.captures(&line).unwrap();
                if let Some(increment) = capture.get(1) {
                    Operation::DealWithIncrement(increment.as_str().parse().unwrap())
                } else if let Some(n) = capture.get(2) {
                    Operation::Cut(n.as_str().parse().unwrap())
                } else {
                    panic!("Invalid input")
                }
            }
        };

        operations.push(op);
    }

    Ok(operations)
}