#![feature(proc_macro_hygiene)]
use test_attr::*;

fn main() {
    let a1: Vec<u8> = my_vec!(1, 1);
    println!("{:?}",a1);
}
