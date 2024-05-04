use crate::context::Context;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::rc::Rc;

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

    pub(crate) fn receiver(&self) -> Rc<Receiver<TaskCallback>> {
        self.receiver.clone()
    }
}
