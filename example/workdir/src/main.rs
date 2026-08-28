fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn test_read_file() {
        let content = fs::read_to_string("test.txt")
            .expect("Cannot read test.txt, please check if it exist");
        println!("{}", content);
    }
}