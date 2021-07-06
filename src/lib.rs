use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{stream::iter, SinkExt};
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct Spliter<S, I>
where
    S: Stream<Item = I> + Send + Sync + Unpin,
    I: Clone,
{
    inner: Pin<Box<S>>,
    num: usize,
    txs: Vec<UnboundedSender<I>>,
    rxs: Vec<UnboundedReceiver<I>>,
}

impl<S, I> Spliter<S, I>
where
    S: Stream<Item = I> + Send + Sync + Unpin + 'static,
    I: Clone + Send + Sync + 'static,
{
    pub fn new(stream: Pin<Box<S>>, num: usize) -> Self {
        Self {
            inner: stream,
            num: num,
            txs: Vec::new(),
            rxs: Vec::new(),
        }
    }

    pub fn split(mut self) -> Vec<UnboundedReceiver<I>> {
        for _ in 0..self.num {
            let (tx, rx) = unbounded::<I>();
            self.txs.push(tx);
            self.rxs.push(rx);
        }
        let mut stream = self.inner;
        let txs = self.txs;
        tokio::spawn(async move {
            while let Some(v) = stream.next().await {
                for mut tx in txs.iter() {
                    tx.send(v.clone()).await.unwrap();
                }
            }
        });
        self.rxs
    }
}

#[tokio::test]
async fn test_split() {
    let stream = iter(vec![1, 2, 3, 4, 5, 6, 7]);
    let spliter = Spliter::new(Box::pin(stream), 2);
    let rxs = spliter.split();
    for (i, mut rx) in rxs.into_iter().enumerate() {
        while let Some(v) = rx.next().await {
            println!("the {}th channel output: {}", i, v);
        }
    }
}
