use super::templates::MembershipForm;
use askama::Template;
use headless_chrome::{types::PrintToPdfOptions, Browser, LaunchOptions};
use std::sync::OnceLock;

static BROWSER: OnceLock<Browser> = OnceLock::new();

fn get_browser() -> Result<&'static Browser, String> {
	if let Some(browser) = BROWSER.get() {
		return Ok(browser);
	}
	let browser = Browser::new(LaunchOptions::default())
		.map_err(|e| format!("Impossibile avviare Chrome: {e}"))?;
	let _ = BROWSER.set(browser);
	Ok(BROWSER.get().expect("browser appena inizializzato"))
}

pub fn generate(form: MembershipForm) -> Result<Vec<u8>, String> {
	let html_content = form.render().map_err(|e| e.to_string())?;
	let browser = get_browser()?;
	let tab = browser.new_tab().map_err(|e| e.to_string())?;

	let data_url = format!("data:text/html;charset=utf-8,{}", urlencoding::encode(&html_content));
	tab.navigate_to(&data_url).map_err(|e| e.to_string())?;
	tab.wait_until_navigated().map_err(|e| e.to_string())?;

	let pdf_options = PrintToPdfOptions {
		landscape: Some(false),
		print_background: Some(true),
		prefer_css_page_size: Some(true),
		..Default::default()
	};

	tab.print_to_pdf(Some(pdf_options)).map_err(|e| e.to_string())
}