use std::{process, time::Duration};
use tokio::{
    self,
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc,
    time::sleep,
};

#[tokio::main]
async fn main() {
    // console_subscriber::init();
    let (sender, mut receiver) = mpsc::channel(10);
    let sender1 = sender.clone();
    tokio::spawn(async move {
        loop {
            let stdin = BufReader::new(tokio::io::stdin());
            let mut lines = stdin.lines();
            if let Ok(line) = lines.next_line().await {
                let line = line.unwrap();
                sender1.send(line).await.unwrap();
            }
        }
    });
    let sender2 = sender.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(1000)).await;
            sender2.send("123".to_string()).await.unwrap();
        }
    });

    loop {
        tokio::select! {
            v = receiver.recv() =>{
                tokio::spawn(async move {
                    println!("receive: {v:?}");
                });
            },
            _= tokio::signal::ctrl_c()=>{
                println!("Terminal!!");
                process::exit(0);
            }
        }
    }
}
