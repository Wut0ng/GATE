use hmac::{Hmac, Mac, digest::{core_api::{CoreWrapper, CtVariableCoreWrapper}, typenum::{UInt, UTerm, B1, B0}}, HmacCore};
use sha2::{Sha256, Sha256VarCore};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT, CONTENT_TYPE};
use std::{time::{Instant, Duration}};

pub struct MemoryManager {
    url_request: String,
    url_websocket: String,
    discord_token: String,
    discord_channel: String,
    pair: String,
    token: String,
    price_decimal: f64,
    price_decimal_count: i64,
    quantity_decimal: f64,
    quantity_decimal_half: f64,
    quantity_decimal_count: i64,
    leverage: u64,
    close_only: bool,
    margin: f64,
    increment: f64,
    close_diff: f64,
    order_amount: u64,
    order_quantity: f64,
    max_position: f64,
    client: reqwest::Client,
    header: HeaderMap,
    signature: CoreWrapper<HmacCore<CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>>>>>,
    top_ask: f64,
    top_bid: f64,
    current_longs: f64,
    current_shorts: f64,
    long_entry_price: f64,
    short_entry_price: f64,
    long_un_pnl: f64,
    short_un_pnl: f64,
    open_longs_filled: u64,
    open_longs_expired: u64,
    close_longs_filled: u64,
    close_longs_expired: u64,
    open_shorts_filled: u64,
    open_shorts_expired: u64,
    close_shorts_filled: u64,
    close_shorts_expired: u64,
    volume: f64,
    balance: f64,
    realized_profit: f64,
    commission: f64,
    order_overflow_count: u64,
    started_time: Instant,
    order_overflow: bool,
    need_restart: bool,
    vip_level: u64,
    soft_position: f64,
    acceleration: f64,
    half_range: f64,
    last_long_open: f64,
    last_short_open: f64
}

impl MemoryManager {
    pub fn new() -> Self {
        let config_str: String = std::fs::read_to_string("ressources/config.json").unwrap();
        let config_json: json::JsonValue = json::parse(&config_str).unwrap();

        let encrypted_key: String = new_magic_crypt!("encrypted_key", 256).decrypt_base64_to_string(String::from(config_json["encrypted_key"].as_str().unwrap())).unwrap();
        let encrypted_secret: String = new_magic_crypt!("encrypted_secret", 256).decrypt_base64_to_string(String::from(config_json["encrypted_secret"].as_str().unwrap())).unwrap();
        let api_key: String = new_magic_crypt!(&encrypted_secret, 256).decrypt_base64_to_string(encrypted_key).unwrap();
        let api_secret: String = new_magic_crypt!(&api_key, 256).decrypt_base64_to_string(encrypted_secret).unwrap();

        let encrypted_token: String = new_magic_crypt!("encrypted_token", 256).decrypt_base64_to_string(String::from(config_json["encrypted_token"].as_str().unwrap())).unwrap();
        let encrypted_channel: String = new_magic_crypt!("encrypted_channel", 256).decrypt_base64_to_string(String::from(config_json["encrypted_channel"].as_str().unwrap())).unwrap();
        let discord_token: String = new_magic_crypt!(&encrypted_channel, 256).decrypt_base64_to_string(encrypted_token).unwrap();
        let discord_channel: String = new_magic_crypt!(&discord_token, 256).decrypt_base64_to_string(encrypted_channel).unwrap();

        let mut url_request = String::from("https://fapi.binance.com");
        let mut url_websocket = String::from("wss://fstream.binance.com");

        if config_json["testnet"].as_bool().unwrap() {
            url_request = String::from("https://testnet.binancefuture.com");
            url_websocket = String::from("wss://stream.binancefuture.com");
        }

        let order_quantity = config_json["order_quantity"].as_f64().unwrap();
        let max_position = config_json["max_position"].as_f64().unwrap();
        let increment = config_json["increment"].as_f64().unwrap();
        let half_range = (0.9 * increment * max_position) / (2.0 * order_quantity);

        let mut header = HeaderMap::new();
        header.insert(USER_AGENT, HeaderValue::from_static("binance-rs"));
        header.insert(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));
        header.insert(HeaderName::from_static("x-mbx-apikey"), HeaderValue::from_str(&api_key).unwrap());

