use crate::NetworkError;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};
use std::marker::PhantomData;

pub trait NetworkTask<B>
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>);

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(&mut self, event: SwarmEvent<B::ToSwarm>) -> Option<SwarmEvent<B::ToSwarm>> {
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&self) -> bool {
		true
	}
}
pub type NetworkTaskBox<B> = Box<dyn NetworkTask<B> + Send + 'static>;

#[derive(Debug)]
pub struct NetworkTaskSpawner<B> {
	pub(crate) tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<B>>,
}
impl<B> NetworkTaskSpawner<B>
where
	B: NetworkBehaviour,
{
	pub fn spawn<T>(&self, task: T) -> Result<(), NetworkError>
	where
		T: NetworkTask<B> + Send + 'static,
	{
		self.tasks.send(Box::new(task))?;
		Ok(())
	}
}
impl<B> Clone for NetworkTaskSpawner<B> {
	fn clone(&self) -> Self {
		Self { tasks: self.tasks.clone() }
	}
}

pub struct FnOnceNetworkTask<F, B>
where
	F: FnOnce(&mut Swarm<B>) + Send + 'static,
{
	_b: PhantomData<B>,
	f: Option<F>,
}
impl<F, B> FnOnceNetworkTask<F, B>
where
	F: FnOnce(&mut Swarm<B>) + Send + 'static,
{
	pub fn new(f: F) -> Self {
		Self { _b: Default::default(), f: Some(f) }
	}
}
impl<B, F> NetworkTask<B> for FnOnceNetworkTask<F, B>
where
	B: NetworkBehaviour,
	F: FnOnce(&mut Swarm<B>) + Send + 'static,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		if let Some(f) = Option::take(&mut self.f) {
			f(swarm);
		}
	}
}
