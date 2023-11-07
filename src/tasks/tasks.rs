use crate::Context;
use std::{
    any::Any,
    future::Future,
    rc::Rc,
    sync::mpsc::{channel, Receiver, Sender},
};

type TaskCallback = Box<dyn FnOnce(&mut Context) + Send + 'static>;

pub struct TaskManager {
    receiver: Rc<Receiver<TaskCallback>>,
    sender: Sender<TaskCallback>,
}

impl TaskManager {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            receiver: Rc::new(receiver),
            sender,
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
    }

    pub(crate) fn receiver(&self) -> Rc<Receiver<TaskCallback>> {
        self.receiver.clone()
    }
}
