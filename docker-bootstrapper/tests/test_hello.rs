use std::env;

#[test]
fn test_hello() {
    let path = env::current_exe().unwrap();
    println!("path: {:?}", path);
    println!("hello, integrated");
}
