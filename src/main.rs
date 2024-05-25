mod memory_manager;
pub mod order_manager;
use chrono::{Timelike, Local};
use hmac::Mac;
use once_cell::sync::Lazy;
use order_manager::Order;
use std::sync::Mutex;
use hex::encode as hex_encode; 
use std::time::{SystemTime, UNIX_EPOCH};
use futures::{stream, StreamExt};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};

static MEM: Lazy<Mutex<memory_manager::MemoryManager>> = Lazy::new(|| Mutex::new(memory_manager::MemoryManager::new()));

#[tokio::main]
async fn main() {
    let _ = tokio::spawn(task_websocket());
    let _ = tokio::spawn(task_trader());
    let _ = tokio::spawn(task_stats());
    let _ = tokio::spawn(task_keepalive());
    let _ = tokio::spawn(task_fullrestart());
    loop {
        sleep(Duration::MAX).await;
    }
}

async fn task_set_last_long_open(price: f64) {
    sleep(Duration::from_secs(1)).await;
    {
        MEM.lock().unwrap().set_last_long_open(price);
    }
}

async fn task_set_last_short_open(price: f64) {
    sleep(Duration::from_secs(1)).await;
    {
        MEM.lock().unwrap().set_last_short_open(price);
    }
}

async fn task_stats() {
    loop {
        sleep(Duration::from_secs(50)).await;
        verify_account_info().await;
    }
}

async fn print_stats() {
    let (duration, volume, order_overflow, balance, commission, trading_delta, long_quantity, short_quantity, price_decimal_count, quantity_decimal_count, current_price, long_close_price, short_close_price, long_entry_price, short_entry_price, long_un_pnl, short_un_pnl, vip_level, long_increment, short_increment) = { MEM.lock().unwrap().get_stats() };
    let mut str_list: Vec<String> = Vec::with_capacity(15);
    let now = Local::now();
    str_list.push(format!("------------- Stats -------------"));
    str_list.push(format!("Current session: {}:{}.{}", (duration.as_secs() / 60) / 60, (duration.as_secs() / 60) % 60, duration.as_secs() % 60));
    str_list.push(format!("Current time: {}:{}.{}", now.hour(), now.minute(), now.second()));
    str_list.push(format!("Vip Level: {}", vip_level));
    str_list.push(format!("Balance: {0:.1$}", balance, price_decimal_count as usize));
    str_list.push(format!("Commission: {0:.1$}", commission, price_decimal_count as usize));
    str_list.push(format!("Trading delta: {0:.1$}", trading_delta, price_decimal_count as usize));
    str_list.push(format!("Volume: {0:.1$}", volume, price_decimal_count as usize));
    str_list.push(format!("Order overflow: {}", order_overflow));
    str_list.push(format!("---------------------------------"));
    str_list.push(format!("30d Volume: {:.2}", volume / duration.as_secs() as f64 * 2.592));
    let salary = (commission + trading_delta) / duration.as_secs() as f64 * 3600.0;
    str_list.push(format!("$/h: {0:.1$}", salary, price_decimal_count as usize));
    str_list.push(format!("$/year: {:.1}k", salary * 8.760));
    str_list.push(format!("---------------------------------"));
    str_list.push(format!("Current price: {:.2}", current_price));
    str_list.push(format!("Long entry price: {}", if long_entry_price < 1.0 { String::from("No longs") } else { format!("{:.2}", long_entry_price) }));
    str_list.push(format!("Long close price: {}", if long_entry_price < 1.0 { String::from("No longs") } else { format!("{:.2}", long_close_price) }));
    str_list.push(format!("Short entry price: {}", if short_entry_price < 1.0 { String::from("No shorts") } else { format!("{:.2}", short_entry_price) }));
    str_list.push(format!("Short close price: {}", if short_entry_price < 1.0 { String::from("No shorts") } else { format!("{:.2}", short_close_price) }));
    str_list.push(format!("---------------------------------"));
    str_list.push(format!("Current longs: {}", if long_entry_price < 1.0 { String::from("No longs") } else { format!("{0:.1$}", long_quantity, quantity_decimal_count as usize) }));
    str_list.push(format!("Longs Un-PNL: {}", if long_entry_price < 1.0 { String::from("No longs") } else { format!("{:.2}", long_un_pnl) }));
    str_list.push(format!("Current shorts: {}", if short_entry_price < 1.0 { String::from("No shorts") } else { format!("{0:.1$}", short_quantity, quantity_decimal_count as usize) }));
    str_list.push(format!("Shorts Un-PNL: {}", if short_entry_price < 1.0 { String::from("No shorts") } else { format!("{:.2}", short_un_pnl) }));
    str_list.push(format!("---------------------------------"));
    str_list.push(format!("Long increment: {:.1}", long_increment));
    str_list.push(format!("Short increment: {:.1}", short_increment));
    str_list.push(format!("---------------------------------"));
    let msg_str = str_list.join("\n");
    println!("\x1b[96m{}\x1b[0m", msg_str);

    let input_msg = get_discord_msg().await;

    send_discord_msg(&format!("```{}```XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXaaGGGGGGGGGGGGGGGGGGGGGGYYYYYYYYYYYYYYY", &msg_str)).await;
    read_user_input(&input_msg).await;
}

async fn get_discord_msg() -> String {
    let (token, channel) = { MEM.lock().unwrap().get_discord()};

    let url = format!("https://discord.com/api/v9/channels/{}/messages?limit=1", channel);

    let mut header = HeaderMap::new();
    header.append("Authorization", HeaderValue::from_str(&token).unwrap());
    
    let text = reqwest::Client::new().get(url).headers(header).send().await.unwrap().text().await.unwrap();
    let my_json = json::parse(&text).unwrap();

    String::from(my_json[0]["content"].as_str().unwrap())
}

async fn send_discord_msg(msg: &str) {
    let (token, channel) = { MEM.lock().unwrap().get_discord()};

    let url = format!("https://discord.com/api/v9/channels/{}/messages", channel);

    let mut header = HeaderMap::new();
    header.insert(CONTENT_LENGTH, format!("{}", msg.len()).parse().unwrap());
    header.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    header.append("Authorization", HeaderValue::from_str(&token).unwrap());

    let mut body = HashMap::new();
    body.insert("content", msg);
    
    reqwest::Client::new().post(url).headers(header).form(&body).send().await.unwrap();
}

