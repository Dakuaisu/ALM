use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::capture::EXPECTED_HTML;

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    text.split_once(start).unwrap().1.split_once(end).unwrap().0
}

fn rows(html: &str) -> Vec<Vec<String>> {
    between(html, "<tbody>", "</tbody>")
        .split("<tr>")
        .skip(1)
        .map(|row| {
            row.split("<td")
                .skip(1)
                .map(|cell| {
                    between(cell, ">", "</td>")
                        .replace("<b>", "")
                        .replace("</b>", "")
                })
                .collect()
        })
        .collect()
}

fn svg_from_html(html: &str) -> String {
    let business = between(
        html,
        "font-size:24px;font-weight:800;color:#1d4ed8\">",
        "</div>",
    );
    let address = between(html, "overflow-wrap:break-word\">", "</div>");
    let mobile = between(html, "font-size:12px;font-weight:700\">", "</div>");
    let number = between(html, "font-size:13px;margin-top:8px\"><b>", "</b>");
    let date = between(html, "<b>Date:</b> ", "</div>");
    let supplier = between(html, "To: <b>", "</b>");
    let total = between(
        html,
        "style=\"text-align:right\">Total</td><td class=\"r\">",
        "</td>",
    );
    let note = between(html, "font-size:12px;color:#333\">", "</div>");
    let rows = rows(html);
    let [a, b] = rows.as_slice() else {
        panic!("expected two HTML order rows")
    };
    let [a_no, a_item, a_pack, a_mrp, a_qty] = a.as_slice() else {
        panic!("expected five cells")
    };
    let [b_no, b_item, b_pack, b_mrp, b_qty] = b.as_slice() else {
        panic!("expected five cells")
    };
    format!(
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"760\" height=\"760\" viewBox=\"0 0 760 760\">\n",
            "<rect width=\"760\" height=\"760\" fill=\"white\"/>\n",
            "<rect x=\"20\" y=\"20\" width=\"720\" height=\"610\" rx=\"6\" fill=\"white\" stroke=\"#334155\" stroke-width=\"1.5\"/>\n",
            "<g font-family=\"Arial,sans-serif\" fill=\"black\">\n",
            "<text x=\"50\" y=\"65\" font-size=\"24\" font-weight=\"bold\" fill=\"#1d4ed8\">{business}</text>\n",
            "<text x=\"50\" y=\"87\" font-size=\"12\">{address}</text>\n",
            "<text x=\"50\" y=\"105\" font-size=\"12\" font-weight=\"bold\">{mobile}</text>\n",
            "<text x=\"710\" y=\"65\" text-anchor=\"end\" font-size=\"18\" font-weight=\"bold\" fill=\"#1d4ed8\">PURCHASE ORDER</text>\n",
            "<text x=\"710\" y=\"87\" text-anchor=\"end\" font-size=\"13\">{number}</text>\n",
            "<text x=\"710\" y=\"105\" text-anchor=\"end\" font-size=\"13\">Date: {date}</text>\n",
            "<path d=\"M50 120H710\" stroke=\"#2563eb\" stroke-width=\"3\"/>\n",
            "<text x=\"50\" y=\"150\" font-size=\"14\">To: {supplier}</text>\n",
            "<path d=\"M50 165H710V305H50Z M50 200H710 M50 235H710 M50 270H710 M90 165V270 M330 165V270 M480 165V270 M590 165V305\" fill=\"none\" stroke=\"#8b97a8\"/>\n",
            "<g font-size=\"14\"><text x=\"65\" y=\"188\">#</text><text x=\"100\" y=\"188\">Item</text><text x=\"340\" y=\"188\">Pack</text><text x=\"490\" y=\"188\">MRP</text><text x=\"600\" y=\"188\">Qty (cases)</text>\n",
            "<text x=\"65\" y=\"223\">{a_no}</text><text x=\"100\" y=\"223\">{a_item}</text><text x=\"340\" y=\"223\">{a_pack}</text><text x=\"490\" y=\"223\">{a_mrp}</text><text x=\"600\" y=\"223\">{a_qty}</text>\n",
            "<text x=\"65\" y=\"258\">{b_no}</text><text x=\"100\" y=\"258\">{b_item}</text><text x=\"340\" y=\"258\">{b_pack}</text><text x=\"490\" y=\"258\">{b_mrp}</text><text x=\"600\" y=\"258\">{b_qty}</text>\n",
            "<text x=\"530\" y=\"293\">Total</text><text x=\"600\" y=\"293\">{total}</text></g>\n",
            "<text x=\"50\" y=\"340\" font-size=\"12\">{note}</text>\n",
            "<text x=\"550\" y=\"400\" font-size=\"11\">For {business}</text>\n",
            "<text x=\"550\" y=\"435\" font-size=\"11\">______________________</text>\n",
            "<text x=\"550\" y=\"455\" font-size=\"11\">Authorised Signatory</text>\n",
            "</g>\n</svg>\n"
        ),
        business = business,
        address = address,
        mobile = mobile,
        number = number,
        date = date,
        supplier = supplier,
        total = total,
        note = note,
        a_no = a_no,
        a_item = a_item,
        a_pack = a_pack,
        a_mrp = a_mrp,
        a_qty = a_qty,
        b_no = b_no,
        b_item = b_item,
        b_pack = b_pack,
        b_mrp = b_mrp,
        b_qty = b_qty
    )
}

