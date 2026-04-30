use tokio::time::{sleep, Duration};
use std::thread;

async fn async_task(id: u8) {
    for i in 0..5 {
        println!("task {id} tick {i}");
        sleep(Duration::from_millis(200)).await;
    }
}

async fn blocking_task() {
    println!("blocking task started");
    thread::sleep(Duration::from_secs(5));
    println!("blocking task finished");
}

#[tokio::main]
async fn main() {
    tokio::join!(
        async_task(1),
        async_task(2),
        blocking_task(),
    );
}
