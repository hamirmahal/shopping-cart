use chrono::{Datelike, Weekday};
use redis::Commands;
use serde::{Deserialize, Serialize};
use serde_json::Result;

struct ShoppingCart {
    products: std::collections::HashMap<String, usize>,
    redis_client: redis::Client,
}

impl ShoppingCart {
    fn new() -> Self {
        let redis_client =
            redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to Redis");
        Self {
            products: std::collections::HashMap::new(),
            redis_client,
        }
    }

    fn add(&mut self, product: &str, quantity: usize) {
        self.products.insert(product.to_owned(), quantity);
        let mut conn = self
            .redis_client
            .get_connection()
            .expect("Failed to connect to Redis");
        let _: () = conn
            .hset("shopping_cart", product, quantity)
            .expect("Failed to add item to Redis");
    }

    fn total(&self, items: &[Item], date: &chrono::NaiveDate) -> f64 {
        let mut total = 0.0;
        for (product, quantity) in &self.products {
            let item = items.iter().find(|item| item.name == *product).unwrap();
            total += match &item.sale {
                Some(sale) => match &sale.date {
                    SaleDate::DayOfWeek(weekday) if date.weekday() == *weekday => {
                        Self::apply_sale_price(&sale.sale_price, *quantity, item.price)
                    }
                    SaleDate::MonthAndDay(month, day)
                        if date.month() == *month && date.day() == *day =>
                    {
                        Self::apply_sale_price(&sale.sale_price, *quantity, item.price)
                    }
                    _ => *quantity as f64 * item.price,
                },
                None => match &item.bulk_pricing {
                    Some(bulk_pricing) if *quantity >= bulk_pricing.amount as usize => {
                        let bulk_count = *quantity / bulk_pricing.amount as usize;
                        let remainder = *quantity % bulk_pricing.amount as usize;
                        bulk_count as f64 * bulk_pricing.total_price + remainder as f64 * item.price
                    }
                    _ => *quantity as f64 * item.price,
                },
            };
        }
        total
    }

    fn apply_sale_price(sale_price: &SalePrice, quantity: usize, price: f64) -> f64 {
        match sale_price {
            SalePrice::QuantityForFixedPrice(sale_quantity, sale_price) => {
                let bulk_count = quantity / *sale_quantity as usize;
                let remainder = quantity % *sale_quantity as usize;
                bulk_count as f64 * *sale_price + remainder as f64 * price
            }
            SalePrice::PercentageOff(discount) => {
                let discounted_price = price * (100 - discount) as f64 / 100.0;
                discounted_price * quantity as f64
            }
            SalePrice::TwoForOne => {
                let pairs = quantity / 2;
                let remainder = quantity % 2;
                pairs as f64 * price + remainder as f64 * price
            }
        }
    }

    fn clear(&mut self) {
        self.products.clear();
        let mut conn = self
            .redis_client
            .get_connection()
            .expect("Failed to connect to Redis");
        let _: () = conn
            .del("shopping_cart")
            .expect("Failed to clear shopping cart in Redis");
    }

