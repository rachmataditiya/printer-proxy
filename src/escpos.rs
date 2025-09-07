use crate::errors::ProxyError;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};

/* ===================== JSON Job (ops optional) ===================== */

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum JsonJob {
    RawBase64 { base64: String },
    Ops { ops: Vec<PrintOp> },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PrintOp {
    #[serde(rename = "init")]
    Init,
    #[serde(rename = "text")]
    Text { data: String, newline: Option<bool> },
    #[serde(rename = "feed")]
    Feed { lines: u8 },
    #[serde(rename = "cut")]
    Cut { mode: Option<String> },
}

/* ===================== ESC/POS Helpers ===================== */

#[derive(Debug, Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    MsbFirst,
    LsbFirst,
}

pub fn esc_init(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[0x1B, 0x40]); // ESC @
}

pub fn esc_align(buf: &mut Vec<u8>, a: Align) {
    let n = match a {
        Align::Left => 0,
        Align::Center => 1,
        Align::Right => 2,
    };
    buf.extend_from_slice(&[0x1B, b'a', n]);
}

pub fn esc_text_line(buf: &mut Vec<u8>, s: &str, newline: bool) {
    buf.extend_from_slice(s.as_bytes());
    if newline {
        buf.push(b'\n');
    }
}

pub fn esc_feed(buf: &mut Vec<u8>, lines: u8) {
    buf.extend_from_slice(&[0x1B, 0x64, lines]); // ESC d n
}

pub fn esc_cut(buf: &mut Vec<u8>, partial: bool) {
    buf.extend_from_slice(&[0x1D, 0x56, if partial { 0x01 } else { 0x00 }]); // GS V m
}

/// GS v 0 m xL xH yL yH data
/// data = bitmap 1bpp, row-major, MSB=left (default ESC/POS)
pub fn esc_raster_image(
    buf: &mut Vec<u8>,
    width: u32,
    height: u32,
    data: &[u8],
    scale_m: u8,
) -> Result<(), ProxyError> {
    let x_bytes = ((width + 7) / 8) as usize;
    let expected = x_bytes * height as usize;
    if data.len() != expected {
        return Err(ProxyError::BadPayload(format!(
            "Ukuran data gambar tidak cocok (got {}, expected {} by bytes/row {})",
            data.len(),
            expected,
            x_bytes
        )));
    }
    let x_l = (x_bytes & 0xFF) as u8;
    let x_h = ((x_bytes >> 8) & 0xFF) as u8;
    let y_l = (height & 0xFF) as u8;
    let y_h = ((height >> 8) & 0xFF) as u8;

    buf.extend_from_slice(&[0x1D, 0x76, 0x30, scale_m, x_l, x_h, y_l, y_h]);
    buf.extend_from_slice(data);
    Ok(())
}

fn bit_reverse_byte(mut b: u8) -> u8 {
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    b
}

/// Apply optional invert and bit-order fix
fn transform_bitmap(
    mut data: Vec<u8>,
    invert: bool,
    bit_order: BitOrder,
) -> Vec<u8> {
    if invert {
        for b in &mut data {
            *b = !*b;
        }
    }
    if bit_order == BitOrder::LsbFirst {
        for b in &mut data {
            *b = bit_reverse_byte(*b);
        }
    }
    data
}

/* ===================== ePOS-Print SOAP Parsing ===================== */

#[derive(Debug, Clone)]
pub struct ImageSpec {
    pub width: u32,
    pub height: u32,
    pub align: Align,
    pub gap_lines: u8,     // feed setelah gambar
    pub scale_m: u8,       // 0:1x, 1:2w, 2:2h, 3:2x
    #[allow(dead_code)]
    pub invert: bool,      // invert bit
    #[allow(dead_code)]
    pub bit_order: BitOrder,
    pub bitmap: Vec<u8>,   // packed 1bpp
}

#[derive(Debug, Clone)]
pub struct EposDoc {
    pub images: Vec<ImageSpec>,
    pub cut: Option<String>, // "feed" / "full"/"partial"/...
}

fn parse_align(val: &str) -> Align {
    match val {
        v if v.eq_ignore_ascii_case("center") => Align::Center,
        v if v.eq_ignore_ascii_case("right") => Align::Right,
        _ => Align::Left,
    }
}

fn parse_scale(val: &str) -> u8 {
    match val.to_ascii_lowercase().as_str() {
        "2w" => 1,
        "2h" => 2,
        "2x" | "2" => 3,
        _ => 0,
    }
}

