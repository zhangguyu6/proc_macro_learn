use test_attr::*;

#[derive(Change)]
struct A {
    #[rename(name = "b")]
    a: u8,
    c: u8,
}

fn main() {
    let a1: ANewer = ANewer { b: 1, c: 2 };
}
