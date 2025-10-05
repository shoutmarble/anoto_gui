# Anoto GUI - Computer Vision Test

Computer vision test application for Anoto PDF with Anoto dot paper. This application provides a graphical user interface to load, visualize, and analyze Anoto dot patterns from images and PDF files.

## Features

- **Load Images**: Support for PNG, JPG, JPEG, BMP, and TIFF formats
- **Load PDFs**: Convert PDF pages to images for analysis
- **Dot Detection**: Automatically detect and highlight dot patterns
- **Interactive Parameters**: Adjust threshold and minimum dot area in real-time
- **Visual Comparison**: Side-by-side view of original and processed images

## Requirements

- Python 3.7 or higher
- OpenCV (opencv-python)
- NumPy
- Pillow
- pdf2image
- tkinter (usually included with Python)

For PDF support, you also need:
- poppler-utils (system package)

## Installation

1. Clone this repository:
```bash
git clone https://github.com/shoutmarble/anoto_gui.git
cd anoto_gui
```

2. Install Python dependencies:
```bash
pip install -r requirements.txt
```

3. Install system dependencies (for PDF support):

**Ubuntu/Debian:**
```bash
sudo apt-get install poppler-utils
```

**macOS:**
```bash
brew install poppler
```

**Windows:**
Download and install poppler from: https://github.com/oschwartz10612/poppler-windows/releases

## Usage

### Running the GUI Application

```bash
python anoto_gui.py
```

### Generating Sample Test Images

To create sample dot pattern images for testing:

```bash
python generate_sample.py
```

This will generate three sample images:
- `sample_pattern_regular.png` - Regular grid pattern
- `sample_pattern_varied.png` - Pattern with position variations
- `sample_pattern_dense.png` - Denser dot pattern

### Using the Application

1. **Load an Image or PDF**:
   - Click "Load Image" to load an image file
   - Click "Load PDF" to load a PDF file (first page will be converted)

2. **Detect Dots**:
   - Click "Detect Dots" to analyze the loaded image
   - Detected dots will be highlighted with green contours
   - Dot centroids are marked with red points

3. **Adjust Parameters**:
   - Use the "Threshold" slider to adjust binary threshold (0-255)
   - Use the "Min Dot Area" slider to filter small noise (1-50 pixels)
   - Click "Detect Dots" again to reprocess with new parameters

4. **Reset View**:
   - Click "Reset" to clear the processed image view

## About Anoto Technology

Anoto dot paper uses a pattern of nearly invisible microdots to encode position information. Each position on the paper has a unique dot pattern that can be read by special digital pens or cameras. This application provides a simple computer vision approach to detect these dot patterns.

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Author

shoutmarble
