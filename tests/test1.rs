use test_attr::*;

#[rename(name = "world")]
fn hello() {
    println!("hello world!");
}

fn main() {
    world();
}
