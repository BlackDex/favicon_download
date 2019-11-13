// src/main.rs
#[macro_use] extern crate lazy_static;
// use log::{info, trace, warn};
use log::{info};

use env_logger::{Env};

use std::error::Error;
use std::env;

// use reqwest::blocking::Client;
use reqwest::{header::HeaderMap, Client, Response};

// use rocket::http::Cookie;
// Use cookie crate instead of rocket::http:Cookie since it is exactly the same
use cookie::Cookie;
use std::time::Duration;

use regex::Regex;

use soup::prelude::*;

use std::vec::Vec;
#[derive(Debug)]
struct IconList {
	priority: u8,
	href: String,
}

#[derive(Debug)]
struct Icon {
    priority: u8,
    href: String,
}

impl Icon {
    fn new(priority: u8, href: String) -> Self {
        Self { href, priority }
    }
}

lazy_static! {
    // Reuse the client between requests
    static ref CLIENT: Client = Client::builder()
        .use_sys_proxy()
        .gzip(true)
        .timeout(Duration::from_secs(5))
        .default_headers(_header_map())
        .build()
        .unwrap();
}

fn main() {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    let rawdomain = &args[1];
    let (iconlist, cookies) = get_icon_url(&rawdomain).unwrap();

    info!("\nCookies: {:#?}\n", cookies);
    info!("\nDomain: {:#?}\nIconList: {:#?}\n", rawdomain, iconlist);
}

/// Returns a Result/Tuple which holds a Vector IconList and a string which holds the cookies from the last response.
/// There will always be a result with a string which will contain https://example.com/favicon.ico and an empty string for the cookies.
/// This does not mean that that location does exists, but it is the default location browser use.
///
/// # Argument
/// * `domain` - A string which holds the domain with extension.
///
/// # Example
/// ```
/// let (mut iconlist, cookie_str) = get_icon_url("github.com")?;
/// let (mut iconlist, cookie_str) = get_icon_url("gitlab.com")?;
/// ```
fn get_icon_url(domain: &str) -> Result<(Vec<Icon>, String), Box<dyn Error>> {
    // Default URL with secure and insecure schemes
    let ssldomain = format!("https://{}", domain);
    let httpdomain = format!("http://{}", domain);

    // Create the iconlist
    let mut iconlist: Vec<Icon> = Vec::new();

    // Create the cookie_str to fill it all the cookies from the response
    // These cookies can be used to request/download the favicon image.
    // Some sites have extra security in place with for example XSRF Tokens.
    let mut cookie_str = String::new();

    let resp = get_page(&ssldomain).or_else(|_| get_page(&httpdomain));
    if let Ok(content) = resp {
        // Extract the URL from the respose in case redirects occured (like @ gitlab.com)
        let url = content.url().clone();

        let raw_cookies = content.headers().get_all("set-cookie");
        cookie_str = raw_cookies
            .iter()
            .filter_map(|raw_cookie| raw_cookie.to_str().ok())
            .map(|cookie_str| {
                if let Ok(cookie) = Cookie::parse(cookie_str) {
                    format!("{}={}; ", cookie.name(), cookie.value())
                } else {
                    String::new()
                }
            })
            .collect::<String>();

        // Add the default favicon.ico to the list with the domain the content responded from.
        iconlist.push(Icon::new(35, url.join("/favicon.ico").unwrap().into_string()));

        let soup = Soup::from_reader(content)?;
        // Search for and filter
        let favicons = soup
            .tag("link")
            .attr("rel", Regex::new(r"icon$|apple.*icon")?) // Only use icon rels
            .attr("href", Regex::new(r"(?i)\w+\.(jpg|jpeg|png|ico)(\?.*)?$")?) // Only allow specific extensions
            .find_all();

        // Loop through all the found icons and determine it's priority
        for favicon in favicons {
            let sizes = favicon.get("sizes");
            let href = favicon.get("href").expect("Missing href");
            let full_href = url.join(&href).unwrap().into_string();

            let priority = get_icon_priority(&full_href, sizes);

            iconlist.push(Icon::new(priority, full_href))
        }
    } else {
        // Add the default favicon.ico to the list with just the given domain
        iconlist.push(Icon::new(35, format!("{}/favicon.ico", ssldomain)));
        iconlist.push(Icon::new(35, format!("{}/favicon.ico", httpdomain)));
    }

    // Sort the iconlist by priority
    iconlist.sort_by_key(|x| x.priority);

    // There always is an icon in the list, so no need to check if it exists, and just return the first one
    Ok((iconlist, cookie_str))
}