    #[allow(dead_code)]
    fn load_from_redis(&mut self) {
        let mut conn = self
            .redis_client
            .get_connection()
            .expect("Failed to connect to Redis");
        let shopping_cart: std::collections::HashMap<String, usize> = conn
            .hgetall("shopping_cart")
            .expect("Failed to load shopping_cart from Redis");
        self.products = shopping_cart;
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Item {
    id: u32,
    name: String,
    #[serde(rename = "imageURL")]
    image_url: String,
    price: f64,
    #[serde(rename = "bulkPricing")]
    bulk_pricing: Option<BulkPricing>,
    sale: Option<Sale>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BulkPricing {
    amount: u32,
    #[serde(rename = "totalPrice")]
    total_price: f64,
}

/// The sale price can be a fixed price, a percentage discount, or a two-for-one deal
/// Dates           | Product                       | Sale Price
/// ----------------|-------------------------------|-----------
/// Every Friday    | 8 Cookies                     | $6.00
/// Every October 1 | Any # of Key Lime Cheesecakes | 25% off
/// Every Tuesday   | Mini Gingerbread Donuts       | Two for one
#[derive(Debug, Deserialize, Serialize)]
enum SalePrice {
    QuantityForFixedPrice(u32, f64),
    PercentageOff(#[serde(deserialize_with = "deserialize_percentage")] u8),
    TwoForOne,
}

fn deserialize_percentage<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u8, D::Error> {
    let percentage = u8::deserialize(deserializer).unwrap();
    match percentage {
        0..=100 => Ok(percentage),
        _ => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(percentage.into()),
            &"a value between 0 and 100",
        )),
    }
}
/// Dates           | Product                       | Sale Price
/// ----------------|-------------------------------|-----------
/// Every Friday    | 8 Cookies                     | $6.00
/// Every October 1 | Any # of Key Lime Cheesecakes | 25% off
/// Every Tuesday   | Mini Gingerbread Donuts       | Two for one
/// `SaleDate` can be either a month and day, or a day of the week.
#[derive(Debug, Deserialize, Serialize)]
enum SaleDate {
    MonthAndDay(u32, u32),
    DayOfWeek(Weekday),
}
#[derive(Debug, Deserialize, Serialize)]
struct Sale {
    date: SaleDate,
    #[serde(rename = "salePrice")]
    sale_price: SalePrice,
}

fn parse(json_data: &str) -> Result<Vec<Item>> {
    let data: serde_json::Value = serde_json::from_str(json_data)?;
    let items = data["treats"].as_array().unwrap();
    let items: Vec<Item> = serde_json::from_value(serde_json::Value::Array(items.clone()))?;
    Ok(items)
}

fn main() -> Result<()> {
    let json_data = r#"
        {
          "treats": [
            {
              "id": 1,
              "name": "Brownie",
              "imageURL": "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTHdr1eTXEMs68Dx-b_mZT0RpifEQ8so6A1unRsJlyJIPe0LUE2HQ",
              "price": 2.0,
              "bulkPricing": {
                "amount": 4,
                "totalPrice": 7.0
              }
            },
            {
              "id": 2,
              "name": "Key Lime Cheesecake",
              "imageURL": "http://1.bp.blogspot.com/-7we9Z0C_fpI/T90JXcg3YsI/AAAAAAAABn4/EN7u2vMuRug/s1600/key+lime+cheesecake+slice+in+front.jpg",
              "price": 8.0,
              "bulkPricing": null
            },
            {
              "id": 3,
              "name": "Cookie",
              "imageURL": "http://www.mayheminthekitchen.com/wp-content/uploads/2015/05/chocolate-cookie-square.jpg",
              "price": 1.25,
              "bulkPricing": {
                "amount": 6,
                "totalPrice": 6.0
              }
            },
            {
              "id": 4,
              "name": "Mini Gingerbread Donut",
              "imageURL": "https://i.etsystatic.com/29050134/r/il/634971/3087380231/il_794xN.3087380231_n32u.jpg",
              "price": 0.5,
              "bulkPricing": null
            }
          ]
        }
    "#;
    let data = parse(json_data)?;
    println!("{:#?}", data);

    let mut cart = ShoppingCart::new();
    cart.add("Key Lime Cheesecake", 1);
    println!(
        "Total: {}",
        cart.total(&data, &chrono::NaiveDate::from_ymd_opt(1, 1, 1).unwrap())
    );
    cart.clear();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let json_data = r#"
        {
            "treats": [
              {
                "id": 1,
                "name": "Brownie",
                "imageURL": "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTHdr1eTXEMs68Dx-b_mZT0RpifEQ8so6A1unRsJlyJIPe0LUE2HQ",
                "price": 2.0,
                "bulkPricing": {
                  "amount": 4,
                  "totalPrice": 7.0
                }
              },

              {
                "id": 2,
                "name": "Key Lime Cheesecake",
                "imageURL": "http://1.bp.blogspot.com/-7we9Z0C_fpI/T90JXcg3YsI/AAAAAAAABn4/EN7u2vMuRug/s1600/key+lime+cheesecake+slice+in+front.jpg",
                "price": 8.0,
                "bulkPricing": null
              },

              {
                "id": 3,
                "name": "Cookie",
                "imageURL": "http://www.mayheminthekitchen.com/wp-content/uploads/2015/05/chocolate-cookie-square.jpg",
                "price": 1.25,
                "bulkPricing": {
                  "amount": 6,
                  "totalPrice": 6.0
                }
              },

              {
                "id": 4,
                "name": "Mini Gingerbread Donut",
                "imageURL": "https://i.etsystatic.com/29050134/r/il/634971/3087380231/il_794xN.3087380231_n32u.jpg",
                "price": 0.5,
                "bulkPricing": null
              }
            ]
          }
        "#;

        let data = parse(json_data).unwrap();

        assert_eq!(data.len(), 4);

        assert_eq!(data[0].id, 1);
        assert_eq!(data[0].name, "Brownie");
        assert_eq!(data[0].price, 2.0);
        assert_eq!(data[0].bulk_pricing.as_ref().unwrap().amount, 4);
        assert_eq!(data[0].bulk_pricing.as_ref().unwrap().total_price, 7.0);

        assert_eq!(data[1].id, 2);
        assert_eq!(data[1].name, "Key Lime Cheesecake");
        assert_eq!(data[1].price, 8.0);
        assert!(data[1].bulk_pricing.is_none());

        assert_eq!(data[2].id, 3);
        assert_eq!(data[2].name, "Cookie");
        assert_eq!(data[2].price, 1.25);
        assert_eq!(data[2].bulk_pricing.as_ref().unwrap().amount, 6);
        assert_eq!(data[2].bulk_pricing.as_ref().unwrap().total_price, 6.0);

        assert_eq!(data[3].id, 4);
        assert_eq!(data[3].name, "Mini Gingerbread Donut");
        assert_eq!(data[3].price, 0.5);
        assert!(data[3].bulk_pricing.is_none());
    }

    #[test]
    fn test_shopping_cart_total() {
        let data = vec![
            Item {
              id: 1,
              name: "Brownie".to_string(),
              image_url: "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTHdr1eTXEMs68Dx-b_mZT0RpifEQ8so6A1unRsJlyJIPe0LUE2HQ".to_string(),
              price: 2.0,
              bulk_pricing: Some(BulkPricing {
                  amount: 4,
                  total_price: 7.0,
              }),
              sale: None,
            },
            Item {
              id: 2,
              name: "Key Lime Cheesecake".to_string(),
              image_url: "http://1.bp.blogspot.com/-7we9Z0C_fpI/T90JXcg3YsI/AAAAAAAABn4/EN7u2vMuRug/s1600/key+lime+cheesecake+slice+in+front.jpg".to_string(),
              price: 8.0,
              bulk_pricing: None,
              sale: None
            },
            Item {
              id: 3,
              name: "Cookie".to_string(),
              image_url: "http://www.mayheminthekitchen.com/wp-content/uploads/2015/05/chocolate-cookie-square.jpg".to_string(),
              price: 1.25,
              bulk_pricing: Some(BulkPricing {
                  amount: 6,
                  total_price: 6.0,
              }),
              sale: None
            },
            Item {
              id: 4,
              name: "Mini Gingerbread Donut".to_string(),
              image_url: "https://i.etsystatic.com/29050134/r/il/634971/3087380231/il_794xN.3087380231_n32u.jpg".to_string(),
              price: 0.5,
              bulk_pricing: None,
              sale: None
            },
        ];

        let dummy_date = &chrono::NaiveDate::from_ymd_opt(1, 1, 1).unwrap();

        let mut cart = ShoppingCart::new();
        cart.add("Cookie", 7);
        assert_eq!(cart.total(&data, dummy_date), 7.25);

        cart.clear();
        cart.add("Cookie", 1);
        cart.add("Brownie", 4);
        cart.add("Key Lime Cheesecake", 1);
        assert_eq!(cart.total(&data, dummy_date), 16.25);

        cart.clear();
        cart.add("Cookie", 8);
        assert_eq!(cart.total(&data, dummy_date), 8.50);

        cart.clear();
        cart.add("Cookie", 1);
        cart.add("Brownie", 1);
        cart.add("Key Lime Cheesecake", 1);
        cart.add("Mini Gingerbread Donut", 2);
        assert_eq!(cart.total(&data, dummy_date), 12.25);

        cart.clear();
        assert_eq!(cart.total(&data, dummy_date), 0.0);
    }

    #[test]
    fn test_sales() {
        let data = vec![
          Item {
            id: 2,
            name: "Key Lime Cheesecake".to_string(),
            image_url: "http://1.bp.blogspot.com/-7we9Z0C_fpI/T90JXcg3YsI/AAAAAAAABn4/EN7u2vMuRug/s1600/key+lime+cheesecake+slice+in+front.jpg".to_string(),
            price: 8.0,
            bulk_pricing: None,
            sale: Some(Sale {
              date: SaleDate::MonthAndDay(10, 1),
                sale_price: SalePrice::PercentageOff(25)
            }),
          },
          Item {
            id: 3,
            name: "Cookie".to_string(),
            image_url: "http://www.mayheminthekitchen.com/wp-content/uploads/2015/05/chocolate-cookie-square.jpg".to_string(),
            price: 1.25,
            bulk_pricing: Some(BulkPricing {
                amount: 6,
                total_price: 6.0,
            }),
            sale: Some(Sale {
              date: SaleDate::DayOfWeek(chrono::Weekday::Fri),
              sale_price: SalePrice::QuantityForFixedPrice(8, 6.0)
              },),
          },
      ];

        let mut cart = ShoppingCart::new();
        cart.add("Cookie", 8);
        cart.add("Key Lime Cheesecake", 4);
        assert_eq!(
            cart.total(
                &data,
                &chrono::NaiveDate::from_ymd_opt(2021, 10, 1).unwrap()
            ),
            30.0
        );
    }

    #[test]
    fn test_percentage_off() {
        let data = vec![
            Item {
                id: 1,
                name: "Apple".to_string(),
                image_url: "".to_string(),
                price: 8.0,
                bulk_pricing: None,
                sale: Some(Sale {
                    date: SaleDate::MonthAndDay(10, 1),
                    sale_price: SalePrice::PercentageOff(25),
                }),
            },
            Item {
                id: 2,
                name: "Banana".to_string(),
                image_url: "".to_string(),
                price: 2.22,
                bulk_pricing: None,
                sale: Some(Sale {
                    date: SaleDate::MonthAndDay(10, 1),
                    sale_price: SalePrice::PercentageOff(0),
                }),
            },
            Item {
                id: 3,
                name: "Carrot".to_string(),
                image_url: "".to_string(),
                price: 3.33,
                bulk_pricing: None,
                sale: Some(Sale {
                    date: SaleDate::MonthAndDay(10, 1),
                    sale_price: SalePrice::PercentageOff(100),
                }),
            },
        ];

        let mut cart = ShoppingCart::new();
        cart.add("Apple", 1);
        assert_eq!(
            cart.total(
                &data,
                &chrono::NaiveDate::from_ymd_opt(2021, 10, 1).unwrap()
            ),
            6.0
        );
        cart.clear();

        cart.add("Banana", 1);
        assert_eq!(
            cart.total(
                &data,
                &chrono::NaiveDate::from_ymd_opt(2021, 10, 1).unwrap()
            ),
            2.22
        );
        cart.clear();

        cart.add("Carrot", 1);
        assert_eq!(
            cart.total(
                &data,
                &chrono::NaiveDate::from_ymd_opt(2021, 10, 1).unwrap()
            ),
            0.0
        );
        cart.clear();
    }

    #[test]
    #[should_panic(expected = "invalid value: integer `101`, expected a value between 0 and 100")]
    fn test_invalid_percentage_in_json() {
        let json_data = r#"
        {
            "treats": [
              {
                "id": 1,
                "name": "Apple",
                "imageURL": "",
                "price": 8.0,
                "bulkPricing": null,
                "sale": {
                  "date": {
                    "MonthAndDay": [10, 1]
                  },
                  "salePrice": {
                    "PercentageOff": 101
                  }
                }
              }
            ]
          }
        "#;
        parse(json_data).unwrap();
    }
}
