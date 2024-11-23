use tokio::join;
use asyncra::SharedValue;

#[asyncra::main]
async fn main() -> asyncra::Result<()> {
    let num = SharedValue::new(12);
    let num_c = num.clone();
    let first = tokio::task::spawn( async move {
        for _ in 0..10 {
            let num_3 = num_c.read::<i32>().await;
            println!("Num now is: {num_3}");
        }
    });
    let second = tokio::task::spawn(async move {
        for _ in 0..10 {
            let num_ = num.read::<i32>().await;
            num.write(num_ + 1);
        }
    });
    join!(first, second);
    Ok(())
}