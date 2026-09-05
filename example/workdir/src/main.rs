fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::env;

    #[test]
    fn test_read_file() {
        println!("PWD: {}", env::current_dir().unwrap().display());
        let content =
            fs::read_to_string("assets/test.txt").expect("Cannot read assets/test.txt, please check if it exist");
        println!("{}", content);
    }
}
