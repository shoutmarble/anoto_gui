#!/usr/bin/env python3
"""
Test script for the Anoto GUI application.
Tests core computer vision functionality without requiring GUI.
"""

import cv2
import numpy as np
import os


def test_dot_detection():
    """Test the dot detection algorithm on generated sample images."""
    print("Testing dot detection functionality...")
    
    sample_files = [
        "sample_pattern_regular.png",
        "sample_pattern_varied.png",
        "sample_pattern_dense.png"
    ]
    
    for filename in sample_files:
        if not os.path.exists(filename):
            print(f"  ⚠ Warning: {filename} not found. Run generate_sample.py first.")
            continue
            
        print(f"\n  Testing: {filename}")
        
        # Load image
        image = cv2.imread(filename)
        if image is None:
            print(f"    ✗ Failed to load image")
            continue
        
        print(f"    ✓ Loaded image: {image.shape[1]}x{image.shape[0]} pixels")
        
        # Convert to grayscale
        gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
        print(f"    ✓ Converted to grayscale")
        
        # Apply threshold
        threshold_value = 127
        _, binary = cv2.threshold(gray, threshold_value, 255, cv2.THRESH_BINARY_INV)
        print(f"    ✓ Applied threshold: {threshold_value}")
        
        # Find contours (dots)
        contours, _ = cv2.findContours(binary, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        print(f"    ✓ Found {len(contours)} contours")
        
        # Filter contours by area
        min_area = 5
        filtered_contours = [cnt for cnt in contours if cv2.contourArea(cnt) >= min_area]
        print(f"    ✓ Filtered to {len(filtered_contours)} dots (min area: {min_area})")
        
        # Calculate centroids
        centroids = []
        for cnt in filtered_contours[:10]:  # Just test first 10
            M = cv2.moments(cnt)
            if M['m00'] != 0:
                cx = int(M['m10'] / M['m00'])
                cy = int(M['m01'] / M['m00'])
                centroids.append((cx, cy))
        
        print(f"    ✓ Calculated {len(centroids)} centroids (sample)")
        
        if len(filtered_contours) > 0:
            print(f"    ✅ Test passed: Successfully detected dots in {filename}")
        else:
            print(f"    ✗ Test failed: No dots detected in {filename}")


def test_image_processing():
    """Test basic image processing operations."""
    print("\nTesting image processing operations...")
    
    # Create a simple test image
    test_image = np.ones((100, 100, 3), dtype=np.uint8) * 255
    
    # Draw some test dots
    cv2.circle(test_image, (20, 20), 3, (0, 0, 0), -1)
    cv2.circle(test_image, (50, 50), 3, (0, 0, 0), -1)
    cv2.circle(test_image, (80, 80), 3, (0, 0, 0), -1)
    
    print("  ✓ Created test image with 3 dots")
    
    # Test grayscale conversion
    gray = cv2.cvtColor(test_image, cv2.COLOR_BGR2GRAY)
    assert gray.shape == (100, 100), "Grayscale conversion failed"
    print("  ✓ Grayscale conversion works")
    
    # Test thresholding
    _, binary = cv2.threshold(gray, 127, 255, cv2.THRESH_BINARY_INV)
    assert binary.shape == (100, 100), "Thresholding failed"
    print("  ✓ Thresholding works")
    
    # Test contour detection
    contours, _ = cv2.findContours(binary, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    assert len(contours) == 3, f"Expected 3 contours, found {len(contours)}"
    print("  ✓ Contour detection works")
    
    print("  ✅ All image processing tests passed")


def main():
    """Run all tests."""
    print("=" * 60)
    print("Anoto GUI - Computer Vision Tests")
    print("=" * 60)
    
    test_image_processing()
    test_dot_detection()
    
    print("\n" + "=" * 60)
    print("Testing complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
