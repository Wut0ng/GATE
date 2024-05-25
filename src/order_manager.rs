use rand::Rng;

#[derive(Clone)]
#[derive(Debug)]
pub struct Order {
    price: f64,
    quantity: f64,
    is_open: bool,
    is_long: bool,
    order_id: u32
}

impl Order {
    pub fn new(price: f64, quantity: f64, is_open: bool, is_long: bool,) -> Self {
        Order {
            price,
            quantity,
            is_open,
            is_long,
            order_id: rand::thread_rng().gen_range(0..4294967295)
        }
    }

    pub fn get_price(&self) -> f64 {
        self.price
    }

    pub fn get_quantity(&self) -> f64 {
        self.quantity
    }

    pub fn set_quantity(&mut self, quantity: f64) {
        self.quantity = quantity;
    }

    pub fn is_real(&self) -> bool {
        self.quantity > 0.0
    }

    pub fn get_id(&self) -> u32 {
        self.order_id
    }

    pub fn to_url_string(&self, pair: &str, price_decimal_count: i64, quantity_decimal_count: i64) -> String {
        format!("{{\"symbol\":\"{0}\",\"side\":\"{1}\",\"positionSide\":\"{2}\",\"type\":\"LIMIT\",\"price\":\"{3:.4$}\",\"timeInForce\":\"GTX\",\"quantity\":\"{5:.6$}\",\"newClientOrderId\":\"{7}\"}}", pair, if self.is_open ^ self.is_long {"SELL"} else {"BUY"}, if self.is_long {"LONG"} else {"SHORT"}, self.price, price_decimal_count as usize, self.quantity, quantity_decimal_count as usize, self.order_id)
    }

    pub fn to_single_url_string(&self, pair: &str, price_decimal_count: i64, quantity_decimal_count: i64) -> String {
        format!("symbol={0}&side={1}&positionSide={2}&type=LIMIT&price={3:.4$}&timeInForce=GTX&quantity={5:.6$}&newClientOrderId={7}", pair, if self.is_open ^ self.is_long {"SELL"} else {"BUY"}, if self.is_long {"LONG"} else {"SHORT"}, self.price, price_decimal_count as usize, self.quantity, quantity_decimal_count as usize, self.order_id)
    }
}