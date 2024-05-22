use crate::NetworkError;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};
use std::marker::PhantomData;

pub trait NetworkTask<B, C>
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C);

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&mut self) -> bool {
		true
	}
}
pub type NetworkTaskBox<B, C> = Box<dyn NetworkTask<B, C> + Send + 'static>;

pub struct NetworkTaskSpawner<B, C> {
	pub(crate) tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<B, C>>,
}
impl<B, C> NetworkTaskSpawner<B, C>
where
	B: NetworkBehaviour,
{
	pub fn spawn<T>(&self, task: T) -> Result<(), NetworkError>
	where
		T: NetworkTask<B, C> + Send + 'static,
	{
		self.tasks.send(Box::new(task))?;
		Ok(())
	}
}
impl<B, C> Clone for NetworkTaskSpawner<B, C> {
	fn clone(&self) -> Self {
		Self { tasks: self.tasks.clone() }
	}
}

pub struct FnOnceNetworkTask<F, B, C>
where
	F: FnOnce(&mut Swarm<B>, &mut C) + Send + 'static,
{
	_b: PhantomData<B>,
	_c: PhantomData<C>,
	f: Option<F>,
}
impl<F, B, C> FnOnceNetworkTask<F, B, C>
where
	F: FnOnce(&mut Swarm<B>, &mut C) + Send + 'static,
{
	pub fn new(f: F) -> Self {
		Self { _b: Default::default(), _c: Default::default(), f: Some(f) }
	}
}
impl<B, C, F> NetworkTask<B, C> for FnOnceNetworkTask<F, B, C>
where
	B: NetworkBehaviour,
	F: FnOnce(&mut Swarm<B>, &mut C) + Send + 'static,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		if let Some(f) = Option::take(&mut self.f) {
			f(swarm, context);
		}
	}
}
