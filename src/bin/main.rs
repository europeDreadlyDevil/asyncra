use asyncra::SharedValue;

#[asyncra::main]
async fn main() -> asyncra::Result<()> {
    let num = SharedValue::new(12);
    let num_c = num.clone();
    let arr = SharedValue::new(vec![0]);
    let arr_c = arr.clone();
    asyncra::spawn_node( async move {
        for _ in 0..10 {
            println!("First read");
            let num_3 = num_c.read_lock::<i32>().await;
            let mut arr_ = arr_c.read_lock::<Vec<i32>>().await;
            arr_.push(num_3);
            arr_c.write_lock(arr_);
            println!("Num now is: {num_3}");
        }
        Ok(())
    })?;
    asyncra::spawn_node(async move {
        for _ in 0..10 {
            println!("Second write");
            let num_ = num.read_lock::<i32>().await;
            num.write_lock(num_ + 1);
        }
        Ok(())
    })?;
    println!("{:?}", arr.read_lock::<Vec<i32>>().await);
    Ok(())
}