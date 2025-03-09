use image::{ImageBuffer, Rgb};

fn main() {
    // Create a 100x100 test image with a simple pattern
    let test_image = ImageBuffer::from_fn(100, 100, |x, y| {
        if (x < 50 && y < 50) || (x >= 50 && y >= 50) {
            Rgb([255, 0, 0]) // red
        } else {
            Rgb([0, 0, 255]) // blue
        }
    });
    
    // Save the test image
    test_image.save("tests/test_image.jpg").unwrap();
    println!("Created test image at tests/test_image.jpg");
}
