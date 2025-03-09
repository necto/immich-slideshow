use std::fs;

fn main() {
    // Create a simple text file as mock image data
    fs::write("tests/test_image.jpg", "mock image data for testing purposes")
        .unwrap();
    println!("Created mock test image at tests/test_image.jpg");
}
