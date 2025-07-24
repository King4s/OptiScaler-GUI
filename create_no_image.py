from PIL import Image

# Create a new black image (e.g., 60x80 pixels, same as resize target)
img = Image.new('RGB', (60, 80), color = 'black')

# Save the image as PNG
img.save("C:\\OptiScaler-GUI\\assets\\icons\\no_image.png")