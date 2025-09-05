use crate::generator::CardGenerator;

#[test]
pub fn test() {
    let input = r#"## Cas 3 des récurrence par partition
Pour
$$ f(n) \in \Omega(n^{log_b a+\varepsilon}) $$
Avec une certaine constante ε > 0.
$$ t(n) < \in \Theta(f(n))$$"#;
    let output = CardGenerator::new(input.to_string()).generate();
    assert_eq!(
        output.back,
        "<p>Pour\n$$ f(n) \\in \\Omega(n^{log_b a+\\varepsilon}) $$\nAvec une certaine constante ε > 0.\n$$ t(n) < \\in \\Theta(f(n))$$</p>"
    );
}
