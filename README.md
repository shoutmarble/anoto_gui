## Anoto Gui App

This is a GUI Application to read Anoto Dot Paper.
## Assets and Tests

- The app includes test assets located at `src/kornia/assets/anoto_dots2.png` used by unit tests and local experiments.
- Unit tests may output annotated images into `output/` (ignored by default) during test runs. A `.gitkeep` file in `output/.gitkeep` ensures the folder exists for test artifacts.
- To add more test input images, place an image file in `src/kornia/assets/` and update the tests under `tests/`.

## API usage: detecting and decoding grid

You can programmatically detect components, deskew the image and extract a simple ASCII grid from detected dots:

```rust
use anoto_dot_reader::kornia::anoto::{detect_components_from_image, detect_grid, AnotoConfig};
let img = image::open("src/kornia/assets/anoto_dots2.png")?;
let config = AnotoConfig::default();
let comps = detect_components_from_image(&img, &config)?;
if let Some((rows, cols, grid, origin)) = detect_grid(&comps, &config) {
	println!("Grid {}x{} origin={:?}\n{}", rows, cols, origin, grid);
}
```



![Anoto Gui](https://raw.githubusercontent.com/shoutmarble/anoto_gui/refs/heads/main/assets/anoto_gui.png)