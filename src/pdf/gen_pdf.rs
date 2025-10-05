use kornia::imgproc::draw;
use oxidize_pdf::{Document, Page, Font, Color, Result};
use serde::Deserialize;
use serde_json;
use std::fs;

#[derive(Deserialize)]
struct BitMatrix {
    data: Vec<Vec<[u8; 2]>>,
}
enum anoto_dot {
    Up,
    Down,
    Left,
    Right,
}

pub fn gen_anoto_pdf() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("My First PDF");
    doc.set_author("Rust Developer");
    
    // Create a page
    let mut page = Page::a4();
    
    let page_width = page.width();
    let page_height = page.height();
    
    let json_str = fs::read_to_string("src/pdf/bitmatrix.json").unwrap();
    let matrix: BitMatrix = serde_json::from_str(&json_str).unwrap();
    //         [0, 0]                   UP
    //           ^                      ^
    //           |                      |
    // [1, 0] <--+--> [0, 1]    LEFT <--+--> RIGHT 
    //           |                      |
    //           v                      v
    //         [1, 1]                 DOWN
 
 
    for (i, row) in matrix.data.iter().enumerate() {
        for (j, cell) in row.iter().enumerate() {
            let x = j as f64 * 10.0;
            let y = i as f64 * 10.0;
            if cell[0] == 0 && cell[1] == 0 {
                draw_anoto_dot(&mut page, x, y, anoto_dot::Up);
            } else if cell[0] == 1 && cell[1] == 0 {
                draw_anoto_dot(&mut page, x, y, anoto_dot::Left);
            } else if cell[0] == 0 && cell[1] == 1 {
                draw_anoto_dot(&mut page, x, y, anoto_dot::Right);
            } else if cell[0] == 1 && cell[1] == 1 {
                draw_anoto_dot(&mut page, x, y, anoto_dot::Down);
            }
        }
    }

    println!("page width={} height={}", page_width as i32, page_height);
    println!("number anoto  dotts width={} height={}", (page_width/10.0) as i32, (page_height/10.0) as i32);
   
    // Add the page and save
    doc.add_page(page);
    doc.save("anoto.pdf")?;
    
    Ok(())
}


fn gen_all_dots_anoto_pdf() -> Result<()> {
    // Create a new document
    let mut doc = Document::new();
    doc.set_title("My First PDF");
    doc.set_author("Rust Developer");
    
    // Create a page
    let mut page = Page::a4();
    
    let page_width = page.width();
    let page_height = page.height();
    for x in (0..page_width as u32).step_by(10) {

        for y in (0..page_height as u32).step_by(10) {

            draw_anoto_dot(&mut page, x as f64, y as f64, anoto_dot::Up);
            draw_anoto_dot(&mut page, x as f64, y as f64, anoto_dot::Down);
            draw_anoto_dot(&mut page, x as f64, y as f64, anoto_dot::Left);
            draw_anoto_dot(&mut page, x as f64, y as f64, anoto_dot::Right);

            // draw_grid_lines(&mut page, 10.0);


        }
    }

    println!("page width={} height={}", page_width as i32, page_height);
    println!("number anoto  dotts width={} height={}", (page_width/10.0) as i32, (page_height/10.0) as i32);
   
    // Add the page and save
    doc.add_page(page);
    doc.save("anoto.pdf")?;
    
    Ok(())
}

fn draw_grid_lines(page: &mut Page, spacing: f64) {
    let page_width = page.width();
    let page_height = page.height();

    // Draw horizontal lines
    for y in (0..page_height as u32).step_by(spacing as usize) {
        page.graphics()
            .set_opacity(1.0)
            .set_stroke_color(Color::Gray(0.5))
            .set_line_width(0.5)
            .move_to(0.0, y as f64)
            .line_to(page_width, y as f64)
            .stroke();
    }

    // Draw vertical lines
    for x in (0..page_width as u32).step_by(spacing as usize) {
        page.graphics()
            .set_opacity(1.0)
            .set_stroke_color(Color::Gray(0.5))
            .set_line_width(0.5)
            .move_to(x as f64, 0.0)
            .line_to(x as f64, page_height)
            .stroke();
    }
}

fn draw_anoto_dot(page: &mut Page, x: f64, y: f64, direction: anoto_dot) {

    let radius = 1.0;

    match direction {
        anoto_dot::Up => {
            let y_up = y + 3.0;
            page.graphics()
                .set_fill_color(Color::blue())
                .circle(x, y_up, radius)
                .fill();
        },
        anoto_dot::Down => {
            let y_down = y - 3.0;
            page.graphics()
                .set_fill_color(Color::black())
                .circle(x, y_down, radius)
                .fill();
        },
        anoto_dot::Left => {
            let x_left = x - 3.0;
            page.graphics()
                .set_fill_color(Color::red())
                .circle(x_left, y, radius)
                .fill();
        },
        anoto_dot::Right => {
            let x_right = x + 3.0;
            page.graphics()
                .set_fill_color(Color::magenta())
                .circle(x_right, y, radius)
                .fill();
        },

    }

}

