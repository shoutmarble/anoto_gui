# Quick Start Guide

## Installation

```bash
# Install dependencies
pip install -r requirements.txt

# For PDF support on Ubuntu/Debian
sudo apt-get install poppler-utils
```

## Basic Usage

### 1. Generate Sample Images

```bash
python generate_sample.py
```

This creates three sample dot pattern images that simulate Anoto dot paper.

### 2. Run the GUI Application

```bash
python anoto_gui.py
```

### 3. Test Computer Vision Functionality

```bash
python test_cv.py
```

### 4. Generate Visualization

```bash
python visualize_detection.py
```

## Features Demonstration

### Dot Pattern Detection
The application uses OpenCV to:
- Convert images to grayscale
- Apply binary thresholding
- Detect contours (dots)
- Calculate dot centroids
- Visualize results with green contours and red centroids

### Adjustable Parameters
- **Threshold**: Adjust binary threshold (0-255) to optimize dot detection
- **Min Dot Area**: Filter out noise by setting minimum dot size (1-50 pixels)

### Supported Formats
- **Images**: PNG, JPG, JPEG, BMP, TIFF
- **PDFs**: Automatically converts first page to image (requires poppler)

## Example Output

After running the test suite:
- Regular pattern: ~1131 dots detected
- Varied pattern: ~1159 dots detected
- Dense pattern: ~2098 dots detected

The visualization scripts create annotated images showing:
- Green outlines around detected dots
- Red dots at calculated centroids
- Detection count overlay
