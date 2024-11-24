use tokio::join;
use asyncra::{extract_benches, SharedValue};

#[asyncra::bench]
async fn bench_test() -> asyncra::Result<()> {
    let num = SharedValue::new(1);
    let num_c = num.clone();
    let arr = SharedValue::new(vec![0]);
    let arr_c = arr.clone();
    let first = tokio::task::spawn( async move {
        for _ in 0..100 {
            let num_3 = num_c.read_lock::<i32>().await;
            let mut arr_ = arr_c.read_lock::<Vec<i32>>().await;
            arr_.push(num_3);
            arr_c.write_lock(arr_);
        }
    });
    let second = tokio::task::spawn(async move {

        for _ in 0..100 {
            let num_ = num.read_lock::<i32>().await;
            num.write_lock(num_ + 1);
        }
    });
    join!(first, second);
    Ok(())
}

#[asyncra::bench]
async fn bench_test_1000() -> asyncra::Result<()> {
    let num = SharedValue::new(1);
    let num_c = num.clone();
    let arr = SharedValue::new(vec![0]);
    let arr_c = arr.clone();
    let first = tokio::task::spawn( async move {
        for _ in 0..1000 {
            let num_3 = num_c.read_lock::<i32>().await;
            let mut arr_ = arr_c.read_lock::<Vec<i32>>().await;
            arr_.push(num_3);
            arr_c.write_lock(arr_);
        }
    });
    let second = tokio::task::spawn(async move {

        for _ in 0..1000 {
            let num_ = num.read_lock::<i32>().await;
            num.write_lock(num_ + 1);
        }
    });
    join!(first, second);
    Ok(())
}

#[asyncra::bench]
async fn bench_test_wo_lock() -> asyncra::Result<()> {
    let num = SharedValue::new(1);
    let num_c = num.clone();
    let arr = SharedValue::new(vec![0]);
    let arr_c = arr.clone();
    let first = tokio::task::spawn( async move {
        for _ in 0..100 {
            let num_3 = num_c.read::<i32>().await;
            let mut arr_ = arr_c.read::<Vec<i32>>().await;
            arr_.push(num_3);
            arr_c.write_lock(arr_);
        }
    });
    let second = tokio::task::spawn(async move {

        for _ in 0..100 {
            let num_ = num.read::<i32>().await;
            num.write(num_ + 1);
        }
    });
    join!(first, second);
    Ok(())
}

#[asyncra::bench]
async fn bench_test_1000_wo_lock() -> asyncra::Result<()> {
    let num = SharedValue::new(1);
    let num_c = num.clone();
    let arr = SharedValue::new(vec![0]);
    let arr_c = arr.clone();
    let first = tokio::task::spawn( async move {
        for _ in 0..1000 {
            let num_3 = num_c.read::<i32>().await;
            let mut arr_ = arr_c.read::<Vec<i32>>().await;
            arr_.push(num_3);
            arr_c.write(arr_);
        }
    });
    let second = tokio::task::spawn(async move {

        for _ in 0..1000 {
            let num_ = num.read::<i32>().await;
            num.write(num_ + 1);
        }
    });
    join!(first, second);
    Ok(())
}

extract_benches!(bench_test, bench_test_1000, bench_test_wo_lock, bench_test_1000_wo_lock);