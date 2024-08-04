struct Container<F> {
    task: F,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_unit() {
        println!("hello, unit");
    }
}
