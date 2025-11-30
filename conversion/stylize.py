"""
Run Neural Style Transfer on the given content and style images.

Accepts 3 arguments:
1. Path to the content image
2. Path to the style image
3. Path to save the stylized image
"""
import os
import sys

if len(sys.argv) != 4:
    print("Usage: python3 stylize.py <content_image_path> <style_image_path> <output_image_path>")
    exit(1)

# suppress tensorflow spam on the output
os.environ["TF_ENABLE_ONEDNN_OPTS"] = "0"
os.environ["TF_CPP_MIN_LOG_LEVEL"] = "2"

from PIL import Image
import numpy as np
import pillow_heif
import tensorflow as tf


def tensor_to_image(tensor):
    """Converts a tensor to a PIL image."""
    tensor = tensor*255
    tensor = np.array(tensor, dtype=np.uint8)
    if np.ndim(tensor) > 3:
        assert tensor.shape[0] == 1
        tensor = tensor[0]
    return Image.fromarray(tensor)


def load_img(path_to_img, max_dim):
    # HEIC is not supported by tf.image, so decode it with pillow-heif + PIL
    if path_to_img.lower().endswith(".heic"):
        heif_file = pillow_heif.read_heif(path_to_img)
        img = Image.frombytes(
            heif_file.mode,
            heif_file.size,
            heif_file.data,
            "raw",
            heif_file.mode,
            heif_file.stride,
        )
    else:
        # Other formats (JPG, PNG, etc.)
        img = Image.open(path_to_img)

    img = img.convert("RGB")  # ensure 3 channels
    img = np.array(img)       # convert to NumPy array
    img = tf.convert_to_tensor(img, dtype=tf.float32) / 255.0

    # Resize
    shape = tf.cast(tf.shape(img)[:-1], tf.float32)
    long_dim = tf.reduce_max(shape)
    scale = max_dim / long_dim
    new_shape = tf.cast(shape * scale, tf.int32)

    img = tf.image.resize(img, new_shape)
    img = img[tf.newaxis, :]  # add batch dim
    return img

hub_model = tf.saved_model.load('/app/saved_model')

content_path = sys.argv[1]
style_path = sys.argv[2]

content_image = load_img(content_path, max_dim=1024)
style_image = load_img(style_path, max_dim=450) # selected empirically for the specific style image

stylized_image = hub_model(tf.constant(content_image), tf.constant(style_image))[0]
combined = tensor_to_image(stylized_image)
combined.save(sys.argv[3])
