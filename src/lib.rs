use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::{clone_trait_object, DynClone};
use lazy_static::lazy_static;
use tokio::join;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::sync::oneshot::{Receiver, Sender};
use uuid::Uuid;

pub use asyncra_macros::main;

lazy_static!{
    pub(crate) static ref TASK_CHANNEL: (TaskSender<Message<Box<dyn CloneableAny>>>, TaskReceiver<Message<Box<dyn CloneableAny>>>) = {
        let (tx, rx) = unbounded_channel();
        (Arc::new(tx), Arc::new(Mutex::new(rx)))
    };
}

pub type TaskSender<T> = Arc<UnboundedSender<T>>;
pub type TaskReceiver<T> = Arc<Mutex<UnboundedReceiver<T>>>;
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

pub enum Message<T> {
    Write {
        data: T,
        id: Uuid
    },
    Read{ id: Uuid, tx: DataSender<T>}
}

pub struct SharedValue(Uuid);

impl SharedValue {
    pub fn new<T: Send + Sync + Clone + Debug + 'static>(data: T) -> Self {
        let id = Uuid::new_v4();
        let s_val = Self(id);
        s_val.write(data);
        s_val
    }

    pub async fn read<T: Send + Sync + Clone + Debug + 'static>(&self) -> T {
        let (tx, rx) = tokio::sync::oneshot::channel();
        TASK_CHANNEL.clone().0.send(Message::Read {id: self.0, tx}).unwrap();
        *rx
            .await
            .unwrap()
            .downcast::<T>()
            .unwrap()
    }

    pub fn write<T: Send + Sync + Clone + Debug + 'static>(&self, data: T) {
        TASK_CHANNEL.clone().0.send(Message::Write {data: Box::new(data), id: self.0.clone()}).unwrap();
    }

}

impl Clone for SharedValue {
    fn clone(&self) -> Self {
        SharedValue(self.0.clone())
    }
}



pub fn wake_runtime<
    M: Future<Output = Result<()>> + Sized,
    R: EventLoopResult<()>,
    E: Future<Output = R> + Sized,
>(
    fabric: fn() -> M,
    event_loop: fn() -> E,
) -> anyhow::Result<()> {
    let rt = Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()?;
    rt.block_on(async move {
        runner(fabric, event_loop)
            .await
            .expect("Running runtime error");
    });
    Ok(())
}

async fn runner<
    M: Future<Output = Result<()>> + Sized,
    ER: EventLoopResult<()>,
    E: Future<Output = ER>,
>(
    fabric: fn() -> M,
    event_loop: fn() -> E,
) -> anyhow::Result<()> {
    let res = join!(fabric(), event_loop());
    (res.0?, res.1.unwrap());
    Ok(())
}

pub async fn event_loop() -> Result<()> {
    let mut store = HashMap::new();
    while let Some(msg) = TASK_CHANNEL.clone().1.lock().await.recv().await {
        match msg {
            Message::Write { id, data } => {
                store.insert(id, data);
            }
            Message::Read { id, tx } => {
                let data = store.get(&id).unwrap();
                let cloned_data = dyn_clone::clone(data);
                if let Err(_) = tx.send(cloned_data) {
                    panic!("Fail to read data")
                }
            }
        }
    }
    Ok(())
}