fn get_page(url: &str) -> Result<Response, Box<dyn Error>> {
    get_page_with_cookies(url, "")
}

fn get_page_with_cookies(url: &str, cookie_str: &str) -> Result<Response, Box<dyn Error>> {
    // Used within bitwarden_rs - not in this test/debug application
    // if check_icon_domain_is_blacklisted(Url::parse(url).unwrap().host_str().unwrap_or_default()) {
    //     warn!("Favicon rel linked to a non blacklisted domain!");
    // }

    if cookie_str.is_empty() {
        CLIENT.get(url).send()?.error_for_status().map_err(Into::into)
    } else {
        CLIENT
            .get(url)
            .header("cookie", cookie_str)
            .send()?
            .error_for_status()
            .map_err(Into::into)
    }
}

/// Returns a Integer with the priority of the type of the icon which to prefer.
/// The lower the number the better.
///
/// # Arguments
/// * `href`  - A string which holds the href value or relative path.
/// * `sizes` - The size of the icon if available as a <width>x<height> value like 32x32.
///
/// # Example
/// ```
/// priority1 = get_icon_priority("http://example.com/path/to/a/favicon.png", "32x32");
/// priority2 = get_icon_priority("https://example.com/path/to/a/favicon.ico", "");
/// ```
fn get_icon_priority(href: &str, sizes: Option<String>) -> u8 {
    // Check if there is a dimension set
    let (width, height) = parse_sizes(sizes);

    // Check if there is a size given
    if width != 0 && height != 0 {
        // Only allow square dimensions
        if width == height {
            // Change priority by given size
            if width == 32 {
                1
            } else if width == 64 {
                2
            } else if width >= 24 && width <= 128 {
                3
            } else if width == 16 {
                4
            } else {
                5
            }
        // There are dimensions available, but the image is not a square
        } else {
            200
        }
    } else {
        // Change priority by file extension
        if href.ends_with(".png") {
            10
        } else if href.ends_with(".jpg") || href.ends_with(".jpeg") {
            20
        } else {
            30
        }
    }
}

/// Returns a Tuple with the width and hight as a seperate value extracted from the sizes attribute
/// It will return 0 for both values if no match has been found.
///
/// # Arguments
/// * `sizes` - The size of the icon if available as a <width>x<height> value like 32x32.
///
/// # Example
/// ```
/// let (width, height) = parse_sizes("64x64"); // (64, 64)
/// let (width, height) = parse_sizes("x128x128"); // (128, 128)
/// let (width, height) = parse_sizes("32"); // (0, 0)
/// ```
fn parse_sizes(sizes: Option<String>) -> (u16, u16) {
    let mut width: u16 = 0;
    let mut height: u16 = 0;

    if let Some(sizes) = sizes {
        match Regex::new(r"(?x)(\d+)\D*(\d+)").unwrap().captures(sizes.trim()) {
            None => {}
            Some(dimensions) => {
                if dimensions.len() >= 3 {
                    width = dimensions[1].parse::<u16>().unwrap_or_default();
                    height = dimensions[2].parse::<u16>().unwrap_or_default();
                }
            }
        }
    }

    (width, height)
}

fn _header_map() -> HeaderMap {
    // Set some default headers for the request.
    // Use a browser like user-agent to make sure most websites will return there correct website.
    use reqwest::header::*;

    macro_rules! headers {
        ($( $name:ident : $value:literal),+ $(,)? ) => {
            let mut headers = HeaderMap::new();
            $( headers.insert($name, HeaderValue::from_static($value)); )+
            headers
        };
    }

    headers! {
        USER_AGENT: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36 Edge/16.16299",
        ACCEPT_LANGUAGE: "en-US,en;q=0.8",
        CACHE_CONTROL: "no-cache",
        PRAGMA: "no-cache",
        ACCEPT: "text/html,application/xhtml+xml,application/xml; q=0.9,image/webp,image/apng,*/*;q=0.8",
    }
}
