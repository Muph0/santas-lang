use std::sync::{Arc, Weak, mpsc};

pub struct InputPipe<T: Clone> {
    weak_tx: Weak<mpsc::Sender<T>>,
    rx: mpsc::Receiver<T>,
}

#[derive(Default)]
pub struct OutputPipe<T: Clone> {
    to: Vec<Arc<mpsc::Sender<T>>>,
}

pub enum InputError {
    /// No values incoming for now
    Empty,
    /// Closed pipe means elf will wait forever
    Closed,
}

impl<T: Clone> InputPipe<T> {
    pub fn new_connected(output: &mut OutputPipe<T>) -> Self {
        let (tx, rx) = mpsc::channel();

        let tx = Arc::new(tx);
        let weak_tx = Arc::downgrade(&tx);

        output.to.push(tx);
        Self { weak_tx, rx }
    }
    pub fn connect(&self, output: &mut OutputPipe<T>) {
        output.connect(self);
    }
    pub fn write(&self, t: T) {
        if let Some(tx) = self.weak_tx.upgrade() {
            _ = tx.send(t);
        }
    }
    pub fn try_read(&self) -> Result<T, InputError> {
        match self.rx.try_recv() {
            Ok(v) => Ok(v),
            Err(mpsc::TryRecvError::Disconnected) => Err(InputError::Closed),
            Err(mpsc::TryRecvError::Empty) => Err(InputError::Empty),
        }
    }
}

impl<T: Clone> OutputPipe<T> {
    pub fn new() -> Self {
        Self { to: vec![] }
    }
    pub fn connect(&mut self, input: &InputPipe<T>) {
        match input.weak_tx.upgrade() {
            Some(tx) => self.to.push(tx),
            None => {
                todo!("re-open closed channel");
            }
        }
    }
    pub fn write(&self, t: T) {
        for to in &self.to {
            _ = to.send(t.clone());
        }
    }
}
