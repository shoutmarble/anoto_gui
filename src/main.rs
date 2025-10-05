mod pdf;
mod kornia;
mod ice;
fn main() {
    println!("dots!");
    // Call the PDF generator
    pdf::gen_anoto_pdf();
    // kornia::image_proc().unwrap();
    // ice::editor::my_counter().unwrap();
}
