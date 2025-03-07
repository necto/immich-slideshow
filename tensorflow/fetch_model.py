"""
This script fetches the pre-trained arbitrary image stylization model from TFHub and saves it locally.
"""
import tensorflow as tf
import tensorflow_hub as hub
import os

# Specify the local path where you want to save the model
local_model_path = '/app/saved_model'

# Load the model from TFHub
hub_model = hub.load('https://tfhub.dev/google/magenta/arbitrary-image-stylization-v1-256/2')

# Save the model locally
os.makedirs(local_model_path, exist_ok=True)
tf.saved_model.save(hub_model, local_model_path)

print(f"Model saved to {local_model_path}")
