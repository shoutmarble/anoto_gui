#!/usr/bin/env python3
"""
Debug: Check what the pattern looks like in file 2
"""
import json

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

with open('anoto_2_minified.json', 'r', encoding='utf-8') as f:
    grid = json.load(f)

print("Expected pattern (8x7):")
for row in EXPECTED_PATTERN:
    print("  " + " ".join(row))

print("\nActual grid from anoto_2_minified.json (8x7):")
for row in grid:
    print("  " + " ".join(row))

print("\nDifferences:")
for r in range(len(EXPECTED_PATTERN)):
    for c in range(len(EXPECTED_PATTERN[0])):
        if EXPECTED_PATTERN[r][c] != grid[r][c]:
            print(f"  [{r},{c}]: expected '{EXPECTED_PATTERN[r][c]}' but got '{grid[r][c]}'")
