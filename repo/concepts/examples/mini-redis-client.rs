use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
enum Command {
    Get{
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

/// Provided the requester, used by the manager to send back the
/// response back to the requster
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);

    use mini_redis::client;
    let manager = tokio::spawn(async move {
        let mut cl = client::connect("127.0.0.1:6379").await.unwrap();

        // start receiving
        while let Some(cmd) = rx.recv().await {
            use Command::*;

            match cmd {
                Get { key, resp } => {
                   let res = cl.get(&key).await;
                   resp.send(res).unwrap();
                },
                Set { key, val, resp } => {
                    let res = cl.set(&key, val).await;
                    resp.send(res).unwrap();
                }
            }
        }
    });

    // we need to use transmitter from two tasks,
    // so make a copy
    let tx2 = tx.clone();

    // spwan those two tasks
    // set a key
    let t1 = tokio::spawn(async move{
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "foo".to_string(),
            resp: resp_tx,
        };

        // send the GET request
        tx.send(cmd).await.unwrap();

        // await the response
        let res = resp_rx.await;
        println!("Got: {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        tx2.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("Got: {:?}", res);
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}