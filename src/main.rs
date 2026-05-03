use axum::{
    extract::Form,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chrono::Local;
use lynpdf_rs::{render_html_to_pdf, RenderOptions, RenderRequest};
use serde::Deserialize;
use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, Deserialize)]
struct CheckoutForm {
    customer_name: String,
    product_name: String,
    quantity: u32,
    unit_price: f64,
    note: Option<String>,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index_page))
        .route("/receipt", post(generate_receipt_pdf));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("lynpdf-rs-example running at http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("server stopped unexpectedly");
}

async fn index_page() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn generate_receipt_pdf(Form(form): Form<CheckoutForm>) -> impl IntoResponse {
    match render_receipt_document(&form) {
        Ok((filename, bytes)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            );

            if let Ok(disposition) =
                HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            {
                headers.insert(header::CONTENT_DISPOSITION, disposition);
            }

            (headers, bytes).into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to create receipt PDF: {err}"),
        )
            .into_response(),
    }
}

fn render_receipt_document(form: &CheckoutForm) -> Result<(String, Vec<u8>), String> {
    let quantity = form.quantity.max(1);
    let unit_price = if form.unit_price.is_sign_negative() {
        0.0
    } else {
        form.unit_price
    };

    let subtotal = unit_price * quantity as f64;
    let vat = subtotal * 0.07;
    let total = subtotal + vat;

    let now = Local::now();
    let invoice_number = format!("INV-{}", now.format("%Y%m%d-%H%M%S"));
    let issue_date = now.format("%Y-%m-%d %H:%M").to_string();

    let html = build_invoice_html(
        &invoice_number,
        &issue_date,
        &form.customer_name,
        &form.product_name,
        quantity,
        unit_price,
        subtotal,
        vat,
        total,
        form.note.as_deref().unwrap_or(""),
    );

    let base_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut options = RenderOptions::default();
    if let Ok(font_dir) = env::var("LYNPDF_RS_EXAMPLE_FONT_DIR") {
        let trimmed = font_dir.trim();
        if !trimmed.is_empty() {
            options = options.with_user_font_dir(trimmed);
        }
    }

    let rendered = render_html_to_pdf(RenderRequest {
        html,
        css: String::new(),
        base_dir,
        css_base_dir: None,
        options,
    })
    .map_err(|err| err.to_string())?;

    Ok((
      format!("receipt-th-{}.pdf", invoice_number.to_lowercase()),
      rendered.bytes,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_invoice_html(
    invoice_number: &str,
    issue_date: &str,
    customer_name: &str,
    product_name: &str,
    quantity: u32,
    unit_price: f64,
    subtotal: f64,
    vat: f64,
    total: f64,
    note: &str,
) -> String {
    let safe_customer_name = escape_html(customer_name);
    let safe_product_name = escape_html(product_name);
    let safe_note = escape_html(note);

    format!(
        r#"<!doctype html>
<html lang="th">
<head>
  <meta charset="utf-8" />
  <title>ใบเสร็จรับเงิน {invoice_number}</title>
  <style>
    @page {{ size: A4; margin: 22mm; }}
    body {{
      font-family: "Sarabun", "Noto Sans Thai", sans-serif;
      color: #162227;
      font-size: 12pt;
      line-height: 1.5;
    }}
    .hero {{
      padding: 14px 18px;
      border-radius: 10px;
      border: 1px solid #8eb6c0;
      background: #edf8fb;
      margin-bottom: 18px;
    }}
    .title {{
      font-size: 21pt;
      font-weight: 700;
      margin: 0;
      color: #0d4a5b;
    }}
    .muted {{ color: #54757e; }}
    .meta {{ margin-top: 8px; }}
    .meta td {{ padding: 2px 0; }}
    .section-title {{
      margin: 18px 0 8px;
      font-size: 13pt;
      font-weight: 700;
      color: #0f5c70;
    }}
    table.items {{ width: 100%; border-collapse: collapse; margin-top: 6px; }}
    table.items th, table.items td {{ border: 1px solid #b3d0d8; padding: 8px; }}
    table.items th {{ background: #e2f3f8; text-align: left; }}
    .num {{ text-align: right; }}
    .summary {{ width: 48%; margin-left: auto; margin-top: 14px; border-collapse: collapse; }}
    .summary td {{ border: 1px solid #b3d0d8; padding: 8px; }}
    .summary .grand {{ font-weight: 700; background: #e9f7ec; }}
    .note {{ margin-top: 18px; padding: 10px 12px; border-left: 4px solid #73a8b6; background: #f4fbfd; }}
    .footer {{ margin-top: 26px; font-size: 10pt; color: #6f8890; }}
  </style>
</head>
<body>
  <div class="hero">
    <p class="title">ใบเสร็จรับเงิน / ใบกำกับภาษี</p>
    <p class="muted">ร้านตัวอย่าง LynPDF RS</p>
  </div>

  <table class="meta">
    <tr><td><strong>เลขที่เอกสาร:</strong></td><td>{invoice_number}</td></tr>
    <tr><td><strong>วันที่ออกเอกสาร:</strong></td><td>{issue_date}</td></tr>
    <tr><td><strong>ลูกค้า:</strong></td><td>{safe_customer_name}</td></tr>
  </table>

  <p class="section-title">รายการสินค้า</p>
  <table class="items">
    <thead>
      <tr>
        <th style="width: 44%;">สินค้า</th>
        <th style="width: 16%;">จำนวน</th>
        <th style="width: 20%;" class="num">ราคาต่อหน่วย (บาท)</th>
        <th style="width: 20%;" class="num">จำนวนเงิน (บาท)</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{safe_product_name}</td>
        <td>{quantity}</td>
        <td class="num">{unit_price:.2}</td>
        <td class="num">{subtotal:.2}</td>
      </tr>
    </tbody>
  </table>

  <table class="summary">
    <tr><td>รวมก่อนภาษี</td><td class="num">{subtotal:.2}</td></tr>
    <tr><td>ภาษีมูลค่าเพิ่ม 7%</td><td class="num">{vat:.2}</td></tr>
    <tr class="grand"><td>ยอดรวมสุทธิ</td><td class="num">{total:.2}</td></tr>
  </table>

  <div class="note">
    <strong>หมายเหตุ:</strong> {safe_note}
  </div>

  <p class="footer">สร้างเอกสารด้วย lynpdf-rs-example ผ่าน lynpdf-rs Rust API</p>
</body>
</html>"#
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="th">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>lynpdf-rs-example | ร้านตัวอย่างภาษาไทย</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Sans+Thai:wght@400;500;700&family=IBM+Plex+Mono:wght@400;600&display=swap');

    :root {
      --ink: #162227;
      --brand: #0f7a8a;
      --brand-soft: #d9f1f4;
      --accent: #f5a623;
      --paper: #fbf8f2;
      --panel: #ffffff;
      --line: #b7d0d6;
      --ok: #2d9f59;
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      min-height: 100vh;
      font-family: 'IBM Plex Sans Thai', 'Sarabun', sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at 8% 16%, #f4e9c8 0, transparent 34%),
        radial-gradient(circle at 92% 12%, #d7edf2 0, transparent 37%),
        linear-gradient(160deg, #fffefb 0%, #f5fafb 56%, #eef6f8 100%);
      display: grid;
      place-items: center;
      padding: 24px;
    }

    .shell {
      width: min(920px, 100%);
      display: grid;
      grid-template-columns: 1.15fr 0.85fr;
      gap: 18px;
      animation: rise 420ms ease-out both;
    }

    .card {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 20px;
      box-shadow: 0 8px 24px rgba(27, 63, 74, 0.09);
    }

    .hero {
      position: relative;
      overflow: hidden;
    }

    .hero::after {
      content: "";
      position: absolute;
      inset: auto -18% -32% auto;
      width: 180px;
      height: 180px;
      background: radial-gradient(circle, rgba(245, 166, 35, 0.22), transparent 68%);
      pointer-events: none;
    }

    h1 {
      margin: 0;
      font-size: clamp(1.4rem, 2.8vw, 2rem);
      line-height: 1.2;
      color: #0f4f60;
    }

    .subtitle {
      margin: 10px 0 0;
      color: #496771;
      font-size: 0.98rem;
    }

    .stats {
      margin-top: 18px;
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }

    .stat {
      background: var(--paper);
      border: 1px solid #e8dbc3;
      border-radius: 12px;
      padding: 10px;
    }

    .stat b {
      display: block;
      font-size: 1.1rem;
      color: #8b5a12;
      font-family: 'IBM Plex Mono', monospace;
    }

    .stat span {
      font-size: 0.82rem;
      color: #6e5e43;
    }

    form {
      display: grid;
      gap: 12px;
    }

    label {
      display: grid;
      gap: 6px;
      font-size: 0.9rem;
      font-weight: 500;
    }

    input,
    select,
    textarea {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 10px 12px;
      font: inherit;
      color: inherit;
      background: #fff;
    }

    textarea {
      min-height: 96px;
      resize: vertical;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }

    button {
      border: 0;
      border-radius: 11px;
      padding: 11px 14px;
      background: linear-gradient(135deg, var(--brand), #13a4ba);
      color: #fff;
      font-weight: 700;
      cursor: pointer;
      transition: transform 160ms ease, box-shadow 160ms ease;
    }

    button:hover {
      transform: translateY(-1px);
      box-shadow: 0 8px 16px rgba(16, 111, 124, 0.24);
    }

    .hint {
      margin-top: 10px;
      color: #3f6a74;
      font-size: 0.85rem;
      padding: 10px;
      border-left: 3px solid var(--ok);
      background: #eefaf2;
      border-radius: 8px;
    }

    @keyframes rise {
      from { opacity: 0; transform: translateY(10px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @media (max-width: 860px) {
      .shell {
        grid-template-columns: 1fr;
      }

      .grid {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
  <div class="shell">
    <section class="card hero">
      <h1>ระบบออกใบเสร็จ PDF ภาษาไทย</h1>
      <p class="subtitle">ฟอร์มตัวอย่างสำหรับทดสอบ Rust API ของ lynpdf-rs: กรอกข้อมูลสินค้าแล้วดาวน์โหลดใบเสร็จ PDF ภาษาไทยได้ทันที</p>
      <div class="stats">
        <div class="stat"><b>POST /receipt</b><span>สร้าง PDF เพื่อนำไปดาวน์โหลด</span></div>
        <div class="stat"><b>Rust + Axum</b><span>เว็บ API แบบง่ายสำหรับงานเอกสาร</span></div>
        <div class="stat"><b>A4 ใบเสร็จ</b><span>เลย์เอาต์พร้อมใช้งานภาษาไทย</span></div>
      </div>
    </section>

    <section class="card">
      <form method="post" action="/receipt">
        <label>
          ชื่อลูกค้า
          <input type="text" name="customer_name" value="บริษัท แอคมี (ประเทศไทย) จำกัด" required />
        </label>

        <label>
          สินค้า
          <select name="product_name">
            <option value="คีย์บอร์ดเพื่อการยศาสตร์">คีย์บอร์ดเพื่อการยศาสตร์</option>
            <option value="ฮับ USB-C 8-in-1">ฮับ USB-C 8-in-1</option>
            <option value="ไลเซนส์ชุดฟอนต์ภาษาไทย">ไลเซนส์ชุดฟอนต์ภาษาไทย</option>
          </select>
        </label>

        <div class="grid">
          <label>
            จำนวน
            <input type="number" name="quantity" min="1" value="2" required />
          </label>
          <label>
            ราคาต่อหน่วย (บาท)
            <input type="number" name="unit_price" min="0" step="0.01" value="1290.00" required />
          </label>
        </div>

        <label>
          หมายเหตุ
          <textarea name="note">ขอบคุณที่สั่งซื้อสินค้า รับประกันสินค้า 12 เดือน และสามารถติดต่อฝ่ายบริการได้ในวันทำการ</textarea>
        </label>

        <button type="submit">สร้างใบเสร็จ PDF</button>
      </form>

      <p class="hint">คำแนะนำ: หากเครื่องยังไม่มีฟอนต์ไทย ให้ตั้งค่า LYNPDF_FONT_DIRS หรือ LYNPDF_RS_EXAMPLE_FONT_DIR ก่อนรันแอป</p>
    </section>
  </div>
</body>
</html>
"#;
