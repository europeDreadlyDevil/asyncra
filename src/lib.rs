use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::{clone_trait_object, DynClone};
use tokio::runtime::Builder;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, Mutex, Notify};
use tokio::sync::oneshot::{Receiver, Sender};

pub use asyncra_macros::main;
pub use asyncra_macros::test;
pub use asyncra_macros::bench;
pub use criterion::{criterion_group, criterion_main, Criterion};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use lazy_static::lazy_static;
use tokio::join;
use tokio::task::JoinHandle;

lazy_static! {
    static ref NODE_MANAGER_CHANNEL: (NodeSender<NodeManagerMessage>, NodeReceiver<NodeManagerMessage>) = {
        let (tx, rx) = unbounded_channel();
        (Arc::new(tx), Arc::new(Mutex::new(rx)))
    };
}

#[macro_export] macro_rules! extract_benches {
    ($( $target:path ),+ $(,)*) => {
        asyncra::criterion_group!(benches, $($target),*);
        asyncra::criterion_main!(benches);
    };
}

pub type NodeSender<T> = Arc<UnboundedSender<T>>;
pub type NodeReceiver<T> = Arc<Mutex<UnboundedReceiver<T>>>;
pub type DataSender<T> = Sender<T>;
pub type DataReceiver<T> = Receiver<T>;

pub type Result<T> = anyhow::Result<T>;

pub trait EventLoopResult<T> {
    fn unwrap(self) -> T;
}
impl EventLoopResult<()> for Result<()> {
    fn unwrap(self) -> () {
        self.unwrap()
    }
}

pub trait CloneableAny: Any + Send + Sync + Downcast + Debug + DynClone {
    fn clone_box(&self) -> Box<dyn CloneableAny + 'static>;
}

impl_downcast!(CloneableAny);
clone_trait_object!(CloneableAny);

impl<T> CloneableAny for T
where
    T: Any + Clone + Send + Sync + Debug,
{
    fn clone_box(&self) -> Box<dyn CloneableAny + 'static> {
        Box::new(self.clone())
    }
}

pub enum SharedValueMessage<T> {
    Write {
        data: T,
    },
    WriteLock {
      data: T,
    },
    Read{ tx: DataSender<T>},
    ReadLock { tx: DataSender<T> }
}

pub enum NodeManagerMessage {
    Reg(JoinHandle<anyhow::Result<()>>),
    Close
}

#[derive(Clone)]
pub struct SharedValue {
    sender: Arc<UnboundedSender<SharedValueMessage<Box<dyn CloneableAny>>>>,
    #[allow(dead_code)]
    notify: Arc<Notify>,
}

#[allow(unused_assignments)]
impl SharedValue {
    pub fn new<T: CloneableAny>(data: T) -> Self {
        let (tx, mut rx) = unbounded_channel();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let s_val = Self {
            sender: Arc::new(tx),
            notify: notify_clone,
        };

        tokio::spawn(async move {
            let mut storage: Option<Box<dyn CloneableAny>> = Some(Box::new(data));
            let mut read_locks = 0;
            let mut write_lock = false;

            while let Some(msg) = rx.recv().await {
                match msg {
                    SharedValueMessage::Write { data } => {
                        if !write_lock {
                            storage = Some(data);
                            notify.notify_waiters();
                        }
                    }
                    SharedValueMessage::Read { tx } => {
                        if let Some(ref value) = storage {
                            let _ = tx.send(value.clone());
                        }
                    }
                    SharedValueMessage::WriteLock { data } => {
                        while write_lock || read_locks > 0 {
                            notify.notified().await;
                        }
                        write_lock = true;
                        storage = Some(data);
                        write_lock = false;
                        notify.notify_waiters();
                    }
                    SharedValueMessage::ReadLock { tx } => {
                        while write_lock {
                            notify.notified().await;
                        }
                        read_locks += 1;
                        if let Some(ref value) = storage {
                            let _ = tx.send(value.clone());
                        }
                        read_locks -= 1;
                        notify.notify_waiters();
                    }
                }
            }
        });

        s_val
    }

    pub async fn read<T: CloneableAny>(&self) -> T {
        let (tx, rx) = oneshot::channel();
        self.sender.send(SharedValueMessage::Read { tx }).unwrap();
        *rx.await.unwrap().downcast::<T>().unwrap()
    }

    pub async fn read_lock<T: CloneableAny>(&self) -> T {
        let (tx, rx) = oneshot::channel();
        self.sender.send(SharedValueMessage::ReadLock { tx }).unwrap();
        *rx.await.unwrap().downcast::<T>().unwrap()
    }

    pub fn write<T: CloneableAny>(&self, data: T) {
        self.sender
            .send(SharedValueMessage::Write {
                data: Box::new(data),
            })
            .unwrap();
    }

    pub fn write_lock<T: CloneableAny>(&self, data: T) {
        self.sender
            .send(SharedValueMessage::WriteLock {
                data: Box::new(data),
            })
            .unwrap();
    }
}

pub fn wake_runtime<
    M: Future<Output = Result<()>> + Sized,
>(
    fabric: fn() -> M,
) -> anyhow::Result<()> {
    let rt = Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let node_manager = tokio::spawn(async move {
            let mut nodes = FuturesUnordered::new();
            loop {
                if let Some(msg) = NODE_MANAGER_CHANNEL.clone().1.lock().await.recv().await {
                    match msg {
                        NodeManagerMessage::Reg(node) => {
                            nodes.push(node);
                        }
                        NodeManagerMessage::Close => {
                            while let Some(_node) = nodes.next().await {}
                            return ();
                        }
                    }
                }
            }
        });
        let handle = async move {
            let res = fabric().await;
            let _ = NODE_MANAGER_CHANNEL.clone().0.send(NodeManagerMessage::Close);
            res
        };
        let res = join!(handle, node_manager);
        res.0
    })?;
    Ok(())
}

pub fn spawn_node<F: Future<Output=anyhow::Result<()>> + Send + Sync + 'static>(fabric: F) -> anyhow::Result<()> {
    NODE_MANAGER_CHANNEL.clone().0.send(NodeManagerMessage::Reg(tokio::task::spawn(fabric)))?;
    Ok(())
}