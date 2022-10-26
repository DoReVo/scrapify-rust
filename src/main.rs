use std::path::Path;

use fantoccini::ClientBuilder;

use tokio::{self, time::Instant};

mod gecko_manager;
mod scraper;
mod shop_list;

// let's set up the sequence of steps we want the browser to take
#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    // Start geckodriver
    let gecko_handle = gecko_manager::start().await;
    // Start calculating time
    let start = Instant::now();
    // Read shop list
    let config = shop_list::read_config().await;
    println!("Total shops to scrape {}", config.len());

    // Create client
    let c = ClientBuilder::native()
        .capabilities(serde_json::from_str(
            r#"{"moz:firefoxOptions": {"args": ["--headless"]}}"#,
        )?)
        .connect("http://localhost:7000")
        .await
        .expect("failed to connect to WebDriver");

    match scraper::start_scrape(&c, config).await {
        Ok(list_of_valid_products) => {
            println!("Scraping finished!");

            // Deserialize into YAML
            match serde_yaml::to_string(&list_of_valid_products) {
                Ok(res) => {
                    // Save YAML to disk
                    match tokio::fs::write(Path::new("result.yaml"), res).await {
                        Ok(_) => println!("Result saved to disk!"),
                        Err(err) => eprintln!("Error during saving result {:?}", err),
                    };
                }
                Err(err) => {
                    eprintln!("Error deserializing results into YAML {:?}", err)
                }
            };
        }
        Err(err) => println!("Error during scraping {:?}", err),
    };

    c.close().await.unwrap();

    gecko_manager::stop(gecko_handle).await.unwrap();

    let duration = start.elapsed();
    println!("Time taken for program to run {:?}", duration);

    Ok(())
}
