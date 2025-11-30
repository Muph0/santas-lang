use std::{
    collections::VecDeque,
    sync::{Arc, Weak, mpsc},
};

pub struct InputPipe<T: Clone> {
    weak_tx: Weak<mpsc::Sender<T>>,
    buffer: VecDeque<T>,
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
        Self {
            weak_tx,
            rx,
            buffer: Default::default(),
        }
    }
    pub fn connect(&self, output: &mut OutputPipe<T>) {
        output.connect(self);
    }
    /// Write directly to the internal (received) buffer
    pub fn write_direct(&mut self, t: T) {
        self.buffer.push_back(t);
    }
    pub fn try_read(&mut self) -> Result<T, InputError> {
        self.recv_to_buffer();
        if let Some(v) = self.buffer.pop_front() {
            return Ok(v);
        }
        match self.rx.try_recv() {
            Ok(v) => Ok(v),
            Err(mpsc::TryRecvError::Disconnected) => Err(InputError::Closed),
            Err(mpsc::TryRecvError::Empty) => Err(InputError::Empty),
        }
    }

    fn recv_to_buffer(&mut self) {
        while let Ok(v) = self.rx.try_recv() {
            self.buffer.push_back(v);
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

impl<T: Clone> std::fmt::Debug for InputPipe<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ptr) = self.weak_tx.upgrade() {
            f.debug_tuple("InputPipe").field(&ptr).finish()
        } else {
            write!(f, "InputPipe( closed )")
        }
    }
}

impl<T: Clone> std::fmt::Debug for OutputPipe<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OutputPipe").field(&self.to).finish()
    }
}
