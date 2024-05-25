async fn trader_loop(current_orders: (Vec<Order>, Vec<Order>, Order, Order)) {
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
                        println!("long price: {}", open_long_orders[0].get_price());
                        open_long_shift_down += 1;
                        target_price -= long_increment;
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
                        target_price += short_increment;
                    } else {
                        break;
                    }
                }
            }
            // Open Longs    ----------------------------------------------------------------------------------------------
            if open_long_shift_down > 0 {
                println!("Shifting open longs down {}", open_long_shift_down);
                if open_long_shift_down >= order_amount {
                    for i in 0..order_amount as usize {
                        open_long_orders[i] = Order::new(open_long_orders[i].get_price() - open_long_shift_down as f64 * long_increment, 0.0, true, true);
                    }
                } else {
                    for i in 0..open_long_shift_down as usize {
                        open_long_orders[i] = Order::new(open_long_orders[i].get_price() - order_amount as f64 * long_increment, 0.0, true, true);
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
            } else {
                let mut open_long_shift_up = 0;
                let mut target_price = open_long_orders[0].get_price() + long_increment;
                loop {
                    if target_price <= top_bid {
                        println!("top_bid: {}", top_bid);
                        println!("long price: {}", open_long_orders[0].get_price());
                        open_long_shift_up += 1;
                        target_price += long_increment;
                    } else {
                        break;
                    }
                }
                if open_long_shift_up > 0 {
                    println!("Shifting open longs up {}", open_long_shift_up);
                    if open_long_shift_up >= order_amount {
                        for i in 0..order_amount as usize {
                            orders_to_cancel.push(open_long_orders[i].clone());
                            open_long_orders[i] = Order::new(open_long_orders[i].get_price() + open_long_shift_up as f64 * long_increment, 0.0, true, true);
                        }
                    } else {
                        open_long_orders.rotate_right(open_long_shift_up as usize);
                        for i in 0..open_long_shift_up as usize {
                            orders_to_cancel.push(open_long_orders[i].clone());
                            open_long_orders[i] = Order::new(open_long_orders[i].get_price() + order_amount as f64 * long_increment, 0.0, true, true);
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
            // Open Longs    ----------------------------------------------------------------------------------------------
            // Open Shorts   ----------------------------------------------------------------------------------------------
            if open_short_shift_up > 0 {
                println!("Shifting open shorts up {}", open_short_shift_up);
                if open_short_shift_up >= order_amount {
                    for i in 0..order_amount as usize {
                        open_short_orders[i] = Order::new(open_short_orders[i].get_price() + open_short_shift_up as f64 * short_increment, 0.0, true, false);
                    }
                } else {
                    for i in 0..open_short_shift_up as usize {
                        open_short_orders[i] = Order::new(open_short_orders[i].get_price() + order_amount as f64 * short_increment, 0.0, true, false);
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
            } else {
                let mut open_short_shift_down = 0;
                let mut target_price = open_short_orders[0].get_price() - short_increment;
                loop {
                    if target_price >= top_ask {
                        open_short_shift_down += 1;
                        target_price -= short_increment;
                    } else {
                        break;
                    }
                }
                if open_short_shift_down > 0 {
                    println!("Shifting open shorts down {}", open_short_shift_down);
                    if open_short_shift_down >= order_amount {
                        for i in 0..order_amount as usize {
                            orders_to_cancel.push(open_short_orders[i].clone());
                            open_short_orders[i] = Order::new(open_short_orders[i].get_price() - open_short_shift_down as f64 * short_increment, 0.0, true, false);
                        }
                    } else {
                        open_short_orders.rotate_right(open_short_shift_down as usize);
                        for i in 0..open_short_shift_down as usize {
                            orders_to_cancel.push(open_short_orders[i].clone());
                            open_short_orders[i] = Order::new(open_short_orders[i].get_price() - order_amount as f64 * short_increment, 0.0, true, false);
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