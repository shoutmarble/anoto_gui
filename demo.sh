#!/bin/bash
# Demo script for Anoto GUI

echo "======================================"
echo "Anoto GUI - Demo Script"
echo "======================================"
echo ""

# Check if dependencies are installed
echo "Checking dependencies..."
if ! python -c "import cv2" 2>/dev/null; then
    echo "❌ OpenCV not installed. Installing dependencies..."
    pip install -r requirements.txt
else
    echo "✓ Dependencies OK"
fi
echo ""

# Generate sample images
echo "Generating sample dot pattern images..."
python generate_sample.py
echo ""

# Run tests
echo "Running computer vision tests..."
python test_cv.py
echo ""

# Generate visualizations
echo "Generating visualizations..."
python visualize_detection.py
echo ""

echo "======================================"
echo "Demo complete!"
echo "======================================"
echo ""
echo "Generated files:"
ls -lh sample_pattern*.png 2>/dev/null | awk '{print "  - " $9}'
echo ""
echo "To run the GUI application:"
echo "  python anoto_gui.py"
echo ""
