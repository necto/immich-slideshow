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

import numpy as np
import PIL.Image
import tensorflow as tf


def tensor_to_image(tensor):
    """Converts a tensor to a PIL image."""
    tensor = tensor*255
    tensor = np.array(tensor, dtype=np.uint8)
    if np.ndim(tensor)>3:
        assert tensor.shape[0] == 1
        tensor = tensor[0]
    return PIL.Image.fromarray(tensor)


def load_img(path_to_img, max_dim):
    """Loads an image from the given path and resizes it."""
    img = tf.io.read_file(path_to_img)
    img = tf.image.decode_image(img, channels=3)
    img = tf.image.convert_image_dtype(img, tf.float32)
    shape = tf.cast(tf.shape(img)[:-1], tf.float32)
    long_dim = max(shape)
    scale = max_dim / long_dim
    new_shape = tf.cast(shape * scale, tf.int32)
    img = tf.image.resize(img, new_shape)
    img = img[tf.newaxis, :]
    return img


hub_model = tf.saved_model.load('/app/saved_model')

content_path = sys.argv[1]
style_path = sys.argv[2]

content_image = load_img(content_path, max_dim=1024)
style_image = load_img(style_path, max_dim=450) # selected empirically for the specific style image

stylized_image = hub_model(tf.constant(content_image), tf.constant(style_image))[0]
combined = tensor_to_image(stylized_image)
combined.save(sys.argv[3])
