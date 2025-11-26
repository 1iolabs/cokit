use crate::NetworkError;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};
use std::{fmt::Debug, time::Instant};

pub trait NetworkTask<B, C>: Debug
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

	/// Task state.
	fn task_state(&mut self) -> NetworkTaskState {
		if self.is_complete() {
			NetworkTaskState::Complete
		} else {
			NetworkTaskState::Waiting
		}
	}
}
pub type NetworkTaskBox<B, C> = Box<dyn NetworkTask<B, C> + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkTaskState {
	Pending,
	Delayed(Instant),
	Waiting,
	Complete,
}

#[derive(Debug)]
pub struct TokioNetworkTaskSpawner<B, C> {
	pub(crate) tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<B, C>>,
}

impl<B, C> Clone for TokioNetworkTaskSpawner<B, C> {
	fn clone(&self) -> Self {
		Self { tasks: self.tasks.clone() }
	}
}
impl<B, C> NetworkTaskSpawner<B, C> for TokioNetworkTaskSpawner<B, C>
where
	B: NetworkBehaviour,
{
	fn spawn_box(&self, task: NetworkTaskBox<B, C>) -> Result<(), NetworkError> {
		self.tasks.send(task)?;
		Ok(())
	}
}

pub trait NetworkTaskSpawner<B, C>
where
	B: NetworkBehaviour,
{
	fn spawn<T>(&self, task: T) -> Result<(), NetworkError>
	where
		T: NetworkTask<B, C> + Send + 'static,
	{
		Ok(self.spawn_box(Box::new(task))?)
	}

	fn spawn_box(&self, task: NetworkTaskBox<B, C>) -> Result<(), NetworkError>;
}
