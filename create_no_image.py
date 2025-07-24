from PIL import Image
import os

# Define the target path for the no_image.png
image_path = "C:\\OptiScaler-GUI\\assets\\icons\\no_image.png"

try:
    # Create a new black image (e.g., 60x80 pixels)
    img = Image.new('RGB', (60, 80), color = 'black')
    
    # Save the image as PNG
    img.save(image_path, format="PNG")
    print(f"Successfully created {image_path}")

    # Verify the image can be opened
    test_img = Image.open(image_path)
    test_img.verify() # Verify that the file is an image
    test_img.close()
    print(f"Successfully verified {image_path}")

except Exception as e:
    print(f"Error creating or verifying no_image.png: {e}")
