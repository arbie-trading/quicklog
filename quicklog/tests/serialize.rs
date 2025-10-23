use quicklog::info;

use common::{BigStruct, SerializeStruct};

mod common;

fn main() {
    setup!();

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    let bs = BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    };

    assert_message_equal!(info!(^s), "s=Hello");
    assert_message_equal!(info!(^s, "with fmt string:"), "with fmt string: s=Hello");
    assert_message_equal!(
        info!(^s, ^bs, "s, bs:"),
        format!(
            "s, bs: s=Hello bs=vec: {:?}, str: {}",
            vec![1; 100],
            "The quick brown fox jumps over the lazy dog"

    // Test Vec serialization
    let vec_i32: Vec<i32> = vec![1, 2, 3, 4, 5];
    assert_message_equal!(info!("numbers: {}", ^vec_i32), "numbers: [1, 2, 3, 4, 5]");

    let vec_empty: Vec<u64> = Vec::new();
    assert_message_equal!(info!("empty: {}", ^vec_empty), "empty: []");

    let vec_strings: Vec<&str> = vec!["hello", "world"];
    assert_message_equal!(info!("words: {}", ^vec_strings), "words: [hello, world]");

    // Test Vec with Option
    let vec_opt: Vec<Option<i32>> = vec![Some(10), None, Some(20)];
    assert_message_equal!(
        info!("options: {}", ^vec_opt),
        "options: [Some(10), None, Some(20)]"
    );
}