fn parse_bool(val: &str) -> bool {
    matches!(
        val.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

fn parse_bit_order(val: &str) -> BitOrder {
    if val.eq_ignore_ascii_case("lsb") || val.eq_ignore_ascii_case("lsb_first") {
        BitOrder::LsbFirst
    } else {
        BitOrder::MsbFirst
    }
}

/// Parse SOAP ePOS-Print menjadi EposDoc (multi-image + cut)
pub fn parse_epos_soap(
    body: &[u8],
    override_invert: Option<bool>,
    override_bit: Option<BitOrder>,
) -> Result<EposDoc, ProxyError> {
    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();

    let mut images: Vec<ImageSpec> = Vec::new();
    let mut collecting_image_text = false;
    let mut current_width: u32 = 0;
    let mut current_height: u32 = 0;
    let mut current_align = Align::Left;
    let mut current_gap: u8 = 0;
    let mut current_scale: u8 = 0;
    let mut current_invert = false;
    let mut current_bit = BitOrder::MsbFirst;
    let mut current_b64 = String::new();

    let mut cut: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_ascii_lowercase();
                if name.ends_with(b"image") {
                    collecting_image_text = true;
                    current_width = 0;
                    current_height = 0;
                    current_align = Align::Left;
                    current_gap = 0;
                    current_scale = 0;
                    current_invert = false;
                    current_bit = BitOrder::MsbFirst;
                    current_b64.clear();

                    for a in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(a.key.as_ref()).to_ascii_lowercase();
                        let val = a.unescape_value().unwrap_or_default().to_string();
                        match key.as_str() {
                            "width" => current_width = val.parse().unwrap_or(0),
                            "height" => current_height = val.parse().unwrap_or(0),
                            "align" => current_align = parse_align(&val),
                            "gap" => current_gap = val.parse().unwrap_or(0),
                            "scale" => current_scale = parse_scale(&val),
                            "invert" => current_invert = parse_bool(&val),
                            "bit_order" => current_bit = parse_bit_order(&val),
                            _ => {}
                        }
                    }
                } else if name.ends_with(b"cut") {
                    for a in e.attributes().flatten() {
                        if a.key.as_ref().eq_ignore_ascii_case(b"type") {
                            cut = Some(a.unescape_value().unwrap_or_default().to_string());
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if collecting_image_text {
                    current_b64.push_str(&t.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_ascii_lowercase();
                if name.ends_with(b"image") {
                    collecting_image_text = false;

                    if current_width == 0 || current_height == 0 || current_b64.is_empty() {
                        return Err(ProxyError::BadPayload(
                            "Elemen <image> tidak lengkap (width/height/base64)".into(),
                        ));
                    }

                    let cleaned: String = current_b64.chars().filter(|c| !c.is_whitespace()).collect();
                    
                    // Pre-allocate with estimated decoded size to avoid reallocations
                    let estimated_decoded_size = (cleaned.len() * 3) / 4; // Base64 decode ratio
                    let mut bitmap = Vec::with_capacity(estimated_decoded_size);
                    BASE64_STANDARD.decode_vec(cleaned.trim(), &mut bitmap).map_err(|e| {
                        ProxyError::BadPayload(format!("Base64 <image> invalid: {e}"))
                    })?;

                    let x_bytes = ((current_width + 7) / 8) as usize;
                    let expected = x_bytes * current_height as usize;
                    if bitmap.len() < expected {
                        let mut padded = Vec::with_capacity(expected);
                        padded.extend_from_slice(&bitmap);
                        padded.resize(expected, 0);
                        bitmap = padded;
                    } else if bitmap.len() > expected {
                        bitmap.truncate(expected);
                    }

                    let invert = override_invert.unwrap_or(current_invert);
                    let bit = override_bit.unwrap_or(current_bit);
                    let bitmap = transform_bitmap(bitmap, invert, bit);

                    images.push(ImageSpec {
                        width: current_width,
                        height: current_height,
                        align: current_align,
                        gap_lines: current_gap,
                        scale_m: current_scale,
                        invert,
                        bit_order: bit,
                        bitmap,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ProxyError::BadPayload(format!("XML parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    if images.is_empty() {
        return Err(ProxyError::BadPayload(
            "Payload ePOS tidak berisi <image>".into(),
        ));
    }

    Ok(EposDoc { images, cut })
}

/// Bangun ESC/POS dari EposDoc
pub fn build_escpos_from_epos_doc(doc: &EposDoc) -> Result<Vec<u8>, ProxyError> {
    // Pre-calculate total capacity needed for better memory allocation
    let total_bitmap_size: usize = doc.images.iter().map(|i| i.bitmap.len()).sum();
    let estimated_commands_size = doc.images.len() * 50; // ~50 bytes per image command overhead
    let mut out = Vec::with_capacity(1024 + total_bitmap_size + estimated_commands_size);
    esc_init(&mut out);

    for img in &doc.images {
        esc_align(&mut out, img.align);
        esc_raster_image(&mut out, img.width, img.height, &img.bitmap, img.scale_m)?;
        if img.gap_lines > 0 {
            esc_feed(&mut out, img.gap_lines);
        }
    }

    esc_align(&mut out, Align::Left);

    if let Some(t) = &doc.cut {
        if t.eq_ignore_ascii_case("feed") {
            esc_feed(&mut out, 3);
            esc_cut(&mut out, false);
        } else if t.eq_ignore_ascii_case("partial") {
            esc_cut(&mut out, true);
        } else {
            esc_cut(&mut out, false);
        }
    } else {
        // Auto-cut after image printing if no explicit cut command is provided
        esc_feed(&mut out, 3);
        esc_cut(&mut out, false);
    }

    Ok(out)
}

pub fn build_escpos_from_ops(ops: &[PrintOp]) -> Result<Vec<u8>, ProxyError> {
    // Better capacity estimation based on operation types
    let estimated_size = ops.iter().map(|op| match op {
        PrintOp::Init => 2,
        PrintOp::Text { data, .. } => data.len() + 1,
        PrintOp::Feed { .. } => 3,
        PrintOp::Cut { .. } => 3,
    }).sum::<usize>();
    let mut out = Vec::with_capacity(estimated_size.max(256));
    
    for op in ops {
        match op {
            PrintOp::Init => esc_init(&mut out),
            PrintOp::Text { data, newline } => esc_text_line(&mut out, data, newline.unwrap_or(true)),
            PrintOp::Feed { lines } => esc_feed(&mut out, *lines),
            PrintOp::Cut { mode } => {
                let partial = matches!(mode.as_deref(), Some("partial" | "PARTIAL" | "p"));
                esc_cut(&mut out, partial);
            }
        }
    }
    Ok(out)
}

// Re-export parsing utilities for use in handlers
pub fn parse_bool_public(val: &str) -> bool {
    parse_bool(val)
}

pub fn parse_bit_order_public(val: &str) -> BitOrder {
    parse_bit_order(val)
}
