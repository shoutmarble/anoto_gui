#!/usr/bin/env python3
"""
Generate a sample Anoto-like dot pattern for testing purposes.
This creates a simple dot pattern image that simulates the appearance of Anoto dot paper.
"""

import cv2
import numpy as np
import random


def generate_dot_pattern(width=800, height=600, dot_spacing=20, dot_size=3, noise_level=0.1):
    """
    Generate a sample dot pattern image similar to Anoto dot paper.
    
    Args:
        width: Image width in pixels
        height: Image height in pixels
        dot_spacing: Average spacing between dots in pixels
        dot_size: Radius of each dot in pixels
        noise_level: Random variation in dot positions (0-1)
    
    Returns:
        numpy array representing the generated image
    """
    # Create a white background
    image = np.ones((height, width, 3), dtype=np.uint8) * 255
    
    # Generate a grid of dots with slight random displacement
    for y in range(0, height, dot_spacing):
        for x in range(0, width, dot_spacing):
            # Add random displacement to simulate Anoto pattern
            offset_x = int(random.uniform(-dot_spacing * noise_level, dot_spacing * noise_level))
            offset_y = int(random.uniform(-dot_spacing * noise_level, dot_spacing * noise_level))
            
            dot_x = x + offset_x
            dot_y = y + offset_y
            
            # Ensure dots are within image bounds
            if 0 <= dot_x < width and 0 <= dot_y < height:
                # Draw a black dot
                cv2.circle(image, (dot_x, dot_y), dot_size, (0, 0, 0), -1)
    
    return image


def main():
    """Generate and save sample dot pattern images."""
    print("Generating sample Anoto-like dot pattern images...")
    
    # Generate different pattern variations
    patterns = [
        ("sample_pattern_regular.png", {"dot_spacing": 20, "dot_size": 2, "noise_level": 0.0}),
        ("sample_pattern_varied.png", {"dot_spacing": 20, "dot_size": 2, "noise_level": 0.3}),
        ("sample_pattern_dense.png", {"dot_spacing": 15, "dot_size": 2, "noise_level": 0.2}),
    ]
    
    for filename, params in patterns:
        image = generate_dot_pattern(**params)
        cv2.imwrite(filename, image)
        print(f"Generated: {filename}")
    
    print("\nSample images generated successfully!")
    print("You can load these images in the Anoto GUI to test dot detection.")


if __name__ == "__main__":
    main()
