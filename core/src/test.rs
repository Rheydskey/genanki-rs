use std::io::Read;

use crate::generator::{CardGenerator, Generator};

#[test]
pub fn test() {
    let path = std::path::Path::new("./tests/test.md");
    let mut file = std::fs::File::open(path).unwrap();
    let mut input = String::new();
    file.read_to_string(&mut input).unwrap();
    let output = Generator::generate_card_from_input(&input, &path);
    let first = &output[0];
    let second = &output[1];
    assert_eq!(first.front, "<h2>Nya</h2>");
    assert_eq!(first.back, r"<p>$ a \implies b $</p>");

    assert_eq!(second.front, "<h2>Meow</h2>");
    assert_eq!(second.back, r"<p>$$ a \implies b $$</p>");
}
