#!/usr/bin/env python3
"""
Verify all test images contain the expected Anoto pattern
"""
import json
from pathlib import Path

# Expected pattern from the user
EXPECTED_PATTERN = [
    ["↑", "↓", "←", "↓", "↓", "←", "↓"],
    ["→", "↑", "↑", "←", "↑", "↓", "↓"],
    ["↓", "↓", "→", "↓", "↑", "→", "←"],
    ["↓", "←", "↑", "←", "↓", "↑", "→"],
    ["←", "→", "↓", "←", "→", "→", "↑"],
    ["↓", "↑", "→", "↑", "→", "↓", "↓"],
    ["↓", "↓", "←", "←", "→", "←", "→"],
    ["↑", "↓", "↓", "→", "→", "→", "←"]
]

def rotate_90(grid):
    """Rotate grid 90 degrees clockwise"""
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    rotated = [[' ' for _ in range(h)] for _ in range(w)]
    for r in range(h):
        for c in range(w):
            rotated[c][h - 1 - r] = grid[r][c]
    return rotated

def flip_horizontal(grid):
    """Flip grid horizontally"""
    return [row[::-1] for row in grid]

def flip_vertical(grid):
    """Flip grid vertically"""
    return grid[::-1]

def all_transformations(grid):
    """Generate all 8 transformations (rotations and flips)"""
    transforms = []
    current = grid
    
    # Original and 3 rotations
    for _ in range(4):
        transforms.append(current)
        current = rotate_90(current)
    
    # Flipped and 3 rotations
    current = flip_horizontal(grid)
    for _ in range(4):
        transforms.append(current)
        current = rotate_90(current)
    
    return transforms

def find_pattern_in_grid(grid, pattern):
    """Find pattern in grid with all transformations, returns (row, col, transform) if found, None otherwise"""
    pattern_h = len(pattern)
    pattern_w = len(pattern[0])
    grid_h = len(grid)
    grid_w = len(grid[0]) if grid_h > 0 else 0
    
    # Try all transformations of the pattern
    for transform_idx, transformed_pattern in enumerate(all_transformations(pattern)):
        tp_h = len(transformed_pattern)
        tp_w = len(transformed_pattern[0]) if tp_h > 0 else 0
        
        if grid_h < tp_h or grid_w < tp_w:
            continue
        
        for row in range(grid_h - tp_h + 1):
            for col in range(grid_w - tp_w + 1):
                # Check if pattern matches at this position
                match = True
                for pr in range(tp_h):
                    for pc in range(tp_w):
                        grid_val = grid[row + pr][col + pc]
                        pattern_val = transformed_pattern[pr][pc]
                        # Skip spaces in grid
                        if grid_val == ' ':
                            match = False
                            break
                        if grid_val != pattern_val:
                            match = False
                            break
                    if not match:
                        break
                if match:
                    transform_names = [
                        "Identity", "Rot90", "Rot180", "Rot270",
                        "FlipH", "FlipH+Rot90", "FlipH+Rot180", "FlipH+Rot270"
                    ]
                    return (row, col, transform_names[transform_idx], transformed_pattern)
    return None

def main():
    # Load all minified JSON files
    minified_files = sorted(Path('.').glob('anoto_*_minified.json'))
    
    print("Verifying Anoto patterns in test images...")
    print("=" * 80)
    print(f"\nExpected pattern ({len(EXPECTED_PATTERN)}x{len(EXPECTED_PATTERN[0])}):")
    for row in EXPECTED_PATTERN:
        print("  " + " ".join(row))
    print("\n" + "=" * 80)
    
    found_count = 0
    not_found = []
    
    for minified_file in minified_files:
        # Extract image number from filename
        img_num = minified_file.stem.split('_')[1]
        
        # Load the minified grid
        with open(minified_file, 'r', encoding='utf-8') as f:
            grid = json.load(f)
        
        # Find the pattern
        result = find_pattern_in_grid(grid, EXPECTED_PATTERN)
        
        if result:
            row, col, transform, transformed_pattern = result
            found_count += 1
            print(f"\n✓ Image {img_num}: FOUND at position (row={row}, col={col})")
            print(f"  File: {minified_file}")
            print(f"  Transform: {transform}")
            if transform != "Identity":
                print(f"  Transformed pattern:")
                for r in transformed_pattern:
                    print(f"    {' '.join(r)}")
        else:
            not_found.append(img_num)
            print(f"\n✗ Image {img_num}: NOT FOUND")
            print(f"  File: {minified_file}")
            print(f"  Grid size: {len(grid)}x{len(grid[0]) if grid else 0}")
    
    print("\n" + "=" * 80)
    print(f"\nSummary:")
    print(f"  Total images processed: {len(minified_files)}")
    print(f"  Patterns found: {found_count}")
    print(f"  Patterns not found: {len(not_found)}")
    
    if not_found:
        print(f"\n  Images without pattern: {', '.join(not_found)}")
    else:
        print("\n  ✓ All images contain the expected pattern!")

if __name__ == "__main__":
    main()
