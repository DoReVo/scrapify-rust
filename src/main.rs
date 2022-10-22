use std::{path::Path, thread, time::Duration};

use fantoccini::{ClientBuilder, Locator};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::{self, time::Instant};
use url::Url;

const LANGUAGE_BTN_SELECTOR: &str =
    "div.language-selection__list-item:nth-child(1) > button:nth-child(1)";

const PRODUCT_CARD_CLASS: &str = ".shopee-search-item-result__item";

const PRODUCT_NAME_CLASS: &str = ".Cve6sh";
const PRODUCT_PRICE_CLASS: &str = ".vioxXd.rVLWG6";

#[derive(Serialize, Deserialize)]
struct ShopConfig {
    shop_id: String,
    shop_name: String,
    search_query: String,
    product_match: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ValidProduct {
    name: String,
    price: String,
    shop_name: String,
}

// let's set up the sequence of steps we want the browser to take
#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    let start = Instant::now();
    // Create client
    let c = ClientBuilder::native()
        .capabilities(serde_json::from_str(
            r#"{"moz:firefoxOptions": {"args": ["--headless"]}}"#,
        )?)
        .connect("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    c.fullscreen_window().await?;

    // let tab = c.new_window(true).await?;
    // c.switch_to_window(tab.handle).await?;

    // Go to shopee and click the language button
    c.goto("https://shopee.com.my").await?;

    let language_btn = c
        .wait()
        .for_element(Locator::Css(LANGUAGE_BTN_SELECTOR))
        .await?;

    language_btn.click().await?;

    // Read shop list from disk
    let x = tokio::fs::read(Path::new("shop_list.yaml")).await?;
    // Parse shop list
    let config: Vec<ShopConfig> = serde_yaml::from_slice(&x).unwrap();

    println!("Total shops to scrape {}", config.len());

    let mut product_list: Vec<ValidProduct> = vec![];

    for shop in &config {
        println!("Scrapping {}", shop.shop_name);

        let url = Url::parse_with_params(
            "https://shopee.com.my/search",
            [("keyword", "graphics card"), ("shop", &shop.shop_id)],
        )
        .unwrap();

        c.goto(&url.to_string()).await?;

        c.wait()
            .for_element(Locator::Css(PRODUCT_CARD_CLASS))
            .await?;

        let products = c.find_all(Locator::Css(PRODUCT_CARD_CLASS)).await?;

        for i in 1..50 {
            // println!("Scrolling into page {:?}", i);
            c.execute("window.scrollByPages(1)", vec![]).await?;
            thread::sleep(Duration::from_millis(20));

            // println!("Res {:?}", res);
        }
        let res = c
            .execute("return document.querySelectorAll('.Cve6sh').length", vec![])
            .await?;

        let mut class_name_not_found_total = 0;
        let mut class_price_not_found_total = 0;
        let mut valid_product_count = 0;

        for product in &products {
            let raw_html = product.html(false).await?;
            let doc = Html::parse_fragment(&raw_html);

            let product_name_selector: Selector = Selector::parse(PRODUCT_NAME_CLASS).unwrap();
            let name_element = doc.select(&product_name_selector).next();

            let product_name = match name_element {
                Some(elem) => elem.text().collect::<String>(),
                None => {
                    class_name_not_found_total += 1;
                    // panic!("No name");
                    continue;
                }
            };

            let product_price_selector = Selector::parse(PRODUCT_PRICE_CLASS).unwrap();
            let price_element = doc.select(&product_price_selector).next();

            let product_price = match price_element {
                Some(elem) => elem.text().collect::<String>(),
                None => {
                    class_price_not_found_total += 1;
                    // panic!("No price");
                    continue;
                }
            };

            // println!("{} {}", product_name, product_price);

            valid_product_count += 1;
            product_list.push(ValidProduct {
                name: product_name,
                price: product_price,
                shop_name: shop.shop_name.clone(),
            });
        }

        println!(
            "Valid product: {}\nName not found: {}\nPrice not found: {}\n\n",
            valid_product_count, class_name_not_found_total, class_price_not_found_total
        );
    }

    let duration = start.elapsed();

    let result_str = serde_yaml::to_string(&product_list).unwrap();
    tokio::fs::write(Path::new("result.yaml"), result_str).await?;

    println!("Time taken for program to run {:?}", duration);
    c.close().await
}
