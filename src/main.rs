// src/main.rs
use std::error::Error;
use std::env;

extern crate reqwest;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT_LANGUAGE, CACHE_CONTROL, PRAGMA, ACCEPT};

extern crate regex;
use regex::Regex;

extern crate soup;
use soup::prelude::*;

use std::vec::Vec;

use std::time::Duration;

#[derive(Debug)]
struct IconList {
	priority: u8,
	href: String,
}

// fn main() -> Result<(), Box<Error>> {
fn main() {
    let args: Vec<String> = env::args().collect();
    let rawdomain = &args[1];
    let url = get_icon_url_extra(&rawdomain);

    println!("\n----------------------\nURL:{}\nRS: {:#?}\nBitwarden: https://icons.bitwarden.com/{}/icon.png\n\n---------------\n", rawdomain, url, rawdomain);
}

fn get_icon_url_extra(rawdomain: &str) -> Result<(String), Box<Error>> {
    let debug = env::var("ICON_DEBUG").unwrap_or("false".to_string());

    // Set some default headers for the request.
    // Use a browser like user-agent to make sure most websites will return there correct website.
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36 Edge/16.16299"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.8"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml; q=0.9,image/webp,image/apng,*/*;q=0.8"));

    let client = Client::builder()
        .gzip(true)
        .timeout(Duration::from_secs(5))
        .default_headers(headers)
        .build()?;


    let ssldomain = format!("https://{}", rawdomain);
    let httpdomain = format!("http://{}", rawdomain);
    // A Default SSL URL if for some reason we can not get the websites main page.
    let mut url = ssldomain.to_string();

    let mut body = "".to_string();
    let resp = client.get(&ssldomain).send().or_else(|_| client.get(&httpdomain).send());
    if let Ok(mut content) = resp {
        if debug == "true" {
            println!("content: {:#?}", content);
        }
        body = content.text().unwrap();
        // Extract the URL from te respose incase redirects occured (like @ gitlab.com)
        url = format!("{}://{}", content.url().scheme(), content.url().host().unwrap());
    }

    // Create the iconlist
    let mut iconlist: Vec<IconList> = Vec::new();
    // Add the default favicon.ico to the list
    iconlist.push(IconList { priority: 35, href: format!("{}{}", url, "/favicon.ico") });

    if ! body.is_empty() {
        let soup = Soup::new(&body);
        // Search for and filter
        let favicons = soup
            .tag("link")
            .attr("rel", Regex::new(r"icon$|apple.*icon")?) // Only use icon rels
            .attr("href", Regex::new(r"(?i)\w+(\.jp(e){0,1}g$|\.png$|\.ico$)")?) // Only allow specific extensions
            .find_all();

        // Loop through all the found icons and determine it's priority
        for favicon in favicons {
            let favicon_sizes = favicon.get("sizes").unwrap_or("".to_string()).to_string();
            let favicon_href = fix_href(&favicon.get("href").unwrap_or("".to_string()).to_string(), &url);
            let favicon_priority = get_icon_priority(&favicon_href, &favicon_sizes);

            iconlist.push(IconList { priority: favicon_priority, href: favicon_href})
        }
    }

    iconlist.sort_by_key(|x| x.priority);
    if debug == "true" {
        println!("{:#?}", iconlist);
    }

    let mut iconurl = "".to_string();
    if let Some(icon) = iconlist.first() {
        iconurl = format!("{}", icon.href);
    }

    Ok(iconurl)
}

/// Returns a String which will have the given href fixed by adding the correct URL if it does not have this already.
///
/// # Arguments
/// * `href` - A string which holds the href value or relative path.
/// * `url`  - A string which holds the URL including http(s) which will preseed the href when needed.
///
/// # Example
/// ```
/// fixed_href1 = fix_href("/path/to/a/image.png", "https://eample.com");
/// fixed_href2 = fix_href("//example.com/path/to/a/second/image.jpg", "https://eample.com");
/// ```
fn get_icon_priority(href: &str, sizes: &str) -> u8 {
    // Check if there is a dimension set
    if ! sizes.is_empty() {
        let dimensions : Vec<&str> = sizes.split("x").collect();
        let width = dimensions[0].parse::<u16>().unwrap();
        let height = dimensions[1].parse::<u16>().unwrap();

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
                100
            }
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

/// Returns a String which will have the given href fixed by adding the correct URL if it does not have this already.
///
/// # Arguments
/// * `href` - A string which holds the href value or relative path.
/// * `url`  - A string which holds the URL including http(s) which will preseed the href when needed.
///
/// # Example
/// ```
/// fixed_href1 = fix_href("/path/to/a/image.png", "https://eample.com");
/// fixed_href2 = fix_href("//example.com/path/to/a/second/image.jpg", "https://eample.com");
/// ```
fn fix_href(href: &str, url: &str) -> String {
    let debug = env::var("ICON_DEBUG").unwrap_or("false".to_string());
    let mut href_output = String::from(href);

    // When the href is starting with //, so without a scheme is valid and would use the browsers scheme.
    // We need to detect this and add the scheme here.
    if href_output.starts_with("//") {
        if debug == "true" {
            println!("No scheme for: {:#?}", href_output);
        }

        if url.starts_with("https") {
            href_output = format!("https:{}", href_output);
        } else {
            href_output = format!("http:{}", href_output);
        }
    // If the href_output just starts with a single / it does not have the host here at all.
    } else if ! href_output.starts_with("http") {
        if debug == "true" {
            println!("No host for: {:#?}", href_output);
        }

        if href_output.starts_with("/") {
            href_output = format!("{}{}", url, href_output);
        } else {
            href_output = format!("{}/{}", url, href_output);
        }
    }

    href_output
}
