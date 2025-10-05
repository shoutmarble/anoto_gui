#!/usr/bin/env python3
"""
Create visual demonstration of dot detection for documentation.
"""

import cv2
import numpy as np
import os


def visualize_detection(input_file, output_file):
    """Create a visualization showing dot detection results."""
    # Load image
    image = cv2.imread(input_file)
    if image is None:
        print(f"Failed to load {input_file}")
        return False
    
    # Convert to grayscale
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    
    # Apply threshold
    _, binary = cv2.threshold(gray, 127, 255, cv2.THRESH_BINARY_INV)
    
    # Find contours (dots)
    contours, _ = cv2.findContours(binary, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    
    # Filter contours by area
    min_area = 5
    filtered_contours = [cnt for cnt in contours if cv2.contourArea(cnt) >= min_area]
    
    # Create result image
    result = image.copy()
    
    # Draw contours in green
    cv2.drawContours(result, filtered_contours, -1, (0, 255, 0), 1)
    
    # Draw centroids in red
    for cnt in filtered_contours:
        M = cv2.moments(cnt)
        if M['m00'] != 0:
            cx = int(M['m10'] / M['m00'])
            cy = int(M['m01'] / M['m00'])
            cv2.circle(result, (cx, cy), 2, (0, 0, 255), -1)
    
    # Add text overlay
    text = f"Detected: {len(filtered_contours)} dots"
    cv2.putText(result, text, (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 
                1, (255, 0, 0), 2, cv2.LINE_AA)
    
    # Save result
    cv2.imwrite(output_file, result)
    print(f"Created visualization: {output_file}")
    return True


def main():
    """Generate visualization for each sample."""
    print("Generating visualizations...")
    
    samples = [
        "sample_pattern_regular.png",
        "sample_pattern_varied.png",
        "sample_pattern_dense.png"
    ]
    
    for sample in samples:
        if os.path.exists(sample):
            base_name = sample.replace('.png', '')
            output = f"{base_name}_detected.png"
            visualize_detection(sample, output)
    
    print("\nVisualization complete!")


if __name__ == "__main__":
    main()
