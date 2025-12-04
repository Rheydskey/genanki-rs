use std::{io::Read, path::Path};

use rstest::{fixture, rstest};

use crate::generator::Generator;

#[fixture]
pub fn get_test_folder<'a>() -> &'static Path {
    std::path::Path::new("./tests")
}

#[rstest]
pub fn test(get_test_folder: &Path) {
    let mut file = std::fs::File::open(get_test_folder.join("test.md")).unwrap();
    let mut input = String::new();
    file.read_to_string(&mut input).unwrap();
    let output = Generator {
        subproject_path: get_test_folder,
    }
    .generate_card_from_input(&input, get_test_folder);
    let first = &output[0];
    let second = &output[1];
    assert_eq!(first.front, "<h2>Nya</h2>");
    assert_eq!(first.back, r"<p>$ a \implies b $</p>");

    assert_eq!(second.front, "<h2>Meow</h2>");
    assert_eq!(second.back, r"<p>$$ a \implies b $$</p>");
}

#[rstest]
#[case(
    r#"## Is Blahaj > boykisser
$$ Blahaj\<Shark> > boykisser\<Cat> $$"#,
    "<h2>Is Blahaj &gt; boykisser</h2>",
    r"<p>$$ Blahaj\<Shark> > boykisser\<Cat> $$</p>"
)]
#[case(
    r#"## 
Empty OwO"#,
    "<h2></h2>",
    r"<p>Empty OwO</p>"
)]
pub fn test_output(
    get_test_folder: &Path,
    #[case] input: &str,
    #[case] front: &str,
    #[case] back: &str,
) {
    let output = Generator {
        subproject_path: get_test_folder,
    }
    .generate_card_from_input(&input, get_test_folder);
    let first = &output[0];
    assert_eq!(first.front, front);
    assert_eq!(first.back, back);
}

#[rstest]
pub fn test_escape_twice(get_test_folder: &Path) {
    let input = r#"
## Is Blahaj > boykisser
$$ Blahaj\<Shark> > boykisser\<Cat> $$

## Is Blahaj > boykisser
$$ Blahaj\<Shark> > boykisser\<Cat> $$
    "#;

    let output = Generator {
        subproject_path: get_test_folder,
    }
    .generate_card_from_input(&input, get_test_folder);
    println!("{:#?}", output);
    let first = &output[0];
    let second = &output[1];
    assert_eq!(first.front, "<h2>Is Blahaj &gt; boykisser</h2>");
    assert_eq!(first.back, r"<p>$$ Blahaj\<Shark> > boykisser\<Cat> $$</p>");

    assert_eq!(second.front, "<h2>Is Blahaj &gt; boykisser</h2>");
    assert_eq!(
        second.back,
        r"<p>$$ Blahaj\<Shark> > boykisser\<Cat> $$</p>"
    );
}

#[rstest]
pub fn test_image_base64(get_test_folder: &Path) {
    let input = r#"
## Is Blahaj > boykisser
![title](blahaj.png)
    "#;
    let output = Generator {
        subproject_path: get_test_folder,
    }
    .generate_card_from_input(&input, get_test_folder);
    println!("{:#?}", output);
    let first = &output[0];
    assert_eq!(first.front, "<h2>Is Blahaj &gt; boykisser</h2>");
    assert_eq!(
        first.back,
        r#"<p><img src="image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAAXNSR0IB2cksfwAAAARnQU1BAACxjwv8YQUAAAAgY0hSTQAAeiYAAICEAAD6AAAAgOgAAHUwAADqYAAAOpgAABdwnLpRPAAAADhJREFUCNc1yDERgEAQBMGZLUIMoBkBnyMOA8TcEtFhe63zTrIDvDNs6DEtACpRn/4BpC0qANPyAYb0EsnPnK8eAAAAAElFTkSuQmCC" alt="title" /></p>"#
    );
}

#[rstest]
pub fn test_absolute_path_image_base64(get_test_folder: &Path) {
    let input = r#"
## Is Blahaj > boykisser
![title](/blahaj.png)
    "#;
    let output = Generator {
        subproject_path: get_test_folder,
    }
    .generate_card_from_input(&input, get_test_folder);
    println!("{:#?}", output);
    let first = &output[0];
    assert_eq!(first.front, "<h2>Is Blahaj &gt; boykisser</h2>");
    assert_eq!(
        first.back,
        r#"<p><img src="image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAAXNSR0IB2cksfwAAAARnQU1BAACxjwv8YQUAAAAgY0hSTQAAeiYAAICEAAD6AAAAgOgAAHUwAADqYAAAOpgAABdwnLpRPAAAADhJREFUCNc1yDERgEAQBMGZLUIMoBkBnyMOA8TcEtFhe63zTrIDvDNs6DEtACpRn/4BpC0qANPyAYb0EsnPnK8eAAAAAElFTkSuQmCC" alt="title" /></p>"#
    );
}
