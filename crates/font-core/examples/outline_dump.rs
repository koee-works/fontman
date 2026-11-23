// Prints one face's metrics and a few glyphs' spacing, to compare with
// fontTools. Run: cargo run -p font-core --example outline_dump -- <font> <text>
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("font path");
    let text = args.next().unwrap_or_else(|| "Hg".into());
    let data = std::fs::read(&path).unwrap();
    let face = ttf_parser::Face::parse(&data, 0).unwrap();
    let o = font_core::outline::outlines_of(&face, &text);
    println!("{:?}", o.metrics);
    for g in o.glyphs {
        println!("{} {:?} adv {} lsb {} bbox {:?} on {} off {}", g.ch, g.name, g.advance, g.lsb, g.bbox, g.on_curve.len(), g.off_curve.len());
    }
}
