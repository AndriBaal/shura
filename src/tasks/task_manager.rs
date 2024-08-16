use crate::context::Context;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{any::Any, future::Future, rc::Rc};

type TaskCallback = Box<dyn FnOnce(&mut Context) + Send + 'static>;

pub struct TaskManager {
    receiver: Rc<Receiver<TaskCallback>>,
    sender: Sender<TaskCallback>,
}

impl TaskManager {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            receiver: Rc::new(receiver),
            sender,
        }
    }


    pub(crate) fn await_future<F: Future + 'static>(future: F) -> F::Output {
        #[cfg(not(target_arch = "wasm32"))] {
            pollster::block_on(future)
        }

        #[cfg(target_arch = "wasm32")] {
            use std::sync::{OnceLock, Arc};
            let mut result = Arc::new(OnceLock::new());
            let result_clone = result.clone();

            let _ = wasm_bindgen_futures::future_to_promise(async move {
                let tmp = future.await;
                result_clone.set(tmp).ok().unwrap();
                Ok(wasm_bindgen_futures::wasm_bindgen::JsValue::NULL)
            });
            
            while result.get().is_none() {

            }

            loop {
                match Arc::get_mut(&mut result) {
                    Some(result) => return result.take().unwrap(),
                    None => ()
                }
            }
        }
    }

    pub fn spawn<R: Send + 'static>(
        &self,
        task: impl FnOnce() -> R + Send + 'static,
        callback: impl FnOnce(&mut Context, R) + Send + 'static,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let sender = self.sender.clone();
            std::thread::spawn(move || {
                let result = (task)();
                sender
                    .send(Box::new(|ctx| {
                        (callback)(ctx, result);
                    }))
                    .unwrap();
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let sender = self.sender.clone();
            let _ = wasm_bindgen_futures::future_to_promise(async move {
                let result = (task)();
                sender
                    .send(Box::new(|ctx| {
                        (callback)(ctx, result);
                    }))
                    .unwrap();
                Ok(wasm_bindgen_futures::wasm_bindgen::JsValue::NULL)
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_async<T>(
        &self,
        task: T,
        callback: impl FnOnce(&mut Context, T::Output) + Send + 'static,
    ) where
        T: Future + Send + 'static,
        T::Output: Any + Send + 'static,
    {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = pollster::block_on(task);
            sender
                .send(Box::new(|ctx| {
                    (callback)(ctx, result);
                }))
                .unwrap();
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn_async<T>(
        &self,
        task: T,
        callback: impl FnOnce(&mut Context, T::Output) + Send + 'static,
    ) where
        T: Future + 'static,
        T::Output: Any + Send + 'static,
    {
        let sender = self.sender.clone();
        let _ = wasm_bindgen_futures::future_to_promise(async move {
            let result = task.await;
            sender
                .send(Box::new(|ctx| {
                    (callback)(ctx, result);
                }))
                .unwrap();
            Ok(wasm_bindgen_futures::wasm_bindgen::JsValue::NULL)
        });
    }

    pub(crate) fn receiver(&self) -> Rc<Receiver<TaskCallback>> {
        self.receiver.clone()
    }
}