        MemoryManager {
            url_request,
            url_websocket,
            discord_token,
            discord_channel,
            pair: String::from(config_json["pair"].as_str().unwrap()),
            token: String::from(""),
            price_decimal: 0.0,
            price_decimal_count: 0,
            quantity_decimal: 0.0,
            quantity_decimal_half: 0.0,
            quantity_decimal_count: 0,
            leverage: config_json["leverage"].as_u64().unwrap(),
            close_only: config_json["close_only"].as_bool().unwrap(),
            margin: config_json["margin"].as_f64().unwrap(),
            close_diff: config_json["close_diff"].as_f64().unwrap(),
            increment,
            order_amount: config_json["order_amount"].as_u64().unwrap(),
            order_quantity,
            max_position,
            client: reqwest::Client::builder().pool_idle_timeout(None).build().unwrap(),
            header,
            signature: Hmac::<Sha256>::new_from_slice(api_secret.as_bytes()).unwrap(),
            top_ask: 0.0,
            top_bid: 0.0,
            current_longs: 0.0,
            current_shorts: 0.0,
            long_entry_price: -1.0,
            short_entry_price: -1.0,
            long_un_pnl: 0.0,
            short_un_pnl: 0.0,
            open_longs_filled: 0,
            open_longs_expired: 0,
            close_longs_filled: 0,
            close_longs_expired: 0,
            open_shorts_filled: 0,
            open_shorts_expired: 0,
            close_shorts_filled: 0,
            close_shorts_expired: 0,
            volume: 0.0,
            balance: 0.0,
            realized_profit: 0.0,
            commission: 0.0,
            order_overflow_count: 0,
            started_time: Instant::now(),
            order_overflow: false,
            need_restart: false,
            vip_level: 0,
            soft_position: config_json["soft_position"].as_f64().unwrap(),
            acceleration: config_json["acceleration"].as_f64().unwrap(),
            half_range,
            last_long_open: 99999.0,
            last_short_open: 0.0
        }
    }

    
    pub fn set_exchange_info(&mut self, token: String, price_decimal: f64, price_decimal_count: i64, quantity_decimal: f64, quantity_decimal_count: i64, min_quantity: f64, max_quantity: f64, max_order_amount: u64) {
        self.token = token;
        self.price_decimal = price_decimal;
        self.price_decimal_count = price_decimal_count;
        self.quantity_decimal = quantity_decimal;
        self.quantity_decimal_half = quantity_decimal / 2.0;
        self.quantity_decimal_count = quantity_decimal_count;
        if self.order_quantity < min_quantity || self.order_quantity > max_quantity {
            panic!("\x1b[91mERROR: Order quantity {} does not match min {} and max {}\x1b[0m", self.order_quantity, min_quantity, max_quantity);
        }
        if self.order_amount as f64 > (max_order_amount as f64 / 4.0) {
            panic!("\x1b[91mERROR: Order amount {} is too high for max order amount {}\x1b[0m", self.order_amount, max_order_amount);
        }
    }

    pub fn get_client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub fn get_header(&self) -> HeaderMap {
        self.header.clone()
    }

    pub fn get_signature(&self) -> CoreWrapper<HmacCore<CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>>>>> {
        self.signature.clone()
    }

    pub fn get_url_request(&self) -> String {
        self.url_request.clone()
    }
    
    pub fn get_url_websocket(&self) -> String {
        self.url_websocket.clone()
    }

    pub fn get_pair(&self) -> String {
        self.pair.clone()
    }

    pub fn get_token(&self) -> String {
        self.token.clone()
    }

    pub fn get_order_amount(&self) -> u64 {
        self.order_amount.clone()
    }

    pub fn get_margin(&self) -> f64 {
        self.margin.clone()
    }

    // pub fn get_base_increments(&self) -> (f64, f64) {
    //     (self.increment.clone(), self.increment.clone())
    // }

    pub fn get_leverage(&self) -> u64 {
        self.leverage.clone()
    }

    pub fn get_order_quantity(&self) -> f64 {
        self.order_quantity.clone()
    }

    pub fn get_max_position(&self) -> f64 {
        self.max_position.clone()
    }
    
    pub fn get_price_decimal_count(&self) -> i64 {
        self.price_decimal_count.clone()
    }

    pub fn get_quantity_decimal_half(&self) -> f64 {
        self.quantity_decimal_half.clone()
    }

    pub fn get_quantity_decimal_count(&self) -> i64 {
        self.quantity_decimal_count.clone()
    }

    pub fn set_marketprice(&mut self, top_ask: f64, top_bid: f64) {
        self.top_ask = top_ask;
        self.top_bid = top_bid;
    }

    pub fn get_marketprice(&self) -> f64 {
        if self.top_ask > 0.0 && self.top_bid > 0.0 {
            ((self.top_ask + self.top_bid) / (2.0 * self.price_decimal)).round() * self.price_decimal
        } else {
            0.0
        }
    }
    
    pub fn get_top_ask(&self) -> f64 {
        self.top_ask
    }

    pub fn get_top_bid(&self) -> f64 {
        self.top_bid
    }

    pub fn set_balance(&mut self, balance: f64) {
        self.balance = balance;
    }

    pub fn set_vip_level(&mut self, vip_level: u64) {
        self.vip_level = vip_level;
    }

    pub fn set_last_long_open(&mut self, price: f64) {
        // println!("Setting last long: {}", price);
        self.last_long_open = price;
    }

    pub fn set_last_short_open(&mut self, price: f64) {
        // println!("Setting last short: {}", price);
        self.last_short_open = price;
    }

    pub fn set_last_prices(&mut self) {
        let mp = self.get_marketprice();
        // println!("Setting last long: {}", mp);
        // println!("Setting last short: {}", mp);
        self.last_long_open = mp;
        self.last_short_open = mp;
    }

    pub fn get_increments(&self) -> (f64, f64) {
        (self.get_long_increment(), self.get_short_increment())
    }

    // pub fn set_current_longs_and_shorts(&mut self, long_quantity: f64, short_quantity: f64, long_entry_price: f64, short_entry_price: f64, long_un_pnl: f64, short_un_pnl: f64) {
    //     self.current_longs = long_quantity;
    //     self.current_shorts = short_quantity;
    //     self.long_entry_price = long_entry_price;
    //     self.short_entry_price = short_entry_price;
    //     self.long_un_pnl = long_un_pnl;
    //     self.short_un_pnl = short_un_pnl;
    // }

    pub fn set_current_longs(&mut self, long_quantity: f64, long_entry_price: f64, long_un_pnl: f64) {
        self.current_longs = long_quantity;
        self.long_entry_price = long_entry_price;
        self.long_un_pnl = long_un_pnl;
    }

    pub fn set_current_shorts(&mut self, short_quantity: f64, short_entry_price: f64, short_un_pnl: f64) {
        self.current_shorts = short_quantity;
        self.short_entry_price = short_entry_price;
        self.short_un_pnl = short_un_pnl;
    }

    // pub fn set_current_longs_shorts_balance(&mut self, long_quantity: f64, short_quantity: f64, balance: f64, long_entry_price: f64, short_entry_price: f64, long_un_pnl: f64, short_un_pnl: f64, vip_level: u64) {
    //     self.current_longs = long_quantity;
    //     self.current_shorts = short_quantity;
    //     self.balance = balance;
    //     self.long_entry_price = long_entry_price;
    //     self.short_entry_price = short_entry_price;
    //     self.long_un_pnl = long_un_pnl;
    //     self.short_un_pnl = short_un_pnl;
    //     self.vip_level = vip_level;
    // }

    pub fn get_open_long_filled(&mut self) -> u64 {
        let temp = self.open_longs_filled;
        self.open_longs_filled = 0;
        temp
    } 

    pub fn new_open_long_filled(&mut self, volume: f64, commission: f64, realized_profit: f64) {
        self.open_longs_filled += 1;
        self.volume += volume;
        self.commission += commission;
        self.realized_profit += realized_profit;
    }

    pub fn get_open_long_expired(&mut self) -> u64 {
        let temp = self.open_longs_expired;
        self.open_longs_expired = 0;
        temp
    }

    pub fn new_open_long_expired(&mut self) {
        self.open_longs_expired += 1;
    }

    pub fn new_close_long_filled(&mut self, volume: f64, commission: f64, realized_profit: f64) {
        self.close_longs_filled += 1;
        self.volume += volume;
        self.commission += commission;
        self.realized_profit += realized_profit;
    }

    pub fn get_close_long_expired(&mut self) -> u64 {
        let temp = self.close_longs_expired;
        self.close_longs_expired = 0;
        temp
    }

    pub fn new_close_long_expired(&mut self) {
        self.close_longs_expired += 1;
    }
    
    pub fn get_open_short_filled(&mut self) -> u64 {
        let temp = self.open_shorts_filled;
        self.open_shorts_filled = 0;
        temp
    } 

    pub fn new_open_short_filled(&mut self, volume: f64, commission: f64, realized_profit: f64) {
        self.open_shorts_filled += 1;
        self.volume += volume;
        self.commission += commission;
        self.realized_profit += realized_profit;
    }

    pub fn get_open_short_expired(&mut self) -> u64 {
        let temp = self.open_shorts_expired;
        self.open_shorts_expired = 0;
        temp
    }

    pub fn new_open_short_expired(&mut self) {
        self.open_shorts_expired += 1;
    }

    pub fn new_close_short_filled(&mut self, volume: f64, commission: f64, realized_profit: f64) {
        self.close_shorts_filled += 1;
        self.volume += volume;
        self.commission += commission;
        self.realized_profit += realized_profit;
    }

    pub fn get_close_short_filled(&mut self) -> u64 {
        let temp = self.close_shorts_filled;
        self.close_shorts_filled = 0;
        temp
    }

    pub fn get_close_long_filled(&mut self) -> u64 {
        let temp = self.close_longs_filled;
        self.close_longs_filled = 0;
        temp
    }

    pub fn get_close_short_expired(&mut self) -> u64 {
        let temp = self.close_shorts_expired;
        self.close_shorts_expired = 0;
        temp
    }

    pub fn new_close_short_expired(&mut self) {
        self.close_shorts_expired += 1;
    }

    pub fn max_open_long(&self) -> f64 {
        if self.close_only {
            0.0
        } else {
            self.max_position - self.current_longs
        }
    }

    pub fn max_close_long(&self) -> f64 {
        self.current_longs
    }

    pub fn max_open_short(&self) -> f64 {
        if self.close_only {
            0.0
        } else {
            self.max_position - self.current_shorts
        }
    }

    pub fn max_close_short(&self) -> f64 {
        self.current_shorts
    }

    pub fn get_updates(&mut self) -> (f64, f64, u64, u64, bool, bool, f64, f64, f64, f64, f64, f64) {
        (
            self.get_top_ask(),
            self.get_top_bid(),
            self.get_open_long_filled() + self.get_open_long_expired(),
            self.get_open_short_filled() + self.get_open_short_expired(),
            self.get_close_long_expired() > 0 || self.get_close_long_filled() > 0,
            self.get_close_short_expired() > 0 || self.get_close_short_filled() > 0,
            self.current_longs,
            self.current_shorts,
            self.top_ask.max(self.long_entry_price + self.close_diff),
            self.top_bid.min(self.short_entry_price - self.close_diff),
            self.get_long_increment(),
            self.get_short_increment()
        )
    }

    pub fn get_long_increment(&self) -> f64 {
        let inc = self.get_long_increment_imp();
        if inc < 2.0 {
            (inc * 4.0).round() / 4.0
        } else {
            (inc * 2.0).round() / 2.0
        }
    }

    pub fn get_long_increment_imp(&self) -> f64 {
        // println!("self.get_long_r_delta() = {}", self.get_long_r_delta());
        // println!("self.get_long_amplitude() = {}", self.get_long_amplitude());
        // println!("self.get_long_range_supposed() = {}", self.get_long_range_supposed());
        // println!("self.get_long_range_current() = {}", self.get_long_range_current());
        self.increment + (self.acceleration * self.get_long_r_delta().sqrt() * self.get_long_amplitude())
    }

    pub fn get_long_amplitude(&self) -> f64 {
        if self.current_longs >= self.soft_position {
            (self.current_longs - self.soft_position) / (self.max_position - self.soft_position) + 1.0
        } else {
            0.0
        }
    }

    pub fn get_long_r_delta(&self) -> f64 {
        let range_supposed = self.get_long_range_supposed();
        let range_current = self.get_long_range_current();
        if range_supposed > range_current {
            (range_supposed - range_current) / self.increment + 1.0
        } else {
            0.0
        }
    }

    pub fn get_long_range_supposed(&self) -> f64 {
        self.half_range * self.current_longs / self.max_position
    }

    pub fn get_long_range_current(&self) -> f64 {
        0.0_f64.max(self.long_entry_price - self.last_long_open)
    }

    pub fn get_short_increment(&self) -> f64 {
        let inc = self.get_short_increment_imp();
        if inc < 2.0 {
            (inc * 4.0).round() / 4.0
        } else {
            (inc * 2.0).round() / 2.0
        }
    }

    pub fn get_short_increment_imp(&self) -> f64 {
        // println!("self.get_short_r_delta() = {}", self.get_short_r_delta());
        // println!("self.get_short_amplitude() = {}", self.get_short_amplitude());
        // println!("self.get_short_range_supposed() = {}", self.get_short_range_supposed());
        // println!("self.get_short_range_current() = {}", self.get_short_range_current());
        self.increment + ((self.acceleration * self.get_short_r_delta().sqrt() * self.get_short_amplitude() * 10.0).round() / 10.0)
    }

    pub fn get_short_amplitude(&self) -> f64 {
        if self.current_shorts >= self.soft_position {
            (self.current_shorts - self.soft_position) / (self.max_position - self.soft_position) + 1.0
        } else {
            0.0
        }
    }

    pub fn get_short_r_delta(&self) -> f64 {
        let range_supposed = self.get_short_range_supposed();
        let range_current = self.get_short_range_current();
        if range_supposed > range_current {
            (range_supposed - range_current) / self.increment + 1.0
        } else {
            0.0
        }
    }

    pub fn get_short_range_supposed(&self) -> f64 {
        self.half_range * self.current_shorts / self.max_position
    }

    pub fn get_short_range_current(&self) -> f64 {
        0.0_f64.max(self.last_short_open - self.short_entry_price)
    }

    pub fn get_close_prices(&self) -> (f64, f64) {
        (
            self.top_ask.max(self.long_entry_price + self.close_diff),
            self.top_bid.min(self.short_entry_price - self.close_diff)
        )
    }

    pub fn get_stats(&self) -> (Duration, f64, u64, f64, f64, f64, f64, f64, i64, i64, f64, f64, f64, f64, f64, f64, f64, u64, f64, f64) {
        (
            self.started_time.elapsed(),
            self.volume,
            self.order_overflow_count,
            self.balance,
            self.commission * -1.0,
            self.realized_profit,
            self.current_longs,
            self.current_shorts,
            self.price_decimal_count,
            self.quantity_decimal_count,
            self.get_marketprice(),
            self.top_ask.max(self.long_entry_price + self.close_diff),
            self.top_bid.min(self.short_entry_price - self.close_diff),
            self.long_entry_price,
            self.short_entry_price,
            self.long_un_pnl,
            self.short_un_pnl,
            self.vip_level,
            self.get_long_increment(),
            self.get_short_increment()
        )
    }

    pub fn start_overflow(&mut self) {
        self.order_overflow = true;
        self.order_overflow_count += 1;
    }

    pub fn is_oveflowing(&self) -> bool {
        self.order_overflow
    }

    pub fn reset_all(&mut self) {
        self.top_ask = 0.0;
        self.top_bid = 0.0;
        self.current_longs = 0.0;
        self.current_shorts = 0.0;
        self.open_longs_filled = 0;
        self.open_longs_expired = 0;
        self.close_longs_filled = 0;
        self.close_longs_expired = 0;
        self.open_shorts_filled = 0;
        self.open_shorts_expired = 0;
        self.close_shorts_filled = 0;
        self.close_shorts_expired = 0;
        self.order_overflow = false;
    }

    pub fn set_need_restart_true(&mut self) {
        self.need_restart = true;
        self.order_overflow = true;
    }

    pub fn is_restart_needed(&self) -> bool {
        self.need_restart
    }

    pub fn set_need_restart_false(&mut self) {
        self.need_restart = false;
    }

    pub fn get_discord(&self) -> (String, String) {
        (self.discord_token.clone(), self.discord_channel.clone())
    }

    pub fn activate_close_only(&mut self) {
        self.close_only = true;
    }
}