async fn read_user_input(input: &str) {
    match input {
        "kill" => {
            println!("\x1b[91mUser input: Kill\x1b[0m");
            send_discord_msg("Okay buddy, I will kill my process!012345678901").await;
            std::process::exit(1);
        },
        "close" => {
            println!("\x1b[91mUser input: Close\x1b[0m");
            send_discord_msg("Okay buddy, I will set myself in close_only mode!012345678901").await;
            { MEM.lock().unwrap().activate_close_only() };
        },
        _ => ()
    }
}

async fn task_keepalive() {
    loop {
        sleep(Duration::from_secs(3000)).await;
        println!("\x1b[94mSending keepalive\x1b[0m");
        send_keepalive().await;
    }
}

async fn task_fullrestart() {
    loop {
        sleep(Duration::from_secs(82000)).await;
        println!("\x1b[94mFull restart\x1b[0m");
        { MEM.lock().unwrap().set_need_restart_true() };
    }
}

async fn task_websocket() {
    loop {
        let (base_url, pair) = {
            let local_mem = &MEM.lock().unwrap();
            (local_mem.get_url_websocket(), local_mem.get_pair())
        };
        let book_ticker = &format!("{}@bookTicker", pair.to_lowercase());
        let url = url::Url::parse(&format!("{}/stream?streams={}/{}", base_url, book_ticker, get_listen_key().await)).unwrap();
        println!("Initiating websocket");
        let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        println!("Connected to websocket");
        let token = wait_token().await;
        let (quantity_decimal_count, price_decimal_count) = {
            let local_mem = &MEM.lock().unwrap();
            (local_mem.get_quantity_decimal_count() as usize, local_mem.get_price_decimal_count() as usize)
        };
        loop {
            let msg = socket.next().await.unwrap().unwrap();
            if msg.is_ping() {
                if MEM.lock().unwrap().is_restart_needed() {
                    let url = url::Url::parse(&format!("{}/stream?streams={}/{}", base_url, book_ticker, get_listen_key().await)).unwrap();
                    println!("Initiating websocket");
                    (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
                    println!("Connected to websocket");
                    { MEM.lock().unwrap().set_need_restart_false() };
                }
            } else {
                let msg_json = json::parse(msg.to_text().unwrap()).unwrap();
                if msg_json["stream"].as_str().unwrap() == book_ticker {
                    MEM.lock().unwrap().set_marketprice(msg_json["data"]["a"].as_str().unwrap().parse::<f64>().unwrap(), msg_json["data"]["b"].as_str().unwrap().parse::<f64>().unwrap());
                } else {
                    match msg_json["data"]["e"].as_str().unwrap() {
                        "ORDER_TRADE_UPDATE" => {
                            match msg_json["data"]["o"]["X"].as_str().unwrap() {
                                "EXPIRED" => {
                                    if msg_json["data"]["o"]["ps"].as_str().unwrap() == "LONG" {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "BUY" {
                                            println!("\x1b[95mOpen long order expired\x1b[0m");
                                            MEM.lock().unwrap().new_open_long_expired();
                                        } else {
                                            println!("\x1b[95mClose long order expired\x1b[0m");
                                            MEM.lock().unwrap().new_close_long_expired();
                                        }
                                    } else {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "SELL" {
                                            println!("\x1b[95mOpen short order expired\x1b[0m");
                                            MEM.lock().unwrap().new_open_short_expired();
                                        } else {
                                            println!("\x1b[95mClose short order expired\x1b[0m");
                                            MEM.lock().unwrap().new_close_short_expired();
                                        }
                                    }
                                },
                                "FILLED" => {
                                    let comission = msg_json["data"]["o"]["n"].as_str().unwrap().parse::<f64>().unwrap();
                                    let pnl = msg_json["data"]["o"]["rp"].as_str().unwrap().parse::<f64>().unwrap();
                                    if msg_json["data"]["o"]["ps"].as_str().unwrap() == "LONG" {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "BUY" {
                                            println!("\x1b[92mOrder filled OPEN {0:.1$} LONG at {2:.3$} with {4:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, comission * -1.0);
                                            MEM.lock().unwrap().new_open_long_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                            let _ = tokio::spawn(task_set_last_long_open(msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap()));
                                        } else {
                                            println!("\x1b[92mOrder filled CLOSE {0:.1$} LONG at {2:.3$} with {4:.3$} PNL and {5:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, pnl, comission * -1.0);
                                            MEM.lock().unwrap().new_close_long_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        }
                                    } else {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "SELL" {
                                            println!("\x1b[92mOrder filled OPEN {0:.1$} SHORT at {2:.3$} with {4:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, comission * -1.0);
                                            MEM.lock().unwrap().new_open_short_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                            let _ = tokio::spawn(task_set_last_short_open(msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap()));
                                        } else {
                                            println!("\x1b[92mOrder filled CLOSE {0:.1$} SHORT at {2:.3$} with {4:.3$} PNL and {5:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, pnl, comission * -1.0);
                                            MEM.lock().unwrap().new_close_short_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        }
                                    }
                                },
                                "PARTIALLY_FILLED" => {
                                    let comission = msg_json["data"]["o"]["n"].as_str().unwrap().parse::<f64>().unwrap();
                                    let pnl = msg_json["data"]["o"]["rp"].as_str().unwrap().parse::<f64>().unwrap();
                                    if msg_json["data"]["o"]["ps"].as_str().unwrap() == "LONG" {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "BUY" {
                                            println!("\x1b[92mOrder PARTIALLY filled OPEN {0:.1$} LONG at {2:.3$} with {4:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, comission * -1.0);
                                            MEM.lock().unwrap().new_open_long_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        } else {
                                            println!("\x1b[92mOrder PARTIALLY filled CLOSE {0:.1$} LONG at {2:.3$} with {4:.3$} PNL and {5:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, pnl, comission * -1.0);
                                            MEM.lock().unwrap().new_close_long_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        }
                                    } else {
                                        if msg_json["data"]["o"]["S"].as_str().unwrap() == "SELL" {
                                            println!("\x1b[92mOrder PARTIALLY filled OPEN {0:.1$} SHORT at {2:.3$} with {4:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, comission * -1.0);
                                            MEM.lock().unwrap().new_open_short_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        } else {
                                            println!("\x1b[92mOrder PARTIALLY filled CLOSE {0:.1$} SHORT at {2:.3$} with {4:.3$} PNL and {5:.4} comission\x1b[0m", msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap(), quantity_decimal_count, msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), price_decimal_count, pnl, comission * -1.0);
                                            MEM.lock().unwrap().new_close_short_filled(msg_json["data"]["o"]["l"].as_str().unwrap().parse::<f64>().unwrap() * msg_json["data"]["o"]["L"].as_str().unwrap().parse::<f64>().unwrap(), comission, pnl);
                                        }
                                    }
                                },
                                _ => ()
                            }
                        },
                        "ACCOUNT_UPDATE" => {
                            for i in 0..msg_json["data"]["a"]["B"].len() {
                                if msg_json["data"]["a"]["B"][i]["a"].as_str().unwrap() == token {
                                    { MEM.lock().unwrap().set_balance(msg_json["data"]["a"]["B"][i]["wb"].as_str().unwrap().parse::<f64>().unwrap()) };
                                    break;
                                }
                            }
                            for i in 0..msg_json["data"]["a"]["P"].len() {
                                if msg_json["data"]["a"]["P"][i]["s"].as_str().unwrap() == pair {
                                    match msg_json["data"]["a"]["P"][i]["ps"].as_str().unwrap() {
                                        "LONG" => {
                                            MEM.lock().unwrap().set_current_longs(
                                                msg_json["data"]["a"]["P"][i]["pa"].as_str().unwrap().parse::<f64>().unwrap(),
                                                msg_json["data"]["a"]["P"][i]["ep"].as_str().unwrap().parse::<f64>().unwrap(),
                                                msg_json["data"]["a"]["P"][i]["up"].as_str().unwrap().parse::<f64>().unwrap()
                                            );
                                        },
                                        "SHORT" => {
                                            MEM.lock().unwrap().set_current_shorts(
                                                msg_json["data"]["a"]["P"][i]["pa"].as_str().unwrap().parse::<f64>().unwrap() * -1.0,
                                                msg_json["data"]["a"]["P"][i]["ep"].as_str().unwrap().parse::<f64>().unwrap(),
                                                msg_json["data"]["a"]["P"][i]["up"].as_str().unwrap().parse::<f64>().unwrap()
                                            );
                                        },
                                        _ => ()
                                    }
                                }
                            }
                        },
                        _ => {
                            println!("Unknown update: {}", msg_json["data"]);
                        }
                    }
                }
            }
        }
    }
}

async fn task_trader() {
    println!("Canceling all orders");
    cancel_all_orders().await;
    println!("Getting exchange infos");
    get_exchange_info().await;
    println!("Applying inital settings");
    apply_intial_settings().await;
    println!("Getting account infos");
    verify_account_info().await;
    println!("Posting initial orders");
    let current_orders = post_initial_orders().await;
    println!("Bot ready and listening");
    trader_loop(current_orders).await;
}

async fn get_exchange_info() {
    let pair = { MEM.lock().unwrap().get_pair() };
    let resp = get_request("/fapi/v1/exchangeInfo").await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to get_exchange_info(). Got the following response from server: {}\x1b[0m", resp));
    let mut triggered = false;
    let mut can_limit = false;
    let mut can_gtx = false;
    let mut token = String::from("");
    let mut price_decimal = 0.0;
    let mut price_decimal_count = 0;
    let mut quantity_decimal = 0.0;
    let mut quantity_decimal_count = 0;
    let mut min_quantity = 0.0;
    let mut max_quantity = 0.0;
    let mut max_order_amount = 0;
    for i in 0..json_resp["symbols"].len() {
        if json_resp["symbols"][i]["symbol"].as_str().unwrap() == pair {
            triggered = true;
            if json_resp["symbols"][i]["status"].as_str().unwrap() != "TRADING" {
                panic!("\x1b[91mERROR: The pair {} exists but is not available for trading\x1b[0m", pair);
            }
            token = String::from(json_resp["symbols"][i]["marginAsset"].as_str().unwrap());
            price_decimal_count = json_resp["symbols"][i]["pricePrecision"].as_i64().unwrap();
            quantity_decimal_count = json_resp["symbols"][i]["quantityPrecision"].as_i64().unwrap();
            for j in 0..json_resp["symbols"][i]["filters"].len() {
                match json_resp["symbols"][i]["filters"][j]["filterType"].as_str().unwrap() {
                    "PRICE_FILTER" => {
                        price_decimal = json_resp["symbols"][i]["filters"][j]["tickSize"].as_str().unwrap().parse::<f64>().unwrap();
                    },
                    "LOT_SIZE" => {
                        quantity_decimal = json_resp["symbols"][i]["filters"][j]["stepSize"].as_str().unwrap().parse::<f64>().unwrap();
                        min_quantity = json_resp["symbols"][i]["filters"][j]["minQty"].as_str().unwrap().parse::<f64>().unwrap();
                        max_quantity = json_resp["symbols"][i]["filters"][j]["maxQty"].as_str().unwrap().parse::<f64>().unwrap();
                    },
                    "MAX_NUM_ORDERS" => {
                        max_order_amount = json_resp["symbols"][i]["filters"][j]["limit"].as_u64().unwrap();
                    },
                    _ => ()
                }
            }
            for j in 0..json_resp["symbols"][i]["orderTypes"].len() {
                if json_resp["symbols"][i]["orderTypes"][j].as_str().unwrap() == "LIMIT" {
                    can_limit = true;
                    break;
                }
            }
            for j in 0..json_resp["symbols"][i]["timeInForce"].len() {
                if json_resp["symbols"][i]["timeInForce"][j].as_str().unwrap() == "GTX" {
                    can_gtx = true;
                    break;
                }
            }
            break;
        }
    }
    if !triggered {
        panic!("\x1b[91mERROR: The pair {} does not exists\x1b[0m", pair);
    }
    if !can_limit {
        panic!("\x1b[91mERROR: The pair {} does not support limit orders\x1b[0m", pair);
    }
    if !can_gtx {
        panic!("\x1b[91mERROR: The pair {} does not support GTX orders\x1b[0m", pair);
    }
    { MEM.lock().unwrap().set_exchange_info(token, price_decimal, price_decimal_count, quantity_decimal, quantity_decimal_count, min_quantity, max_quantity, max_order_amount) };
}

async fn apply_intial_settings() {
    set_leverage().await;
    set_position_mode().await;
    set_multiassets_mode().await;
    set_margin_mode().await;
}

async fn verify_account_info() {
    let account_info = get_account_info().await;
    let (pair, _quantity_decimal_count, _price_decimal_count, token, _max_position) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_pair(), local_mem.get_quantity_decimal_count(), local_mem.get_price_decimal_count(), local_mem.get_token(), local_mem.get_max_position())
    };
    {
        MEM.lock().unwrap().set_vip_level(
            account_info["feeTier"].as_u64().unwrap()
        );
    }
    for i in 0..account_info["assets"].len() {
        if account_info["assets"][i]["asset"].as_str().unwrap() == token {
            MEM.lock().unwrap().set_balance(
                account_info["assets"][i]["walletBalance"].as_str().unwrap().parse::<f64>().unwrap()
            );
        }
    }
    for i in 0..account_info["positions"].len() {
        if account_info["positions"][i]["symbol"].as_str().unwrap() == pair {
            match account_info["positions"][i]["positionSide"].as_str().unwrap() {
                "LONG" => {
                    MEM.lock().unwrap().set_current_longs(
                        account_info["positions"][i]["positionAmt"].as_str().unwrap().parse::<f64>().unwrap(),
                        account_info["positions"][i]["entryPrice"].as_str().unwrap().parse::<f64>().unwrap(),
                        account_info["positions"][i]["unrealizedProfit"].as_str().unwrap().parse::<f64>().unwrap()
                    );
                },
                "SHORT" => {
                    MEM.lock().unwrap().set_current_shorts(
                        account_info["positions"][i]["positionAmt"].as_str().unwrap().parse::<f64>().unwrap() * -1.0,
                        account_info["positions"][i]["entryPrice"].as_str().unwrap().parse::<f64>().unwrap(),
                        account_info["positions"][i]["unrealizedProfit"].as_str().unwrap().parse::<f64>().unwrap()
                    );
                },
                _ => ()
            }
        }
    }
    if !account_info["canTrade"].as_bool().unwrap_or(false) {
        panic!("\x1b[91mERROR: Your account is not allowed to trade\x1b[0m");
    }
    print_stats().await;
}

async fn post_initial_orders() -> (Vec<Order>, Vec<Order>, Order, Order) {
    let (
        order_amount,
        order_quantity,
        margin,
        open_long_pool,
        close_long_pool,
        open_short_pool,
        close_short_pool,
        (close_long_price, close_short_price),
        quantity_decimal_half
    ) = { let local_mem = &MEM.lock().unwrap();
        (
            local_mem.get_order_amount(),
            local_mem.get_order_quantity(),
            local_mem.get_margin(),
            local_mem.max_open_long(),
            local_mem.max_close_long(),
            local_mem.max_open_short(),
            local_mem.max_close_short(),
            local_mem.get_close_prices(),
            local_mem.get_quantity_decimal_half()
        )
    };
    let market_price = wait_marketprice().await;
    let (long_increment, short_increment) = { MEM.lock().unwrap().get_increments() };
    let mut upper_price = market_price + margin;
    let mut lower_price = market_price - margin;
    let open_long_quantities = distribute_quantity(order_amount as usize, order_quantity, open_long_pool, quantity_decimal_half);
    let open_short_quantities = distribute_quantity(order_amount as usize, order_quantity, open_short_pool, quantity_decimal_half);
    let mut open_long_orders: Vec<Order> = Vec::with_capacity(order_amount as usize);
    let mut open_short_orders: Vec<Order> = Vec::with_capacity(order_amount as usize);
    let mut orders_to_post: Vec<Order> = vec![];
    for i in 0..order_amount as usize {
        open_long_orders.push(Order::new(lower_price, open_long_quantities[i], true, true));
        orders_to_post.push(open_long_orders[i].clone());
        open_short_orders.push(Order::new(upper_price, open_short_quantities[i], true, false));
        orders_to_post.push(open_short_orders[i].clone());
        upper_price += short_increment;
        lower_price -= long_increment;
    }
    let close_long_order = Order::new(close_long_price, close_long_pool, false, true);
    orders_to_post.push(close_long_order.clone());
    let close_short_order = Order::new(close_short_price, close_short_pool, false, false);
    orders_to_post.push(close_short_order.clone());
    post_multiple_orders(orders_to_post).await;
    (open_long_orders, open_short_orders, close_long_order, close_short_order)
}

async fn trader_loop(current_orders: (Vec<Order>, Vec<Order>, Order, Order)) {
    let (mut last_long_increment, mut last_short_increment) = { MEM.lock().unwrap().get_increments() };
    let (
        mut open_long_orders,
        mut open_short_orders,
        mut close_long_order,
        mut close_short_order
    ) = current_orders;
    let (
        order_quantity, 
        order_amount, 
        quantity_decimal_half
    ) = { let local_mem = &MEM.lock().unwrap();
        (
            local_mem.get_order_quantity(),
            local_mem.get_order_amount(),
            local_mem.get_quantity_decimal_half()
        )
    };
    loop {
        sleep(Duration::ZERO).await;
        if MEM.lock().unwrap().is_oveflowing() {
            cancel_all_orders().await;
            sleep(Duration::from_secs(2)).await;
            cancel_all_orders().await;
            sleep(Duration::from_secs(15)).await;
            { MEM.lock().unwrap().reset_all() };
            println!("Reposting orders");
            (open_long_orders, open_short_orders, close_long_order, close_short_order) = post_initial_orders().await;
            println!("Bot ready and listening");
        } else {
            let (
                top_ask,
                top_bid,
                mut open_long_shift_down,
                mut open_short_shift_up,
                close_long_repost,
                close_short_repost,
                current_longs,
                current_shorts,
                long_close_price,
                short_close_price,
                long_increment,
                short_increment
            ) = { MEM.lock().unwrap().get_updates() };
            let mut orders_to_cancel: Vec<Order> = vec![];
            let mut orders_to_post: Vec<Order> = vec![];
            if open_long_shift_down == 0 && !open_long_orders[0].is_real() {
                let mut target_price = open_long_orders[0].get_price();
                loop {
                    if target_price > top_bid {
                        open_long_shift_down += 1;
                        target_price -= last_long_increment;
                    } else {
                        break;
                    }
                }
            }
            if open_short_shift_up == 0 && !open_short_orders[0].is_real() {
                let mut target_price = open_short_orders[0].get_price();
                loop {
                    if target_price < top_ask {
                        open_short_shift_up += 1;
                        target_price += last_short_increment;
                    } else {
                        break;
                    }
                }
            }
            // Open Longs    ----------------------------------------------------------------------------------------------
            if open_long_shift_down > 0 {
                if last_long_increment != long_increment {    // Increment change START ------------------------
                    println!("\x1b[95mLong increment change: {} -> {}\x1b[0m", last_long_increment, long_increment);
                    last_long_increment = long_increment;
                    let open_long_pool = { MEM.lock().unwrap().max_open_long() };
                    let quantities = distribute_quantity(order_amount as usize, order_quantity, open_long_pool, quantity_decimal_half);
                    for i in 0..order_amount as usize {
                        orders_to_cancel.push(open_long_orders[i].clone());
                        open_long_orders[i] = Order::new(top_bid - (last_long_increment * i as f64), quantities[i], true, true);
                        orders_to_post.push(open_long_orders[i].clone());
                    }
                } else {                                        // Increment change END ------------------------
                    println!("Shifting open longs down {}", open_long_shift_down);
                    if open_long_shift_down >= order_amount {
                        for i in 0..order_amount as usize {
                            open_long_orders[i] = Order::new(open_long_orders[i].get_price() - open_long_shift_down as f64 * last_long_increment, 0.0, true, true);
                        }
                    } else {
                        for i in 0..open_long_shift_down as usize {
                            open_long_orders[i] = Order::new(open_long_orders[i].get_price() - order_amount as f64 * last_long_increment, 0.0, true, true);
                        }
                        open_long_orders.rotate_left(open_long_shift_down as usize);
                    }
                    let open_long_pool = { MEM.lock().unwrap().max_open_long() };
                    let quantities = distribute_quantity(order_amount as usize, order_quantity, open_long_pool, quantity_decimal_half);
                    for i in 0..order_amount as usize {
                        if open_long_orders[i].get_quantity() != quantities[i] {
                            orders_to_cancel.push(open_long_orders[i].clone());
                            open_long_orders[i] = Order::new(open_long_orders[i].get_price(), quantities[i], true, true);
                            orders_to_post.push(open_long_orders[i].clone());
                        }
                    }
                }
            } else {
                let mut open_long_shift_up = 0;
                let mut target_price = open_long_orders[0].get_price() + last_long_increment;
                loop {
                    if target_price <= top_bid {
                        open_long_shift_up += 1;
                        target_price += last_long_increment;
                    } else {
                        break;
                    }
                }
                if open_long_shift_up > 0 {
                    if last_long_increment != long_increment {    // Increment change START ------------------------
                        println!("\x1b[95mLong increment change: {} -> {}\x1b[0m", last_long_increment, long_increment);
                        last_long_increment = long_increment;
                        let open_long_pool = { MEM.lock().unwrap().max_open_long() };
                        let quantities = distribute_quantity(order_amount as usize, order_quantity, open_long_pool, quantity_decimal_half);
                        for i in 0..order_amount as usize {
                            orders_to_cancel.push(open_long_orders[i].clone());
                            open_long_orders[i] = Order::new(top_bid - (last_long_increment * i as f64), quantities[i], true, true);
                            orders_to_post.push(open_long_orders[i].clone());
                        }
                    } else {                                        // Increment change END ------------------------
                        println!("Shifting open longs up {}", open_long_shift_up);
                        if open_long_shift_up >= order_amount {
                            for i in 0..order_amount as usize {
                                orders_to_cancel.push(open_long_orders[i].clone());
                                open_long_orders[i] = Order::new(open_long_orders[i].get_price() + open_long_shift_up as f64 * last_long_increment, 0.0, true, true);
                            }
                        } else {
                            open_long_orders.rotate_right(open_long_shift_up as usize);
                            for i in 0..open_long_shift_up as usize {
                                orders_to_cancel.push(open_long_orders[i].clone());
                                open_long_orders[i] = Order::new(open_long_orders[i].get_price() + order_amount as f64 * last_long_increment, 0.0, true, true);
                            }
                        }
                        let open_long_pool = { MEM.lock().unwrap().max_open_long() };
                        let quantities = distribute_quantity(order_amount as usize, order_quantity, open_long_pool, quantity_decimal_half);
                        for i in 0..order_amount as usize {
                            if open_long_orders[i].get_quantity() != quantities[i] {
                                orders_to_cancel.push(open_long_orders[i].clone());
                                open_long_orders[i] = Order::new(open_long_orders[i].get_price(), quantities[i], true, true);
                                orders_to_post.push(open_long_orders[i].clone());
                            }
                        }
                    }
                }
            }
            // Open Longs    ----------------------------------------------------------------------------------------------
            // Open Shorts   ----------------------------------------------------------------------------------------------
            if open_short_shift_up > 0 {
                if last_short_increment != short_increment {    // Increment change START ------------------------
                    println!("\x1b[95mShort increment change: {} -> {}\x1b[0m", last_short_increment, short_increment);
                    last_short_increment = short_increment;
                    let open_short_pool = { MEM.lock().unwrap().max_open_short() };
                    let quantities = distribute_quantity(order_amount as usize, order_quantity, open_short_pool, quantity_decimal_half);
                    for i in 0..order_amount as usize {
                        orders_to_cancel.push(open_short_orders[i].clone());
                        open_short_orders[i] = Order::new(top_ask + (last_short_increment * i as f64), quantities[i], true, false);
                        orders_to_post.push(open_short_orders[i].clone());
                    }
                } else {                                        // Increment change END ------------------------
                    println!("Shifting open shorts up {}", open_short_shift_up);
                    if open_short_shift_up >= order_amount {
                        for i in 0..order_amount as usize {
                            open_short_orders[i] = Order::new(open_short_orders[i].get_price() + open_short_shift_up as f64 * last_short_increment, 0.0, true, false);
                        }
                    } else {
                        for i in 0..open_short_shift_up as usize {
                            open_short_orders[i] = Order::new(open_short_orders[i].get_price() + order_amount as f64 * last_short_increment, 0.0, true, false);
                        }
                        open_short_orders.rotate_left(open_short_shift_up as usize);
                    }
                    let open_short_pool = { MEM.lock().unwrap().max_open_short() };
                    let quantities = distribute_quantity(order_amount as usize, order_quantity, open_short_pool, quantity_decimal_half);
                    for i in 0..order_amount as usize {
                        if open_short_orders[i].get_quantity() != quantities[i] {
                            orders_to_cancel.push(open_short_orders[i].clone());
                            open_short_orders[i] = Order::new(open_short_orders[i].get_price(), quantities[i], true, false);
                            orders_to_post.push(open_short_orders[i].clone());
                        }
                    }
                }
            } else {
                let mut open_short_shift_down = 0;
                let mut target_price = open_short_orders[0].get_price() - last_short_increment;
                loop {
                    if target_price >= top_ask {
                        open_short_shift_down += 1;
                        target_price -= last_short_increment;
                    } else {
                        break;
                    }
                }
                if open_short_shift_down > 0 {
                    if last_short_increment != short_increment {    // Increment change START ------------------------
                        println!("\x1b[95mShort increment change: {} -> {}\x1b[0m", last_short_increment, short_increment);
                        last_short_increment = short_increment;
                        let open_short_pool = { MEM.lock().unwrap().max_open_short() };
                        let quantities = distribute_quantity(order_amount as usize, order_quantity, open_short_pool, quantity_decimal_half);
                        for i in 0..order_amount as usize {
                            orders_to_cancel.push(open_short_orders[i].clone());
                            open_short_orders[i] = Order::new(top_ask + (last_short_increment * i as f64), quantities[i], true, false);
                            orders_to_post.push(open_short_orders[i].clone());
                        }
                    } else {                                        // Increment change END ------------------------
                        println!("Shifting open shorts down {}", open_short_shift_down);
                        if open_short_shift_down >= order_amount {
                            for i in 0..order_amount as usize {
                                orders_to_cancel.push(open_short_orders[i].clone());
                                open_short_orders[i] = Order::new(open_short_orders[i].get_price() - open_short_shift_down as f64 * last_short_increment, 0.0, true, false);
                            }
                        } else {
                            open_short_orders.rotate_right(open_short_shift_down as usize);
                            for i in 0..open_short_shift_down as usize {
                                orders_to_cancel.push(open_short_orders[i].clone());
                                open_short_orders[i] = Order::new(open_short_orders[i].get_price() - order_amount as f64 * last_short_increment, 0.0, true, false);
                            }
                        }
                        let open_short_pool = { MEM.lock().unwrap().max_open_short() };
                        let quantities = distribute_quantity(order_amount as usize, order_quantity, open_short_pool, quantity_decimal_half);
                        for i in 0..order_amount as usize {
                            if open_short_orders[i].get_quantity() != quantities[i] {
                                orders_to_cancel.push(open_short_orders[i].clone());
                                open_short_orders[i] = Order::new(open_short_orders[i].get_price(), quantities[i], true, false);
                                orders_to_post.push(open_short_orders[i].clone());
                            }
                        }
                    }
                }
            }
            // Open Shorts   ----------------------------------------------------------------------------------------------
            // Close Longs   ----------------------------------------------------------------------------------------------
            if close_long_repost || (current_longs - close_long_order.get_quantity()).abs() > 0.005 || (long_close_price - close_long_order.get_price()).abs() > 5.0 {
                if !close_long_repost {
                    orders_to_cancel.push(close_long_order);
                }
                close_long_order = Order::new(long_close_price, current_longs, false, true);
                orders_to_post.push(close_long_order.clone());
            }
            // Close Longs   ----------------------------------------------------------------------------------------------
            // Close Shorts  ----------------------------------------------------------------------------------------------
            if close_short_repost || (current_shorts - close_short_order.get_quantity()).abs() > 0.005 || (short_close_price - close_short_order.get_price()).abs() > 5.0 {
                if !close_short_repost {
                    orders_to_cancel.push(close_short_order);
                }
                close_short_order = Order::new(short_close_price, current_shorts, false, false);
                orders_to_post.push(close_short_order.clone());
            }
            // Close Shorts  ----------------------------------------------------------------------------------------------
            cancel_multiple_orders(orders_to_cancel).await;
            post_multiple_orders(orders_to_post).await;
            sleep(Duration::ZERO).await;
        }
    }
}

fn distribute_quantity(amount: usize, target_quantity: f64, mut pool: f64, quantity_decimal_half: f64) -> Vec<f64> {
    let mut quantities: Vec<f64> = vec![0.0; amount];
    for i in 0..amount {
        if pool > target_quantity {
            quantities[i] = target_quantity;
            pool -= target_quantity;
        } else {
            if pool >= quantity_decimal_half {
                quantities[i] = pool;
            }
            break;
        }
    }
    quantities
}

async fn wait_marketprice() -> f64 {
    loop {
        sleep(Duration::ZERO).await;
        let marketprice = { MEM.lock().unwrap().get_marketprice() };
        if marketprice > 0.0 {
            MEM.lock().unwrap().set_last_prices();
            return marketprice
        }
    }
}

async fn wait_token() -> String {
    loop {
        sleep(Duration::ZERO).await;
        let token = { MEM.lock().unwrap().get_token() };
        if token != "" {
            return token
        }
    }
}

fn get_timestamp() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - 1000
}

async fn cancel_all_orders() {
    let resp = delete_request("/fapi/v1/allOpenOrders", &format!("symbol={}&", { MEM.lock().unwrap().get_pair() })).await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to cancel_all_orders(). Got the following response from server: {}\x1b[0m", resp));
    if json_resp["code"].as_i64().expect(&format!("\x1b[91mERROR: Failed to cancel_all_orders(). Got the following response from server: {}\x1b[0m", resp)) != 200 {
        panic!("\x1b[91mERROR: Failed to cancel_all_orders(). Got the following response from server: {}\x1b[0m", resp);
    }
}

async fn set_leverage() {
    let (pair, leverage) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_pair(), local_mem.get_leverage())
    };
    let resp = post_request("/fapi/v1/leverage", &format!("symbol={}&leverage={}&", pair, leverage)).await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to set_leverage(). Got the following response from server: {}\x1b[0m", resp));
    if json_resp["leverage"].as_u64().expect(&format!("\x1b[91mERROR: Failed to set_leverage(). Got the following response from server: {}\x1b[0m", resp)) != leverage {
        panic!("\x1b[91mERROR: Failed to set_leverage(). Got the following response from server: {}\x1b[0m", resp);
    }
}

async fn set_position_mode() {
    let resp = post_request("/fapi/v1/positionSide/dual", "dualSidePosition=true&").await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to set_position_mode(). Got the following response from server: {}\x1b[0m", resp));
    let code = json_resp["code"].as_i64().expect(&format!("\x1b[91mERROR: Failed to set_position_mode(). Got the following response from server: {}\x1b[0m", resp));
    if code != 200 && code != -4059 {
        panic!("\x1b[91mERROR: Failed to set_position_mode(). Got the following response from server: {}\x1b[0m", resp);
    }
}

async fn set_multiassets_mode() {
    let resp = post_request("/fapi/v1/multiAssetsMargin", "multiAssetsMargin=false&").await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to set_multiassets_mode(). Got the following response from server: {}\x1b[0m", resp));
    let code = json_resp["code"].as_i64().expect(&format!("\x1b[91mERROR: Failed to set_multiassets_mode(). Got the following response from server: {}\x1b[0m", resp));
    if code != 200 && code != -4171 {
        panic!("\x1b[91mERROR: Failed to set_multiassets_mode(). Got the following response from server: {}\x1b[0m", resp);
    }
}

async fn set_margin_mode() {
    let resp = post_request("/fapi/v1/marginType", &format!("symbol={}&marginType=CROSSED&", { &MEM.lock().unwrap().get_pair() })).await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to set_margin_mode(). Got the following response from server: {}\x1b[0m", resp));
    let code = json_resp["code"].as_i64().expect(&format!("\x1b[91mERROR: Failed to set_margin_mode(). Got the following response from server: {}\x1b[0m", resp));
    if code != 200 && code != -4046 {
        panic!("\x1b[91mERROR: Failed to set_margin_mode(). Got the following response from server: {}\x1b[0m", resp);
    }
}

async fn get_account_info() -> json::JsonValue {
    let resp = get_request("/fapi/v2/account").await;
    json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to get_account_info(). Got the following response from server: {}\x1b[0m", resp))
}

async fn get_listen_key() -> String {
    let resp = post_request("/fapi/v1/listenKey", "").await;
    let json_resp = json::parse(&resp).expect(&format!("\x1b[91mERROR: Failed to get_listen_key(). Got the following response from server: {}\x1b[0m", resp));
    String::from(json_resp["listenKey"].as_str().expect(&format!("\x1b[91mERROR: Failed to get_listen_key(). Got the following response from server: {}\x1b[0m", resp)))
}

async fn send_keepalive() -> json::JsonValue {
    put_request("/fapi/v1/listenKey").await
}

async fn delete_request(endpoint: &str, params: &str) -> String {
    let (client, header,mut signature, base_url) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header(), local_mem.get_signature(), local_mem.get_url_request())
    };
    let payload = format!("{}timestamp={}", params, get_timestamp());
    signature.update(payload.as_bytes());
    let url = format!("{}{}?{}&signature={}", base_url, endpoint, payload, hex_encode(signature.finalize().into_bytes()));
    client.delete(url).headers(header).send().await.unwrap().text().await.unwrap()
}

async fn get_request(endpoint: &str) -> String {
    let (client, header,mut signature, base_url) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header(), local_mem.get_signature(), local_mem.get_url_request())
    };
    let payload = format!("timestamp={}", get_timestamp());
    signature.update(payload.as_bytes());
    let url = format!("{}{}?{}&signature={}", base_url, endpoint, payload, hex_encode(signature.finalize().into_bytes()));
    client.get(url).headers(header).send().await.unwrap().text().await.unwrap()
}

async fn post_request(endpoint: &str, params: &str) -> String {
    let (client, header,mut signature, base_url) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header(), local_mem.get_signature(), local_mem.get_url_request())
    };
    let payload = format!("{}timestamp={}", params, get_timestamp());
    signature.update(payload.as_bytes());
    let url = format!("{}{}?{}&signature={}", base_url, endpoint, payload, hex_encode(signature.finalize().into_bytes()));
    client.post(url).headers(header).send().await.unwrap().text().await.unwrap()
}

async fn put_request(endpoint: &str) -> json::JsonValue {
    let (client, header,mut signature, base_url) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header(), local_mem.get_signature(), local_mem.get_url_request())
    };
    let payload = format!("timestamp={}", get_timestamp());
    signature.update(payload.as_bytes());
    let url = format!("{}{}?{}&signature={}", base_url, endpoint, payload, hex_encode(signature.finalize().into_bytes()));
    let resp: &str = &client.put(url).headers(header).send().await.unwrap().text().await.unwrap();
    json::parse(resp).unwrap()
}

async fn post_multiple_orders(orders_to_post: Vec<Order>) {
    if orders_to_post.len() > 0 {
        let (signature, base_url, pair, price_decimal_count, quantity_decimal_count) = {
            let local_mem = &MEM.lock().unwrap();
            (local_mem.get_signature(), local_mem.get_url_request(), local_mem.get_pair(), local_mem.get_price_decimal_count(), local_mem.get_quantity_decimal_count())
        };
        let mut vec_of_orders = vec![];
        let mut vec_of_vec_of_orders = vec![];
        for i in 0..orders_to_post.len() {
            if orders_to_post[i].is_real() {
                vec_of_orders.push(orders_to_post[i].clone());
                if vec_of_orders.len() >= 5 {
                    vec_of_vec_of_orders.push(vec_of_orders);
                    vec_of_orders = vec![];
                }
            }
        }
        if vec_of_orders.len() > 0 {
            vec_of_vec_of_orders.push(vec_of_orders);
        }
        let timestamp = get_timestamp();
        let mut urls: Vec<String> = vec![];
        for i in 0..vec_of_vec_of_orders.len() {
            if vec_of_vec_of_orders[i].len() == 1 {
                let payload = format!("{}&timestamp={}", vec_of_vec_of_orders[i][0].to_single_url_string(&pair, price_decimal_count, quantity_decimal_count), timestamp);
                let mut current_signature = signature.clone();
                current_signature.update(payload.as_bytes());
                urls.push(format!("{}/fapi/v1/order?{}&signature={}", base_url, payload, hex_encode(current_signature.finalize().into_bytes())));
            } else {
                let mut batch_orders: String = String::from("[");
                for j in 0..vec_of_vec_of_orders[i].len() {
                    if j > 0 {
                        batch_orders.push_str(&format!(",{}", vec_of_vec_of_orders[i][j].to_url_string(&pair, price_decimal_count, quantity_decimal_count)));
                    } else {
                        batch_orders.push_str(&vec_of_vec_of_orders[i][j].to_url_string(&pair, price_decimal_count, quantity_decimal_count));
                    }
                }
                batch_orders.push_str("]");
                let payload = format!("batchOrders={}&timestamp={}", urlencoding::encode(&batch_orders), timestamp);
                let mut current_signature = signature.clone();
                current_signature.update(payload.as_bytes());
                urls.push(format!("{}/fapi/v1/batchOrders?{}&signature={}", base_url, payload, hex_encode(current_signature.finalize().into_bytes())));
            }
        }
        if urls.len() > 0 {
            parallel_post_requests(urls).await;
        }
    }
}

async fn cancel_multiple_orders(orders_to_cancel: Vec<Order>) {
    if orders_to_cancel.len() > 0 {
        let (signature, base_url, pair) = {
            let local_mem = &MEM.lock().unwrap();
            (local_mem.get_signature(), local_mem.get_url_request(), local_mem.get_pair())
        };
        let mut vec_of_orders = vec![];
        let mut vec_of_vec_of_orders = vec![];
        for i in 0..orders_to_cancel.len() {
            if orders_to_cancel[i].is_real() {
                vec_of_orders.push(orders_to_cancel[i].clone());
                if vec_of_orders.len() >= 5 {
                    vec_of_vec_of_orders.push(vec_of_orders);
                    vec_of_orders = vec![];
                }
            }
        }
        if vec_of_orders.len() > 0 {
            vec_of_vec_of_orders.push(vec_of_orders);
        }
        let timestamp = get_timestamp();
        let mut urls: Vec<String> = vec![];
        for i in 0..vec_of_vec_of_orders.len() {
            if vec_of_vec_of_orders[i].len() == 1 {
                let payload = format!("symbol={}&origClientOrderId={}&timestamp={}", pair, vec_of_vec_of_orders[i][0].get_id(), timestamp);
                let mut current_signature = signature.clone();
                current_signature.update(payload.as_bytes());
                urls.push(format!("{}/fapi/v1/order?{}&signature={}", base_url, payload, hex_encode(current_signature.finalize().into_bytes())));
            } else {
                let mut order_ids: String = String::from("[");
                for j in 0..vec_of_vec_of_orders[i].len() {
                    if j > 0 {
                        order_ids.push_str(&format!(",\"{}\"", vec_of_vec_of_orders[i][j].get_id()));
                    } else {
                        order_ids.push_str(&format!("\"{}\"", vec_of_vec_of_orders[i][j].get_id()));
                    }
                }
                order_ids.push_str("]");
                let payload = format!("symbol={}&origClientOrderIdList={}&timestamp={}", pair, urlencoding::encode(&order_ids), timestamp);
                let mut current_signature = signature.clone();
                current_signature.update(payload.as_bytes());
                urls.push(format!("{}/fapi/v1/batchOrders?{}&signature={}", base_url, payload, hex_encode(current_signature.finalize().into_bytes())));
            }
        }
        if urls.len() > 0 {
            parallel_delete_requests(urls).await;
        }
    }
}

async fn parallel_post_requests(urls: Vec<String>) {
    let (client, header) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header())
    };
    let parallel_requests: usize = urls.len();
    let bodies = stream::iter(urls).map(|url| {
        let clonned_client = client.clone();
        let clonned_header = header.clone();
        tokio::spawn(async move {
            clonned_client.post(url).headers(clonned_header).send().await
        })
    }).buffer_unordered(parallel_requests);

    bodies.for_each(|resp| async {
        let my_text = &resp.unwrap().unwrap().text().await.unwrap();
        let resp_json = json::parse(my_text).ok();
        let mut failed: bool = false;
        if resp_json.is_some() {
            let resp_json_unwrapped = resp_json.unwrap();
            let single_status = resp_json_unwrapped["status"].as_str();
            if single_status.is_some() {
                if single_status.unwrap() != "NEW" {
                    failed = true;
                }
            } else {
                let code = resp_json_unwrapped["code"].as_i64();
                if code.is_some() {
                    if code.unwrap() != -2022 {
                        println!("\x1b[91mFailed to post close order\x1b[0m");
                        failed = true;
                    }
                } else {
                    for i in 0..resp_json_unwrapped.len() {
                        let status = resp_json_unwrapped[i]["status"].as_str();
                        if status.is_some() {
                            if status.unwrap() != "NEW" {
                                failed = true;
                                break
                            }
                        } else {
                            let code = resp_json_unwrapped[i]["code"].as_i64();
                            if code.is_some() {
                                if code.unwrap() != -2022 {
                                    println!("\x1b[91mFailed to post close order\x1b[0m");
                                    failed = true;
                                    break
                                }
                            } else {
                                failed = true;
                                break
                            }
                        }
                    }
                }
            }
        } else {
            failed = true;
        }
        if failed {
            println!("\x1b[91mFailed to post order\x1b[0m");
            println!("\x1b[91m{}\x1b[0m", my_text);
            order_overflow();
        }
    }).await;
}