fn run(mut command: Command, dir: &Path) {
    let result = command
        .env("TMPDIR", dir)
        .env("HOME", dir)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "local image tool failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn validate_png(png: &[u8], dir: &Path, path: &Path) {
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(&png[16..24], &[0, 0, 2, 248, 0, 0, 2, 248]);
    let decoded = dir.join("decoded.bmp");
    let mut command = Command::new("sips");
    command
        .args(["-s", "format", "bmp"])
        .arg(path)
        .arg("--out")
        .arg(&decoded);
    run(command, dir);
    let bmp = std::fs::read(&decoded).unwrap();
    assert_eq!(&bmp[..2], b"BM");
    let word = |offset| u32::from_le_bytes(bmp[offset..offset + 4].try_into().unwrap());
    let height = i32::from_le_bytes(bmp[22..26].try_into().unwrap());
    assert_eq!((word(18), height.unsigned_abs()), (760, 760));
    let depth = u16::from_le_bytes(bmp[28..30].try_into().unwrap());
    assert!(
        depth == 24 || depth == 32,
        "unexpected decoded bitmap depth {depth}"
    );
    match word(30) {
        0 => {}
        3 => assert_eq!((word(54), word(58), word(62)), (0xff0000, 0xff00, 0xff)),
        compression => panic!("unexpected decoded bitmap compression {compression}"),
    }
    let bytes_per_pixel = usize::from(depth / 8);
    let stride = (760 * bytes_per_pixel + 3) & !3;
    let offset = usize::try_from(word(10)).unwrap();
    let pixels = &bmp[offset..offset + stride * 760];
    let nonwhite = pixels
        .chunks_exact(bytes_per_pixel)
        .filter(|pixel| pixel[..3] != [255, 255, 255])
        .count();
    assert!(nonwhite > 760, "decoded image is blank");
    let region_has_ink = |(left, right, top, bottom): (usize, usize, usize, usize)| {
        (top..bottom).any(|y| {
            let row = if height < 0 { y } else { 759 - y };
            (left..right).any(|x| {
                let start = row * stride + x * bytes_per_pixel;
                pixels[start..start + 3]
                    .iter()
                    .any(|channel| *channel < 200)
            })
        })
    };
    for (label, region) in [
        ("first MRP", (490, 550, 207, 229)),
        ("first quantity", (600, 650, 207, 229)),
        ("second MRP", (490, 550, 242, 264)),
        ("second quantity", (600, 650, 242, 264)),
    ] {
        assert!(region_has_ink(region), "decoded image lacks {label}");
    }
    std::fs::remove_file(decoded).unwrap();
}

pub(super) fn render(html: &str, width: u32) -> (PathBuf, Vec<u8>) {
    assert_eq!(width, 760);
    assert_eq!(html, EXPECTED_HTML.trim_end_matches('\n'));
    let svg = svg_from_html(html);
    assert_eq!(svg, include_str!("expected_supplier.svg"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = root.join(format!("l07-03-po-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let svg_path = dir.join("supplier.svg");
    std::fs::write(dir.join("supplier.html"), html).unwrap();
    std::fs::write(&svg_path, svg).unwrap();
    let mut command = Command::new("qlmanage");
    command
        .args(["-t", "-s", "760", "-o"])
        .arg(&dir)
        .arg(&svg_path);
    run(command, &dir);
    let png_path = dir.join("supplier.svg.png");
    let png = std::fs::read(&png_path).expect("Quick Look must produce a new PNG");
    validate_png(&png, &dir, &png_path);
    (dir, png)
}
