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

fn main() -> Result<(), Box<Error>> {
    let args: Vec<String> = env::args().collect();
    let rawdomain = &args[1];
    let url = get_icon_url_extra(&rawdomain);

    println!("\n----------------------\nURL:{}\nRS: {:#?}\nBitwarden: https://icons.bitwarden.com/{}/icon.png\n\n---------------\n", rawdomain, url, rawdomain);

    Ok(())
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

    let mut body = "".to_string();


    let ssldomain = format!("https://{}", rawdomain);
    let httpdomain = format!("http://{}", rawdomain);
    // A Default SSL URL if for some reason we can not get the websites main page.
    let mut url = ssldomain.to_string();
    let resp = client.get(&ssldomain).send().or_else(|_| client.get(&httpdomain).send());
    if let Ok(mut content) = resp {
        if debug == "true" {
            println!("content: {:#?}", content);
        }
        body = content.text().unwrap();
        // Extract the URL from te respose incase redirects occured (like @ gitlab.com)
        url = format!("{}://{}", content.url().scheme(), content.url().host().unwrap());
    }

    // Parse HTML document
    let soup = Soup::new(&body);

    // Some debug stuff
    //println!("Reqested domain: {}", url);

    let favicons = soup
        .tag("link")
        .attr("rel", Regex::new(r"icon$|apple.*icon")?) // Only use icon rels
        .attr("href", Regex::new(r"(?i)\.jp(e){0,1}g$|\.png$|\.ico$")?) // Only allow specific extensions
        .find_all();


    // Create the iconlist
    let mut iconlist: Vec<IconList> = Vec::new();

    // Add the default favicon.ico to the list
    iconlist.push(IconList { priority: 35, href: format!("{}{}", url, "/favicon.ico") });

    // Loop through all the found icons and determine it's priority
    for favicon in favicons {
        let favicon_sizes = favicon.get("sizes").unwrap_or("0x0".to_string()).to_string();
        let mut favicon_href = favicon.get("href").unwrap_or("".to_string()).to_string();

        // Only continue if href is not empty
        let favicon_priority: u8;

        // Check if there is a dimension set
        if favicon_sizes != "0x0".to_string() {
            let dimensions : Vec<&str> = favicon_sizes.split("x").collect();
            let favicon_width = dimensions[0].parse::<u16>().unwrap();
            let favicon_height = dimensions[1].parse::<u16>().unwrap();

            // Only allow square dimensions
            if favicon_width == favicon_height {
                // Change priority by given size
                if favicon_width == 32 {
                    favicon_priority = 1;
                } else if favicon_width == 64 {
                    favicon_priority = 2;
                } else if favicon_width >= 24 && favicon_width <= 128 {
                    favicon_priority = 3;
                } else if favicon_width == 16 {
                    favicon_priority = 4;
                } else {
                    favicon_priority = 100;
                }
            } else {
                favicon_priority = 200;
            }
        } else {
            // Change priority by file extension
            if favicon_href.ends_with(".png") == true {
                favicon_priority = 10;
            } else if favicon_href.ends_with(".jpg") == true || favicon_href.ends_with(".jpeg") {
                favicon_priority = 20;
            } else {
                favicon_priority = 30;
            }
        }

        // When the href is starting with //, so without a scheme is valid and would use the browsers scheme.
        // We need to detect this and add the scheme here.
        if favicon_href.starts_with("//") == true {
            if debug == "true" {
                println!("No scheme for: {:#?}", favicon_href);
            }

            if url.starts_with("https") == true {
                favicon_href = format!("https:{}", favicon_href);
            } else {
                favicon_href = format!("http:{}", favicon_href);
            }
        // If the href just starts with a single / it does not have the host here at all.
        } else if favicon_href.starts_with("http") == false {
            if debug == "true" {
                println!("No host for: {:#?}", favicon_href);
            }

            if favicon_href.starts_with("/") == true {
                favicon_href = format!("{}{}", url, favicon_href);
            } else {
                favicon_href = format!("{}/{}", url, favicon_href);
            }
        }

        iconlist.push(IconList { priority: favicon_priority, href: favicon_href})
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