async fn parallel_delete_requests(urls: Vec<String>) {
    let (client, header) = {
        let local_mem = &MEM.lock().unwrap();
        (local_mem.get_client(), local_mem.get_header())
    };
    let parallel_requests: usize = urls.len();
    let bodies = stream::iter(urls).map(|url| {
        let clonned_client = client.clone();
        let clonned_header = header.clone();
        tokio::spawn(async move {
            clonned_client.delete(url).headers(clonned_header).send().await
        })
    }).buffer_unordered(parallel_requests);

    bodies.for_each(|resp| async {
        let my_text = resp.unwrap().unwrap().text().await.unwrap();
        let resp_json = json::parse(&my_text).ok();
        let mut failed: bool = false;
        if resp_json.is_some() {
            let resp_json_unwrapped = resp_json.unwrap();
            let single_status = resp_json_unwrapped["status"].as_str();
            if single_status.is_some() {
                if single_status.unwrap() != "CANCELED" {
                    failed = true;
                }
            } else {
                for i in 0..resp_json_unwrapped.len() {
                    let status = resp_json_unwrapped[i]["status"].as_str();
                    if status.is_some() {
                        if status.unwrap() != "CANCELED" {
                            failed = true;
                            break
                        }
                    } else {
                        failed = true;
                        break
                    }
                }
            }
        } else {
            failed = true;
        }
        if failed {
            println!("\x1b[91mFailed to cancel order\x1b[0m");
        }
    }).await;
}

fn order_overflow() {
    println!("\x1b[91mOrder overflow\x1b[0m");
    { MEM.lock().unwrap().start_overflow() };
}