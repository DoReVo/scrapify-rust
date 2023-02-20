use std::{error::Error, thread, time::Duration};

use fantoccini::{Client, Locator};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::shop_list::ShopConfig;

const LANGUAGE_BTN_SELECTOR: &str =
    "div.language-selection__list-item:nth-child(1) > button:nth-child(1)";
const _PAGE_BUTTONS_CLASS: &str = "[class='shopee-button-no-outline']";

const PRODUCT_CARD_CLASS: &str = ".shopee-search-item-result__item";
const PRODUCT_NAME_CLASS: &str = ".Cve6sh";
const PRODUCT_PRICE_CLASS: &str = ".vioxXd.rVLWG6";

const PRODUCT_URL_SELECTOR: &str = "a[data-sqe='link']";

const PAGE_CONTROLLSER_CLASS: &str = ".shopee-page-controller";

#[derive(Serialize, Deserialize)]
pub struct ValidProduct {
    name: String,
    price: String,
    shop_name: String,
    url: String,
}

pub async fn start_scrape(
    c: &Client,
    config: Vec<ShopConfig>,
) -> Result<Vec<ValidProduct>, Box<dyn Error>> {
    // Make window fullscreen
    match c.fullscreen_window().await {
        Ok(_) => (),
        Err(err) => panic!("Failed to make window fullscreen {:?}", err),
    };

    // Go to shopee and click the language button
    match c.goto("https://shopee.com.my").await {
        Ok(_) => (),
        Err(err) => panic!("Error going to shopee.com.my {:?}", err),
    };

    let language_btn = match c
        .wait()
        .for_element(Locator::Css(LANGUAGE_BTN_SELECTOR))
        .await
    {
        Ok(val) => val,
        Err(err) => panic!(
            "Error, cannot found language button to click on initial page load {:?}",
            err
        ),
    };

    match language_btn.click().await {
        Ok(_) => (),
        Err(err) => panic!("Error clicking language button {:?}", err),
    };

    // Hold list of all found products
    let mut product_list: Vec<ValidProduct> = vec![];

    for shop in &config {
        println!("Scrapping {}", shop.shop_name);

        // Construct URL instance for current shop
        let url = match Url::parse_with_params(
            "https://shopee.com.my/search",
            [
                ("keyword", "graphics card"),
                ("shop", &shop.shop_id),
                ("page", "0"),
            ],
        ) {
            Ok(url) => url,
            Err(err) => {
                eprintln!(
                    "Error constructing url instance {:?}, continue to next shop",
                    err
                );
                continue;
            }
        };

        // Go to shop page
        match c.goto(&url.to_string()).await {
            Ok(_) => (),
            Err(err) => {
                eprintln!(
                    "Error going to {} page {:?}, continue to next shop",
                    shop.shop_name, err
                );
                continue;
            }
        };

        // Wait for a product card to appear on the page
        match c.wait().for_element(Locator::Css(PRODUCT_CARD_CLASS)).await {
            Ok(_) => (),
            Err(err) => {
                eprintln!(
                    "Error, product card does not appear on page for shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        /* Find all product cards on the page
        At this point, there will be product cards,
        but there will be no text on them, no pricing, no product name.
        As the content of these cards are lazy loaded when they comes into view. */
        let products = match c.find_all(Locator::Css(PRODUCT_CARD_CLASS)).await {
            Ok(cards) => cards,
            Err(err) => {
                eprintln!(
                    "Error finding all product cards on page for shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        /* Scroll the page to the bottom,
        50 times should be long enough,
        calling window.scrollByPages should
        be enough to get to the absolute bottom.
        This is to trigger the lazy loading of the card content,
        so the product name and price gets loaded by the web app. */
        for _i in 1..50 {
            // println!("Scrolling into page {:?}", i);
            c.execute("window.scrollByPages(1)", vec![]).await?;
            thread::sleep(Duration::from_millis(20));
            // println!("Res {:?}", res);
        }

        /* Execute javascript that retrieve all
        product card on the page */
        // let _res = c
        //     .execute("return document.querySelectorAll('.Cve6sh').length", vec![])
        //     .await?;

        let mut class_name_not_found_total = 0;
        let mut class_price_not_found_total = 0;
        let mut class_url_not_found_total = 0;
        let mut valid_product_count = 0;

        // For each products found on the store page,
        // get its name and price
        for product in &products {
            let raw_html = match product.html(false).await {
                Ok(raw_html) => raw_html,
                Err(err) => {
                    eprintln!(
                        "Error getting HTML of product card for shop {} {:?}",
                        shop.shop_name, err
                    );
                    continue;
                }
            };
            let doc = Html::parse_fragment(&raw_html);

            let product_name_selector: Selector = match Selector::parse(PRODUCT_NAME_CLASS) {
                Ok(selector) => selector,
                Err(err) => {
                    eprintln!(
                        "Error constructing CSS selector for product name in shop {} {:?}",
                        shop.shop_name, err
                    );
                    continue;
                }
            };
            // There will always be 1 name
            let name_element = doc.select(&product_name_selector).next();

            let product_name = match name_element {
                Some(elem) => elem.text().collect::<String>(),
                None => {
                    class_name_not_found_total += 1;
                    continue;
                }
            };

            let product_price_selector = match Selector::parse(PRODUCT_PRICE_CLASS) {
                Ok(selector) => selector,
                Err(err) => {
                    eprintln!(
                        "Error constructing CSS selector for product price in shop {} {:?}",
                        shop.shop_name, err
                    );
                    continue;
                }
            };
            let price_element = doc.select(&product_price_selector).next();

            let product_price = match price_element {
                Some(elem) => elem.text().collect::<String>(),
                None => {
                    class_price_not_found_total += 1;
                    continue;
                }
            };

            // Product URL selector
            let product_url_selector = match Selector::parse(PRODUCT_URL_SELECTOR) {
                Ok(selector) => selector,
                Err(err) => {
                    eprintln!(
                        "Error constructing CSS selector for product a tag in shop {} {:?}",
                        shop.shop_name, err
                    );
                    continue;
                }
            };
            let url_element = doc.select(&product_url_selector).next();

            let product_url = match url_element {
                Some(elem) => match elem.value().attr("href") {
                    Some(link) => link,
                    None => {
                        class_url_not_found_total += 1;
                        continue;
                    }
                },
                None => {
                    class_url_not_found_total += 1;
                    continue;
                }
            };

            let mut final_url = "https://shopee.com.my".to_owned();
            final_url.push_str(product_url);

            let mut cleaned_url = Url::parse(&final_url).unwrap();
            cleaned_url.set_query(Some(""));

            valid_product_count += 1;
            product_list.push(ValidProduct {
                name: product_name,
                price: product_price,
                shop_name: shop.shop_name.clone(),
                url: cleaned_url.to_string(),
            });
        }

        let page_controller_selector = match Selector::parse(PAGE_CONTROLLSER_CLASS) {
            Ok(selector) => selector,
            Err(err) => {
                eprintln!(
                    "Error constructing CSS selector to find page controller in shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        let page_controller_button_selector = match Selector::parse("button") {
            Ok(selector) => selector,
            Err(err) => {
                eprintln!(
                    "Error constructing CSS selector to find page button in shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        let page_controller_element = match c.find(Locator::Css(PAGE_CONTROLLSER_CLASS)).await {
            Ok(elem) => elem,
            Err(err) => {
                eprintln!(
                    "Error finding page controller element for shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        let raw_html = match page_controller_element.html(false).await {
            Ok(raw_html) => raw_html,
            Err(err) => {
                eprintln!(
                    "Error getting HTML of page controller element for shop {} {:?}",
                    shop.shop_name, err
                );
                continue;
            }
        };

        let page_controller_element = Html::parse_fragment(&raw_html);

        let page_controller_element = page_controller_element
            .select(&page_controller_selector)
            .next();

        let page_controller_bar = match page_controller_element {
            Some(elem) => elem,
            None => continue,
        };

        let mut btn_arr: Vec<i32> = vec![];

        for elem in page_controller_bar.select(&page_controller_button_selector) {
            let num = match elem.text().next() {
                Some(txt) => txt,
                None => {
                    eprintln!("Error getting text from button");
                    continue;
                }
            };
            let num: i32 = match num.to_string().parse() {
                Ok(num) => num,
                Err(_err) => {
                    continue;
                }
            };
            btn_arr.push(num);
        }

        // Get page numbers
        println!(
            "Valid product: {}\nName not found: {}\nPrice not found: {}\nURL not found: {}\nPages: {:?} \n\n",
            valid_product_count,
            class_name_not_found_total,
            class_price_not_found_total,
            class_url_not_found_total,
            btn_arr
        );
    }

    return Ok(product_list);
}
