use axum::routing::{get, post};
use axum::{Json, Router};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::usize;

type IdMap = Arc<RwLock<HashMap<String, String>>>;
lazy_static! {
    static ref ID_MAP: IdMap = {
        let mut map = HashMap::new();
        let rw_lock = RwLock::new(map);
        Arc::new(rw_lock)
    };
}

#[derive(Debug, Serialize, Deserialize)]
struct Receipt {
    retailer: String,
    #[serde(rename = "purchaseDate")]
    date: String,
    #[serde(rename = "purchaseTime")]
    time: String,
    items: Vec<Item>,
    total: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    #[serde(rename = "shortDescription")]
    desc: String,
    price: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessResponse {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PointsResponse {
    points: String,
}

async fn process_receipt(receipt: axum::extract::Json<Receipt>) -> Json<ProcessResponse> {
    let id = uuid::Uuid::new_v4().to_string();

    // calculate points
    let alphanum: usize = receipt
        .retailer
        .chars()
        .filter(|c| c.is_alphanumeric())
        .count();

    let (round, quarter): (usize, usize) = if let Ok(amount) = receipt.total.parse::<f64>() {
        let r: usize = if amount - amount.trunc() == 0.0 {
            50
        } else {
            0
        };

        let q: usize = if (amount * 100.0) % 25.0 == 0.0 {
            25
        } else {
            0
        };
        (r, q)
    } else {
        (0, 0)
    };

    let mut item_points = 5 * (receipt.items.len() / 2);
    for item in receipt.items.iter() {
        if let Ok(price) = item.price.parse::<f64>() {
            if item.desc.trim().len() % 3 == 0 {
                let price = price * 0.2;
                let price: usize = price.ceil() as usize;
                item_points += price
            }
        }
    }

    let odd: usize = if odd_date(&receipt.date) { 6 } else { 0 };
    let time: usize = if time_check(&receipt.time) { 10 } else { 0 };

    let points = alphanum + round + quarter + item_points + odd + time;

    if let Ok(mut id_map) = ID_MAP.write() {
        id_map.insert(id.clone(), format!("{}", points));
    }

    let response = ProcessResponse { id };
    Json(response)
}

fn odd_date(date_str: &str) -> bool {
    if let Some(day_str) = date_str.split('-').nth(2) {
        if let Ok(day) = day_str.parse::<usize>() {
            return day % 2 != 0;
        }
    }
    false
}

fn time_check(time_str: &str) -> bool {
    if let Some((hour, minute)) = time_str
        .split(':')
        .next()
        .and_then(|hour| time_str.split(':').nth(1).map(|minute| (hour, minute)))
    {
        if let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) {
            return (hour == 14 && minute >= 0)
                || (hour == 15 && minute == 0)
                || (hour == 16 && minute == 0);
        }
    }
    false
}

async fn get_receipt_points(id: axum::extract::Path<String>) -> Json<PointsResponse> {

    let id = id.to_string();
    if let Ok(mut id_map) = ID_MAP.write() {
        let default = "Unknown".to_string();
        let points = id_map.get(&id).unwrap_or(&default);
        let response = PointsResponse{
            points: points.to_string()
        };
        return Json(response);
    }
    let response = PointsResponse{
        points: format!("unknown")
    };
    return Json(response)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/receipts/process", post(process_receipt))
        .route("/receipts/:id/points", get(get_receipt_points));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!(
        "Server running on http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}
