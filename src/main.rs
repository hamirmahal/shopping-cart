use serde::{Deserialize, Serialize};
use serde_json::Result;

struct ShoppingCart<'a> {
    products: std::collections::HashMap<&'a str, usize>,
}

impl<'a> ShoppingCart<'a> {
    fn new() -> Self {
        Self {
            products: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, product: &'a str, quantity: usize) {
        self.products.insert(product, quantity);
    }

    fn total(&self, items: &[Item]) -> f64 {
        let mut total = 0.0;
        for (product, quantity) in &self.products {
            let item = items.iter().find(|item| item.name == *product).unwrap();
            if let Some(bulk_pricing) = &item.bulk_pricing {
                let bulk_price = bulk_pricing.total_price;
                let bulk_quantity = bulk_pricing.amount;
                let bulk_count = quantity / bulk_quantity as usize;
                let remainder = quantity % bulk_quantity as usize;
                total += bulk_count as f64 * bulk_price + remainder as f64 * item.price;
            } else {
                total += *quantity as f64 * item.price;
            }
        }
        total
    }

    fn clear(&mut self) {
        self.products.clear();
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
}

#[derive(Debug, Deserialize, Serialize)]
struct BulkPricing {
    amount: u32,
    #[serde(rename = "totalPrice")]
    total_price: f64,
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
    println!("Total: {}", cart.total(&data));
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
            },
            Item {
          id: 2,
          name: "Key Lime Cheesecake".to_string(),
          image_url: "http://1.bp.blogspot.com/-7we9Z0C_fpI/T90JXcg3YsI/AAAAAAAABn4/EN7u2vMuRug/s1600/key+lime+cheesecake+slice+in+front.jpg".to_string(),
          price: 8.0,
          bulk_pricing: None,
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
            },
            Item {
          id: 4,
          name: "Mini Gingerbread Donut".to_string(),
          image_url: "https://i.etsystatic.com/29050134/r/il/634971/3087380231/il_794xN.3087380231_n32u.jpg".to_string(),
          price: 0.5,
          bulk_pricing: None,
            },
        ];

        let mut cart = ShoppingCart::new();
        cart.add("Cookie", 7);
        assert_eq!(cart.total(&data), 7.25);

        cart.clear();
        cart.add("Cookie", 1);
        cart.add("Brownie", 4);
        cart.add("Key Lime Cheesecake", 1);
        assert_eq!(cart.total(&data), 16.25);

        cart.clear();
        cart.add("Cookie", 8);
        assert_eq!(cart.total(&data), 8.50);

        cart.clear();
        cart.add("Cookie", 1);
        cart.add("Brownie", 1);
        cart.add("Key Lime Cheesecake", 1);
        cart.add("Mini Gingerbread Donut", 2);
        assert_eq!(cart.total(&data), 12.25);

        cart.clear();
        assert_eq!(cart.total(&data), 0.0);
    }
}
