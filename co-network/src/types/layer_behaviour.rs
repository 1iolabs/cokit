use futures::{stream::FusedStream, Stream};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};
use std::{
	marker::PhantomData,
	pin::Pin,
	task::{Context, Poll},
};

/// A "Higher-Order" NetworkBehaviour that runs atop of the Swarm.
/// This can be used to create a protocol that (only) composes of other protocols.
pub trait LayerBehaviour<B>
where
	B: NetworkBehaviour,
{
	type ToSwarm: Send + 'static;
	type ToLayer: Send + 'static;

	fn on_swarm_event(&mut self, event: &SwarmEvent<<B as NetworkBehaviour>::ToSwarm>);

	fn on_layer_event(&mut self, swarm: &mut Swarm<B>, event: Self::ToLayer) -> Option<Self::ToSwarm>;

	fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::ToLayer>;
}

#[pin_project::pin_project]
pub struct Layer<B, L>
where
	B: NetworkBehaviour,
	L: LayerBehaviour<B>,
{
	_b: PhantomData<B>,
	layer: L,
}
impl<B, L> Layer<B, L>
where
	B: NetworkBehaviour,
	L: LayerBehaviour<B>,
{
	pub fn new(_behaviour: &B, layer: L) -> Self {
		Self { layer, _b: Default::default() }
	}

	pub fn layer_mut(&mut self) -> &mut L {
		&mut self.layer
	}
}
impl<B, L> LayerBehaviour<B> for Layer<B, L>
where
	B: NetworkBehaviour,
	L: LayerBehaviour<B>,
{
	type ToSwarm = L::ToSwarm;
	type ToLayer = L::ToLayer;

	fn on_swarm_event(&mut self, event: &SwarmEvent<<B as NetworkBehaviour>::ToSwarm>) {
		self.layer_mut().on_swarm_event(event)
	}

	fn on_layer_event(&mut self, swarm: &mut Swarm<B>, event: Self::ToLayer) -> Option<Self::ToSwarm> {
		self.layer_mut().on_layer_event(swarm, event)
	}

	fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::ToLayer> {
		self.layer_mut().poll(cx)
	}
}
impl<B, L> Stream for Layer<B, L>
where
	B: NetworkBehaviour,
	L: LayerBehaviour<B>,
{
	type Item = L::ToLayer;

	/// Note: This stream is infinite.
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.as_mut().layer.poll(cx).map(Some)
	}
}
/// As we produce the events in an infinite manner the stream will never be terminated.
impl<B, L> FusedStream for Layer<B, L>
where
	B: NetworkBehaviour,
	L: LayerBehaviour<B>,
{
	fn is_terminated(&self) -> bool {
		false
	}
}
