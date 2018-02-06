
/// Uppercase the first character of each word in a string
pub fn unwords(text: &str, pattern: &str) -> String {
    let text = text.to_lowercase();
    let words = text.split(pattern).collect::<Vec<&str>>();

    let mut text = String::default();

    for (k, v) in words.iter().enumerate() {
        let mut s = v.to_string();

        {
            let s = s.get_mut(0..1);
            s.map(|s| s.make_ascii_uppercase() );
        }

        if k != 0 {
            text.push('-');
        }

        text.push_str(&s);
    }

    text
}